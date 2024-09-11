use crate::config;
use crate::errors::TelemetryError;
use crate::executor;
use crate::executor::ResultDetails;
use crate::machine;
use crate::test;
use crate::test::definition::RequestDescriptor;
use crate::test::http::Header;
use crate::test::Definition;
use hyper::header::HeaderValue;
use hyper::{body, Body, Client, Request};
use hyper_tls::HttpsConnector;
use log::{debug, trace};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::env;
use std::error::Error;
use ulid::Ulid;
use url::Url;
use uuid::Uuid;

const TELEMETRY_BASE_URL: &str = "https://ingestion.jikken.io/v1";
const TELEMETRY_DEV_BASE_URL: &str = "https://dev-ingestion.jikken.io/v1";

#[derive(Clone)]
pub struct Session {
    pub token: Uuid,
    pub session_id: Uuid,
    // pub start_time: chrono::DateTime<chrono::Utc>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SessionPost<'a> {
    pub version: String,
    pub os: String,
    pub machine_id: String,
    pub tests: u32,
    pub args: serde_json::Value,
    pub validation: serde_json::Value,
    pub config: serde_json::Value,
    pub environments: Vec<&'a str>,
    pub projects: Vec<&'a str>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SessionCompletedPost {
    pub runtime: u32,
    pub status: u32,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SessionResponse {
    pub session_id: String,
    pub identifier: Option<ulid::Ulid>,
}

#[derive(Clone)]
pub struct Test {
    pub test_id: Uuid,
    pub session: Session,
    // pub start_time: chrono::DateTime<chrono::Utc>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct TestPost {
    pub session_id: String,
    pub identifier: Option<String>,
    pub platform_id: ulid::Ulid,
    pub definition: serde_json::Value,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct TestCompletedPost {
    pub session_id: String,
    pub iteration: u32,
    pub stage: u32,
    pub stage_type: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stage_name: Option<String>,
    pub status: u32,
    pub runtime: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub environment: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct TestResponse {
    pub test_id: String,
}

fn redact_string(_val: &str) -> String {
    static REDACTED_VALUE: &str = "*********";
    REDACTED_VALUE.to_string()
}

fn redact_headers(headers: &mut [Header]) {
    let should_redact = |s: &str| s.to_lowercase() == "authorization";

    headers
        .iter_mut()
        .filter(|h| should_redact(h.header.as_str()))
        .for_each(|h| {
            h.value = redact_string(&h.value);
        });
}

fn redact_request(req: &mut RequestDescriptor) {
    redact_headers(req.headers.as_mut())
}

//temporal values will result in a new hash for test
//definition which means a new test
fn remove_temporal_values(td: test::Definition) -> test::Definition {
    test::Definition {
        global_variables: vec![],
        ..td
    }
}

fn prune_definition(td: test::Definition) -> test::Definition {
    remove_temporal_values(td)
}

fn redact_definition(mut td: test::Definition) -> test::Definition {
    _ = td.setup.as_mut().map(|s| redact_request(&mut s.request));

    _ = td.cleanup.always.as_mut().map(redact_request);

    _ = td.cleanup.onfailure.as_mut().map(redact_request);

    _ = td.cleanup.onsuccess.as_mut().map(redact_request);

    td.stages.iter_mut().for_each(|s| {
        redact_request(&mut s.request);
        if let Some(c) = s.compare.as_mut() {
            redact_headers(c.headers.as_mut())
        };
    });

    td
}

fn redact_result_details(mut rd: ResultDetails) -> ResultDetails {
    redact_headers(rd.request.headers.as_mut());
    if let Some(c) = rd.compare_request.as_mut() {
        redact_headers(c.headers.as_mut())
    }
    rd
}

fn get_url(url: &str, config: &config::Config) -> String {
    format!(
        "{}{}",
        if config.settings.dev_mode.unwrap_or(false) {
            TELEMETRY_DEV_BASE_URL
        } else {
            TELEMETRY_BASE_URL
        },
        url
    )
}

fn get_config(config: &config::Config) -> serde_json::Value {
    let global_variables = config.generate_global_variables();
    let mut config_json = serde_json::to_value(config).unwrap();
    config_json.as_object_mut().unwrap().insert(
        "globals".to_string(),
        serde_json::to_value(&global_variables).unwrap(),
    );
    config_json
}

pub async fn create_session(
    token: Uuid,
    tests: Vec<&Definition>,
    args_json: Box<serde_json::Value>,
    config: &config::Config,
) -> Result<Session, Box<dyn Error + Send + Sync>> {
    let client = Client::builder().build::<_, Body>(HttpsConnector::new());
    let uri = get_url("/sessions", config);
    trace!("telemetry session url({})", uri);
    match Url::parse(&uri) {
        Ok(_) => {}
        Err(error) => {
            return Err(Box::from(format!("invalid telemetry url: {}", error)));
        }
    }

    let validation_json = serde_json::json!({}); // todo: add validation report once validation is implemented
    let config_json = get_config(config);

    let m = machine::new();
    let machine_id = m.generate_machine_id();

    let mut test_count: u32 = 0;
    let mut environments: HashSet<&str> = HashSet::new();
    let mut projects: HashSet<&str> = HashSet::new();

    tests.iter().for_each(|t| {
        test_count += t.iterate;
        if let Some(env) = &t.environment {
            environments.insert(env);
        }
        if let Some(project) = &t.project {
            projects.insert(project);
        }
    });

    let post_body = SessionPost {
        version: crate::VERSION.to_string(),
        os: env::consts::OS.to_string(),
        machine_id,
        tests: test_count,
        args: *args_json,
        validation: validation_json,
        config: config_json,
        environments: Vec::from_iter(environments),
        projects: Vec::from_iter(projects),
    };

    let post_string = serde_json::to_string(&post_body)?;
    trace!("telemetry_body: {}", post_string);

    let request = Request::builder()
        .uri(uri)
        .method("POST")
        .header("Authorization", token.to_string())
        .header("Content-Type", HeaderValue::from_static("application/json"))
        .body(Body::from(post_string));

    if let Ok(req) = request {
        let response = client.request(req).await?;
        let status = response.status();

        if status.as_u16() != 201 {
            // session creation failed
            debug!("session creation failed: status({})", status);
            return Err(Box::from(TelemetryError {
                reason: "session creation failed".to_string(),
            }));
        }

        let (_, body) = response.into_parts();

        let response_bytes = body::to_bytes(body).await?;
        let response: SessionResponse = serde_json::from_slice(response_bytes.as_ref())?;
        let session_id = uuid::Uuid::parse_str(&response.session_id)?;

        return Ok(Session {
            token,
            session_id,
            // start_time: chrono::Utc::now(),
        });
    }

    Err(Box::from(TelemetryError {
        reason: "invalid session request".to_string(),
    }))
}

pub async fn create_test(
    session: &Session,
    definition: test::Definition,
    config: &config::Config,
) -> Result<Test, Box<dyn Error + Send + Sync>> {
    let client = Client::builder().build::<_, Body>(HttpsConnector::new());
    let uri = get_url("/tests", config);
    trace!("telemetry test url({})", uri);
    match Url::parse(&uri) {
        Ok(_) => {}
        Err(error) => {
            return Err(Box::from(format!("invalid telemetry url: {}", error)));
        }
    }

    let redacted_definition = redact_definition(definition.clone());
    let pruned_definition = prune_definition(redacted_definition);
    let definition_json = serde_json::to_value(&pruned_definition)?;

    let ulid = Ulid::from_string(&definition.platform_id.expect("Platform ID is required"))
        .expect("Platform ID must be valid");

    let post_body = TestPost {
        session_id: session.session_id.to_string(),
        identifier: pruned_definition.id,
        platform_id: ulid,
        definition: definition_json,
    };

    let post_string = serde_json::to_string(&post_body)?;
    trace!("telemetry_body: {}", post_string);

    let request = Request::builder()
        .uri(uri)
        .method("POST")
        .header("Authorization", session.token.to_string())
        .header("Content-Type", HeaderValue::from_static("application/json"))
        .body(Body::from(post_string));

    if let Ok(req) = request {
        let response = client.request(req).await?;
        let status = response.status();

        if status.as_u16() != 201 {
            // session creation failed
            debug!("test creation failed: status({})", status);
            return Err(Box::from(TelemetryError {
                reason: "test creation failed".to_string(),
            }));
        }

        let (_, body) = response.into_parts();

        let response_bytes = body::to_bytes(body).await?;
        let response: TestResponse = serde_json::from_slice(response_bytes.as_ref())?;
        let test_id = uuid::Uuid::parse_str(&response.test_id)?;

        return Ok(Test {
            test_id,
            session: session.clone(),
            // start_time: chrono::Utc::now(),
        });
    }

    Err(Box::from(TelemetryError {
        reason: "invalid test request".to_string(),
    }))
}

pub async fn complete_stage(
    td: &test::Definition,
    test: &Test,
    iteration: u32,
    stage: &executor::StageResult,
    config: &config::Config,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let client = Client::builder().build::<_, Body>(HttpsConnector::new());
    let uri = get_url(&format!("/tests/{}/completed", test.test_id), config);
    trace!("telemetry test url({})", uri);
    match Url::parse(&uri) {
        Ok(_) => {}
        Err(error) => {
            return Err(Box::from(format!("invalid telemetry url: {}", error)));
        }
    }

    let details_json = serde_json::to_value(redact_result_details(stage.details.clone()))?;
    let post_data = TestCompletedPost {
        session_id: test.session.session_id.to_string(),
        iteration,
        stage: stage.stage,
        stage_type: stage.stage_type.clone() as u32,
        stage_name: stage.stage_name.clone(),
        status: stage.status.clone() as u32,
        runtime: stage.runtime,
        details: Some(details_json),
        project: stage.project.clone(),
        environment: stage.environment.clone(),
    };

    let post_body = serde_json::to_value(post_data)?;

    let mut post_string = serde_json::to_string(&post_body)?;
    post_string = td.redact_secrets(&post_string);
    trace!("telemetry_body: {}", post_string);

    let request = Request::builder()
        .uri(&uri)
        .method("POST")
        .header("Authorization", test.session.token.to_string())
        .header("Content-Type", HeaderValue::from_static("application/json"))
        .body(Body::from(post_string));

    if let Ok(req) = request {
        let response = client.request(req).await?;
        let status = response.status();

        if status.as_u16() != 201 {
            // session creation failed
            debug!("test stage completion failed: status({})", status);
            return Err(Box::from(TelemetryError {
                reason: "test stage completion failed".to_string(),
            }));
        }
    } else {
        return Err(Box::from(TelemetryError {
            reason: "invalid test request".to_string(),
        }));
    }

    Ok(())
}

pub async fn complete_stage_skipped(
    test: &Test,
    test_definition: &test::Definition,
    config: &config::Config,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let client = Client::builder().build::<_, Body>(HttpsConnector::new());
    let uri = get_url(&format!("/tests/{}/completed", test.test_id), config);
    trace!("telemetry complete stage url: {}", uri);
    if let Err(error) = Url::parse(&uri) {
        return Err(Box::from(format!("invalid telemetry url: {}", error)));
    }

    let post_data = TestCompletedPost {
        session_id: test.session.session_id.to_string(),
        iteration: 0,
        stage: 0,
        stage_type: 2, // Normal
        stage_name: None,
        status: 5, // Skipped
        runtime: 0,
        details: None,
        project: test_definition.project.clone(),
        environment: test_definition.environment.clone(),
    };

    let post_body = serde_json::to_value(post_data)?;
    let post_string = serde_json::to_string(&post_body)?;
    trace!("telemetry body: {}", post_string);

    let request = Request::builder()
        .uri(&uri)
        .method("POST")
        .header("Authorization", test.session.token.to_string())
        .header("Content-Type", HeaderValue::from_static("application/json"))
        .body(Body::from(post_string));

    if let Ok(req) = request {
        let response = client.request(req).await?;
        let status = response.status();

        if status.as_u16() != 201 {
            debug!("test stage completion failed: status({})", status);
            return Err(Box::from(TelemetryError {
                reason: "test stage completion failed".to_string(),
            }));
        }
    } else {
        return Err(Box::from(TelemetryError {
            reason: "invalid test request".to_string(),
        }));
    }

    Ok(())
}

pub async fn complete_session(
    session: &Session,
    runtime: u32,
    status: u32,
    config: &config::Config,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let client = Client::builder().build::<_, Body>(HttpsConnector::new());
    let uri = get_url(
        &format!("/sessions/{}/completed", session.session_id),
        config,
    );
    trace!("telemetry session url({})", uri);
    match Url::parse(&uri) {
        Ok(_) => {}
        Err(error) => {
            return Err(Box::from(format!("invalid telemetry url: {}", error)));
        }
    }

    let post_body = SessionCompletedPost { runtime, status };

    let post_string = serde_json::to_string(&post_body)?;
    trace!("telemetry_body: {}", post_string);

    let request = Request::builder()
        .uri(uri)
        .method("POST")
        .header("Authorization", session.token.to_string())
        .header("Content-Type", HeaderValue::from_static("application/json"))
        .body(Body::from(post_string));

    if let Ok(req) = request {
        let response = client.request(req).await?;
        let status = response.status();

        if status.as_u16() != 200 {
            // session creation failed
            debug!("session completion failed: status({})", status);
            return Err(Box::from(TelemetryError {
                reason: "session completion failed".to_string(),
            }));
        }
    }

    Ok(())
}

pub enum PlatformIdFailure {
    Missing,
    Invalid,
    Duplicate,
}

pub fn validate_platform_ids(
    definitions: Vec<&Definition>,
) -> Result<(), Vec<(&Definition, PlatformIdFailure)>> {
    let mut failures: Vec<(&Definition, PlatformIdFailure)> = Vec::new();
    let mut duplicate_check: HashSet<Ulid> = HashSet::new();

    for definition in definitions {
        let Some(id_raw) = definition.platform_id.as_ref() else {
            failures.push((definition, PlatformIdFailure::Missing));
            continue;
        };

        let Ok(id) = Ulid::from_string(&id_raw) else {
            failures.push((definition, PlatformIdFailure::Invalid));
            continue;
        };

        if duplicate_check.contains(&id) {
            failures.push((definition, PlatformIdFailure::Duplicate));
        }

        duplicate_check.insert(id);
    }

    if failures.len() > 0 {
        return Err(failures);
    }

    Ok(())
}

#[cfg(test)]
mod tests {

    use test::Variable;

    use crate::{
        executor::ExpectedResultData,
        test::definition::{CompareDescriptor, RequestResponseDescriptor, StageDescriptor},
    };

    use self::executor::RequestDetails;

    use super::*;
    fn headers_factory() -> Vec<Header> {
        vec![
            Header {
                header: "AuthorIzation".to_string(), //dumb casing on purpose :)
                value: "super_secret_key".to_string(),
                matches_variable: false.into(),
            },
            Header {
                header: "Foo".to_string(),
                value: "Bar".to_string(),
                matches_variable: false.into(),
            },
        ]
    }

    #[test]
    fn redact_string_redacts() {
        assert_eq!("*********", redact_string("foo"));
    }

    #[test]
    fn redact_headers_redacts() {
        let mut headers = headers_factory();

        redact_headers(&mut headers);
        assert!(!headers.iter().any(|h| h.value == "super_secret_key"));
    }

    #[test]
    fn redact_definition_redacts() {
        let request = RequestDescriptor {
            method: test::http::Verb::Get,
            body: None,
            headers: headers_factory(),
            params: vec![],
            url: "foo".to_string(),
        };

        let td = test::Definition {
            name: None,
            description: None,
            id: None,
            platform_id: None,
            project: None,
            environment: None,
            requires: None,
            tags: Vec::new(),
            iterate: 0,
            variables: Vec::new(),
            global_variables: Vec::new(),
            stages: vec![StageDescriptor {
                name: None,
                response: None,
                // source_path: "".to_string(),
                variables: vec![],
                request: request.clone(),
                compare: Some(CompareDescriptor {
                    add_headers: vec![],
                    method: test::http::Verb::Get,
                    add_params: vec![],
                    body: None,
                    headers: headers_factory(),
                    url: "foo2".to_string(),
                    ignore_headers: vec![],
                    ignore_params: vec![],
                    params: vec![],
                    strict: true,
                }),
                delay: None,
            }],
            setup: Some(RequestResponseDescriptor {
                response: None,
                request: request.clone(),
            }),
            cleanup: test::definition::CleanupDescriptor {
                onsuccess: Some(request.clone()),
                onfailure: Some(request.clone()),
                always: Some(request.clone()),
            },
            disabled: false,
            filename: "/a/path.jkt".to_string(),
            index: 0,
        };

        let get_all_headers = |td: test::Definition| -> Vec<Header> {
            td.cleanup
                .always
                .unwrap()
                .headers
                .into_iter()
                .chain(td.cleanup.onfailure.unwrap().headers.into_iter())
                .chain(td.cleanup.onsuccess.unwrap().headers.into_iter())
                .chain(
                    td.stages
                        .into_iter()
                        .map(|s| {
                            s.request
                                .headers
                                .into_iter()
                                .chain(s.compare.unwrap().headers.into_iter())
                        })
                        .flatten(),
                )
                .chain(td.setup.unwrap().request.headers.into_iter())
                .collect()
        };

        let redacted_td = redact_definition(td.clone());
        let redacted_headers = get_all_headers(redacted_td);
        let nonredacted_headers = get_all_headers(td);

        assert!(nonredacted_headers
            .iter()
            .any(|h| h.value == "super_secret_key"));

        assert!(!redacted_headers
            .iter()
            .any(|h| h.value == "super_secret_key"));
    }

    #[test]
    fn redact_resultdetails_redacts() {
        let rd = RequestDetails {
            body: serde_json::Value::Null,
            headers: headers_factory(),
            url: "".to_string(),
            method: test::http::Verb::Get.as_method(),
        };

        let redacted = redact_result_details(ResultDetails {
            actual: None,
            request: rd.clone(),
            compare_actual: None,
            expected: ExpectedResultData {
                body: None,
                headers: vec![],
                status: None,
                ..ExpectedResultData::new()
            },
            compare_request: Some(rd.clone()),
        });

        assert!(!redacted
            .request
            .headers
            .iter()
            .chain(redacted.compare_request.unwrap().headers.iter())
            .any(|h| h.value == "super_secret_key"));
    }

    #[test]
    fn remove_temporal_values_has_globals() {
        let request = RequestDescriptor {
            method: test::http::Verb::Get,
            body: None,
            headers: headers_factory(),
            params: vec![],
            url: "foo".to_string(),
        };

        let td = test::Definition {
            name: None,
            description: None,
            id: None,
            platform_id: None,
            project: None,
            environment: None,
            requires: None,
            tags: Vec::new(),
            iterate: 0,
            variables: Vec::new(),
            global_variables: vec![Variable {
                name: "my_global".to_string(),
                source_path: "".to_string(),
                value: test::ValueOrDatumOrFileOrSecret::Value {
                    value: serde_json::Value::from("hello"),
                },
            }],
            stages: vec![StageDescriptor {
                name: None,
                response: None,
                // source_path: "".to_string(),
                variables: vec![],
                request: request.clone(),
                compare: Some(CompareDescriptor {
                    add_headers: vec![],
                    method: test::http::Verb::Get,
                    add_params: vec![],
                    body: None,
                    headers: headers_factory(),
                    url: "foo2".to_string(),
                    ignore_headers: vec![],
                    ignore_params: vec![],
                    params: vec![],
                    strict: true,
                }),
                delay: None,
            }],
            setup: Some(RequestResponseDescriptor {
                response: None,
                request: request.clone(),
            }),
            cleanup: test::definition::CleanupDescriptor {
                onsuccess: Some(request.clone()),
                onfailure: Some(request.clone()),
                always: Some(request.clone()),
            },
            disabled: false,
            filename: "/a/path.jkt".to_string(),
            index: 0,
        };
        let before = td.clone();
        let pruned = remove_temporal_values(td);
        assert_eq!(
            Definition {
                global_variables: vec![],
                ..before
            },
            pruned
        )
    }

    #[test]
    fn remove_temporal_values_has_no_globals() {
        let request = RequestDescriptor {
            method: test::http::Verb::Get,
            body: None,
            headers: headers_factory(),
            params: vec![],
            url: "foo".to_string(),
        };

        let td = test::Definition {
            name: None,
            description: None,
            id: None,
            platform_id: None,
            project: None,
            environment: None,
            requires: None,
            tags: Vec::new(),
            iterate: 0,
            variables: Vec::new(),
            global_variables: Vec::new(),
            stages: vec![StageDescriptor {
                name: None,
                response: None,
                // source_path: "".to_string(),
                variables: vec![],
                request: request.clone(),
                compare: Some(CompareDescriptor {
                    add_headers: vec![],
                    method: test::http::Verb::Get,
                    add_params: vec![],
                    body: None,
                    headers: headers_factory(),
                    url: "foo2".to_string(),
                    ignore_headers: vec![],
                    ignore_params: vec![],
                    params: vec![],
                    strict: true,
                }),
                delay: None,
            }],
            setup: Some(RequestResponseDescriptor {
                response: None,
                request: request.clone(),
            }),
            cleanup: test::definition::CleanupDescriptor {
                onsuccess: Some(request.clone()),
                onfailure: Some(request.clone()),
                always: Some(request.clone()),
            },
            disabled: false,
            filename: "/a/path.jkt".to_string(),
            index: 0,
        };
        let before = td.clone();
        let pruned = remove_temporal_values(td);
        assert_eq!(
            Definition {
                global_variables: vec![],
                ..before
            },
            pruned
        )
    }
}
