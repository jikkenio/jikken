use serde_json::{json, Map, Value};
use std::error::Error;

pub fn filter_json(path: &str, depth: usize, json: serde_json::Value) -> Result<serde_json::Value, Box<dyn Error + Send + Sync>> {
    let path_segments: Vec<&str> = path.split(".").collect();

    // println!("path ({}), depth({}), json({})", path, depth, json);

    if depth + 1 > path_segments.len() { return Ok(json) }
    
    if path_segments.len() == depth + 1 {
        let segment = path_segments[depth];

        // println!("segment ({})", segment);
        match json {
            serde_json::Value::Object(_) => {
                let mut map: Map<String, Value> = serde_json::from_value(json)?;
                
                if map.contains_key(segment) {
                    map.remove(segment);
                }
                
                return Ok(json!(map));
            },
            serde_json::Value::Array(a) => {
                let mut results = Vec::new();
                
                for item in a.into_iter() {
                    match item {
                        serde_json::Value::Object(_) => {
                            results.push(filter_json(path, depth, item)?);
                        },
                        _ => results.push(item)
                    }
                }

                return Ok(json!(results));
            },
            _ => return Ok(json)
        }
    }

    let current_segment = path_segments[depth];
    // println!("current_segment ({})", current_segment);
    match json {
        serde_json::Value::Object(_) => {
            let mut map: Map<String, Value> = serde_json::from_value(json)?;
            
            if map.contains_key(current_segment) {
                let result = filter_json(path, depth + 1, map.get(current_segment).unwrap_or(&serde_json::Value::Null).clone())?;
                map.remove(current_segment);
                map.insert(current_segment.to_string(), result);
            }

            return Ok(json!(map));
        },
        serde_json::Value::Array(a) => {
            let mut results = Vec::new();
            
            for item in a.into_iter() {
                match item {
                    serde_json::Value::Object(_) => {
                        results.push(filter_json(path, depth, item)?);
                    },
                    _ => results.push(item)
                }
            }

            return Ok(json!(results));
        },
        _ => return Ok(json)
    }
}


#[cfg(test)]
mod test {
    use crate::json_filter;

    #[tokio::test]
    async fn object() {
        let input_data = r#"{
            "test": "name",
            "items": [{
                "one": 1,
                "two": 2
            },
            {
                "one": 3,
                "two": 4
            },
            {
                "one": 5,
                "two": 6
            }]
        }"#;

        let expected_data = r#"{
            "test": "name"
        }"#;
    
        let result = json_filter::filter_json("items", 0, serde_json::from_str(input_data).unwrap()).unwrap();
        let expected_result: serde_json::Value = serde_json::from_str(expected_data).unwrap();
        assert_eq!(result, expected_result);
    }

    #[tokio::test]
    async fn array_object() {
        let input_data = r#"[{
            "test": "name",
            "items": [{
                "one": 1,
                "two": 2
            },
            {
                "one": 3,
                "two": 4
            },
            {
                "one": 5,
                "two": 6
            }]
        },{
            "test": "name2",
            "items": []
        }]"#;

        let expected_data = r#"[{
            "test": "name"
        },{
            "test": "name2"
        }]"#;
    
        let result = json_filter::filter_json("items", 0, serde_json::from_str(input_data).unwrap()).unwrap();
        let expected_result: serde_json::Value = serde_json::from_str(expected_data).unwrap();
        assert_eq!(result, expected_result);
    }

    #[tokio::test]
    async fn object_array_object() {
        let input_data = r#"{
            "test": "name",
            "items": [{
                "one": 1,
                "two": 2
            },
            {
                "one": 3,
                "two": 4
            },
            {
                "one": 5,
                "two": 6
            }]
        }"#;

        let expected_data = r#"{
            "test": "name",
            "items": [{
                "one": 1
            },
            {
                "one": 3
            },
            {
                "one": 5
            }]
        }"#;
    
        let result = json_filter::filter_json("items.two", 0, serde_json::from_str(input_data).unwrap()).unwrap();
        let expected_result: serde_json::Value = serde_json::from_str(expected_data).unwrap();
        assert_eq!(result, expected_result);
    }

    #[tokio::test]
    async fn array_object_array_object() {
        let input_data = r#"[{
            "test": "name",
            "items": [{
                "one": 1,
                "two": 2
            },
            {
                "one": 3,
                "two": 4
            },
            {
                "one": 5,
                "two": 6
            }]
        },{
            "test": "name2",
            "items": [{
                "one": 1,
                "two": 10
            }]
        }]"#;

        let expected_data = r#"[{
            "test": "name",
            "items": [{
                "one": 1
            },
            {
                "one": 3
            },
            {
                "one": 5
            }]
        },{
            "test": "name2",
            "items": [{
                "one": 1
            }]
        }]"#;
    
        let result = json_filter::filter_json("items.two", 0, serde_json::from_str(input_data).unwrap()).unwrap();
        let expected_result: serde_json::Value = serde_json::from_str(expected_data).unwrap();
        assert_eq!(result, expected_result);
    }

    #[tokio::test]
    async fn no_matches() {
        let input_data = r#"[{
            "test": "name",
            "items": [{
                "one": 1,
                "two": 2
            },
            {
                "one": 3,
                "two": 4
            },
            {
                "one": 5,
                "two": 6
            }]
        },{
            "test": "name2",
            "items": [{
                "one": 1,
                "two": 10
            }]
        }]"#;

        let result = json_filter::filter_json("items.three", 0, serde_json::from_str(input_data).unwrap()).unwrap();
        let expected_result: serde_json::Value = serde_json::from_str(input_data).unwrap();
        assert_eq!(result, expected_result);
    }
}