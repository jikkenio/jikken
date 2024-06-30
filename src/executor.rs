use crate::config;
use crate::json::extractor::extract_json;
use crate::telemetry;
use crate::test;
use crate::test::definition::ResponseDescriptor;
use crate::test::file::BodyOrSchema;
use crate::test::file::BodyOrSchemaChecker;
use crate::test::file::Checker;
use crate::test::file::ValueOrNumericSpecification;
use crate::test::http;
use crate::test::http::Header;
use crate::test::Definition;
use crate::test::{definition, validation, Variable};
use crate::TagMode;
use hyper::header::HeaderValue;
use hyper::{body, Body, Client, Request};
use hyper_tls::HttpsConnector;
use log::{debug, error, info, trace, warn};
use serde::Serialize;
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fmt;
use std::io::Write;
use std::time::Instant;
use std::vec;
use url::Url;
use validated::Validated::{self, Good};

#[derive(Default)]
pub struct Report {
    pub test_files: u16,
    pub run: u16,
    pub passed: u16,
    pub failed: u16,
    pub skipped: u16,
}

impl From<ExecutionResult> for Report {
    fn from(execution_result: ExecutionResult) -> Self {
        let test_files = execution_result.test_results.len();
        let totals = execution_result
            .test_results
            .into_iter()
            .map(|tr| {
                if tr.iteration_results.is_empty() {
                    (0, 0, 1)
                } else {
                    tr.iteration_results.into_iter().fold(
                        (0, 0, 0),
                        |(passed, failed, skipped), iteration_result| match iteration_result.status
                        {
                            TestStatus::Failed => (passed, failed + 1, skipped),
                            TestStatus::Passed => (passed + 1, failed, skipped),
                            TestStatus::Skipped => (passed, failed, skipped + 1),
                        },
                    )
                }
            })
            .fold(
                (0, 0, 0),
                |(total_passed, total_failed, total_skipped), (passed, failed, skipped)| {
                    (
                        total_passed + passed,
                        total_failed + failed,
                        total_skipped + skipped,
                    )
                },
            );

        Report {
            skipped: totals.2,
            failed: totals.1,
            passed: totals.0,
            test_files: test_files as u16,
            run: totals.1 + totals.0,
        }
    }
}

pub struct IterationResult {
    pub iteration_number: u32,
    pub status: TestStatus,
    pub stage_results: Option<Result<(bool, Vec<StageResult>), Box<dyn Error + Send + Sync>>>,
}

impl IterationResult {
    pub fn new(
        iteration_number: u32,
        stage_results: Result<(bool, Vec<StageResult>), Box<dyn Error + Send + Sync>>,
    ) -> Self {
        //Determine test status here and store in the status field
        let passed = *stage_results
            .as_ref()
            .map(|(passed, _)| passed)
            .unwrap_or(&false);

        Self {
            iteration_number,
            status: if passed {
                TestStatus::Passed
            } else {
                TestStatus::Failed
            },
            stage_results: Some(stage_results),
        }
    }

    pub fn new_skipped(iteration_number: u32) -> Self {
        Self {
            iteration_number,
            status: TestStatus::Skipped,
            stage_results: None,
        }
    }
}

pub struct TestResult {
    pub test_name: String,
    pub iteration_results: Vec<IterationResult>,
}

pub struct ExecutionResult {
    //Elapsed Time?
    //Start Time?
    pub test_results: Vec<TestResult>,
}

struct FormattedExecutionResult(String);

impl fmt::Display for FormattedExecutionResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

trait ExecutionResultFormatter {
    fn format(&self, res: &ExecutionResult) -> FormattedExecutionResult;
}

fn formatted_result_to_file<T: ExecutionResultFormatter>(
    formatter: T,
    execution_result: &ExecutionResult,
    file: &str,
) -> Result<(), std::io::Error> {
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(file)
        .and_then(|mut f| f.write(formatter.format(execution_result).0.as_bytes()))
        .map(|_| ())
}

struct JunitResultFormatter;

impl ExecutionResultFormatter for JunitResultFormatter {
    fn format(&self, res: &ExecutionResult) -> FormattedExecutionResult {
        let mut lines: Vec<String> = Vec::new();

        lines.push(r#"<?xml version="1.0" encoding="UTF-8"?>"#.to_string());
        lines.push(r#"<testsuites>"#.to_string());
        for test in res.test_results.iter() {
            lines.push(format!(r#"<testsuite name="{}">"#, test.test_name));
            for iteration_result in test.iteration_results.iter() {
                let test_iteration_name = format!(
                    "{}.Iterations.{}",
                    test.test_name.as_str(),
                    iteration_result.iteration_number + 1
                );
                lines.push(format!(
                    r#"<testsuite name="{}">"#,
                    test_iteration_name.as_str(),
                ));
                for stage_result in iteration_result.stage_results.iter() {
                    match &stage_result {
                        Ok((_passed, stage_results)) => {
                            for (stage_number, stage_result) in stage_results.iter().enumerate() {
                                if stage_result.status == TestStatus::Passed {
                                    lines.push(format!(
                                        r#"<testcase name="stage_{}" classname="{}"/>"#,
                                        stage_number + 1,
                                        test_iteration_name.as_str()
                                    ));
                                } else {
                                    lines.push(format!(
                                        r#"<testcase name="stage_{}" classname="{}">"#,
                                        stage_number + 1,
                                        test_iteration_name.as_str()
                                    ));

                                    if let validated::Validated::Fail(nec) =
                                        &stage_result.validation
                                    {
                                        for i in nec {
                                            lines.push(format!(
                                                r#"<failure message="{}" type="AssertionError"/>"#,
                                                i
                                            ));
                                        }
                                    }

                                    lines.push(r#"</testcase>"#.to_string());
                                }
                            }
                        }
                        Err(_) => {
                            lines.push(
                                r#"<testcase name="Initial" classname="Initial" />"#.to_string(),
                            );
                        }
                    }
                }

                lines.push("</testsuite>".to_string());
            }
            lines.push("</testsuite>".to_string());
        }
        lines.push("</testsuites>".to_string());

        FormattedExecutionResult(lines.join("\n"))
    }
}

trait ExecutionPolicy {
    fn name(&self) -> String;
    async fn execute(
        &mut self,
        state: &mut State,
        telemetry: &Option<telemetry::Session>,
        test: &test::Definition,
        iteration: u32,
        config: &config::Config,
    ) -> Result<(bool, Vec<StageResult>), Box<dyn Error + Send + Sync>>;

    async fn skip(
        &mut self,
        telemetry: &Option<telemetry::Session>,
        test: &test::Definition,
        config: &config::Config,
    ) -> Result<(), Box<dyn Error + Send + Sync>>;
}

struct DryRunExecutionPolicy;

impl ExecutionPolicy for DryRunExecutionPolicy {
    fn name(&self) -> String {
        "Dry Run".to_string()
    }

    async fn execute(
        &mut self,
        state: &mut State,
        _telemetry: &Option<telemetry::Session>,
        test: &test::Definition,
        iteration: u32,
        _config: &config::Config,
    ) -> Result<(bool, Vec<StageResult>), Box<dyn Error + Send + Sync>> {
        dry_run(state, test, iteration)
            .await
            .map(|passed| (passed, vec![] as Vec<StageResult>))
    }

    async fn skip(
        &mut self,
        _telemetry: &Option<telemetry::Session>,
        _test: &test::Definition,
        _config: &config::Config,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        Ok(())
    }
}

struct ActualRunExecutionPolicy;

impl ExecutionPolicy for ActualRunExecutionPolicy {
    fn name(&self) -> String {
        "Running".to_string()
    }

    async fn execute(
        &mut self,
        state: &mut State,
        telemetry: &Option<telemetry::Session>,
        test: &test::Definition,
        iteration: u32,
        config: &config::Config,
    ) -> Result<(bool, Vec<StageResult>), Box<dyn Error + Send + Sync>> {
        let telemetry_test = if let Some(s) = &telemetry {
            match telemetry::create_test(s, test.clone(), config).await {
                Ok(t) => Some(t),
                Err(e) => {
                    debug!("telemetry failed: {}", e);
                    None
                }
            }
        } else {
            None
        };

        run(state, test, iteration, telemetry_test, config).await
    }

    async fn skip(
        &mut self,
        telemetry: &Option<telemetry::Session>,
        test: &test::Definition,
        config: &config::Config,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let session = match &telemetry {
            Some(session) => session,
            None => {
                debug!("missing telemetry session");
                return Err(Box::from("missing telemetry session"));
            }
        };

        match telemetry::create_test(session, test.clone(), config).await {
            Ok(telemetry_test) => {
                match telemetry::complete_stage_skipped(&telemetry_test, test, config).await {
                    Ok(_) => Ok(()),
                    Err(error) => {
                        debug!("telemetry test completion failed: {}", error);
                        Err(error)
                    }
                }
            }
            Err(error) => {
                debug!("telemetry test creation failed: {}", error);
                Err(error)
            }
        }
    }
}

struct FailurePolicy<T: ExecutionPolicy> {
    wrapped_policy: T,
    failed: bool,
}

impl<T: ExecutionPolicy> FailurePolicy<T> {
    fn new(policy: T) -> FailurePolicy<T> {
        FailurePolicy {
            wrapped_policy: policy,
            failed: false,
        }
    }
}

impl<T: ExecutionPolicy> ExecutionPolicy for FailurePolicy<T> {
    fn name(&self) -> String {
        self.wrapped_policy.name()
    }

    async fn execute(
        &mut self,
        state: &mut State,
        telemetry: &Option<telemetry::Session>,
        test: &test::Definition,
        iteration: u32,
        config: &config::Config,
    ) -> Result<(bool, Vec<StageResult>), Box<dyn Error + Send + Sync>> {
        let ret = self
            .wrapped_policy
            .execute(state, telemetry, test, iteration, config)
            .await;
        let passed = ret.as_ref().map(|(passed, _)| *passed).unwrap_or_default();
        self.failed = !passed;
        ret
    }

    async fn skip(
        &mut self,
        telemetry: &Option<telemetry::Session>,
        test: &test::Definition,
        config: &config::Config,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        self.wrapped_policy.skip(telemetry, test, config).await
    }
}

async fn run_tests<T: ExecutionPolicy>(
    tests: Vec<Vec<test::Definition>>,
    telemetry: Option<telemetry::Session>,
    mut exec_policy: T,
    config: &config::Config,
) -> ExecutionResult {
    let flattened_tests: Vec<test::Definition> = tests.into_iter().flatten().collect();
    let total_count = flattened_tests.len();
    let mut results: Vec<TestResult> = Vec::new();

    let mut state = State {
        variables: HashMap::new(),
        cookies: HashMap::new(),
    };
    let start_time = Instant::now();

    let mut any_failures = false;
    let mut message_displayed = false;

    for (i, test) in flattened_tests.into_iter().enumerate() {
        if any_failures && !config.settings.continue_on_failure && !message_displayed {
            warn!("Skipping remaining tests due to continueOnFailure setting.");
            log::logger().flush();
            message_displayed = true;
        }

        //let mut test_result: Vec<Result<(bool, Vec<StageResult>), Box<dyn Error + Send + Sync>>> =
        //    Vec::new();
        let mut iteration_results: Vec<IterationResult> = Vec::new();
        let test_name = test.name.clone().unwrap_or(format!("Test{}", i + 1));
        for iteration in 0..test.iterate {
            // TODO: clean this up based on policies
            // I don't see a clean way to access it without refactoring
            if any_failures && !config.settings.continue_on_failure {
                if iteration == 0 {
                    info!(
                        "{} Test ({}/{}) `{}`...\x1b[33mSKIPPED\x1b[0m\n",
                        exec_policy.name(),
                        i + 1,
                        total_count,
                        &test_name,
                    );
                    let _ = exec_policy.skip(&telemetry, &test, config).await;
                    iteration_results.push(IterationResult::new_skipped(iteration));
                }
                break;
            }

            if test.disabled {
                info!(
                    "{} Test ({}/{}) `{}`...\x1b[33mDISABLED\x1b[0m\n",
                    exec_policy.name(),
                    i + 1,
                    total_count,
                    &test_name,
                );
                let _ = exec_policy.skip(&telemetry, &test, config).await;
                iteration_results.push(IterationResult::new_skipped(iteration));
                break;
            }

            info!(
                "{} Test ({}/{}) `{}` Iteration({}/{})...",
                exec_policy.name(),
                i + 1,
                total_count,
                &test_name,
                iteration + 1,
                test.iterate,
            );

            let result = exec_policy
                .execute(&mut state, &telemetry, &test, iteration, config)
                .await;

            match &result {
                Ok(p) => {
                    if p.0 {
                        info!("\x1b[32mPASSED\x1b[0m\n");
                    } else {
                        any_failures = true;
                        info!("\x1b[31mFAILED\x1b[0m\n");
                    }
                }
                Err(e) => {
                    any_failures = true;
                    info!("\x1b[31mFAILED\x1b[0m\n");
                    error!("{}", e);
                }
            }

            log::logger().flush();

            iteration_results.push(IterationResult::new(iteration, result));
        }
        results.push(TestResult {
            test_name,
            iteration_results,
        });
    }

    let runtime = start_time.elapsed().as_millis() as u32;

    if let Some(s) = &telemetry {
        let status = if any_failures { 2 } else { 1 };
        _ = telemetry::complete_session(s, runtime, status, config).await;
    }

    ExecutionResult {
        test_results: results,
    }
}

struct StateCookie {
    domain: String,
    path: String,
    key: String,
    value: String,
    secure: bool,
}

impl StateCookie {
    pub fn new(data: String) -> Option<StateCookie> {
        debug!("cookie new: {}", &data);
        let segments: Vec<&str> = data.split(';').collect();
        let cookie_value: Vec<&str> = segments
            .first()
            .expect("cookie should have segments")
            .split('=')
            .collect();

        let key: String = cookie_value.first().unwrap_or(&"").trim().to_string();
        let value: String = cookie_value.last().unwrap_or(&"").trim().to_string();
        let mut domain: String = "".to_string();
        let mut path: String = "/".to_string();
        let mut secure: bool = false;

        for s in segments {
            let key_value: Vec<&str> = s.split('=').collect();
            let k = key_value.first().unwrap_or(&"").trim();
            let v = key_value.last().unwrap_or(&"").trim().to_string();

            match k {
                "Domain" => domain = v,
                "Path" => path = v,
                "Secure" => secure = true,
                &_ => {}
            }
        }

        Some(StateCookie {
            domain,
            path,
            key,
            value,
            secure,
        })
    }

    pub fn update(&mut self, new_cookie: StateCookie) {
        self.value = new_cookie.value;
    }
}

struct State {
    variables: HashMap<String, String>,
    cookies: HashMap<String, HashMap<String, StateCookie>>,
}

#[derive(PartialEq, Eq, Clone)]
pub enum StageType {
    Setup = 1,
    Normal = 2,
    Cleanup = 3,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum TestStatus {
    Passed = 1,
    Failed = 2,
    Skipped = 5,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ResponseResultData {
    pub headers: Vec<http::Header>,
    pub status: u16,
    pub body: serde_json::Value,
}

impl ResponseResultData {
    pub async fn from_response(resp: hyper::Response<Body>) -> Option<ResponseResultData> {
        debug!("Received response : {resp:?}");

        let response_status = resp.status();
        // TODO: We'll have to revisit this to support non-ASCII headers
        let headers = resp
            .headers()
            .iter()
            .map(|h| http::Header::new(h.0.to_string(), h.1.to_str().unwrap_or("").to_string()))
            .collect();
        let (_, body) = resp.into_parts();
        let response_bytes = body::to_bytes(body).await;

        match response_bytes {
            Ok(resp_data) => match serde_json::from_slice(resp_data.as_ref()) {
                Ok(data) => {
                    debug!("Body is {data}");
                    Some(ResponseResultData {
                        headers,
                        status: response_status.as_u16(),
                        body: data,
                    })
                }
                Err(e) => {
                    // TODO: add support for non JSON responses
                    debug!("response is not valid JSON data: {}", e);
                    debug!("{}", std::str::from_utf8(&resp_data).unwrap_or(""));
                    Some(ResponseResultData {
                        headers,
                        status: response_status.as_u16(),
                        body: serde_json::Value::Null,
                    })
                }
            },
            Err(e) => {
                error!("unable to get response bytes: {}", e);
                None
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ExpectedResultData {
    pub headers: Vec<http::Header>,
    pub status: Option<ValueOrNumericSpecification<u16>>,
    pub body: Option<BodyOrSchema>,
    pub strict: bool,
}

impl ExpectedResultData {
    pub fn new() -> Self {
        Self {
            headers: Vec::default(),
            status: Option::default(),
            body: Option::default(),
            strict: true,
        }
    }
    //Consider making get_body a static method that
    //accepts the global vars. Passing the Definition seems wrong
    pub fn from_request(
        req: Option<ResponseDescriptor>,
        td: &test::Definition,
        state_variables: &HashMap<String, String>,
        variables: &[Variable],
        iteration: u32,
    ) -> ExpectedResultData {
        req.map(|r| ExpectedResultData {
            headers: r.headers,
            status: r.status,
            body: td.get_expected_request_body(&r.body, state_variables, variables, iteration), //.unwrap_or(serde_json::Value::Null),
            strict: r.strict,
        })
        .unwrap_or(ExpectedResultData::new())
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct RequestDetails {
    pub headers: Vec<http::Header>,
    pub url: String,
    pub method: http::Method,
    pub body: serde_json::Value,
}

#[derive(Debug, Clone, Serialize)]
pub struct ResultDetails {
    pub request: RequestDetails,
    pub expected: ExpectedResultData,
    pub actual: Option<ResponseResultData>,
    pub compare_request: Option<RequestDetails>,
    pub compare_actual: Option<ResponseResultData>,
}

#[derive(Clone)]
pub struct StageResult {
    pub stage: u32,
    pub stage_type: StageType,
    pub stage_name: Option<String>,
    pub runtime: u32,
    pub status: TestStatus,
    pub details: ResultDetails,
    pub validation: Validated<Vec<()>, String>,
    pub project: Option<String>,
    pub environment: Option<String>,
}

fn load_test_from_path(filename: &str) -> Option<test::File> {
    let load_result = test::file::load(filename);
    match load_result {
        Ok(file) => Some(file),
        Err(e) => {
            error!("unable to load test file ({}) data: {}", filename, e);
            None
        }
    }
}

fn validate_test_file(
    test_file: test::File,
    global_variables: &[test::Variable],
    project: Option<String>,
    environment: Option<String>,
) -> Option<test::Definition> {
    let name = test_file
        .name
        .clone()
        .unwrap_or_else(|| test_file.filename.clone());
    let res = validation::validate_file(test_file, global_variables, project, environment);
    match res {
        Ok(file) => Some(file),
        Err(e) => {
            error!("Test \"{}\" failed validation: {}.", name, e);
            None
        }
    }
}

//consider using a set for tags and leverage set operations
//insted of raw loops
fn ignored_due_to_tag_filter(
    test_definition: &test::Definition,
    tags: &[String],
    tag_mode: &TagMode,
) -> bool {
    let test_name = test_definition
        .name
        .clone()
        .unwrap_or("UKNOWN_NAME".to_string());

    match tag_mode {
        TagMode::OR => {
            for t in tags.iter() {
                if test_definition.tags.contains(t) {
                    return false;
                }
            }

            debug!(
                "test `{}` doesn't match any tags: {}",
                test_name,
                tags.join(", ")
            );
            true
        }
        TagMode::AND => {
            for t in tags.iter() {
                if !test_definition.tags.contains(t) {
                    debug!("test `{}` is missing tag: {}", test_name, t);
                    return true;
                }
            }
            false
        }
    }
}

fn schedule_impl(
    graph: &BTreeMap<String, BTreeSet<String>>,
    scheduled_nodes: &BTreeSet<String>,
) -> BTreeSet<String> {
    let mut ignore: BTreeSet<String> = BTreeSet::new();
    ignore.clone_from(scheduled_nodes);

    //Is there a way to do in 1 iteration?
    graph
        .iter()
        .filter(|(node, _)| !scheduled_nodes.contains(*node))
        .for_each(|(_, edges)| {
            edges.iter().for_each(|e| _ = ignore.insert(e.clone()));
        });
    return graph
        .keys()
        .filter(|s| !ignore.contains(*s))
        .cloned()
        .collect();
}

fn construct_test_execution_graph_v2(
    tests_to_run: Vec<test::Definition>,
    tests_to_ignore: Vec<test::Definition>,
) -> Vec<Vec<Definition>> {
    let tests_by_id: HashMap<String, test::Definition> = tests_to_run
        .clone()
        .into_iter()
        .chain(tests_to_ignore)
        .map(|td| (td.id.clone(), td))
        .collect();

    trace!("determine test execution order based on dependency graph");

    //Nodes are IDs ; Directed edges imply ordering; i.e. A -> B; B depends on A
    let mut graph: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    tests_to_run
        .iter()
        .map(|td| (td.id.clone(), td))
        .for_each(|(id, definition)| {
            if let Some(req) = definition.requires.as_ref() {
                let required_def = tests_by_id.get(req);
                if required_def.is_none() {
                    return;
                }

                if required_def.unwrap().disabled {
                    warn!(
                        "Test \"{}\" requires a disabled test: \"{}\"",
                        definition.name.as_deref().unwrap_or(id.as_str()),
                        required_def.unwrap().id
                    );
                    //should we do transitive disablement?
                }

                if let Some(edges) = graph.get_mut(req) {
                    edges.insert(id.clone());
                } else {
                    graph.insert(req.clone(), BTreeSet::from([id.clone()]));
                }
            }

            let node_for_id = graph.get(&id);
            if node_for_id.is_none() {
                graph.insert(id.clone(), BTreeSet::new());
            }
            //intution: if it already has a dependent, its simply a test
            //depended on by multiple other tests and not a duplicate ID made in error
            else if node_for_id.unwrap().is_empty() {
                warn!("Skipping test, found duplicate test id: {}", id.clone());
            }
        });

    let mut jobs: Vec<BTreeSet<String>> = Vec::new();
    let mut scheduled_nodes: BTreeSet<String> = BTreeSet::new();
    while graph.len() != scheduled_nodes.len() {
        let job = schedule_impl(&graph, &scheduled_nodes);
        job.iter()
            .for_each(|n| _ = scheduled_nodes.insert(n.clone()));
        jobs.push(job);
    }

    let job_definitions: Vec<Vec<Definition>> = jobs
        .into_iter()
        .map(|hs| {
            hs.into_iter()
                .map(|id| tests_by_id.get(&id).unwrap().clone())
                .collect::<Vec<Definition>>()
        })
        .collect();

    let flattened_jobs = job_definitions
        .iter()
        .flatten()
        .collect::<Vec<&Definition>>();

    if tests_to_run.len() != flattened_jobs.len() {
        //not smart enough on rust to write generic lambda in order to not repeat myself here
        let s1: HashSet<String> = tests_to_run
            .iter()
            .map(|td| td.name.clone().unwrap_or(td.id.clone()))
            .collect();
        let s2: HashSet<String> = flattened_jobs
            .iter()
            .map(|td| td.name.clone().unwrap_or(td.id.clone()))
            .collect();
        let missing_tests = (&s1 - &s2)
            .into_iter()
            .filter(|s| !s.is_empty())
            .map(|s| format!("\"{}\"", s))
            .collect::<Vec<String>>()
            .join(",");

        if !missing_tests.is_empty() {
            warn!("Warning: Required tests not found.");
            warn!(
                "Check the 'requires' tag in the following test definition(s): {}.\n\n",
                missing_tests
            );
        }
    }

    for (count, job) in job_definitions.iter().enumerate() {
        trace!(
            "Job {count}, Tests: {}",
            job.iter().fold("".to_string(), |acc, x| format!(
                "{},{}",
                acc,
                x.name.as_ref().unwrap_or(&x.id)
            ))
        )
    }

    job_definitions
}

pub fn tests_from_files(
    config: &config::Config,
    files: Vec<String>,
    tags: Vec<String>,
    project: Option<String>,
    environment: Option<String>,
    tag_mode: TagMode,
) -> (Vec<test::Definition>, Vec<test::Definition>) {
    let global_variables = config.generate_global_variables();
    let mut tests_to_ignore: Vec<test::Definition> = Vec::new();
    let tests_to_run: Vec<test::Definition> = files
        .into_iter()
        .filter_map(|s| load_test_from_path(s.as_str()))
        .filter_map(|f| {
            validate_test_file(f, &global_variables, project.clone(), environment.clone())
        })
        .filter_map(|f| {
            if !ignored_due_to_tag_filter(&f, &tags, &tag_mode) {
                Some(f)
            } else {
                tests_to_ignore.push(f);
                None
            }
        })
        .collect();
    (tests_to_run, tests_to_ignore)
}

pub async fn execute_tests(
    config: config::Config,
    tests_to_run: Vec<test::Definition>,
    mode_dryrun: bool,
    tests_to_ignore: Vec<test::Definition>,
    junit_file: Option<String>,
    cli_args: Box<serde_json::Value>,
) -> Report {
    if !tests_to_ignore.is_empty() {
        trace!("filtering out tests which don't match the tag pattern")
    }

    trace!("determine test execution order based on dependency graph");

    let tests_to_run_with_dependencies =
        construct_test_execution_graph_v2(tests_to_run.clone(), tests_to_ignore.clone());
    let all_tests: Vec<&Definition> = tests_to_run_with_dependencies.iter().flatten().collect();

    let mut session: Option<telemetry::Session> = None;

    if !mode_dryrun {
        if let Some(token) = &config.settings.api_key {
            if let Ok(t) = uuid::Uuid::parse_str(token) {
                match telemetry::create_session(t, all_tests, cli_args, &config).await {
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

    let execution_result = if mode_dryrun {
        run_tests(
            tests_to_run_with_dependencies,
            session,
            FailurePolicy::new(DryRunExecutionPolicy),
            &config,
        )
        .await
    } else {
        run_tests(
            tests_to_run_with_dependencies,
            session,
            FailurePolicy::new(ActualRunExecutionPolicy),
            &config,
        )
        .await
    };

    _ = junit_file.and_then(|f| {
        formatted_result_to_file(JunitResultFormatter {}, &execution_result, f.as_str())
            .map_err(|e| {
                error!("Error writing junit report to {f} : {e}");
                e
            })
            .ok()
    });

    Report::from(execution_result)
}

async fn run(
    state: &mut State,
    td: &test::Definition,
    iteration: u32,
    test: Option<telemetry::Test>,
    config: &config::Config,
) -> Result<(bool, Vec<StageResult>), Box<dyn Error + Send + Sync>> {
    let mut results = Vec::new();
    let mut setup_result = validate_setup(state, td, iteration).await?;

    if let Some(test_telemetry) = &test {
        if !setup_result.1.is_empty() {
            let telemetry_result = telemetry::complete_stage(
                td,
                test_telemetry,
                iteration,
                &setup_result.1[0],
                config,
            )
            .await;
            if let Err(e) = telemetry_result {
                debug!("telemetry stage completion failed: {}", e);
            }
        }
    }
    results.append(&mut setup_result.1);
    let mut success = setup_result.0;

    if success {
        let td_results = validate_td(state, td, iteration, test.clone(), config).await;

        match td_results {
            Ok(mut r) => {
                results.append(&mut r.1);
                success = r.0;
            }
            Err(e) => {
                trace!("td validation error: {}", e);
                success = false;
            }
        }

        let cleanup_result = run_cleanup(state, td, iteration, success, results.len() as u32).await;
        match cleanup_result {
            Ok(mut r) => {
                if let Some(test_telemetry) = &test {
                    for result in r.1.iter() {
                        let telemetry_result = telemetry::complete_stage(
                            td,
                            test_telemetry,
                            iteration,
                            result,
                            config,
                        )
                        .await;
                        if let Err(e) = telemetry_result {
                            debug!("telemetry stage completion failed: {}", e);
                        }
                    }
                }
                results.append(&mut r.1);
                success &= r.0;
            }
            Err(e) => {
                trace!("cleanup validation error: {}", e);
                success = false;
            }
        }
    }

    Ok((success, results))
}

async fn dry_run(
    state: &State,
    td: &test::Definition,
    iteration: u32,
) -> Result<bool, Box<dyn Error + Send + Sync>> {
    validate_dry_run(state, td, iteration)
}

async fn validate_td(
    state: &mut State,
    td: &test::Definition,
    iteration: u32,
    test: Option<telemetry::Test>,
    config: &config::Config,
) -> Result<(bool, Vec<StageResult>), Box<dyn Error + Send + Sync>> {
    let mut results = Vec::new();

    for (stage_index, stage) in td.stages.iter().enumerate() {
        let stage_result = validate_stage(state, td, stage, stage_index, iteration).await?;

        if let Some(test_telemetry) = &test {
            let telemetry_result =
                telemetry::complete_stage(td, test_telemetry, iteration, &stage_result, config)
                    .await;
            if let Err(e) = telemetry_result {
                debug!("telemetry stage completion failed: {}", e);
            }
        }

        let failed = stage_result.status == TestStatus::Failed;
        results.push(stage_result);

        if failed {
            return Ok((false, results));
        }
    }

    Ok((true, results))
}

fn process_response(
    stage: u32,
    stage_type: StageType,
    stage_name: Option<String>,
    runtime: u32,
    details: ResultDetails,
    ignore_body: &[String],
    project: Option<String>,
    environment: Option<String>,
) -> StageResult {
    trace!("process_response()");

    let mut result = StageResult {
        stage,
        stage_type,
        stage_name,
        runtime,
        details: details.clone(),
        status: TestStatus::Passed,
        validation: Validated::Good(vec![()]),
        project,
        environment,
    };

    let validate_headers = |validation_type: &str,
                            _expected: &Vec<Header>,
                            _actual: &Vec<Header>|
     -> Validated<(), String> {
        if !_expected.is_empty() {
            //no logic currently
            trace!("validating {}headers", validation_type);
        }
        Good(())
    };

    let validate_status_code = |validation_type: &str,
                                expected: &Option<ValueOrNumericSpecification<u16>>,
                                actual: u16|
     -> Vec<Validated<(), String>> {
        match expected {
            None => vec![Good(())].into_iter().collect(),
            Some(t) => {
                trace!("validating {}status codes", validation_type);

                t.check(&actual, &|expected, actual| -> String {
                    format!("Expected status code {expected} but received {actual}")
                })
            }
        }
    };

    let validate_body = |validation_type: &str,
                         expected: &std::option::Option<BodyOrSchema>,
                         actual: &serde_json::Value,
                         ignore_body: &[String],
                         strict: bool|
     -> Vec<Validated<(), String>> {
        trace!("In validate body({:?})", expected);
        if let Some(exp) = expected {
            trace!("validating body");
            BodyOrSchemaChecker {
                value_or_schema: exp,
                ignore_values: ignore_body,
                strict,
            }
            .check(actual, &|e, a| {
                format!(
                    "Expected {}{} did not match actual {}",
                    validation_type, e, a
                )
            })
        } else {
            vec![Good(())]
        }
    };

    if let Some(resp) = &details.actual {
        let mut validation: Vec<Validated<(), String>> = vec![Good(())];

        validation.push(validate_headers(
            "",
            &details.expected.headers,
            &resp.headers,
        ));
        validation.append(validate_status_code("", &details.expected.status, resp.status).as_mut());
        validation.append(
            validate_body(
                "",
                &details.expected.body,
                &resp.body,
                ignore_body,
                details.expected.strict,
            )
            .as_mut(),
        );

        validation.append(
            //if a compare request was specified, validate it
            details
                .compare_actual
                .map(|compare_request_result| {
                    let mut ret = vec![];
                    ret.append(
                        validate_status_code(
                            "compare ",
                            &Some(ValueOrNumericSpecification::<u16>::Value(
                                compare_request_result.status,
                            )),
                            resp.status,
                        )
                        .as_mut(),
                    );
                    ret.append(
                        validate_body(
                            "compare ",
                            &Some(BodyOrSchema::Body(compare_request_result.body)),
                            &resp.body,
                            ignore_body,
                            details.expected.strict,
                        )
                        .as_mut(),
                    );
                    ret
                })
                .unwrap_or(vec![Good(())])
                .as_mut(),
        );

        result.validation = validation.into_iter().collect();
        result.status = if result.validation.is_fail() {
            TestStatus::Failed
        } else {
            TestStatus::Passed
        };
    } else if details.expected != ExpectedResultData::new() {
        // a result was specified,
        //and we failed to get an actual response
        result.validation = Validated::fail("failed to get response".to_string());
        result.status = TestStatus::Failed;
    }

    if let validated::Validated::Fail(nec) = &result.validation {
        let error_str = nec
            .into_iter()
            .fold("Response Validation Error(s):".to_string(), |acc, curr| {
                format!("{acc}\n{curr}")
            });
        error!("{error_str}\n");
    }

    result
}

async fn validate_setup(
    state: &mut State,
    td: &test::Definition,
    iteration: u32,
) -> Result<(bool, Vec<StageResult>), Box<dyn Error + Send + Sync>> {
    if let Some(setup) = &td.setup {
        let req_method = setup.request.method.as_method();
        let req_url = td.get_url(
            iteration,
            &setup.request.url,
            &setup.request.params,
            &state.variables,
            &td.variables,
        );
        let req_headers = td.get_setup_request_headers(iteration);
        let req_body = td.get_request_body(
            &setup.request.body,
            &state.variables,
            &td.variables,
            iteration,
        );

        let resolved_request = test::definition::ResolvedRequest::new(
            req_url.clone(),
            req_method.clone(),
            req_headers.clone(),
            req_body.clone(),
        );

        debug!("executing setup stage: {}", req_url);

        let expected = ExpectedResultData::from_request(
            setup.response.clone(),
            td,
            &state.variables,
            &td.variables,
            iteration,
        );
        let start_time = Instant::now();
        let req_response = process_request(state, resolved_request).await?;
        let runtime = start_time.elapsed().as_millis() as u32;
        let actual = ResponseResultData::from_response(req_response).await;

        let request = RequestDetails {
            headers: req_headers
                .iter()
                .map(|h| http::Header::new(h.0.clone(), h.1.clone()))
                .collect(),
            url: req_url.to_string(),
            method: req_method,
            body: req_body.unwrap_or(serde_json::Value::Null),
        };

        let details = ResultDetails {
            request,
            expected,
            actual,
            compare_request: None,
            compare_actual: None,
        };

        let result = process_response(
            0,
            StageType::Setup,
            None,
            runtime,
            details,
            &setup.response.clone().map_or(Vec::new(), |r| r.ignore),
            td.project.clone(),
            td.environment.clone(),
        );

        // extract variables and add them to the state
        if let Some(r) = &setup.response {
            if let Some(a) = &result.details.actual {
                for v in &r.extract {
                    match extract_json(&v.field, 0, a.body.clone()) {
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
            }
        }

        return Ok((result.status == TestStatus::Passed, vec![result]));
    }

    Ok((true, Vec::new()))
}

async fn run_cleanup(
    state: &mut State,
    td: &test::Definition,
    iteration: u32,
    succeeded: bool,
    stage_count: u32,
) -> Result<(bool, Vec<StageResult>), Box<dyn Error + Send + Sync>> {
    let mut results = Vec::new();
    let mut counter = stage_count;

    if td.cleanup.always.is_some()
        || td.cleanup.onsuccess.is_some()
        || td.cleanup.onfailure.is_some()
    {
        debug!("running test cleanup");
    } else {
        return Ok((true, results));
    }

    if succeeded {
        if let Some(onsuccess) = &td.cleanup.onsuccess {
            debug!("execute onsuccess request");
            let success_method = onsuccess.method.as_method();
            let success_url = &td.get_url(
                iteration,
                &onsuccess.url,
                &onsuccess.params,
                &state.variables,
                &td.variables,
            );
            let success_headers = td.get_headers(&onsuccess.headers, iteration);
            let success_body =
                td.get_request_body(&onsuccess.body, &state.variables, &td.variables, iteration);
            let resolved_request = test::definition::ResolvedRequest::new(
                success_url.clone(),
                success_method.clone(),
                success_headers.clone(),
                success_body.clone(),
            );

            let expected = ExpectedResultData::from_request(
                None,
                td,
                &state.variables,
                &td.variables,
                iteration,
            );
            let start_time = Instant::now();
            let req_response = process_request(state, resolved_request).await?;
            let runtime = start_time.elapsed().as_millis() as u32;
            let actual = ResponseResultData::from_response(req_response).await;

            let request = RequestDetails {
                headers: success_headers
                    .iter()
                    .map(|h| http::Header::new(h.0.clone(), h.1.clone()))
                    .collect(),
                url: success_url.to_string(),
                method: success_method,
                body: success_body.unwrap_or(serde_json::Value::Null),
            };

            let details = ResultDetails {
                request,
                expected,
                actual,
                compare_request: None,
                compare_actual: None,
            };

            let result = process_response(
                counter,
                StageType::Cleanup,
                None,
                runtime,
                details,
                &Vec::new(),
                td.project.clone(),
                td.environment.clone(),
            );
            counter += 1;
            results.push(result);
        }
    } else if let Some(onfailure) = &td.cleanup.onfailure {
        debug!("execute onfailure request");
        let failure_method = onfailure.method.as_method();
        let failure_url = &td.get_url(
            iteration,
            &onfailure.url,
            &onfailure.params,
            &state.variables,
            &td.variables,
        );
        let failure_headers = td.get_headers(&onfailure.headers, iteration);
        let failure_body =
            td.get_request_body(&onfailure.body, &state.variables, &td.variables, iteration);
        let resolved_request = test::definition::ResolvedRequest::new(
            failure_url.clone(),
            failure_method.clone(),
            failure_headers.clone(),
            failure_body.clone(),
        );

        let expected =
            ExpectedResultData::from_request(None, td, &state.variables, &td.variables, iteration);
        let start_time = Instant::now();
        let req_response = process_request(state, resolved_request).await?;
        let runtime = start_time.elapsed().as_millis() as u32;
        let actual = ResponseResultData::from_response(req_response).await;

        let request = RequestDetails {
            headers: failure_headers
                .iter()
                .map(|h| http::Header::new(h.0.clone(), h.1.clone()))
                .collect(),
            url: failure_url.to_string(),
            method: failure_method,
            body: failure_body.unwrap_or(serde_json::Value::Null),
        };

        let details = ResultDetails {
            request,
            expected,
            actual,
            compare_request: None,
            compare_actual: None,
        };

        let result = process_response(
            counter,
            StageType::Cleanup,
            None,
            runtime,
            details,
            &Vec::new(),
            td.project.clone(),
            td.environment.clone(),
        );
        counter += 1;
        results.push(result);
    }

    if let Some(request) = &td.cleanup.always {
        debug!("execute cleanup request");
        let req_method = request.method.as_method();
        let req_url = &td.get_url(
            iteration,
            &request.url,
            &request.params,
            &state.variables,
            &td.variables,
        );
        let req_headers = td.get_cleanup_request_headers(iteration);
        let req_body =
            td.get_request_body(&request.body, &state.variables, &td.variables, iteration);
        let resolved_request = test::definition::ResolvedRequest::new(
            req_url.clone(),
            req_method.clone(),
            req_headers.clone(),
            req_body.clone(),
        );

        let expected =
            ExpectedResultData::from_request(None, td, &state.variables, &td.variables, iteration);
        let start_time = Instant::now();
        let req_response = process_request(state, resolved_request).await?;
        let runtime = start_time.elapsed().as_millis() as u32;
        let actual = ResponseResultData::from_response(req_response).await;

        let request = RequestDetails {
            headers: req_headers
                .iter()
                .map(|h| http::Header::new(h.0.clone(), h.1.clone()))
                .collect(),
            url: req_url.to_string(),
            method: req_method,
            body: req_body.unwrap_or(serde_json::Value::Null),
        };

        let details = ResultDetails {
            request,
            expected,
            actual,
            compare_request: None,
            compare_actual: None,
        };

        let result = process_response(
            counter,
            StageType::Cleanup,
            None,
            runtime,
            details,
            &Vec::new(),
            td.project.clone(),
            td.environment.clone(),
        );
        results.push(result);
    }

    Ok((true, results))
}

async fn validate_stage(
    state: &mut State,
    td: &test::Definition,
    stage: &definition::StageDescriptor,
    stage_index: usize,
    iteration: u32,
) -> Result<StageResult, Box<dyn Error + Send + Sync>> {
    let stage_name = stage.name.clone().unwrap_or((stage_index + 1).to_string());
    debug!("execute stage {stage_name}");

    //Darius here? Or could be in caller

    let req_method = stage.request.method.as_method();
    let req_url = &td.get_url(
        iteration,
        &stage.request.url,
        &stage.request.params,
        &state.variables,
        &[&stage.variables[..], &td.variables[..]].concat(),
    );
    let req_headers = td.get_headers(&stage.request.headers, iteration);
    let req_body = td.get_request_body(
        &stage.request.body,
        &state.variables,
        &[&stage.variables[..], &td.variables[..]].concat(),
        iteration,
    );

    let resolved_request = test::definition::ResolvedRequest::new(
        req_url.clone(),
        req_method.clone(),
        req_headers.clone(),
        req_body.clone(),
    );
    debug!("executing test stage {stage_name}: {req_url}");
    let expected = ExpectedResultData::from_request(
        stage.response.clone(),
        td,
        &state.variables,
        &[&stage.variables[..], &td.variables[..]].concat(),
        iteration,
    );
    let request = RequestDetails {
        headers: req_headers
            .iter()
            .map(|h| http::Header::new(h.0.clone(), h.1.clone()))
            .collect(),
        url: req_url.to_string(),
        method: req_method,
        body: req_body.unwrap_or(serde_json::Value::Null),
    };
    let mut compare_response_opt = None;
    let mut compare_request = None;

    let start_time = Instant::now();
    let req_response = process_request(state, resolved_request).await?;

    if let Some(compare) = &stage.compare {
        debug!("execute stage {stage_name} comparison");
        let params = stage.get_compare_parameters();

        let compare_method = compare.method.as_method();
        let compare_url = &td.get_url(
            iteration,
            &compare.url,
            &params,
            &state.variables,
            &[&stage.variables[..], &td.variables[..]].concat(),
        );
        let compare_headers = td.get_stage_compare_headers(stage_index, iteration);
        let compare_body = td.get_compare_body(
            compare,
            &state.variables,
            &[&stage.variables[..], &td.variables[..]].concat(),
            iteration,
        );

        let resolved_compare_request = test::definition::ResolvedRequest::new(
            compare_url.clone(),
            compare_method.clone(),
            compare_headers.clone(),
            compare_body.clone(),
        );

        compare_request = Some(RequestDetails {
            headers: compare_headers
                .iter()
                .map(|h| http::Header::new(h.0.clone(), h.1.clone()))
                .collect(),
            url: compare_url.to_string(),
            method: compare_method,
            body: compare_body.unwrap_or(serde_json::Value::Null),
        });

        compare_response_opt = Some(process_request(state, resolved_compare_request).await?);
    }

    let runtime = start_time.elapsed().as_millis() as u32;
    let actual = ResponseResultData::from_response(req_response).await;
    let mut compare_actual = None;

    if let Some(compare_response) = compare_response_opt {
        compare_actual = ResponseResultData::from_response(compare_response).await;
    }

    let details = ResultDetails {
        request,
        expected,
        actual,
        compare_request,
        compare_actual,
    };

    let result = process_response(
        stage_index as u32,
        StageType::Normal,
        stage.name.clone(),
        runtime,
        details,
        &stage.response.clone().map_or(Vec::new(), |r| r.ignore),
        td.project.clone(),
        td.environment.clone(),
    );

    // extract variables and add them to the state
    if let Some(r) = &stage.response {
        if let Some(a) = &result.details.actual {
            for v in &r.extract {
                match extract_json(&v.field, 0, a.body.clone()) {
                    Ok(result) => {
                        let converted_result = match result {
                            serde_json::Value::Bool(b) => b.to_string(),
                            serde_json::Value::Number(n) => n.to_string(),
                            serde_json::Value::String(s) => s.to_string(),
                            _ => "".to_string(),
                        };
                        debug!("extracting variable: {} = {}", v.name, converted_result);
                        state.variables.insert(v.name.clone(), converted_result);
                    }
                    Err(error) => {
                        error!("no json result found: {}", error);
                    }
                }
            }
        }
    }

    if stage.delay.is_some_and(|d| d > 0) {
        tokio::time::sleep(tokio::time::Duration::from_millis(stage.delay.unwrap())).await;
    }

    Ok(result)
}

fn http_request_from_test_spec(
    state: &State,
    resolved_request: test::definition::ResolvedRequest,
) -> Result<Request<Body>, Box<dyn Error + Send + Sync>> {
    let vars: Vec<(String, &String)> = state
        .variables
        .iter()
        .map(|(k, v)| (format!("${{{}}}", k), v))
        .collect();

    //Where all can we resolve variables? May be worth making an external function
    let variable_resolver = |variable: String| -> String {
        vars.iter().fold(variable, |acc, (var_name, var_value)| {
            acc.replace(var_name, var_value)
        })
    };

    let (tld_prefix, is_secure) = if resolved_request.url.starts_with("http://") {
        (resolved_request.url[7..].to_string().to_lowercase(), false)
    } else if resolved_request.url.starts_with("https://") {
        (resolved_request.url[8..].to_string().to_lowercase(), true)
    } else {
        (resolved_request.url.clone().to_lowercase(), false)
    };

    let cookies = state
        .cookies
        .iter()
        .filter(|(k, _)| tld_prefix.starts_with(&k.to_lowercase()))
        .flat_map(|(_, v)| {
            v.iter()
                .map(|(_, cookie)| {
                    if !(cookie.secure ^ is_secure) {
                        (
                            "Cookie".to_string(),
                            format!("{}={}", cookie.key.clone(), cookie.value.clone()),
                        )
                    } else {
                        ("".to_string(), "".to_string())
                    }
                })
                .collect::<Vec<(String, String)>>()
        })
        .filter(|(s, _)| !s.is_empty())
        .collect::<Vec<(String, String)>>();

    debug!("matched cookies: {:?}", cookies);

    let maybe_body = resolved_request
        .body
        .as_ref()
        .map(|b| serde_json::to_string(&b).unwrap());

    Url::parse(&resolved_request.url)
        .map_err(|e| Box::<dyn Error + Send + Sync>::from(format!("invalid request url: {}", e)))
        .and_then(|url| {
            let builder = Request::builder()
                .uri(url.as_str())
                .method(resolved_request.method.to_hyper())
                .header("Content-Type", HeaderValue::from_static("application/json"))
                .header(
                    "Content-Length",
                    HeaderValue::from(maybe_body.as_ref().map(|s| s.len()).unwrap_or_default()),
                );

            cookies
                .iter()
                .chain(resolved_request.headers.iter())
                .fold(builder, |builder, (k, v)| {
                    builder.header(k, variable_resolver(v.clone()))
                })
                .body(maybe_body.map(Body::from).unwrap_or(Body::empty()))
                .map_err(|e| Box::from(format!("bad request result: {}", e)))
        })
}

async fn process_request(
    state: &mut State,
    resolved_request: test::definition::ResolvedRequest,
) -> Result<hyper::Response<Body>, Box<dyn Error + Send + Sync>> {
    let client = Client::builder().build::<_, Body>(HttpsConnector::new());
    debug!("url({})", resolved_request.url);

    match http_request_from_test_spec(state, resolved_request) {
        Ok(req) => {
            debug!("sending request: {req:?}");
            let response = client.request(req).await?;
            let cookies = response.headers().get_all("Set-Cookie");
            for c in cookies.iter() {
                let cookie_raw = StateCookie::new(c.to_str().unwrap().to_string());
                if let Some(cookie) = cookie_raw {
                    let cookie_fullpath: String = format!("{}{}", cookie.domain, cookie.path);

                    debug!("cookie in response: {}", &cookie_fullpath);

                    if !state.cookies.contains_key(&cookie_fullpath) {
                        state
                            .cookies
                            .insert(cookie_fullpath.clone(), HashMap::new());
                    }

                    let sub_map = state.cookies.get_mut(&cookie_fullpath).unwrap();

                    if !sub_map.contains_key(&cookie.key) {
                        sub_map.insert(cookie.key.clone(), cookie);
                    } else {
                        sub_map.get_mut(&cookie.key).unwrap().update(cookie);
                    }
                }
            }
            Ok(response)
        }
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
            &state.variables,
            &td.variables,
        );
        let setup_headers = td.get_setup_request_headers(iteration);
        let setup_body = td.get_request_body(
            &setup.request.body,
            &state.variables,
            &td.variables,
            iteration,
        );
        info!("setup: {} {}\n", setup_method, setup_url);
        if !setup_headers.is_empty() {
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
            if let Some(setup_response_status) = &r.status {
                info!(
                    "validate setup_response_status with defined_status: {:?}\n",
                    setup_response_status
                );
            }

            for v in &r.extract {
                info!(
                    "attempt to extract value from response: {} = valueOf({})\n",
                    v.name, v.field
                );
            }

            if !r.ignore.is_empty() {
                info!("prune fields from setup_response_body\n");
                for i in r.ignore.iter() {
                    info!("filter: {}\n", i);
                }
            }

            if let Some(b) = &r.body {
                if !r.ignore.is_empty() {
                    info!(
                        "validate filtered setup_response_body matches defined body: {:?}\n",
                        b.data
                    );
                } else {
                    info!(
                        "validate setup_response_body matches defined body: {:?}\n",
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
            &state.variables,
            &[&stage.variables[..], &td.variables[..]].concat(),
        );
        let stage_headers = td.get_headers(&stage.request.headers, iteration);
        let stage_body = td.get_request_body(
            &stage.request.body,
            &state.variables,
            &[&stage.variables[..], &td.variables[..]].concat(),
            iteration,
        );
        info!(
            "stage {}: {} {}\n",
            stage_index + 1,
            stage_method,
            stage_url
        );
        if !stage_headers.is_empty() {
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
            if let Some(stage_response_status) = &r.status {
                info!(
                    "validate response_status with defined_status: {:?}\n",
                    stage_response_status
                );
            }

            for v in &r.extract {
                info!(
                    "attempt to extract value from response: {} = valueOf({})\n",
                    v.name, v.field
                );
            }

            if !r.ignore.is_empty() {
                info!("prune fields from response_body\n");
                for i in r.ignore.iter() {
                    info!("filter: {}\n", i);
                }
            }

            if let Some(b) = &r.body {
                if !r.ignore.is_empty() {
                    info!(
                        "validate filtered response_body matches defined body: {:?}\n",
                        b.data
                    );
                } else {
                    info!(
                        "validate response_body matches defined body: {:?}\n",
                        b.data
                    );
                }
            }
        }

        if let Some(stage_compare) = &stage.compare {
            // construct compare block
            let params = stage.get_compare_parameters();

            let compare_url = &td.get_url(
                iteration,
                &stage_compare.url,
                &params,
                &state.variables,
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
                    let key_search = format!("${{{}}}", gv.0);
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

            if !stage_compare_headers.is_empty() {
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
                if !r.ignore.is_empty() {
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
        let onsuccess_url = &td.get_url(
            iteration,
            &onsuccess.url,
            &onsuccess.params,
            &state.variables,
            &td.variables,
        );
        let onsuccess_headers = td.get_setup_request_headers(iteration);
        let onsuccess_body =
            td.get_request_body(&onsuccess.body, &state.variables, &td.variables, iteration);
        info!("onsuccess: {} {}\n", onsuccess_method, onsuccess_url);
        if !onsuccess_headers.is_empty() {
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
        let onfailure_url = &td.get_url(
            iteration,
            &onfailure.url,
            &onfailure.params,
            &state.variables,
            &td.variables,
        );
        let onfailure_headers = td.get_setup_request_headers(iteration);
        let onfailure_body =
            td.get_request_body(&onfailure.body, &state.variables, &td.variables, iteration);
        info!("onfailure: {} {}\n", onfailure_method, onfailure_url);
        if !onfailure_headers.is_empty() {
            info!("onfailure_headers:\n");
            for (key, value) in onfailure_headers.iter() {
                info!("-- {}: {}\n", key, value);
            }
        }

        if let Some(body) = onfailure_body {
            info!("onfailure_body: {}\n", body);
        }
    }

    if let Some(request) = &td.cleanup.always {
        info!("run cleanup requests:\n");
        let cleanup_method = request.method.as_method();
        let cleanup_url = &td.get_url(
            iteration,
            &request.url,
            &request.params,
            &state.variables,
            &td.variables,
        );
        let cleanup_headers = td.get_setup_request_headers(iteration);
        let cleanup_body =
            td.get_request_body(&request.body, &state.variables, &td.variables, iteration);
        info!("cleanup: {} {}\n", cleanup_method, cleanup_url);
        if !cleanup_headers.is_empty() {
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

#[cfg(test)]
mod tests {
    use crate::test::file::NumericSpecification;
    use crate::test::file::Specification;

    use self::test::definition::ResolvedRequest;
    use hyper::Response;
    use std::any::Any;

    use super::*;
    use adjacent_pair_iterator::AdjacentPairIterator;
    use hyper::StatusCode;
    use nonempty_collections::*;
    use serde_json::json;

    #[test]
    fn process_response_multiple_failures() {
        let expected = ExpectedResultData {
            status: Some(ValueOrNumericSpecification::Value(200)),
            body: Some(BodyOrSchema::Body(json!({
                "Name" : "Bob"
            }))),
            headers: Vec::default(),
            ..ExpectedResultData::new()
        };

        let ignore_body: [String; 0] = [];
        let actual = process_response(
            0,
            StageType::Normal,
            None,
            0,
            ResultDetails {
                request: RequestDetails {
                    body: serde_json::Value::default(),
                    headers: Vec::default(),
                    method: http::Verb::Post.as_method(),
                    url: "".to_string(),
                },
                expected: expected.clone(),
                actual: Option::from(ResponseResultData {
                    body: serde_json::Value::default(),
                    status: 200,
                    headers: Vec::default(),
                }),
                compare_request: Some(RequestDetails {
                    body: serde_json::Value::default(),
                    headers: Vec::default(),
                    method: http::Verb::Post.as_method(),
                    url: "".to_string(),
                }),
                compare_actual: Some(ResponseResultData {
                    body: json!({
                        "Name" : "Bob"
                    }),
                    status: 200,
                    headers: Vec::default(),
                }),
            },
            &ignore_body,
            None,
            None,
        );
        assert_eq!(actual.status, TestStatus::Failed);
        assert_eq!(actual.validation, Validated::Fail(nev![
            String::from("Expected body {\"Name\":\"Bob\"} did not match actual body null ; json atoms at path \"(root)\" are not equal:\n    lhs:\n        null\n    rhs:\n        {\n          \"Name\": \"Bob\"\n        }"),
            String::from("Expected compare body {\"Name\":\"Bob\"} did not match actual body null ; json atoms at path \"(root)\" are not equal:\n    lhs:\n        null\n    rhs:\n        {\n          \"Name\": \"Bob\"\n        }")
        ]));
    }

    #[test]
    fn process_response_no_result() {
        let expected = ExpectedResultData {
            status: Some(ValueOrNumericSpecification::Value(1)), //bc we coalesce status to 0 in ResultData::from_request
            body: None,
            headers: Vec::default(),
            ..ExpectedResultData::new()
        };

        let ignore_body: [String; 0] = [];
        let actual = process_response(
            0,
            StageType::Normal,
            None,
            0,
            ResultDetails {
                request: RequestDetails {
                    body: serde_json::Value::default(),
                    headers: Vec::default(),
                    method: http::Verb::Post.as_method(),
                    url: "".to_string(),
                },
                expected: expected.clone(),
                actual: None,
                compare_request: None,
                compare_actual: None,
            },
            &ignore_body,
            None,
            None,
        );
        assert_eq!(actual.status, TestStatus::Failed);
        assert!(actual.validation.is_fail());
        assert_eq!(
            actual.validation,
            Validated::fail("failed to get response".to_string())
        );
    }

    //note : no test for headers, we don't currently support it
    #[test]
    fn process_response_body_mismatch() {
        let expected = ExpectedResultData {
            status: Some(ValueOrNumericSpecification::Value(200)),
            body: Some(BodyOrSchema::Body(json!({
                "Name" : "Bob"
            }))),
            headers: Vec::default(),
            ..ExpectedResultData::new()
        };

        let ignore_body: [String; 0] = [];
        let actual = process_response(
            0,
            StageType::Normal,
            None,
            0,
            ResultDetails {
                request: RequestDetails {
                    body: serde_json::Value::default(),
                    headers: Vec::default(),
                    method: http::Verb::Post.as_method(),
                    url: "".to_string(),
                },
                expected: expected.clone(),
                actual: Some(ResponseResultData {
                    body: serde_json::Value::default(),
                    headers: Vec::default(),
                    status: 200,
                }),
                compare_request: None,
                compare_actual: None,
            },
            &ignore_body,
            None,
            None,
        );
        assert_eq!(actual.status, TestStatus::Failed);
        assert_eq!(actual.validation, Validated::fail(
            String::from("Expected body {\"Name\":\"Bob\"} did not match actual body null ; json atoms at path \"(root)\" are not equal:\n    lhs:\n        null\n    rhs:\n        {\n          \"Name\": \"Bob\"\n        }"
        )));
    }

    #[test]
    fn process_response_body_match() {
        let expected = ExpectedResultData {
            status: Some(ValueOrNumericSpecification::Value(200)),
            body: Some(BodyOrSchema::Body(json!({
                "Name" : "Bob"
            }))),
            headers: Vec::default(),
            ..ExpectedResultData::new()
        };

        let ignore_body: [String; 0] = [];
        let actual = process_response(
            0,
            StageType::Normal,
            None,
            0,
            ResultDetails {
                request: RequestDetails {
                    body: serde_json::Value::default(),
                    headers: Vec::default(),
                    method: http::Verb::Post.as_method(),
                    url: "".to_string(),
                },
                expected: expected.clone(),
                actual: Some(ResponseResultData {
                    status: 200,
                    body: json!({
                        "Name": "Bob"
                    }),
                    headers: Vec::default(),
                }),
                compare_request: None,
                compare_actual: None,
            },
            &ignore_body,
            None,
            None,
        );
        assert_eq!(actual.status, TestStatus::Passed);
        assert!(actual.validation.is_good());
    }

    #[test]
    fn process_response_status_match() {
        let expected = ExpectedResultData {
            status: Some(ValueOrNumericSpecification::Schema(NumericSpecification {
                specification: Some(Specification::OneOf(vec![200, 201, 202])),
                min: None,
                max: None,
            })),
            body: None,
            headers: Vec::default(),
            ..ExpectedResultData::new()
        };

        let ignore_body: [String; 0] = [];
        let actual = process_response(
            0,
            StageType::Normal,
            None,
            0,
            ResultDetails {
                request: RequestDetails {
                    body: serde_json::Value::default(),
                    headers: Vec::default(),
                    method: http::Verb::Post.as_method(),
                    url: "".to_string(),
                },
                expected: expected.clone(),
                actual: Some(ResponseResultData {
                    body: serde_json::Value::default(),
                    headers: Vec::default(),
                    status: 200,
                }),
                compare_request: None,
                compare_actual: None,
            },
            &ignore_body,
            None,
            None,
        );
        assert_eq!(actual.status, TestStatus::Passed);
        assert!(actual.validation.is_good());
    }

    #[test]
    fn process_response_status_mismatch() {
        let expected = ExpectedResultData {
            status: Some(ValueOrNumericSpecification::Value(200)),
            body: None,
            headers: Vec::default(),
            ..ExpectedResultData::new()
        };

        let ignore_body: [String; 0] = [];
        let actual = process_response(
            0,
            StageType::Normal,
            None,
            0,
            ResultDetails {
                request: RequestDetails {
                    body: serde_json::Value::default(),
                    headers: Vec::default(),
                    method: http::Verb::Post.as_method(),
                    url: "".to_string(),
                },
                expected: expected.clone(),
                actual: Option::from(ResponseResultData {
                    status: 500,
                    body: serde_json::Value::default(),
                    headers: Vec::default(),
                }),
                compare_request: None,
                compare_actual: None,
            },
            &ignore_body,
            None,
            None,
        );
        assert_eq!(actual.status, TestStatus::Failed);
        assert_eq!(
            actual.validation,
            Validated::fail("Expected status code 200 but received 500".to_string())
        );
    }

    #[tokio::test]
    async fn from_bad_response() {
        let rep = Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body(Body::empty());

        let result = ResponseResultData::from_response(rep.unwrap()).await;
        assert_eq!(400, result.as_ref().unwrap().status);
    }

    #[tokio::test]
    async fn from_response_object_body() {
        let val = json!({
            "name": "John Doe",
            "age": 43
        });

        let rep = Response::builder()
            .status(StatusCode::OK)
            .body(Body::from(val.to_string()));

        let result = ResponseResultData::from_response(rep.unwrap()).await;
        assert_eq!(200, result.as_ref().unwrap().status);
        assert_eq!(val.to_string(), result.as_ref().unwrap().body.to_string());
    }

    #[tokio::test]
    async fn from_response_string_body() {
        let rep = Response::builder()
            .status(StatusCode::OK)
            //notice, serde will only capture it if its quoted
            //could we detect this and possibly account for it?
            .body(Body::from("\"ok;\""));

        let result = ResponseResultData::from_response(rep.unwrap()).await;
        assert_eq!(200, result.as_ref().unwrap().status);
        assert_eq!("ok;", result.as_ref().unwrap().body.as_str().unwrap());
    }

    #[tokio::test]
    async fn from_response_empty_body() {
        let rep = Response::builder()
            .header("foo", "bar")
            .status(StatusCode::OK)
            .body(Body::empty());

        let result = ResponseResultData::from_response(rep.unwrap()).await;
        assert_eq!(200, result.as_ref().unwrap().status);
        assert_eq!(1, result.as_ref().unwrap().headers.len());
        assert!(result.as_ref().unwrap().body.is_null());
    }

    #[test]
    fn http_request_from_test_spec_post() {
        let mut state = State {
            variables: HashMap::new(),
            cookies: HashMap::new(),
        };
        state
            .variables
            .insert("MY_VARIABLE".to_string(), "foo".to_string());
        state
            .variables
            .insert("MY_VARIABLE2".to_string(), "bar".to_string());

        let body = serde_json::json!({ "an": "object" });
        let res = http_request_from_test_spec(
            &state,
            ResolvedRequest::new(
                "https://google.com".to_string(),
                http::Verb::Post.as_method(),
                vec![(
                    "header".to_string(),
                    "${MY_VARIABLE}-${MY_VARIABLE2}".to_string(),
                )],
                Some(body),
            ),
        );
        let expected: Request<()> = Request::default();
        assert_ne!(expected.type_id(), res.as_ref().unwrap().body().type_id());

        assert_eq!(3, res.as_ref().unwrap().headers().len());

        assert_eq!(
            "foo-bar",
            res.as_ref().unwrap().headers().get("header").unwrap()
        );
    }

    fn construct_definition_for_dependency_graph(
        id: &str,
        requires: Option<String>,
    ) -> test::Definition {
        test::Definition {
            name: None,
            description: None,
            id: String::from(id),
            project: None,
            environment: None,
            requires,
            tags: vec![String::from("myTag"), String::from("myTag2")],
            iterate: 0,
            variables: Vec::new(),
            global_variables: Vec::new(),
            stages: Vec::new(),
            setup: None,
            cleanup: definition::CleanupDescriptor {
                onsuccess: None,
                onfailure: None,
                always: None,
            },
            disabled: false,
            filename: "/a/path.jkt".to_string(),
        }
    }

    #[test]
    fn no_dependencies_is_one_execution_node() {
        let defs = vec!["A", "B", "C", "D"]
            .into_iter()
            .map(|id| construct_definition_for_dependency_graph(id, None))
            .collect();

        let actual = construct_test_execution_graph_v2(
            defs,
            vec![construct_definition_for_dependency_graph("E", None)],
        );
        assert_eq!(1, actual.len());
        assert_eq!(4, actual.get(0).unwrap().len());
    }

    #[test]
    fn one_root_dependency_is_two_execution_nodes() {
        let mut defs = vec!["A", "B", "C", "D"]
            .into_iter()
            .map(|id| construct_definition_for_dependency_graph(id, Some("Parent".to_string())))
            .collect::<Vec<Definition>>();

        defs.push(construct_definition_for_dependency_graph("Parent", None));

        let actual = construct_test_execution_graph_v2(
            defs,
            vec![construct_definition_for_dependency_graph("E", None)],
        );

        assert_eq!(2, actual.len());
        assert_eq!(1, actual.get(0).unwrap().len());
        assert_eq!("Parent", actual.get(0).unwrap().get(0).unwrap().id);
        assert_eq!(4, actual.get(1).unwrap().len());
    }

    #[test]
    fn straight_line_dependency_is_node_chain() {
        let defs = vec!["A", "B", "C", "D"]
            .adjacent_pairs()
            .into_iter()
            .enumerate()
            .map(|(pos, (fst, snd))| {
                let mut res: Vec<Definition> = Vec::new();
                if pos == 0 {
                    res.push(construct_definition_for_dependency_graph(fst, None));
                }

                res.push(construct_definition_for_dependency_graph(
                    snd,
                    Some(fst.to_string()),
                ));

                return res;
            })
            .flatten()
            .collect::<Vec<Definition>>();

        let actual = construct_test_execution_graph_v2(defs, Vec::new());

        assert_eq!(4, actual.len());
    }

    fn default_definition_for_filtering() -> test::Definition {
        test::Definition {
            name: None,
            description: None,
            id: String::from("id"),
            project: None,
            environment: None,
            requires: None,
            tags: vec![String::from("myTag"), String::from("myTag2")],
            iterate: 0,
            variables: Vec::new(),
            global_variables: Vec::new(),
            stages: Vec::new(),
            setup: None,
            cleanup: definition::CleanupDescriptor {
                onsuccess: None,
                onfailure: None,
                always: None,
            },
            disabled: false,
            filename: "/a/path.jkt".to_string(),
        }
    }

    #[test]
    fn or_filter_not_exists() {
        let test_definition = default_definition_for_filtering();
        let tags = vec![String::from("nonexistant")];
        let tag_mode = TagMode::AND;
        assert_eq!(
            true,
            ignored_due_to_tag_filter(&test_definition, &tags, &tag_mode)
        );
    }

    #[test]
    fn or_filter_exists() {
        let test_definition = default_definition_for_filtering();
        let tags = vec![String::from("myTag")];
        let tag_mode = TagMode::AND;
        assert_eq!(
            false,
            ignored_due_to_tag_filter(&test_definition, &tags, &tag_mode)
        );
    }

    #[test]
    fn and_filter_not_exists() {
        let test_definition = default_definition_for_filtering();
        let tags = vec![String::from("nonexistant")];
        let tag_mode = TagMode::AND;
        assert_eq!(
            true,
            ignored_due_to_tag_filter(&test_definition, &tags, &tag_mode)
        );
    }

    #[test]
    fn and_filter_partial_match() {
        let test_definition = default_definition_for_filtering();
        let tags = vec![String::from("myTag"), String::from("nonexistant")];
        let tag_mode = TagMode::AND;
        assert_eq!(
            true,
            ignored_due_to_tag_filter(&test_definition, &tags, &tag_mode)
        );
    }

    #[test]
    fn and_filter_match() {
        let test_definition = default_definition_for_filtering();
        let tags = vec![String::from("myTag"), String::from("myTag2")];
        let tag_mode = TagMode::AND;
        assert_eq!(
            false,
            ignored_due_to_tag_filter(&test_definition, &tags, &tag_mode)
        );
    }

    #[test]
    fn and_filter_exists() {
        let test_definition = default_definition_for_filtering();
        let tags = vec![String::from("myTag")];
        let tag_mode = TagMode::AND;
        assert_eq!(
            false,
            ignored_due_to_tag_filter(&test_definition, &tags, &tag_mode)
        );
    }

    #[test]
    fn empty_execution_result_is_all_skips() {
        let execution_result = ExecutionResult {
            test_results: vec![TestResult {
                test_name: "name".to_string(),
                iteration_results: vec![],
            }],
        };

        let report = Report::from(execution_result);
        assert_eq!(0, report.failed);

        assert_eq!(0, report.passed);

        assert_eq!(1, report.skipped);
    }
} //mod tests
