#![allow(dead_code)]
use super::test_descriptor::{HttpVerb, TestDescriptor};
use hyper::{body, Body, Client, Method, Request};
use hyper_tls::HttpsConnector;
use serde_json::{json, Map, Value};
use std::io::{self, Write};

pub struct TestRunner {
    run: u16,
    passed: u16,
    failed: u16,
}

impl TestRunner {
    pub fn new() -> TestRunner {
        TestRunner {
            run: 0,
            passed: 0,
            failed: 0,
        }
    }

    pub async fn run(
        &self,
        td: TestDescriptor,
        count: usize,
    ) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        print!(
            "Running `{}`...",
            td.name.clone().unwrap_or(format!("Test {}", count))
        );
        io::stdout().flush().unwrap();

        // let result: bool = if td.is_comparison {
        //     TestRunner::validate_td_comparison_mode(td).await?
        // } else {
        let result: bool = TestRunner::validate_td(td).await?;
        // };

        if result {
            println!("\x1b[32mPASSED!\x1b[0m");
        } else {
            println!("\x1b[31mFAILED!\x1b[0m");
        }

        Ok(result)
    }

    async fn validate_td(
        td: TestDescriptor,
    ) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        // if td.request.url.is_none() {
        //     return Ok(false);
        // }

        let uri = &td.request.url; //.unwrap();
        let client = Client::builder().build::<_, hyper::Body>(HttpsConnector::new());

        let mut req_builder = Request::builder().uri(uri);

        println!("uri: {}", uri);

        match &td.request.method {
            Some(HttpVerb::Post) => req_builder = req_builder.method(Method::POST),
            Some(HttpVerb::Patch) => req_builder = req_builder.method(Method::PATCH),
            Some(HttpVerb::Put) => req_builder = req_builder.method(Method::PUT),
            Some(_) => req_builder = req_builder.method(Method::GET),
            None => req_builder = req_builder.method(Method::GET),
        }

        for header in (&td.request).get_headers() {
            req_builder = req_builder.header(header.0, header.1);
        }

        let req = req_builder.body(Body::empty()).unwrap();
        let resp = client.request(req).await?;

        let mut pass = true;

        match &td.response {
            Some(r) => {
                let (parts, body) = resp.into_parts();

                match &r.body {
                    Some(b) => {
                        // let v: Value = serde_json::from_str(&String::from_utf8(b).unwrap())?;
                        let bytes = body::to_bytes(body).await?;
                        match serde_json::from_slice(bytes.as_ref()) {
                            Ok(l) => {
                                let rv: Value = l;
                                // println!("Response: {}", rv.to_string());
                                let body_test =
                                    TestRunner::validate_body(rv, b.clone(), Vec::new());
                                pass &= body_test;
                            }
                            Err(_) => {
                                // println!("Error: {}", e);
                                // TODO: add body comparison messaging
                                pass = false;
                            }
                        }
                        // println!("Body: {}", v.to_string());
                    }
                    None => (),
                }

                let status_test = match r.status {
                    Some(code) => TestRunner::validate_status_code(parts.status, code),
                    None => true, // none defined, skip this step
                };
                pass &= status_test;
            }
            None => (),
        }

        // for (name, value) in resp.headers() {
        //     println!("Header: {} -> {}", name, value.to_str().unwrap());
        // }
        // let data = body::to_bytes(resp.into_body()).await?;
        // let data_compare = body::to_bytes(resp_compare.into_body()).await?;
        // pass &= data == data_compare;
        // let data_str = String::from_utf8(data.to_vec());
        // println!("Body: {}", data_str.unwrap_or(String::from("unable to load body data")));

        Ok(pass)
    }

    async fn validate_td_comparison_mode(
        td: TestDescriptor,
    ) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        let uri = &td.request.url;
        let uri_compare = td.compare.clone().unwrap().get_url();
        let client = Client::builder().build::<_, hyper::Body>(HttpsConnector::new());

        let mut req_builder = Request::builder().uri(uri);

        match &td.request.method {
            Some(HttpVerb::Post) => req_builder = req_builder.method(Method::POST),
            Some(HttpVerb::Patch) => req_builder = req_builder.method(Method::PATCH),
            Some(HttpVerb::Put) => req_builder = req_builder.method(Method::PUT),
            Some(_) => req_builder = req_builder.method(Method::GET),
            None => req_builder = req_builder.method(Method::GET),
        }

        for header in &td.request.get_headers() {
            req_builder = req_builder.header(&header.0, &header.1);
        }

        let req = req_builder.body(Body::empty()).unwrap();

        let mut req_comparison_builder = Request::builder().uri(uri_compare);

        match td.compare.clone().unwrap().method {
            Some(HttpVerb::Post) => {
                req_comparison_builder = req_comparison_builder.method(Method::POST)
            }
            Some(HttpVerb::Patch) => {
                req_comparison_builder = req_comparison_builder.method(Method::PATCH)
            }
            Some(HttpVerb::Put) => {
                req_comparison_builder = req_comparison_builder.method(Method::PUT)
            }
            Some(_) => req_comparison_builder = req_comparison_builder.method(Method::GET),
            None => req_comparison_builder = req_comparison_builder.method(Method::GET),
        }

        for header in &td.compare.unwrap().get_headers() {
            req_comparison_builder = req_comparison_builder.header(&header.0, &header.1);
        }

        let req_comparison = req_comparison_builder.body(Body::empty()).unwrap();

        let resp = client.request(req).await?;
        let resp_compare = client.request(req_comparison).await?;

        let mut pass = true;
        let status_test = TestRunner::validate_status_codes(resp.status(), resp_compare.status());

        pass &= status_test;

        // for (name, value) in resp.headers() {
        //     println!("Header: {} -> {}", name, value.to_str().unwrap());
        // }

        let data = body::to_bytes(resp.into_body()).await?;
        let data_compare = body::to_bytes(resp_compare.into_body()).await?;

        match serde_json::from_slice(data.as_ref()) {
            Ok(data_json) => match serde_json::from_slice(data_compare.as_ref()) {
                Ok(data_compare_json) => {
                    pass &= TestRunner::validate_body(data_json, data_compare_json, Vec::new());
                }
                _ => pass = false,
            },
            _ => pass = false,
        };

        // pass &= TestRunner::validate_body(data, data_compare, td.ignore);

        // let data_str = String::from_utf8(data.to_vec());

        // println!("Body: {}", data_str.unwrap_or(String::from("unable to load body data")));

        Ok(pass)
    }

    fn validate_status_codes(actual: hyper::StatusCode, expected: hyper::StatusCode) -> bool {
        let result = actual == expected;
        let label = if result { "PASS" } else { "FAIL" };
        if label == "FAIL" {
            println!(
                "Expected: {}, Actual: {}",
                expected.as_u16(),
                actual.as_u16()
            );
        }
        // println!("validating status code: {}", label);
        result
    }

    fn validate_status_code(actual: hyper::StatusCode, expected: u16) -> bool {
        let result = actual.as_u16() == expected;
        let label = if result { "PASS" } else { "FAIL" };
        if label == "FAIL" {
            println!("Expected: {}, Actual: {}", expected, actual.as_u16());
        }
        // println!("validating status code: {}", label);
        result
    }

    // TODO: I need to add support for ignore when comparing two urls.
    fn validate_body(actual: Value, expected: Value, ignore: Vec<String>) -> bool {
        if ignore.is_empty() {
            return actual == expected;
        }

        let mut map: Map<String, Value> =
            serde_json::from_value(actual).expect("failed to read file");

        for v in ignore {
            if !v.contains('.') {
                map.remove(&v);
            }
        }

        let adjusted_actual = json!(map);

        adjusted_actual == expected
        // get to the nested object "nested"
        // let nested = map.get_mut("age")
        //     .expect("should exist")
        //     .as_object_mut()
        //     .expect("should be an object");

        // // now remove the child from there
        // nested.remove("to.be.removed");
    }

    // fn request_from_td(rd: RequestDescriptor) -> Request {
    //     let mut req_builder = Request::builder()
    //         .uri(rd.url.unwrap());

    //     match rd.verb {
    //         Some(HttpVerb::POST) => req_builder = req_builder.method(Method::POST),
    //         Some(HttpVerb::PATCH) => req_builder = req_builder.method(Method::PATCH),
    //         Some(HttpVerb::PUT) => req_builder = req_builder.method(Method::PUT),
    //         Some(_) => req_builder = req_builder.method(Method::GET),
    //         None => req_builder = req_builder.method(Method::GET)
    //     }

    //     for (k, v) in rd.headers {
    //         req_builder = req_builder.header(k, v);
    //     }

    //     let req = req_builder.body(Body::empty()).unwrap();

    //     return req;
    // }
}
