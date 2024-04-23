use crate::test;
use crate::test::file::ValueOrSpecification;
use crate::test::{file, http, validation};
use serde::{Deserialize, Serialize};
use std::cell::Cell;
use std::collections::HashSet;

use super::file::BodyOrSchema;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RequestBody {
    pub data: BodyOrSchema,

    #[serde(skip_serializing, skip_deserializing)]
    pub matches_variable: Cell<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestDescriptor {
    pub method: http::Verb,
    pub url: String,
    pub params: Vec<http::Parameter>,
    pub headers: Vec<http::Header>,
    pub body: Option<RequestBody>,
}

// TODO: add validation logic to verify the descriptor is valid
impl RequestDescriptor {
    pub fn new(request: file::UnvalidatedRequest) -> Result<RequestDescriptor, validation::Error> {
        let validated_params = match request.params {
            Some(params) => params
                .iter()
                .map(|v| http::Parameter {
                    param: v.param.clone(),
                    value: v.value.clone(),
                    matches_variable: Cell::from(false),
                })
                .collect(),
            None => Vec::new(),
        };

        let validated_headers = match request.headers {
            Some(headers) => headers
                .iter()
                .map(|h| http::Header {
                    header: h.header.clone(),
                    value: h.value.clone(),
                    matches_variable: Cell::from(false),
                })
                .collect(),
            None => Vec::new(),
        };

        let request_body = request.body.map(|b| RequestBody {
            data: b,
            matches_variable: Cell::from(false),
        });

        Ok(RequestDescriptor {
            method: request.method.unwrap_or(http::Verb::Get),
            url: request.url,
            params: validated_params,
            headers: validated_headers,
            body: request_body,
        })
    }

    pub fn new_opt(
        request_opt: Option<file::UnvalidatedRequest>,
    ) -> Result<Option<RequestDescriptor>, validation::Error> {
        match request_opt {
            Some(request) => Ok(Some(RequestDescriptor::new(request)?)),
            None => Ok(None),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompareDescriptor {
    pub method: http::Verb,
    pub url: String,
    pub params: Vec<http::Parameter>,
    pub add_params: Vec<http::Parameter>,
    pub ignore_params: Vec<String>,
    pub headers: Vec<http::Header>,
    pub add_headers: Vec<http::Header>,
    pub ignore_headers: Vec<String>,
    pub body: Option<RequestBody>,
}

impl CompareDescriptor {
    pub fn new_opt(
        request_opt: Option<file::UnvalidatedCompareRequest>,
    ) -> Result<Option<CompareDescriptor>, validation::Error> {
        match request_opt {
            Some(request) => {
                let validated_params = match request.params {
                    Some(params) => params
                        .iter()
                        .map(|p| http::Parameter {
                            param: p.param.clone(),
                            value: p.value.clone(),
                            matches_variable: Cell::from(false),
                        })
                        .collect(),
                    None => Vec::new(),
                };

                let mut validated_add_params = Vec::new();
                let mut validated_ignore_params = Vec::new();

                if validated_params.is_empty() {
                    validated_add_params = match request.add_params {
                        Some(params) => params
                            .iter()
                            .map(|p| http::Parameter {
                                param: p.param.clone(),
                                value: p.value.clone(),
                                matches_variable: Cell::from(false),
                            })
                            .collect(),
                        None => Vec::new(),
                    };

                    validated_ignore_params = match request.ignore_params {
                        Some(params) => params.to_vec(),
                        None => Vec::new(),
                    };
                }

                let validated_headers = match request.headers {
                    Some(headers) => headers
                        .iter()
                        .map(|h| http::Header {
                            header: h.header.clone(),
                            value: h.value.clone(),
                            matches_variable: Cell::from(false),
                        })
                        .collect(),
                    None => Vec::new(),
                };

                let mut validated_add_headers = Vec::new();
                let mut validated_ignore_headers = Vec::new();

                if validated_headers.is_empty() {
                    validated_add_headers = match request.add_headers {
                        Some(headers) => headers
                            .iter()
                            .map(|h| http::Header {
                                header: h.header.clone(),
                                value: h.value.clone(),
                                matches_variable: Cell::from(false),
                            })
                            .collect(),
                        None => Vec::new(),
                    };

                    validated_ignore_headers = match request.ignore_headers {
                        Some(headers) => headers.to_vec(),
                        None => Vec::new(),
                    };
                }

                let compare_body = request.body.map(|b| RequestBody {
                    data: b,
                    matches_variable: Cell::from(false),
                });

                Ok(Some(CompareDescriptor {
                    method: request.method.unwrap_or(http::Verb::Get),
                    url: request.url,
                    params: validated_params,
                    add_params: validated_add_params,
                    ignore_params: validated_ignore_params,
                    headers: validated_headers,
                    add_headers: validated_add_headers,
                    ignore_headers: validated_ignore_headers,
                    body: compare_body,
                }))
            }
            None => Ok(None),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub struct ResponseExtraction {
    pub name: String,
    pub field: String,
}

impl ResponseExtraction {
    pub fn new() -> ResponseExtraction {
        ResponseExtraction {
            name: "".to_string(),
            field: "".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseDescriptor {
    pub status: Option<ValueOrSpecification<u16>>,
    pub headers: Vec<http::Header>,
    pub body: Option<RequestBody>,
    pub ignore: Vec<String>,
    pub extract: Vec<ResponseExtraction>,
}

// TODO: add validation logic to verify the descriptor is valid
impl ResponseDescriptor {
    pub fn new_opt(
        response: Option<file::UnvalidatedResponse>,
    ) -> Result<Option<ResponseDescriptor>, validation::Error> {
        match response {
            Some(res) => {
                let validated_headers = match res.headers {
                    Some(headers) => headers
                        .iter()
                        .map(|h| http::Header {
                            header: h.header.clone(),
                            value: h.value.clone(),
                            matches_variable: Cell::from(false),
                        })
                        .collect(),
                    None => Vec::new(),
                };

                let validated_ignore = res.ignore.unwrap_or_default();
                let validated_extraction = res.extract.unwrap_or_default();

                let response_body = res.body.map(|b| RequestBody {
                    data: b,
                    matches_variable: Cell::from(false),
                });

                Ok(Some(ResponseDescriptor {
                    status: res.status,
                    headers: validated_headers,
                    body: response_body,
                    ignore: validated_ignore,
                    extract: validated_extraction,
                }))
            }
            None => Ok(None),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageDescriptor {
    pub request: RequestDescriptor,
    pub compare: Option<CompareDescriptor>,
    pub response: Option<ResponseDescriptor>,
    pub variables: Vec<test::Variable>,
    pub name: Option<String>,
    //I would prefer to do this is Option<chrono::duration>
    //But it requires too much effort in serialization/deserialization
    pub delay: Option<u64>,
    //#[serde(skip_serializing)]
    //pub source_path: String,
}

impl StageDescriptor {
    pub fn new(
        stage: file::UnvalidatedStage,
        source_path: &str,
    ) -> Result<StageDescriptor, validation::Error> {
        Ok(StageDescriptor {
            request: RequestDescriptor::new(stage.request)?,
            compare: CompareDescriptor::new_opt(stage.compare)?,
            response: ResponseDescriptor::new_opt(stage.response)?,
            variables: test::Variable::validate_variables_opt(stage.variables, source_path)?,
            // source_path: source_path.to_string(),
            name: stage.name,
            delay: stage.delay,
        })
    }

    pub fn validate_stages_opt(
        request_opt: Option<file::UnvalidatedRequest>,
        compare_opt: Option<file::UnvalidatedCompareRequest>,
        response_opt: Option<file::UnvalidatedResponse>,
        stages_opt: Option<Vec<file::UnvalidatedStage>>,
        source_path: &str,
    ) -> Result<Vec<StageDescriptor>, validation::Error> {
        let mut results = Vec::new();
        let mut count = 0;

        if let Some(request) = request_opt {
            results.push(StageDescriptor {
                request: RequestDescriptor::new(request)?,
                compare: CompareDescriptor::new_opt(compare_opt)?,
                response: ResponseDescriptor::new_opt(response_opt)?,
                variables: Vec::new(),
                // source_path: source_path.to_string(),
                name: Some("request".to_string()),
                delay: None,
            });
            count += 1;
        }

        match stages_opt {
            None => Ok(results),
            Some(stages) => {
                count += stages.len();
                results.append(
                    &mut stages
                        .into_iter()
                        .map(|s| StageDescriptor::new(s, source_path))
                        .filter_map(|v| match v {
                            Ok(x) => Some(x),
                            Err(_) => None,
                        })
                        .collect::<Vec<StageDescriptor>>(),
                );
                if results.len() != count {
                    Err(validation::Error {
                        reason: "blah".to_string(),
                    })
                } else {
                    Ok(results)
                }
            }
        }
    }

    pub fn get_compare_parameters(&self) -> Vec<http::Parameter> {
        if let Some(c) = &self.compare {
            if !c.params.is_empty() {
                return c.params.clone();
            }

            let ignore_lookup: HashSet<String> = c.ignore_params.iter().cloned().collect();
            return self
                .request
                .clone()
                .params
                .into_iter()
                .filter(|p| !ignore_lookup.contains(&p.param))
                .chain(c.add_params.clone())
                .collect();
        }

        Vec::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestResponseDescriptor {
    pub request: RequestDescriptor,
    pub response: Option<ResponseDescriptor>,
}

impl RequestResponseDescriptor {
    pub fn new_opt(
        reqresp_opt: Option<file::UnvalidatedRequestResponse>,
    ) -> Result<Option<RequestResponseDescriptor>, validation::Error> {
        match reqresp_opt {
            Some(reqresp) => Ok(Some(RequestResponseDescriptor {
                request: RequestDescriptor::new(reqresp.request)?,
                response: ResponseDescriptor::new_opt(reqresp.response)?,
            })),
            None => Ok(None),
        }
    }
}

pub struct ResolvedRequest {
    // pub req_resp: RequestResponseDescriptor,
    pub url: String,
    pub method: http::Method,
    pub headers: Vec<(String, String)>,
    pub body: Option<serde_json::Value>,
}

impl ResolvedRequest {
    pub fn new(
        url: String,
        method: http::Method,
        headers: Vec<(String, String)>,
        body: Option<serde_json::Value>,
    ) -> ResolvedRequest {
        ResolvedRequest {
            url,
            method,
            headers,
            body,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanupDescriptor {
    pub onsuccess: Option<RequestDescriptor>,
    pub onfailure: Option<RequestDescriptor>,
    pub always: Option<RequestDescriptor>,
}

impl CleanupDescriptor {
    pub fn new(
        cleanup_opt: Option<file::UnvalidatedCleanup>,
    ) -> Result<CleanupDescriptor, validation::Error> {
        match cleanup_opt {
            Some(cleanup) => Ok(CleanupDescriptor {
                onsuccess: RequestDescriptor::new_opt(cleanup.onsuccess)?,
                onfailure: RequestDescriptor::new_opt(cleanup.onfailure)?,
                always: RequestDescriptor::new_opt(cleanup.always)?,
            }),
            None => Ok(CleanupDescriptor {
                onsuccess: None,
                onfailure: None,
                always: None,
            }),
        }
    }
}
