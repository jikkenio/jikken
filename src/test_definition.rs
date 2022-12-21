use crate::errors::ValidationError;
use crate::test_file::{
    UnvalidatedCompareRequest, UnvalidatedRequest, UnvalidatedResponse, UnvalidatedTest,
    UnvalidatedVariable,
};
use chrono::{offset::TimeZone, Days, Local, Months, NaiveDate};
use hyper::Method;
use log::trace;
use serde::{Deserialize, Serialize};
use std::cell::Cell;
use std::collections::HashSet;

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

    pub fn validate(&self) -> bool {
        true
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompareDescriptor {
    pub method: HttpVerb,
    pub url: String,
    pub params: Vec<HttpParameter>,
    pub add_params: Vec<HttpParameter>,
    pub ignore_params: Vec<String>,
    pub headers: Vec<HttpHeader>,
    pub add_headers: Vec<HttpHeader>,
    pub ignore_headers: Vec<String>,
    pub body: Option<serde_json::Value>,
}

impl CompareDescriptor {
    pub fn new_opt(
        request_opt: Option<UnvalidatedCompareRequest>,
    ) -> Result<Option<CompareDescriptor>, ValidationError> {
        match request_opt {
            Some(request) => {
                let validated_params = match request.params {
                    Some(params) => params
                        .iter()
                        .map(|p| HttpParameter {
                            param: p.param.clone(),
                            value: p.value.clone(),
                            matches_variable: Cell::from(false),
                        })
                        .collect(),
                    None => Vec::new(),
                };

                let mut validated_add_params = Vec::new();
                let mut validated_ignore_params = Vec::new();

                if validated_params.len() == 0 {
                    validated_add_params = match request.add_params {
                        Some(params) => params
                            .iter()
                            .map(|p| HttpParameter {
                                param: p.param.clone(),
                                value: p.value.clone(),
                                matches_variable: Cell::from(false),
                            })
                            .collect(),
                        None => Vec::new(),
                    };

                    validated_ignore_params = match request.ignore_params {
                        Some(params) => params.iter().map(|p| p.clone()).collect(),
                        None => Vec::new(),
                    };
                }

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

                let mut validated_add_headers = Vec::new();
                let mut validated_ignore_headers = Vec::new();

                if validated_headers.len() == 0 {
                    validated_add_headers = match request.add_headers {
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

                    validated_ignore_headers = match request.ignore_headers {
                        Some(headers) => headers.iter().map(|h| h.clone()).collect(),
                        None => Vec::new(),
                    };
                }

                Ok(Some(CompareDescriptor {
                    method: request.method.unwrap_or(HttpVerb::Get),
                    url: request.url,
                    params: validated_params,
                    add_params: validated_add_params,
                    ignore_params: validated_ignore_params,
                    headers: validated_headers,
                    add_headers: validated_add_headers,
                    ignore_headers: validated_ignore_headers,
                    body: request.body,
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
pub struct ResponseDescriptor {
    pub status: Option<u16>,
    pub headers: Vec<HttpHeader>,
    pub body: Option<serde_json::Value>,
    pub ignore: Vec<String>,
}

// TODO: add validation logic to verify the descriptor is valid
impl ResponseDescriptor {
    pub fn new_opt(
        response: Option<UnvalidatedResponse>,
    ) -> Result<Option<ResponseDescriptor>, ValidationError> {
        match response {
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

    pub fn generate_value(&self, update: bool) -> String {
        let result = match self.data_type {
            VariableTypes::Int => self.generate_int_value(update),
            VariableTypes::String => self.generate_string_value(update),
            VariableTypes::Date => self.generate_date_value(update),
            VariableTypes::Datetime => String::from(""),
        };

        trace!("generate_value result: {}", result);

        return result;
    }

    fn generate_int_value(&self, update: bool) -> String {
        match &self.value {
            serde_yaml::Value::Number(v) => {
                trace!("number expression: {:?}", v);
                return format!("{}", v);
            }
            serde_yaml::Value::Sequence(seq) => {
                trace!("sequence expression: {:?}", seq);
                if update && seq.len() < (self.index.get() + 1) as usize {
                    self.index.set(0);
                }

                let test = &seq[self.index.get() as usize];
                let test_string = match test {
                    serde_yaml::Value::Number(st) => st.as_i64().unwrap_or(0),
                    _ => 0,
                };

                if update {
                    self.index.set(self.index.get() + 1);
                }

                return format!("{}", test_string);
            }
            serde_yaml::Value::Mapping(map) => {
                trace!("map expression: {:?}", map);
                return String::from("no");
            }
            _ => {
                return String::from("");
            }
        }
    }

    fn generate_string_value(&self, update: bool) -> String {
        match &self.value {
            serde_yaml::Value::String(v) => {
                trace!("number expression: {:?}", v);
                return format!("{}", v);
            }
            serde_yaml::Value::Sequence(seq) => {
                trace!("sequence expression: {:?}", seq);

                if update && seq.len() < (self.index.get() + 1) as usize {
                    self.index.set(0);
                }

                let test = &seq[self.index.get() as usize];
                let test_string = match test {
                    serde_yaml::Value::String(st) => st.to_string(),
                    _ => "".to_string(),
                };

                if update {
                    self.index.set(self.index.get() + 1);
                }

                return format!("{}", test_string);
            }
            serde_yaml::Value::Mapping(map) => {
                trace!("map expression: {:?}", map);
                return String::from("no");
            }
            _ => {
                return String::from("");
            }
        }
    }

    fn generate_date_value(&self, update: bool) -> String {
        // TODO: Add proper error handling
        match &self.value {
            serde_yaml::Value::String(v) => {
                trace!("string expression: {:?}", v);
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
                trace!("sequence expression: {:?}", seq);
                if update && seq.len() < (self.index.get() + 1) as usize {
                    self.index.set(0);
                }

                let test = &seq[self.index.get() as usize];

                let test_string: &str = match test {
                    serde_yaml::Value::String(st) => st,
                    _ => "",
                };

                let parse_attempt = NaiveDate::parse_from_str(test_string, "%Y-%m-%d");

                if update {
                    self.index.set(self.index.get() + 1);
                }

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
                trace!("map expression: {:?}", map);
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
    pub id: String,
    pub tags: Vec<String>,
    pub iterate: u32,
    pub request: RequestDescriptor,
    pub compare: Option<CompareDescriptor>,
    pub response: Option<ResponseDescriptor>,
    pub variables: Vec<TestVariable>,
}

// TODO: add validation logic to verify the descriptor is valid
// TODO: Validation should be type driven for compile time correctness
impl TestDefinition {
    pub fn new(test: UnvalidatedTest) -> Result<TestDefinition, ValidationError> {
        let new_tags = if let Some(tags) = test.tags {
            tags.to_lowercase()
                .split_whitespace()
                .map(|s| s.to_string())
                .collect()
        } else {
            Vec::new()
        };

        let td = TestDefinition {
            name: test.name,
            id: test.id.unwrap_or("".to_string()),
            tags: new_tags,
            iterate: test.iterate.unwrap_or(1),
            request: RequestDescriptor::new(test.request)?,
            compare: CompareDescriptor::new_opt(test.compare)?,
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
        trace!("updating variable matching");
        for variable in self.variables.iter() {
            let var_pattern = format!("${}$", variable.name.trim());
            trace!("pattern: {}", var_pattern);

            for header in self.request.headers.iter() {
                if header.value.contains(var_pattern.as_str()) {
                    header.matches_variable.set(true);
                    trace!("setting match true: {}", header.header);
                }
            }

            for param in self.request.params.iter() {
                if param.value.contains(var_pattern.as_str()) {
                    param.matches_variable.set(true);
                    trace!("setting match true: {}", param.param);
                }
            }

            if let Some(compare) = &self.compare {
                for header in compare.headers.iter() {
                    if header.value.contains(var_pattern.as_str()) {
                        header.matches_variable.set(true);
                        trace!("compare setting match true: {}", header.header);
                    }
                }

                for header in compare.add_headers.iter() {
                    if header.value.contains(var_pattern.as_str()) {
                        header.matches_variable.set(true);
                        trace!("compare add_header setting match true: {}", header.header);
                    }
                }

                for param in compare.params.iter() {
                    if param.value.contains(var_pattern.as_str()) {
                        param.matches_variable.set(true);
                        trace!("compare setting match true: {}", param.param);
                    }
                }

                for param in compare.add_params.iter() {
                    if param.value.contains(var_pattern.as_str()) {
                        param.matches_variable.set(true);
                        trace!("compare add_param setting match true: {}", param.param);
                    }
                }
            }

            if let Some(response) = &self.response {
                for header in response.headers.iter() {
                    if header.value.contains(var_pattern.as_str()) {
                        header.matches_variable.set(true);
                        trace!("response setting match true: {}", header.header);
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
                    let p = self.get_processed_param(param, true);
                    format!("{}={}", p.0, p.1)
                } else {
                    format!("{}={}", param.param, param.value)
                }
            })
            .collect();

        format!("{}?{}", self.request.url, joined.join("&"))
    }

    pub fn get_compare_url(&self) -> String {
        match self.compare.as_ref() {
            Some(compare) => {
                let ignore_lookup: HashSet<String> =
                    compare.ignore_params.iter().cloned().collect();

                let joined: Vec<String>;

                if compare.params.len() > 0 {
                    joined = compare
                        .params
                        .iter()
                        .map(|param| {
                            if param.matches_variable.get() {
                                let p = self.get_processed_param(param, false);
                                format!("{}={}", p.0, p.1)
                            } else {
                                format!("{}={}", param.param, param.value)
                            }
                        })
                        .collect();
                } else {
                    joined = self
                        .request
                        .params
                        .iter()
                        .filter(|p| !ignore_lookup.contains(&p.param))
                        .chain(compare.add_params.iter())
                        .map(|p| {
                            if p.matches_variable.get() {
                                let param = self.get_processed_param(p, false);
                                format!("{}={}", param.0, param.1)
                            } else {
                                format!("{}={}", p.param, p.value)
                            }
                        })
                        .collect();
                }

                format!("{}?{}", self.compare.clone().unwrap().url, joined.join("&"))
            }
            None => String::from(""),
        }
    }

    fn get_processed_param(&self, parameter: &HttpParameter, update: bool) -> (String, String) {
        // println!("processing param: {}", parameter.param);
        for variable in self.variables.iter() {
            let var_pattern = format!("${}$", variable.name);

            if !parameter.value.contains(var_pattern.as_str()) {
                continue;
            }

            let replacement = variable.generate_value(update);
            return (
                parameter.param.clone(),
                parameter
                    .value
                    .replace(var_pattern.as_str(), replacement.as_str()),
            );
        }

        (String::from(""), String::from(""))
    }

    fn get_processed_header(&self, header: &HttpHeader, update: bool) -> (String, String) {
        for variable in self.variables.iter() {
            let var_pattern = format!("${}$", variable.name);

            if !header.value.contains(var_pattern.as_str()) {
                continue;
            }

            let replacement = variable.generate_value(update);
            return (
                header.header.clone(),
                header
                    .value
                    .replace(var_pattern.as_str(), replacement.as_str()),
            );
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

        match self.compare.as_ref() {
            Some(compare) => {
                let ignore_lookup: HashSet<String> =
                    compare.ignore_headers.iter().cloned().collect();

                let results: Vec<(String, String)>;

                if compare.headers.len() > 0 {
                    results = compare
                        .headers
                        .iter()
                        .map(|h| {
                            if h.matches_variable.get() {
                                let header = self.get_processed_header(h, false);
                                (header.0, header.1)
                            } else {
                                (h.header.clone(), h.value.clone())
                            }
                        })
                        .collect();
                } else {
                    results = self
                        .request
                        .headers
                        .iter()
                        .filter(|h| !ignore_lookup.contains(&h.header))
                        .chain(compare.add_headers.iter())
                        .map(|h| {
                            if h.matches_variable.get() {
                                let header = self.get_processed_header(h, false);
                                (header.0, header.1)
                            } else {
                                (h.header.clone(), h.value.clone())
                            }
                        })
                        .collect();
                }

                results
            }
            None => Vec::new(),
        }
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
