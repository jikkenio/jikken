use crate::test_definition::{HttpVerb, Modifier, VariableTypes};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnvalidatedHttpHeader {
    pub header: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnvalidatedResponse {
    pub status: Option<u16>,
    pub headers: Option<Vec<UnvalidatedHttpHeader>>,
    pub body: Option<serde_json::Value>,
    pub ignore: Option<Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnvalidatedTest {
    pub name: Option<String>,
    pub iterate: Option<u32>,
    pub request: UnvalidatedRequest,
    pub compare: Option<UnvalidatedRequest>,
    pub response: Option<UnvalidatedResponse>,
    pub variables: Option<Vec<UnvalidatedVariable>>,
}
