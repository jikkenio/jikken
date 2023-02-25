use crate::test;
use crate::test::file;
use crate::test::http;
use crate::test::validation;
use serde::{Deserialize, Serialize};
use std::cell::Cell;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RequestBody {
    pub data: serde_json::Value,

    #[serde(skip_serializing, skip_deserializing)]
    pub matches_variable: Cell<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

        let request_body = match request.body {
            Some(b) => Some(RequestBody {
                data: b,
                matches_variable: Cell::from(false),
            }),
            None => None,
        };

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

    pub fn validate(&self) -> bool {
        true
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

                if validated_params.len() == 0 {
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
                        Some(params) => params.iter().map(|p| p.clone()).collect(),
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

                if validated_headers.len() == 0 {
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
                        Some(headers) => headers.iter().map(|h| h.clone()).collect(),
                        None => Vec::new(),
                    };
                }

                let compare_body = match request.body {
                    Some(b) => Some(RequestBody {
                        data: b,
                        matches_variable: Cell::from(false),
                    }),
                    None => None,
                };

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

    pub fn validate(&self) -> bool {
        true
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResponseDescriptor {
    pub status: Option<u16>,
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

                let validated_ignore = match res.ignore {
                    Some(ignore) => ignore,
                    None => Vec::new(),
                };

                let validated_extraction = match res.extract {
                    Some(extract) => extract,
                    None => Vec::new(),
                };

                let response_body = match res.body {
                    Some(b) => Some(RequestBody {
                        data: b,
                        matches_variable: Cell::from(false),
                    }),
                    None => None,
                };

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

    pub fn validate(&self) -> bool {
        true
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StageDescriptor {
    pub request: RequestDescriptor,
    pub compare: Option<CompareDescriptor>,
    pub response: Option<ResponseDescriptor>,
    pub variables: Vec<test::Variable>,
}

impl StageDescriptor {
    pub fn new(stage: file::UnvalidatedStage) -> Result<StageDescriptor, validation::Error> {
        Ok(StageDescriptor {
            request: RequestDescriptor::new(stage.request)?,
            compare: CompareDescriptor::new_opt(stage.compare)?,
            response: ResponseDescriptor::new_opt(stage.response)?,
            variables: test::Variable::validate_variables_opt(stage.variables)?,
        })
    }

    pub fn validate_stages_opt(
        request_opt: Option<file::UnvalidatedRequest>,
        compare_opt: Option<file::UnvalidatedCompareRequest>,
        response_opt: Option<file::UnvalidatedResponse>,
        stages_opt: Option<Vec<file::UnvalidatedStage>>,
    ) -> Result<Vec<StageDescriptor>, validation::Error> {
        let mut results = Vec::new();
        let mut count = 0;

        if let Some(request) = request_opt {
            results.push(StageDescriptor {
                request: RequestDescriptor::new(request)?,
                compare: CompareDescriptor::new_opt(compare_opt)?,
                response: ResponseDescriptor::new_opt(response_opt)?,
                variables: Vec::new(),
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
                        .map(|s| StageDescriptor::new(s))
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
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

    pub fn validate(&self) -> bool {
        true
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CleanupDescriptor {
    pub onsuccess: Option<RequestDescriptor>,
    pub onfailure: Option<RequestDescriptor>,
    pub request: Option<RequestDescriptor>,
}

impl CleanupDescriptor {
    pub fn new(
        cleanup_opt: Option<file::UnvalidatedCleanup>,
    ) -> Result<CleanupDescriptor, validation::Error> {
        match cleanup_opt {
            Some(cleanup) => Ok(CleanupDescriptor {
                onsuccess: RequestDescriptor::new_opt(cleanup.onsuccess)?,
                onfailure: RequestDescriptor::new_opt(cleanup.onfailure)?,
                request: RequestDescriptor::new_opt(cleanup.request)?,
            }),
            None => Ok(CleanupDescriptor {
                onsuccess: None,
                onfailure: None,
                request: None,
            }),
        }
    }

    pub fn validate(&self) -> bool {
        true
    }
}
