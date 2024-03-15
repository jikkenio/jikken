pub mod definition;
pub mod file;
pub mod http;
pub mod template;
pub mod validation;
pub mod variable;

use crate::test::definition::RequestBody;
use chrono::{offset::TimeZone, Days, Local, Months, NaiveDate};
use log::{debug, error, trace};
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::collections::HashSet;
use std::hash::{Hash, Hasher};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
pub struct File {
    pub name: Option<String>,
    pub id: Option<String>,
    pub project: Option<String>,
    pub env: Option<String>,
    pub tags: Option<String>,
    pub requires: Option<String>,
    pub iterate: Option<u32>,
    pub setup: Option<file::UnvalidatedRequestResponse>,
    pub request: Option<file::UnvalidatedRequest>,
    pub compare: Option<file::UnvalidatedCompareRequest>,
    pub response: Option<file::UnvalidatedResponse>,
    pub stages: Option<Vec<file::UnvalidatedStage>>,
    pub cleanup: Option<file::UnvalidatedCleanup>,
    pub variables: Option<Vec<file::UnvalidatedVariable>>,
    pub disabled: Option<bool>,

    #[serde(skip_serializing, skip_deserializing)]
    pub filename: String,
}

impl File {
    pub fn generate_id(&self) -> String {
        let mut s = DefaultHasher::new();
        self.hash(&mut s);
        format!("{}", s.finish())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Variable {
    pub name: String,
    #[serde(rename = "type")]
    pub data_type: variable::Type,
    pub value: serde_yaml::Value,
    pub modifier: Option<variable::Modifier>,
    pub format: Option<String>,
    pub file: Option<String>,

    #[serde(skip_serializing)]
    pub source_path: String,
}

impl Variable {
    pub fn new(
        variable: file::UnvalidatedVariable,
        source_path: &str,
    ) -> Result<Variable, validation::Error> {
        // TODO: Add validation errors
        Ok(Variable {
            name: variable.name.clone(),
            data_type: variable.data_type.unwrap_or(variable::Type::String).clone(),
            value: variable
                .value
                .unwrap_or(serde_yaml::from_str("{}").unwrap())
                .clone(),
            modifier: variable.modifier.clone(),
            format: variable.format,
            file: variable.file,
            source_path: source_path.to_string(),
        })
    }

    pub fn validate_variables_opt(
        variables: Option<Vec<file::UnvalidatedVariable>>,
        source_path: &str,
    ) -> Result<Vec<Variable>, validation::Error> {
        match variables {
            None => Ok(Vec::new()),
            Some(vars) => {
                let count = vars.len();
                let results = vars
                    .into_iter()
                    .map(|f| Variable::new(f, source_path))
                    .filter_map(|v| match v {
                        Ok(x) => Some(x),
                        Err(_) => None,
                    })
                    .collect::<Vec<Variable>>();
                if results.len() != count {
                    Err(validation::Error {
                        reason: "blah".to_string(),
                    })
                } else {
                    Ok(results)
                }
            }
        }
    }

    pub fn generate_value(&self, iteration: u32, global_variables: Vec<Variable>) -> String {
        let result = match self.data_type {
            variable::Type::Int => self.generate_int_value(iteration),
            variable::Type::String => self.generate_string_value(iteration, global_variables),
            variable::Type::Date => self.generate_date_value(iteration, global_variables),
            variable::Type::Datetime => String::from(""),
        };

        debug!("generate_value result: {}", result);

        result
    }

    fn generate_int_value(&self, iteration: u32) -> String {
        match &self.value {
            serde_yaml::Value::Number(v) => {
                debug!("number expression: {:?}", v);
                format!("{}", v)
            }
            serde_yaml::Value::Sequence(seq) => {
                debug!("sequence expression: {:?}", seq);
                let test = &seq[iteration as usize];
                let test_string = match test {
                    serde_yaml::Value::Number(st) => st.as_i64().unwrap_or(0),
                    _ => 0,
                };
                format!("{}", test_string)
            }
            serde_yaml::Value::Mapping(map) => {
                debug!("map expression: {:?}", map);
                String::from("")
            }
            _ => String::from(""),
        }
    }

    fn generate_string_value(&self, iteration: u32, global_variables: Vec<Variable>) -> String {
        match &self.file {
            Some(f) => {
                let file = if Path::new(f).exists() {
                    f.clone()
                } else {
                    format!("{}{}", self.source_path, f)
                };

                match std::fs::read_to_string(&file) {
                    Ok(file_data) => file_data.trim().to_string(),
                    Err(e) => {
                        error!("error loading file ({}) content: {}", file, e);

                        "".to_string()
                    }
                }
            }
            None => match &self.value {
                serde_yaml::Value::String(v) => {
                    debug!("string expression: {:?}", v);

                    if v.contains('$') {
                        let mut modified_value = v.clone();
                        for variable in global_variables.iter() {
                            let var_pattern = format!("${{{}}}", variable.name.trim());
                            if !modified_value.contains(&var_pattern) {
                                continue;
                            }

                            if let serde_yaml::Value::String(s) = &variable.value {
                                modified_value = modified_value.replace(&var_pattern, s);
                            }
                        }

                        return modified_value;
                    }

                    v.to_string()
                }
                serde_yaml::Value::Sequence(seq) => {
                    debug!("sequence expression: {:?}", seq);
                    let test = &seq[iteration as usize];
                    let test_string = match test {
                        serde_yaml::Value::String(st) => st.to_string(),
                        _ => "".to_string(),
                    };

                    if test_string.contains('$') {
                        let mut modified_value = test_string;
                        for variable in global_variables.iter() {
                            let var_pattern = format!("${{{}}}", variable.name.trim());
                            if !modified_value.contains(&var_pattern) {
                                continue;
                            }

                            if let serde_yaml::Value::String(s) = &variable.value {
                                modified_value = modified_value.replace(&var_pattern, s);
                            }
                        }

                        return modified_value;
                    }

                    test_string
                }
                serde_yaml::Value::Mapping(map) => {
                    debug!("map expression: {:?}", map);
                    String::from("")
                }
                _ => String::from(""),
            },
        }
    }

    fn generate_date_value(&self, iteration: u32, global_variables: Vec<Variable>) -> String {
        // TODO: Add proper error handling
        match &self.value {
            serde_yaml::Value::String(v) => {
                debug!("string expression: {:?}", v);
                let mut result_date;

                let modified_value = if v.contains('$') {
                    let mut mv = v.clone();
                    for variable in global_variables.iter() {
                        let var_pattern = format!("${{{}}}", variable.name.trim());
                        if !mv.contains(&var_pattern) {
                            continue;
                        }

                        if let serde_yaml::Value::String(s) = variable.value.clone() {
                            mv = mv.replace(&var_pattern, &s);
                        }
                    }

                    mv
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
                format!("{}", result_date.format("%Y-%m-%d"))
            }
            serde_yaml::Value::Sequence(seq) => {
                debug!("sequence expression: {:?}", seq);
                let test = &seq[iteration as usize];

                let test_string: &str = match test {
                    serde_yaml::Value::String(st) => st,
                    _ => "",
                };

                let modified_string = if test_string.contains('$') {
                    let mut modified_value: String = test_string.to_string();
                    for variable in global_variables.iter() {
                        let var_pattern = format!("${{{}}}", variable.name.trim());
                        if !modified_value.contains(&var_pattern) {
                            continue;
                        }

                        if let serde_yaml::Value::String(s) = &variable.value {
                            modified_value = modified_value.replace(&var_pattern, s);
                        }
                    }

                    modified_value
                } else {
                    test_string.to_string()
                };

                let parse_attempt = NaiveDate::parse_from_str(&modified_string, "%Y-%m-%d");

                match parse_attempt {
                    Ok(p) => {
                        format!(
                            "{}",
                            Local
                                .from_local_datetime(&p.and_hms_opt(0, 0, 0).unwrap())
                                .unwrap()
                                .format("%Y-%m-%d")
                        )
                    }
                    Err(e) => {
                        error!("parse_attempt failed");
                        error!("{}", e);
                        String::from("")
                    }
                }
            }
            serde_yaml::Value::Mapping(map) => {
                debug!("map expression: {:?}", map);
                String::from("")
            }
            _ => String::from(""),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Definition {
    pub name: Option<String>,
    pub id: String,
    pub project: Option<String>,
    pub environment: Option<String>,
    pub requires: Option<String>,
    pub tags: Vec<String>,
    pub iterate: u32,
    pub variables: Vec<Variable>,
    pub global_variables: Vec<Variable>,
    pub stages: Vec<definition::StageDescriptor>,
    pub setup: Option<definition::RequestResponseDescriptor>,
    pub cleanup: definition::CleanupDescriptor,
    pub disabled: bool,

    #[serde(skip_serializing, skip_deserializing)]
    pub filename: String,
}

// TODO: add validation logic to verify the descriptor is valid
// TODO: Validation should be type driven for compile time correctness
impl Definition {
    fn update_request_variables(request: &definition::RequestDescriptor, var_pattern: &str) {
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

    fn update_compare_variables(compare: &definition::CompareDescriptor, var_pattern: &str) {
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

    fn update_response_variables(response: &definition::ResponseDescriptor, var_pattern: &str) {
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
            let var_pattern = format!("${{{}}}", variable.name.trim());
            // debug!("pattern: {}", var_pattern);

            if let Some(setup) = self.setup.as_ref() {
                Definition::update_request_variables(&setup.request, var_pattern.as_str());

                if let Some(response) = &setup.response {
                    Definition::update_response_variables(response, var_pattern.as_str());
                }
            }

            if let Some(always) = &self.cleanup.always {
                Definition::update_request_variables(always, var_pattern.as_str());
            }

            if let Some(onsuccess) = &self.cleanup.onsuccess {
                Definition::update_request_variables(onsuccess, var_pattern.as_str());
            }

            if let Some(onfailure) = &self.cleanup.onfailure {
                Definition::update_request_variables(onfailure, var_pattern.as_str());
            }
        }

        for stage in self.stages.iter() {
            for variable in stage
                .variables
                .iter()
                .chain(self.variables.iter().chain(self.global_variables.iter()))
            {
                let var_pattern = format!("${{{}}}", variable.name.trim());
                // debug!("pattern: {}", var_pattern);

                Definition::update_request_variables(&stage.request, var_pattern.as_str());

                if let Some(compare) = &stage.compare {
                    Definition::update_compare_variables(compare, var_pattern.as_str());
                }

                if let Some(response) = &stage.response {
                    Definition::update_response_variables(response, var_pattern.as_str());
                }
            }
        }
    }

    pub fn get_url(
        &self,
        iteration: u32,
        url: &str,
        params: &[http::Parameter],
        variables: &[Variable],
    ) -> String {
        let joined: Vec<_> = params
            .iter()
            .map(|param| {
                let p = self.get_processed_param(param, iteration);
                format!("{}={}", p.0, p.1)
            })
            .collect();

        let modified_url = if url.contains('$') {
            let mut replaced_url = url.to_string();

            for variable in variables.iter().chain(self.global_variables.iter()) {
                let var_pattern = format!("${{{}}}", variable.name);

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

        if !joined.is_empty() {
            format!("{}?{}", modified_url, joined.join("&"))
        } else {
            modified_url
        }
    }

    fn get_processed_param(&self, parameter: &http::Parameter, iteration: u32) -> (String, String) {
        if parameter.matches_variable.get() {
            for variable in self.variables.iter().chain(self.global_variables.iter()) {
                let var_pattern = format!("${{{}}}", variable.name);

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
        }

        (parameter.param.clone(), parameter.value.clone())
    }

    fn get_processed_header(&self, header: &http::Header, iteration: u32) -> (String, String) {
        for variable in self.variables.iter().chain(self.global_variables.iter()) {
            let var_pattern = format!("${{{}}}", variable.name);

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

    pub fn get_headers(&self, headers: &[http::Header], iteration: u32) -> Vec<(String, String)> {
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
        match &self.cleanup.always {
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
                let results = if !compare.headers.is_empty() {
                    compare
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
                        .collect()
                } else {
                    let ignore_lookup: HashSet<String> =
                        compare.ignore_headers.iter().cloned().collect();

                    stage
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
                        .collect()
                };

                results
            }
            None => Vec::new(),
        }
    }

    pub fn get_body(
        &self,
        body: &Option<RequestBody>,
        variables: &[Variable],
        iteration: u32,
    ) -> Option<serde_json::Value> {
        if let Some(body) = &body {
            if !body.matches_variable.get() {
                return Some(body.data.clone());
            }

            let mut body_str = match serde_json::to_string(&body.data) {
                Ok(s) => s,
                Err(_) => "".to_string(),
            };

            for variable in variables.iter().chain(self.global_variables.iter()) {
                let var_pattern = format!("${{{}}}", variable.name);

                if !body_str.contains(var_pattern.as_str()) {
                    continue;
                }

                let replacement = variable.generate_value(iteration, self.global_variables.clone());
                body_str = body_str
                    .replace(var_pattern.as_str(), replacement.as_str())
                    .trim()
                    .to_string();
            }

            return match serde_json::from_str(&body_str) {
                Ok(result) => Some(result),
                Err(e) => {
                    error!("error formatting value: {}", e);
                    None
                }
            };
        }

        None
    }

    pub fn get_compare_body(
        &self,
        compare: &definition::CompareDescriptor,
        variables: &[Variable],
        iteration: u32,
    ) -> Option<serde_json::Value> {
        self.get_body(&compare.body, variables, iteration)
    }
}

//------------------TESTS---------------------------------

#[cfg(test)]
mod tests {

    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::*;
    use crate::test::definition::CleanupDescriptor;

    #[test]
    fn none_body_returns_none() {
        let vars: Vec<Variable> = vec![];
        let td = Definition {
            name: None,
            id: "id".to_string(),
            project: None,
            environment: None,
            requires: None,
            tags: vec![],
            iterate: 0,
            variables: vec![],
            global_variables: vec![],
            stages: vec![],
            setup: None,
            cleanup: CleanupDescriptor {
                onsuccess: None,
                onfailure: None,
                always: None,
            },
            disabled: false,
            filename: "/a/path.jkt".to_string(),
        };
        assert_eq!(None, td.get_body(&None, vars.as_slice(), 1))
    }

    #[test]
    fn body_novars_unchanged() {
        let vars: Vec<Variable> = vec![];
        let td = Definition {
            name: None,
            id: "id".to_string(),
            project: None,
            environment: None,
            requires: None,
            tags: vec![],
            iterate: 0,
            variables: vec![Variable {
                name: "my_var".to_string(),
                data_type: variable::Type::String,
                value: serde_yaml::to_value("my_val").unwrap(),
                modifier: None,
                format: None,
                file: None,
                source_path: "path".to_string(),
            }],
            global_variables: vec![],
            stages: vec![],
            setup: None,
            cleanup: CleanupDescriptor {
                onsuccess: None,
                onfailure: None,
                always: None,
            },
            disabled: false,
            filename: "/a/path.jkt".to_string(),
        };

        let body = RequestBody {
            data: serde_json::to_value("this_is_my_body").unwrap(),
            matches_variable: false.into(),
        };

        assert_eq!(
            serde_json::to_value("this_is_my_body").ok(),
            td.get_body(&Some(body), vars.as_slice(), 1)
        )
    }

    #[test]
    fn body_withvars_changed() {
        //Our expected sub is in this vector as opposed
        //to the vars in the TD.
        let vars: Vec<Variable> = vec![Variable {
            name: "my_var".to_string(),
            data_type: variable::Type::String,
            value: serde_yaml::to_value("my_val2").unwrap(),
            modifier: None,
            format: None,
            file: None,
            source_path: "path".to_string(),
        }];
        let td = Definition {
            name: None,
            id: "id".to_string(),
            project: None,
            environment: None,
            requires: None,
            tags: vec![],
            iterate: 0,
            variables: vec![Variable {
                name: "my_var".to_string(),
                data_type: variable::Type::String,
                value: serde_yaml::to_value("my_val").unwrap(),
                modifier: None,
                format: None,
                file: None,
                source_path: "path".to_string(),
            }],
            global_variables: vec![Variable {
                name: "my_var2".to_string(),
                data_type: variable::Type::String,
                value: serde_yaml::to_value("my_val3").unwrap(),
                modifier: None,
                format: None,
                file: None,
                source_path: "path".to_string(),
            }],
            stages: vec![],
            setup: None,
            cleanup: CleanupDescriptor {
                onsuccess: None,
                onfailure: None,
                always: None,
            },
            disabled: false,
            filename: "/a/path.jkt".to_string(),
        };

        let body = RequestBody {
            data: serde_json::to_value(format!("this_is_my_body_${{my_var}}_${{my_var2}}"))
                .unwrap(),
            matches_variable: true.into(),
        };

        assert_eq!(
            serde_json::to_value("this_is_my_body_my_val2_my_val3").ok(),
            td.get_body(&Some(body), vars.as_slice(), 1)
        )
    }
}
