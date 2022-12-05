use crate::errors::ValidationError;
use crate::test_file::{
    UnvalidatedRequest, UnvalidatedResponse, UnvalidatedTest, UnvalidatedVariable,
};
use chrono::{offset::TimeZone, Days, Local, Months, NaiveDate};
use hyper::Method;
use serde::{Deserialize, Serialize};
use std::cell::Cell;

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
pub struct HttpHeader {
    pub header: String,
    pub value: String,

    matches_variable: Cell<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HttpParameter {
    pub param: String,
    pub value: String,

    matches_variable: Cell<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RequestDescriptor {
    pub method: HttpVerb,
    pub url: String,
    pub params: Vec<HttpParameter>,
    pub headers: Vec<HttpHeader>,
    pub body: Option<serde_json::Value>,
}

// TODO: add validation logic to verify the descriptor is valid
impl RequestDescriptor {
    pub fn new(request: UnvalidatedRequest) -> Result<RequestDescriptor, ValidationError> {
        let validated_params = match request.params {
            Some(params) => params
                .iter()
                .map(|v| HttpParameter {
                    param: v.param.clone(),
                    value: v.value.clone(),
                    matches_variable: Cell::from(false),
                })
                .collect(),
            None => Vec::new(),
        };

        let validated_headers = match request.headers {
            Some(headers) => headers
                .iter()
                .map(|h| HttpHeader {
                    header: h.header.clone(),
                    value: h.value.clone(),
                    matches_variable: Cell::from(false),
                })
                .collect(),
            None => Vec::new(),
        };

        Ok(RequestDescriptor {
            method: request.method.unwrap_or(HttpVerb::Get),
            url: request.url,
            params: validated_params,
            headers: validated_headers,
            body: request.body,
        })
    }

    pub fn new_opt(
        request: Option<UnvalidatedRequest>,
    ) -> Result<Option<RequestDescriptor>, ValidationError> {
        match request {
            Some(req) => match RequestDescriptor::new(req) {
                Ok(result) => Ok(Some(result)),
                Err(e) => Err(e),
            },
            None => Ok(None),
        }
    }

    pub fn validate(&self) -> bool {
        true
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResponseDescriptor {
    pub status: Option<u16>,
    pub headers: Vec<HttpHeader>,
    pub body: Option<serde_json::Value>,
    pub ignore: Vec<String>,
}

// TODO: add validation logic to verify the descriptor is valid
impl ResponseDescriptor {
    pub fn new_opt(
        request: Option<UnvalidatedResponse>,
    ) -> Result<Option<ResponseDescriptor>, ValidationError> {
        match request {
            Some(req) => {
                let validated_headers = match req.headers {
                    Some(headers) => headers
                        .iter()
                        .map(|h| HttpHeader {
                            header: h.header.clone(),
                            value: h.value.clone(),
                            matches_variable: Cell::from(false),
                        })
                        .collect(),
                    None => Vec::new(),
                };

                let validated_ignore = match req.ignore {
                    Some(ignore) => ignore,
                    None => Vec::new(),
                };

                Ok(Some(ResponseDescriptor {
                    status: req.status,
                    headers: validated_headers,
                    body: req.body,
                    ignore: validated_ignore,
                }))
            }
            None => Ok(None),
        }
    }

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
pub struct Range {
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
    pub value: serde_yaml::Value,
    pub modifier: Option<Modifier>,
    pub format: Option<String>,

    #[serde(skip_serializing, skip_deserializing)]
    index: Cell<u32>,
}

impl TestVariable {
    pub fn new(variable: &UnvalidatedVariable) -> Result<TestVariable, ValidationError> {
        // TODO: Add validation errors
        Ok(TestVariable {
            name: variable.name.clone(),
            data_type: variable.data_type.clone(),
            value: variable.value.clone(),
            modifier: variable.modifier.clone(),
            format: variable.format.clone(),
            index: Cell::from(0),
        })
    }

    pub fn generate_value(&self) -> String {
        let result = match self.data_type {
            VariableTypes::Int => self.generate_int_value(),
            VariableTypes::String => self.generate_string_value(),
            VariableTypes::Date => self.generate_date_value(),
            VariableTypes::Datetime => String::from(""),
        };

        // println!("result: {}", result);

        return result;
    }

    fn generate_int_value(&self) -> String {
        match &self.value {
            serde_yaml::Value::Number(v) => {
                // println!("number expression: {:?}", v);
                return format!("{}", v);
            }
            serde_yaml::Value::Sequence(seq) => {
                // println!("sequence expression: {:?}", seq);
                // println!("current index: {}", self.index.get());

                if seq.len() < (self.index.get() + 1) as usize {
                    self.index.set(0);
                }

                let test = &seq[self.index.get() as usize];
                let test_string = match test {
                    serde_yaml::Value::Number(st) => st.as_i64().unwrap_or(0),
                    _ => 0,
                };

                // println!("test_number: {}", test_string);
                self.index.set(self.index.get() + 1);
                // println!("new index: {}", self.index.get());
                return format!("{}", test_string);
            }
            serde_yaml::Value::Mapping(map) => {
                println!("map expression: {:?}", map);
                return String::from("no");
            }
            _ => {
                return String::from("");
            }
        }
    }

    fn generate_string_value(&self) -> String {
        match &self.value {
            serde_yaml::Value::String(v) => {
                // println!("number expression: {:?}", v);
                return format!("{}", v);
            }
            serde_yaml::Value::Sequence(seq) => {
                // println!("sequence expression: {:?}", seq);
                // println!("current index: {}", self.index.get());

                if seq.len() < (self.index.get() + 1) as usize {
                    self.index.set(0);
                }

                let test = &seq[self.index.get() as usize];
                let test_string = match test {
                    serde_yaml::Value::String(st) => st.to_string(),
                    _ => "".to_string(),
                };

                // println!("test_number: {}", test_string);
                self.index.set(self.index.get() + 1);
                // println!("new index: {}", self.index.get());
                return format!("{}", test_string);
            }
            serde_yaml::Value::Mapping(map) => {
                println!("map expression: {:?}", map);
                return String::from("no");
            }
            _ => {
                return String::from("");
            }
        }
    }

    fn generate_date_value(&self) -> String {
        // TODO: Add proper error handling
        match &self.value {
            serde_yaml::Value::String(v) => {
                // println!("string expression: {:?}", v);
                let mut result_date;
                
                let parse_attempt = NaiveDate::parse_from_str(&v, "%Y-%m-%d");
                if let Ok(p) = parse_attempt {
                    result_date = Local
                        .from_local_datetime(&p.and_hms_opt(0, 0, 0).unwrap())
                        .unwrap();
                } else {
                    return String::from("");
                }

                // TODO: Change modifiers to static types with enums
                if let Some(m) = &self.modifier {
                    let mod_value_result = m.value.parse::<u64>();
                    if let Ok(mod_value) = mod_value_result {
                        match m.operation.to_lowercase().as_str() {
                            "add" => {
                                let modified_date = match m.unit.to_lowercase().as_str() {
                                    "days" => result_date.checked_add_days(Days::new(mod_value)),
                                    "weeks" => {
                                        result_date.checked_add_days(Days::new(mod_value * 7))
                                    }
                                    "months" => result_date
                                        .checked_add_months(Months::new(mod_value as u32)),
                                    // TODO: add support for years
                                    _ => None,
                                };

                                if let Some(md) = modified_date {
                                    result_date = md;
                                }
                            }
                            "subtract" => {
                                let modified_date = match m.unit.to_lowercase().as_str() {
                                    "days" => result_date.checked_sub_days(Days::new(mod_value)),
                                    "weeks" => {
                                        result_date.checked_sub_days(Days::new(mod_value * 7))
                                    }
                                    "months" => result_date
                                        .checked_sub_months(Months::new(mod_value as u32)),
                                    // TODO: add support for years
                                    _ => None,
                                };

                                if let Some(md) = modified_date {
                                    result_date = md;
                                }
                            }
                            _ => {}
                        }
                    }
                }
                return format!("{}", result_date.format("%Y-%m-%d"));
            }
            serde_yaml::Value::Sequence(seq) => {
                // println!("sequence expression: {:?}", seq);
                if seq.len() < (self.index.get() + 1) as usize {
                    self.index.set(0);
                }

                let test = &seq[self.index.get() as usize];

                let test_string: &str = match test {
                    serde_yaml::Value::String(st) => st,
                    _ => "",
                };

                // println!("test_string: {}", test_string);

                let parse_attempt = NaiveDate::parse_from_str(test_string, "%Y-%m-%d");
                self.index.set(self.index.get() + 1);

                match parse_attempt {
                    Ok(p) => {
                        return format!(
                            "{}",
                            Local
                                .from_local_datetime(&p.and_hms_opt(0, 0, 0).unwrap())
                                .unwrap()
                                .format("%Y-%m-%d")
                        );
                    }
                    Err(e) => {
                        println!("parse_attempt failed");
                        println!("{}", e);
                        return String::from("");
                    }
                }
            }
            serde_yaml::Value::Mapping(map) => {
                println!("map expression: {:?}", map);
                return String::from("no");
            }
            _ => {
                return String::from("");
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TestDefinition {
    pub name: Option<String>,
    pub iterate: u32,
    pub request: RequestDescriptor,
    pub compare: Option<RequestDescriptor>,
    pub response: Option<ResponseDescriptor>,
    pub variables: Vec<TestVariable>,
}

// TODO: add validation logic to verify the descriptor is valid
// TODO: Validation should be type driven for compile time correctness
impl TestDefinition {
    pub fn new(test: UnvalidatedTest) -> Result<TestDefinition, ValidationError> {
        let td = TestDefinition {
            name: test.name,
            iterate: test.iterate.unwrap_or(1),
            request: RequestDescriptor::new(test.request)?,
            compare: RequestDescriptor::new_opt(test.compare)?,
            response: ResponseDescriptor::new_opt(test.response)?,
            variables: TestDefinition::validate_variables_opt(test.variables)?,
        };

        td.update_variable_matching();
        Ok(td)
    }

    fn validate_variables_opt(
        variables: Option<Vec<UnvalidatedVariable>>,
    ) -> Result<Vec<TestVariable>, ValidationError> {
        match variables {
            None => Ok(Vec::new()),
            Some(vars) => {
                let count = vars.len();
                let results = vars
                    .iter()
                    .map(|v| TestVariable::new(v))
                    .filter_map(|v| match v {
                        Ok(x) => Some(x),
                        Err(_) => None,
                    })
                    .collect::<Vec<TestVariable>>();
                if results.len() != count {
                    Err(ValidationError)
                } else {
                    Ok(results)
                }
            }
        }
    }

    fn update_variable_matching(&self) {
        // println!("updating variable matching");
        for variable in self.variables.iter() {
            let var_pattern = format!("${}$", variable.name.trim());
            // println!("pattern: {}", var_pattern);

            for header in self.request.headers.iter() {
                if header.value.contains(var_pattern.as_str()) {
                    header.matches_variable.set(true);
                    // println!("setting match true: {}", header.header);
                }
            }

            for param in self.request.params.iter() {
                if param.value.contains(var_pattern.as_str()) {
                    param.matches_variable.set(true);
                }
            }

            if let Some(compare) = &self.compare {
                for header in compare.headers.iter() {
                    if header.value.contains(var_pattern.as_str()) {
                        header.matches_variable.set(true);

                    }                    
                }

                for param in compare.params.iter() {
                    if param.value.contains(var_pattern.as_str()) {
                        param.matches_variable.set(true);
                        // println!("setting match true: {}", param.param);
                    }                
                }
            }

            if let Some(response) = &self.response {
                for header in response.headers.iter() {
                    if header.value.contains(var_pattern.as_str()) {
                        header.matches_variable.set(true);
                    }
                }
            }
        }
    }

    pub fn get_request_url(&self) -> String {
        // TODO: inject variable replacement
        let joined: Vec<_> = self
            .request
            .params
            .iter()
            .map(|param| {
                if param.matches_variable.get() {
                    let p = self.get_processed_param(param);
                    format!("{}={}", p.0, p.1)
                } else {
                    format!("{}={}", param.param, param.value)
                }
            })
            .collect();

        format!("{}?{}", self.request.url, joined.join("&"))
    }

    pub fn get_compare_url(&self) -> String {
        // TODO: make this safe with optionals
        let joined: Vec<_> = self
            .compare.clone().unwrap()
            .params
            .iter()
            .map(|param| {
                if param.matches_variable.get() {
                    let p = self.get_processed_param(param);
                    format!("{}={}", p.0, p.1)
                } else {
                    format!("{}={}", param.param, param.value)
                }
            })
            .collect();

        format!("{}?{}", self.compare.clone().unwrap().url, joined.join("&"))
    }

    fn get_processed_param(&self, parameter: &HttpParameter) -> (String, String) {
        // println!("processing param: {}", parameter.param);
        for variable in self.variables.iter() {
            let var_pattern = format!("${}$", variable.name);

            if !parameter.value.contains(var_pattern.as_str()) {
                continue;
            }

            let replacement = variable.generate_value();
            return (parameter.param.clone(), parameter.value.replace(var_pattern.as_str(), replacement.as_str()));            
        }

        (String::from(""), String::from(""))
    }

    pub fn get_request_headers(&self) -> Vec<(String, String)> {
        // TODO: inject variable replacement
        self.request
            .headers
            .iter()
            .map(|kvp| (kvp.header.clone(), kvp.value.clone()))
            .collect()
    }

    pub fn get_compare_headers(&self) -> Vec<(String, String)> {
        // TODO: inject variable replacement
        self.compare.clone().unwrap()
            .headers
            .iter()
            .map(|kvp| (kvp.header.clone(), kvp.value.clone()))
            .collect()
    }

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
}
