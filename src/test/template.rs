use crate::test;
use crate::test::{definition, file, http};
use std::cell::Cell;
use std::error::Error;
use uuid::Uuid;

use super::File;

pub fn template() -> Result<test::File, Box<dyn Error + Send + Sync>> {
    Ok(test::File::default())
}

pub fn template_staged() -> Result<test::File, Box<dyn Error + Send + Sync>> {
    let default = File::default();
    Ok(test::File {
        request: None,
        response: None,
        stages: Some(vec![new_stage()]),
        ..default
    })
}

pub fn template_full() -> Result<test::File, Box<dyn Error + Send + Sync>> {
    //Do not use default as a basis, as opposed to the others.
    //Compilation error will serve as a reminder to init to "full" state
    Ok(test::File {
        filename: "".to_string(),
        name: Some("".to_string()),
        id: Some(Uuid::new_v4().to_string()),
        project: Some("".to_string()),
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
        variables: Some(vec![new_full_variables()?]),
        disabled: Some(false),
        description: Some("".to_string()),
    })
}

fn new_full_cleanup() -> Result<file::UnvalidatedCleanup, Box<dyn Error + Send + Sync>> {
    Ok(file::UnvalidatedCleanup {
        onsuccess: Some(new_full_request()?),
        onfailure: Some(new_full_request()?),
        always: Some(new_full_request()?),
    })
}

fn new_full_request_response(
) -> Result<file::UnvalidatedRequestResponse, Box<dyn Error + Send + Sync>> {
    Ok(file::UnvalidatedRequestResponse {
        request: new_full_request()?,
        response: Some(new_full_response()?),
    })
}

fn new_stage() -> file::UnvalidatedStage {
    file::UnvalidatedStage {
        request: new_request(),
        compare: None,
        response: Some(new_response()),
        variables: None,
        name: None,
        delay: None,
    }
}

fn new_full_stage() -> Result<file::UnvalidatedStage, Box<dyn Error + Send + Sync>> {
    Ok(file::UnvalidatedStage {
        request: new_full_request()?,
        compare: Some(new_full_compare()?),
        response: Some(new_full_response()?),
        variables: Some(vec![new_full_variables()?]),
        name: None,
        delay: None,
    })
}

//Do we want to create a variable of every type as part of the full template?
fn new_full_variables() -> Result<file::UnvalidatedVariable, Box<dyn Error + Send + Sync>> {
    Ok(file::UnvalidatedVariable {
        name: "".to_string(),
        value: file::StringOrDatumOrFile::Value("".to_string()),
    })
}

fn new_response() -> file::UnvalidatedResponse {
    file::UnvalidatedResponse::default()
}

fn new_full_response() -> Result<file::UnvalidatedResponse, Box<dyn Error + Send + Sync>> {
    Ok(file::UnvalidatedResponse {
        status: Some(test::file::ValueOrSpecification::Value(200)),
        headers: Some(vec![new_header()]),
        body: Some(serde_json::from_str("{}")?),
        ignore: Some(vec!["".to_string()]),
        extract: Some(vec![definition::ResponseExtraction::new()]),
    })
}

fn new_full_compare() -> Result<file::UnvalidatedCompareRequest, Box<dyn Error + Send + Sync>> {
    Ok(file::UnvalidatedCompareRequest {
        method: Some(http::Verb::Get),
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

fn new_request() -> file::UnvalidatedRequest {
    file::UnvalidatedRequest::default()
}

fn new_full_request() -> Result<file::UnvalidatedRequest, Box<dyn Error + Send + Sync>> {
    Ok(file::UnvalidatedRequest {
        method: Some(http::Verb::Get),
        url: "".to_string(),
        params: Some(vec![new_parameter()]),
        headers: Some(vec![new_header()]),
        body: Some(serde_json::from_str("{}")?),
    })
}

fn new_header() -> http::Header {
    http::Header {
        header: "".to_string(),
        value: "".to_string(),

        matches_variable: Cell::from(false),
    }
}

fn new_parameter() -> http::Parameter {
    http::Parameter {
        param: "".to_string(),
        value: "".to_string(),

        matches_variable: Cell::from(false),
    }
}
