use crate::errors::TestFailure;
use crate::json_extractor::extract_json;
use crate::json_filter::filter_json;
use crate::test_definition::{StageDescriptor, TestDefinition};
use hyper::header::HeaderValue;
use hyper::{body, Body, Client, Request};
use hyper_tls::HttpsConnector;
use log::{error, info, debug};
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
        info!(
            "Running Test ({}\\{}) `{}` Iteration ({}\\{})...",
            count + 1,
            total,
            td.name.clone().unwrap_or(format!("Test {}", count + 1)),
            iteration + 1,
            td.iterate
        );
        io::stdout().flush().unwrap();

        self.run += 1;

        let mut result = self.validate_setup(td, iteration).await;
        if result.is_ok() {
            result = self.validate_td(td, iteration).await;
            _ = self.run_cleanup(td, iteration, result.is_ok()).await;
        }

        match result {
            Ok(_) => {
                info!("\x1b[32mPASSED\x1b[0m\n");
                self.passed += 1;
                return true;
            }
            Err(e) => {
                info!("\x1b[31mFAILED\x1b[0m\n");
                error!("{}", e);
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
        info!(
            "Dry Run Test ({}\\{}) `{}` Iteration({}\\{})\n",
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
                error!("{}", e);
                return false;
            }
        }
    }

    fn validate_dry_run(
        &mut self,
        td: &TestDefinition,
        iteration: u32,
    ) -> Result<bool, Box<dyn Error + Send + Sync>> {
        // construct request block

        if let Some(setup) = &td.setup {
            let setup_method = setup.request.method.as_method();
            let setup_url = &td.get_url(
                iteration,
                &setup.request.url,
                &setup.request.params,
                &td.variables,
            );
            let setup_headers = td.get_setup_request_headers(iteration);
            let setup_body = td.get_body(&setup.request, &td.variables, iteration);
            info!("setup: {} {}\n", setup_method, setup_url);
            if setup_headers.len() > 0 {
                info!("setup_headers:\n");
                for (key, value) in setup_headers.iter() {
                    info!("-- {}: {}\n", key, value);
                }
            }

            if let Some(body) = setup_body {
                info!("setup_body: {}\n", body);
            }

            if let Some(r) = &setup.response {
                // compare to response definition
                if let Some(setup_response_status) = r.status {
                    info!(
                        "validate setup_response_status with defined_status: {}\n",
                        setup_response_status
                    );
                }

                for v in &r.extract {
                    info!(
                        "attempt to extract value from response: {} = valueOf({})\n",
                        v.name, v.field
                    );
                }

                if r.ignore.len() > 0 {
                    info!("prune fields from setup_response_body\n");
                    for i in r.ignore.iter() {
                        info!("filter: {}\n", i);
                    }
                }

                if let Some(b) = &r.body {
                    if r.ignore.len() > 0 {
                        info!(
                            "validate filtered setup_response_body matches defined body: {}\n",
                            b.data
                        );
                    } else {
                        info!(
                            "validate setup_response_body matches defined body: {}\n",
                            b.data
                        );
                    }
                }
            }
        }

        for (stage_index, stage) in td.stages.iter().enumerate() {
            let stage_method = stage.request.method.as_method();
            let stage_url = &td.get_url(
                iteration,
                &stage.request.url,
                &stage.request.params,
                &[&stage.variables[..], &td.variables[..]].concat(),
            );
            let stage_headers = td.get_headers(&stage.request.headers, iteration);
            let stage_body = td.get_body(
                &stage.request,
                &[&stage.variables[..], &td.variables[..]].concat(),
                iteration,
            );
            info!("stage {}: {} {}\n", stage_index + 1, stage_method, stage_url);
            if stage_headers.len() > 0 {
                info!("headers:\n");
                for (key, value) in stage_headers.iter() {
                    info!("-- {}: {}\n", key, value);
                }
            }

            if let Some(body) = stage_body {
                info!("body: {}\n", body);
            }

            if let Some(r) = &stage.response {
                // compare to response definition
                if let Some(stage_response_status) = r.status {
                    info!(
                        "validate response_status with defined_status: {}\n",
                        stage_response_status
                    );
                }

                for v in &r.extract {
                    info!(
                        "attempt to extract value from response: {} = valueOf({})\n",
                        v.name, v.field
                    );
                }

                if r.ignore.len() > 0 {
                    info!("prune fields from response_body\n");
                    for i in r.ignore.iter() {
                        info!("filter: {}\n", i);
                    }
                }

                if let Some(b) = &r.body {
                    if r.ignore.len() > 0 {
                        info!(
                            "validate filtered response_body matches defined body: {}\n",
                            b.data
                        );
                    } else {
                        info!("validate response_body matches defined body: {}\n", b.data);
                    }
                }
            }

            if let Some(stage_compare) = &stage.compare {
                // construct compare block
                let params = if stage_compare.params.len() > 0 {
                    &stage_compare.params
                } else {
                    &stage.request.params
                };

                let compare_url = &td.get_url(
                    iteration,
                    &stage_compare.url,
                    &params,
                    &[&stage.variables[..], &td.variables[..]].concat(),
                );

                match Url::parse(compare_url) {
                    Ok(_) => {}
                    Err(error) => {
                        return Err(Box::from(format!("invalid stage compare url: {}", error)));
                    }
                }

                let stage_compare_method = &stage_compare.method.as_method().to_string();
                let mut stage_compare_headers = HashMap::new();

                for header in td.get_stage_compare_headers(stage_index, iteration) {
                    let mut header_value: String = header.1;

                    for gv in self.global_variables.iter() {
                        let key_search = format!("${}$", gv.0);
                        header_value = header_value.replace(&key_search, gv.1);
                    }

                    stage_compare_headers.insert(header.0, header_value);
                }

                let stage_compare_body = match &stage_compare.body {
                    Some(b) => {
                        stage_compare_headers
                            .insert("Content-Type".to_string(), "application/json".to_string());
                        match serde_json::to_string(b) {
                            Ok(body) => Some(body),
                            Err(_) => None,
                        }
                    }
                    None => None,
                };

                info!("comparison mode\n");
                info!("compare_request: {} {}\n", stage_compare_method, compare_url);

                if stage_compare_headers.len() > 0 {
                    info!("compare_headers:\n");
                    for (key, value) in stage_compare_headers.iter() {
                        info!("-- {}: {}\n", key, value);
                    }
                }

                if let Some(body) = stage_compare_body {
                    info!("compare_body: {}", body);
                }

                // compare to comparison response
                info!("validate request_status_code matches compare_request_status_code\n");

                if let Some(r) = &stage.response {
                    if r.ignore.len() > 0 {
                        info!("prune fields from compare_response_body\n");
                        for i in r.ignore.iter() {
                            info!("filter: {}\n", i);
                        }
                        info!(
                            "validate filtered response_body matches filtered compare_response_body\n"
                        );
                    } else {
                        info!("validate response_body matches compare_response_body\n");
                    }
                } else {
                    info!("validate response_body matches compare_response_body\n");
                }
            }
        }

        // TODO: fix dryrun cleanup explanation
        if let Some(cleanup) = &td.cleanup {
            let cleanup_method = cleanup.request.method.as_method();
            let cleanup_url = &td.get_url(
                iteration,
                &cleanup.request.url,
                &cleanup.request.params,
                &td.variables,
            );
            let cleanup_headers = td.get_setup_request_headers(iteration);
            let cleanup_body = td.get_body(&cleanup.request, &td.variables, iteration);
            info!("cleanup: {} {}\n", cleanup_method, cleanup_url);
            if cleanup_headers.len() > 0 {
                info!("cleanup_headers:\n");
                for (key, value) in cleanup_headers.iter() {
                    info!("-- {}: {}\n", key, value);
                }
            }

            if let Some(body) = cleanup_body {
                info!("cleanup_body: {}\n", body);
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
            // debug!(
            //     "data doesn't match: req({}) compare({})",
            //     modified_actual,
            //     modified_expected
            // );
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
                        reason: format!("response body doesn't match{}", msg),
                    }));
                }
            }
        }

        Ok(r)
    }

    async fn validate_td(
        &mut self,
        td: &TestDefinition,
        iteration: u32,
    ) -> Result<bool, Box<dyn Error + Send + Sync>> {
        let mut result = true;

        for (stage_index, stage) in td.stages.iter().enumerate() {
            result &= self
                .validate_stage(td, stage, stage_index, iteration)
                .await?;
        }

        Ok(result)
    }

    async fn validate_setup(
        &mut self,
        td: &TestDefinition,
        iteration: u32,
    ) -> Result<bool, Box<dyn Error + Send + Sync>> {
        if let Some(setup) = &td.setup {
            let req_method = setup.request.method.as_method();
            let req_url = &td.get_url(
                iteration,
                &setup.request.url,
                &setup.request.params,
                &td.variables,
            );
            let req_headers = td.get_setup_request_headers(iteration);
            let req_body = td.get_body(&setup.request, &td.variables, iteration);
            let req_response = TestRunner::process_request(
                req_method,
                req_url,
                req_headers,
                req_body,
                &self.global_variables,
            )
            .await?;

            let response_status = req_response.status();
            let (_, body) = req_response.into_parts();
            let response_bytes = body::to_bytes(body).await?;

            if let Some(r) = &setup.response {
                // compare to response definition
                if let Some(setup_response_status) = r.status {
                    TestRunner::validate_status_code(response_status, setup_response_status)?;
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
                                    error!("no json result found: {}", error);
                                }
                            }
                        }

                        if let Some(b) = &r.body {
                            TestRunner::validate_body(rv, b.data.clone(), r.ignore.clone())?;
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
        }

        Ok(true)
    }

    async fn run_cleanup(
        &mut self,
        td: &TestDefinition,
        iteration: u32,
        succeeded: bool,
    ) -> Result<bool, Box<dyn Error + Send + Sync>> {
        if let Some(cleanup) = &td.cleanup {
            if succeeded {
                if let Some(onsuccess) = &cleanup.onsuccess {
                    let success_method = onsuccess.method.as_method();
                    let success_url =
                        &td.get_url(iteration, &onsuccess.url, &onsuccess.params, &td.variables);
                    let success_headers = td.get_headers(&onsuccess.headers, iteration);
                    let success_body = td.get_body(onsuccess, &td.variables, iteration);
                    _ = TestRunner::process_request(
                        success_method,
                        success_url,
                        success_headers,
                        success_body,
                        &self.global_variables,
                    )
                    .await?;
                }
            } else {
                if let Some(onfailure) = &cleanup.onfailure {
                    let failure_method = onfailure.method.as_method();
                    let failure_url =
                        &td.get_url(iteration, &onfailure.url, &onfailure.params, &td.variables);
                    let failure_headers = td.get_headers(&onfailure.headers, iteration);
                    let failure_body = td.get_body(onfailure, &td.variables, iteration);
                    _ = TestRunner::process_request(
                        failure_method,
                        failure_url,
                        failure_headers,
                        failure_body,
                        &self.global_variables,
                    )
                    .await?;
                }
            }

            let req_method = cleanup.request.method.as_method();
            let req_url = &td.get_url(
                iteration,
                &cleanup.request.url,
                &cleanup.request.params,
                &td.variables,
            );
            let req_headers = td.get_cleanup_request_headers(iteration);
            let req_body = td.get_body(&cleanup.request, &td.variables, iteration);
            _ = TestRunner::process_request(
                req_method,
                req_url,
                req_headers,
                req_body,
                &self.global_variables,
            )
            .await?;
        }

        Ok(true)
    }

    async fn validate_stage(
        &mut self,
        td: &TestDefinition,
        stage: &StageDescriptor,
        stage_index: usize,
        iteration: u32,
    ) -> Result<bool, Box<dyn Error + Send + Sync>> {
        let req_method = stage.request.method.as_method();
        let req_uri = &td.get_url(
            iteration,
            &stage.request.url,
            &stage.request.params,
            &[&stage.variables[..], &td.variables[..]].concat(),
        );
        let req_headers = td.get_headers(&stage.request.headers, iteration);
        let req_body = td.get_body(
            &stage.request,
            &[&stage.variables[..], &td.variables[..]].concat(),
            iteration,
        );
        let req_response = TestRunner::process_request(
            req_method,
            req_uri,
            req_headers,
            req_body,
            &self.global_variables,
        )
        .await?;

        let mut compare_response_opt = None;

        if let Some(compare) = &stage.compare {
            let params = if compare.params.len() > 0 {
                &compare.params
            } else {
                &stage.request.params
            };

            let compare_method = compare.method.as_method();
            let compare_uri = &td.get_url(
                iteration,
                &compare.url,
                &params,
                &[&stage.variables[..], &td.variables[..]].concat(),
            );
            let compare_headers = td.get_stage_compare_headers(stage_index, iteration);
            let compare_body = td.get_compare_body(
                compare,
                &[&stage.variables[..], &td.variables[..]].concat(),
                iteration,
            );
            compare_response_opt = Some(
                TestRunner::process_request(
                    compare_method,
                    compare_uri,
                    compare_headers,
                    compare_body,
                    &self.global_variables,
                )
                .await?,
            );
        }

        let response_status = req_response.status();
        let (_, body) = req_response.into_parts();
        let response_bytes = body::to_bytes(body).await?;

        if let Some(r) = &stage.response {
            // compare to response definition
            if let Some(stage_response_status) = r.status {
                TestRunner::validate_status_code(response_status, stage_response_status)?;
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
                                error!("no json result found: {}", error);
                            }
                        }
                    }

                    if let Some(b) = &r.body {
                        TestRunner::validate_body(rv, b.data.clone(), r.ignore.clone())?;
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

        if let Some(compare_resp) = compare_response_opt {
            // compare to comparison response
            TestRunner::validate_status_codes(response_status, compare_resp.status())?;

            let compare_data = body::to_bytes(compare_resp.into_body()).await?;
            let ignored_json_fields = match &stage.response {
                Some(r) => r.ignore.to_owned(),
                None => Vec::new(),
            };

            match serde_json::from_slice(response_bytes.as_ref()) {
                Ok(data_json) => match serde_json::from_slice(compare_data.as_ref()) {
                    Ok(compare_data_json) => {
                        TestRunner::validate_body(
                            data_json,
                            compare_data_json,
                            ignored_json_fields,
                        )?;
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
            }
        }

        Ok(true)
    }

    async fn process_request(
        http_method: hyper::Method,
        uri: &str,
        headers: Vec<(String, String)>,
        body: Option<serde_json::Value>,
        global_variables: &HashMap<String, String>,
    ) -> Result<hyper::Response<Body>, Box<dyn Error + Send + Sync>> {
        let client = Client::builder().build::<_, Body>(HttpsConnector::new());
        debug!("Url: {}", uri);
        match Url::parse(uri) {
            Ok(_) => {}
            Err(error) => {
                return Err(Box::from(format!("invalid request url: {}", error)));
            }
        }

        let mut req_builder = Request::builder().uri(uri);
        req_builder = req_builder.method(http_method);

        for header in headers {
            let mut header_value: String = header.1;

            for gv in global_variables.iter() {
                let key_search = format!("${}$", gv.0);
                header_value = header_value.replace(&key_search, gv.1);
            }

            req_builder = req_builder.header(&header.0, header_value);
        }

        let req_body = match body {
            Some(b) => {
                req_builder = req_builder
                    .header("Content-Type", HeaderValue::from_static("application/json"));
                Body::from(serde_json::to_string(&b)?)
            }
            None => Body::empty(),
        };

        let req_opt = req_builder.body(req_body);
        match req_opt {
            Ok(req) => Ok(client.request(req).await?),
            Err(error) => Err(Box::from(format!("bad request result: {}", error))),
        }
    }
}
