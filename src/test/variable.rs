use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub enum Type {
    #[serde(alias = "int", alias = "INT")]
    Int,
    #[serde(alias = "string", alias = "STRING")]
    String,
    #[serde(alias = "date", alias = "DATE")]
    Date,
    #[serde(alias = "datetime", alias = "DATETIME")]
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

pub fn parse_source_path<'a>(path: &'a str) -> String {
    let index = path.rfind("/");

    let mut result = match index {
        Some(i) => path[0..i].to_string(),
        None => "./".to_string(),
    };

    if !result.ends_with("/") {
        result = format!("{}/", result);
    }

    result
}
