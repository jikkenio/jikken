use crate::json::filter::filter_json;
use crate::test;
use crate::test::file::Validated::Good;
use crate::test::{definition, http, variable};
use log::error;
use log::trace;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::error::Error;
use std::fmt::Display;
use std::fmt::{self};
use std::fs;
use std::hash::{Hash, Hasher};
use validated::Validated;

//add pattern
#[derive(Serialize, Debug, Clone, Deserialize, PartialEq, PartialOrd, Hash)]
#[serde(rename_all = "camelCase")]
pub struct Specification<T> {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub val: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub one_of: Option<Vec<T>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub none_of: Option<Vec<T>>,
}

pub trait Checker {
    type Item;
    fn check(
        &self,
        val: &Self::Item,
        formatter: &impl Fn(&str, &str) -> String,
    ) -> Vec<Validated<(), String>>;
}

impl<T> Specification<T>
where
    T: PartialEq,
    T: Display,
    T: PartialOrd,
    T: fmt::Debug,
{
    fn check_val(
        &self,
        actual: &T,
        formatter: &impl Fn(&str, &str) -> String,
    ) -> Validated<(), String> {
        match &self.val {
            Some(t) => {
                if t == actual {
                    Good(())
                } else {
                    Validated::fail(formatter(
                        format!("{}", t).as_str(),
                        format!("{}", actual).as_str(),
                    ))
                }
            }
            None => Good(()),
        }
    }

    fn check_min(
        &self,
        actual: &T,
        formatter: &impl Fn(&str, &str) -> String,
    ) -> Validated<(), String> {
        match &self.min {
            Some(t) => {
                if t <= actual {
                    Good(())
                } else {
                    Validated::fail(formatter(
                        format!("minimum of {}", t).as_str(),
                        format!("{}", actual).as_str(),
                    ))
                }
            }
            None => Good(()),
        }
    }

    fn check_max(
        &self,
        actual: &T,
        formatter: &impl Fn(&str, &str) -> String,
    ) -> Validated<(), String> {
        match &self.max {
            Some(t) => {
                if t >= actual {
                    Good(())
                } else {
                    Validated::fail(formatter(
                        format!("maximum of {}", t).as_str(),
                        format!("{}", actual).as_str(),
                    ))
                }
            }
            None => Good(()),
        }
    }

    fn check_one_of(
        &self,
        actual: &T,
        formatter: &impl Fn(&str, &str) -> String,
    ) -> Validated<(), String> {
        match &self.one_of {
            Some(t) => {
                if t.contains(actual) {
                    Good(())
                } else {
                    Validated::fail(formatter(
                        format!("one of {:?}", t).as_str(),
                        format!("{}", actual).as_str(),
                    ))
                }
            }
            None => Good(()),
        }
    }

    fn check_none_of(
        &self,
        actual: &T,
        formatter: &impl Fn(&str, &str) -> String,
    ) -> Validated<(), String> {
        match &self.none_of {
            Some(t) => {
                if !t.contains(actual) {
                    Good(())
                } else {
                    Validated::fail(formatter(
                        format!("none of {:?}", t).as_str(),
                        format!("{}", actual).as_str(),
                    ))
                }
            }
            None => Good(()),
        }
    }
}

impl<T> Checker for Specification<T>
where
    T: PartialEq,
    T: Display,
    T: PartialOrd,
    T: fmt::Debug,
{
    type Item = T;
    fn check(
        &self,
        val: &T,
        formatter: &impl Fn(&str, &str) -> String,
    ) -> Vec<Validated<(), String>> {
        vec![
            self.check_val(&val, formatter),
            self.check_min(&val, formatter),
            self.check_max(&val, formatter),
            self.check_none_of(&val, formatter),
            self.check_one_of(&val, formatter),
        ]
    }
}

#[derive(Serialize, Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
#[serde(tag = "type")]
pub enum DatumSchema {
    Float {
        #[serde(flatten)]
        specification: Option<Specification<f64>>,
    },
    Int {
        #[serde(flatten)]
        specification: Option<Specification<i64>>,
    },
    String {
        #[serde(flatten)]
        specification: Option<Specification<String>>,
    },
    List {
        #[serde(skip_serializing_if = "Option::is_none")]
        schema: Option<Box<DatumSchema>>,
    },
    Object {
        #[serde(skip_serializing_if = "Option::is_none")]
        schema: Option<BTreeMap<String, DatumSchema>>,
    },
}

impl DatumSchema {
    fn check(
        &self,
        actual: &serde_json::Value,
        formatter: &impl Fn(&str, &str) -> String,
    ) -> Vec<Validated<(), String>> {
        let mut ret = self.check_float(actual, formatter);
        ret.append(self.check_int(actual, formatter).as_mut());
        ret.append(self.check_string(actual, formatter).as_mut());
        ret.append(self.check_list(actual, formatter).as_mut());
        ret.append(self.check_object(actual, formatter).as_mut());
        ret
    }

    fn check_float(
        &self,
        actual: &serde_json::Value,
        formatter: &impl Fn(&str, &str) -> String,
    ) -> Vec<Validated<(), String>> {
        match self {
            DatumSchema::Float { specification } => {
                if !actual.is_f64() {
                    return vec![Validated::fail(formatter("float type", "type"))];
                }

                specification
                    .as_ref()
                    .map(|s| s.check(&actual.as_f64().unwrap(), formatter))
                    .unwrap_or(vec![Good(())])
            }
            _ => vec![Good(())],
        }
    }

    fn check_int(
        &self,
        actual: &serde_json::Value,
        formatter: &impl Fn(&str, &str) -> String,
    ) -> Vec<Validated<(), String>> {
        match self {
            DatumSchema::Int { specification } => {
                if !actual.is_i64() {
                    return vec![Validated::fail(formatter("int type", "type"))];
                }

                specification
                    .as_ref()
                    .map(|s| s.check(&actual.as_i64().unwrap(), formatter))
                    .unwrap_or(vec![Good(())])
            }
            _ => vec![Good(())],
        }
    }

    fn check_string(
        &self,
        actual: &serde_json::Value,
        formatter: &impl Fn(&str, &str) -> String,
    ) -> Vec<Validated<(), String>> {
        match self {
            DatumSchema::String { specification } => {
                if !actual.is_string() {
                    return vec![Validated::fail(formatter("string type", "type"))];
                }

                specification
                    .as_ref()
                    .map(|s| s.check(&actual.as_str().unwrap().to_string(), formatter))
                    .unwrap_or(vec![Good(())])
            }
            _ => vec![Good(())],
        }
    }

    fn check_list(
        &self,
        actual: &serde_json::Value,
        formatter: &impl Fn(&str, &str) -> String,
    ) -> Vec<Validated<(), String>> {
        match self {
            DatumSchema::List { schema } => {
                if !actual.is_array() {
                    return vec![Validated::fail(formatter("array type", "different type"))];
                }

                schema
                    .as_ref()
                    .map(|s| {
                        actual
                            .as_array()
                            .unwrap()
                            .iter()
                            .map(|v| s.check(v, formatter))
                            .flatten()
                            .collect()
                    })
                    .unwrap_or(vec![Good(())])
            }
            _ => vec![Good(())],
        }
    }

    fn check_object(
        &self,
        actual: &serde_json::Value,
        formatter: &impl Fn(&str, &str) -> String,
    ) -> Vec<Validated<(), String>> {
        match self {
            DatumSchema::Object { schema } => {
                if !actual.is_object() {
                    return vec![Validated::fail(formatter("object type", "different type"))];
                }

                let vals = actual.as_object().unwrap();
                schema
                    .as_ref()
                    .map(|bt| {
                        bt.iter()
                            .map(|(k, datum)| {
                                vals.get(k)
                                    .map(|v| datum.check(v, formatter))
                                    .unwrap_or(vec![Validated::fail(formatter(
                                        format!(r#"member "{k}""#).as_str(),
                                        format!(r#"object with "{k}" missing"#).as_str(),
                                    ))])
                            })
                            .flatten()
                            .collect()
                    })
                    .unwrap_or(vec![Good(())])
            }
            _ => vec![Good(())],
        }
    }
}

impl Checker for DatumSchema {
    type Item = serde_json::Value;
    fn check(
        &self,
        val: &Self::Item,
        formatter: &impl Fn(&str, &str) -> String,
    ) -> Vec<Validated<(), String>> {
        self.check(val, formatter)
    }
}

#[derive(Serialize, Debug, Clone, Deserialize, PartialEq)]
pub struct DocumentSchema {
    #[serde(rename = "_jk_schema")]
    pub schema: DatumSchema,
}

impl Checker for DocumentSchema {
    type Item = serde_json::Value;
    fn check(
        &self,
        val: &serde_json::Value,
        formatter: &impl Fn(&str, &str) -> String,
    ) -> Vec<Validated<(), String>> {
        self.schema.check(val, formatter)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnvalidatedRequest {
    pub method: Option<http::Verb>,
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Vec<http::Parameter>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<Vec<http::Header>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<ValueOrSchema>,
}

impl Default for UnvalidatedRequest {
    fn default() -> Self {
        Self {
            method: None,
            url: "".to_string(),
            params: None,
            headers: None,
            body: None,
        }
    }
}

impl Hash for UnvalidatedRequest {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.method.hash(state);
        self.url.hash(state);
        self.params.hash(state);
        self.headers.hash(state);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnvalidatedCompareRequest {
    pub method: Option<http::Verb>,
    pub url: String,
    pub params: Option<Vec<http::Parameter>>,
    pub add_params: Option<Vec<http::Parameter>>,
    pub ignore_params: Option<Vec<String>>,
    pub headers: Option<Vec<http::Header>>,
    pub add_headers: Option<Vec<http::Header>>,
    pub ignore_headers: Option<Vec<String>>,
    pub body: Option<ValueOrSchema>, //Option<serde_json::Value>,
}

impl Hash for UnvalidatedCompareRequest {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.method.hash(state);
        self.url.hash(state);
        self.params.hash(state);
        self.add_params.hash(state);
        self.ignore_params.hash(state);
        self.headers.hash(state);
        self.add_headers.hash(state);
        self.ignore_headers.hash(state);
    }
}

#[derive(Debug, Serialize, Clone, PartialEq, Deserialize, Hash)]
#[serde(untagged)]
pub enum ValueOrSpecification<T> {
    Value(T),
    Schema(Specification<T>),
}

impl<T> Checker for ValueOrSpecification<T>
where
    T: PartialEq,
    T: Display,
    T: PartialOrd,
    T: fmt::Debug,
{
    type Item = T;
    fn check(
        &self,
        val: &Self::Item,
        formatter: &impl Fn(&str, &str) -> String,
    ) -> Vec<Validated<(), String>> {
        match &self {
            ValueOrSpecification::Value(t) => {
                if t == val {
                    vec![Good(())]
                } else {
                    vec![Validated::fail(formatter(
                        format!("{}", t).as_str(),
                        format!("{}", val).as_str(),
                    ))]
                }
            }
            ValueOrSpecification::Schema(s) => s.check(val, formatter),
        }
    }
}

#[derive(Debug, Serialize, Clone, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum ValueOrSchema {
    Schema(DocumentSchema),
    Value(serde_json::Value),
}

pub struct ValueOrSchemaChecker<'a> {
    pub value_or_schema: &'a ValueOrSchema,
    pub ignore_values: &'a [String],
}

impl<'a> ValueOrSchemaChecker<'a> {
    pub fn check_schema(
        actual: &serde_json::Value,
        schema: &DocumentSchema,
        ignore: &[String],
        formatter: &impl Fn(&str, &str) -> String,
    ) -> Result<Vec<Validated<(), String>>, Box<dyn Error + Send + Sync>> {
        trace!("validating response body using schema");
        //\todo How do I apply ignore in this case?
        //Or does it even make sense? If not, we would need to
        //factor it out of response and into the "Value" part of ValueOrSchema
        Ok(schema.check(actual, formatter))
    }

    pub fn check_expected_value(
        actual: &serde_json::Value,
        expected: &serde_json::Value,
        ignore: &[String],
        formatter: &impl Fn(&str, &str) -> String,
    ) -> Result<Vec<Validated<(), String>>, Box<dyn Error + Send + Sync>> {
        trace!("validating response body");
        let mut modified_actual = actual.clone();
        let mut modified_expected = expected.clone();

        // TODO: make this more efficient, with a single pass filter
        for path in ignore.iter() {
            trace!("stripping path({}) from response", path);
            modified_actual = filter_json(path, 0, modified_actual)?;
            modified_expected = filter_json(path, 0, modified_expected)?;
        }

        trace!("compare json");
        let r = modified_actual == modified_expected;

        if !r {
            let result = assert_json_diff::assert_json_matches_no_panic(
                &modified_actual,
                &modified_expected,
                assert_json_diff::Config::new(assert_json_diff::CompareMode::Strict),
            );
            return match result {
                Ok(_) => Ok(vec![Validated::fail(formatter(
                    format!("body {modified_expected}").as_str(),
                    format!("body {modified_actual}").as_str(),
                ))]),
                Err(msg) => Ok(vec![Validated::fail(formatter(
                    format!("body {modified_expected}").as_str(),
                    format!("body {modified_actual} ; {msg}").as_str(),
                ))]),
            };
        }

        Ok(vec![Good(())])
    }
}

impl<'a> Checker for ValueOrSchemaChecker<'a> {
    type Item = serde_json::Value;
    fn check(
        &self,
        val: &Self::Item,
        formatter: &impl Fn(&str, &str) -> String,
    ) -> Vec<Validated<(), String>> {
        let res = match self.value_or_schema {
            ValueOrSchema::Value(v) => {
                ValueOrSchemaChecker::check_expected_value(val, v, self.ignore_values, formatter)
            }
            ValueOrSchema::Schema(s) => {
                ValueOrSchemaChecker::check_schema(val, s, self.ignore_values, formatter)
            }
        };

        match res {
            Ok(v) => v,
            Err(e) => vec![Validated::fail(format!(
                "Error encountered when comparing : {}",
                e.to_string()
            ))],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnvalidatedResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<ValueOrSpecification<u16>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<Vec<http::Header>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<ValueOrSchema>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ignore: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extract: Option<Vec<definition::ResponseExtraction>>,
}

impl Default for UnvalidatedResponse {
    fn default() -> Self {
        Self {
            status: Some(ValueOrSpecification::Value(200)),
            headers: None,
            body: None,
            ignore: None,
            extract: None,
        }
    }
}

impl Hash for UnvalidatedResponse {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.status.hash(state);
        self.headers.hash(state);
        self.ignore.hash(state);
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Hash)]
#[serde(rename_all = "camelCase")]
pub struct UnvalidatedVariable {
    pub name: String,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub data_type: Option<variable::Type>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<serde_yaml::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modifier: Option<variable::Modifier>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
pub struct UnvalidatedStage {
    pub request: UnvalidatedRequest,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compare: Option<UnvalidatedCompareRequest>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response: Option<UnvalidatedResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variables: Option<Vec<UnvalidatedVariable>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delay: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
pub struct UnvalidatedRequestResponse {
    pub request: UnvalidatedRequest,
    pub response: Option<UnvalidatedResponse>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
pub struct UnvalidatedCleanup {
    pub onsuccess: Option<UnvalidatedRequest>,
    pub onfailure: Option<UnvalidatedRequest>,
    pub always: Option<UnvalidatedRequest>,
}

pub fn load(filename: &str) -> Result<test::File, Box<dyn Error + Send + Sync>> {
    let file_data = fs::read_to_string(filename)?;
    let result: Result<test::File, serde_yaml::Error> = serde_yaml::from_str(&file_data);
    match result {
        Ok(mut file) => {
            file.filename = String::from(filename);
            Ok(file)
        }
        Err(e) => {
            error!("unable to parse file ({}) data: {}", filename, e);
            Err(Box::from(e))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn specification_val_checker() {
        let spec = Specification::<u16> {
            val: Some(12),
            min: None,
            max: None,
            one_of: None,
            none_of: None,
        };

        let f = spec
            .check(&12, &|_e, _a| "".to_string())
            .into_iter()
            .collect::<Validated<Vec<()>, String>>();
        assert_eq!(true, f.is_good());

        assert_eq!(
            true,
            spec.check(&22, &|_e, _a| "".to_string())
                .into_iter()
                .collect::<Validated<Vec<()>, String>>()
                .is_fail()
        );
    }

    #[test]
    fn specification_min_checker() {
        let spec = Specification::<u16> {
            val: None,
            min: Some(50),
            max: None,
            one_of: None,
            none_of: None,
        };

        assert_eq!(
            true,
            spec.check(&22, &|_e, _a| "".to_string())
                .into_iter()
                .collect::<Validated<Vec<()>, String>>()
                .is_fail()
        );

        assert_eq!(
            true,
            spec.check(&50, &|_e, _a| "".to_string())
                .into_iter()
                .collect::<Validated<Vec<()>, String>>()
                .is_good()
        );

        assert_eq!(
            true,
            spec.check(&100, &|_e, _a| "".to_string())
                .into_iter()
                .collect::<Validated<Vec<()>, String>>()
                .is_good()
        );
    }

    #[test]
    fn specification_max_checker() {
        let spec = Specification::<u16> {
            val: None,
            min: None,
            max: Some(50),
            one_of: None,
            none_of: None,
        };

        assert_eq!(
            true,
            spec.check(&22, &|_e, _a| "".to_string())
                .into_iter()
                .collect::<Validated<Vec<()>, String>>()
                .is_good()
        );

        assert_eq!(
            true,
            spec.check(&50, &|_e, _a| "".to_string())
                .into_iter()
                .collect::<Validated<Vec<()>, String>>()
                .is_good()
        );

        assert_eq!(
            true,
            spec.check(&100, &|_e, _a| "".to_string())
                .into_iter()
                .collect::<Validated<Vec<()>, String>>()
                .is_fail()
        );
    }

    #[test]
    fn specification_one_of_checker() {
        let spec = Specification::<u16> {
            val: None,
            min: None,
            max: None,
            one_of: Some(vec![1, 2, 3, 4, 5]),
            none_of: None,
        };

        assert_eq!(
            true,
            spec.check(&22, &|_e, _a| "".to_string())
                .into_iter()
                .collect::<Validated<Vec<()>, String>>()
                .is_fail()
        );

        for i in 1..5 {
            assert_eq!(
                true,
                spec.check(&i, &|_e, _a| "".to_string())
                    .into_iter()
                    .collect::<Validated<Vec<()>, String>>()
                    .is_good()
            );
        }
    }

    #[test]
    fn specification_none_of_checker() {
        let spec = Specification::<u16> {
            val: None,
            min: None,
            max: None,
            one_of: None,
            none_of: Some(vec![1, 2, 3, 4, 5]),
        };

        assert_eq!(
            true,
            spec.check(&22, &|_e, _a| "".to_string())
                .into_iter()
                .collect::<Validated<Vec<()>, String>>()
                .is_good()
        );

        for i in 1..5 {
            assert_eq!(
                true,
                spec.check(&i, &|_e, _a| "".to_string())
                    .into_iter()
                    .collect::<Validated<Vec<()>, String>>()
                    .is_fail()
            );
        }
    }

    #[test]
    fn specification_errors_accumulate() {
        let spec = Specification::<u16> {
            val: Some(1),
            min: Some(200),
            max: Some(100),
            one_of: Some(vec![1, 2, 4]),
            none_of: Some(vec![101]),
        };

        assert_eq!(
            5,
            spec.check(&101, &|_e, _a| "".to_string())
                .into_iter()
                .collect::<Validated<Vec<()>, String>>()
                .ok()
                .err()
                .unwrap()
                .len()
        );
    }

    #[test]
    fn datum_float_type_validation() {
        assert_eq!(
            true,
            DatumSchema::Float {
                specification: None,
            }
            .check(&serde_json::json!({}), &|_e, _a| "".to_string())
            .into_iter()
            .collect::<Validated<Vec<()>, String>>()
            .is_fail(),
        );

        assert_eq!(
            false,
            DatumSchema::Float {
                specification: None,
            }
            .check(&serde_json::json!(4.53), &|_e, _a| "".to_string())
            .into_iter()
            .collect::<Validated<Vec<()>, String>>()
            .is_fail(),
        );
    }

    #[test]
    fn datum_int_type_validation() {
        assert_eq!(
            true,
            DatumSchema::Int {
                specification: None,
            }
            .check(&serde_json::json!({}), &|_e, _a| "".to_string())
            .into_iter()
            .collect::<Validated<Vec<()>, String>>()
            .is_fail(),
        );

        assert_eq!(
            false,
            DatumSchema::Int {
                specification: None,
            }
            .check(&serde_json::json!(4), &|_e, _a| "".to_string())
            .into_iter()
            .collect::<Validated<Vec<()>, String>>()
            .is_fail(),
        );
    }

    #[test]
    fn datum_string_type_validation() {
        assert_eq!(
            true,
            DatumSchema::String {
                specification: None,
            }
            .check(&serde_json::json!({}), &|_e, _a| "".to_string())
            .into_iter()
            .collect::<Validated<Vec<()>, String>>()
            .is_fail(),
        );

        assert_eq!(
            false,
            DatumSchema::String {
                specification: None,
            }
            .check(&serde_json::json!("hello"), &|_e, _a| "".to_string())
            .into_iter()
            .collect::<Validated<Vec<()>, String>>()
            .is_fail(),
        );
    }

    #[test]
    fn datum_list_type_validation() {
        assert_eq!(
            true,
            DatumSchema::List { schema: None }
                .check(&serde_json::json!({}), &|_e, _a| "".to_string())
                .into_iter()
                .collect::<Validated<Vec<()>, String>>()
                .is_fail(),
        );

        assert_eq!(
            false,
            DatumSchema::List { schema: None }
                .check(&serde_json::json!([]), &|_e, _a| "".to_string())
                .into_iter()
                .collect::<Validated<Vec<()>, String>>()
                .is_fail(),
        );
    }

    #[test]
    fn datum_object_type_validation() {
        assert_eq!(
            false,
            DatumSchema::Object { schema: None }
                .check(&serde_json::json!({}), &|_e, _a| "".to_string())
                .into_iter()
                .collect::<Validated<Vec<()>, String>>()
                .is_fail(),
        );

        assert_eq!(
            true,
            DatumSchema::Object { schema: None }
                .check(&serde_json::json!([]), &|_e, _a| "".to_string())
                .into_iter()
                .collect::<Validated<Vec<()>, String>>()
                .is_fail(),
        );
    }

    fn construct_datum_schema_object() -> DatumSchema {
        DatumSchema::Object {
            schema: Some(BTreeMap::from([
                (
                    "name".to_string(),
                    DatumSchema::String {
                        specification: Some(Specification {
                            val: None,
                            min: None,
                            max: None,
                            one_of: Some(vec!["foo".to_string(), "bar".to_string()]),
                            none_of: None,
                        }),
                    },
                ),
                (
                    "cars".to_string(),
                    DatumSchema::List {
                        schema: Some(Box::from(DatumSchema::String {
                            specification: None,
                        })),
                    },
                ),
            ])),
        }
    }

    #[test]
    fn datum_object_member_validation() {
        let datum = construct_datum_schema_object();

        assert_eq!(
            false,
            datum
                .check(
                    &serde_json::json!({
                        "name" : "foo",
                        "cars" : [ "bmw", "porsche", "mercedes"]
                    }),
                    &|_e, _a| "".to_string()
                )
                .into_iter()
                .collect::<Validated<Vec<()>, String>>()
                .is_fail(),
        );
    }

    #[test]
    fn datum_object_member_validation_nested_schema_invalidation_detected() {
        let datum = construct_datum_schema_object();

        assert_eq!(
            true,
            datum
                .check(
                    &serde_json::json!({
                        "name" : "foo",
                        "cars" : [ 1, 2, 3]
                    }),
                    &|_e, _a| "".to_string()
                )
                .into_iter()
                .collect::<Validated<Vec<()>, String>>()
                .is_fail(),
        );
    }

    #[test]
    fn datum_object_member_validation_missing_member_detected() {
        let datum = construct_datum_schema_object();

        assert_eq!(
            true,
            datum
                .check(
                    &serde_json::json!({
                        "name" : "foo"
                    }),
                    &|_e, _a| "".to_string()
                )
                .into_iter()
                .collect::<Validated<Vec<()>, String>>()
                .is_fail(),
        );
    }

    #[test]
    fn unvalidated_response_schema_has_desired_serde_behavior() {
        let foo = serde_json::json!({ "body" : {
            "_jk_schema": {
                "type" : "object"
            }
        }});

        let again: UnvalidatedResponse = serde_json::from_value(foo).unwrap();

        assert_eq!(
            true,
            match again.body.unwrap() {
                ValueOrSchema::Schema(..) => true,
                ValueOrSchema::Value(..) => false,
            }
        );
    }

    #[test]
    fn unvalidated_response_value_has_desired_serde_behavior() {
        let foo = serde_json::json!({ "body" : {
            "my_api_dto`": {
                "foo" : "bar"
            }
        }});

        let again: UnvalidatedResponse = serde_json::from_value(foo).unwrap();

        assert_eq!(
            true,
            match again.body.unwrap() {
                ValueOrSchema::Schema(..) => false,
                ValueOrSchema::Value(..) => true,
            }
        );
    }
}
