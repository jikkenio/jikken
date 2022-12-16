use crate::errors::TestFailure;
use crate::test_definition::TestDefinition;
use hyper::{body, Body, Client, Request};
use hyper_tls::HttpsConnector;
use log::{error, trace};
use serde_json::{json, Map, Value};
use std::error::Error;
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
        &mut self,
        td: &TestDefinition,
        count: usize,
        total: usize,
        iteration: u32,
    ) -> bool {
        print!(
            "Running Test ({}\\{}) `{}` Iteration({}\\{})...",
            count,
            total,
            td.name.clone().unwrap_or(format!("Test {}", count)),
            iteration,
            td.iterate
        );
        io::stdout().flush().unwrap();

        self.run += 1;
        let result = if td.compare.is_some() {
            TestRunner::validate_td_comparison_mode(td).await
        } else {
            TestRunner::validate_td(td).await
        };

        match result {
            Ok(_) => {
                println!("\x1b[32mPASSED!\x1b[0m");
                self.passed += 1;
                return true;
            }
            Err(e) => {
                println!("\x1b[31mFAILED!\x1b[0m");
                println!("{}", e);
                return false;
            }
        }
    }

    // TODO: Possibly refactor/combine logic to avoid duplication with comparison mode
    async fn validate_td(td: &TestDefinition) -> Result<bool, Box<dyn Error + Send + Sync>> {
        let uri = &td.get_request_url();
        let client = Client::builder().build::<_, Body>(HttpsConnector::new());

        let mut req_builder = Request::builder().uri(uri);
        req_builder = req_builder.method(&td.request.method.as_method());

        for header in td.get_request_headers() {
            req_builder = req_builder.header(header.0, header.1);
        }

        let req = req_builder.body(Body::empty()).unwrap();
        let resp = client.request(req).await?;

        if let Some(r) = &td.response {
            let (parts, body) = resp.into_parts();

            if let Some(b) = &r.body {
                let bytes = body::to_bytes(body).await?;
                match serde_json::from_slice(bytes.as_ref()) {
                    Ok(l) => {
                        let rv: Value = l;
                        TestRunner::validate_body(rv, b.clone(), Vec::new())?;
                    }
                    Err(e) => {
                        error!("{}", e);
                        return Err(Box::from(TestFailure {
                            reason: "response is not valid JSON".to_string(),
                        }));
                    }
                }
            }

            if let Some(code) = r.status {
                TestRunner::validate_status_code(parts.status, code)?;
            }
        }

        Ok(true)
    }

    // TODO: Possibly refactor/combine logic to avoid so much duplication
    async fn validate_td_comparison_mode(
        td: &TestDefinition,
    ) -> Result<bool, Box<dyn Error + Send + Sync>> {
        let uri_compare = td.get_compare_url();
        let uri = &td.get_request_url();
        let client = Client::builder().build::<_, Body>(HttpsConnector::new());

        trace!("Url: {}", uri);
        trace!("Compare_Url: {}", uri_compare);

        let mut req_builder = Request::builder().uri(uri);
        req_builder = req_builder.method(&td.request.method.as_method());

        for header in td.get_request_headers() {
            req_builder = req_builder.header(&header.0, &header.1);
        }

        let mut req_comparison_builder = Request::builder().uri(uri_compare);
        req_comparison_builder =
            req_comparison_builder.method(&td.compare.clone().unwrap().method.as_method());

        for header in td.get_compare_headers() {
            req_comparison_builder = req_comparison_builder.header(&header.0, &header.1);
        }

        // TODO: support bodies for comparison request
        let req = req_builder.body(Body::empty()).unwrap();
        let req_comparison = req_comparison_builder.body(Body::empty()).unwrap();

        let resp = client.request(req).await?;
        let resp_compare = client.request(req_comparison).await?;

        TestRunner::validate_status_codes(resp.status(), resp_compare.status())?;

        if let Some(td_response) = &td.response {
            if let Some(td_response_status) = td_response.status {
                TestRunner::validate_status_code(resp.status(), td_response_status)?;
            }
        }

        let data = body::to_bytes(resp.into_body()).await?;
        let data_compare = body::to_bytes(resp_compare.into_body()).await?;

        match serde_json::from_slice(data.as_ref()) {
            Ok(data_json) => match serde_json::from_slice(data_compare.as_ref()) {
                Ok(data_compare_json) => {
                    TestRunner::validate_body(data_json, data_compare_json, Vec::new())?;
                }
                Err(e) => {
                    error!("{}", e);
                    return Err(Box::from(TestFailure {
                        reason: "comparison response is not valid JSON".to_string(),
                    }));
                }
            },
            Err(e) => {
                error!("{}", e);
                return Err(Box::from(TestFailure {
                    reason: "response is not valid JSON".to_string(),
                }));
            }
        };

        Ok(true)
    }

    fn validate_status_codes(
        actual: hyper::StatusCode,
        expected: hyper::StatusCode,
    ) -> Result<bool, Box<dyn Error + Send + Sync>> {
        let equality = actual == expected;
        if !equality {
            return Err(Box::from(TestFailure {
                reason: format!(
                    "http status codes don't match: actual({}) expected({})",
                    actual, expected
                ),
            }));
        }

        Ok(true)
    }

    fn validate_status_code(
        actual: hyper::StatusCode,
        expected: u16,
    ) -> Result<bool, Box<dyn Error + Send + Sync>> {
        let equality = actual.as_u16() == expected;
        if !equality {
            return Err(Box::from(TestFailure {
                reason: format!(
                    "http status codes don't match: actual({}) expected({})",
                    actual, expected
                ),
            }));
        }

        Ok(true)
    }

    // TODO: Add support for ignore when comparing two urls.
    // TODO: Add support for nested ignore hierarchies.
    fn validate_body(
        actual: Value,
        expected: Value,
        ignore: Vec<String>,
    ) -> Result<bool, Box<dyn Error + Send + Sync>> {
        if ignore.is_empty() {
            let r = actual == expected;

            if !r {
                trace!("data doesn't match: req({}) compare({})", actual, expected);

                let result = assert_json_diff::assert_json_matches_no_panic(
                    &actual,
                    &expected,
                    assert_json_diff::Config::new(assert_json_diff::CompareMode::Strict),
                );
                match result {
                    Ok(_) => {
                        return Err(Box::from(TestFailure {
                            reason: "response body doesn't match".to_string(),
                        }));
                    }
                    Err(msg) => {
                        return Err(Box::from(TestFailure {
                            reason: format!("response body doesn't match\n{}", msg),
                        }));
                    }
                }
            }

            return Ok(r);
        }

        let mut map: Map<String, Value> =
            serde_json::from_value(actual).expect("failed to read file");

        for v in ignore {
            if !v.contains('.') {
                map.remove(&v);
            }
        }

        let adjusted_actual = json!(map);

        let result = adjusted_actual == expected;

        if !result {
            trace!(
                "data doesn't match: req({}) compare({})",
                adjusted_actual,
                expected
            );

            let result = assert_json_diff::assert_json_matches_no_panic(
                &adjusted_actual,
                &expected,
                assert_json_diff::Config::new(assert_json_diff::CompareMode::Strict),
            );
            match result {
                Ok(_) => {
                    return Err(Box::from(TestFailure {
                        reason: "response body doesn't match".to_string(),
                    }));
                }
                Err(msg) => {
                    return Err(Box::from(TestFailure {
                        reason: format!("response body doesn't match\n{}", msg),
                    }));
                }
            }
        }

        Ok(result)
    }
}
