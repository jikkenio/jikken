use hyper::Method;
use serde::{Deserialize, Serialize};
use chrono::{NaiveDateTime, Local, offset::TimeZone, Days, Months};
use std::collections::{HashMap};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum HttpVerb {
    Get,
    Post,
    Put,
    Patch,
    Undefined,
}

impl HttpVerb {
    pub fn as_method(&self) -> Method {
        match &self {
            HttpVerb::Post => Method::POST,
            HttpVerb::Patch => Method::PATCH,
            HttpVerb::Put => Method::PUT,
            _ => Method::GET,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HttpKvp {
    pub key: Option<String>,
    pub value: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RequestDescriptor {
    pub method: Option<HttpVerb>,
    pub url: String,
    pub params: Option<Vec<HttpKvp>>,
    pub headers: Option<Vec<HttpKvp>>,
    pub body: Option<serde_json::Value>,
}

// TODO: add validation logic to verify the descriptor is valid
impl RequestDescriptor {
    pub fn validate(&self) -> bool {
        true
    }

    pub fn get_url(&self) -> String {
        let joined: Vec<_> = match &self.params {
            Some(p) => p
                .iter()
                .map(|kvp| {
                    format!(
                        "{}={}",
                        kvp.key.as_ref().unwrap(),
                        kvp.value.as_ref().unwrap()
                    )
                })
                .collect(),
            _ => Vec::new(),
        };

        format!("{}?{}", self.url, joined.join("&"))
    }

    pub fn get_headers(&self) -> Vec<(String, String)> {
        match &self.headers {
            Some(h) => h
                .iter()
                .filter(|kvp| {
                    if kvp.key.as_ref().unwrap_or(&String::from("")) == "" {
                        return false;
                    }
                    if kvp.value.as_ref().unwrap_or(&String::from("")) == "" {
                        return false;
                    }
                    true
                })
                .map(|kvp| {
                    (
                        kvp.key.as_ref().unwrap().clone(),
                        kvp.value.as_ref().unwrap().clone(),
                    )
                })
                .collect(),
            _ => Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResponseDescriptor {
    pub status: Option<u16>,
    pub headers: Option<Vec<HttpKvp>>,
    pub body: Option<serde_json::Value>,
    pub ignore: Option<Vec<String>>,
}

// TODO: add validation logic to verify the descriptor is valid
impl ResponseDescriptor {
    pub fn validate(&self) -> bool {
        true
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum VariableTypes {
    Int,
    String,
    Date,
    Datetime,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VariableRange {
    pub min: String,
    pub max: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Modifier {
    pub operation: String,
    pub value: String,
    pub unit: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TestVariable {
    pub name: String,
    pub data_type: VariableTypes,
    pub value: Option<String>,
    pub range: Option<VariableRange>,
    pub modifier: Option<Modifier>,
    pub format: Option<String>,
}

impl TestVariable {
    pub fn generate_value(&self) -> String {
        return match self.data_type {
            VariableTypes::Int => String::from(""),
            VariableTypes::String => String::from(""),
            VariableTypes::Date => {
                // TODO: Add proper error handling
                if let Some(v) = &self.value {
                    if v.starts_with("$") {
                        let mut result_date;
                        if v == "$TODAY" || v == "$NOW" {
                            result_date = Local::now();
                        } else {
                            let parse_attempt = NaiveDateTime::parse_from_str(&v, "%Y-%m-%d");
                            if let Ok(p) = parse_attempt {
                                result_date = Local.from_local_datetime(&p).unwrap();
                            } else {
                                return String::from("");
                            }
                        }

                        // TODO: Change modifiers to static types with enums
                        if let Some(m) = &self.modifier {
                            let mod_value_result = m.value.parse::<u64>();
                            if let Ok(mod_value) = mod_value_result {
                                match m.operation.to_lowercase().as_str() {
                                    "add" => {
                                        let modified_date = match m.unit.to_lowercase().as_str() {
                                            "days" => {
                                                result_date.checked_add_days(Days::new(mod_value))
                                            },
                                            "weeks" => {
                                                result_date.checked_add_days(Days::new(mod_value*7))
                                            },
                                            "months" => {
                                                result_date.checked_add_months(Months::new(mod_value as u32))
                                            },
                                            // TODO: add support for years
                                            _ => {
                                                None
                                            }
                                        };

                                        if let Some(md) = modified_date {
                                            result_date = md;
                                        }
                                    },
                                    "subtract" => {
                                        let modified_date = match m.unit.to_lowercase().as_str() {
                                            "days" => {
                                                result_date.checked_sub_days(Days::new(mod_value))
                                            },
                                            "weeks" => {
                                                result_date.checked_sub_days(Days::new(mod_value*7))
                                            },
                                            "months" => {
                                                result_date.checked_sub_months(Months::new(mod_value as u32))
                                            },
                                            // TODO: add support for years
                                            _ => {
                                                None
                                            }
                                        };

                                        if let Some(md) = modified_date {
                                            result_date = md;
                                        }
                                    },
                                    _ => {

                                    }
                                }
                            }
                        }

                        return format!("{}", result_date.format("%Y-%m-%d"));
                    }
                }
                return String::from("");
            },
            VariableTypes::Datetime => String::from(""),
        };
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TestDescriptor {
    pub name: Option<String>,
    pub request: RequestDescriptor,
    pub compare: Option<RequestDescriptor>,
    pub response: Option<ResponseDescriptor>,
    pub variables: Option<Vec<TestVariable>>,
}

// TODO: add validation logic to verify the descriptor is valid
// TODO: Validation should be type driven for compile time correctness
impl TestDescriptor {
    pub fn validate(&self) -> bool {
        let mut valid_td = self.request.validate();
        if let Some(compare) = &self.compare {
            valid_td &= compare.validate();
        }
        if let Some(resp) = &self.response {
            valid_td &= resp.validate();
        }

        valid_td
    }

    pub fn process_variables(&mut self) {
        let mut variable_map = HashMap::new();

        if let Some(variables) = &self.variables {
            for v in variables {
                variable_map.insert(format!("{{{}}}", &v.name), v.generate_value());
            }
        }

        if variable_map.is_empty() {
            return;
        }

        if let Some(params) = &self.request.params {
            let mut new_params = Vec::new();
            let mut any_modification = false;

            for p in params.iter() {
                let mut new_p = p.clone();
                if let Some(p_value) = &p.value {
                    if p_value.starts_with("{") {
                        for (key, value) in &variable_map {
                            let new_value = p_value.replace(key, &value);

                            if new_value != *p_value {
                                new_p.value = Some(new_value);
                                any_modification = true;
                            }
                        }
                    }
                }

                new_params.push(new_p)
            }

            if any_modification {
                self.request.params = Some(new_params);
            }
        }

        // println!("Resolved url: {}", self.request.get_url());
    }
}
