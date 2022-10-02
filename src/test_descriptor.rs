use serde::{Serialize, Deserialize};

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub enum HttpVerb {
    GET,
    POST,
    PUT,
    PATCH,
    UNDEFINED
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct HttpKvp {
    pub key: Option<String>,
    pub value: Option<String>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct RequestDescriptor {
    pub method: Option<HttpVerb>,
    pub url: String,
    pub params: Option<Vec<HttpKvp>>,
    pub headers: Option<Vec<HttpKvp>>,
    pub body: Option<serde_json::Value>,
}

// TODO: add validation logic to verify the descriptor is valid
impl RequestDescriptor {
    pub fn validate(&self) -> bool {
        true
    }

    pub fn get_url(&self) -> String {
        let joined: Vec<_> = self.params.unwrap_or(Vec::new()).iter().map(|kvp| format!("{}={}", kvp.key.unwrap(), kvp.value.unwrap())).collect();
        format!("{}?{}", self.url, joined.join("&"))
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct ResponseDescriptor {
    pub status: Option<u16>,
    pub headers: Option<Vec<HttpKvp>>,
    pub body: Option<serde_json::Value>,
    pub ignore: Option<Vec<String>>,
}

// TODO: add validation logic to verify the descriptor is valid
impl ResponseDescriptor {
    pub fn validate(&self) -> bool {
        true
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct TestDescriptor {
    pub name: Option<String>,
    pub request: RequestDescriptor,
    pub compare:  Option<RequestDescriptor>,
    pub response: Option<ResponseDescriptor>,
}

// TODO: add validation logic to verify the descriptor is valid
impl TestDescriptor {
    pub fn validate(&self) -> bool {
        self.request.validate()
    }
}