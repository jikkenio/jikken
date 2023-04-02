use crate::config;
use crate::errors::TestFailure;
use crate::json::extractor::extract_json;
use crate::json::filter::filter_json;
use crate::telemetry;
use crate::test;
use crate::test::http;
use crate::test::definition::RequestResponseDescriptor;
use crate::test::definition::ResponseDescriptor;
use crate::test::{definition, validation};
use crate::Commands;
use crate::TagMode;
use hyper::Method;
use hyper::header::HeaderValue;
use hyper::{body, Body, Client, Request};
use hyper_tls::HttpsConnector;
use log::{debug, error, info, trace};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::io::{self, Write};
use url::Url;
use std::time::{Duration, Instant};

pub struct Report {
    run: u16,
    passed: u16,
    failed: u16,
}

struct State {
    variables: HashMap<String, String>,
}


pub enum TestStatus {
    Invalid, // this may be removed, but could indicate the test was unable to be executed
    Passed,
    Failed,
}

pub struct ResultData {
    headers: Vec<http::Header>,
    status: u32,
    body: serde_json::Value,
}

impl ResultData {
    fn default() -> ResultData {
        ResultData { headers: Vec::new(), status: 0, body: serde_json::Value::Null }
    }

    pub fn from_response(resp: Option<ResponseDescriptor>) -> ResultData {
        if let Some(r) = resp {
            return ResultData{
                headers: r.headers,
                status: r.status.unwrap_or(0) as u32,
                body: r.body.map_or(serde_json::Value::Null, |b| b.data),
            };
        }
        
        ResultData::default()
    }
}

struct RequestDetails {
    headers: Vec<http::Header>,
    url: String,
    method: hyper::Method,
    body: serde_json::Value,
}

impl RequestDetails {
    pub fn from_resolved(req: test::definition::ResolvedRequest) -> RequestDetails {
        RequestDetails { 
            headers: req.headers.iter().map(|h| http::Header::new(h.0, h.1)).collect(),
            url: req.url,
            method: req.method,
            body: req.body.unwrap_or(serde_json::Value::Null),
        }
    }
}

struct ResultDetails {
    request: RequestDetails,
    expected: ResultData,
    actual: Option<ResultData>,
}

struct StageResult {
    stage: u32,
    stage_type: u32,
    runtime: u32,
    status: TestStatus,
    details: ResultDetails,
}

pub async fn execute_tests(
    config: config::Config,
    files: Vec<String>,
    cli: &crate::Cli,
    tags: Vec<String>,
    tag_mode: TagMode,
) -> Report {
    let global_variables = config.generate_global_variables();
    let mut tests_to_ignore: Vec<test::Definition> = Vec::new();
    let mut tests_to_run: Vec<test::Definition> = files
        .iter()
        .filter_map(|filename| {
            let result = test::file::load(filename);
            match result {
                Ok(file) => Some(file),
                Err(e) => {
                    error!("unable to load test file ({}) data: {}", filename, e);
                    None
                }
            }
        })
        .filter_map(|f| {
            let name = f.name.clone().unwrap_or(f.filename.clone());
            let result = validation::validate_file(f, &global_variables);
            match result {
                Ok(td) => {
                    if tags.len() > 0 {
                        let td_tags: HashSet<String> = HashSet::from_iter(td.clone().tags);
                        match tag_mode {
                            TagMode::OR => {
                                for t in tags.iter() {
                                    if td_tags.contains(t) {
                                        return Some(td);
                                    }
                                }

                                tests_to_ignore.push(td.clone());

                                debug!(
                                    "test `{}` doesn't match any tags: {}",
                                    name,
                                    tags.join(", ")
                                );

                                return None;
                            }
                            TagMode::AND => {
                                for t in tags.iter() {
                                    if !td_tags.contains(t) {
                                        tests_to_ignore.push(td.clone());

                                        debug!("test `{}` is missing tag: {}", name, t);
                                        return None;
                                    }
                                }
                            }
                        }
                    }

                    Some(td)
                }
                Err(e) => {
                    error!("test ({}) failed validation: {}", name, e);
                    None
                }
            }
        })
        .collect();

    if tests_to_ignore.len() > 0 {
        trace!("filtering out tests which don't match the tag pattern")
    }

    let tests_by_id: HashMap<String, test::Definition> = tests_to_run
        .clone()
        .into_iter()
        .chain(tests_to_ignore.into_iter())
        .map(|td| (td.id.clone(), td))
        .collect();

    tests_to_run.sort_by(|a, b| a.name.partial_cmp(&b.name).unwrap());

    let mut duplicate_filter: HashSet<String> = HashSet::new();
    let mut tests_to_run_with_dependencies: Vec<test::Definition> = Vec::new();

    trace!("determine test execution order based on dependency graph");

    for td in tests_to_run.into_iter() {
        match &td.requires {
            Some(req) => {
                if tests_by_id.contains_key(req) {
                    if !duplicate_filter.contains(req) {
                        duplicate_filter.insert(req.clone());
                        tests_to_run_with_dependencies
                            .push(tests_by_id.get(req).unwrap().to_owned());
                    }
                }
            }
            _ => {}
        }

        if !duplicate_filter.contains(&td.id) {
            duplicate_filter.insert(td.id.clone());
            tests_to_run_with_dependencies.push(td);
        }
    }

    let total_count = tests_to_run_with_dependencies.len();
    let mut session: Option<telemetry::Session> = None;

    let mode_dryrun = match cli.command {
        Commands::DryRun {
            tags: _,
            tags_or: _,
        } => true,
        _ => false,
    };

    if !mode_dryrun {
        if let Some(token) = &config.settings.api_key {
            if let Ok(t) = uuid::Uuid::parse_str(&token) {
                match telemetry::create_session(t, total_count as u32, &cli, &config).await {
                    Ok(sess) => {
                        session = Some(sess);
                    }
                    Err(e) => {
                        debug!("telemetry failed: {}", e);
                    }
                }
            } else {
                debug!("invalid api token: {}", &token);
            }
        }
    }

    let mut state = State {
        variables: HashMap::new(),
    };

    let mut run_count: u16 = 0;
    let mut passed_count: u16 = 0;
    let mut failed_count: u16 = 0;

    for (i, td) in tests_to_run_with_dependencies.into_iter().enumerate() {
        for iteration in 0..td.iterate {
            run_count = run_count + 1;

            let mut passed = true;
            
            if mode_dryrun {
                info!(
                    "Dry Run Test ({}\\{}) `{}` Iteration({}\\{})\n",
                    i + 1,
                    total_count,
                    td.name.clone().unwrap_or(format!("Test {}", i + 1)),
                    iteration + 1,
                    td.iterate,
                );

                let result = dry_run(&state, &td, iteration).await;

                if let Err(e) = result {
                    passed = false;
                    error!("{}", e);
                }
            } else {
                info!(
                    "Running Test ({}\\{}) `{}` Iteration ({}\\{})...",
                    i + 1,
                    total_count,
                    td.name.clone().unwrap_or(format!("Test {}", i + 1)),
                    iteration + 1,
                    td.iterate,
                );
                io::stdout().flush().unwrap();
                debug!(""); // print a new line if we're in debug | trace mode

                let _test = if let Some(s) = &session {
                    match telemetry::create_test(s, &td).await {
                        Ok(t) => {
                            Some(t)
                        },
                        Err(e) => {
                            debug!("telemetry failed: {}", e);
                            None
                        }
                    }
                } else  {
                    None
                };

                let result = run(&mut state, &td, iteration).await;

                match result {
                    Ok(p) => {
                        if p {
                            info!("\x1b[32mPASSED\x1b[0m\n");
                        } else {
                            info!("\x1b[31mFAILED\x1b[0m\n");
                            passed = false;
                        }
                    },
                    Err(e) => {
                        info!("\x1b[31mFAILED\x1b[0m\n");
                        error!("{}", e);
                        passed = false;
                    }
                }

                // if let Some(t) = test {
                //     telemetry::
                // }
            };


            if passed {
                passed_count = passed_count + 1;
            } else {
                failed_count = failed_count + 1;
            }

            if !config.settings.continue_on_failure && !passed {
                std::process::exit(1);
            }
        }
    }

    Report {
        run: run_count,
        passed: passed_count,
        failed: failed_count,
    }
}

async fn run(state: &mut State, td: &test::Definition, iteration: u32) -> Result<bool, Box<dyn Error + Send + Sync>> {
    let mut result = validate_setup(state, td, iteration).await;
    if result.is_ok() {
        result = validate_td(state, td, iteration).await;
        _ = run_cleanup(state, td, iteration, result.is_ok()).await;
    }

    result
}

async fn dry_run(
    state: &State,
    td: &test::Definition,
    iteration: u32,
) -> Result<bool, Box<dyn Error + Send + Sync>> {
    validate_dry_run(state, td, iteration)
}

fn validate_status_codes(
    actual: hyper::StatusCode,
    expected: hyper::StatusCode,
// ) -> Result<bool, Box<dyn Error + Send + Sync>> {
) -> bool {
    trace!("validating status codes");
    actual == expected
    // if !equality {
    //     return Err(Box::from(TestFailure {
    //         reason: format!(
    //             "http status codes don't match: actual({}) expected({})",
    //             actual, expected
    //         ),
    //     }));
    // }
    // Ok(true)
}

fn validate_status_code(
    actual: hyper::StatusCode,
    expected: u16,
// ) -> Result<bool, Box<dyn Error + Send + Sync>> {
) -> bool {
    trace!("validating status codes");
    actual.as_u16() == expected
    // if !equality {
    //     return Err(Box::from(TestFailure {
    //         reason: format!(
    //             "http status codes don't match: actual({}) expected({})",
    //             actual, expected
    //         ),
    //     }));
    // }

    // Ok(true)
}

// TODO: Add support for ignore when comparing two urls.
// TODO: Add support for nested ignore hierarchies.
fn validate_body(
    actual: Value,
    expected: Value,
    ignore: Vec<String>,
) -> Result<bool, Box<dyn Error + Send + Sync>> {
    trace!("validating response body");
    let mut modified_actual = actual.clone();
    let mut modified_expected = expected.clone();

    // TODO: make this more efficient, with a single pass filter
    for path in ignore.iter() {
        trace!("stripping path({}) from response", path);
        modified_actual = filter_json(path, 0, modified_actual)?;
        modified_expected = filter_json(path, 0, modified_expected)?;
    }

    trace!("compare json");
    let r = modified_actual == modified_expected;

    if !r {
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

async fn validate_td(
    state: &mut State,
    td: &test::Definition,
    iteration: u32,
) -> Result<bool, Box<dyn Error + Send + Sync>> {
    let mut result = true;

    for (stage_index, stage) in td.stages.iter().enumerate() {
        result &= validate_stage(state, td, stage, stage_index, iteration).await?;
    }

    Ok(result)
}

async fn validate_setup(
    state: &mut State,
    td: &test::Definition,
    iteration: u32,
) -> Result<Vec<StageResult>, Box<dyn Error + Send + Sync>> {
    let mut result: Option<StageResult> = None;

    if let Some(setup) = &td.setup {
        let req_method = setup.request.method.as_method();
        let req_url = td.get_url(
            iteration,
            &setup.request.url,
            &setup.request.params,
            &td.variables,
        );
        let req_headers = td.get_setup_request_headers(iteration);
        let req_body = td.get_body(&setup.request, &td.variables, iteration);

        let resolved_request = test::definition::ResolvedRequest::new(setup, req_url.clone(), req_method.clone(), req_headers.clone(), req_body.clone());

        debug!("executing setup stage: {}", req_url);

        let expected = ResultData::from_response(setup.response.clone());
        let start_time = Instant::now();
        let req_response_res = process_request(state, resolved_request).await;
        let runtime = start_time.elapsed();
        
        match req_response_res {
            Err(e) => {
                return Ok(vec!(StageResult{
                    stage: 0,
                    stage_type: 1,
                    runtime: runtime.as_millis() as u32,
                    details: ResultDetails {
                        request: RequestDetails{
                            headers: req_headers.iter().map(|h| http::Header::new(h.0.clone(), h.1.clone())).collect(),
                            url: req_url.to_string(),
                            method: req_method,
                            body: req_body.unwrap_or(serde_json::Value::Null),
                        },
                        expected,
                        actual: None
                    },
                    status: TestStatus::Failed
                }));
            },
            Ok(req_response) => {
                let response_status = req_response.status();
                let (_, body) = req_response.into_parts();
                let response_bytes = body::to_bytes(body).await?;
        
                if let Some(r) = &setup.response {
                    // compare to response definition
                    if let Some(setup_response_status) = r.status {
                        validate_status_code(response_status, setup_response_status);
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
                                        state.variables.insert(v.name.clone(), converted_result);
                                    }
                                    Err(error) => {
                                        error!("no json result found: {}", error);
                                    }
                                }
                            }
        
                            if let Some(b) = &r.body {
                                validate_body(rv, b.data.clone(), r.ignore.clone())?;
                            }
                        }
                        Err(e) => {
                            error!("response is not valid JSON: {}", e);
                            // return Err(Box::from(TestFailure {
                            //     reason: "response is not valid JSON".to_string(),
                            // }));
                        }
                    }
                }
            }
        }
    }

    Ok(Vec::new())
}

async fn validate_response(state: &mut State, request: test::definition::ResolvedRequest, response_result: Result<hyper::Response<Body>, Box<dyn Error + Send + Sync>>, stage: u32, stage_type: u32, runtime: u32) -> StageResult {
    let expected = ResultData::from_response(request.req_resp.response);
    let request_details = RequestDetails::from_resolved(request);
    
    match response_result {
        Err(e) => {
            return StageResult{
                stage,
                stage_type,
                runtime,
                status: TestStatus::Invalid,
                details: ResultDetails {
                    request: request_details,
                    expected,
                    actual: None
                },
            };
        },
        Ok(req_response) => {
            let response_status = req_response.status();
            let (_, body) = req_response.into_parts();
            let response_bytes = body::to_bytes(body).await;
    
            match response_bytes {
                Ok(bytes) => {
                    if let Some(r) = request.req_resp.response {
                        // compare to response definition
                        if let Some(setup_response_status) = r.status {
                            validate_status_code(response_status, setup_response_status);
                        }
            
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
                                            state.variables.insert(v.name.clone(), converted_result);
                                        }
                                        Err(error) => {
                                            error!("no json result found: {}", error);
                                        }
                                    }
                                }
            
                                if let Some(b) = &r.body {
                                    validate_body(rv, b.data.clone(), r.ignore.clone());
                                }
                            },
                            Err(e) => {
                                error!("response is not valid JSON: {}", e);
                                // return Err(Box::from(TestFailure {
                                //     reason: "response is not valid JSON".to_string(),
                                // }));
                            }
                        }
                    }
                }, 
                Err(e) => {

                }
            }
        }
    }
}

async fn run_cleanup(
    state: &mut State,
    td: &test::Definition,
    iteration: u32,
    succeeded: bool,
) -> Result<bool, Box<dyn Error + Send + Sync>> {
    if td.cleanup.request.is_some()
        || td.cleanup.onsuccess.is_some()
        || td.cleanup.onfailure.is_some()
    {
        debug!("running test cleanup");
    } else {
        return Ok(true);
    }

    if succeeded {
        if let Some(onsuccess) = &td.cleanup.onsuccess {
            debug!("execute onsucess request");
            let success_method = onsuccess.method.as_method();
            let success_url =
                &td.get_url(iteration, &onsuccess.url, &onsuccess.params, &td.variables);
            let success_headers = td.get_headers(&onsuccess.headers, iteration);
            let success_body = td.get_body(onsuccess, &td.variables, iteration);
            _ = process_request(
                state,
                success_method,
                success_url,
                success_headers,
                success_body,
            )
            .await?;
        }
    } else {
        if let Some(onfailure) = &td.cleanup.onfailure {
            debug!("execute onfailure request");
            let failure_method = onfailure.method.as_method();
            let failure_url =
                &td.get_url(iteration, &onfailure.url, &onfailure.params, &td.variables);
            let failure_headers = td.get_headers(&onfailure.headers, iteration);
            let failure_body = td.get_body(onfailure, &td.variables, iteration);
            _ = process_request(
                state,
                failure_method,
                failure_url,
                failure_headers,
                failure_body,
            )
            .await?;
        }
    }

    if let Some(request) = &td.cleanup.request {
        debug!("execute cleanup request");
        let req_method = request.method.as_method();
        let req_url = &td.get_url(iteration, &request.url, &request.params, &td.variables);
        let req_headers = td.get_cleanup_request_headers(iteration);
        let req_body = td.get_body(&request, &td.variables, iteration);
        _ = process_request(state, req_method, req_url, req_headers, req_body).await?;
    }

    Ok(true)
}

async fn validate_stage(
    state: &mut State,
    td: &test::Definition,
    stage: &definition::StageDescriptor,
    stage_index: usize,
    iteration: u32,
) -> Result<StageResult, Box<dyn Error + Send + Sync>> {
    debug!("execute stage request");
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

    let req_response = process_request(state, req_method, req_uri, req_headers, req_body).await?;
    let mut compare_response_opt = None;

    if let Some(compare) = &stage.compare {
        debug!("execute stage comparison");
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
            process_request(
                state,
                compare_method,
                compare_uri,
                compare_headers,
                compare_body,
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
            validate_status_code(response_status, stage_response_status)?;
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
                            state.variables.insert(v.name.clone(), converted_result);
                        }
                        Err(error) => {
                            error!("no json result found: {}", error);
                        }
                    }
                }

                if let Some(b) = &r.body {
                    validate_body(rv, b.data.clone(), r.ignore.clone())?;
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
        validate_status_codes(response_status, compare_resp.status())?;

        let compare_data = body::to_bytes(compare_resp.into_body()).await?;
        let ignored_json_fields = match &stage.response {
            Some(r) => r.ignore.to_owned(),
            None => Vec::new(),
        };

        match serde_json::from_slice(response_bytes.as_ref()) {
            Ok(data_json) => match serde_json::from_slice(compare_data.as_ref()) {
                Ok(compare_data_json) => {
                    validate_body(data_json, compare_data_json, ignored_json_fields)?;
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
    state: &State,
    resolved_request: test::definition::ResolvedRequest,
    // http_method: hyper::Method,
    // uri: &str,
    // headers: Vec<(String, String)>,
    // body: Option<serde_json::Value>,
) -> Result<hyper::Response<Body>, Box<dyn Error + Send + Sync>> {
    let client = Client::builder().build::<_, Body>(HttpsConnector::new());
    debug!("url({})", resolved_request.url);
    match Url::parse(&resolved_request.url) {
        Ok(_) => {}
        Err(error) => {
            return Err(Box::from(format!("invalid request url: {}", error)));
        }
    }

    let mut req_builder = Request::builder().uri(resolved_request.url);
    req_builder = req_builder.method(resolved_request.method);

    for header in resolved_request.headers {
        let mut header_value: String = header.1;

        for gv in state.variables.iter() {
            let key_search = format!("${}$", gv.0);
            header_value = header_value.replace(&key_search, gv.1);
        }

        debug!("header({}) value({})", &header.0, &header_value);
        req_builder = req_builder.header(&header.0, header_value);
    }

    let req_body = match resolved_request.body {
        Some(b) => {
            req_builder =
                req_builder.header("Content-Type", HeaderValue::from_static("application/json"));
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

fn validate_dry_run(
    state: &State,
    td: &test::Definition,
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
        info!(
            "stage {}: {} {}\n",
            stage_index + 1,
            stage_method,
            stage_url
        );
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

                for gv in state.variables.iter() {
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
            info!(
                "compare_request: {} {}\n",
                stage_compare_method, compare_url
            );

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

    if let Some(onsuccess) = &td.cleanup.onsuccess {
        info!("when test successful, run onsuccess request:\n");
        let onsuccess_method = onsuccess.method.as_method();
        let onsuccess_url =
            &td.get_url(iteration, &onsuccess.url, &onsuccess.params, &td.variables);
        let onsuccess_headers = td.get_setup_request_headers(iteration);
        let onsuccess_body = td.get_body(&onsuccess, &td.variables, iteration);
        info!("onsuccess: {} {}\n", onsuccess_method, onsuccess_url);
        if onsuccess_headers.len() > 0 {
            info!("onsuccess_headers:\n");
            for (key, value) in onsuccess_headers.iter() {
                info!("-- {}: {}\n", key, value);
            }
        }

        if let Some(body) = onsuccess_body {
            info!("onsuccess_body: {}\n", body);
        }
    }

    if let Some(onfailure) = &td.cleanup.onfailure {
        info!("when test fails, run onfailure request:\n");
        let onfailure_method = onfailure.method.as_method();
        let onfailure_url =
            &td.get_url(iteration, &onfailure.url, &onfailure.params, &td.variables);
        let onfailure_headers = td.get_setup_request_headers(iteration);
        let onfailure_body = td.get_body(&onfailure, &td.variables, iteration);
        info!("onfailure: {} {}\n", onfailure_method, onfailure_url);
        if onfailure_headers.len() > 0 {
            info!("onfailure_headers:\n");
            for (key, value) in onfailure_headers.iter() {
                info!("-- {}: {}\n", key, value);
            }
        }

        if let Some(body) = onfailure_body {
            info!("onfailure_body: {}\n", body);
        }
    }

    if let Some(request) = &td.cleanup.request {
        info!("run cleanup requests:\n");
        let cleanup_method = request.method.as_method();
        let cleanup_url = &td.get_url(iteration, &request.url, &request.params, &td.variables);
        let cleanup_headers = td.get_setup_request_headers(iteration);
        let cleanup_body = td.get_body(&request, &td.variables, iteration);
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