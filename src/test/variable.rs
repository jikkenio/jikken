use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub enum Type {
    Int,
    String,
    Date,
    Datetime,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Range {
    pub min: String,
    pub max: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub struct Modifier {
    pub operation: String,
    pub value: String,
    pub unit: String,
}

impl Modifier {
    pub fn new() -> Modifier {
        Modifier {
            operation: "".to_string(),
            value: "".to_string(),
            unit: "".to_string(),
        }
    }
}
