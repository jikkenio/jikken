use crate::test_definition::{HttpVerb, Modifier, ResponseExtraction, VariableTypes};
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::error::Error;
use std::hash::{Hash, Hasher};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub struct UnvalidatedHttpHeader {
    pub header: String,
    pub value: String,
}

impl UnvalidatedHttpHeader {
    pub fn new() -> UnvalidatedHttpHeader {
        UnvalidatedHttpHeader {
            header: "".to_string(),
            value: "".to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub struct UnvalidatedHttpParameter {
    pub param: String,
    pub value: String,
}

impl UnvalidatedHttpParameter {
    pub fn new() -> UnvalidatedHttpParameter {
        UnvalidatedHttpParameter {
            param: "".to_string(),
            value: "".to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnvalidatedRequest {
    pub method: Option<HttpVerb>,
    pub url: String,
    pub params: Option<Vec<UnvalidatedHttpParameter>>,
    pub headers: Option<Vec<UnvalidatedHttpHeader>>,
    pub body: Option<serde_json::Value>,
}

impl UnvalidatedRequest {
    pub fn new() -> UnvalidatedRequest {
        UnvalidatedRequest {
            method: None,
            url: "".to_string(),
            params: None,
            headers: None,
            body: None,
        }
    }

    pub fn new_full() -> Result<UnvalidatedRequest, Box<dyn Error + Send + Sync>> {
        Ok(UnvalidatedRequest {
            method: Some(HttpVerb::Get),
            url: "".to_string(),
            params: Some(vec![UnvalidatedHttpParameter::new()]),
            headers: Some(vec![UnvalidatedHttpHeader::new()]),
            body: Some(serde_json::from_str("{}")?),
        })
    }
}

impl Hash for UnvalidatedRequest {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.method.hash(state);
        self.url.hash(state);
        self.params.hash(state);
        self.headers.hash(state);
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnvalidatedCompareRequest {
    pub method: Option<HttpVerb>,
    pub url: String,
    pub params: Option<Vec<UnvalidatedHttpParameter>>,
    pub add_params: Option<Vec<UnvalidatedHttpParameter>>,
    pub ignore_params: Option<Vec<String>>,
    pub headers: Option<Vec<UnvalidatedHttpHeader>>,
    pub add_headers: Option<Vec<UnvalidatedHttpHeader>>,
    pub ignore_headers: Option<Vec<String>>,
    pub body: Option<serde_json::Value>,
}

impl UnvalidatedCompareRequest {
    // pub fn new() -> UnvalidatedCompareRequest {
    //     UnvalidatedCompareRequest {
    //         method: None,
    //         url: "".to_string(),
    //         params: None,
    //         add_params: None,
    //         ignore_params: None,
    //         headers: None,
    //         add_headers: None,
    //         ignore_headers: None,
    //         body: None,
    //     }
    // }

    pub fn new_full() -> Result<UnvalidatedCompareRequest, Box<dyn Error + Send + Sync>> {
        Ok(UnvalidatedCompareRequest {
            method: Some(HttpVerb::Get),
            url: "".to_string(),
            params: Some(vec![UnvalidatedHttpParameter::new()]),
            add_params: Some(vec![UnvalidatedHttpParameter::new()]),
            ignore_params: Some(vec!["".to_string()]),
            headers: Some(vec![UnvalidatedHttpHeader::new()]),
            add_headers: Some(vec![UnvalidatedHttpHeader::new()]),
            ignore_headers: Some(vec!["".to_string()]),
            body: Some(serde_json::from_str("{}")?),
        })
    }
}

impl Hash for UnvalidatedCompareRequest {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.method.hash(state);
        self.url.hash(state);
        self.params.hash(state);
        self.add_params.hash(state);
        self.ignore_params.hash(state);
        self.headers.hash(state);
        self.add_headers.hash(state);
        self.ignore_headers.hash(state);
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnvalidatedResponse {
    pub status: Option<u16>,
    pub headers: Option<Vec<UnvalidatedHttpHeader>>,
    pub body: Option<serde_json::Value>,
    pub ignore: Option<Vec<String>>,
    pub extract: Option<Vec<ResponseExtraction>>,
}

impl UnvalidatedResponse {
    pub fn new() -> UnvalidatedResponse {
        UnvalidatedResponse {
            status: Some(200),
            headers: None,
            body: None,
            ignore: None,
            extract: None,
        }
    }

    pub fn new_full() -> Result<UnvalidatedResponse, Box<dyn Error + Send + Sync>> {
        Ok(UnvalidatedResponse {
            status: Some(200),
            headers: Some(vec![UnvalidatedHttpHeader::new()]),
            body: Some(serde_json::from_str("{}")?),
            ignore: Some(vec!["".to_string()]),
            extract: Some(vec![ResponseExtraction::new()]),
        })
    }
}

impl Hash for UnvalidatedResponse {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.status.hash(state);
        self.headers.hash(state);
        self.ignore.hash(state);
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Hash)]
#[serde(rename_all = "camelCase")]
pub struct UnvalidatedVariable {
    pub name: String,
    pub data_type: VariableTypes,
    pub value: serde_yaml::Value,
    pub modifier: Option<Modifier>,
    pub format: Option<String>,
}

impl UnvalidatedVariable {
    // pub fn new() -> Result<UnvalidatedVariable, Box<dyn Error + Send + Sync>> {
    //     Ok(UnvalidatedVariable{
    //         name: "".to_string(),
    //         data_type: VariableTypes::String,
    //         value: serde_json::from_str("{}")?,
    //         modifier: None,
    //         format: None,
    //     })
    // }

    pub fn new_full() -> Result<UnvalidatedVariable, Box<dyn Error + Send + Sync>> {
        Ok(UnvalidatedVariable {
            name: "".to_string(),
            data_type: VariableTypes::String,
            value: serde_json::from_str("{}")?,
            modifier: Some(Modifier::new()),
            format: Some("".to_string()),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub struct UnvalidatedStage {
    pub request: UnvalidatedRequest,
    pub compare: Option<UnvalidatedCompareRequest>,
    pub response: Option<UnvalidatedResponse>,
    pub variables: Option<Vec<UnvalidatedVariable>>,
}

impl UnvalidatedStage {
    pub fn new() -> UnvalidatedStage {
        UnvalidatedStage {
            request: UnvalidatedRequest::new(),
            compare: None,
            response: Some(UnvalidatedResponse::new()),
            variables: None,
        }
    }

    pub fn new_full() -> Result<UnvalidatedStage, Box<dyn Error + Send + Sync>> {
        Ok(UnvalidatedStage {
            request: UnvalidatedRequest::new_full()?,
            compare: Some(UnvalidatedCompareRequest::new_full()?),
            response: Some(UnvalidatedResponse::new_full()?),
            variables: Some(vec![UnvalidatedVariable::new_full()?]),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub struct UnvalidatedRequestResponse {
    pub request: UnvalidatedRequest,
    pub response: Option<UnvalidatedResponse>,
}

impl UnvalidatedRequestResponse {
    // pub fn new() -> UnvalidatedRequestResponse {
    //     UnvalidatedRequestResponse {
    //         request: UnvalidatedRequest::new(),
    //         response: Some(UnvalidatedResponse::new())
    //     }
    // }

    pub fn new_full() -> Result<UnvalidatedRequestResponse, Box<dyn Error + Send + Sync>> {
        Ok(UnvalidatedRequestResponse {
            request: UnvalidatedRequest::new_full()?,
            response: Some(UnvalidatedResponse::new_full()?),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub struct UnvalidatedCleanup {
    pub request: UnvalidatedRequest,
    pub onsuccess: Option<UnvalidatedRequest>,
    pub onfailure: Option<UnvalidatedRequest>,
}

impl UnvalidatedCleanup {
    pub fn new_full() -> Result<UnvalidatedCleanup, Box<dyn Error + Send + Sync>> {
        Ok(UnvalidatedCleanup {
            request: UnvalidatedRequest::new_full()?,
            onsuccess: Some(UnvalidatedRequest::new_full()?),
            onfailure: Some(UnvalidatedRequest::new_full()?),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub struct UnvalidatedTest {
    pub name: Option<String>,
    pub id: Option<String>,
    pub env: Option<String>,
    pub tags: Option<String>,
    pub requires: Option<String>,
    pub iterate: Option<u32>,
    pub setup: Option<UnvalidatedRequestResponse>,
    pub request: Option<UnvalidatedRequest>,
    pub compare: Option<UnvalidatedCompareRequest>,
    pub response: Option<UnvalidatedResponse>,
    pub stages: Option<Vec<UnvalidatedStage>>,
    pub cleanup: Option<UnvalidatedCleanup>,
    pub variables: Option<Vec<UnvalidatedVariable>>,
}

impl UnvalidatedTest {
    pub fn generate_id(&self) -> String {
        let mut s = DefaultHasher::new();
        self.hash(&mut s);
        format!("{}", s.finish())
    }

    pub fn template() -> Result<UnvalidatedTest, Box<dyn Error + Send + Sync>> {
        Ok(UnvalidatedTest {
            name: Some("".to_string()),
            id: Some(Uuid::new_v4().to_string()),
            env: None,
            tags: None,
            requires: None,
            iterate: None,
            setup: None,
            request: Some(UnvalidatedRequest::new()),
            compare: None,
            response: Some(UnvalidatedResponse::new()),
            stages: None,
            cleanup: None,
            variables: None,
        })
    }

    pub fn template_staged() -> Result<UnvalidatedTest, Box<dyn Error + Send + Sync>> {
        Ok(UnvalidatedTest {
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
            stages: Some(vec![UnvalidatedStage::new()]),
            cleanup: None,
            variables: None,
        })
    }

    pub fn template_full() -> Result<UnvalidatedTest, Box<dyn Error + Send + Sync>> {
        Ok(UnvalidatedTest {
            name: Some("".to_string()),
            id: Some(Uuid::new_v4().to_string()),
            env: Some("".to_string()),
            tags: Some("".to_string()),
            requires: Some("".to_string()),
            iterate: Some(1),
            setup: Some(UnvalidatedRequestResponse::new_full()?),
            request: Some(UnvalidatedRequest::new_full()?),
            compare: Some(UnvalidatedCompareRequest::new_full()?),
            response: Some(UnvalidatedResponse::new_full()?),
            stages: Some(vec![UnvalidatedStage::new_full()?]),
            cleanup: Some(UnvalidatedCleanup::new_full()?),
            variables: Some(vec![UnvalidatedVariable::new_full()?]),
        })
    }
}
