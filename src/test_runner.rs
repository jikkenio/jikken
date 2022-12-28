use crate::errors::TestFailure;
use crate::json_extractor::extract_json;
use crate::json_filter::filter_json;
use crate::test_definition::TestDefinition;
use hyper::header::HeaderValue;
use hyper::{body, Body, Client, Request};
use hyper_tls::HttpsConnector;
use log::{error, trace};
use serde_json::Value;
use std::collections::HashMap;
use std::error::Error;
use std::io::{self, Write};

pub struct TestRunner {
    run: u16,
    passed: u16,
    failed: u16,

    global_variables: HashMap<String, String>,
}

impl TestRunner {
    pub fn new() -> TestRunner {
        TestRunner {
            run: 0,
            passed: 0,
            failed: 0,

            global_variables: HashMap::new(),
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
            count + 1,
            total,
            td.name.clone().unwrap_or(format!("Test {}", count)),
            iteration + 1,
            td.iterate
        );
        io::stdout().flush().unwrap();

        self.run += 1;
        let result = if td.compare.is_some() {
            self.validate_td_comparison_mode(td, iteration).await
        } else {
            self.validate_td(td, iteration).await
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
                self.failed += 1;
                return false;
            }
        }
    }

    // TODO: Possibly refactor/combine logic to avoid duplication with comparison mode
    async fn validate_td(
        &mut self,
        td: &TestDefinition,
        iteration: u32,
    ) -> Result<bool, Box<dyn Error + Send + Sync>> {
        let uri = &td.get_request_url(iteration);
        let client = Client::builder().build::<_, Body>(HttpsConnector::new());

        let mut req_builder = Request::builder().uri(uri);
        req_builder = req_builder.method(&td.request.method.as_method());

        for header in td.get_request_headers(iteration) {
            let mut header_value: String = header.1;

            for gv in self.global_variables.iter() {
                let key_search = format!("${}$", gv.0);
                header_value = header_value.replace(&key_search, gv.1);
            }

            req_builder = req_builder.header(header.0, header_value);
        }

        let req_body = match &td.request.body {
            Some(b) => {
                req_builder = req_builder
                    .header("Content-Type", HeaderValue::from_static("application/json"));
                Body::from(b.to_string())
            }
            None => Body::empty(),
        };

        let req = req_builder.body(req_body).unwrap();
        let resp = client.request(req).await?;

        let ignored_json_fields = match &td.response {
            Some(r) => r.ignore.to_owned(),
            None => Vec::new(),
        };

        if let Some(r) = &td.response {
            let (parts, body) = resp.into_parts();

            let bytes = body::to_bytes(body).await?;
            match serde_json::from_slice(bytes.as_ref()) {
                Ok(l) => {
                    let rv: Value = l;
                    for v in &r.extract {
                        match extract_json(&v.field, 0, rv.clone()) {
                            Ok(result) => {
                                let converted_result = match result {
                                    serde_json::Value::Bool(b) => b.to_string(),
                                    serde_json::Value::Number(n) => n.to_string(),
                                    serde_json::Value::String(s) => s.to_string(),
                                    _ => "".to_string(),
                                };
                                self.global_variables
                                    .insert(v.name.clone(), converted_result);
                            }
                            _ => {}
                        }
                    }

                    if let Some(b) = &r.body {
                        TestRunner::validate_body(rv, b.clone(), ignored_json_fields)?;
                    }
                }
                Err(e) => {
                    error!("{}", e);
                    return Err(Box::from(TestFailure {
                        reason: "response is not valid JSON".to_string(),
                    }));
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
        &mut self,
        td: &TestDefinition,
        iteration: u32,
    ) -> Result<bool, Box<dyn Error + Send + Sync>> {
        let uri_compare = td.get_compare_url(iteration);
        let uri = &td.get_request_url(iteration);
        let client = Client::builder().build::<_, Body>(HttpsConnector::new());

        trace!("Url: {}", uri);
        trace!("Compare_Url: {}", uri_compare);

        let mut req_builder = Request::builder().uri(uri);
        req_builder = req_builder.method(&td.request.method.as_method());

        for header in td.get_request_headers(iteration) {
            let mut header_value: String = header.1;

            for gv in self.global_variables.iter() {
                let key_search = format!("${}$", gv.0);
                header_value = header_value.replace(&key_search, gv.1);
            }

            req_builder = req_builder.header(&header.0, header_value);
        }

        let mut req_comparison_builder = Request::builder().uri(uri_compare);
        req_comparison_builder =
            req_comparison_builder.method(&td.compare.clone().unwrap().method.as_method());

        for header in td.get_compare_headers(iteration) {
            let mut header_value: String = header.1;

            for gv in self.global_variables.iter() {
                let key_search = format!("${}$", gv.0);
                header_value = header_value.replace(&key_search, gv.1);
            }

            req_comparison_builder = req_comparison_builder.header(&header.0, header_value);
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
        let ignored_json_fields = match &td.response {
            Some(r) => r.ignore.to_owned(),
            None => Vec::new(),
        };

        match serde_json::from_slice(data.as_ref()) {
            Ok(data_json) => match serde_json::from_slice(data_compare.as_ref()) {
                Ok(data_compare_json) => {
                    TestRunner::validate_body(data_json, data_compare_json, ignored_json_fields)?;
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
        let mut modified_actual = actual.clone();
        let mut modified_expected = expected.clone();

        // TODO: make this more efficient, with a single pass filter
        for path in ignore.iter() {
            modified_actual = filter_json(path, 0, modified_actual)?;
            modified_expected = filter_json(path, 0, modified_expected)?;
        }

        let r = modified_actual == modified_expected;

        if !r {
            trace!(
                "data doesn't match: req({}) compare({})",
                modified_actual,
                modified_expected
            );

            let result = assert_json_diff::assert_json_matches_no_panic(
                &modified_actual,
                &modified_expected,
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

        Ok(r)
    }
}
