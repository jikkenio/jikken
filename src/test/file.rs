use crate::test;
use crate::test::{definition, http, variable};
use log::error;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fs;
use std::hash::{Hash, Hasher};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnvalidatedRequest {
    pub method: Option<http::Verb>,
    pub url: String,
    pub params: Option<Vec<http::Parameter>>,
    pub headers: Option<Vec<http::Header>>,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnvalidatedCompareRequest {
    pub method: Option<http::Verb>,
    pub url: String,
    pub params: Option<Vec<http::Parameter>>,
    pub add_params: Option<Vec<http::Parameter>>,
    pub ignore_params: Option<Vec<String>>,
    pub headers: Option<Vec<http::Header>>,
    pub add_headers: Option<Vec<http::Header>>,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnvalidatedResponse {
    pub status: Option<u16>,
    pub headers: Option<Vec<http::Header>>,
    pub body: Option<serde_json::Value>,
    pub ignore: Option<Vec<String>>,
    pub extract: Option<Vec<definition::ResponseExtraction>>,
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
    #[serde(rename = "type")]
    pub data_type: Option<variable::Type>,
    pub value: Option<serde_yaml::Value>,
    pub modifier: Option<variable::Modifier>,
    pub format: Option<String>,
    pub file: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
pub struct UnvalidatedStage {
    pub request: UnvalidatedRequest,
    pub compare: Option<UnvalidatedCompareRequest>,
    pub response: Option<UnvalidatedResponse>,
    pub variables: Option<Vec<UnvalidatedVariable>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
pub struct UnvalidatedRequestResponse {
    pub request: UnvalidatedRequest,
    pub response: Option<UnvalidatedResponse>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
pub struct UnvalidatedCleanup {
    pub onsuccess: Option<UnvalidatedRequest>,
    pub onfailure: Option<UnvalidatedRequest>,
    pub always: Option<UnvalidatedRequest>,
}

pub fn load(filename: &str) -> Result<test::File, Box<dyn Error + Send + Sync>> {
    let file_data = fs::read_to_string(filename)?;
    let result: Result<test::File, serde_yaml::Error> = serde_yaml::from_str(&file_data);
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
