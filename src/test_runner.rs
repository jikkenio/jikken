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
use url::Url;

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
            td.name.clone().unwrap_or(format!("Test {}", count + 1)),
            iteration + 1,
            td.iterate
        );
        io::stdout().flush().unwrap();

        self.run += 1;
        let result = self.validate_td(td, iteration).await;

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

    pub async fn dry_run(
        &mut self,
        td: &TestDefinition,
        count: usize,
        total: usize,
        iteration: u32,
    ) -> bool {
        println!(
            "Dry Run Test ({}\\{}) `{}` Iteration({}\\{})",
            count + 1,
            total,
            td.name.clone().unwrap_or(format!("Test {}", count + 1)),
            iteration + 1,
            td.iterate
        );

        self.run += 1;
        let result = self.validate_dry_run(td, iteration);

        match result {
            Ok(_) => {
                return true;
            }
            Err(e) => {
                println!("{}", e);
                return false;
            }
        }
    }

    async fn validate_td(&mut self, td: &TestDefinition, iteration: u32) -> Result<bool, Box<dyn Error + Send + Sync>> {
        let client = Client::builder().build::<_, Body>(HttpsConnector::new());

        // construct request block
        let uri = &td.get_request_url(iteration);
        trace!("Url: {}", uri);

        match Url::parse(uri) {
            Ok(_) => {}
            Err(error) => {
                return Err(Box::from(format!("invalid request url: {}", error)));
            }
        }

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

        let req_body = match &td.request.body {
            Some(b) => {
                req_builder = req_builder
                    .header("Content-Type", HeaderValue::from_static("application/json"));
                Body::from(b.to_string())
            }
            None => Body::empty(),
        };

        let req_opt = req_builder.body(req_body);
        let mut req_compare_opt: Option<_> = None;

        if let Some(td_compare) = &td.compare {
            // construct compare block
            let uri_compare = &td.get_compare_url(iteration);
            trace!("Compare_Url: {}", uri_compare);

            match Url::parse(uri_compare) {
                Ok(_) => {}
                Err(error) => {
                    return Err(Box::from(format!("invalid compare url: {}", error)));
                }
            }

            let mut req_comparison_builder = Request::builder().uri(uri_compare);
            req_comparison_builder = req_comparison_builder.method(&td_compare.method.as_method());

            for header in td.get_compare_headers(iteration) {
                let mut header_value: String = header.1;

                for gv in self.global_variables.iter() {
                    let key_search = format!("${}$", gv.0);
                    header_value = header_value.replace(&key_search, gv.1);
                }

                req_comparison_builder = req_comparison_builder.header(&header.0, header_value);
            }

            let req_compare_body = match &td_compare.body {
                Some(b) => {
                    req_comparison_builder = req_comparison_builder
                        .header("Content-Type", HeaderValue::from_static("application/json"));
                    Body::from(b.to_string())
                }
                None => Body::empty(),
            };

            req_compare_opt = Some(req_comparison_builder.body(req_compare_body));
        }

        match req_opt {
            Ok(req) => {
                let resp = client.request(req).await?;
                let response_status = resp.status();
                let (_, body) = resp.into_parts();
                let response_bytes = body::to_bytes(body).await?;

                if let Some(r) = &td.response {
                    // compare to response definition
                    if let Some(td_response_status) = r.status {
                        TestRunner::validate_status_code(response_status, td_response_status)?;
                    }

                    match serde_json::from_slice(response_bytes.as_ref()) {
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
                                    Err(error) => {
                                        println!("no json result found: {}", error);
                                    }
                                }
                            }

                            if let Some(b) = &r.body {
                                TestRunner::validate_body(rv, b.clone(), r.ignore.clone())?;
                            }
                        }
                        Err(e) => {
                            error!("{}", e);
                            return Err(Box::from(TestFailure {
                                reason: "response is not valid JSON".to_string(),
                            }));
                        }
                    }
                }

                if let Some(req_compare_result) = req_compare_opt {
                    match req_compare_result {
                        Ok(req_compare) => {
                            // compare to comparison response
                            let resp_compare = client.request(req_compare).await?;
                            TestRunner::validate_status_codes(
                                response_status,
                                resp_compare.status(),
                            )?;

                            let data_compare = body::to_bytes(resp_compare.into_body()).await?;
                            let ignored_json_fields = match &td.response {
                                Some(r) => r.ignore.to_owned(),
                                None => Vec::new(),
                            };

                            match serde_json::from_slice(response_bytes.as_ref()) {
                                Ok(data_json) => {
                                    match serde_json::from_slice(data_compare.as_ref()) {
                                        Ok(data_compare_json) => {
                                            TestRunner::validate_body(
                                                data_json,
                                                data_compare_json,
                                                ignored_json_fields,
                                            )?;
                                        }
                                        Err(e) => {
                                            error!("{}", e);
                                            return Err(Box::from(TestFailure {
                                                reason: "comparison response is not valid JSON"
                                                    .to_string(),
                                            }));
                                        }
                                    }
                                }
                                Err(e) => {
                                    error!("{}", e);
                                    return Err(Box::from(TestFailure {
                                        reason: "response is not valid JSON".to_string(),
                                    }));
                                }
                            };
                        }
                        Err(error) => {
                            return Err(Box::from(format!("bad compare result: {}", error)));
                        }
                    }
                }
            }
            Err(error) => {
                return Err(Box::from(format!("bad request result: {}", error)));
            }
        }

        Ok(true)
    }

    fn validate_dry_run(&mut self, td: &TestDefinition, iteration: u32) -> Result<bool, Box<dyn Error + Send + Sync>> {
        // construct request block
        let uri = &td.get_request_url(iteration);
        trace!("Url: {}", uri);

        match Url::parse(uri) {
            Ok(_) => {}
            Err(error) => {
                return Err(Box::from(format!("invalid request url: {}", error)));
            }
        }

        let request_method = &td.request.method.as_method().to_string();
        let mut request_headers = HashMap::new();

        for header in td.get_request_headers(iteration) {
            let mut header_value: String = header.1;

            for gv in self.global_variables.iter() {
                let key_search = format!("${}$", gv.0);
                header_value = header_value.replace(&key_search, gv.1);
            }

            request_headers.insert(header.0, header_value);
        }

        let req_body = match &td.request.body {
            Some(b) => {
                request_headers.insert("Content-Type".to_string(), "application/json".to_string());
                Body::from(b.to_string())
            }
            None => Body::empty(),
        };

        println!("request: {} {}", request_method, uri);
        println!("request_headers: ");
        for (key, value) in request_headers.iter() {
            println!("-- {}: {}", key, value);
        }

        println!("request_body: {:?}", req_body);

        if let Some(r) = &td.response {
            // compare to response definition
            if let Some(td_response_status) = r.status {
                println!("validate Request status with defined status: {}", td_response_status);
            }

            for v in &r.extract {
                println!("attempt to extract value from response: {} = valueOf({})", v.name, v.field);
            }

            if r.ignore.len() > 0 {
                println!("prune out fields from response_body");
                for i in r.ignore.iter() {
                    println!("filter out: {}", i);
                }
            }

            if let Some(b) = &r.body
            {   
                if r.ignore.len() > 0 {
                    println!("validate filtered response_body matches defined body: {}", b);
                } else {
                    println!("validate response_body matches defined body: {}", b);
                }
            }
        }

        if let Some(td_compare) = &td.compare {
            // construct compare block
            let uri_compare = &td.get_compare_url(iteration);
            trace!("Compare_Url: {}", uri_compare);

            match Url::parse(uri_compare) {
                Ok(_) => {}
                Err(error) => {
                    return Err(Box::from(format!("invalid compare url: {}", error)));
                }
            }

            let request_compare_method = &td_compare.method.as_method().to_string();
            let mut request_compare_headers = HashMap::new();

            for header in td.get_compare_headers(iteration) {
                let mut header_value: String = header.1;

                for gv in self.global_variables.iter() {
                    let key_search = format!("${}$", gv.0);
                    header_value = header_value.replace(&key_search, gv.1);
                }

                request_compare_headers.insert(header.0, header_value);
            }

            let req_compare_body = match &td_compare.body {
                Some(b) => {
                    request_compare_headers.insert("Content-Type".to_string(), "application/json".to_string());
                    Body::from(b.to_string())
                }
                None => Body::empty(),
            };

            println!("comparison mode");
            println!("compare_request: {} {}", request_compare_method, uri_compare);
            println!("compare_headers: ");
            

            for (key, value) in request_compare_headers.iter() {
                println!("-- {}: {}", key, value);
            }

            println!("compare_body: {:?}", req_compare_body);

            // compare to comparison response
            println!("validate request_status_code matches compare_request_status_code");

            if let Some(r) = &td.response {
                if r.ignore.len() > 0 {
                    println!("prune out fields from compare_response_body");
                    for i in r.ignore.iter() {
                        println!("filter out: {}", i);
                    }
                    println!("validate filtered response_body matches filtered compare_response_body");
                } else {
                    println!("validate response_body matches compare_response_body");
                }
            } else {
                println!("validate response_body matches compare_response_body");
            }   
        }

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
