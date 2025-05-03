use serde_json::{json, Map, Value};
use std::error::Error;

pub fn extract_json(
    path: &str,
    depth: usize,
    json: serde_json::Value,
) -> Result<serde_json::Value, Box<dyn Error + Send + Sync>> {
    let path_segments: Vec<&str> = path.split('.').collect();

    // println!("path ({}), depth({}), json({})", path, depth, json);

    if depth + 1 > path_segments.len() {
        return Ok(json);
    }

    if path_segments.len() == depth + 1 {
        let segment = path_segments[depth];

        // println!("segment ({})", segment);
        match json {
            serde_json::Value::Object(_) => {
                let map: Map<String, Value> = serde_json::from_value(json)?;

                if map.contains_key(segment) {
                    return Ok(map.get(segment).unwrap().to_owned());
                }

                return Err(Box::from("path not found".to_string()));
            }
            serde_json::Value::Array(a) => {
                let mut results = Vec::new();

                for item in a.into_iter() {
                    if let serde_json::Value::Object(_) = item {
                        results.push(extract_json(path, depth, item)?);
                    }
                }

                return Ok(json!(results));
            }
            _ => return Ok(json.clone()),
        }
    }

    let current_segment = path_segments[depth];
    // println!("current_segment ({})", current_segment);
    match json {
        serde_json::Value::Object(_) => {
            let map: Map<String, Value> = serde_json::from_value(json)?;

            if map.contains_key(current_segment) {
                return extract_json(
                    path,
                    depth + 1,
                    map.get(current_segment)
                        .unwrap_or(&serde_json::Value::Null)
                        .clone(),
                );
            }

            Err(Box::from("path not found".to_string()))
        }
        serde_json::Value::Array(a) => {
            let mut results: Vec<serde_json::Value> = Vec::new();

            for item in a.into_iter() {
                if let serde_json::Value::Object(_) = item {
                    if let Ok(r) = extract_json(path, depth, item) {
                        results.push(r)
                    }
                }
            }

            let mut flattened = Vec::new();

            for r in results.iter().cloned() {
                if let serde_json::Value::Array(a) = r {
                    for i in a {
                        flattened.push(i);
                    }
                }
            }

            if !flattened.is_empty() {
                return Ok(json!(flattened));
            }

            if !results.is_empty() {
                return Ok(json!(results));
            }

            Err(Box::from("path not found".to_string()))
        }
        _ => Err(Box::from("path not found".to_string())),
    }
}
