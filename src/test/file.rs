use crate::json::filter::filter_json;
use crate::test;
use crate::test::file::Validated::Good;
use crate::test::{definition, http, variable};
use chrono::NaiveDate;
use chrono::NaiveDateTime;
use chrono::TimeZone;
use chrono::{DateTime, ParseError};
use chrono::{Datelike, Local};
use chrono::{Days, Months};
use log::error;
use log::trace;
use nonempty_collections::{IntoNonEmptyIterator, NonEmptyIterator};
use num::Num;
use rand::distributions::uniform::SampleUniform;
use rand::rngs::ThreadRng;
use rand::Rng;
use regex::Regex;
use rnglib::Language;
use rnglib::RNG;
use serde::de::Visitor;
use serde::Deserializer;
use serde::{Deserialize, Serialize};
use serde_json::Map;
use serde_json::Value;
use std::collections::BTreeMap;
use std::error::Error;
use std::fmt::{self};
use std::fmt::{Debug, Display};
use std::fs;
use std::hash::{Hash, Hasher};
use validated::Validated;

impl std::cmp::PartialEq<String> for VariableName {
    fn eq(&self, other: &String) -> bool {
        self.0 == *other
    }
}

#[derive(PartialEq, Eq, Debug, Serialize, Clone)]
pub struct VariableName(String);

impl VariableName {
    pub fn val(&self) -> String {
        self.0.clone()
    }
}

struct VariableNameVisitor;

impl<'de> Visitor<'de> for VariableNameVisitor {
    type Value = VariableName;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("a string starting with a $")
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        if !value.starts_with('$') && !value.starts_with("\"$\"") {
            return Err(E::custom("expecting identifier starting with $"));
        }

        Ok(VariableName(value.to_string()))
    }
}

impl<'de> Deserialize<'de> for VariableName {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_str(VariableNameVisitor)
    }
}

//aka RefOrT , where Ref should refer to a variable
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum UnvalidatedVariableNameOrComponent<T> {
    VariableName(VariableName),
    Component(T),
}

#[derive(Serialize, Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum Specification<T> {
    AnyOf(Vec<T>),
    OneOf(Vec<T>),
    NoneOf(Vec<T>),
    Value(T),
    #[serde(untagged)]
    UnTaggedValue(T),
}

impl<T> Default for Specification<T> {
    fn default() -> Self {
        Specification::NoneOf(vec![])
    }
}

#[derive(Serialize, Debug, Clone, Default, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NumericSpecification<T: std::fmt::Display + Clone + PartialOrd> {
    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    pub specification: Option<Specification<T>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max: Option<T>,
}

pub type BooleanSpecification = Specification<bool>;
pub type FloatSpecification = NumericSpecification<f64>;
pub type IntegerSpecification = NumericSpecification<i64>;

#[derive(Hash, Serialize, Debug, Clone, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct StringSpecification {
    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    pub specification: Option<Specification<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_length: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_length: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pattern: Option<String>,
}

#[derive(Debug, Serialize, Clone, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum ValueOrDatumSchema {
    Datum(DatumSchema),
    Values(Value),
}

impl Hash for ValueOrDatumSchema {
    fn hash<H: Hasher>(&self, state: &mut H) {
        serde_json::to_string(self).unwrap().hash(state)
    }
}

#[derive(Debug, Serialize, Clone, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum ValuesOrSchema {
    Schemas(Specification<Box<DatumSchema>>),
    Values(Specification<Vec<Value>>),
}

impl ValuesOrSchema {
    pub fn generate_if_constrained(&self, rng: &mut ThreadRng) -> Option<Value> {
        trace!("generate_if_constrained()");
        match self {
            ValuesOrSchema::Schemas(schema) => schema.schema_generate_if_constrained(rng),
            ValuesOrSchema::Values(vals) => vals
                .generate_if_constrained(rng)
                .map(serde_json::Value::from),
        }
    }

    fn check(
        &self,
        val: &Vec<Value>,
        strict: bool,
        formatter: &impl Fn(&str, &str) -> String,
    ) -> Vec<Validated<(), String>> {
        match &self {
            ValuesOrSchema::Schemas(schema) => schema.schema_check(val, strict, formatter),
            ValuesOrSchema::Values(vals) => vals.vec_check(val, strict, formatter),
        }
    }
}

#[derive(Serialize, Debug, Clone, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct SequenceSpecification {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema: Option<ValuesOrSchema>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_length: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_length: Option<i64>,
}

#[derive(Hash, Default, Serialize, Debug, Clone, Deserialize, PartialEq)]
pub struct DateSpecification {
    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    pub specification: Option<Specification<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max: Option<String>,
    pub format: Option<String>,
    pub modifier: Option<variable::Modifier>,
}

#[derive(Hash, Default, Serialize, Debug, Clone, Deserialize, PartialEq)]
pub struct DateTimeSpecification {
    #[serde(flatten)]
    pub specification: Option<Specification<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max: Option<String>,
    pub format: Option<String>,
    pub modifier: Option<variable::Modifier>,
}

#[derive(Hash, Default, Serialize, Debug, Clone, Deserialize, PartialEq)]
pub struct NameSpecification {
    #[serde(flatten)]
    pub specification: StringSpecification,
}

#[derive(Hash, Default, Serialize, Debug, Clone, Deserialize, PartialEq)]
pub struct EmailSpecification {
    #[serde(flatten)]
    pub specification: StringSpecification,
}

pub trait Checker {
    type Item;
    fn check(
        &self,
        val: &Self::Item,
        formatter: &impl Fn(&str, &str) -> String,
    ) -> Vec<Validated<(), String>>;
}

impl Checker for NameSpecification {
    type Item = String;
    fn check(
        &self,
        val: &Self::Item,
        formatter: &impl Fn(&str, &str) -> String,
    ) -> Vec<Validated<(), String>> {
        self.specification.check(val, formatter)
    }
}

impl Checker for EmailSpecification {
    type Item = String;
    fn check(
        &self,
        val: &Self::Item,
        formatter: &impl Fn(&str, &str) -> String,
    ) -> Vec<Validated<(), String>> {
        //standard browser regex for email
        let email_regex: Regex = Regex::new(
            r"^[a-zA-Z0-9.!#$%&'*+/=?^_`{|}~-]+@[a-zA-Z0-9](?:[a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?(?:\.[a-zA-Z0-9](?:[a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?)*$",
        )
        .unwrap();

        let matches = email_regex.is_match(val);

        if !matches {
            trace!("failed email regex");
            vec![Validated::fail(formatter("email format", val))]
        } else {
            self.specification.check(val, formatter)
        }
    }
}

impl Specification<Box<DatumSchema>> {
    fn schema_check(
        &self,
        vals: &Vec<Value>,
        strict: bool,
        formatter: &impl Fn(&str, &str) -> String,
    ) -> Vec<Validated<(), String>> {
        let findings = match self {
            Specification::NoneOf(none_ofs) => {
                self.schema_check_none_of(vals, none_ofs, strict, formatter)
            }
            Specification::AnyOf(any_ofs) => {
                self.schema_any_one_of(vals, any_ofs, strict, formatter)
            }
            Specification::OneOf(one_ofs) => {
                self.schema_check_one_of(vals, one_ofs, strict, formatter)
            }
            Specification::Value(val) => self.schema_check_val(vals, val, strict, formatter),
            Specification::UnTaggedValue(val) => {
                self.schema_check_val(vals, val, strict, formatter)
            }
        };

        vec![findings]
    }

    fn schema_generate_if_constrained(&self, rng: &mut ThreadRng) -> Option<Value> {
        trace!("schema_generate_if_constrained()");
        match &self {
            Specification::Value(v) => generate_value_from_schema(v, 1),
            Specification::UnTaggedValue(v) => generate_value_from_schema(v, 1),
            Specification::OneOf(oneofs) => oneofs
                .get(rng.gen_range(0..oneofs.len()))
                .and_then(|s| generate_value_from_schema(s, 1)),
            Specification::AnyOf(oneofs) => oneofs
                .get(rng.gen_range(0..oneofs.len()))
                .and_then(|s| generate_value_from_schema(s, 1)),
            _ => None,
        }
    }

    fn schema_check_val(
        &self,
        actuals: &Vec<Value>,
        specified_value: &DatumSchema,
        strict: bool,
        formatter: &impl Fn(&str, &str) -> String,
    ) -> Validated<(), String> {
        if actuals.iter().all(|actual| {
            specified_value
                .check(actual, strict, formatter)
                .iter()
                .all(|v| v.is_good())
        }) {
            Good(())
        } else {
            Validated::fail(formatter(
                format!("{:?}", specified_value).as_str(),
                format!("{:?}", actuals).as_str(),
            ))
        }
    }

    fn schema_check_one_of(
        &self,
        actuals: &Vec<Value>,
        specified_values: &[Box<DatumSchema>],
        strict: bool,
        formatter: &impl Fn(&str, &str) -> String,
    ) -> Validated<(), String> {
        let matchers = specified_values
            .iter()
            .filter(|v| {
                actuals.iter().all(|actual| {
                    v.check(actual, strict, formatter)
                        .iter()
                        .all(|validation| validation.is_good())
                })
            })
            .collect::<Vec<&Box<DatumSchema>>>();
        if matchers.len() == 1 {
            Good(())
        } else {
            Validated::fail(formatter(
                format!("one of {:?}", specified_values).as_str(),
                format!("{:?}", actuals).as_str(),
            ))
        }
    }

    fn schema_any_one_of(
        &self,
        actuals: &Vec<Value>,
        specified_values: &[Box<DatumSchema>],
        strict: bool,
        formatter: &impl Fn(&str, &str) -> String,
    ) -> Validated<(), String> {
        if actuals.iter().all(|actual| {
            specified_values.iter().any(|x| {
                x.check(actual, strict, formatter)
                    .iter()
                    .all(|v| v.is_good())
            })
        }) {
            Good(())
        } else {
            Validated::fail(formatter(
                format!("one of {:?}", specified_values).as_str(),
                format!("{:?}", actuals).as_str(),
            ))
        }
    }

    fn schema_check_none_of(
        &self,
        actuals: &Vec<Value>,
        specified_values: &[Box<DatumSchema>],
        strict: bool,
        formatter: &impl Fn(&str, &str) -> String,
    ) -> Validated<(), String> {
        if actuals.iter().all(|actual| {
            !specified_values.iter().any(|x| {
                x.check(actual, strict, formatter)
                    .iter()
                    .all(|v| v.is_good())
            })
        }) {
            Good(())
        } else {
            Validated::fail(formatter(
                format!("none of {:?}", specified_values).as_str(),
                format!("{:?}", actuals).as_str(),
            ))
        }
    }
}

impl Specification<Vec<Value>> {
    //Strict currently isn't supported for the arrays
    //We can add later, but assert_json_diff::CompareMode::Inclusive produces
    //weird results for arrays
    fn vec_check(
        &self,
        vals: &Vec<Value>,
        strict: bool,
        formatter: &impl Fn(&str, &str) -> String,
    ) -> Vec<Validated<(), String>> {
        let findings = match self {
            Specification::NoneOf(none_ofs) => {
                self.vec_check_none_of(vals, none_ofs, strict, formatter)
            }
            Specification::AnyOf(any_ofs) => {
                self.vec_check_any_of(vals, any_ofs, strict, formatter)
            }
            Specification::OneOf(one_ofs) => {
                self.vec_check_one_of(vals, one_ofs, strict, formatter)
            }
            Specification::Value(val) => self.vec_check_val(vals, val, strict, formatter),
            Specification::UnTaggedValue(val) => self.vec_check_val(vals, val, strict, formatter),
        };

        vec![findings]
    }

    fn vec_check_val(
        &self,
        actual: &Vec<Value>,
        specified_value: &Vec<Value>,
        _strict: bool,
        formatter: &impl Fn(&str, &str) -> String,
    ) -> Validated<(), String> {
        if specified_value == actual {
            Good(())
        } else {
            Validated::fail(formatter(
                format!("{:?}", specified_value).as_str(),
                format!("{:?}", actual).as_str(),
            ))
        }
    }

    fn vec_check_one_of(
        &self,
        actual: &Vec<Value>,
        specified_values: &[Vec<Value>],
        _strict: bool,
        formatter: &impl Fn(&str, &str) -> String,
    ) -> Validated<(), String> {
        let hits = specified_values
            .iter()
            .filter(|v| *v == actual)
            .collect::<Vec<&Vec<Value>>>();

        if hits.len() == 1 {
            Good(())
        } else {
            Validated::fail(formatter(
                format!("one of {:?}", specified_values).as_str(),
                format!("{:?}", actual).as_str(),
            ))
        }
    }

    fn vec_check_any_of(
        &self,
        actual: &Vec<Value>,
        specified_values: &[Vec<Value>],
        _strict: bool,
        formatter: &impl Fn(&str, &str) -> String,
    ) -> Validated<(), String> {
        if specified_values.contains(actual) {
            Good(())
        } else {
            Validated::fail(formatter(
                format!("one of {:?}", specified_values).as_str(),
                format!("{:?}", actual).as_str(),
            ))
        }
    }

    fn vec_check_none_of(
        &self,
        actual: &Vec<Value>,
        specified_values: &[Vec<Value>],
        _strict: bool,
        formatter: &impl Fn(&str, &str) -> String,
    ) -> Validated<(), String> {
        if !specified_values.contains(actual) {
            Good(())
        } else {
            Validated::fail(formatter(
                format!("none of {:?}", specified_values).as_str(),
                format!("{:?}", actual).as_str(),
            ))
        }
    }
}

impl<T> Specification<T>
where
    T: PartialEq,
    T: fmt::Debug,
    T: Clone,
{
    fn generate_if_constrained(&self, rng: &mut ThreadRng) -> Option<T> {
        trace!("generate_if_constrained{:?}", &self);
        match &self {
            Specification::Value(v) => Some(v.clone()),
            Specification::UnTaggedValue(v) => Some(v.clone()),
            Specification::OneOf(oneofs) => oneofs.get(rng.gen_range(0..oneofs.len())).cloned(),
            Specification::AnyOf(anyofs) => anyofs.get(rng.gen_range(0..anyofs.len())).cloned(),
            Specification::NoneOf(_) => None,
        }
    }

    fn check_val(
        &self,
        actual: &T,
        specified_value: &T,
        formatter: &impl Fn(&str, &str) -> String,
    ) -> Validated<(), String> {
        if specified_value == actual {
            Good(())
        } else {
            Validated::fail(formatter(
                format!("{:?}", specified_value).as_str(),
                format!("{:?}", actual).as_str(),
            ))
        }
    }

    fn check_one_of(
        &self,
        actual: &T,
        specified_values: &[T],
        formatter: &impl Fn(&str, &str) -> String,
    ) -> Validated<(), String> {
        let hits = specified_values
            .iter()
            .filter(|v| *v == actual)
            .collect::<Vec<&T>>();

        if hits.len() == 1 {
            Good(())
        } else {
            Validated::fail(formatter(
                format!("one of {:?}", specified_values).as_str(),
                format!("{:?}", actual).as_str(),
            ))
        }
    }

    fn check_any_of(
        &self,
        actual: &T,
        specified_values: &[T],
        formatter: &impl Fn(&str, &str) -> String,
    ) -> Validated<(), String> {
        if specified_values.contains(actual) {
            Good(())
        } else {
            Validated::fail(formatter(
                format!("one of {:?}", specified_values).as_str(),
                format!("{:?}", actual).as_str(),
            ))
        }
    }

    fn check_none_of(
        &self,
        actual: &T,
        specified_values: &[T],
        formatter: &impl Fn(&str, &str) -> String,
    ) -> Validated<(), String> {
        if !specified_values.contains(actual) {
            Good(())
        } else {
            Validated::fail(formatter(
                format!("none of {:?}", specified_values).as_str(),
                format!("{:?}", actual).as_str(),
            ))
        }
    }
}

impl<T> Hash for Specification<T>
where
    T: PartialEq,
    T: Display,
    T: PartialOrd,
    T: fmt::Debug,
    T: Display,
    T: Serialize,
    T: Clone,
{
    fn hash<H: Hasher>(&self, state: &mut H) {
        serde_json::to_string(self).unwrap().hash(state)
    }
}

impl Hash for SequenceSpecification {
    fn hash<H: Hasher>(&self, state: &mut H) {
        serde_json::to_string(self).unwrap().hash(state)
    }
}

impl<T> Hash for NumericSpecification<T>
where
    T: PartialEq,
    T: Display,
    T: PartialOrd,
    T: fmt::Debug,
    T: Serialize,
    T: Clone,
{
    fn hash<H: Hasher>(&self, state: &mut H) {
        serde_json::to_string(self).unwrap().hash(state)
    }
}

fn min_less_than_equal_max<T: PartialOrd>(min: &Option<T>, max: &Option<T>) -> bool {
    min.as_ref()
        .zip(max.as_ref())
        .map(|(min, max)| min <= max)
        .unwrap_or(true)
}

impl<T> NumericSpecification<T>
where
    T: PartialEq,
    T: Display,
    T: PartialOrd,
    T: fmt::Debug,
    T: Clone,
{
    pub fn new(
        specification: Option<Specification<T>>,
        min: Option<T>,
        max: Option<T>,
    ) -> Result<Self, String> {
        if min_less_than_equal_max(&min, &max) {
            Ok(Self {
                specification,
                min,
                max,
            })
        } else {
            Err("min must be less than or equal to max".to_string())
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
}

impl<T> Checker for Specification<T>
where
    T: PartialEq,
    T: fmt::Debug,
    T: Clone,
{
    type Item = T;
    fn check(
        &self,
        val: &T,
        formatter: &impl Fn(&str, &str) -> String,
    ) -> Vec<Validated<(), String>> {
        vec![match self {
            Specification::NoneOf(nones) => self.check_none_of(val, nones, formatter),
            Specification::OneOf(oneofs) => self.check_one_of(val, oneofs, formatter),
            Specification::AnyOf(anyofs) => self.check_any_of(val, anyofs, formatter),
            Specification::Value(specified_value) => {
                self.check_val(val, specified_value, formatter)
            }
            Specification::UnTaggedValue(specified_value) => {
                self.check_val(val, specified_value, formatter)
            }
        }]
    }
}

impl<T> Checker for NumericSpecification<T>
where
    T: PartialEq,
    T: Display,
    T: PartialOrd,
    T: fmt::Debug,
    T: Clone,
{
    type Item = T;
    fn check(
        &self,
        val: &T,
        formatter: &impl Fn(&str, &str) -> String,
    ) -> Vec<Validated<(), String>> {
        let mut ret = vec![
            self.check_min(val, formatter),
            self.check_max(val, formatter),
        ];
        ret.append(
            self.specification
                .as_ref()
                .map(|s| s.check(val, formatter))
                .unwrap_or_default()
                .as_mut(),
        );
        ret
    }
}

impl StringSpecification {
    pub fn new(
        specification: Option<Specification<String>>,
        min_length: Option<i64>,
        max_length: Option<i64>,
        pattern: Option<String>,
    ) -> Result<Self, String> {
        let negative_validator = |number: &Option<i64>, var_name: &str| {
            number
                .as_ref()
                .map(|s| {
                    if s.is_negative() {
                        Validated::fail(format!("negative value provided for {var_name}"))
                    } else {
                        Good(*number)
                    }
                })
                .unwrap_or(Validated::Good(None))
        };

        let negative_validation_max = negative_validator(&max_length, "max");
        let negative_validation_min = negative_validator(&min_length, "min");
        let relation_validation = if min_less_than_equal_max(&min_length, &max_length) {
            Good(())
        } else {
            Validated::fail("min_length must be less than or equal to max_length".to_string())
        };

        let pattern_validation = pattern
            //.as_ref()
            .map(|p| {
                Regex::new(p.as_str())
                    .map(|_| Good(Some(p)))
                    .unwrap_or_else(|e| {
                        Validated::fail(format!("invalid regex supplied for pattern : {}", e))
                    })
            })
            .unwrap_or(Good(None));

        negative_validation_max
            .map4(
                negative_validation_min,
                relation_validation,
                pattern_validation,
                |_, _, _, p| Self {
                    max_length,
                    min_length,
                    specification,
                    pattern: p,
                },
            )
            .ok()
            .map_err(|nev| {
                nev.into_nonempty_iter()
                    .reduce(|acc, e| format!("{},{}", acc, e))
            })
    }

    fn check_pattern(
        &self,
        actual: &str,
        formatter: &impl Fn(&str, &str) -> String,
    ) -> Validated<(), String> {
        match &self.pattern {
            Some(p) => {
                let result = Regex::new(p).map(|re| {
                    if re.is_match(actual) {
                        Good(())
                    } else {
                        Validated::fail(formatter(format!("pattern of {}", p).as_str(), actual))
                    }
                });

                result.unwrap_or_else(|e| {
                    Validated::fail(formatter("valid regex", format!("{:?}", e).as_str()))
                })
            }
            _ => Good(()),
        }
    }

    fn check_min_length(
        &self,
        actual: &str,
        formatter: &impl Fn(&str, &str) -> String,
    ) -> Validated<(), String> {
        match &self.min_length {
            Some(t) => {
                if *t <= actual.len() as i64 {
                    Good(())
                } else {
                    Validated::fail(formatter(
                        format!("minimum length of {}", t).as_str(),
                        actual,
                    ))
                }
            }
            None => Good(()),
        }
    }

    fn check_max_length(
        &self,
        actual: &str,
        formatter: &impl Fn(&str, &str) -> String,
    ) -> Validated<(), String> {
        match &self.max_length {
            Some(t) => {
                if *t >= actual.len() as i64 {
                    Good(())
                } else {
                    Validated::fail(formatter(
                        format!("maximum length of {}", t).as_str(),
                        actual,
                    ))
                }
            }
            None => Good(()),
        }
    }
}

impl SequenceSpecification {
    pub fn new(
        schema: Option<ValuesOrSchema>,
        min_length: Option<i64>,
        max_length: Option<i64>,
    ) -> Result<Self, String> {
        let negative_validator = |number: &Option<i64>, var_name: &str| {
            number
                .as_ref()
                .map(|s| {
                    if s.is_negative() {
                        Validated::fail(format!("negative value provided for {var_name}"))
                    } else {
                        Good(*number)
                    }
                })
                .unwrap_or(Validated::Good(None))
        };

        let negative_validation_max = negative_validator(&max_length, "max");
        let negative_validation_min = negative_validator(&min_length, "min");
        let relation_validation = if min_less_than_equal_max(&min_length, &max_length) {
            Good(())
        } else {
            Validated::fail("min_length must be less than or equal to max_length".to_string())
        };

        negative_validation_max
            .map3(negative_validation_min, relation_validation, |_, _, _| {
                Self {
                    max_length,
                    min_length,
                    schema,
                }
            })
            .ok()
            .map_err(|nev| {
                nev.into_nonempty_iter()
                    .reduce(|acc, e| format!("{},{}", acc, e))
            })
    }

    fn check_min_length(
        &self,
        actual: &Vec<Value>,
        formatter: &impl Fn(&str, &str) -> String,
    ) -> Validated<(), String> {
        match &self.min_length {
            Some(t) => {
                if *t <= actual.len() as i64 {
                    Good(())
                } else {
                    Validated::fail(formatter(
                        format!("minimum length of {}", t).as_str(),
                        format!("{:?}", actual).as_str(),
                    ))
                }
            }
            None => Good(()),
        }
    }

    fn check_max_length(
        &self,
        actual: &Vec<Value>,
        formatter: &impl Fn(&str, &str) -> String,
    ) -> Validated<(), String> {
        match &self.max_length {
            Some(t) => {
                if *t >= actual.len() as i64 {
                    Good(())
                } else {
                    Validated::fail(formatter(
                        format!("maximum length of {}", t).as_str(),
                        format!("{:?}", actual).as_str(),
                    ))
                }
            }
            None => Good(()),
        }
    }

    fn check(
        &self,
        val: &Vec<Value>,
        strict: bool,
        formatter: &impl Fn(&str, &str) -> String,
    ) -> Vec<Validated<(), String>> {
        let mut ret = vec![
            self.check_min_length(val, formatter),
            self.check_max_length(val, formatter),
        ];
        ret.append(
            self.schema
                .as_ref()
                .map(|s| s.check(val, strict, formatter))
                .unwrap_or_default()
                .as_mut(),
        );
        ret
    }
}

impl Checker for StringSpecification {
    type Item = String;
    fn check(
        &self,
        val: &String,
        formatter: &impl Fn(&str, &str) -> String,
    ) -> Vec<Validated<(), String>> {
        let mut ret = vec![
            self.check_min_length(val, formatter),
            self.check_max_length(val, formatter),
            self.check_pattern(val, formatter),
        ];
        ret.append(
            self.specification
                .as_ref()
                .map(|s| s.check(val, formatter))
                .unwrap_or_default()
                .as_mut(),
        );
        ret
    }
}

impl NameSpecification {
    pub fn new(string_specification: StringSpecification) -> Result<Self, String> {
        StringSpecification::new(
            string_specification.specification,
            string_specification.min_length,
            string_specification.max_length,
            string_specification.pattern,
        )
        .map(|s| Self { specification: s })
    }
}

impl EmailSpecification {
    pub fn new(string_specification: StringSpecification) -> Result<Self, String> {
        StringSpecification::new(
            string_specification.specification,
            string_specification.min_length,
            string_specification.max_length,
            string_specification.pattern,
        )
        .map(|s| Self { specification: s })
    }
}

impl DateSpecification {
    const DEFAULT_FORMAT: &'static str = "%Y-%m-%d";

    // \todo validate modifier,
    pub fn new(
        specification: Option<Specification<String>>,
        min: Option<String>,
        max: Option<String>,
        format: Option<String>,
        modifier: Option<variable::Modifier>,
    ) -> Result<Self, String> {
        let date_validator = |date_string: &Option<String>, var_name: &str| {
            date_string
                .as_ref()
                .map(|s| {
                    let res = Self::str_to_time_with_format(
                        s,
                        format.as_ref().unwrap_or(&Self::DEFAULT_FORMAT.to_string()),
                    );
                    if res.is_err() {
                        Validated::fail(format!("invalid date provided for {var_name} "))
                    } else {
                        Good(date_string.clone())
                    }
                })
                .unwrap_or(Validated::Good(None))
        };
        date_validator(&min, "min")
            .map2(date_validator(&max, "max"), |min_v, max_v| Self {
                format,
                min: min_v,
                max: max_v,
                modifier,
                specification,
            })
            .ok()
            .map_err(|nev| {
                nev.into_nonempty_iter()
                    .reduce(|acc, e| format!("{},{}", acc, e))
            })
    }

    fn check_min(
        &self,
        actual: &DateTime<Local>,
        formatter: &impl Fn(&str, &str) -> String,
    ) -> Validated<(), String> {
        match &self.min {
            Some(t) => {
                if self
                    .str_to_time(t)
                    .map(|min| min <= *actual)
                    .unwrap_or_default()
                {
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
        actual: &DateTime<Local>,
        formatter: &impl Fn(&str, &str) -> String,
    ) -> Validated<(), String> {
        match &self.max {
            Some(t) => {
                if self
                    .str_to_time(t)
                    .map(|max| max >= *actual)
                    .unwrap_or_default()
                {
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

    fn get_format(&self) -> String {
        self.format
            .clone()
            .unwrap_or(Self::DEFAULT_FORMAT.to_string())
    }

    fn str_to_time_with_format(
        string_val: &str,
        format: &str,
    ) -> Result<DateTime<Local>, ParseError> {
        NaiveDate::parse_from_str(string_val, format).map(|d| {
            Local
                .from_local_datetime(&d.and_hms_opt(0, 0, 0).unwrap())
                .unwrap()
        })
    }

    pub fn str_to_time(&self, string_val: &str) -> Result<DateTime<Local>, ParseError> {
        Self::str_to_time_with_format(string_val, &self.get_format())
    }

    fn time_to_str(&self, time: &DateTime<Local>) -> String {
        let format = self.get_format();
        format!("{}", time.format(&format))
    }

    //validates format stuff and applies the modifier
    //this can be used to generate or validate
    //But its not a "random" generator like our other specifications
    fn get(&self, string_val: &str) -> Result<String, ParseError> {
        //debug!("string expression: {:?}", v);
        //let mut result_date;
        self.str_to_time(string_val)
            .map(|mut dt| {
                if let Some(m) = &self.modifier {
                    let mod_value_result = m.value.parse::<u64>();
                    if let Ok(mod_value) = mod_value_result {
                        match m.operation.to_lowercase().as_str() {
                            "add" => {
                                let modified_date = match m.unit.to_lowercase().as_str() {
                                    "days" => dt.checked_add_days(Days::new(mod_value)),
                                    "weeks" => dt.checked_add_days(Days::new(mod_value * 7)),
                                    "months" => {
                                        dt.checked_add_months(Months::new(mod_value as u32))
                                    }
                                    // TODO: add support for years
                                    _ => None,
                                };

                                if let Some(md) = modified_date {
                                    dt = md;
                                }
                            }
                            "subtract" => {
                                let modified_date = match m.unit.to_lowercase().as_str() {
                                    "days" => dt.checked_sub_days(Days::new(mod_value)),
                                    "weeks" => dt.checked_sub_days(Days::new(mod_value * 7)),
                                    "months" => {
                                        dt.checked_sub_months(Months::new(mod_value as u32))
                                    }
                                    // TODO: add support for years
                                    _ => None,
                                };

                                if let Some(md) = modified_date {
                                    dt = md;
                                }
                            }
                            _ => {}
                        }
                    }
                }
                dt
            })
            .map(|d| self.time_to_str(&d))
    }
}

impl Checker for DateSpecification {
    type Item = String;
    fn check(
        &self,
        val: &Self::Item,
        formatter: &impl Fn(&str, &str) -> String,
    ) -> Vec<Validated<(), String>> {
        let maybe_time = self.str_to_time(val);
        if maybe_time.is_err() {
            return vec![Validated::fail(formatter("date type", "type"))];
        }

        let time = maybe_time.unwrap();

        let mut ret = vec![
            self.check_min(&time, formatter),
            self.check_max(&time, formatter),
        ];

        ret.append(
            self.specification
                .as_ref()
                .map(|s| s.check(val, formatter))
                .unwrap_or_default()
                .as_mut(),
        );

        ret
    }
}

impl DateTimeSpecification {
    const DEFAULT_FORMAT: &'static str = "%Y-%m-%d %H:%M:%S%.f";

    pub fn new(
        specification: Option<Specification<String>>,
        min: Option<String>,
        max: Option<String>,
        format: Option<String>,
        modifier: Option<variable::Modifier>,
    ) -> Result<Self, String> {
        let date_validator = |date_string: &Option<String>, var_name: &str| {
            date_string
                .as_ref()
                .map(|s| {
                    let res = Self::str_to_time_with_format(
                        s,
                        format.as_ref().unwrap_or(&Self::DEFAULT_FORMAT.to_string()),
                    );
                    if res.is_err() {
                        Validated::fail(format!("invalid date provided for {var_name} "))
                    } else {
                        Good(date_string.clone())
                    }
                })
                .unwrap_or(Validated::Good(None))
        };
        date_validator(&min, "min")
            .map2(date_validator(&max, "max"), |min_v, max_v| Self {
                format,
                min: min_v,
                max: max_v,
                modifier,
                specification,
            })
            .ok()
            .map_err(|nev| {
                nev.into_nonempty_iter()
                    .reduce(|acc, e| format!("{},{}", acc, e))
            })
    }

    fn check_min(
        &self,
        actual: &DateTime<Local>,
        formatter: &impl Fn(&str, &str) -> String,
    ) -> Validated<(), String> {
        match &self.min {
            Some(t) => {
                if self
                    .str_to_time(t)
                    .map(|min| min <= *actual)
                    .unwrap_or_default()
                {
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
        actual: &DateTime<Local>,
        formatter: &impl Fn(&str, &str) -> String,
    ) -> Validated<(), String> {
        match &self.max {
            Some(t) => {
                if self
                    .str_to_time(t)
                    .map(|max| max >= *actual)
                    .unwrap_or_default()
                {
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

    fn get_format(&self) -> String {
        self.format
            .clone()
            .unwrap_or(Self::DEFAULT_FORMAT.to_string())
    }

    pub fn str_to_time_with_format(
        string_val: &str,
        format: &str,
    ) -> Result<DateTime<Local>, ParseError> {
        NaiveDateTime::parse_from_str(string_val, format)
            .map(|d| Local.from_local_datetime(&d).unwrap())
    }

    pub fn str_to_time(&self, string_val: &str) -> Result<DateTime<Local>, ParseError> {
        Self::str_to_time_with_format(string_val, &self.get_format())
    }

    fn time_to_str(&self, time: &DateTime<Local>) -> String {
        let format = self.get_format();
        format!("{}", time.format(&format))
    }

    //validates format stuff and applies the modifier
    //this can be used to generate or validate
    //But its not a "random" generator like our other specifications
    fn get(&self, string_val: &str) -> Result<String, ParseError> {
        //debug!("string expression: {:?}", v);
        //let mut result_date;
        self.str_to_time(string_val)
            .map(|mut dt| {
                if let Some(m) = &self.modifier {
                    let mod_value_result = m.value.parse::<u64>();
                    if let Ok(mod_value) = mod_value_result {
                        match m.operation.to_lowercase().as_str() {
                            "add" => {
                                let modified_date = match m.unit.to_lowercase().as_str() {
                                    "days" => dt.checked_add_days(Days::new(mod_value)),
                                    "weeks" => dt.checked_add_days(Days::new(mod_value * 7)),
                                    "months" => {
                                        dt.checked_add_months(Months::new(mod_value as u32))
                                    }
                                    // TODO: add support for years
                                    _ => None,
                                };

                                if let Some(md) = modified_date {
                                    dt = md;
                                }
                            }
                            "subtract" => {
                                let modified_date = match m.unit.to_lowercase().as_str() {
                                    "days" => dt.checked_sub_days(Days::new(mod_value)),
                                    "weeks" => dt.checked_sub_days(Days::new(mod_value * 7)),
                                    "months" => {
                                        dt.checked_sub_months(Months::new(mod_value as u32))
                                    }
                                    // TODO: add support for years
                                    _ => None,
                                };

                                if let Some(md) = modified_date {
                                    dt = md;
                                }
                            }
                            _ => {}
                        }
                    }
                }
                dt
            })
            .map(|d| self.time_to_str(&d))
    }
}

impl Checker for DateTimeSpecification {
    type Item = String;
    fn check(
        &self,
        val: &Self::Item,
        formatter: &impl Fn(&str, &str) -> String,
    ) -> Vec<Validated<(), String>> {
        let maybe_time = self.str_to_time(val);
        if maybe_time.is_err() {
            return vec![Validated::fail(formatter("date type", "type"))];
        }

        let time = maybe_time.unwrap();

        let mut ret = vec![
            self.check_min(&time, formatter),
            self.check_max(&time, formatter),
        ];

        ret.append(
            self.specification
                .as_ref()
                .map(|s| s.check(val, formatter))
                .unwrap_or_default()
                .as_mut(),
        );

        ret
    }
}

#[derive(Hash, Serialize, Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
#[serde(tag = "type")]
pub enum DatumSchema {
    Boolean {
        #[serde(flatten)]
        specification: Option<BooleanSpecification>,
    },
    Float {
        #[serde(flatten)]
        specification: Option<FloatSpecification>,
    },
    Int {
        #[serde(flatten)]
        specification: Option<IntegerSpecification>,
    },
    String {
        #[serde(flatten)]
        specification: Option<StringSpecification>,
    },
    Date {
        #[serde(flatten)]
        specification: Option<DateSpecification>,
    },
    DateTime {
        #[serde(flatten)]
        specification: Option<DateTimeSpecification>,
    },
    Name {
        #[serde(flatten)]
        specification: Option<NameSpecification>,
    },
    Email {
        #[serde(flatten)]
        specification: Option<EmailSpecification>,
    },
    List {
        #[serde(flatten)]
        specification: Option<SequenceSpecification>,
    },
    Object {
        #[serde(skip_serializing_if = "Option::is_none")]
        schema: Option<BTreeMap<String, ValueOrDatumSchema>>,
    },
}

impl DatumSchema {
    fn check(
        &self,
        actual: &serde_json::Value,
        strict: bool,
        formatter: &impl Fn(&str, &str) -> String,
    ) -> Vec<Validated<(), String>> {
        trace!("validating datum");
        match self {
            DatumSchema::Date { specification } => {
                Self::check_date(specification, actual, formatter)
            }
            DatumSchema::DateTime { specification } => {
                Self::check_datetime(specification, actual, formatter)
            }
            DatumSchema::Email { specification } => {
                Self::check_email(specification, actual, formatter)
            }
            DatumSchema::Float { specification } => {
                Self::check_float(specification, actual, formatter)
            }
            DatumSchema::Boolean { specification } => {
                Self::check_bool(specification, actual, formatter)
            }
            DatumSchema::Int { specification } => Self::check_int(specification, actual, formatter),
            DatumSchema::List { specification } => {
                Self::check_list(specification, actual, strict, formatter)
            }
            DatumSchema::Object { schema } => {
                Self::check_value_or_datumschema(schema, actual, strict, formatter)
            }
            DatumSchema::Name { specification } => {
                Self::check_name(specification, actual, formatter)
            }
            DatumSchema::String { specification } => {
                Self::check_string(specification, actual, formatter)
            }
        }
    }

    fn check_name(
        spec: &Option<NameSpecification>,
        actual: &serde_json::Value,
        formatter: &impl Fn(&str, &str) -> String,
    ) -> Vec<Validated<(), String>> {
        if !actual.is_string() {
            return vec![Validated::fail(formatter("string type", "type"))];
        }

        spec.as_ref()
            .map(|s| s.check(&String::from(actual.as_str().unwrap()), formatter))
            .unwrap_or(vec![Good(())])
            .clone()
    }

    fn check_email(
        spec: &Option<EmailSpecification>,
        actual: &serde_json::Value,
        formatter: &impl Fn(&str, &str) -> String,
    ) -> Vec<Validated<(), String>> {
        if !actual.is_string() {
            return vec![Validated::fail(formatter("string type", "type"))];
        }

        spec.as_ref()
            .map(|s| s.check(&String::from(actual.as_str().unwrap()), formatter))
            .unwrap_or(vec![Good(())])
            .clone()
    }

    fn check_date(
        spec: &Option<DateSpecification>,
        actual: &serde_json::Value,
        formatter: &impl Fn(&str, &str) -> String,
    ) -> Vec<Validated<(), String>> {
        if !actual.is_string() {
            return vec![Validated::fail(formatter("string type", "type"))];
        }

        spec.as_ref()
            .map(|s| s.check(&String::from(actual.as_str().unwrap()), formatter))
            .unwrap_or(vec![Good(())])
    }

    fn check_datetime(
        spec: &Option<DateTimeSpecification>,
        actual: &serde_json::Value,
        formatter: &impl Fn(&str, &str) -> String,
    ) -> Vec<Validated<(), String>> {
        if !actual.is_string() {
            return vec![Validated::fail(formatter("string type", "type"))];
        }

        spec.as_ref()
            .map(|s| s.check(&String::from(actual.as_str().unwrap()), formatter))
            .unwrap_or(vec![Good(())])
    }

    fn check_bool(
        spec: &Option<BooleanSpecification>,
        actual: &serde_json::Value,
        formatter: &impl Fn(&str, &str) -> String,
    ) -> Vec<Validated<(), String>> {
        if !actual.is_boolean() {
            return vec![Validated::fail(formatter("bool type", "type"))];
        }

        spec.as_ref()
            .map(|s| s.check(&actual.as_bool().unwrap(), formatter))
            .unwrap_or(vec![Good(())])
    }

    fn check_float(
        spec: &Option<NumericSpecification<f64>>,
        actual: &serde_json::Value,
        formatter: &impl Fn(&str, &str) -> String,
    ) -> Vec<Validated<(), String>> {
        if !actual.is_f64() {
            return vec![Validated::fail(formatter("float type", "type"))];
        }

        spec.as_ref()
            .map(|s| s.check(&actual.as_f64().unwrap(), formatter))
            .unwrap_or(vec![Good(())])
    }

    fn check_int(
        spec: &Option<NumericSpecification<i64>>,
        actual: &serde_json::Value,
        formatter: &impl Fn(&str, &str) -> String,
    ) -> Vec<Validated<(), String>> {
        if !actual.is_i64() {
            return vec![Validated::fail(formatter("int type", "type"))];
        }

        spec.as_ref()
            .map(|s| s.check(&actual.as_i64().unwrap(), formatter))
            .unwrap_or(vec![Good(())])
    }

    fn check_string(
        spec: &Option<StringSpecification>,
        actual: &serde_json::Value,
        formatter: &impl Fn(&str, &str) -> String,
    ) -> Vec<Validated<(), String>> {
        if !actual.is_string() {
            return vec![Validated::fail(formatter("string type", "type"))];
        }

        spec.as_ref()
            .map(|s| s.check(&actual.as_str().unwrap().to_string(), formatter))
            .unwrap_or(vec![Good(())])
    }

    fn check_list(
        specification: &Option<SequenceSpecification>,
        actual: &serde_json::Value,
        strict: bool,
        formatter: &impl Fn(&str, &str) -> String,
    ) -> Vec<Validated<(), String>> {
        if !actual.is_array() {
            return vec![Validated::fail(formatter("array type", "different type"))];
        }

        specification
            .as_ref()
            .map(|s| s.check(actual.as_array().unwrap(), strict, formatter))
            .unwrap_or(vec![Good(())])
    }

    /*
       Strict :
            If there is any difference between the keys of actual and expected, we fail

       Non strict:
            If there is stuff in expected that is not in actual, we fail
            If there is stuff in actual that is not in expected, we're ok

        Factored Out:
            Always fail if stuff in expected that is not in actual
    */
    fn check_value_or_datumschema(
        schema: &Option<BTreeMap<String, ValueOrDatumSchema>>,
        actual: &serde_json::Value,
        strict: bool,
        formatter: &impl Fn(&str, &str) -> String,
    ) -> Vec<Validated<(), String>> {
        if !actual.is_object() {
            return vec![Validated::fail(formatter("object type", "different type"))];
        }

        let vals = actual.as_object().unwrap();
        schema
            .as_ref()
            .map(|bt| {
                let mut in_expected_and_not_in_actual = bt
                    .iter()
                    .filter(|(k, _)| !vals.contains_key(k.as_str()))
                    .map(|(k, _)| {
                        Validated::fail(formatter(
                            format!(r#"member "{k}""#).as_str(),
                            format!(r#"object with "{k}" missing"#).as_str(),
                        ))
                    })
                    .collect::<Vec<Validated<(), String>>>();

                let mut actual_validations = vals
                    .iter()
                    .flat_map(|(k, value)| {
                        bt.get(k)
                            .map(|value_or_datum| match value_or_datum {
                                ValueOrDatumSchema::Datum(datum) => {
                                    datum.check(value, strict, formatter)
                                }
                                ValueOrDatumSchema::Values(expected) => {
                                    Self::check_value(expected, value, strict, formatter)
                                }
                            })
                            .unwrap_or_else(|| {
                                if strict {
                                    vec![Validated::fail(formatter(
                                        format!(r#"member "{k}""#).as_str(),
                                        format!(r#"object with "{k}" missing"#).as_str(),
                                    ))]
                                } else {
                                    vec![Good(())]
                                }
                            })
                    })
                    .collect::<Vec<Validated<(), String>>>();

                in_expected_and_not_in_actual.append(actual_validations.as_mut());
                in_expected_and_not_in_actual
            })
            .unwrap_or(vec![Good(())])
    }

    fn check_value(
        expected: &serde_json::Value,
        actual: &serde_json::Value,
        strict: bool,
        formatter: &impl Fn(&str, &str) -> String,
    ) -> Vec<Validated<(), String>> {
        let compare_mode = if strict {
            assert_json_diff::CompareMode::Strict
        } else {
            assert_json_diff::CompareMode::Inclusive
        };

        let result = assert_json_diff::assert_json_matches_no_panic(
            &actual,
            &expected,
            assert_json_diff::Config::new(compare_mode),
        );
        match result {
            Ok(_) => vec![Good(())],
            Err(msg) => vec![Validated::fail(formatter(
                format!("{:?}", expected).as_str(),
                format!("{:?} ; {msg}", actual).as_str(),
            ))],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct UnvalidatedRequest {
    pub method: Option<http::Verb>,
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Vec<http::Parameter>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<Vec<http::Header>>,
    //Requests can only contain a body OR a body_schema
    //We used to signify this using (serde-flattened)enums, but its
    //easier to manage validation errors if we flatten the
    //structure manually in this manner and leave the enums only
    //in the (Validated)RequestDescriptor struct
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<UnvalidatedVariableNameOrValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body_schema: Option<UnvalidatedVariableNameOrDatumSchema>,
}

impl Default for UnvalidatedRequest {
    fn default() -> Self {
        Self {
            method: None,
            url: "".to_string(),
            params: None,
            headers: None,
            body: None,
            body_schema: None,
        }
    }
}

impl Hash for UnvalidatedRequest {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.method.hash(state);
        self.url.hash(state);
        self.params.hash(state);
        self.headers.hash(state);
        serde_json::to_string(&self.body_schema)
            .unwrap()
            .hash(state);
        serde_json::to_string(&self.body).unwrap().hash(state);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct UnvalidatedCompareRequest {
    pub method: Option<http::Verb>,
    pub url: String,
    pub params: Option<Vec<http::Parameter>>,
    pub add_params: Option<Vec<http::Parameter>>,
    pub ignore_params: Option<Vec<String>>,
    pub headers: Option<Vec<http::Header>>,
    pub add_headers: Option<Vec<http::Header>>,
    pub ignore_headers: Option<Vec<String>>,
    //Requests can only contain a body OR a body_schema
    //We used to signify this using (serde-flattened)enums, but its
    //easier to manage validation errors if we flatten the
    //structure manually in this manner and leave the enums only
    //in the (Validated)CompareDescriptor struct
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body_schema: Option<DatumSchema>,
    pub strict: Option<bool>,
}

impl Hash for UnvalidatedCompareRequest {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.method.hash(state);
        self.url.hash(state);
        self.params.hash(state);
        serde_json::to_string(&self.body).unwrap().hash(state);
        self.body_schema.hash(state);
        self.add_params.hash(state);
        self.ignore_params.hash(state);
        self.headers.hash(state);
        self.add_headers.hash(state);
        self.ignore_headers.hash(state);
    }
}

#[derive(Hash, Debug, Serialize, Clone, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum ValueOrNumericSpecification<
    T: std::fmt::Display + std::fmt::Debug + std::cmp::PartialOrd + Serialize + Clone,
> {
    Value(T),
    Schema(NumericSpecification<T>),
}

impl<T> Checker for ValueOrNumericSpecification<T>
where
    T: PartialEq,
    T: Display,
    T: PartialOrd,
    T: fmt::Debug,
    T: Serialize,
    T: Clone,
{
    type Item = T;
    fn check(
        &self,
        val: &Self::Item,
        formatter: &impl Fn(&str, &str) -> String,
    ) -> Vec<Validated<(), String>> {
        match &self {
            ValueOrNumericSpecification::Value(t) => {
                if t == val {
                    vec![Good(())]
                } else {
                    vec![Validated::fail(formatter(
                        format!("{}", t).as_str(),
                        format!("{}", val).as_str(),
                    ))]
                }
            }
            ValueOrNumericSpecification::Schema(s) => s.check(val, formatter),
        }
    }
}

#[derive(Debug, Serialize, Clone, Deserialize, PartialEq)]
pub enum BodyOrSchema {
    #[serde(rename = "bodySchema")]
    Schema(DatumSchema),
    #[serde(rename = "body")]
    Body(serde_json::Value),
}

impl Hash for BodyOrSchema {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            BodyOrSchema::Schema(ds) => ds.hash(state),
            BodyOrSchema::Body(v) => serde_json::to_string(v).unwrap().hash(state),
        }
    }
}

pub type UnvalidatedVariableNameOrValue = UnvalidatedVariableNameOrComponent<serde_json::Value>;

pub type UnvalidatedVariableNameOrDatumSchema = UnvalidatedVariableNameOrComponent<DatumSchema>;

/**
    We expose variables to the user as things
    that are either:
        - Values
        - Datums
        - Files
    However, our implementation type also treats Secrets as
    obfuscated variables by leveraging SecretValue's.

    This requires us to use 2 different types for the implementation
    and the data file (jk::test::File) interface.
**/
#[derive(Debug, Serialize, Clone, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum ValueOrDatumOrFile {
    File { file: String },
    Value { value: Value },
    Schema(DatumSchema),
}

impl Hash for ValueOrDatumOrFile {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            ValueOrDatumOrFile::File { file } => file.hash(state),
            ValueOrDatumOrFile::Schema(s) => s.hash(state),
            ValueOrDatumOrFile::Value { value } => {
                serde_json::to_string(value).unwrap().hash(state)
            }
        }
    }
}

pub struct BodyOrSchemaChecker<'a> {
    pub value_or_schema: &'a BodyOrSchema,
    pub ignore_values: &'a [String],
    pub strict: bool,
}

impl<'a> BodyOrSchemaChecker<'a> {
    fn apply_ignored_values(
        &self,
        actual: &serde_json::Value,
        expected: &serde_json::Value,
    ) -> (serde_json::Value, serde_json::Value) {
        let mut modified_actual = actual.clone();
        let mut modified_expected = expected.clone();

        // TODO: make this more efficient, with a single pass filter
        for path in self.ignore_values.iter() {
            trace!("stripping path({}) from response", path);
            modified_actual = filter_json(path, 0, modified_actual).unwrap();
            modified_expected = filter_json(path, 0, modified_expected).unwrap();
        }

        (modified_actual, modified_expected)
    }

    fn apply_ignored_values_datum_schema(
        &self,
        actual: &serde_json::Value,
        schema: &DatumSchema,
    ) -> (serde_json::Value, DatumSchema) {
        //DatumSchema isomorphic to Value
        //But we want the "schema" member of DatumSchema::Object
        // \todo revisit to support nested ignore : foo.bar.car
        let mut datum_schema_as_value = serde_json::to_value(schema).unwrap();

        let (actual, expected_schema_val) = self.apply_ignored_values(
            actual,
            datum_schema_as_value
                .get("schema")
                .unwrap_or(&serde_json::json!(null)),
        );

        datum_schema_as_value
            .as_object_mut()
            .unwrap()
            .insert("schema".to_string(), expected_schema_val);
        (
            actual,
            serde_json::from_value(datum_schema_as_value).unwrap(),
        )
    }

    pub fn check_schema(
        &self,
        actual: &serde_json::Value,
        schema: &DatumSchema,
        strict: bool,
        formatter: &impl Fn(&str, &str) -> String,
    ) -> Result<Vec<Validated<(), String>>, Box<dyn Error + Send + Sync>> {
        trace!("validating response body using schema");
        //It doesn't make much sense *today* to modify
        //schema when applying ignored members, but
        //I can see future use cases that would break if we didn't

        let (modified_actual, modified_schema) =
            self.apply_ignored_values_datum_schema(actual, schema);

        trace!(
            "After modified, {:?} \n {:?}",
            modified_actual,
            modified_schema
        );

        Ok(modified_schema.check(&modified_actual, strict, formatter))
    }

    pub fn check_expected_value(
        &self,
        actual: &serde_json::Value,
        expected: &serde_json::Value,
        formatter: &impl Fn(&str, &str) -> String,
    ) -> Result<Vec<Validated<(), String>>, Box<dyn Error + Send + Sync>> {
        trace!("validating response body");
        let (modified_actual, modified_expected) = self.apply_ignored_values(actual, expected);

        trace!("compare json");
        let compare_mode = if self.strict {
            assert_json_diff::CompareMode::Strict
        } else {
            assert_json_diff::CompareMode::Inclusive
        };

        let result = assert_json_diff::assert_json_matches_no_panic(
            &modified_actual,
            &modified_expected,
            assert_json_diff::Config::new(compare_mode),
        );
        match result {
            Ok(_) => Ok(vec![Good(())]),
            Err(msg) => Ok(vec![Validated::fail(formatter(
                format!("body {modified_expected}").as_str(),
                format!("body {modified_actual} ; {msg}").as_str(),
            ))]),
        }
    }
}

impl<'a> Checker for BodyOrSchemaChecker<'a> {
    type Item = serde_json::Value;
    fn check(
        &self,
        val: &Self::Item,
        formatter: &impl Fn(&str, &str) -> String,
    ) -> Vec<Validated<(), String>> {
        let res = match self.value_or_schema {
            BodyOrSchema::Body(v) => {
                BodyOrSchemaChecker::check_expected_value(self, val, v, formatter)
            }
            BodyOrSchema::Schema(s) => {
                BodyOrSchemaChecker::check_schema(self, val, s, self.strict, formatter)
            }
        };

        match res {
            Ok(v) => v,
            Err(e) => vec![Validated::fail(format!(
                "Error encountered when comparing : {}",
                e
            ))],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct UnvalidatedResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<ValueOrNumericSpecification<u16>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<Vec<http::Header>>,
    //Responses can only contain a body OR a body_schema
    //We used to signify this using (serde-flattened)enums, but its
    //easier to manage validation errors if we flatten the
    //structure manually in this manner and leave the enums only
    //in the (Validated)ResponseDescriptor struct
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<UnvalidatedVariableNameOrValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body_schema: Option<UnvalidatedVariableNameOrDatumSchema>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ignore: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extract: Option<Vec<definition::ResponseExtraction>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strict: Option<bool>,
}

impl Hash for UnvalidatedResponse {
    fn hash<H: Hasher>(&self, state: &mut H) {
        serde_json::to_string(&self.body).unwrap().hash(state);
        self.status.hash(state);
        self.headers.hash(state);
        serde_json::to_string(&self.body_schema)
            .unwrap()
            .hash(state);
        self.ignore.hash(state);
        self.extract.hash(state);
        self.strict.hash(state);
    }
}

impl Default for UnvalidatedResponse {
    fn default() -> Self {
        Self {
            status: Some(ValueOrNumericSpecification::Value(200)),
            headers: None,
            body: None,
            ignore: None,
            extract: None,
            strict: None,
            body_schema: None,
        }
    }
}

#[derive(Hash, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnvalidatedVariable {
    pub name: String,
    #[serde(flatten)]
    pub value: ValueOrDatumOrFile,
}

#[derive(Hash, Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
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

#[derive(Hash, Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UnvalidatedRequestResponse {
    pub request: UnvalidatedRequest,
    pub response: Option<UnvalidatedResponse>,
}

#[derive(Hash, Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
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
            trace!("File is {:?}", file);
            file.filename = String::from(filename);
            Ok(file)
        }
        Err(e) => {
            error!("unable to parse file ({}) data: {}", filename, e);
            Err(Box::from(e))
        }
    }
}

pub fn generate_number<T>(spec: &NumericSpecification<T>, max_attempts: u16) -> Option<T>
where
    T: num::Num
        + num::Bounded
        + Copy
        + rand::distributions::uniform::SampleUniform
        + std::cmp::PartialOrd
        + std::default::Default
        + Clone
        + PartialEq
        + Display
        + PartialOrd
        + fmt::Debug,
    Specification<T>: Checker<Item = T>,
{
    let mut rng = rand::thread_rng();
    (0..max_attempts)
        .map(|_| {
            spec.specification
                .as_ref()
                .and_then(|s| s.generate_if_constrained(&mut rng))
                .unwrap_or(generate_number_in_range(
                    spec.min.unwrap_or(T::min_value()),
                    spec.max.unwrap_or(T::max_value()),
                    &mut rng,
                ))
        })
        .find(|v| {
            spec.check(v, &|_e, _a| "".to_string())
                .into_iter()
                .collect::<Validated<Vec<()>, String>>()
                .is_good()
        })
}

pub fn generate_bool(spec: &BooleanSpecification, max_attempts: u16) -> Option<bool> {
    let mut rng = rand::thread_rng();

    for _ in 0..max_attempts {
        let ret = spec
            .generate_if_constrained(&mut rng)
            .unwrap_or(generate_number_in_range(0, 100, &mut rng) % 2 == 0);

        let r = spec.check(&ret, &|_e, _a| "".to_string());
        if r.into_iter()
            .collect::<Validated<Vec<()>, String>>()
            .is_good()
        {
            return Some(ret);
        }
    }

    None
}

pub fn generate_float(spec: &FloatSpecification, max_attempts: u16) -> Option<f64> {
    generate_number::<f64>(spec, max_attempts)
}

pub fn generate_string(spec: &StringSpecification, max_attempts: u16) -> Option<String> {
    trace!("generate_string()");
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ\
                            abcdefghijklmnopqrstuvwxyz\
                            "; // 0123456789)(*&^%$#@!~";

    let mut rng = rand::thread_rng();
    let string_length: usize = rng.gen_range(1..50);

    for _ in 0..max_attempts {
        let ret = spec
            .specification
            .as_ref()
            .and_then(|s| s.generate_if_constrained(&mut rng))
            .unwrap_or(
                (0..string_length)
                    .map(|_| {
                        let idx = rng.gen_range(0..CHARSET.len());
                        CHARSET[idx] as char
                    })
                    .collect::<String>(),
            );
        let r = spec.check(&ret, &|_e, _a| "".to_string());
        if r.into_iter()
            .collect::<Validated<Vec<()>, String>>()
            .is_good()
        {
            return Some(ret);
        }
    }

    None
}

fn generate_number_in_range<T: Num + SampleUniform + PartialOrd + Copy + Debug>(
    min: T,
    max: T,
    rng: &mut ThreadRng,
) -> T {
    if min >= max {
        min
    } else {
        rng.gen_range(min..=max)
    }
}

pub fn generate_date(spec: &DateSpecification, max_attempts: u16) -> Option<String> {
    let min = spec
        .min
        .as_ref()
        .and_then(|date_str| spec.str_to_time(date_str.as_str()).ok())
        .unwrap_or(
            DateTime::<Local>::default()
                .with_day(1)
                .unwrap()
                .with_month(1)
                .unwrap(),
        );

    let max = spec
        .max
        .as_ref()
        .and_then(|date_str| spec.str_to_time(date_str.as_str()).ok())
        .unwrap_or(Local::now());

    let year_range = (min.year(), max.year());
    let month_range = (max.month(), min.month());
    let day_range = (max.day(), min.day());

    let mut rng = rand::thread_rng();

    (0..max_attempts)
        .map(|_| {
            spec.specification
                .as_ref()
                //issue here is "generate_if_constrained" can't be used indiscriminately ; it doesn't apply modifier unless we do it at
                //parse time. Would require Unvalidated version of type. So we have to match
                .and_then(|s| match s {
                    Specification::Value(v) => spec.get(v).ok(),
                    Specification::UnTaggedValue(v) => spec.get(v).ok(),
                    _ => s.generate_if_constrained(&mut rng),
                })
                .unwrap_or_else(|| {
                    let year = generate_number_in_range(year_range.0, year_range.1, &mut rng);

                    let month = if year == year_range.1 {
                        generate_number_in_range(month_range.0, month_range.1, &mut rng)
                    } else {
                        generate_number_in_range(1, 12, &mut rng)
                    };

                    let day = if month == month_range.1 && year == year_range.1 {
                        generate_number_in_range(day_range.0, day_range.1, &mut rng)
                    } else {
                        generate_number_in_range(day_range.0, 28, &mut rng)
                    };

                    chrono::NaiveDate::default()
                        .with_year(year)
                        .and_then(|d| d.with_month(month))
                        .and_then(|d| d.with_day(day))
                        .map(|d| Local.from_local_datetime(&d.and_hms_opt(0, 0, 0).unwrap()))
                        .map(|d| spec.time_to_str(&d.unwrap()))
                        .unwrap_or_default()
                })
        })
        .find(|date_str| {
            spec.check(date_str, &|_e, _a| "".to_string())
                .into_iter()
                .collect::<Validated<Vec<()>, String>>()
                .is_good()
        })
}

pub fn generate_datetime(spec: &DateTimeSpecification, max_attempts: u16) -> Option<String> {
    let min = spec
        .min
        .as_ref()
        .and_then(|date_str| spec.str_to_time(date_str.as_str()).ok())
        .unwrap_or(
            DateTime::<Local>::default()
                .with_day(1)
                .unwrap()
                .with_month(1)
                .unwrap(),
        );

    let max = spec
        .max
        .as_ref()
        .and_then(|date_str| spec.str_to_time(date_str.as_str()).ok())
        .unwrap_or(Local::now());

    let year_range = (min.year(), max.year());
    let month_range = (max.month(), min.month());
    let day_range = (max.day(), min.day());

    let mut rng = rand::thread_rng();

    (0..max_attempts)
        .map(|_| {
            spec.specification
                .as_ref()
                //issue here is "generate_if_constrained" can't be used indiscriminately ; it doesn't apply modifier unless we do it at
                //parse time. Would require Unvalidated version of type. So we have to match
                .and_then(|s| match s {
                    Specification::Value(v) => spec.get(v).ok(),
                    Specification::UnTaggedValue(v) => spec.get(v).ok(),
                    _ => s.generate_if_constrained(&mut rng),
                })
                .unwrap_or_else(|| {
                    let year = generate_number_in_range(year_range.0, year_range.1, &mut rng);

                    let month = if year == year_range.1 {
                        generate_number_in_range(month_range.0, month_range.1, &mut rng)
                    } else {
                        generate_number_in_range(1, 12, &mut rng)
                    };

                    let day = if month == month_range.1 && year == year_range.1 {
                        generate_number_in_range(day_range.0, day_range.1, &mut rng)
                    } else {
                        generate_number_in_range(day_range.0, 28, &mut rng)
                    };

                    chrono::NaiveDateTime::default()
                        .with_year(year)
                        .and_then(|d| d.with_month(month))
                        .and_then(|d| d.with_day(day))
                        .map(|d| Local.from_local_datetime(&d))
                        .map(|d| spec.time_to_str(&d.unwrap()))
                        .unwrap_or_default()
                })
        })
        .find(|date_str| {
            spec.check(date_str, &|_e, _a| "".to_string())
                .into_iter()
                .collect::<Validated<Vec<()>, String>>()
                .is_good()
        })
}

pub fn generate_name(spec: &NameSpecification, max_attempts: u16) -> Option<String> {
    let name_rng = RNG::from(&Language::Fantasy);
    let mut rng = rand::thread_rng();
    (0..max_attempts)
        .map(|_| {
            spec.specification
                .specification
                .as_ref()
                .and_then(|s| s.generate_if_constrained(&mut rng))
                .unwrap_or_else(|| name_rng.generate_name())
        })
        .find(|v| {
            spec.specification
                .check(v, &|_e, _a| "".to_string())
                .into_iter()
                .collect::<Validated<Vec<()>, String>>()
                .is_good()
        })
}

pub fn generate_email(spec: &EmailSpecification, max_attempts: u16) -> Option<String> {
    generate_string(&spec.specification, max_attempts)
        .map(|ran_string| format!("{}@gmail.com", ran_string))
}

pub fn generate_list(spec: &SequenceSpecification, max_attempts: u16) -> Option<Value> {
    // what ever shall I generate??? -> Went with ints for now
    // you should instead have a list of generators you random access into
    trace!("generate_list({:?})", spec);
    let mut rng = rand::thread_rng();
    let min_length = spec.min_length.unwrap_or(generate_number_in_range(
        0,
        spec.max_length.unwrap_or(100),
        &mut rng,
    ));
    let max_length = spec.max_length.unwrap_or(generate_number_in_range(
        min_length + 1,
        min_length * 2,
        &mut rng,
    ));
    (0..max_attempts)
        .map(|_| {
            spec.schema
                .as_ref()
                .map(|s| match s {
                    ValuesOrSchema::Schemas(_) => (min_length..max_length)
                        .map(|_| serde_json::Value::from(s.generate_if_constrained(&mut rng)))
                        .collect::<Vec<Value>>(),
                    ValuesOrSchema::Values(v) => {
                        v.generate_if_constrained(&mut rng).unwrap_or_default()
                    }
                })
                .unwrap_or_else(|| {
                    (min_length..max_length)
                        .map(|_| serde_json::Value::from(rng.gen::<i64>()))
                        .collect()
                })
        })
        .find(|v| {
            let ret = spec
                .check(v, false, &|_e, _a| "".to_string())
                .into_iter()
                .collect::<Validated<Vec<()>, String>>();
            ret.is_good()
        })
        .map(Value::from)
}

pub fn generate_value_from_schema(
    schema: &DatumSchema,
    max_attempts: u16,
) -> Option<serde_json::Value> {
    trace!("generate_value_from_schema({:?})", schema);
    return match schema {
        DatumSchema::Boolean { specification } => generate_bool(
            specification
                .as_ref()
                .unwrap_or(&BooleanSpecification::default()),
            max_attempts,
        )
        .map(serde_json::Value::from),
        DatumSchema::Float { specification } => generate_float(
            specification
                .as_ref()
                .unwrap_or(&FloatSpecification::default()),
            max_attempts,
        )
        .map(serde_json::Value::from),
        DatumSchema::Int { specification } => generate_number(
            specification
                .as_ref()
                .unwrap_or(&NumericSpecification::<i64>::default()),
            max_attempts,
        )
        .map(serde_json::Value::from),
        DatumSchema::String { specification } => generate_string(
            specification
                .as_ref()
                .unwrap_or(&StringSpecification::default()),
            max_attempts,
        )
        .map(serde_json::Value::from),
        DatumSchema::Date {
            specification: date,
        } => generate_date(
            date.as_ref().unwrap_or(&DateSpecification::default()),
            max_attempts,
        )
        .map(serde_json::Value::from),
        DatumSchema::DateTime {
            specification: date,
        } => generate_datetime(
            date.as_ref().unwrap_or(&DateTimeSpecification::default()),
            max_attempts,
        )
        .map(serde_json::Value::from),
        DatumSchema::Name {
            specification: name,
        } => generate_name(
            name.as_ref().unwrap_or(&NameSpecification::default()),
            max_attempts,
        )
        .map(serde_json::Value::from),
        DatumSchema::Email {
            specification: email,
        } => generate_email(
            email.as_ref().unwrap_or(&EmailSpecification::default()),
            max_attempts,
        )
        .map(serde_json::Value::from),
        DatumSchema::List { specification } => generate_list(
            specification
                .as_ref()
                .unwrap_or(&SequenceSpecification::default()),
            max_attempts,
        )
        .map(serde_json::Value::from),
        DatumSchema::Object { schema } => {
            let f = schema
                .as_ref()
                .map(|s| {
                    s.iter()
                        .filter_map(|(k, value_or_datumschema)| {
                            let ret = match value_or_datumschema {
                                ValueOrDatumSchema::Datum(datum) => {
                                    generate_value_from_schema(datum, max_attempts)
                                }
                                ValueOrDatumSchema::Values(v) => Some(v.clone()),
                            };
                            ret.as_ref()?;

                            Some((k.clone(), ret.unwrap()))
                        })
                        .collect::<Map<String, serde_json::Value>>()
                })
                .unwrap_or_default();
            Some(serde_json::Value::Object(f))
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn specification_none_of_checker() {
        let spec = Specification::NoneOf(vec![1, 2, 3, 4, 5]);

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
    fn specification_val_checker() {
        let spec = Specification::Value(12);

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
    fn specification_one_of_checker() {
        let spec = Specification::OneOf(vec![1, 2, 3, 4, 5]);

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
    fn numeric_specification_min_checker() {
        let spec = NumericSpecification::<u16> {
            min: Some(50),
            ..Default::default()
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
    fn numeric_specification_max_checker() {
        let spec = NumericSpecification::<u16> {
            max: Some(50),
            ..Default::default()
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
    fn string_specification_max_length_checker() {
        let spec = StringSpecification {
            max_length: Some(5),
            ..Default::default()
        };

        assert_eq!(
            true,
            spec.check(&"hello".to_string(), &|_e, _a| "".to_string())
                .into_iter()
                .collect::<Validated<Vec<()>, String>>()
                .is_good()
        );

        assert_eq!(
            true,
            spec.check(&"hell".to_string(), &|_e, _a| "".to_string())
                .into_iter()
                .collect::<Validated<Vec<()>, String>>()
                .is_good()
        );

        assert_eq!(
            true,
            spec.check(&"nooooooo".to_string(), &|_e, _a| "".to_string())
                .into_iter()
                .collect::<Validated<Vec<()>, String>>()
                .is_fail()
        );
    }

    #[test]
    fn string_specification_min_length_checker() {
        let spec = StringSpecification {
            min_length: Some(5),
            ..Default::default()
        };

        assert_eq!(
            true,
            spec.check(&"hello".to_string(), &|_e, _a| "".to_string())
                .into_iter()
                .collect::<Validated<Vec<()>, String>>()
                .is_good()
        );

        assert_eq!(
            true,
            spec.check(&"hellooooo".to_string(), &|_e, _a| "".to_string())
                .into_iter()
                .collect::<Validated<Vec<()>, String>>()
                .is_good()
        );

        assert_eq!(
            true,
            spec.check(&"no".to_string(), &|_e, _a| "".to_string())
                .into_iter()
                .collect::<Validated<Vec<()>, String>>()
                .is_fail()
        );
    }

    #[test]
    fn string_pattern_checker() {
        let spec = StringSpecification {
            pattern: Some(r"^helloworld(!)+$".to_string()),
            ..Default::default()
        };

        assert_eq!(
            true,
            spec.check(&"helloworld".to_string(), &|_e, _a| "".to_string())
                .into_iter()
                .collect::<Validated<Vec<()>, String>>()
                .is_fail()
        );

        assert_eq!(
            true,
            spec.check(&"helloworld!".to_string(), &|_e, _a| "".to_string())
                .into_iter()
                .collect::<Validated<Vec<()>, String>>()
                .is_good()
        );

        assert_eq!(
            true,
            spec.check(&"helloworld!!!!!!".to_string(), &|_e, _a| "".to_string())
                .into_iter()
                .collect::<Validated<Vec<()>, String>>()
                .is_good()
        );
    }

    #[test]
    fn numericspecification_no_inputs() {
        assert!(IntegerSpecification::new(None, None, None).is_ok());
    }

    #[test]
    fn numericspecification_min_max_agreemnt() {
        assert!(IntegerSpecification::new(None, Some(12), Some(12)).is_ok());
        assert!(IntegerSpecification::new(None, Some(12), Some(24)).is_ok());
        assert_eq!(
            IntegerSpecification::new(None, Some(24), Some(12)).unwrap_err(),
            "min must be less than or equal to max".to_string()
        );
    }

    #[test]
    fn stringspecification_no_inputs() {
        assert!(StringSpecification::new(None, None, None, None).is_ok());
    }

    #[test]
    fn stringspecification_negative_input() {
        assert_eq!(
            StringSpecification::new(None, Some(-12), None, None).unwrap_err(),
            "negative value provided for min".to_string()
        );
        assert_eq!(
            StringSpecification::new(None, Some(-12), Some(-12), None).unwrap_err(),
            "negative value provided for max,negative value provided for min".to_string()
        );
        assert_eq!(
            StringSpecification::new(None, None, Some(-24), None).unwrap_err(),
            "negative value provided for max".to_string()
        );
        assert!(StringSpecification::new(None, Some(12), None, None).is_ok());
    }

    #[test]
    fn stringspecification_min_max_agreement() {
        assert_eq!(
            StringSpecification::new(None, Some(24), Some(12), None).unwrap_err(),
            "min_length must be less than or equal to max_length".to_string()
        );
        assert!(StringSpecification::new(None, Some(1), Some(1), None).is_ok());
        assert!(StringSpecification::new(None, Some(12), Some(24), None).is_ok());
    }

    #[test]
    fn string_specification_simple_pattern() {
        assert!(StringSpecification::new(None, None, None, Some("simple".to_string())).is_ok());
    }

    #[test]
    fn string_specification_invalid_pattern() {
        assert!(StringSpecification::new(None, None, None, Some("?simple?".to_string())).is_err());
    }

    #[test]
    fn string_specification_complex_pattern() {
        assert!(StringSpecification::new(None, None, None, Some("^simple?".to_string())).is_ok());
    }

    #[test]
    fn datespecification_valid_inputs() {
        assert!(DateSpecification::new(None, None, None, None, None).is_ok());
    }

    #[test]
    fn datespecification_invalid_input() {
        let res = DateSpecification::new(
            None,
            Some("Hello!".to_string()),
            Some("2020-09-12".to_string()),
            None,
            None,
        );

        assert!(res.is_err());
        assert!(res.err().unwrap().as_str().contains("min"));
    }

    #[test]
    fn datespecification_invalid_inputs() {
        let res = DateSpecification::new(
            None,
            Some("Hello!".to_string()),
            Some("Hello".to_string()),
            None,
            None,
        );

        assert!(res.is_err());
        assert!(res.as_ref().err().unwrap().as_str().contains("max"));
        assert!(res.as_ref().err().unwrap().as_str().contains("min"));
    }

    #[test]
    fn datum_float_type_validation() {
        assert_eq!(
            true,
            DatumSchema::Float {
                specification: None,
            }
            .check(&serde_json::json!({}), true, &|_e, _a| "".to_string())
            .into_iter()
            .collect::<Validated<Vec<()>, String>>()
            .is_fail(),
        );

        assert_eq!(
            false,
            DatumSchema::Float {
                specification: None,
            }
            .check(&serde_json::json!(4.53), true, &|_e, _a| "".to_string())
            .into_iter()
            .collect::<Validated<Vec<()>, String>>()
            .is_fail(),
        );
    }

    #[test]
    fn datum_date_type_validation() {
        assert_eq!(
            true,
            DatumSchema::Date {
                specification: None
            }
            .check(&serde_json::json!({}), true, &|_e, _a| "".to_string())
            .into_iter()
            .collect::<Validated<Vec<()>, String>>()
            .is_fail(),
        );

        assert_eq!(
            false,
            DatumSchema::Date {
                specification: None
            }
            .check(&serde_json::json!("2024-12-08"), true, &|_e, _a| ""
                .to_string())
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
            .check(&serde_json::json!({}), true, &|_e, _a| "".to_string())
            .into_iter()
            .collect::<Validated<Vec<()>, String>>()
            .is_fail(),
        );

        assert_eq!(
            false,
            DatumSchema::Int {
                specification: None,
            }
            .check(&serde_json::json!(4), true, &|_e, _a| "".to_string())
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
            .check(&serde_json::json!({}), true, &|_e, _a| "".to_string())
            .into_iter()
            .collect::<Validated<Vec<()>, String>>()
            .is_fail(),
        );

        assert_eq!(
            false,
            DatumSchema::String {
                specification: None,
            }
            .check(&serde_json::json!("hello"), true, &|_e, _a| "".to_string())
            .into_iter()
            .collect::<Validated<Vec<()>, String>>()
            .is_fail(),
        );
    }

    #[test]
    fn datum_list_type_validation() {
        assert_eq!(
            true,
            DatumSchema::List {
                specification: None
            }
            .check(&serde_json::json!({}), true, &|_e, _a| "".to_string())
            .into_iter()
            .collect::<Validated<Vec<()>, String>>()
            .is_fail(),
        );

        assert_eq!(
            false,
            DatumSchema::List {
                specification: None
            }
            .check(&serde_json::json!([]), true, &|_e, _a| "".to_string())
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
                .check(&serde_json::json!({}), true, &|_e, _a| "".to_string())
                .into_iter()
                .collect::<Validated<Vec<()>, String>>()
                .is_fail(),
        );

        assert_eq!(
            true,
            DatumSchema::Object { schema: None }
                .check(&serde_json::json!([]), true, &|_e, _a| "".to_string())
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
                    ValueOrDatumSchema::Datum(DatumSchema::String {
                        specification: Some(StringSpecification {
                            specification: Some(Specification::OneOf(vec![
                                "foo".to_string(),
                                "bar".to_string(),
                            ])),
                            ..Default::default()
                        }),
                    }),
                ),
                (
                    "cars".to_string(),
                    ValueOrDatumSchema::Datum(DatumSchema::List {
                        specification: Some(SequenceSpecification {
                            schema: Some(ValuesOrSchema::Schemas(Specification::Value(Box::new(
                                DatumSchema::String {
                                    specification: None,
                                },
                            )))),
                            ..Default::default()
                        }),
                    }),
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
                    true,
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
                    true,
                    &|_e, _a| "".to_string()
                )
                .into_iter()
                .collect::<Validated<Vec<()>, String>>()
                .is_fail(),
        );
    }

    #[test]
    fn datum_object_member_validation_missing_expected_member_detected_strict() {
        let datum = construct_datum_schema_object();

        assert_eq!(
            true,
            datum
                .check(
                    &serde_json::json!({
                        "name" : "foo"
                    }),
                    true,
                    &|_e, _a| "".to_string()
                )
                .into_iter()
                .collect::<Validated<Vec<()>, String>>()
                .is_fail(),
        );
    }

    #[test]
    fn datum_object_member_validation_missing_expected_member_detected_non_strict() {
        let datum = construct_datum_schema_object();

        assert_eq!(
            true,
            datum
                .check(
                    &serde_json::json!({
                        "name" : "foo"
                    }),
                    false,
                    &|_e, _a| "".to_string()
                )
                .into_iter()
                .collect::<Validated<Vec<()>, String>>()
                .is_fail(),
        );
    }

    #[test]
    fn datum_object_member_validation_missing_actual_member_detected_strict() {
        let datum = construct_datum_schema_object();

        assert_eq!(
            true,
            datum
                .check(
                    &serde_json::json!({
                        "name" : "foo",
                        "cars": ["audi", "mercedes", "bmw"],
                        "airlines" : ["aa", "delta"]
                    }),
                    true,
                    &|_e, _a| "".to_string()
                )
                .into_iter()
                .collect::<Validated<Vec<()>, String>>()
                .is_fail(),
        );
    }

    #[test]
    fn datum_object_member_validation_missing_actual_member_detected_non_strict() {
        let datum = construct_datum_schema_object();

        assert_eq!(
            false,
            datum
                .check(
                    &serde_json::json!({
                        "name" : "foo",
                        "cars": ["audi", "mercedes", "bmw"],
                        "airlines" : ["aa", "delta"]
                    }),
                    false,
                    &|_e, _a| "".to_string()
                )
                .into_iter()
                .collect::<Validated<Vec<()>, String>>()
                .is_fail(),
        );
    }

    #[test]
    fn body_or_schema_checker_non_empty_ignores() {
        let body_or_schema = BodyOrSchema::Schema(construct_datum_schema_object());
        let ignores = vec!["cars".to_string()];
        let checker: BodyOrSchemaChecker = BodyOrSchemaChecker {
            ignore_values: &ignores,
            strict: true,
            value_or_schema: &body_or_schema,
        };
        assert_eq!(
            false,
            checker
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
    fn body_or_schema_checker_empty_ignores() {
        let body_or_schema = BodyOrSchema::Schema(construct_datum_schema_object());
        let ignores: Vec<String> = vec![];
        let checker: BodyOrSchemaChecker = BodyOrSchemaChecker {
            ignore_values: &ignores,
            strict: true,
            value_or_schema: &body_or_schema,
        };
        assert_eq!(
            true,
            checker
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
    fn number_generation() {
        let spec = NumericSpecification::<u16> {
            min: Some(1),
            max: Some(9),
            ..Default::default()
        };

        let num = generate_number(&spec, 10);

        assert!(num.is_some());
        assert!(spec
            .check(&num.unwrap(), &|_e, _a| "".to_string())
            .into_iter()
            .collect::<Validated<Vec<()>, String>>()
            .is_good());
    }

    #[test]
    fn string_generation() {
        let spec = StringSpecification {
            specification: Some(Specification::NoneOf(vec![
                "foo".to_string(),
                "bar".to_string(),
            ])),
            ..Default::default()
        };

        let val = generate_string(&spec, 10);

        assert!(val.is_some());
        assert!(spec
            .check(&val.unwrap(), &|_e, _a| "".to_string())
            .into_iter()
            .collect::<Validated<Vec<()>, String>>()
            .is_good());
    }

    #[test]
    fn object_generation() {
        let schema = construct_datum_schema_object();
        let val = generate_value_from_schema(&schema, 10);
        assert!(val.is_some());
        assert!(schema
            .check(&val.unwrap(), true, &|_e, _a| "".to_string())
            .into_iter()
            .collect::<Validated<Vec<()>, String>>()
            .is_good());
    }

    #[test]
    fn number_generation_max_greater_than_min() {
        let mut rng = rand::thread_rng();
        let res = generate_number_in_range(10, 0, &mut rng);
        assert_eq!(10, res);
    }

    #[test]
    fn number_generation_range_of_1() {
        let mut rng = rand::thread_rng();
        let res = generate_number_in_range(10, 10, &mut rng);
        assert_eq!(10, res);
    }

    #[test]
    fn date_generation() {
        let spec = DateSpecification::default();
        let val = generate_date(&spec, 10);
        assert!(val.is_some());
        assert!(spec
            .check(&val.unwrap(), &|_e, _a| "".to_string())
            .into_iter()
            .collect::<Validated<Vec<()>, String>>()
            .is_good());
    }

    #[test]
    fn bool_generation() {
        let spec = BooleanSpecification::default();
        let val = generate_bool(&spec, 10);
        assert!(val.is_some());
        assert!(spec
            .check(&val.unwrap(), &|_e, _a| "".to_string())
            .into_iter()
            .collect::<Validated<Vec<()>, String>>()
            .is_good());
    }

    #[test]
    fn datetime_generation() {
        let spec = DateTimeSpecification::default();
        let val = generate_datetime(&spec, 10);
        assert!(val.is_some());
        assert!(spec
            .check(&val.unwrap(), &|_e, _a| "".to_string())
            .into_iter()
            .collect::<Validated<Vec<()>, String>>()
            .is_good());
    }

    #[test]
    fn name_generation() {
        let spec = NameSpecification::default();
        let val = generate_name(&spec, 10);
        assert!(val.is_some());
        assert!(spec
            .check(&val.unwrap(), &|_e, _a| "".to_string())
            .into_iter()
            .collect::<Validated<Vec<()>, String>>()
            .is_good());
    }

    #[test]
    fn email_generation() {
        let spec = EmailSpecification::default();
        let val = generate_email(&spec, 10);
        assert!(val.is_some());
        assert!(spec
            .check(&val.unwrap(), &|_e, _a| "".to_string())
            .into_iter()
            .collect::<Validated<Vec<()>, String>>()
            .is_good());
    }

    #[test]
    fn variable_name_deserialization_from_improper_string() {
        let res = serde_yaml::from_str::<VariableName>("not_a_variable");
        assert!(res.is_err());
    }

    #[test]
    fn variable_name_deserialization_from_proper_string() {
        let res = serde_yaml::from_str::<VariableName>("${my_var}");
        assert!(res.is_ok());
        assert_eq!("${my_var}", res.unwrap().val());
    }

    #[test]
    fn sequence_specification_min_length_more_than_actual() {
        assert!(SequenceSpecification {
            min_length: Some(5),
            ..Default::default()
        }
        .check_min_length(
            &vec![
                serde_json::Value::from(1),
                serde_json::Value::from(2),
                serde_json::Value::from(3),
                serde_json::Value::from(4)
            ],
            &|s, _| s.to_string()
        )
        .is_fail());
    }

    #[test]
    fn sequence_specification_min_length_equal_to_length() {
        assert!(SequenceSpecification {
            min_length: Some(5),
            ..Default::default()
        }
        .check_min_length(
            &vec![
                serde_json::Value::from(1),
                serde_json::Value::from(2),
                serde_json::Value::from(3),
                serde_json::Value::from(4),
                serde_json::Value::from(5)
            ],
            &|s, _| s.to_string()
        )
        .is_good());
    }

    #[test]
    fn sequence_specification_min_length_less_than_actual() {
        assert!(SequenceSpecification {
            min_length: Some(5),
            ..Default::default()
        }
        .check_min_length(
            &vec![
                serde_json::Value::from(1),
                serde_json::Value::from(2),
                serde_json::Value::from(3),
                serde_json::Value::from(4),
                serde_json::Value::from(5),
                serde_json::Value::from(6)
            ],
            &|s, _| s.to_string()
        )
        .is_good());
    }

    #[test]
    fn sequence_specification_min_check_no_bounds() {
        assert!(SequenceSpecification {
            ..Default::default()
        }
        .check_min_length(
            &vec![
                serde_json::Value::from(1),
                serde_json::Value::from(2),
                serde_json::Value::from(3),
                serde_json::Value::from(4),
                serde_json::Value::from(5)
            ],
            &|s, _| s.to_string()
        )
        .is_good());
    }

    #[test]
    fn sequence_specification_max_length_more_than_actual() {
        assert!(SequenceSpecification {
            max_length: Some(5),
            ..Default::default()
        }
        .check_max_length(
            &vec![
                serde_json::Value::from(1),
                serde_json::Value::from(2),
                serde_json::Value::from(3),
                serde_json::Value::from(4)
            ],
            &|s, _| s.to_string()
        )
        .is_good());
    }

    #[test]
    fn sequence_specification_max_length_equal_to_length() {
        assert!(SequenceSpecification {
            min_length: Some(5),
            ..Default::default()
        }
        .check_max_length(
            &vec![
                serde_json::Value::from(1),
                serde_json::Value::from(2),
                serde_json::Value::from(3),
                serde_json::Value::from(4),
                serde_json::Value::from(5)
            ],
            &|s, _| s.to_string()
        )
        .is_good());
    }

    #[test]
    fn sequence_specification_max_length_less_than_actual() {
        assert!(SequenceSpecification {
            max_length: Some(5),
            ..Default::default()
        }
        .check_max_length(
            &vec![
                serde_json::Value::from(1),
                serde_json::Value::from(2),
                serde_json::Value::from(3),
                serde_json::Value::from(4),
                serde_json::Value::from(5),
                serde_json::Value::from(6)
            ],
            &|s, _| s.to_string()
        )
        .is_fail());
    }

    #[test]
    fn sequence_specification_max_check_no_bounds() {
        assert!(SequenceSpecification {
            ..Default::default()
        }
        .check_max_length(
            &vec![
                serde_json::Value::from(1),
                serde_json::Value::from(2),
                serde_json::Value::from(3),
                serde_json::Value::from(4),
                serde_json::Value::from(5)
            ],
            &|s, _| s.to_string()
        )
        .is_good());
    }

    #[test]
    fn datum_specification_any_of_following_spec() {
        let spec = Specification::<Box<DatumSchema>>::AnyOf(vec![
            Box::from(DatumSchema::String {
                specification: None,
            }),
            Box::from(DatumSchema::Int {
                specification: None,
            }),
        ]);

        if let Specification::<Box<DatumSchema>>::AnyOf(v) = &spec {
            assert!(spec
                .schema_any_one_of(
                    &vec![serde_json::Value::from(1), serde_json::Value::from("hello")],
                    v,
                    true,
                    &|s, _| s.to_string(),
                )
                .is_good());
        }
    }

    #[test]
    fn datum_specification_any_of_not_following_spec() {
        let spec = Specification::<Box<DatumSchema>>::AnyOf(vec![
            Box::from(DatumSchema::String {
                specification: None,
            }),
            Box::from(DatumSchema::Int {
                specification: None,
            }),
        ]);

        if let Specification::<Box<DatumSchema>>::AnyOf(v) = &spec {
            assert!(spec
                .schema_any_one_of(
                    &vec![
                        serde_json::Value::from(1.25),
                        serde_json::Value::from("hello")
                    ],
                    v,
                    true,
                    &|s, _| s.to_string(),
                )
                .is_fail());
        }
    }

    #[test]
    fn datum_specification_one_of_not_following_spec() {
        let spec = Specification::<Box<DatumSchema>>::OneOf(vec![
            Box::from(DatumSchema::String {
                specification: None,
            }),
            Box::from(DatumSchema::Int {
                specification: None,
            }),
        ]);

        if let Specification::<Box<DatumSchema>>::OneOf(v) = &spec {
            assert!(spec
                .schema_check_one_of(
                    &vec![
                        serde_json::Value::from(1.25),
                        serde_json::Value::from("hello")
                    ],
                    v,
                    true,
                    &|s, _| s.to_string(),
                )
                .is_fail());
        }
    }

    #[test]
    fn datum_specification_one_of_following_spec() {
        let spec = Specification::<Box<DatumSchema>>::OneOf(vec![
            Box::from(DatumSchema::String {
                specification: None,
            }),
            Box::from(DatumSchema::Int {
                specification: None,
            }),
        ]);

        if let Specification::<Box<DatumSchema>>::OneOf(v) = &spec {
            assert!(spec
                .schema_check_one_of(
                    &vec![
                        serde_json::Value::from("world"),
                        serde_json::Value::from("hello")
                    ],
                    v,
                    true,
                    &|s, _| s.to_string(),
                )
                .is_good());
        }
    }

    #[test]
    fn datum_specification_none_of_not_following_spec() {
        let spec =
            Specification::<Box<DatumSchema>>::NoneOf(vec![Box::from(DatumSchema::String {
                specification: None,
            })]);

        if let Specification::<Box<DatumSchema>>::NoneOf(v) = &spec {
            assert!(spec
                .schema_check_none_of(
                    &vec![
                        serde_json::Value::from(1.25),
                        serde_json::Value::from("hello")
                    ],
                    v,
                    true,
                    &|s, _| s.to_string(),
                )
                .is_fail());
        }
    }

    #[test]
    fn datum_specification_none_of_following_spec() {
        let spec = Specification::<Box<DatumSchema>>::NoneOf(vec![Box::from(DatumSchema::Int {
            specification: None,
        })]);

        if let Specification::<Box<DatumSchema>>::NoneOf(v) = &spec {
            assert!(spec
                .schema_check_none_of(
                    &vec![
                        serde_json::Value::from("world"),
                        serde_json::Value::from("hello")
                    ],
                    v,
                    true,
                    &|s, _| s.to_string(),
                )
                .is_good());
        }
    }

    #[test]
    fn value_specification_none_of_not_following_spec() {
        let spec = Specification::<Vec<Value>>::NoneOf(vec![vec![serde_json::Value::from(1)]]);

        if let Specification::<Vec<Value>>::NoneOf(v) = &spec {
            assert!(spec
                .check_none_of(&vec![serde_json::Value::from(1)], v, &|s, _| s.to_string(),)
                .is_fail());
        }
    }

    #[test]
    fn value_specification_none_of_following_spec() {
        let spec = Specification::<Vec<Value>>::NoneOf(vec![vec![serde_json::Value::from(1)]]);

        if let Specification::<Vec<Value>>::NoneOf(v) = &spec {
            assert!(spec
                .check_none_of(
                    &vec![serde_json::Value::from(2), serde_json::Value::from(3)],
                    v,
                    &|s, _| s.to_string(),
                )
                .is_good());
        }
    }

    #[test]
    fn value_specification_any_of_following_spec() {
        let spec = Specification::<Vec<Value>>::AnyOf(vec![vec![serde_json::Value::from(1)]]);

        if let Specification::<Vec<Value>>::AnyOf(v) = &spec {
            assert!(spec
                .check_any_of(&vec![serde_json::Value::from(1)], v, &|s, _| s.to_string(),)
                .is_good());
        }
    }

    #[test]
    fn value_specification_any_of_not_following_spec() {
        let spec = Specification::<Vec<Value>>::AnyOf(vec![vec![serde_json::Value::from(1)]]);

        if let Specification::<Vec<Value>>::AnyOf(v) = &spec {
            assert!(spec
                .check_any_of(
                    &vec![serde_json::Value::from(2), serde_json::Value::from(3)],
                    v,
                    &|s, _| s.to_string(),
                )
                .is_fail());
        }
    }

    #[test]
    fn list_generation_sequence_specification_has_max() {
        let spec = SequenceSpecification {
            max_length: Some(5),
            ..Default::default()
        };
        assert!(generate_list(&spec, 1,).is_some());
    }

    #[test]
    fn list_generation_sequence_specification_has_min() {
        let spec = SequenceSpecification {
            min_length: Some(1),
            ..Default::default()
        };
        assert!(generate_list(&spec, 1,).is_some());
    }

    #[test]
    fn list_generation_default_sequence_specification() {
        let spec = SequenceSpecification::default();
        assert!(generate_list(&spec, 1,).is_some());
    }
}
