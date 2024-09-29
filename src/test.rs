pub mod definition;
pub mod file;
pub mod http;
pub mod template;
pub mod validation;
pub mod variable;
use crate::test::file::BodyOrSchema;

use self::file::{generate_value_from_schema, UnvalidatedRequest, UnvalidatedResponse};
use crate::test::definition::RequestBody;
use crate::test::file::DatumSchema;
use crate::test::file::FloatSpecification;
use crate::test::file::IntegerSpecification;
use crate::test::file::NameSpecification;
use crate::test::file::UnvalidatedVariable3;
use crate::test::file::ValueOrDatumOrFile;
use file::DateSpecification;
use file::DateTimeSpecification;
use file::EmailSpecification;
use file::SequenceSpecification;
use file::StringSpecification;
use file::UnvalidatedDatumSchemaVariable2;
use log::{debug, error, trace};
use regex::Regex;
use serde::Serializer;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fmt::Debug;
use std::fmt::{self};
use std::hash::{Hash, Hasher};
use std::path::Path;
use ulid::Ulid;

#[derive(Deserialize, PartialEq)]
pub struct SecretValue(String);

impl SecretValue {
    const REDACTED_VALUE: &'static str = "******";
    fn redacted_value(&self) -> String {
        let len = self.0.len();
        match len {
            0..=20 => Self::REDACTED_VALUE.to_string(),
            _ => format!(
                "{}{}{}",
                &self.0[0..4],
                Self::REDACTED_VALUE,
                &self.0[len - 4..len]
            ),
        }
    }

    pub fn new(v: &str) -> Self {
        Self(v.to_string())
    }

    fn redact(&self, s: &str) -> String {
        s.replace(&self.0, &self.redacted_value())
    }
}

impl Serialize for SecretValue {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.redacted_value())
    }
}

impl fmt::Display for SecretValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", &self.redacted_value())
    }
}

impl fmt::Debug for SecretValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", &self.redacted_value())
    }
}

impl Hash for SecretValue {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.hash(state)
    }
}

impl Clone for SecretValue {
    fn clone(&self) -> Self {
        SecretValue(self.0.clone())
    }
}

#[derive(Debug, Serialize, Clone, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum ValueOrDatumOrFileOrSecret {
    File {
        value: String,
    },
    Secret {
        value: SecretValue,
    },
    Schema {
        value: DatumSchema,
    },
    Value {
        value: serde_json::Value,
    },
    #[serde(rename_all = "camelCase")]
    ValueSet {
        value_set: serde_json::Value,
    },
}

impl TryFrom<UnvalidatedVariable3> for ValueOrDatumOrFileOrSecret {
    type Error = String;

    fn try_from(value: UnvalidatedVariable3) -> Result<Self, Self::Error> {
        match value {
            // \todo : check if file is valid?
            //         we could, but will we ever store responses to file?
            //         basically a TOCTOU question
            UnvalidatedVariable3::File(f) => Ok(ValueOrDatumOrFileOrSecret::File { value: f.file }),
            UnvalidatedVariable3::Simple(s) => {
                Ok(ValueOrDatumOrFileOrSecret::Value { value: s.value })
            }
            UnvalidatedVariable3::Datum(ds) => TryInto::<DatumSchema>::try_into(ds)
                .map(|a| ValueOrDatumOrFileOrSecret::Schema { value: a }),
            UnvalidatedVariable3::ValueSet(vs) => Ok(ValueOrDatumOrFileOrSecret::ValueSet {
                value_set: serde_json::Value::Array(vs.value_set),
            }),
        }
    }
}

/*
    \todo : Address min/max specified AND value
*/
impl TryFrom<ValueOrDatumOrFile> for ValueOrDatumOrFileOrSecret {
    type Error = String;

    fn try_from(value: ValueOrDatumOrFile) -> Result<Self, Self::Error> {
        trace!("try_from({:?})", value);
        match value {
            ValueOrDatumOrFile::Value { value } => Ok(ValueOrDatumOrFileOrSecret::Value { value }),
            // \todo : check if file is valid?
            //         we could, but will we ever store responses to file?
            //         basically a TOCTOU question
            ValueOrDatumOrFile::ValueSet { value_set } => {
                Ok(ValueOrDatumOrFileOrSecret::ValueSet {
                    value_set: serde_json::Value::Array(value_set),
                })
            }
            ValueOrDatumOrFile::File { file } => {
                Ok(ValueOrDatumOrFileOrSecret::File { value: file })
            }
            ValueOrDatumOrFile::Schema(schema) => {
                match schema {
                    DatumSchema::Name { specification } => specification
                        .map(|s| {
                            NameSpecification::new(s.specification).map(|s| {
                                ValueOrDatumOrFileOrSecret::Schema {
                                    value: DatumSchema::Name {
                                        specification: Some(s),
                                    },
                                }
                            })
                        })
                        .unwrap_or(Ok(ValueOrDatumOrFileOrSecret::Schema {
                            value: DatumSchema::Email {
                                specification: None,
                            },
                        })),
                    // \todo: Should recursively validate
                    DatumSchema::Object { schema } => Ok(ValueOrDatumOrFileOrSecret::Schema {
                        value: DatumSchema::Object { schema },
                    }),
                    DatumSchema::List { specification } => specification
                        .map(|s| {
                            SequenceSpecification::new(
                                s.schema,
                                s.length,
                                s.min_length,
                                s.max_length,
                            )
                            .map(|s| {
                                ValueOrDatumOrFileOrSecret::Schema {
                                    value: DatumSchema::List {
                                        specification: Some(s),
                                    },
                                }
                            })
                        })
                        .unwrap_or(Ok(ValueOrDatumOrFileOrSecret::Schema {
                            value: DatumSchema::List {
                                specification: None,
                            },
                        })),
                    DatumSchema::Email { specification } => specification
                        .map(|s| {
                            EmailSpecification::new(s.specification).map(|s| {
                                ValueOrDatumOrFileOrSecret::Schema {
                                    value: DatumSchema::Email {
                                        specification: Some(s),
                                    },
                                }
                            })
                        })
                        .unwrap_or(Ok(ValueOrDatumOrFileOrSecret::Schema {
                            value: DatumSchema::Email {
                                specification: None,
                            },
                        })),
                    DatumSchema::Boolean { specification } => {
                        Ok(ValueOrDatumOrFileOrSecret::Schema {
                            value: DatumSchema::Boolean { specification },
                        })
                    }
                    DatumSchema::Float { specification } => specification
                        .map(|s| {
                            FloatSpecification::new(s.specification, s.min, s.max).map(|s| {
                                ValueOrDatumOrFileOrSecret::Schema {
                                    value: DatumSchema::Float {
                                        specification: Some(s),
                                    },
                                }
                            })
                        })
                        .unwrap_or(Ok(ValueOrDatumOrFileOrSecret::Schema {
                            value: DatumSchema::Float {
                                specification: None,
                            },
                        })),
                    DatumSchema::Integer { specification } => specification
                        .map(|s| {
                            IntegerSpecification::new(s.specification, s.min, s.max).map(|s| {
                                ValueOrDatumOrFileOrSecret::Schema {
                                    value: DatumSchema::Integer {
                                        specification: Some(s),
                                    },
                                }
                            })
                        })
                        .unwrap_or(Ok(ValueOrDatumOrFileOrSecret::Schema {
                            value: DatumSchema::Integer {
                                specification: None,
                            },
                        })),
                    DatumSchema::String { specification } => specification
                        .map(|s| {
                            StringSpecification::new(
                                s.specification,
                                s.length,
                                s.min_length,
                                s.max_length,
                                s.pattern,
                            )
                            .map(|s| {
                                ValueOrDatumOrFileOrSecret::Schema {
                                    value: DatumSchema::String {
                                        specification: Some(s),
                                    },
                                }
                            })
                        })
                        .unwrap_or(Ok(ValueOrDatumOrFileOrSecret::Schema {
                            value: DatumSchema::String {
                                specification: None,
                            },
                        })),
                    DatumSchema::Date { specification } => specification
                        .map(|ds| {
                            DateSpecification::new(
                                ds.specification,
                                ds.min,
                                ds.max,
                                ds.format,
                                ds.modifier,
                            )
                            .map(|s| {
                                ValueOrDatumOrFileOrSecret::Schema {
                                    value: DatumSchema::Date {
                                        specification: Some(s),
                                    },
                                }
                            })
                        })
                        .unwrap_or(Ok(ValueOrDatumOrFileOrSecret::Schema {
                            value: DatumSchema::Date {
                                specification: None,
                            },
                        })),
                    DatumSchema::DateTime { specification } => specification
                        .map(|ds| {
                            DateTimeSpecification::new(
                                ds.specification,
                                ds.min,
                                ds.max,
                                ds.format,
                                ds.modifier,
                            )
                            .map(|s| {
                                ValueOrDatumOrFileOrSecret::Schema {
                                    value: DatumSchema::DateTime {
                                        specification: Some(s),
                                    },
                                }
                            })
                        })
                        .unwrap_or(Ok(ValueOrDatumOrFileOrSecret::Schema {
                            value: DatumSchema::Integer {
                                specification: None,
                            },
                        })),
                }
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct File {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(rename = "platformId")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub platform_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disabled: Option<bool>,
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
    pub variables2: Option<Vec<file::UnvalidatedVariable3>>,
    #[serde(skip_serializing, skip_deserializing)]
    pub filename: String,
}

impl Default for File {
    fn default() -> Self {
        Self {
            filename: "".to_string(),
            name: Some("".to_string()),
            id: None,
            platform_id: Some(Ulid::new().to_string()),
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
            variables2: None,
            disabled: None,
            description: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Variable {
    pub name: String,
    #[serde(flatten)]
    pub value: ValueOrDatumOrFileOrSecret,

    #[serde(skip_serializing)]
    pub source_path: String,
}

impl Variable {
    pub fn new(
        variable: file::UnvalidatedVariable,
        source_path: &str,
    ) -> Result<Variable, validation::Error> {
        let regex = Regex::new(r"(?i)^[a-z0-9-_]+$").unwrap();
        if !regex.is_match(variable.name.as_str()) {
            debug!("variable name '{}' is invalid", variable.name);
            return Err(validation::Error{reason: "name is invalid - may only contain alphanumeric characters, hyphens, and underscores".to_string()});
        }

        variable
            .value
            .try_into()
            .map(|v| Variable {
                name: variable.name,
                value: v,
                source_path: source_path.to_string(),
            })
            .map_err(|e| validation::Error { reason: e })
    }

    pub fn new2(
        variable: file::UnvalidatedVariable3,
        source_path: &str,
    ) -> Result<Variable, validation::Error> {
        let name = match &variable {
            UnvalidatedVariable3::File(f) => Some(f.name.clone()),
            UnvalidatedVariable3::Simple(s) => Some(s.name.clone()),
            UnvalidatedVariable3::Datum(d) => match d {
                UnvalidatedDatumSchemaVariable2::Boolean(b) => b.name.clone(),
                UnvalidatedDatumSchemaVariable2::Date(d) => d.name.clone(),
                UnvalidatedDatumSchemaVariable2::DateTime(d) => d.name.clone(),
                UnvalidatedDatumSchemaVariable2::Email(e) => e.name.clone(),
                UnvalidatedDatumSchemaVariable2::Float(f) => f.name.clone(),
                UnvalidatedDatumSchemaVariable2::Integer(i) => i.name.clone(),
                UnvalidatedDatumSchemaVariable2::Name(n) => n.name.clone(),
                UnvalidatedDatumSchemaVariable2::String(s) => s.name.clone(),
                UnvalidatedDatumSchemaVariable2::List(l) => l.name.clone(),
                UnvalidatedDatumSchemaVariable2::Object { name, schema: _ } => name.clone(),
            },
            UnvalidatedVariable3::ValueSet(vs) => Some(vs.name.clone()),
        };

        return match name {
            None => Err(validation::Error {
                reason: "Name must be provided for variables".to_string(),
            }),
            Some(n) => {
                let regex = Regex::new(r"(?i)^[a-z0-9-_]+$").unwrap();
                if !regex.is_match(n.as_str()) {
                    debug!("variable name '{}' is invalid", n);
                    return Err(validation::Error{reason: "name is invalid - may only contain alphanumeric characters, hyphens, and underscores".to_string()});
                }
                TryInto::<ValueOrDatumOrFileOrSecret>::try_into(variable)
                    .map(|vdfs| Variable {
                        name: n,
                        source_path: source_path.to_string(),
                        value: vdfs,
                    })
                    .map_err(|e| validation::Error { reason: e })
            }
        };
    }

    pub fn validate_variables_opt(
        variables: Option<Vec<file::UnvalidatedVariable>>,
        source_path: &str,
    ) -> Result<Vec<Variable>, validation::Error> {
        let mut errors: Vec<String> = vec![];

        let ret = variables
            .map(|vars| {
                vars.into_iter()
                    .map(|f| (f.name.clone(), Variable::new(f, source_path)))
                    .filter_map(|(name, v)| match v {
                        Ok(x) => Some(x),
                        Err(e) => {
                            errors.push(format!("variable \"{}\" {}", name, e));
                            None
                        }
                    })
                    .collect::<Vec<Variable>>()
            })
            .unwrap_or_default();

        if !errors.is_empty() {
            return Err(validation::Error {
                reason: errors.join(","),
            });
        }

        Ok(ret)
    }

    pub fn validate_variables_opt2(
        variables: Option<Vec<file::UnvalidatedVariable3>>,
        source_path: &str,
    ) -> Result<Vec<Variable>, validation::Error> {
        let mut errors: Vec<String> = vec![];

        let ret = variables
            .map(|vars| {
                vars.into_iter()
                    .map(|f| Variable::new2(f, source_path))
                    .filter_map(|v| match v {
                        Ok(x) => Some(x),
                        Err(e) => {
                            errors.push(format!("variable error: {}", e));
                            None
                        }
                    })
                    .collect::<Vec<Variable>>()
            })
            .unwrap_or_default();

        if !errors.is_empty() {
            return Err(validation::Error {
                reason: errors.join(","),
            });
        }

        Ok(ret)
    }

    pub fn generate_value(
        &self,
        definition: &Definition,
        iteration: u32,
        global_variables: &[Variable],
    ) -> String {
        match &self.value {
            ValueOrDatumOrFileOrSecret::File { value: file } => {
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

                match std::fs::read_to_string(&file_path) {
                    Ok(file_data) => file_data.trim().to_string(),
                    Err(e) => {
                        error!("error loading file ({}) content: {}", file_path, e);

                        "".to_string()
                    }
                }
            }
            ValueOrDatumOrFileOrSecret::Secret { value: secret } => definition.resolve_variables(
                secret.0.as_str(),
                &HashMap::new(),
                global_variables,
                iteration,
            ),
            ValueOrDatumOrFileOrSecret::Value { value: v } => serde_json::to_string(v)
                .map(|jv| {
                    let ret = definition.resolve_variables(
                        jv.as_str(),
                        &HashMap::new(),
                        global_variables,
                        iteration,
                    );
                    ret
                })
                .unwrap_or_default()
                .trim_matches('"')
                .to_string(),
            ValueOrDatumOrFileOrSecret::ValueSet { value_set: v } => {
                let length = v.as_array().unwrap_or(&Vec::new()).len();
                if length == 0 {
                    // divide by zero
                    return "".to_string();
                }
                let index = iteration % length as u32;
                serde_json::to_string(
                    v.get(index as usize)
                        .unwrap_or(&serde_json::Value::from("")),
                )
                .map(|jv| {
                    let ret = definition.resolve_variables(
                        jv.as_str(),
                        &HashMap::new(),
                        global_variables,
                        iteration,
                    );
                    ret
                })
                .unwrap_or_default()
                .trim_matches('"')
                .to_string()
            }
            ValueOrDatumOrFileOrSecret::Schema { value: d } => serde_json::to_string(d)
                .map(|jv| {
                    definition.resolve_variables(
                        jv.as_str(),
                        &HashMap::new(),
                        global_variables,
                        iteration,
                    )
                })
                .and_then(|rs| serde_json::from_str::<DatumSchema>(rs.as_str()))
                .ok()
                .and_then(|ds| generate_value_from_schema(&ds, 10))
                .and_then(|v| serde_json::to_string(&v).ok())
                .unwrap_or_default()
                .trim_matches('"')
                .to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Definition {
    pub name: Option<String>,
    pub description: Option<String>,
    pub id: Option<String>,
    pub platform_id: Option<String>,
    pub project: Option<String>,
    pub environment: Option<String>,
    pub requires: Option<String>,
    pub tags: Vec<String>,
    pub iterate: u32,
    pub variables: Vec<Variable>,
    pub variables2: Vec<Variable>,
    pub global_variables: Vec<Variable>,
    pub stages: Vec<definition::StageDescriptor>,
    pub setup: Option<definition::RequestResponseDescriptor>,
    pub cleanup: definition::CleanupDescriptor,
    pub disabled: bool,

    #[serde(skip_serializing, skip_deserializing)]
    pub file_data: File,

    #[serde(skip_serializing, skip_deserializing)]
    pub index: usize,
}

// TODO: add validation logic to verify the descriptor is valid
// TODO: Validation should be type driven for compile time correctness
impl Definition {
    //Instead of iterating could we create a regex from all secret values
    //and do it in 1 call. Introduce encapsulation to easily do that in future
    pub fn redact_secrets(&self, s: &str) -> String {
        self.global_variables
            .iter()
            .filter_map(|v| match &v.value {
                ValueOrDatumOrFileOrSecret::Secret { value: secret } => Some(secret),
                _ => None,
            })
            .fold(s.to_string(), |acc, secret| secret.redact(acc.as_str()))
    }

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
        trace!("resolve_body_variables({:?})", body);
        match body {
            BodyOrSchema::Schema(s) => {
                self.resolve_schema_variables(s, state_variables, variables, iteration)
            }
            BodyOrSchema::Body(v) => {
                self.resolve_body_value_variables(v, state_variables, variables, iteration)
            }
        }
    }

    fn resolve_body_value_variables(
        &self,
        json_val: &serde_json::Value,
        state_variables: &HashMap<String, String>,
        variables: &[Variable],
        iteration: u32,
    ) -> Option<BodyOrSchema> {
        trace!("resolve_body_value_variables()");
        serde_json::to_string(&json_val)
            .map(|jv| self.resolve_variables(jv.as_str(), state_variables, variables, iteration))
            .and_then(|rs| {
                trace!("rs is {}", rs);
                let rsv = rs.replace(['\n', '\r'], "");
                trace!("rsv is {}", rsv);
                let mut ret = serde_json::from_str::<serde_json::Value>(rsv.as_str());
                if ret.is_err() {
                    trace!("Error resolving. Attempting to trim quotes {:?}", ret);
                    ret = serde_json::from_str::<serde_json::Value>(rsv.trim_matches('\"'));
                    if ret.is_err() {
                        error!("Error producing json body! {:?} : {:?}", ret, rsv);
                    }
                    ret
                } else {
                    trace!("GOOD VAL {:?}", ret);
                    ret
                }
            })
            .map(BodyOrSchema::Body)
            .ok()
    }

    fn resolve_schema_variables(
        &self,
        schema: &DatumSchema,
        state_variables: &HashMap<String, String>,
        variables: &[Variable],
        iteration: u32,
    ) -> Option<BodyOrSchema> {
        trace!("resolve_schema_variables()");
        let ret = serde_json::to_string(&schema)
            .map(|jv| self.resolve_variables(jv.as_str(), state_variables, variables, iteration))
            .and_then(|rs| serde_json::from_str::<DatumSchema>(rs.as_str()))
            .map_err(|e| {
                trace!("resolve_schema_variables(): Error is {e}");
                error!("Error producing json body from schema! {e}");
                e
            })
            .map(BodyOrSchema::Schema)
            .map_err(|e| {
                trace!("resolve_schema_variables(): Error2 is {e}");
                e
            })
            .ok();
        ret
    }

    fn resolve_variables(
        &self,
        json_val: &str,
        state_variables: &HashMap<String, String>,
        variables: &[Variable],
        iteration: u32,
    ) -> String {
        debug!("resolve_variables({})", json_val);
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
            //play with recursion here, these could be complex variables
            //ALSO you can generate values from complex variables prior to running
            //a stage? That way they're consistent and can just be replaced here without
            //issue?
        }

        for variable in variables.iter().chain(self.global_variables.iter()) {
            let var_pattern = format!("${{{}}}", variable.name);
            if !mut_string.contains(var_pattern.as_str()) {
                continue;
            }

            debug!("variable match: {} =====> {:?}", var_pattern, variable);

            let replacement = variable.generate_value(self, iteration, &self.global_variables);

            debug!("replacement is {}", replacement);

            debug!("replacement is {:?}", replacement);
            trace!(
                "Variable name=>value {:?}===>{:?}",
                var_pattern,
                &variable.value
            );

            //Do extra for non string stuff
            let do_extra = match &variable.value {
                ValueOrDatumOrFileOrSecret::Schema { value: ds } => !matches!(
                    ds,
                    DatumSchema::String { .. }
                        | DatumSchema::Name { .. }
                        | DatumSchema::Date { .. }
                        | DatumSchema::DateTime { .. }
                        | DatumSchema::Email { .. }
                ),
                ValueOrDatumOrFileOrSecret::Value { value: v } => {
                    !matches!(v, serde_json::Value::String(_))
                }
                _ => false,
            };

            //if not a string, we want to even replace the quotes
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
                    .replace(var_pattern.as_str(), replacement.as_str())
                    .to_string();
            } else {
                mut_string = mut_string
                    .replace(var_pattern.as_str(), replacement.as_str())
                    .trim()
                    .to_string();
            }
        }

        debug!("mut string is {}", mut_string);
        debug!("mut string is {:?}", mut_string);
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

    use file::UnvalidatedVariable;
    use serde_json::Value;

    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::*;
    use crate::test::definition::CleanupDescriptor;

    #[test]
    fn variable_new_with_valid_name() {
        let var = UnvalidatedVariable {
            name: "VALID-name_123".to_string(),
            value: ValueOrDatumOrFile::Value {
                value: Value::default(),
            },
        };

        let res = Variable::new(var, "");
        assert!(res.is_ok());
    }

    #[test]
    fn variable_new_with_invalid_name_dot() {
        let var = UnvalidatedVariable {
            name: "invalid.name".to_string(),
            value: ValueOrDatumOrFile::Value {
                value: Value::default(),
            },
        };

        let res = Variable::new(var, "");
        assert!(res.is_err());
    }

    #[test]
    fn variable_new_with_invalid_name_space() {
        let var = UnvalidatedVariable {
            name: "invalid name".to_string(),
            value: ValueOrDatumOrFile::Value {
                value: Value::default(),
            },
        };

        let res = Variable::new(var, "");
        assert!(res.is_err());
    }

    #[test]
    fn variable_new_with_invalid_name_empty() {
        let var = UnvalidatedVariable {
            name: "".to_string(),
            value: ValueOrDatumOrFile::Value {
                value: Value::default(),
            },
        };

        let res = Variable::new(var, "");
        assert!(res.is_err());
    }

    #[test]
    fn secret_value_debug_obsfucated() {
        assert_eq!(
            SecretValue::REDACTED_VALUE,
            format!("{:?}", SecretValue::new("hello"))
        );
    }

    #[test]
    fn secret_value_obsfucated() {
        assert_eq!(
            SecretValue::REDACTED_VALUE,
            format!("{}", SecretValue::new("hello"))
        );
    }

    #[test]
    fn secret_value_obsfucation_of_small_strings() {
        assert_eq!(
            SecretValue::REDACTED_VALUE,
            SecretValue::new("hello").redacted_value()
        );
    }

    #[test]
    fn secret_value_obsfucation_of_large_strings() {
        assert_eq!(
            format!("{}{}{}", "myna", SecretValue::REDACTED_VALUE, "hady"),
            SecretValue::new("mynameisslimshadyyesimtherealshady").redacted_value()
        );
    }

    #[test]
    fn string_datum_file_or_secret_from_impl() {
        assert_eq!(
            ValueOrDatumOrFileOrSecret::File {
                value: "file".to_string()
            },
            ValueOrDatumOrFile::File {
                file: "file".to_string()
            }
            .try_into()
            .unwrap()
        );

        assert_eq!(
            ValueOrDatumOrFileOrSecret::Value {
                value: serde_json::Value::from("val".to_string())
            },
            ValueOrDatumOrFile::Value {
                value: serde_json::Value::from("val".to_string())
            }
            .try_into()
            .unwrap()
        );

        assert_eq!(
            ValueOrDatumOrFileOrSecret::Schema {
                value: DatumSchema::Integer {
                    specification: None
                }
            },
            ValueOrDatumOrFile::Schema(DatumSchema::Integer {
                specification: None
            })
            .try_into()
            .unwrap()
        );
    }

    #[test]
    fn none_body_returns_none() {
        let vars: Vec<Variable> = vec![];
        let td = Definition {
            name: None,
            description: None,
            id: None,
            platform_id: None,
            project: None,
            environment: None,
            requires: None,
            tags: vec![],
            iterate: 0,
            variables: vec![],
            variables2: vec![],
            global_variables: vec![],
            stages: vec![],
            setup: None,
            cleanup: CleanupDescriptor {
                onsuccess: None,
                onfailure: None,
                always: None,
            },
            disabled: false,
            file_data: File::default(),
            index: 0,
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
            id: None,
            platform_id: None,
            project: None,
            environment: None,
            requires: None,
            tags: vec![],
            iterate: 0,
            variables: vec![Variable {
                name: "my_var".to_string(),
                value: ValueOrDatumOrFileOrSecret::Value {
                    value: serde_json::Value::from("my_val".to_string()),
                },
                source_path: "path".to_string(),
            }],
            variables2: vec![Variable {
                name: "my_var".to_string(),
                value: ValueOrDatumOrFileOrSecret::Value {
                    value: serde_json::Value::from("my_val".to_string()),
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
            file_data: File::default(),
            index: 0,
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
            value: ValueOrDatumOrFileOrSecret::Value {
                value: serde_json::Value::from("my_val2".to_string()),
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
            id: None,
            platform_id: None,
            project: None,
            environment: None,
            requires: None,
            tags: vec![],
            iterate: 0,
            variables: vec![Variable {
                name: "my_var".to_string(),
                value: ValueOrDatumOrFileOrSecret::Value {
                    value: serde_json::Value::from("my_val".to_string()),
                },
                source_path: "path".to_string(),
            }],
            variables2: vec![Variable {
                name: "my_var".to_string(),
                value: ValueOrDatumOrFileOrSecret::Value {
                    value: serde_json::Value::from("my_val".to_string()),
                },
                source_path: "path".to_string(),
            }],
            global_variables: vec![Variable {
                name: "my_var2".to_string(),
                value: ValueOrDatumOrFileOrSecret::Value {
                    value: serde_json::Value::from("my_val3".to_string()),
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
            file_data: File::default(),
            index: 0,
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
