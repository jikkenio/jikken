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
                if let Ok(index) = segment.parse::<usize>() {
                    return a
                        .get(index)
                        .cloned()
                        .ok_or_else(|| Box::from("array index out of bounds".to_string()) as Box<dyn Error + Send + Sync>);
                }

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
            if let Ok(index) = current_segment.parse::<usize>() {
                return a
                    .get(index)
                    .cloned()
                    .ok_or_else(|| Box::from("array index out of bounds".to_string()) as Box<dyn Error + Send + Sync>)
                    .and_then(|item| extract_json(path, depth + 1, item));
            }

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

#[cfg(test)]
mod test {
    use super::extract_json;
    use serde_json::json;

    // === Existing behavior: object extraction ===

    #[test]
    fn extract_simple_field_from_object() {
        let data = json!({"name": "alice"});
        let result = extract_json("name", 0, data).unwrap();
        assert_eq!(result, json!("alice"));
    }

    #[test]
    fn extract_nested_field_from_object() {
        let data = json!({"auth": {"token": "abc123"}});
        let result = extract_json("auth.token", 0, data).unwrap();
        assert_eq!(result, json!("abc123"));
    }

    #[test]
    fn extract_deeply_nested_field() {
        let data = json!({"a": {"b": {"c": 42}}});
        let result = extract_json("a.b.c", 0, data).unwrap();
        assert_eq!(result, json!(42));
    }

    #[test]
    fn extract_number_value() {
        let data = json!({"count": 5});
        let result = extract_json("count", 0, data).unwrap();
        assert_eq!(result, json!(5));
    }

    #[test]
    fn extract_bool_value() {
        let data = json!({"active": true});
        let result = extract_json("active", 0, data).unwrap();
        assert_eq!(result, json!(true));
    }

    #[test]
    fn extract_missing_field_returns_error() {
        let data = json!({"name": "alice"});
        assert!(extract_json("missing", 0, data).is_err());
    }

    #[test]
    fn extract_missing_nested_field_returns_error() {
        let data = json!({"a": {"b": 1}});
        assert!(extract_json("a.missing", 0, data).is_err());
    }

    // === Existing behavior: field from nested array (iterate all items) ===

    #[test]
    fn extract_field_from_nested_array() {
        let data = json!({"data": [{"id": 1}, {"id": 2}, {"id": 3}]});
        let result = extract_json("data.id", 0, data).unwrap();
        assert_eq!(result, json!([1, 2, 3]));
    }

    #[test]
    fn extract_field_from_root_array_iterates_all() {
        let data = json!([{"field": "a"}, {"field": "b"}]);
        let result = extract_json("field", 0, data).unwrap();
        assert_eq!(result, json!(["a", "b"]));
    }

    // === New behavior: numeric index into arrays ===

    #[test]
    fn extract_first_element_from_root_array() {
        let data = json!([{"field": "value1"}, {"field": "value2"}]);
        let result = extract_json("0", 0, data).unwrap();
        assert_eq!(result, json!({"field": "value1"}));
    }

    #[test]
    fn extract_second_element_from_root_array() {
        let data = json!([{"field": "value1"}, {"field": "value2"}]);
        let result = extract_json("1", 0, data).unwrap();
        assert_eq!(result, json!({"field": "value2"}));
    }

    #[test]
    fn extract_field_from_root_array_by_index() {
        let data = json!([{"field": "value1"}, {"field": "value2"}]);
        let result = extract_json("0.field", 0, data).unwrap();
        assert_eq!(result, json!("value1"));
    }

    #[test]
    fn extract_field_from_root_array_second_item() {
        let data = json!([{"field": "value1"}, {"field": "value2"}]);
        let result = extract_json("1.field", 0, data).unwrap();
        assert_eq!(result, json!("value2"));
    }

    #[test]
    fn extract_nested_field_via_array_index() {
        let data = json!({"data": [{"id": 10}, {"id": 20}]});
        let result = extract_json("data.0.id", 0, data).unwrap();
        assert_eq!(result, json!(10));
    }

    #[test]
    fn extract_nested_field_via_array_index_second() {
        let data = json!({"data": [{"id": 10}, {"id": 20}]});
        let result = extract_json("data.1.id", 0, data).unwrap();
        assert_eq!(result, json!(20));
    }

    #[test]
    fn extract_deep_nested_with_multiple_indices() {
        let data = json!({"items": [{"sub": [{"val": "deep"}]}]});
        let result = extract_json("items.0.sub.0.val", 0, data).unwrap();
        assert_eq!(result, json!("deep"));
    }

    #[test]
    fn extract_index_out_of_bounds_returns_error() {
        let data = json!([{"field": "value"}]);
        assert!(extract_json("5.field", 0, data).is_err());
    }

    #[test]
    fn extract_index_out_of_bounds_final_segment() {
        let data = json!({"data": [1, 2]});
        assert!(extract_json("data.10", 0, data).is_err());
    }

    #[test]
    fn extract_root_array_of_scalars_by_index() {
        let data = json!([100, 200, 300]);
        let result = extract_json("1", 0, data).unwrap();
        assert_eq!(result, json!(200));
    }
}
