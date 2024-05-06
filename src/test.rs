pub mod definition;
pub mod file;
pub mod http;
pub mod template;
pub mod validation;
pub mod variable;
use crate::test::file::BodyOrSchema;

use crate::test::definition::RequestBody;
use crate::test::file::DatumSchema;
use crate::test::file::StringOrDatumOrFile;
use log::{debug, error, trace};
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::path::Path;
use uuid::Uuid;

use self::file::{generate_value_from_schema, UnvalidatedRequest, UnvalidatedResponse};

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
pub struct File {
    pub name: Option<String>,
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub env: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub requires: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub iterate: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub setup: Option<file::UnvalidatedRequestResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request: Option<file::UnvalidatedRequest>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compare: Option<file::UnvalidatedCompareRequest>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response: Option<file::UnvalidatedResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stages: Option<Vec<file::UnvalidatedStage>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cleanup: Option<file::UnvalidatedCleanup>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variables: Option<Vec<file::UnvalidatedVariable>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disabled: Option<bool>,

    #[serde(skip_serializing, skip_deserializing)]
    pub filename: String,
}

impl Default for File {
    fn default() -> Self {
        Self {
            filename: "".to_string(),
            name: Some("".to_string()),
            id: Some(Uuid::new_v4().to_string()),
            project: None,
            env: None,
            tags: None,
            requires: None,
            iterate: None,
            setup: None,
            request: Some(UnvalidatedRequest::default()),
            compare: None,
            response: Some(UnvalidatedResponse::default()),
            stages: None,
            cleanup: None,
            variables: None,
            disabled: None,
            description: None,
        }
    }
}

impl File {
    pub fn generate_id(&self) -> String {
        let mut s = DefaultHasher::new();
        self.hash(&mut s);
        format!("{}", s.finish())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Variable {
    pub name: String,
    #[serde(flatten)]
    pub value: StringOrDatumOrFile,

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
            value: variable.value,
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
    pub fn generate_value(
        &self,
        definition: &Definition,
        iteration: u32,
        global_variables: &[Variable],
    ) -> String {
        match &self.value {
            StringOrDatumOrFile::File { file } => {
                let file_path = if Path::new(file).exists() {
                    file.clone()
                } else {
                    format!(
                        "{}{}{}",
                        self.source_path,
                        std::path::MAIN_SEPARATOR_STR,
                        file
                    )
                };

                return match std::fs::read_to_string(&file_path) {
                    Ok(file_data) => file_data.trim().to_string(),
                    Err(e) => {
                        error!("error loading file ({}) content: {}", file_path, e);

                        "".to_string()
                    }
                };
            }
            StringOrDatumOrFile::Value { value: str_val } => {
                return definition.resolve_variables(
                    str_val,
                    &HashMap::new(),
                    &global_variables,
                    iteration,
                );
            }
            StringOrDatumOrFile::Schema(d) => {
                return serde_json::to_string(d)
                    .map(|jv| {
                        definition.resolve_variables(
                            jv.as_str(),
                            &HashMap::new(),
                            &global_variables,
                            iteration,
                        )
                    })
                    .and_then(|rs| serde_json::from_str::<DatumSchema>(rs.as_str()))
                    .ok()
                    .and_then(|ds| generate_value_from_schema(&ds, 10))
                    .and_then(|v| serde_json::to_string(&v).ok())
                    .unwrap_or_default()
                    .trim_matches('"')
                    .to_string();
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Definition {
    pub name: Option<String>,
    pub description: Option<String>,
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
        state_variables: &HashMap<String, String>,
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

            let state_vars: Vec<(String, &String)> = state_variables
                .iter()
                .map(|(k, v)| (format!("${{{}}}", k), v))
                .collect();

            for (var_pattern, value) in &state_vars {
                if !replaced_url.contains(var_pattern) {
                    continue;
                }

                debug!("state variable match: {}", var_pattern);
                replaced_url
                    .clone_from(&replaced_url.replace(var_pattern.as_str(), value.as_str()));
            }

            for variable in variables.iter().chain(self.global_variables.iter()) {
                let var_pattern = format!("${{{}}}", variable.name);

                if !replaced_url.contains(var_pattern.as_str()) {
                    continue;
                }

                let replacement = variable.generate_value(self, iteration, &self.global_variables);
                replaced_url
                    .clone_from(&replaced_url.replace(var_pattern.as_str(), replacement.as_str()))
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

                let replacement = variable.generate_value(self, iteration, &self.global_variables);
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

            let replacement = variable.generate_value(self, iteration, &self.global_variables);
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

    pub fn resolve_body_variables(
        &self,
        body: &BodyOrSchema,
        state_variables: &HashMap<String, String>,
        variables: &[Variable],
        iteration: u32,
    ) -> Option<BodyOrSchema> {
        return match body {
            BodyOrSchema::Schema(s) => {
                self.resolve_schema_variables(s, state_variables, variables, iteration)
            }
            BodyOrSchema::Body(v) => {
                self.resolve_body_value_variables(v, state_variables, variables, iteration)
            }
        };
    }

    fn resolve_body_value_variables(
        &self,
        json_val: &serde_json::Value,
        state_variables: &HashMap<String, String>,
        variables: &[Variable],
        iteration: u32,
    ) -> Option<BodyOrSchema> {
        serde_json::to_string(&json_val)
            .map(|jv| self.resolve_variables(jv.as_str(), state_variables, variables, iteration))
            .and_then(|rs| {
                let rsv = rs.replace("\n", "").replace("\r", "");
                let ret = serde_json::from_str::<serde_json::Value>(rsv.as_str());
                if ret.is_err() {
                    return serde_json::from_str::<serde_json::Value>(rsv.trim_matches('\"'));
                } else {
                    return ret;
                }
            })
            .map(|val| BodyOrSchema::Body(val))
            .ok()
    }

    fn resolve_schema_variables(
        &self,
        schema: &DatumSchema,
        state_variables: &HashMap<String, String>,
        variables: &[Variable],
        iteration: u32,
    ) -> Option<BodyOrSchema> {
        serde_json::to_string(&schema)
            .map(|jv| self.resolve_variables(jv.as_str(), state_variables, variables, iteration))
            .and_then(|rs| serde_json::from_str::<DatumSchema>(rs.as_str()))
            .map(|schema| BodyOrSchema::Schema(schema))
            .ok()
    }

    fn resolve_variables(
        &self,
        json_val: &str,
        state_variables: &HashMap<String, String>,
        variables: &[Variable],
        iteration: u32,
    ) -> String {
        let mut mut_string = json_val.to_string();

        let state_vars: Vec<(String, &String)> = state_variables
            .iter()
            .map(|(k, v)| (format!("${{{}}}", k), v))
            .collect();

        for (var_pattern, value) in &state_vars {
            if !mut_string.contains(var_pattern) {
                continue;
            }

            debug!("state variable match: {}", var_pattern);
            mut_string = mut_string
                .replace(var_pattern.as_str(), value.as_str())
                .trim()
                .to_string();
        }

        for variable in variables.iter().chain(self.global_variables.iter()) {
            let var_pattern = format!("${{{}}}", variable.name);
            if !mut_string.contains(var_pattern.as_str()) {
                continue;
            }

            debug!("variable match: {}", var_pattern);

            let replacement = variable.generate_value(self, iteration, &self.global_variables);

            //Do extra for non string stuff
            let do_extra = match &variable.value {
                StringOrDatumOrFile::Schema(ds) => match ds {
                    DatumSchema::String { .. } => false,
                    _ => true,
                },
                _ => false,
            };

            if do_extra {
                mut_string = mut_string.trim_matches('"').to_string();
                let expected_lead_pattern = "\"";
                let expected_trail_pattern = "\"";

                mut_string = mut_string
                    .replace(
                        format!(
                            "{}{}{}",
                            expected_lead_pattern, var_pattern, expected_trail_pattern
                        )
                        .as_str(),
                        replacement.as_str(),
                    )
                    .trim()
                    .to_string();
            } else {
                mut_string = mut_string
                    .replace(var_pattern.as_str(), replacement.as_str())
                    .trim()
                    .to_string();
            }
        }
        mut_string
    }

    //Make a body for a request you will issue
    //May need a separate one for compare since it may never
    //make sense to specify a schema for comparisons?
    pub fn get_request_body(
        &self,
        body: &Option<RequestBody>,
        state_variables: &HashMap<String, String>,
        variables: &[Variable],
        iteration: u32,
    ) -> Option<serde_json::Value> {
        trace!("get_request_body({:?})", body);
        if let Some(body) = body {
            return self
                .resolve_body_variables(&body.data, state_variables, variables, iteration)
                .and_then(|b| match b {
                    BodyOrSchema::Schema(s) => generate_value_from_schema(&s, 10),
                    BodyOrSchema::Body(v) => Some(v),
                });
        }

        None
    }

    //Make a body for response validation!
    pub fn get_expected_request_body(
        &self,
        body: &Option<RequestBody>,
        state_variables: &HashMap<String, String>,
        variables: &[Variable],
        iteration: u32,
    ) -> Option<BodyOrSchema> {
        if let Some(body) = body {
            if !body.matches_variable.get() && state_variables.is_empty() {
                return Some(body.data.clone());
            }

            return self.resolve_body_variables(&body.data, state_variables, variables, iteration);
        }

        None
    }

    pub fn get_compare_body(
        &self,
        compare: &definition::CompareDescriptor,
        state_variables: &HashMap<String, String>,
        variables: &[Variable],
        iteration: u32,
    ) -> Option<serde_json::Value> {
        self.get_request_body(&compare.body, state_variables, variables, iteration)
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
            description: None,
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
        assert_eq!(
            None,
            td.get_request_body(&None, &HashMap::new(), vars.as_slice(), 1)
        )
    }

    #[test]
    fn body_novars_unchanged() {
        let vars: Vec<Variable> = vec![];
        let td = Definition {
            name: None,
            description: None,
            id: "id".to_string(),
            project: None,
            environment: None,
            requires: None,
            tags: vec![],
            iterate: 0,
            variables: vec![Variable {
                name: "my_var".to_string(),
                value: StringOrDatumOrFile::Value {
                    value: "my_val".to_string(),
                },
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
            data: BodyOrSchema::Body(serde_json::to_value("this_is_my_body").unwrap()),
            matches_variable: false.into(),
        };

        assert_eq!(
            serde_json::to_value("this_is_my_body").ok(),
            td.get_request_body(&Some(body), &HashMap::new(), vars.as_slice(), 1)
        )
    }

    #[test]
    fn body_withvars_changed() {
        //Our expected sub is in this vector as opposed
        //to the vars in the TD.
        let vars: Vec<Variable> = vec![Variable {
            name: "my_var".to_string(),
            value: StringOrDatumOrFile::Value {
                value: "my_val2".to_string(),
            },
            //data_type: variable::Type::String,
            //value: serde_yaml::to_value("my_val2").unwrap(),
            //modifier: None,
            //format: None,
            //file: None,
            source_path: "path".to_string(),
        }];
        let td = Definition {
            name: None,
            description: None,
            id: "id".to_string(),
            project: None,
            environment: None,
            requires: None,
            tags: vec![],
            iterate: 0,
            variables: vec![Variable {
                name: "my_var".to_string(),
                value: StringOrDatumOrFile::Value {
                    value: "my_val".to_string(),
                },
                source_path: "path".to_string(),
            }],
            global_variables: vec![Variable {
                name: "my_var2".to_string(),
                value: StringOrDatumOrFile::Value {
                    value: "my_val3".to_string(),
                },
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
            data: BodyOrSchema::Body(
                serde_json::to_value(format!("this_is_my_body_${{my_var}}_${{my_var2}}")).unwrap(),
            ),
            matches_variable: true.into(),
        };

        assert_eq!(
            serde_json::to_value("this_is_my_body_my_val2_my_val3").ok(),
            td.get_request_body(&Some(body), &HashMap::new(), vars.as_slice(), 1)
        )
    }
}
