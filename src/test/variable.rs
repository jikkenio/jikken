use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Hash, PartialOrd)]
pub struct Modifier {
    pub operation: String,
    pub value: String,
    pub unit: String,
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
