use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum HttpVerb {
    Get,
    Post,
    Put,
    Patch,
    Undefined,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HttpKvp {
    pub key: Option<String>,
    pub value: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
        let joined: Vec<_> = match &self.params {
            Some(p) => p
                .iter()
                .map(|kvp| {
                    format!(
                        "{}={}",
                        kvp.key.as_ref().unwrap(),
                        kvp.value.as_ref().unwrap()
                    )
                })
                .collect(),
            _ => Vec::new(),
        };

        format!("{}?{}", self.url, joined.join("&"))
    }

    pub fn get_headers(&self) -> Vec<(String, String)> {
        match &self.headers {
            Some(h) => h
                .iter()
                .filter(|kvp| {
                    if kvp.key.as_ref().unwrap_or(&String::from("")) == "" {
                        return false;
                    }
                    if kvp.value.as_ref().unwrap_or(&String::from("")) == "" {
                        return false;
                    }
                    true
                })
                .map(|kvp| {
                    (
                        kvp.key.as_ref().unwrap().clone(),
                        kvp.value.as_ref().unwrap().clone(),
                    )
                })
                .collect(),
            _ => Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TestDescriptor {
    pub name: Option<String>,
    pub request: RequestDescriptor,
    pub compare: Option<RequestDescriptor>,
    pub response: Option<ResponseDescriptor>,
}

// TODO: add validation logic to verify the descriptor is valid
impl TestDescriptor {
    pub fn validate(&self) -> bool {
        self.request.validate()
    }
}
