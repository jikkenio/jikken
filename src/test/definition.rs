use crate::errors::ValidationError;
use crate::test::file::{
    TestFile, UnvalidatedCleanup, UnvalidatedCompareRequest, UnvalidatedRequest,
    UnvalidatedRequestResponse, UnvalidatedResponse, UnvalidatedStage, UnvalidatedVariable,
};
use crate::test::http::{HttpHeader, HttpParameter, HttpVerb};
use chrono::{offset::TimeZone, Days, Local, Months, NaiveDate};
use log::{debug, error, trace};
use serde::{Deserialize, Serialize};
use std::cell::Cell;
use std::collections::HashSet;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RequestBody {
    pub data: serde_json::Value,
    matches_variable: Cell<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RequestDescriptor {
    pub method: HttpVerb,
    pub url: String,
    pub params: Vec<HttpParameter>,
    pub headers: Vec<HttpHeader>,
    pub body: Option<RequestBody>,
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

        let request_body = match request.body {
            Some(b) => Some(RequestBody {
                data: b,
                matches_variable: Cell::from(false),
            }),
            None => None,
        };

        Ok(RequestDescriptor {
            method: request.method.unwrap_or(HttpVerb::Get),
            url: request.url,
            params: validated_params,
            headers: validated_headers,
            body: request_body,
        })
    }

    pub fn new_opt(
        request_opt: Option<UnvalidatedRequest>,
    ) -> Result<Option<RequestDescriptor>, ValidationError> {
        match request_opt {
            Some(request) => Ok(Some(RequestDescriptor::new(request)?)),
            None => Ok(None),
        }
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
    pub body: Option<RequestBody>,
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

                let compare_body = match request.body {
                    Some(b) => Some(RequestBody {
                        data: b,
                        matches_variable: Cell::from(false),
                    }),
                    None => None,
                };

                Ok(Some(CompareDescriptor {
                    method: request.method.unwrap_or(HttpVerb::Get),
                    url: request.url,
                    params: validated_params,
                    add_params: validated_add_params,
                    ignore_params: validated_ignore_params,
                    headers: validated_headers,
                    add_headers: validated_add_headers,
                    ignore_headers: validated_ignore_headers,
                    body: compare_body,
                }))
            }
            None => Ok(None),
        }
    }

    pub fn validate(&self) -> bool {
        true
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub struct ResponseExtraction {
    pub name: String,
    pub field: String,
}

impl ResponseExtraction {
    pub fn new() -> ResponseExtraction {
        ResponseExtraction {
            name: "".to_string(),
            field: "".to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResponseDescriptor {
    pub status: Option<u16>,
    pub headers: Vec<HttpHeader>,
    pub body: Option<RequestBody>,
    pub ignore: Vec<String>,
    pub extract: Vec<ResponseExtraction>,
}

// TODO: add validation logic to verify the descriptor is valid
impl ResponseDescriptor {
    pub fn new_opt(
        response: Option<UnvalidatedResponse>,
    ) -> Result<Option<ResponseDescriptor>, ValidationError> {
        match response {
            Some(res) => {
                let validated_headers = match res.headers {
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

                let validated_ignore = match res.ignore {
                    Some(ignore) => ignore,
                    None => Vec::new(),
                };

                let validated_extraction = match res.extract {
                    Some(extract) => extract,
                    None => Vec::new(),
                };

                let response_body = match res.body {
                    Some(b) => Some(RequestBody {
                        data: b,
                        matches_variable: Cell::from(false),
                    }),
                    None => None,
                };

                Ok(Some(ResponseDescriptor {
                    status: res.status,
                    headers: validated_headers,
                    body: response_body,
                    ignore: validated_ignore,
                    extract: validated_extraction,
                }))
            }
            None => Ok(None),
        }
    }

    pub fn validate(&self) -> bool {
        true
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Hash)]
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TestVariable {
    pub name: String,
    pub data_type: VariableTypes,
    pub value: serde_yaml::Value,
    pub modifier: Option<Modifier>,
    pub format: Option<String>,
}

impl TestVariable {
    pub fn new(variable: UnvalidatedVariable) -> Result<TestVariable, ValidationError> {
        // TODO: Add validation errors
        Ok(TestVariable {
            name: variable.name.clone(),
            data_type: variable.data_type.clone(),
            value: variable.value.clone(),
            modifier: variable.modifier.clone(),
            format: variable.format.clone(),
        })
    }

    pub fn validate_variables_opt(
        variables: Option<Vec<UnvalidatedVariable>>,
    ) -> Result<Vec<TestVariable>, ValidationError> {
        match variables {
            None => Ok(Vec::new()),
            Some(vars) => {
                let count = vars.len();
                let results = vars
                    .into_iter()
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

    pub fn generate_value(&self, iteration: u32, global_variables: Vec<TestVariable>) -> String {
        let result = match self.data_type {
            VariableTypes::Int => self.generate_int_value(iteration),
            VariableTypes::String => self.generate_string_value(iteration, global_variables),
            VariableTypes::Date => self.generate_date_value(iteration, global_variables),
            VariableTypes::Datetime => String::from(""),
        };

        debug!("generate_value result: {}", result);

        return result;
    }

    fn generate_int_value(&self, iteration: u32) -> String {
        match &self.value {
            serde_yaml::Value::Number(v) => {
                debug!("number expression: {:?}", v);
                return format!("{}", v);
            }
            serde_yaml::Value::Sequence(seq) => {
                debug!("sequence expression: {:?}", seq);
                let test = &seq[iteration as usize];
                let test_string = match test {
                    serde_yaml::Value::Number(st) => st.as_i64().unwrap_or(0),
                    _ => 0,
                };
                return format!("{}", test_string);
            }
            serde_yaml::Value::Mapping(map) => {
                debug!("map expression: {:?}", map);
                return String::from("");
            }
            _ => {
                return String::from("");
            }
        }
    }

    fn generate_string_value(&self, iteration: u32, global_variables: Vec<TestVariable>) -> String {
        match &self.value {
            serde_yaml::Value::String(v) => {
                debug!("string expression: {:?}", v);

                if v.contains("$") {
                    let mut modified_value = v.clone();
                    for variable in global_variables.iter() {
                        let var_pattern = format!("${}$", variable.name.trim());
                        if !modified_value.contains(&var_pattern) {
                            continue;
                        }

                        if let serde_yaml::Value::String(s) = &variable.value {
                            modified_value = modified_value.replace(&var_pattern, &s);
                        }
                    }

                    return format!("{}", modified_value);
                }

                return format!("{}", v);
            }
            serde_yaml::Value::Sequence(seq) => {
                debug!("sequence expression: {:?}", seq);
                let test = &seq[iteration as usize];
                let test_string = match test {
                    serde_yaml::Value::String(st) => st.to_string(),
                    _ => "".to_string(),
                };

                if test_string.contains("$") {
                    let mut modified_value = test_string;
                    for variable in global_variables.iter() {
                        let var_pattern = format!("${}$", variable.name.trim());
                        if !modified_value.contains(&var_pattern) {
                            continue;
                        }

                        if let serde_yaml::Value::String(s) = &variable.value {
                            modified_value = modified_value.replace(&var_pattern, &s);
                        }
                    }

                    return format!("{}", modified_value);
                }

                return format!("{}", test_string);
            }
            serde_yaml::Value::Mapping(map) => {
                debug!("map expression: {:?}", map);
                return String::from("");
            }
            _ => {
                return String::from("");
            }
        }
    }

    fn generate_date_value(&self, iteration: u32, global_variables: Vec<TestVariable>) -> String {
        // TODO: Add proper error handling
        match &self.value {
            serde_yaml::Value::String(v) => {
                debug!("string expression: {:?}", v);
                let mut result_date;

                let modified_value = if v.contains("$") {
                    let mut mv = v.clone();
                    for variable in global_variables.iter() {
                        let var_pattern = format!("${}$", variable.name.trim());
                        if !mv.contains(&var_pattern) {
                            continue;
                        }

                        if let serde_yaml::Value::String(s) = variable.value.clone() {
                            mv = mv.replace(&var_pattern, &s);
                        }
                    }

                    format!("{}", mv)
                } else {
                    v.to_string()
                };

                let parse_attempt = NaiveDate::parse_from_str(&modified_value, "%Y-%m-%d");
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
                debug!("sequence expression: {:?}", seq);
                let test = &seq[iteration as usize];

                let test_string: &str = match test {
                    serde_yaml::Value::String(st) => st,
                    _ => "",
                };

                let modified_string = if test_string.contains("$") {
                    let mut modified_value: String = test_string.to_string();
                    for variable in global_variables.iter() {
                        let var_pattern = format!("${}$", variable.name.trim());
                        if !modified_value.contains(&var_pattern) {
                            continue;
                        }

                        if let serde_yaml::Value::String(s) = &variable.value {
                            modified_value = modified_value.replace(&var_pattern, &s);
                        }
                    }

                    format!("{}", modified_value)
                } else {
                    test_string.to_string()
                };

                let parse_attempt = NaiveDate::parse_from_str(&modified_string, "%Y-%m-%d");

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
                        error!("parse_attempt failed");
                        error!("{}", e);
                        return String::from("");
                    }
                }
            }
            serde_yaml::Value::Mapping(map) => {
                debug!("map expression: {:?}", map);
                return String::from("");
            }
            _ => {
                return String::from("");
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StageDescriptor {
    pub request: RequestDescriptor,
    pub compare: Option<CompareDescriptor>,
    pub response: Option<ResponseDescriptor>,
    pub variables: Vec<TestVariable>,
}

impl StageDescriptor {
    pub fn new(stage: UnvalidatedStage) -> Result<StageDescriptor, ValidationError> {
        Ok(StageDescriptor {
            request: RequestDescriptor::new(stage.request)?,
            compare: CompareDescriptor::new_opt(stage.compare)?,
            response: ResponseDescriptor::new_opt(stage.response)?,
            variables: TestVariable::validate_variables_opt(stage.variables)?,
        })
    }

    pub fn validate_stages_opt(
        request_opt: Option<UnvalidatedRequest>,
        compare_opt: Option<UnvalidatedCompareRequest>,
        response_opt: Option<UnvalidatedResponse>,
        stages_opt: Option<Vec<UnvalidatedStage>>,
    ) -> Result<Vec<StageDescriptor>, ValidationError> {
        let mut results = Vec::new();
        let mut count = 0;

        if let Some(request) = request_opt {
            results.push(StageDescriptor {
                request: RequestDescriptor::new(request)?,
                compare: CompareDescriptor::new_opt(compare_opt)?,
                response: ResponseDescriptor::new_opt(response_opt)?,
                variables: Vec::new(),
            });
            count += 1;
        }

        match stages_opt {
            None => Ok(results),
            Some(stages) => {
                count += stages.len();
                results.append(
                    &mut stages
                        .into_iter()
                        .map(|s| StageDescriptor::new(s))
                        .filter_map(|v| match v {
                            Ok(x) => Some(x),
                            Err(_) => None,
                        })
                        .collect::<Vec<StageDescriptor>>(),
                );
                if results.len() != count {
                    Err(ValidationError)
                } else {
                    Ok(results)
                }
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RequestResponseDescriptor {
    pub request: RequestDescriptor,
    pub response: Option<ResponseDescriptor>,
}

impl RequestResponseDescriptor {
    pub fn new_opt(
        reqresp_opt: Option<UnvalidatedRequestResponse>,
    ) -> Result<Option<RequestResponseDescriptor>, ValidationError> {
        match reqresp_opt {
            Some(reqresp) => Ok(Some(RequestResponseDescriptor {
                request: RequestDescriptor::new(reqresp.request)?,
                response: ResponseDescriptor::new_opt(reqresp.response)?,
            })),
            None => Ok(None),
        }
    }

    pub fn validate(&self) -> bool {
        true
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CleanupDescriptor {
    pub onsuccess: Option<RequestDescriptor>,
    pub onfailure: Option<RequestDescriptor>,
    pub request: Option<RequestDescriptor>,
}

impl CleanupDescriptor {
    pub fn new(
        cleanup_opt: Option<UnvalidatedCleanup>,
    ) -> Result<CleanupDescriptor, ValidationError> {
        match cleanup_opt {
            Some(cleanup) => Ok(CleanupDescriptor {
                onsuccess: RequestDescriptor::new_opt(cleanup.onsuccess)?,
                onfailure: RequestDescriptor::new_opt(cleanup.onfailure)?,
                request: RequestDescriptor::new_opt(cleanup.request)?,
            }),
            None => Ok(CleanupDescriptor {
                onsuccess: None,
                onfailure: None,
                request: None,
            }),
        }
    }

    pub fn validate(&self) -> bool {
        true
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TestDefinition {
    pub name: Option<String>,
    pub id: String,
    pub environment: Option<String>,
    pub requires: Option<String>,
    pub tags: Vec<String>,
    pub iterate: u32,
    pub variables: Vec<TestVariable>,
    pub global_variables: Vec<TestVariable>,
    pub stages: Vec<StageDescriptor>,
    pub setup: Option<RequestResponseDescriptor>,
    pub cleanup: CleanupDescriptor,
}

// TODO: add validation logic to verify the descriptor is valid
// TODO: Validation should be type driven for compile time correctness
impl TestDefinition {
    pub fn new(
        test: TestFile,
        global_variables: Vec<TestVariable>,
    ) -> Result<TestDefinition, ValidationError> {
        let new_tags = if let Some(tags) = test.tags.as_ref() {
            tags.to_lowercase()
                .split_whitespace()
                .map(|s| s.to_string())
                .collect()
        } else {
            Vec::new()
        };

        let generated_id = test.generate_id();

        let td = TestDefinition {
            name: test.name,
            id: test.id.unwrap_or(generated_id).to_lowercase(),
            environment: test.env,
            requires: test.requires,
            tags: new_tags,
            iterate: test.iterate.unwrap_or(1),
            variables: TestVariable::validate_variables_opt(test.variables)?,
            global_variables: global_variables,
            stages: StageDescriptor::validate_stages_opt(
                test.request,
                test.compare,
                test.response,
                test.stages,
            )?,
            setup: RequestResponseDescriptor::new_opt(test.setup)?,
            cleanup: CleanupDescriptor::new(test.cleanup)?,
        };

        td.update_variable_matching();
        Ok(td)
    }

    fn update_request_variables(request: &RequestDescriptor, var_pattern: &str) {
        for header in request.headers.iter() {
            if header.matches_variable.get() {
                continue;
            }

            if header.value.contains(var_pattern) {
                header.matches_variable.set(true);
                debug!("setting match true: {}", header.header);
            }
        }

        for param in request.params.iter() {
            if param.matches_variable.get() {
                continue;
            }

            if param.value.contains(var_pattern) {
                param.matches_variable.set(true);
                debug!("setting match true: {}", param.param);
            }
        }

        if let Some(body) = request.body.as_ref() {
            let body_data = match serde_json::to_string(&body.data) {
                Ok(s) => Some(s),
                Err(_) => None,
            };

            if let Some(b) = &body_data {
                if b.contains(var_pattern) {
                    body.matches_variable.set(true);
                    debug!("request body match true: {}", var_pattern);
                }
            }
        }
    }

    fn update_compare_variables(compare: &CompareDescriptor, var_pattern: &str) {
        for header in compare.headers.iter() {
            if header.matches_variable.get() {
                continue;
            }

            if header.value.contains(var_pattern) {
                header.matches_variable.set(true);
                debug!("setting match true: {}", header.header);
            }
        }

        for param in compare.params.iter() {
            if param.matches_variable.get() {
                continue;
            }

            if param.value.contains(var_pattern) {
                param.matches_variable.set(true);
                debug!("setting match true: {}", param.param);
            }
        }

        if let Some(body) = &compare.body {
            let body_data = match serde_json::to_string(&body.data) {
                Ok(s) => Some(s),
                Err(_) => None,
            };

            if let Some(b) = &body_data {
                if b.contains(var_pattern) {
                    body.matches_variable.set(true);
                    debug!("compare body match true: {}", var_pattern);
                }
            }
        }
    }

    fn update_response_variables(response: &ResponseDescriptor, var_pattern: &str) {
        for header in response.headers.iter() {
            if header.matches_variable.get() {
                continue;
            }

            if header.value.contains(var_pattern) {
                header.matches_variable.set(true);
                debug!("setting match true: {}", header.header);
            }
        }

        if let Some(body) = response.body.as_ref() {
            let body_data = match serde_json::to_string(&body.data) {
                Ok(s) => Some(s),
                Err(_) => None,
            };

            if let Some(b) = &body_data {
                if b.contains(var_pattern) {
                    body.matches_variable.set(true);
                    debug!("response body match true: {}", var_pattern);
                }
            }
        }
    }

    fn update_variable_matching(&self) {
        trace!("scanning test definition for variable pattern matches");

        for variable in self.variables.iter().chain(self.global_variables.iter()) {
            let var_pattern = format!("${}$", variable.name.trim());
            // debug!("pattern: {}", var_pattern);

            if let Some(setup) = self.setup.as_ref() {
                TestDefinition::update_request_variables(&setup.request, var_pattern.as_str());

                if let Some(response) = &setup.response {
                    TestDefinition::update_response_variables(&response, var_pattern.as_str());
                }
            }

            if let Some(request) = &self.cleanup.request {
                TestDefinition::update_request_variables(&request, var_pattern.as_str());
            }

            if let Some(onsuccess) = &self.cleanup.onsuccess {
                TestDefinition::update_request_variables(&onsuccess, var_pattern.as_str());
            }

            if let Some(onfailure) = &self.cleanup.onfailure {
                TestDefinition::update_request_variables(&onfailure, var_pattern.as_str());
            }
        }

        for stage in self.stages.iter() {
            for variable in stage
                .variables
                .iter()
                .chain(self.variables.iter().chain(self.global_variables.iter()))
            {
                let var_pattern = format!("${}$", variable.name.trim());
                // debug!("pattern: {}", var_pattern);

                TestDefinition::update_request_variables(&stage.request, var_pattern.as_str());

                if let Some(compare) = &stage.compare {
                    TestDefinition::update_compare_variables(&compare, var_pattern.as_str());
                }

                if let Some(response) = &stage.response {
                    TestDefinition::update_response_variables(&response, var_pattern.as_str());
                }
            }
        }
    }

    pub fn get_url(
        &self,
        iteration: u32,
        url: &str,
        params: &Vec<HttpParameter>,
        variables: &Vec<TestVariable>,
    ) -> String {
        let joined: Vec<_> = params
            .iter()
            .map(|param| {
                if param.matches_variable.get() {
                    let p = self.get_processed_param(param, iteration);
                    format!("{}={}", p.0, p.1)
                } else {
                    format!("{}={}", param.param, param.value)
                }
            })
            .collect();

        let modified_url = if url.contains("$") {
            let mut replaced_url = url.to_string();

            for variable in variables.iter().chain(self.global_variables.iter()) {
                let var_pattern = format!("${}$", variable.name);

                if !replaced_url.contains(var_pattern.as_str()) {
                    continue;
                }

                let replacement = variable.generate_value(iteration, self.global_variables.clone());
                replaced_url = replaced_url
                    .replace(var_pattern.as_str(), replacement.as_str())
                    .clone()
            }

            replaced_url
        } else {
            url.to_string()
        };

        if joined.len() > 0 {
            format!("{}?{}", modified_url, joined.join("&"))
        } else {
            modified_url.to_string()
        }
    }

    fn get_processed_param(&self, parameter: &HttpParameter, iteration: u32) -> (String, String) {
        for variable in self.variables.iter().chain(self.global_variables.iter()) {
            let var_pattern = format!("${}$", variable.name);

            if !parameter.value.contains(var_pattern.as_str()) {
                continue;
            }

            let replacement = variable.generate_value(iteration, self.global_variables.clone());
            return (
                parameter.param.clone(),
                parameter
                    .value
                    .replace(var_pattern.as_str(), replacement.as_str()),
            );
        }

        (String::from(""), String::from(""))
    }

    fn get_processed_header(&self, header: &HttpHeader, iteration: u32) -> (String, String) {
        for variable in self.variables.iter().chain(self.global_variables.iter()) {
            let var_pattern = format!("${}$", variable.name);

            if !header.value.contains(var_pattern.as_str()) {
                continue;
            }

            let replacement = variable.generate_value(iteration, self.global_variables.clone());
            return (
                header.header.clone(),
                header
                    .value
                    .replace(var_pattern.as_str(), replacement.as_str()),
            );
        }

        (String::from(""), String::from(""))
    }

    pub fn get_setup_request_headers(&self, iteration: u32) -> Vec<(String, String)> {
        match self.setup.as_ref() {
            Some(setup) => setup
                .request
                .headers
                .iter()
                .map(|h| {
                    if h.matches_variable.get() {
                        let header = self.get_processed_header(h, iteration);
                        (header.0, header.1)
                    } else {
                        (h.header.clone(), h.value.clone())
                    }
                })
                .collect(),
            None => Vec::new(),
        }
    }

    pub fn get_headers(&self, headers: &Vec<HttpHeader>, iteration: u32) -> Vec<(String, String)> {
        headers
            .iter()
            .map(|h| {
                if h.matches_variable.get() {
                    let header = self.get_processed_header(h, iteration);
                    (header.0, header.1)
                } else {
                    (h.header.clone(), h.value.clone())
                }
            })
            .collect()
    }

    pub fn get_cleanup_request_headers(&self, iteration: u32) -> Vec<(String, String)> {
        match &self.cleanup.request {
            Some(request) => self.get_headers(&request.headers, iteration),
            None => Vec::new(),
        }
    }

    pub fn get_stage_compare_headers(
        &self,
        stage_index: usize,
        iteration: u32,
    ) -> Vec<(String, String)> {
        let stage = self.stages.get(stage_index).unwrap();
        match stage.compare.as_ref() {
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
                                let header = self.get_processed_header(h, iteration);
                                (header.0, header.1)
                            } else {
                                (h.header.clone(), h.value.clone())
                            }
                        })
                        .collect();
                } else {
                    results = stage
                        .request
                        .headers
                        .iter()
                        .filter(|h| !ignore_lookup.contains(&h.header))
                        .chain(compare.add_headers.iter())
                        .map(|h| {
                            if h.matches_variable.get() {
                                let header = self.get_processed_header(h, iteration);
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

    pub fn get_body(
        &self,
        request: &RequestDescriptor,
        variables: &Vec<TestVariable>,
        iteration: u32,
    ) -> Option<serde_json::Value> {
        if let Some(body) = &request.body {
            if !body.matches_variable.get() {
                return Some(body.data.clone());
            }

            let mut body_str = match serde_json::to_string(&body.data) {
                Ok(s) => s,
                Err(_) => "".to_string(),
            };

            for variable in variables.iter().chain(self.global_variables.iter()) {
                let var_pattern = format!("${}$", variable.name);

                if !body_str.contains(var_pattern.as_str()) {
                    continue;
                }

                let replacement = variable.generate_value(iteration, self.global_variables.clone());
                body_str = body_str.replace(var_pattern.as_str(), replacement.as_str());
            }

            return match serde_json::from_str(body_str.as_str()) {
                Ok(result) => Some(result),
                Err(_) => None,
            };
        }

        None
    }

    pub fn get_compare_body(
        &self,
        compare: &CompareDescriptor,
        variables: &Vec<TestVariable>,
        iteration: u32,
    ) -> Option<serde_json::Value> {
        if let Some(body) = &compare.body {
            if !body.matches_variable.get() {
                return Some(body.data.clone());
            }

            let mut body_str = match serde_json::to_string(&body.data) {
                Ok(s) => s,
                Err(_) => "".to_string(),
            };

            for variable in variables.iter().chain(self.global_variables.iter()) {
                let var_pattern = format!("${}$", variable.name);

                if !body_str.contains(var_pattern.as_str()) {
                    continue;
                }

                let replacement = variable.generate_value(iteration, self.global_variables.clone());
                body_str = body_str.replace(var_pattern.as_str(), replacement.as_str());
            }

            return match serde_json::from_str(body_str.as_str()) {
                Ok(result) => Some(result),
                Err(_) => None,
            };
        }

        None
    }

    pub fn validate(&self) -> bool {
        trace!("validating test definition");
        let mut valid_td = true;

        if let Some(setup) = &self.setup {
            valid_td &= setup.validate();
        }

        valid_td &= self.cleanup.validate();

        for stage in self.stages.iter() {
            valid_td &= stage.request.validate();

            if let Some(compare) = &stage.compare {
                valid_td &= compare.validate();
            }

            if let Some(resp) = &stage.response {
                valid_td &= resp.validate();
            }
        }

        valid_td
    }
}