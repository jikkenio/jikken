use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::hash::Hash;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Modifier {
    pub operation: String,
    pub value: Value,
    pub unit: String,
}

impl Hash for Modifier {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        serde_json::to_string(self).unwrap().hash(state);
    }
}

impl Modifier {
    //We need an enum for modifier
    pub fn get_inverse(&self) -> Modifier {
        match self.operation.as_str() {
            "add" => Self {
                operation: "subtract".to_string(),
                value: self.value.clone(),
                unit: self.unit.clone(),
            },
            "subtract" => Self {
                operation: "add".to_string(),
                value: self.value.clone(),
                unit: self.unit.clone(),
            },
            _ => self.clone(),
        }
    }
}

pub fn parse_source_path(path: &str) -> String {
    let index = path.rfind('/');

    let mut result = match index {
        Some(i) => path[0..i].to_string(),
        None => "./".to_string(),
    };

    if !result.ends_with('/') {
        result = format!("{}/", result);
    }

    result
}
