use crate::config;
use crate::errors::TelemetryError;
use crate::executor;
use crate::machine;
use crate::test;
use hyper::header::HeaderValue;
use hyper::{body, Body, Client, Request};
use hyper_tls::HttpsConnector;
use log::{debug, trace};
use serde::{Deserialize, Serialize};
use std::env;
use std::error::Error;
use url::Url;
use uuid::Uuid;

const TELEMETRY_BASE_URL: &str = "https://ingestion.jikken.io/v1";

#[derive(Clone)]
pub struct Session {
    pub token: Uuid,
    pub session_id: Uuid,
    pub start_time: chrono::DateTime<chrono::Utc>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SessionPost {
    pub version: String,
    pub os: String,
    pub machine_id: String,
    pub tests: u32,
    pub args: serde_json::Value,
    pub validation: serde_json::Value,
    pub config: serde_json::Value,
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
}

#[derive(Clone)]
pub struct Test {
    pub test_id: Uuid,
    pub session: Session,
    pub start_time: chrono::DateTime<chrono::Utc>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct TestPost {
    pub session_id: String,
    pub identifier: String,
    pub definition: serde_json::Value,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct TestCompletedPost {
    pub session_id: String,
    pub iteration: u32,
    pub stage: u32,
    pub stage_type: u32,
    pub status: u32,
    pub runtime: u32,
    pub details: serde_json::Value,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct TestResponse {
    pub test_id: String,
}

pub async fn create_session(
    token: Uuid,
    test_count: u32,
    cli: &crate::Cli,
    config: &config::Config,
) -> Result<Session, Box<dyn Error + Send + Sync>> {
    let client = Client::builder().build::<_, Body>(HttpsConnector::new());
    let uri = format!("{}/sessions", TELEMETRY_BASE_URL);
    trace!("telemetry session url({})", uri);
    match Url::parse(&uri) {
        Ok(_) => {}
        Err(error) => {
            return Err(Box::from(format!("invalid telemetry url: {}", error)));
        }
    }

    let args_json = serde_json::to_value(cli)?;
    let validation_json = serde_json::json!({}); // todo: add validation report once validation is implemented
    let config_json = serde_json::to_value(config)?;

    let m = machine::new();
    let machine_id = m.generate_machine_id();

    let post_body = SessionPost {
        version: crate::VERSION.to_string(),
        os: env::consts::OS.to_string(),
        machine_id,
        tests: test_count,
        args: args_json,
        validation: validation_json,
        config: config_json,
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
            start_time: chrono::Utc::now(),
        });
    }

    Err(Box::from(TelemetryError {
        reason: "invalid session request".to_string(),
    }))
}

pub async fn create_test(
    session: &Session,
    definition: &test::Definition,
) -> Result<Test, Box<dyn Error + Send + Sync>> {
    let client = Client::builder().build::<_, Body>(HttpsConnector::new());
    let uri = format!("{}/tests", TELEMETRY_BASE_URL);
    trace!("telemetry test url({})", uri);
    match Url::parse(&uri) {
        Ok(_) => {}
        Err(error) => {
            return Err(Box::from(format!("invalid telemetry url: {}", error)));
        }
    }

    let definition_json = serde_json::to_value(definition)?;

    let post_body = TestPost {
        session_id: session.session_id.to_string(),
        identifier: definition.id.clone(),
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
            start_time: chrono::Utc::now(),
        });
    }

    Err(Box::from(TelemetryError {
        reason: "invalid test request".to_string(),
    }))
}

pub async fn complete_stage(
    test: &Test,
    iteration: u32,
    stage: &executor::StageResult,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let client = Client::builder().build::<_, Body>(HttpsConnector::new());
    let uri = format!("{}/tests/{}/completed", TELEMETRY_BASE_URL, test.test_id);
    trace!("telemetry test url({})", uri);
    match Url::parse(&uri) {
        Ok(_) => {}
        Err(error) => {
            return Err(Box::from(format!("invalid telemetry url: {}", error)));
        }
    }

    let details_json = serde_json::to_value(&stage.details)?;
    let post_data = TestCompletedPost {
        session_id: test.session.session_id.to_string(),
        iteration,
        stage: stage.stage,
        stage_type: stage.stage_type.clone() as u32,
        status: stage.status.clone() as u32,
        runtime: stage.runtime,
        details: details_json,
    };

    let post_body = serde_json::to_value(post_data)?;

    let post_string = serde_json::to_string(&post_body)?;
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

pub async fn complete_session(
    session: &Session,
    runtime: u32,
    status: u32,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let client = Client::builder().build::<_, Body>(HttpsConnector::new());
    let uri = format!(
        "{}/sessions/{}/completed",
        TELEMETRY_BASE_URL, session.session_id
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
