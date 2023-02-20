use crate::test::definition::{Modifier, ResponseExtraction, VariableTypes};
use crate::test::http::{HttpHeader, HttpParameter, HttpVerb};
use log::error;
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::error::Error;
use std::fs;
use std::hash::{Hash, Hasher};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnvalidatedRequest {
    pub method: Option<HttpVerb>,
    pub url: String,
    pub params: Option<Vec<HttpParameter>>,
    pub headers: Option<Vec<HttpHeader>>,
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
    pub params: Option<Vec<HttpParameter>>,
    pub add_params: Option<Vec<HttpParameter>>,
    pub ignore_params: Option<Vec<String>>,
    pub headers: Option<Vec<HttpHeader>>,
    pub add_headers: Option<Vec<HttpHeader>>,
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
    pub headers: Option<Vec<HttpHeader>>,
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
    pub request: Option<UnvalidatedRequest>,
    pub onsuccess: Option<UnvalidatedRequest>,
    pub onfailure: Option<UnvalidatedRequest>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub struct TestFile {
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

    #[serde(skip_serializing, skip_deserializing)]
    pub filename: String,
}

impl TestFile {
    pub fn load(filename: &str) -> Result<TestFile, Box<dyn Error + Send + Sync>> {
        let file_data = fs::read_to_string(filename)?;
        let result: Result<TestFile, serde_yaml::Error> = serde_yaml::from_str(&file_data);
        match result {
            Ok(mut file) => {
                file.filename = String::from(filename);
                Ok(file)
            }
            Err(e) => {
                error!("unable to parse file ({}) data: {}", filename, e);
                Err(Box::from(e))
            }
        }
    }

    pub fn generate_id(&self) -> String {
        let mut s = DefaultHasher::new();
        self.hash(&mut s);
        format!("{}", s.finish())
    }
}
