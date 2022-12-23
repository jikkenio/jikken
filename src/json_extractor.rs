use serde_json::{json, Map, Value};
use std::error::Error;

pub fn extract_json(path: &str, depth: usize, json: serde_json::Value) -> Result<serde_json::Value, Box<dyn Error + Send + Sync>> {
    let path_segments: Vec<&str> = path.split(".").collect();

    // println!("path ({}), depth({}), json({})", path, depth, json);

    if depth + 1 > path_segments.len() { return Ok(json) }
    
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
            },
            serde_json::Value::Array(a) => {
                let mut results = Vec::new();
                
                for item in a.into_iter() {
                    match item {
                        serde_json::Value::Object(_) => {
                            results.push(extract_json(path, depth, item)?);
                        },
                        _ => {}
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
            let map: Map<String, Value> = serde_json::from_value(json)?;
            
            if map.contains_key(current_segment) {
                return Ok(extract_json(path, depth + 1, map.get(current_segment).unwrap_or(&serde_json::Value::Null).clone())?);
            }

            return Err(Box::from("path not found".to_string()));
        },
        serde_json::Value::Array(a) => {
            let mut results: Vec<serde_json::Value> = Vec::new();
            
            for item in a.into_iter() {
                match item {
                    serde_json::Value::Object(_) => {
                        match extract_json(path, depth, item) {
                            Ok(r) => results.push(r),
                            _ => {}
                        }
                    },
                    _ => {}
                }
            }

            // println!("results ({:?})", results.clone());
            let mut flattened = Vec::new();

            for r in results.to_owned() {
                match r {
                    serde_json::Value::Array(a) => {
                        for i in a {
                            flattened.push(i);
                        }
                    },
                    _ => {}
                }
            }

            if flattened.len() > 0 {
                return Ok(json!(flattened));
            }

            if results.len() > 0 {
                return Ok(json!(results));
            }

            return Err(Box::from("path not found".to_string()))
        },
        _ => return Err(Box::from("path not found".to_string()))
    }
}

#[cfg(test)]
mod test {
    use crate::json_extractor;

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

        let expected_result = serde_json::Value::String("name".to_string());
    
        let result = json_extractor::extract_json("test", 0, serde_json::from_str(input_data).unwrap()).unwrap();
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

        let expected_data = r#"[1, 3, 5]"#;
    
        let result = json_extractor::extract_json("items.one", 0, serde_json::from_str(input_data).unwrap()).unwrap();
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

        let expected_data = r#"[2, 4, 6]"#;
    
        let result = json_extractor::extract_json("items.two", 0, serde_json::from_str(input_data).unwrap()).unwrap();
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

        let expected_data = r#"[2, 4, 6, 10]"#;
    
        let result = json_extractor::extract_json("items.two", 0, serde_json::from_str(input_data).unwrap()).unwrap();
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

        let result = json_extractor::extract_json("items.three", 0, serde_json::from_str(input_data).unwrap());
        match result {
            Ok(o) => {
                println!("okay: {:?}", o);
                assert!(false)
            },
            Err(_) => assert!(true)
        }
    }
}