use crate::test::definition::{Modifier, ResponseExtraction, VariableTypes};
use crate::test::file::{
    TestFile, UnvalidatedCleanup, UnvalidatedCompareRequest, UnvalidatedRequest,
    UnvalidatedRequestResponse, UnvalidatedResponse, UnvalidatedStage, UnvalidatedVariable,
};
use crate::test::http::{HttpHeader, HttpParameter, HttpVerb};
use std::cell::Cell;
use std::error::Error;
use uuid::Uuid;

pub fn template() -> Result<TestFile, Box<dyn Error + Send + Sync>> {
    Ok(TestFile {
        filename: "".to_string(),
        name: Some("".to_string()),
        id: Some(Uuid::new_v4().to_string()),
        env: None,
        tags: None,
        requires: None,
        iterate: None,
        setup: None,
        request: Some(new_request()),
        compare: None,
        response: Some(new_response()),
        stages: None,
        cleanup: None,
        variables: None,
    })
}

pub fn template_staged() -> Result<TestFile, Box<dyn Error + Send + Sync>> {
    Ok(TestFile {
        filename: "".to_string(),
        name: Some("".to_string()),
        id: Some(Uuid::new_v4().to_string()),
        env: None,
        tags: None,
        requires: None,
        iterate: None,
        setup: None,
        request: None,
        compare: None,
        response: None,
        stages: Some(vec![new_stage()]),
        cleanup: None,
        variables: None,
    })
}

pub fn template_full() -> Result<TestFile, Box<dyn Error + Send + Sync>> {
    Ok(TestFile {
        filename: "".to_string(),
        name: Some("".to_string()),
        id: Some(Uuid::new_v4().to_string()),
        env: Some("".to_string()),
        tags: Some("".to_string()),
        requires: Some("".to_string()),
        iterate: Some(1),
        setup: Some(new_full_request_response()?),
        request: Some(new_full_request()?),
        compare: Some(new_full_compare()?),
        response: Some(new_full_response()?),
        stages: Some(vec![new_full_stage()?]),
        cleanup: Some(new_full_cleanup()?),
        variables: Some(vec![new_full_variable()?]),
    })
}

fn new_full_cleanup() -> Result<UnvalidatedCleanup, Box<dyn Error + Send + Sync>> {
    Ok(UnvalidatedCleanup {
        request: Some(new_full_request()?),
        onsuccess: Some(new_full_request()?),
        onfailure: Some(new_full_request()?),
    })
}

fn new_full_request_response() -> Result<UnvalidatedRequestResponse, Box<dyn Error + Send + Sync>> {
    Ok(UnvalidatedRequestResponse {
        request: new_full_request()?,
        response: Some(new_full_response()?),
    })
}

fn new_stage() -> UnvalidatedStage {
    UnvalidatedStage {
        request: new_request(),
        compare: None,
        response: Some(new_response()),
        variables: None,
    }
}

fn new_full_stage() -> Result<UnvalidatedStage, Box<dyn Error + Send + Sync>> {
    Ok(UnvalidatedStage {
        request: new_full_request()?,
        compare: Some(new_full_compare()?),
        response: Some(new_full_response()?),
        variables: Some(vec![new_full_variable()?]),
    })
}

fn new_full_variable() -> Result<UnvalidatedVariable, Box<dyn Error + Send + Sync>> {
    Ok(UnvalidatedVariable {
        name: "".to_string(),
        data_type: VariableTypes::String,
        value: serde_json::from_str("{}")?,
        modifier: Some(Modifier::new()),
        format: Some("".to_string()),
    })
}

fn new_response() -> UnvalidatedResponse {
    UnvalidatedResponse {
        status: Some(200),
        headers: None,
        body: None,
        ignore: None,
        extract: None,
    }
}

fn new_full_response() -> Result<UnvalidatedResponse, Box<dyn Error + Send + Sync>> {
    Ok(UnvalidatedResponse {
        status: Some(200),
        headers: Some(vec![new_header()]),
        body: Some(serde_json::from_str("{}")?),
        ignore: Some(vec!["".to_string()]),
        extract: Some(vec![ResponseExtraction::new()]),
    })
}

fn new_full_compare() -> Result<UnvalidatedCompareRequest, Box<dyn Error + Send + Sync>> {
    Ok(UnvalidatedCompareRequest {
        method: Some(HttpVerb::Get),
        url: "".to_string(),
        params: Some(vec![new_parameter()]),
        add_params: Some(vec![new_parameter()]),
        ignore_params: Some(vec!["".to_string()]),
        headers: Some(vec![new_header()]),
        add_headers: Some(vec![new_header()]),
        ignore_headers: Some(vec!["".to_string()]),
        body: Some(serde_json::from_str("{}")?),
    })
}

fn new_request() -> UnvalidatedRequest {
    UnvalidatedRequest {
        method: None,
        url: "".to_string(),
        params: None,
        headers: None,
        body: None,
    }
}

fn new_full_request() -> Result<UnvalidatedRequest, Box<dyn Error + Send + Sync>> {
    Ok(UnvalidatedRequest {
        method: Some(HttpVerb::Get),
        url: "".to_string(),
        params: Some(vec![new_parameter()]),
        headers: Some(vec![new_header()]),
        body: Some(serde_json::from_str("{}")?),
    })
}

fn new_header() -> HttpHeader {
    HttpHeader {
        header: "".to_string(),
        value: "".to_string(),

        matches_variable: Cell::from(false),
    }
}

fn new_parameter() -> HttpParameter {
    HttpParameter {
        param: "".to_string(),
        value: "".to_string(),

        matches_variable: Cell::from(false),
    }
}
