use crate::test_definition::{HttpVerb, Modifier, ResponseExtraction, VariableTypes};
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub struct UnvalidatedHttpHeader {
    pub header: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub struct UnvalidatedHttpParameter {
    pub param: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnvalidatedRequest {
    pub method: Option<HttpVerb>,
    pub url: String,
    pub params: Option<Vec<UnvalidatedHttpParameter>>,
    pub headers: Option<Vec<UnvalidatedHttpHeader>>,
    pub body: Option<serde_json::Value>,
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

    #[serde(skip_serializing, skip_deserializing)]
    index: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub struct UnvalidatedStage {
    pub request: UnvalidatedRequest,
    pub compare: Option<UnvalidatedCompareRequest>,
    pub response: Option<UnvalidatedResponse>,
    pub variables: Option<Vec<UnvalidatedVariable>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub struct UnvalidatedRequestResponse {
    pub request: UnvalidatedRequest,
    pub response: Option<UnvalidatedResponse>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub struct UnvalidatedCleanup {
    pub onsuccess: Option<UnvalidatedRequestResponse>,
    pub onfailure: Option<UnvalidatedRequestResponse>,
    pub request: UnvalidatedRequest,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub struct UnvalidatedTest {
    pub name: Option<String>,
    pub id: Option<String>,
    pub requires: Option<String>,
    pub tags: Option<String>,
    pub iterate: Option<u32>,
    pub request: Option<UnvalidatedRequest>,
    pub compare: Option<UnvalidatedCompareRequest>,
    pub response: Option<UnvalidatedResponse>,
    pub variables: Option<Vec<UnvalidatedVariable>>,
    pub stages: Option<Vec<UnvalidatedStage>>,
    pub setup: Option<UnvalidatedRequestResponse>,
    pub cleanup: Option<UnvalidatedCleanup>,
}

impl UnvalidatedTest {
    pub fn generate_id(&self) -> String {
        let mut s = DefaultHasher::new();
        self.hash(&mut s);
        format!("{}", s.finish())
    }
}
