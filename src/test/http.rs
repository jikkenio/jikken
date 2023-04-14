use hyper::Method;
use serde::{Deserialize, Serialize};
use std::cell::Cell;
use std::hash::{Hash, Hasher};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
    Get,
    Post,
    Put,
    Patch,
    Undefined,
}

impl Verb {
    pub fn as_method(&self) -> Method {
        match &self {
            Verb::Post => Method::POST,
            Verb::Patch => Method::PATCH,
            Verb::Put => Method::PUT,
            _ => Method::GET,
        }
    }
}
