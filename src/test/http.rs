use hyper;
use serde::{Deserialize, Serialize, Serializer};
use std::cell::Cell;
use std::fmt;
use std::hash::{Hash, Hasher};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Header {
    pub header: String,
    pub value: String,

    #[serde(skip_serializing, skip_deserializing)]
    pub matches_variable: Cell<bool>,
}

impl Header {
    pub fn new(header: String, value: String) -> Header {
        Header {
            header,
            value,
            matches_variable: Cell::from(false),
        }
    }
}

impl Hash for Header {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.header.hash(state);
        self.value.hash(state);
    }

    fn hash_slice<H: Hasher>(data: &[Self], state: &mut H)
    where
        Self: Sized,
    {
        for piece in data {
            piece.hash(state);
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Parameter {
    pub param: String,
    pub value: String,

    #[serde(skip_serializing, skip_deserializing)]
    pub matches_variable: Cell<bool>,
}

impl Hash for Parameter {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.param.hash(state);
        self.value.hash(state);
    }

    fn hash_slice<H: Hasher>(data: &[Self], state: &mut H)
    where
        Self: Sized,
    {
        for piece in data {
            piece.hash(state);
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub enum Verb {
    #[serde(alias = "get", alias = "GET")]
    Get,
    #[serde(alias = "post", alias = "POST")]
    Post,
    #[serde(alias = "put", alias = "PUT")]
    Put,
    #[serde(alias = "patch", alias = "PATCH")]
    Patch,
    Undefined,
}

impl Verb {
    pub fn as_method(&self) -> Method {
        match &self {
            Verb::Post => Method(hyper::Method::POST),
            Verb::Patch => Method(hyper::Method::PATCH),
            Verb::Put => Method(hyper::Method::PUT),
            _ => Method(hyper::Method::GET),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Method(hyper::Method);

impl Method {
    pub fn to_hyper(&self) -> hyper::Method {
        self.0.clone()
    }
}

impl Serialize for Method {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.0.as_str())
    }
}

impl fmt::Display for Method {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
