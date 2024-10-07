use crate::json::filter::filter_json;
use crate::test;
use crate::test::file::Validated::Good;
use crate::test::variable::Modifier;
use crate::test::{definition, http, variable};
use crate::validated::ValidatedExt;
use chrono::NaiveDate;
use chrono::TimeZone;
use chrono::{DateTime, ParseError};
use chrono::{Datelike, Local};
use chrono::{Days, Months};
use chrono::{Duration, NaiveDateTime};
use log::debug;
use log::error;
use log::trace;
use nonempty_collections::{IntoNonEmptyIterator, NonEmptyIterator};
use num::{Num, Signed};
use rand::distributions::uniform::SampleUniform;
use rand::rngs::ThreadRng;
use rand::Rng;
use regex::Regex;
use serde::de::Visitor;
use serde::Deserializer;
use serde::{Deserialize, Serialize};
use serde_json::Map;
use serde_json::Value;
use std::cmp::{max, min};
use std::collections::BTreeMap;
use std::error::Error;
use std::fmt::{self};
use std::fmt::{Debug, Display};
use std::fs;
use std::hash::{Hash, Hasher};
use validated::Validated;

const GIVEN_NAMES: [&str; 20] = [
    "James",
    "Michael",
    "Robert",
    "John",
    "David",
    "William",
    "Richard",
    "Joseph",
    "Thomas",
    "Christopher",
    "Mary",
    "Patricia",
    "Jennifer",
    "Linda",
    "Elizabeth",
    "Barbara",
    "Susan",
    "Jessica",
    "Karen",
    "Sarah",
];

const SURNAMES: [&str; 20] = [
    "Smith", "Johnson", "Williams", "Brown", "Jones", "Miller", "Davis", "Wilson", "Anderson",
    "Thomas", "Taylor", "Moore", "Jackson", "Martin", "Lee", "Thompson", "Harris", "Clark",
    "Lewis", "Robinson",
];

const EMAIL_DOMAINS: [&str; 3] = ["example.com", "example.net", "example.org"];

impl std::cmp::PartialEq<String> for VariableName {
    fn eq(&self, other: &String) -> bool {
        self.0 == *other
    }
}

#[derive(PartialEq, Eq, Debug, Serialize, Clone)]
pub struct VariableName(pub String);

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
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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

impl<T> TryFrom<UnvalidatedSpecification<T>> for Option<Specification<T>> {
    type Error = String;
    fn try_from(unvalidated: UnvalidatedSpecification<T>) -> Result<Self, Self::Error> {
        let specified = vec![
            &unvalidated.any_of,
            &unvalidated.one_of,
            &unvalidated.none_of,
        ]
        .into_iter()
        .filter(|o| o.is_some())
        .count();
        if specified > 1 || (specified == 1 && unvalidated.value.is_some()) {
            return Err(
                "can only specify one of the following constraints: oneOf, anyOf, noneOf, or value"
                    .to_string(),
            );
        }
        return match (
            unvalidated.value,
            unvalidated.any_of,
            unvalidated.one_of,
            unvalidated.none_of,
        ) {
            (Some(v), _, _, _) => Ok(Some(Specification::Value(v))),
            (_, Some(vs), _, _) => Ok(Some(Specification::AnyOf(vs))),
            (_, _, Some(vs), _) => Ok(Some(Specification::OneOf(vs))),
            (_, _, _, Some(vs)) => Ok(Some(Specification::NoneOf(vs))),
            _ => Ok(None),
        };
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
    pub length: Option<i64>,
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
    pub length: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_length: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_length: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema: Option<ValuesOrSchema>,
}

#[derive(Hash, Default, Serialize, Debug, Clone, Deserialize, PartialEq)]
pub struct DateSpecification {
    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    pub specification: Option<Specification<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modifier: Option<variable::Modifier>,
}

#[derive(Hash, Default, Serialize, Debug, Clone, Deserialize, PartialEq)]
pub struct DateTimeSpecification {
    #[serde(flatten)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub specification: Option<Specification<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
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

impl<T> Hash for UnvalidatedNumericSpecification<T>
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

fn validate1<T: Copy>(
    pred: &impl Fn(&T) -> bool,
    val: &Option<T>,
    message: String,
) -> Validated<Option<T>, String> {
    val.map(|v| {
        if !pred(&v) {
            Validated::fail(message)
        } else {
            Good(val.clone())
        }
    })
    .unwrap_or(Validated::Good(None))
}

fn validate2<T>(
    pred: &impl Fn(&T, &T) -> bool,
    val: &Option<T>,
    val2: &Option<T>,
    message: String,
) -> Validated<(), String> {
    val.as_ref()
        .zip(val2.as_ref())
        .map(|(v, v2)| {
            if !pred(v, v2) {
                Validated::fail(message)
            } else {
                Good(())
            }
        })
        .unwrap_or(Good(()))
}

fn non_negative_validator<T: Signed + Copy>(
    num: &Option<T>,
    variable_name: &str,
) -> Validated<Option<T>, String> {
    validate1::<T>(
        &|val| val.is_positive() || val.is_zero(),
        num,
        format!("negative value provided for {variable_name}"),
    )
}

fn less_than_or_equal_validator<T: PartialOrd>(
    lhs: &Option<T>,
    rhs: &Option<T>,
    lhs_variable_name: &str,
    rhs_variable_name: &str,
) -> Validated<(), String> {
    validate2::<T>(
        &|lhs, rhs| lhs <= rhs,
        lhs,
        rhs,
        format!("{lhs_variable_name} must be less than or equal to {rhs_variable_name}"),
    )
}

//Once new approach is vetted, we can make this a proper sum
//type
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
        let is_none_of = match specification.as_ref() {
            Some(Specification::NoneOf(_)) => true,
            _ => false,
        };
        let violation =
            specification.is_some() && !is_none_of && min.as_ref().or(max.as_ref()).is_some();
        if violation {
            return Err(
                "Cannot specify min or max alongside either of oneOf, anyOf, or value".to_string(),
            );
        }
        less_than_or_equal_validator(&min, &max, "min", "max")
            .map(|_| Self {
                specification,
                min,
                max,
            })
            .ok()
            .map_err(|nev| {
                nev.into_nonempty_iter()
                    .reduce(|acc, e| format!("{},{}", acc, e))
            })
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

impl<T> TryFrom<UnvalidatedNumericSpecification<T>> for NumericSpecification<T>
where
    T: PartialEq,
    T: Display,
    T: PartialOrd,
    T: fmt::Debug,
    T: Clone,
{
    type Error = String;

    fn try_from(
        unvalidated_numeric: UnvalidatedNumericSpecification<T>,
    ) -> Result<Self, Self::Error> {
        let unvalidated_spec = UnvalidatedSpecification::<T> {
            name: unvalidated_numeric.name,
            value: unvalidated_numeric.value,
            any_of: unvalidated_numeric.any_of,
            one_of: unvalidated_numeric.one_of,
            none_of: unvalidated_numeric.none_of,
        };

        let maybe_spec = TryInto::<Option<Specification<T>>>::try_into(unvalidated_spec);
        maybe_spec
            .and_then(|spec| Self::new(spec, unvalidated_numeric.min, unvalidated_numeric.max))
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
        length: Option<i64>,
        min_length: Option<i64>,
        max_length: Option<i64>,
        pattern: Option<String>,
    ) -> Result<Self, String> {
        let is_none_of = match specification.as_ref() {
            Some(Specification::NoneOf(_)) => true,
            _ => false,
        };
        let violation = specification.is_some()
            && !is_none_of
            && length
                .as_ref()
                .or(min_length.as_ref())
                .or(max_length.as_ref())
                .or(pattern.as_ref().map(|_| &25)) //random value to make iti64
                .is_some();

        if violation {
            return Err("Cannot specify minLength, maxLength, or pattern alongside either of oneOf, anyOf, or value".to_string());
        }

        let negative_validation_length = non_negative_validator(&length, "length");
        let negative_validation_max = non_negative_validator(&max_length, "maxLength");
        let negative_validation_min = non_negative_validator(&min_length, "minLength");
        let relation_validation =
            less_than_or_equal_validator(&min_length, &max_length, "minLength", "maxLength");

        let length_not_combined_with_min_or_max_validation =
            if length.and(min_length).is_some() || length.and(max_length).is_some() {
                Validated::fail(
                    "length cannot be specified alongside minLength or maxLength".to_string(),
                )
            } else {
                Good(())
            };

        let pattern_validation = pattern
            .map(|p| {
                Regex::new(p.as_str())
                    .map(|_| Good(Some(p)))
                    .unwrap_or_else(|e| {
                        Validated::fail(format!("invalid regex supplied for pattern: {}", e))
                    })
            })
            .unwrap_or(Good(None));

        negative_validation_max
            .map5(
                length_not_combined_with_min_or_max_validation
                    .map2(negative_validation_length, |_, _| length),
                negative_validation_min,
                relation_validation,
                pattern_validation,
                |_, _, _, _, p| Self {
                    length,
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

    fn check_length(
        &self,
        actual: &str,
        formatter: &impl Fn(&str, &str) -> String,
    ) -> Validated<(), String> {
        match &self.length {
            Some(t) => {
                if *t == actual.len() as i64 {
                    Good(())
                } else {
                    Validated::fail(formatter(format!("length of {}", t).as_str(), actual))
                }
            }
            None => Good(()),
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

impl TryFrom<UnvalidatedStringSpecification> for StringSpecification {
    type Error = String;

    fn try_from(unvalidated_string: UnvalidatedStringSpecification) -> Result<Self, Self::Error> {
        let unvalidated_spec = UnvalidatedSpecification::<String> {
            name: unvalidated_string.name,
            value: unvalidated_string.value,
            any_of: unvalidated_string.any_of,
            one_of: unvalidated_string.one_of,
            none_of: unvalidated_string.none_of,
        };

        let maybe_spec = TryInto::<Option<Specification<String>>>::try_into(unvalidated_spec);
        maybe_spec.and_then(|spec| {
            Self::new(
                spec,
                unvalidated_string.length,
                unvalidated_string.min_length,
                unvalidated_string.max_length,
                unvalidated_string.pattern,
            )
        })
    }
}

impl SequenceSpecification {
    pub fn new(
        schema: Option<ValuesOrSchema>,
        length: Option<i64>,
        min_length: Option<i64>,
        max_length: Option<i64>,
    ) -> Result<Self, String> {
        let negative_validation_length = non_negative_validator(&length, "length");
        let negative_validation_max = non_negative_validator(&max_length, "maxLength");
        let negative_validation_min = non_negative_validator(&min_length, "minLength");
        let relation_validation =
            less_than_or_equal_validator(&min_length, &max_length, "minLength", "maxLength");

        let length_not_combined_with_min_or_max_validation =
            if length.and(min_length).is_some() || length.and(max_length).is_some() {
                Validated::fail(
                    "length cannot be specified alongside minLength or maxLength".to_string(),
                )
            } else {
                Good(())
            };

        negative_validation_length
            .map5(
                length_not_combined_with_min_or_max_validation,
                negative_validation_max,
                negative_validation_min,
                relation_validation,
                |_, _, _, _, _| Self {
                    length,
                    max_length,
                    min_length,
                    schema,
                },
            )
            .ok()
            .map_err(|nev| {
                nev.into_nonempty_iter()
                    .reduce(|acc, e| format!("{},{}", acc, e))
            })
    }

    fn check_length(
        &self,
        actual: &Vec<Value>,
        formatter: &impl Fn(&str, &str) -> String,
    ) -> Validated<(), String> {
        match &self.length {
            Some(t) => {
                if *t == actual.len() as i64 {
                    Good(())
                } else {
                    Validated::fail(formatter(
                        format!("length of {}", t).as_str(),
                        format!("{:?}", actual).as_str(),
                    ))
                }
            }
            None => Good(()),
        }
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
            self.check_length(val, formatter),
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
            self.check_length(val, formatter),
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
            string_specification.length,
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
            string_specification.length,
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
        let is_none_of = match specification.as_ref() {
            Some(Specification::NoneOf(_)) => true,
            _ => false,
        };

        let violation =
            specification.is_some() && !is_none_of && min.as_ref().or(max.as_ref()).is_some();

        if violation {
            return Err(
                "Cannot specify min or max alongside either of oneOf, anyOf, or value".to_string(),
            );
        }

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

        let modifier_validation = if modifier.is_some() {
            specification
                .as_ref()
                .map(|s| match s {
                    Specification::Value(_) => Good(modifier.clone()),
                    _ => {
                        if modifier.is_some() {
                            Validated::fail(
                                "modifier can only be used with \"value\" field".to_string(),
                            )
                        } else {
                            Good(modifier.clone())
                        }
                    }
                })
                .unwrap_or_else(|| {
                    Validated::fail("modifier can only be used with \"value\" field".to_string())
                })
        } else {
            Good(modifier.clone())
        };

        date_validator(&min, "min")
            .map3(
                date_validator(&max, "max"),
                modifier_validation,
                |min_v, max_v, modifier| Self {
                    format,
                    min: min_v,
                    max: max_v,
                    modifier,
                    specification,
                },
            )
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

    pub fn apply_modifier(time: &DateTime<Local>, modifier: &Modifier) -> Option<DateTime<Local>> {
        trace!("apply_modifier({:?},{:?})", time, modifier);

        let val = match &modifier.value {
            Value::Number(val) => val.as_u64(),
            Value::String(val) => val.parse::<u64>().ok(),
            _ => None,
        };

        val.and_then(|mod_value| {
            match modifier.operation.to_lowercase().as_str() {
                "add" => {
                    let modified_date = match modifier.unit.to_lowercase().as_str() {
                        "days" => time.checked_add_days(Days::new(mod_value)),
                        "weeks" => time.checked_add_days(Days::new(mod_value * 7)),
                        "months" => time.checked_add_months(Months::new(mod_value as u32)),
                        // TODO: add support for years
                        _ => None,
                    };
                    modified_date
                }
                "subtract" => {
                    let modified_date = match modifier.unit.to_lowercase().as_str() {
                        "days" => time.checked_sub_days(Days::new(mod_value)),
                        "weeks" => time.checked_sub_days(Days::new(mod_value * 7)),
                        "months" => time.checked_sub_months(Months::new(mod_value as u32)),
                        // TODO: add support for years
                        _ => None,
                    };
                    modified_date
                }
                _ => None,
            }
        })
    }

    //validates format stuff and applies the modifier
    //this can be used to generate or validate
    //But its not a "random" generator like our other specifications
    fn get(&self, string_val: &str) -> Option<String> {
        trace!("get({:?})", string_val);
        self.str_to_time(string_val)
            .ok()
            .and_then(|dt| {
                if let Some(m) = &self.modifier {
                    Self::apply_modifier(&dt, m)
                } else {
                    Some(dt)
                }
            })
            .map(|d| self.time_to_str(&d))
    }
}

impl TryFrom<UnvalidatedDateSpecification> for DateSpecification {
    type Error = String;

    fn try_from(unvalidated_date: UnvalidatedDateSpecification) -> Result<Self, Self::Error> {
        let unvalidated_spec = UnvalidatedSpecification::<String> {
            name: unvalidated_date.name,
            value: unvalidated_date.value,
            any_of: unvalidated_date.any_of,
            one_of: unvalidated_date.one_of,
            none_of: unvalidated_date.none_of,
        };

        let maybe_spec = TryInto::<Option<Specification<String>>>::try_into(unvalidated_spec);

        maybe_spec.and_then(|spec| {
            Self::new(
                spec,
                unvalidated_date.min,
                unvalidated_date.max,
                unvalidated_date.format,
                unvalidated_date.modifier,
            )
        })
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

        //If there is a modifier specified, we have to apply the inverse..
        //Without doing so, we'll fail validation
        let modified_time = self
            .modifier
            .as_ref()
            .map(|m| m.get_inverse())
            .and_then(|m| Self::apply_modifier(&time, &m))
            .unwrap_or(time);

        debug!("Time is {:?}; Modified time is {:?}", time, modified_time);
        let mut ret = vec![
            self.check_min(&modified_time, formatter),
            self.check_max(&modified_time, formatter),
        ];

        ret.append(
            self.specification
                .as_ref()
                .map(|s| s.check(&self.time_to_str(&modified_time), formatter))
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
        let is_none_of = match specification.as_ref() {
            Some(Specification::NoneOf(_)) => true,
            _ => false,
        };

        let violation =
            specification.is_some() && !is_none_of && min.as_ref().or(max.as_ref()).is_some();

        if violation {
            return Err(
                "Cannot specify min or max alongside either of oneOf, anyOf, or value".to_string(),
            );
        }

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
        let modifier_validation = if modifier.is_some() {
            specification
                .as_ref()
                .map(|s| match s {
                    Specification::Value(_) => Good(modifier.clone()),
                    _ => {
                        if modifier.is_some() {
                            Validated::fail(
                                "modifier can only be used with \"value\" field".to_string(),
                            )
                        } else {
                            Good(modifier.clone())
                        }
                    }
                })
                .unwrap_or_else(|| {
                    Validated::fail("modifier can only be used with \"value\" field".to_string())
                })
        } else {
            Good(modifier.clone())
        };
        date_validator(&min, "min")
            .map3(
                date_validator(&max, "max"),
                modifier_validation,
                |min_v, max_v, modifier| Self {
                    format,
                    min: min_v,
                    max: max_v,
                    modifier,
                    specification,
                },
            )
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

    pub fn apply_modifier(time: &DateTime<Local>, modifier: &Modifier) -> Option<DateTime<Local>> {
        trace!("apply_modifier({:?},{:?})", time, modifier);

        let val = match &modifier.value {
            Value::Number(val) => val.as_u64(),
            Value::String(val) => val.parse::<u64>().ok(),
            _ => None,
        };

        val.and_then(|mod_value| {
            match modifier.operation.to_lowercase().as_str() {
                "add" => {
                    let modified_date = match modifier.unit.to_lowercase().as_str() {
                        "days" => time.checked_add_days(Days::new(mod_value)),
                        "weeks" => time.checked_add_days(Days::new(mod_value * 7)),
                        "months" => time.checked_add_months(Months::new(mod_value as u32)),
                        // TODO: add support for years
                        _ => None,
                    };
                    modified_date
                }
                "subtract" => {
                    let modified_date = match modifier.unit.to_lowercase().as_str() {
                        "days" => time.checked_sub_days(Days::new(mod_value)),
                        "weeks" => time.checked_sub_days(Days::new(mod_value * 7)),
                        "months" => time.checked_sub_months(Months::new(mod_value as u32)),
                        // TODO: add support for years
                        _ => None,
                    };
                    modified_date
                }
                _ => None,
            }
        })
    }

    //validates format stuff and applies the modifier
    //this can be used to generate or validate
    //But its not a "random" generator like our other specifications
    fn get(&self, string_val: &str) -> Option<String> {
        trace!("get({:?})", string_val);
        self.str_to_time(string_val)
            .ok()
            .and_then(|dt| {
                if let Some(m) = &self.modifier {
                    Self::apply_modifier(&dt, m)
                } else {
                    Some(dt)
                }
            })
            .map(|d| self.time_to_str(&d))
    }
}

impl TryFrom<UnvalidatedDateSpecification> for DateTimeSpecification {
    type Error = String;

    fn try_from(unvalidated_date: UnvalidatedDateSpecification) -> Result<Self, Self::Error> {
        let unvalidated_spec = UnvalidatedSpecification::<String> {
            name: unvalidated_date.name,
            value: unvalidated_date.value,
            any_of: unvalidated_date.any_of,
            one_of: unvalidated_date.one_of,
            none_of: unvalidated_date.none_of,
        };

        let maybe_spec = TryInto::<Option<Specification<String>>>::try_into(unvalidated_spec);

        maybe_spec.and_then(|spec| {
            Self::new(
                spec,
                unvalidated_date.min,
                unvalidated_date.max,
                unvalidated_date.format,
                unvalidated_date.modifier,
            )
        })
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

        //If there is a modifier specified, we have to apply the inverse..
        //Without doing so, we'll fail validation
        let modified_time = self
            .modifier
            .as_ref()
            .map(|m| m.get_inverse())
            .and_then(|m| Self::apply_modifier(&time, &m))
            .unwrap_or(time);

        debug!("Time is {:?}; Modified time is {:?}", time, modified_time);
        let mut ret = vec![
            self.check_min(&modified_time, formatter),
            self.check_max(&modified_time, formatter),
        ];

        ret.append(
            self.specification
                .as_ref()
                .map(|s| s.check(&self.time_to_str(&modified_time), formatter))
                .unwrap_or_default()
                .as_mut(),
        );

        ret
    }
}

#[derive(Hash, Serialize, Debug, Clone, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum DatumSchema {
    #[serde(alias = "boolean", alias = "Bool", alias = "bool")]
    Boolean {
        #[serde(flatten)]
        specification: Option<BooleanSpecification>,
    },
    #[serde(alias = "float")]
    Float {
        #[serde(flatten)]
        specification: Option<FloatSpecification>,
    },
    #[serde(alias = "integer", alias = "Int", alias = "int")]
    Integer {
        #[serde(flatten)]
        specification: Option<IntegerSpecification>,
    },
    #[serde(alias = "string")]
    String {
        #[serde(flatten)]
        specification: Option<StringSpecification>,
    },
    #[serde(alias = "date")]
    Date {
        #[serde(flatten)]
        specification: Option<DateSpecification>,
    },
    #[serde(alias = "dateTime", alias = "Datetime", alias = "datetime")]
    DateTime {
        #[serde(flatten)]
        specification: Option<DateTimeSpecification>,
    },
    #[serde(alias = "name")]
    Name {
        #[serde(flatten)]
        specification: Option<NameSpecification>,
    },
    #[serde(alias = "email")]
    Email {
        #[serde(flatten)]
        specification: Option<EmailSpecification>,
    },
    #[serde(alias = "list")]
    List {
        #[serde(flatten)]
        specification: Option<SequenceSpecification>,
    },
    #[serde(alias = "object")]
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
            DatumSchema::Integer { specification } => {
                Self::check_int(specification, actual, formatter)
            }
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

impl TryFrom<UnvalidatedDatumSchemaVariable> for DatumSchema {
    type Error = String;
    fn try_from(unvalidated: UnvalidatedDatumSchemaVariable) -> Result<Self, Self::Error> {
        match unvalidated {
            UnvalidatedDatumSchemaVariable::Boolean(specification) => {
                TryInto::<Option<BooleanSpecification>>::try_into(specification).map(|spec| {
                    DatumSchema::Boolean {
                        specification: spec,
                    }
                })
            }
            UnvalidatedDatumSchemaVariable::Email(spec) => {
                TryInto::<StringSpecification>::try_into(spec).map(|s| DatumSchema::Email {
                    specification: Some(EmailSpecification { specification: s }),
                })
            }
            UnvalidatedDatumSchemaVariable::Name(spec) => {
                TryInto::<StringSpecification>::try_into(spec).map(|s| DatumSchema::Name {
                    specification: Some(NameSpecification { specification: s }),
                })
            }
            UnvalidatedDatumSchemaVariable::String(spec) => {
                TryInto::<StringSpecification>::try_into(spec).map(|s| DatumSchema::String {
                    specification: Some(s),
                })
            }
            UnvalidatedDatumSchemaVariable::Float(spec) => {
                TryInto::<FloatSpecification>::try_into(spec).map(|s| DatumSchema::Float {
                    specification: Some(s),
                })
            }
            UnvalidatedDatumSchemaVariable::Integer(spec) => {
                TryInto::<IntegerSpecification>::try_into(spec).map(|s| DatumSchema::Integer {
                    specification: Some(s),
                })
            }
            UnvalidatedDatumSchemaVariable::DateTime(spec) => {
                TryInto::<DateTimeSpecification>::try_into(spec).map(|s| DatumSchema::DateTime {
                    specification: Some(s),
                })
            }
            UnvalidatedDatumSchemaVariable::Date(spec) => {
                TryInto::<DateSpecification>::try_into(spec).map(|s| DatumSchema::Date {
                    specification: Some(s),
                })
            }
            UnvalidatedDatumSchemaVariable::Object { name: _, schema } => match schema {
                None => Ok(DatumSchema::Object { schema: None }),
                Some(schema_val) => {
                    let f = schema_val
                        .into_iter()
                        .map(|(k, v)| match v {
                            UnvalidatedValueOrDatumSchema::Datum(ud) => {
                                TryInto::<DatumSchema>::try_into(ud)
                                    .map(|ds| (k, ValueOrDatumSchema::Datum(ds)))
                            }
                            UnvalidatedValueOrDatumSchema::Values(v) => {
                                Ok((k, ValueOrDatumSchema::Values(v)))
                            }
                        })
                        .collect::<Result<BTreeMap<String, ValueOrDatumSchema>, String>>();
                    f.map(|tree| DatumSchema::Object { schema: Some(tree) })
                }
            },
            UnvalidatedDatumSchemaVariable::List(unvalidated) => {
                let violation = unvalidated.length.is_some()
                    && unvalidated
                        .max_length
                        .as_ref()
                        .or(unvalidated.min_length.as_ref())
                        .is_some();
                if violation {
                    return Err(
                        "Cannot specify minLength or maxLength alongside length".to_string()
                    );
                }
                let values_or_schema = match unvalidated.schema {
                    None => Ok(None),
                    Some(values_or_schema) => match values_or_schema {
                        UnvalidatedValuesOrSchema::Schemas(s) => {
                            let any_of = match s.any_of {
                                None => Ok(None),
                                Some(us) => us
                                    .into_iter()
                                    .map(|u| TryInto::<DatumSchema>::try_into(*u))
                                    .map(|u| u.map(Box::new))
                                    .collect::<Result<Vec<Box<DatumSchema>>, String>>()
                                    .map(Some),
                            };
                            let one_of = match s.one_of {
                                None => Ok(None),
                                Some(us) => us
                                    .into_iter()
                                    .map(|u| TryInto::<DatumSchema>::try_into(*u))
                                    .map(|u| u.map(Box::new))
                                    .collect::<Result<Vec<Box<DatumSchema>>, String>>()
                                    .map(Some),
                            };
                            let none_of = match s.none_of {
                                None => Ok(None),
                                Some(us) => us
                                    .into_iter()
                                    .map(|u| TryInto::<DatumSchema>::try_into(*u))
                                    .map(|u| u.map(Box::new))
                                    .collect::<Result<Vec<Box<DatumSchema>>, String>>()
                                    .map(Some),
                            };

                            let value = match s.value {
                                None => Ok(None),
                                Some(v) => {
                                    TryInto::<DatumSchema>::try_into(*v).map(Box::new).map(Some)
                                }
                            };

                            match (any_of, none_of, one_of, value) {
                                (Ok(anys), Ok(nones), Ok(ones), Ok(val)) => {
                                    TryInto::<Option<Specification<Box<DatumSchema>>>>::try_into(
                                        UnvalidatedSpecification::<Box<DatumSchema>> {
                                            name: None,
                                            any_of: anys,
                                            none_of: nones,
                                            one_of: ones,
                                            value: val,
                                        },
                                    )
                                    .map(|a| a.map(ValuesOrSchema::Schemas))
                                }
                                //I lose error info here.
                                _ => Err("Foo".to_string()),
                            }
                        }
                        //Simply treat it as a Tagged schema that only has value specified.
                        UnvalidatedValuesOrSchema::UntaggedSchema(schema) => {
                            TryInto::<DatumSchema>::try_into(*schema)
                                .map(Box::new)
                                .and_then(|ds| {
                                    TryInto::<Option<Specification<Box<DatumSchema>>>>::try_into(
                                        UnvalidatedSpecification::<Box<DatumSchema>> {
                                            name: None,
                                            any_of: None,
                                            none_of: None,
                                            one_of: None,
                                            value: Some(ds),
                                        },
                                    )
                                })
                                .map(|a| a.map(ValuesOrSchema::Schemas))
                        }
                        UnvalidatedValuesOrSchema::Values(v) => {
                            TryInto::<Option<Specification<Vec<Value>>>>::try_into(v)
                                .map(|a| a.map(ValuesOrSchema::Values))
                        }
                        //Simply make it a tagged variant and reapply simple transformation from above
                        UnvalidatedValuesOrSchema::UntaggedLiterals(literals) => {
                            let foo = UnvalidatedSpecification::<Vec<Value>> {
                                value: Some(literals),
                                any_of: None,
                                name: None,
                                none_of: None,
                                one_of: None,
                            };
                            TryInto::<Option<Specification<Vec<Value>>>>::try_into(foo)
                                .map(|a| a.map(ValuesOrSchema::Values))
                        }
                    },
                };
                values_or_schema.and_then(|schema| {
                    SequenceSpecification::new(
                        schema,
                        unvalidated.length,
                        unvalidated.min_length,
                        unvalidated.max_length,
                    )
                    .map(|s| DatumSchema::List {
                        specification: Some(s),
                    })
                })
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct UnvalidatedRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
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
        serde_json::to_string(&self.body).unwrap().hash(state);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct UnvalidatedCompareRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub method: Option<http::Verb>,
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Vec<http::Parameter>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub add_params: Option<Vec<http::Parameter>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ignore_params: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<Vec<http::Header>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub add_headers: Option<Vec<http::Header>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ignore_headers: Option<Vec<String>>,
    //Requests can only contain a body OR a body_schema
    //We used to signify this using (serde-flattened)enums, but its
    //easier to manage validation errors if we flatten the
    //structure manually in this manner and leave the enums only
    //in the (Validated)CompareDescriptor struct
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<UnvalidatedVariableNameOrValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strict: Option<bool>,
}

impl Hash for UnvalidatedCompareRequest {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.method.hash(state);
        self.url.hash(state);
        self.params.hash(state);
        serde_json::to_string(&self.body).unwrap().hash(state);
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
    File {
        file: String,
    },
    Schema(DatumSchema),
    Value {
        value: Value,
    },
    #[serde(rename_all = "camelCase")]
    ValueSet {
        value_set: Vec<Value>,
    },
}

impl Hash for ValueOrDatumOrFile {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            ValueOrDatumOrFile::File { file } => file.hash(state),
            ValueOrDatumOrFile::Schema(s) => s.hash(state),
            ValueOrDatumOrFile::Value { value } => {
                serde_json::to_string(value).unwrap().hash(state)
            }
            ValueOrDatumOrFile::ValueSet { value_set } => {
                serde_json::to_string(value_set).unwrap().hash(state)
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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
#[serde(untagged, rename_all = "camelCase")]
pub enum UnvalidatedVariable {
    File(UnvalidatedFileVariable),
    Datum(UnvalidatedDatumSchemaVariable),
    Simple(SimpleValueVariable),
    ValueSet(UnvalidatedValueSet),
}

#[derive(Hash, Serialize, Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UnvalidatedValueSet {
    pub name: String,
    pub value_set: Vec<Value>,
}

#[derive(Hash, Serialize, Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UnvalidatedFileVariable {
    pub name: String,
    pub file: String,
}

#[derive(Hash, Serialize, Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SimpleValueVariable {
    pub name: String,
    pub value: serde_json::Value,
}

#[derive(Default, Serialize, Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct UnvalidatedSpecification<T> {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub any_of: Option<Vec<T>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub one_of: Option<Vec<T>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub none_of: Option<Vec<T>>,
}

impl<T> Hash for UnvalidatedSpecification<T>
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

#[derive(Default, Serialize, Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct UnvalidatedNumericSpecification<T: std::fmt::Display + Clone + PartialOrd> {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub any_of: Option<Vec<T>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub one_of: Option<Vec<T>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub none_of: Option<Vec<T>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max: Option<T>,
}

#[derive(Default, Serialize, Hash, Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct UnvalidatedStringSpecification {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub any_of: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub one_of: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub none_of: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub length: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_length: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_length: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pattern: Option<String>,
}

#[derive(Default, Hash, Serialize, Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct UnvalidatedDateSpecification {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub any_of: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub one_of: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub none_of: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub min: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modifier: Option<variable::Modifier>,
}

pub type UnvalidatedFloatSpecification = UnvalidatedNumericSpecification<f64>;
pub type UnvalidatedIntegerSpecification = UnvalidatedNumericSpecification<i64>;

#[derive(Debug, Serialize, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, untagged)]
pub enum UnvalidatedValueOrDatumSchema {
    Datum(UnvalidatedDatumSchemaVariable),
    Values(Value),
}

impl Hash for UnvalidatedValueOrDatumSchema {
    fn hash<H: Hasher>(&self, state: &mut H) {
        serde_json::to_string(self).unwrap().hash(state)
    }
}

#[derive(Debug, Serialize, Clone, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum UnvalidatedValuesOrSchema {
    Schemas(UnvalidatedSpecification<Box<UnvalidatedDatumSchemaVariable>>),
    Values(UnvalidatedSpecification<Vec<Value>>),
    //I don't know if we advertise untagged schemas
    //There are use cases but... its a stretch
    UntaggedSchema(Box<UnvalidatedDatumSchemaVariable>),
    UntaggedLiterals(Vec<Value>),
}
#[derive(Serialize, Debug, Clone, Deserialize, PartialEq, Default)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct UnvalidatedSequenceSpecification {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema: Option<UnvalidatedValuesOrSchema>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub length: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_length: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_length: Option<i64>,
}

impl Hash for UnvalidatedSequenceSpecification {
    fn hash<H: Hasher>(&self, state: &mut H) {
        serde_json::to_string(self).unwrap().hash(state)
    }
}

#[derive(Hash, Serialize, Debug, Clone, Deserialize, PartialEq)]
#[serde(tag = "type", deny_unknown_fields)]
pub enum UnvalidatedDatumSchemaVariable {
    #[serde(alias = "Boolean", alias = "boolean", alias = "Bool", alias = "bool")]
    Boolean(UnvalidatedSpecification<bool>),
    #[serde(alias = "float")]
    Float(UnvalidatedFloatSpecification),
    #[serde(alias = "Integer", alias = "integer", alias = "Int", alias = "int")]
    Integer(UnvalidatedIntegerSpecification),
    #[serde(alias = "string")]
    String(UnvalidatedStringSpecification),
    #[serde(alias = "date")]
    Date(UnvalidatedDateSpecification),
    #[serde(
        alias = "DateTime",
        alias = "dateTime",
        alias = "Datetime",
        alias = "datetime"
    )]
    DateTime(UnvalidatedDateSpecification),
    #[serde(alias = "name", alias = "Name")]
    Name(UnvalidatedStringSpecification),
    #[serde(alias = "email", alias = "Email")]
    Email(UnvalidatedStringSpecification),
    #[serde(alias = "list", alias = "List")]
    List(UnvalidatedSequenceSpecification),
    #[serde(alias = "object", alias = "Object")]
    Object {
        #[serde(skip_serializing_if = "Option::is_none")]
        name: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        //Supports not having to explicitly specify type for every object member
        schema: Option<BTreeMap<String, UnvalidatedValueOrDatumSchema>>,
    },
}

#[derive(Hash, Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct UnvalidatedStage {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub request: UnvalidatedRequest,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compare: Option<UnvalidatedCompareRequest>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response: Option<UnvalidatedResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variables: Option<Vec<UnvalidatedVariable>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delay: Option<u64>,
}

#[derive(Hash, Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct UnvalidatedRequestResponse {
    pub request: UnvalidatedRequest,
    pub response: Option<UnvalidatedResponse>,
}

#[derive(Hash, Debug, Clone, Serialize, Deserialize, PartialEq)]
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

pub fn generate_number<T>(
    spec: &NumericSpecification<T>,
    max_attempts: u16,
    default_min: T,
    default_max: T,
) -> Option<T>
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
                    spec.min.unwrap_or(default_min),
                    spec.max.unwrap_or(default_max),
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
    // note: this could potentially overflow with very large floats, but there doesn't seem to be another way to do this...
    generate_number::<f64>(spec, max_attempts, 0_f64, 100_f64)
        .map(|f| (f * 1001.0).round() / 1000.0)
}

pub fn generate_integer(spec: &NumericSpecification<i64>, max_attempts: u16) -> Option<i64> {
    generate_number::<i64>(spec, max_attempts, 0_i64, 100_i64)
}

pub fn generate_string(spec: &StringSpecification, max_attempts: u16) -> Option<String> {
    trace!("generate_string()");
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ\
                            abcdefghijklmnopqrstuvwxyz\
                            "; // 0123456789)(*&^%$#@!~";

    let mut rng = rand::thread_rng();
    let min_length = spec
        .min_length
        .unwrap_or(min(5, spec.max_length.map(|m| m / 2).unwrap_or(5)));
    let max_length = spec.max_length.unwrap_or(max(min_length * 2, 20));
    let string_length = spec
        .length
        .unwrap_or(rng.gen_range(min_length..=max_length));

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
        .unwrap_or(
            min.with_year(max(Local::now().year(), min.year() + 10))
                .unwrap(),
        );

    let mut rng = rand::thread_rng();

    (0..max_attempts)
        .map(|_| {
            spec.specification
                .as_ref()
                //issue here is "generate_if_constrained" can't be used indiscriminately ; it doesn't apply modifier unless we do it at
                //parse time. Would require Unvalidated version of type. So we have to match
                .and_then(|s| match s {
                    Specification::Value(v) => spec.get(v),
                    Specification::UnTaggedValue(v) => spec.get(v),
                    _ => s.generate_if_constrained(&mut rng),
                })
                .unwrap_or_else(|| {
                    let days_diff = (max - min).num_days();
                    let new_date = min + Duration::days(rng.gen_range(0..=days_diff));
                    spec.time_to_str(&new_date)
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
        .unwrap_or(
            min.with_year(max(Local::now().year(), min.year() + 10))
                .unwrap(),
        );

    let mut rng = rand::thread_rng();

    (0..max_attempts)
        .map(|_| {
            spec.specification
                .as_ref()
                //issue here is "generate_if_constrained" can't be used indiscriminately ; it doesn't apply modifier unless we do it at
                //parse time. Would require Unvalidated version of type. So we have to match
                .and_then(|s| match s {
                    Specification::Value(v) => spec.get(v),
                    Specification::UnTaggedValue(v) => spec.get(v),
                    _ => s.generate_if_constrained(&mut rng),
                })
                .unwrap_or_else(|| {
                    let seconds_diff = (max - min).num_seconds();
                    let new_date_time = min + Duration::seconds(rng.gen_range(0..=seconds_diff));
                    spec.time_to_str(&new_date_time)
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
    let mut rng = rand::thread_rng();
    (0..max_attempts)
        .map(|_| {
            spec.specification
                .specification
                .as_ref()
                .and_then(|s| s.generate_if_constrained(&mut rng))
                .unwrap_or_else(|| {
                    format!(
                        "{} {}",
                        GIVEN_NAMES.get(rng.gen_range(0..20)).unwrap(),
                        SURNAMES.get(rng.gen_range(0..20)).unwrap()
                    )
                })
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
    let mut rng = rand::thread_rng();
    generate_string(&spec.specification, max_attempts)
        .map(|s| format!("{}@{}", s, EMAIL_DOMAINS.get(rng.gen_range(0..3)).unwrap()))
}

pub fn generate_list(spec: &SequenceSpecification, max_attempts: u16) -> Option<Value> {
    // what ever shall I generate??? -> Went with ints for now
    // you should instead have a list of generators you random access into
    trace!("generate_list({:?})", spec);
    let mut rng = rand::thread_rng();
    let min_length = spec.min_length.unwrap_or(1);
    let max_length = spec.max_length.unwrap_or(max(min_length * 2, 10));
    let actual_length = spec
        .length
        .unwrap_or(rng.gen_range(min_length..=max_length));

    (0..max_attempts)
        .map(|_| {
            spec.schema
                .as_ref()
                .map(|s| match s {
                    ValuesOrSchema::Schemas(_) => (0..actual_length)
                        .map(|_| serde_json::Value::from(s.generate_if_constrained(&mut rng)))
                        .collect::<Vec<Value>>(),
                    ValuesOrSchema::Values(v) => {
                        v.generate_if_constrained(&mut rng).unwrap_or_default()
                    }
                })
                .unwrap_or_else(|| {
                    let int_spec = NumericSpecification::<i64>::default();
                    (0..actual_length)
                        .map(|_| Value::from(generate_integer(&int_spec, max_attempts)))
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
        DatumSchema::Integer { specification } => generate_integer(
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
    use serde_json::json;
    use variable::Modifier;

    use super::*;
    #[test]
    fn unvalidated_specification_unknown_fields_detected() {
        let json = json!({
            "name" : "blah",
            "madeup" : "yo"
        });

        assert!(serde_json::from_value::<UnvalidatedSpecification<bool>>(json).is_err());
    }

    #[test]
    fn unvalidated_specification_disallow_any_of_and_one_of() {
        let unvalidated = UnvalidatedSpecification::<bool> {
            any_of: Some(vec![]),
            none_of: Some(vec![]),
            ..Default::default()
        };

        let attempt: Result<Option<Specification<bool>>, String> = unvalidated.try_into();
        match attempt {
            Err(e) => assert_eq!(
                "can only specify one of the following constraints: oneOf, anyOf, noneOf, or value",
                e
            ),
            Ok(_) => assert!(false),
        };
    }

    #[test]
    fn unvalidated_specification_disallow_value_and_any_of() {
        let unvalidated = UnvalidatedSpecification::<bool> {
            any_of: Some(vec![]),
            value: Some(false),
            ..Default::default()
        };

        let attempt: Result<Option<Specification<bool>>, String> = unvalidated.try_into();
        match attempt {
            Err(e) => assert_eq!(
                "can only specify one of the following constraints: oneOf, anyOf, noneOf, or value",
                e
            ),
            Ok(_) => assert!(false),
        };
    }

    #[test]
    fn unvalidated_specification_disallow_value_and_one_of() {
        let unvalidated = UnvalidatedSpecification::<bool> {
            one_of: Some(vec![]),
            value: Some(false),
            ..Default::default()
        };

        let attempt: Result<Option<Specification<bool>>, String> = unvalidated.try_into();
        match attempt {
            Err(e) => assert_eq!(
                "can only specify one of the following constraints: oneOf, anyOf, noneOf, or value",
                e
            ),
            Ok(_) => assert!(false),
        };
    }

    #[test]
    fn unvalidated_specification_disallow_value_and_none_of() {
        let unvalidated = UnvalidatedSpecification::<bool> {
            none_of: Some(vec![]),
            value: Some(false),
            ..Default::default()
        };

        let attempt: Result<Option<Specification<bool>>, String> = unvalidated.try_into();
        match attempt {
            Err(_) => assert!(true),
            Ok(_) => assert!(false),
        };
    }

    #[test]
    fn unvalidated_numeric_specification_unknown_fields_detected() {
        let json = json!({
            "name" : "blah",
            "madeup" : "yo"
        });

        assert!(serde_json::from_value::<UnvalidatedFloatSpecification>(json).is_err());
    }

    #[test]
    fn unvalidated_numeric_specification_disallow_any_of_and_one_of() {
        let unvalidated = UnvalidatedFloatSpecification {
            any_of: Some(vec![]),
            none_of: Some(vec![]),
            ..Default::default()
        };

        let attempt: Result<FloatSpecification, String> = unvalidated.try_into();
        match attempt {
            Err(e) => assert_eq!(
                "can only specify one of the following constraints: oneOf, anyOf, noneOf, or value",
                e
            ),
            Ok(_) => assert!(false),
        };
    }

    #[test]
    fn unvalidated_numeric_specification_disallow_value_and_any_of() {
        let unvalidated = UnvalidatedFloatSpecification {
            any_of: Some(vec![]),
            value: Some(12.0),
            ..Default::default()
        };

        let attempt: Result<FloatSpecification, String> = unvalidated.try_into();
        match attempt {
            Err(e) => assert_eq!(
                "can only specify one of the following constraints: oneOf, anyOf, noneOf, or value",
                e
            ),
            Ok(_) => assert!(false),
        };
    }

    #[test]
    fn unvalidated_numeric_specification_disallow_value_and_one_of() {
        let unvalidated = UnvalidatedFloatSpecification {
            one_of: Some(vec![]),
            value: Some(12.0),
            ..Default::default()
        };

        let attempt: Result<FloatSpecification, String> = unvalidated.try_into();
        match attempt {
            Err(e) => assert_eq!(
                "can only specify one of the following constraints: oneOf, anyOf, noneOf, or value",
                e
            ),
            Ok(_) => assert!(false),
        };
    }

    #[test]
    fn unvalidated_numeric_specification_disallow_value_and_none_of() {
        let unvalidated = UnvalidatedFloatSpecification {
            none_of: Some(vec![]),
            value: Some(12.0),
            ..Default::default()
        };

        let attempt: Result<FloatSpecification, String> = unvalidated.try_into();
        match attempt {
            Err(_) => assert!(true),
            Ok(_) => assert!(false),
        };
    }

    #[test]
    fn unvalidated_string_specification_length_alongside_any_of() {
        let unvalidated = UnvalidatedStringSpecification {
            length: Some(12),
            any_of: Some(vec![]),
            ..Default::default()
        };

        let attempt: Result<StringSpecification, String> = unvalidated.try_into();
        match attempt {
            Err(e) => assert_eq!(
                "Cannot specify minLength, maxLength, or pattern alongside either of oneOf, anyOf, or value",
                e
            ),
            Ok(_) => assert!(false),
        };
    }

    #[test]
    fn unvalidated_string_specification_length_alongside_one_of() {
        let unvalidated = UnvalidatedStringSpecification {
            length: Some(12),
            one_of: Some(vec![]),
            ..Default::default()
        };

        let attempt: Result<StringSpecification, String> = unvalidated.try_into();
        match attempt {
            Err(e) => assert_eq!(
                "Cannot specify minLength, maxLength, or pattern alongside either of oneOf, anyOf, or value",
                e
            ),
            Ok(_) => assert!(false),
        };
    }

    #[test]
    fn unvalidated_string_specification_length_alongside_max() {
        let unvalidated = UnvalidatedStringSpecification {
            length: Some(12),
            min_length: Some(24),
            ..Default::default()
        };

        let attempt: Result<StringSpecification, String> = unvalidated.try_into();
        match attempt {
            Err(e) => assert_eq!(
                "length cannot be specified alongside minLength or maxLength",
                e
            ),
            Ok(_) => assert!(false),
        };
    }

    #[test]
    fn unvalidated_string_specification_unknown_fields_detected() {
        let json = json!({
            "name" : "blah",
            "madeup" : "yo"
        });

        assert!(serde_json::from_value::<UnvalidatedStringSpecification>(json).is_err());
    }

    #[test]
    fn unvalidated_string_specification_disallow_any_of_and_one_of() {
        let unvalidated = UnvalidatedStringSpecification {
            any_of: Some(vec![]),
            none_of: Some(vec![]),
            ..Default::default()
        };

        let attempt: Result<StringSpecification, String> = unvalidated.try_into();
        match attempt {
            Err(e) => assert_eq!(
                "can only specify one of the following constraints: oneOf, anyOf, noneOf, or value",
                e
            ),
            Ok(_) => assert!(false),
        };
    }

    #[test]
    fn unvalidated_string_specification_disallow_value_and_any_of() {
        let unvalidated = UnvalidatedStringSpecification {
            any_of: Some(vec![]),
            value: Some("".to_string()),
            ..Default::default()
        };

        let attempt: Result<StringSpecification, String> = unvalidated.try_into();
        match attempt {
            Err(e) => assert_eq!(
                "can only specify one of the following constraints: oneOf, anyOf, noneOf, or value",
                e
            ),
            Ok(_) => assert!(false),
        };
    }

    #[test]
    fn unvalidated_string_specification_disallow_value_and_one_of() {
        let unvalidated = UnvalidatedStringSpecification {
            one_of: Some(vec![]),
            value: Some("".to_string()),
            ..Default::default()
        };

        let attempt: Result<StringSpecification, String> = unvalidated.try_into();
        match attempt {
            Err(e) => assert_eq!(
                "can only specify one of the following constraints: oneOf, anyOf, noneOf, or value",
                e
            ),
            Ok(_) => assert!(false),
        };
    }

    #[test]
    fn unvalidated_string_specification_disallow_value_and_none_of() {
        let unvalidated = UnvalidatedStringSpecification {
            none_of: Some(vec![]),
            value: Some("".to_string()),
            ..Default::default()
        };

        let attempt: Result<StringSpecification, String> = unvalidated.try_into();
        match attempt {
            Err(_) => assert!(true),
            Ok(_) => assert!(false),
        };
    }

    #[test]
    fn unvalidated_date_specification_unknown_fields_detected() {
        let json = json!({
            "name" : "blah",
            "madeup" : "yo"
        });

        assert!(serde_json::from_value::<UnvalidatedDateSpecification>(json).is_err());
    }

    #[test]
    fn unvalidated_date_specification_disallow_any_of_and_one_of() {
        let unvalidated = UnvalidatedDateSpecification {
            any_of: Some(vec![]),
            none_of: Some(vec![]),
            ..Default::default()
        };

        let attempt: Result<DateSpecification, String> = unvalidated.try_into();
        match attempt {
            Err(e) => assert_eq!(
                "can only specify one of the following constraints: oneOf, anyOf, noneOf, or value",
                e
            ),
            Ok(_) => assert!(false),
        };
    }

    #[test]
    fn unvalidated_date_specification_disallow_value_and_any_of() {
        let unvalidated = UnvalidatedDateSpecification {
            any_of: Some(vec![]),
            value: Some("2020-09-12".to_string()),
            ..Default::default()
        };

        let attempt: Result<DateSpecification, String> = unvalidated.try_into();
        match attempt {
            Err(e) => assert_eq!(
                "can only specify one of the following constraints: oneOf, anyOf, noneOf, or value",
                e
            ),
            Ok(_) => assert!(false),
        };
    }

    #[test]
    fn unvalidated_date_specification_disallow_value_and_one_of() {
        let unvalidated = UnvalidatedDateSpecification {
            one_of: Some(vec![]),
            value: Some("2020-09-12".to_string()),
            ..Default::default()
        };

        let attempt: Result<DateSpecification, String> = unvalidated.try_into();
        match attempt {
            Err(e) => assert_eq!(
                "can only specify one of the following constraints: oneOf, anyOf, noneOf, or value",
                e
            ),
            Ok(_) => assert!(false),
        };
    }

    #[test]
    fn unvalidated_date_specification_disallow_value_and_none_of() {
        let unvalidated = UnvalidatedDateSpecification {
            none_of: Some(vec![]),
            value: Some("2020-09-12".to_string()),
            ..Default::default()
        };

        let attempt: Result<DateSpecification, String> = unvalidated.try_into();
        match attempt {
            Err(_) => assert!(true),
            Ok(_) => assert!(false),
        };
    }

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
    fn string_specification_length_checker() {
        let spec = StringSpecification {
            length: Some(5),
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
                .is_fail()
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
    fn numeric_specification_no_inputs() {
        assert!(IntegerSpecification::new(None, None, None).is_ok());
    }

    #[test]
    fn numeric_specification_min_max_agreemnt() {
        assert!(IntegerSpecification::new(None, Some(12), Some(12)).is_ok());
        assert!(IntegerSpecification::new(None, Some(12), Some(24)).is_ok());
        assert_eq!(
            IntegerSpecification::new(None, Some(24), Some(12)).unwrap_err(),
            "min must be less than or equal to max".to_string()
        );
    }

    #[test]
    fn string_specification_no_inputs() {
        assert!(StringSpecification::new(None, None, None, None, None).is_ok());
    }

    #[test]
    fn string_specification_negative_input() {
        assert_eq!(
            StringSpecification::new(None, Some(-12), None, None, None).unwrap_err(),
            "negative value provided for length".to_string()
        );
        assert_eq!(
            StringSpecification::new(None, None, Some(-12), None, None).unwrap_err(),
            "negative value provided for minLength".to_string()
        );
        assert_eq!(
            StringSpecification::new(None, None, Some(-12), Some(-12), None).unwrap_err(),
            "negative value provided for maxLength,negative value provided for minLength"
                .to_string()
        );
        assert_eq!(
            StringSpecification::new(None, None, None, Some(-24), None).unwrap_err(),
            "negative value provided for maxLength".to_string()
        );
        assert!(StringSpecification::new(None, Some(12), None, None, None).is_ok());
    }

    #[test]
    fn string_specification_min_max_agreement() {
        assert_eq!(
            StringSpecification::new(None, None, Some(24), Some(12), None).unwrap_err(),
            "minLength must be less than or equal to maxLength".to_string()
        );
        assert!(StringSpecification::new(None, None, Some(1), Some(1), None).is_ok());
        assert!(StringSpecification::new(None, None, Some(12), Some(24), None).is_ok());
    }

    #[test]
    fn string_specification_min_max_and_length_specified() {
        vec![
            StringSpecification::new(None, Some(12), Some(24), Some(30), None),
            StringSpecification::new(None, Some(12), Some(24), None, None),
            StringSpecification::new(None, Some(12), None, Some(24), None),
        ]
        .into_iter()
        .for_each(|s| {
            assert_eq!(
                s.unwrap_err(),
                "length cannot be specified alongside minLength or maxLength".to_string()
            )
        });

        assert!(StringSpecification::new(None, Some(12), None, None, None).is_ok());
    }

    #[test]
    fn string_specification_simple_pattern() {
        assert!(
            StringSpecification::new(None, None, None, None, Some("simple".to_string())).is_ok()
        );
    }

    #[test]
    fn string_specification_invalid_pattern() {
        assert!(
            StringSpecification::new(None, None, None, None, Some("?simple?".to_string())).is_err()
        );
    }

    #[test]
    fn string_specification_complex_pattern() {
        assert!(
            StringSpecification::new(None, None, None, None, Some("^simple?".to_string())).is_ok()
        );
    }

    #[test]
    fn date_specification_valid_inputs() {
        assert!(DateSpecification::new(None, None, None, None, None).is_ok());
    }

    #[test]
    fn date_specification_invalid_input() {
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
    fn date_specification_invalid_inputs() {
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
    fn datetime_specification_valid_inputs() {
        assert!(DateTimeSpecification::new(None, None, None, None, None).is_ok());
    }

    #[test]
    fn datetime_specification_invalid_input() {
        let res = DateTimeSpecification::new(
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
    fn datetime_specification_invalid_inputs() {
        let res = DateTimeSpecification::new(
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
    fn date_specification_modifier_missing_value() {
        let validations = vec![
            DateSpecification::new(
                None,
                None,
                None,
                None,
                Some(Modifier {
                    operation: "add".to_string(),
                    value: serde_json::to_value(1).unwrap(),
                    unit: "days".to_string(),
                }),
            ),
            DateSpecification::new(
                Some(Specification::AnyOf(vec![
                    "2020-09-12".to_string(),
                    "2020-09-13".to_string(),
                ])),
                None,
                None,
                None,
                Some(Modifier {
                    operation: "add".to_string(),
                    value: serde_json::to_value("1").unwrap(),
                    unit: "days".to_string(),
                }),
            ),
        ];

        validations.into_iter().for_each(|res| {
            assert!(res.is_err());
            assert!(res.err().unwrap().as_str().contains("value"));
        });
    }

    #[test]
    fn date_specification_modifier_with_value() {
        let res = DateSpecification::new(
            Some(Specification::Value("2020-09-12".to_string())),
            None,
            None,
            None,
            Some(Modifier {
                operation: "add".to_string(),
                value: serde_json::to_value("1").unwrap(),
                unit: "days".to_string(),
            }),
        );

        assert!(res.is_ok());
    }

    #[test]
    fn datetime_specification_modifier_missing_value() {
        let validations = vec![
            DateTimeSpecification::new(
                None,
                None,
                None,
                None,
                Some(Modifier {
                    operation: "add".to_string(),
                    value: serde_json::to_value("1").unwrap(),
                    unit: "days".to_string(),
                }),
            ),
            DateTimeSpecification::new(
                Some(Specification::AnyOf(vec![
                    "2020-09-12".to_string(),
                    "2020-09-13".to_string(),
                ])),
                None,
                None,
                None,
                Some(Modifier {
                    operation: "add".to_string(),
                    value: serde_json::to_value("1").unwrap(),
                    unit: "days".to_string(),
                }),
            ),
        ];

        validations.into_iter().for_each(|res| {
            assert!(res.is_err());
            assert!(res.err().unwrap().as_str().contains("value"));
        });
    }

    #[test]
    fn datetime_specification_modifier_with_value() {
        let res = DateTimeSpecification::new(
            Some(Specification::Value("2020-09-12".to_string())),
            None,
            None,
            None,
            Some(Modifier {
                operation: "add".to_string(),
                value: serde_json::to_value("1").unwrap(),
                unit: "days".to_string(),
            }),
        );

        assert!(res.is_ok());
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
            DatumSchema::Integer {
                specification: None,
            }
            .check(&serde_json::json!({}), true, &|_e, _a| "".to_string())
            .into_iter()
            .collect::<Validated<Vec<()>, String>>()
            .is_fail(),
        );

        assert_eq!(
            false,
            DatumSchema::Integer {
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
    fn body_or_schema_checker_body_empty_ignores() {
        let body_or_schema = BodyOrSchema::Body(json!(
            r#"
                { 
                    "car":"bmw",
                    "plane":"merc",
                    "name": "foo"
                }
            "#
        ));
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
    fn body_or_schema_checker_body_non_empty_ignores() {
        let body_or_schema = BodyOrSchema::Body(json!({
                        "foo" : {
                            "foo3" : 3
                        },
                        "bars" : {
                            "bars2" : {},
                            "another1" : serde_json::Value::Null,
                            "another2" : "",
                            "another3" : "hi"
                        },
                        "plane": serde_json::Value::Null,
                        "name": "foo"
        }));
        let ignores = vec![
            "bars.bars2.bars3".to_string(),
            "foo.foo2".to_string(),
            "cars".to_string(),
        ];
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
                        "cars" : ["yo"],
                        "foo" : {
                            "foo3" : 3
                        },
                        "bars" : {
                            "bars2":{
                                "bars3" : 3
                            },
                            "another1" : serde_json::Value::Null,
                            "another2" : "",
                            "another3" : "hi"
                        },
                        "plane": serde_json::Value::Null,
                        "name": "foo"

                    }),
                    &|_e, _a| "".to_string()
                )
                .into_iter()
                .collect::<Validated<Vec<()>, String>>()
                .is_fail(),
        );
    }

    #[test]
    fn body_or_schema_checker_schema_empty_ignores() {
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
        let spec = NumericSpecification::<i16>::default();
        let num = generate_number(&spec, 10, 0_i16, 100_i16);

        assert!(num.is_some());
        let val = num.unwrap();
        assert!(val >= 0);
        assert!(val <= 100);
        assert!(spec
            .check(&num.unwrap(), &|_e, _a| "".to_string())
            .into_iter()
            .collect::<Validated<Vec<()>, String>>()
            .is_good());
    }

    #[test]
    fn number_generation_with_min_max() {
        let spec = NumericSpecification::<u16> {
            min: Some(1),
            max: Some(9),
            ..Default::default()
        };

        let num = generate_number(&spec, 10, 0_u16, 100_u16);

        assert!(num.is_some());
        let val = num.unwrap();
        assert!(val >= 1);
        assert!(val <= 9);
        assert!(spec
            .check(&num.unwrap(), &|_e, _a| "".to_string())
            .into_iter()
            .collect::<Validated<Vec<()>, String>>()
            .is_good());
    }

    #[test]
    fn string_generation() {
        let spec = StringSpecification::default();
        let val = generate_string(&spec, 10);

        assert!(val.is_some());
        let string = val.clone().unwrap();
        assert!(string.len() >= 5);
        assert!(string.len() <= 20);
        assert!(spec
            .check(&val.unwrap(), &|_e, _a| "".to_string())
            .into_iter()
            .collect::<Validated<Vec<()>, String>>()
            .is_good());
    }

    #[test]
    fn string_generation_with_none_of() {
        let spec = StringSpecification {
            specification: Some(Specification::NoneOf(vec![
                "foo".to_string(),
                "bar".to_string(),
            ])),
            ..Default::default()
        };

        let val = generate_string(&spec, 10);

        assert!(val.is_some());
        let string = val.clone().unwrap();
        assert!(!string.eq("foo"));
        assert!(!string.eq("bar"));
        assert!(spec
            .check(&val.unwrap(), &|_e, _a| "".to_string())
            .into_iter()
            .collect::<Validated<Vec<()>, String>>()
            .is_good());
    }

    #[test]
    fn string_generation_with_length() {
        let spec = StringSpecification {
            length: Some(10),
            ..Default::default()
        };

        let val = generate_string(&spec, 10);

        assert!(val.is_some());
        assert!(val.clone().unwrap_or("".to_string()).len() == 10usize);
        assert!(spec
            .check(&val.unwrap(), &|_e, _a| "".to_string())
            .into_iter()
            .collect::<Validated<Vec<()>, String>>()
            .is_good());
    }

    #[test]
    fn string_generation_with_min_max_length() {
        let spec = StringSpecification {
            min_length: Some(1),
            max_length: Some(5),
            ..Default::default()
        };

        let val = generate_string(&spec, 10);
        let val_length = val.clone().unwrap_or("".to_string()).len();

        assert!(val.is_some());
        assert!(val_length >= 1usize && val_length <= 5usize);
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
    fn date_generation_with_min_max() {
        let min_date = "2024-01-01";
        let max_date = "2024-12-31";
        let spec = DateSpecification::new(
            None,
            Some(min_date.to_string()),
            Some(max_date.to_string()),
            None,
            None,
        )
        .unwrap();
        let val = generate_date(&spec, 10);
        assert!(val.is_some());
        let date = spec.str_to_time(val.clone().unwrap().as_str()).unwrap();
        assert!(date > spec.str_to_time(min_date).unwrap());
        assert!(date < spec.str_to_time(max_date).unwrap());
        assert!(spec
            .check(&val.unwrap(), &|_e, _a| "".to_string())
            .into_iter()
            .collect::<Validated<Vec<()>, String>>()
            .is_good());
    }

    #[test]
    fn date_generation_with_modifier() {
        let spec = DateSpecification::new(
            Some(Specification::Value("2020-09-12".to_string())),
            None,
            None,
            None,
            Some(Modifier {
                operation: "add".to_string(),
                value: serde_json::to_value("1").unwrap(),
                unit: "days".to_string(),
            }),
        )
        .unwrap();

        let val = generate_date(&spec, 10);
        assert!(val.is_some());
        assert!(spec
            .check(&val.as_ref().unwrap(), &|_e, _a| "".to_string())
            .into_iter()
            .collect::<Validated<Vec<()>, String>>()
            .is_good());

        assert_eq!("2020-09-13", val.unwrap());
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
    fn datetime_generation_with_modifier() {
        let spec = DateTimeSpecification::new(
            Some(Specification::Value(
                "2020-09-12 04:27:27.477711492".to_string(),
            )),
            None,
            None,
            None,
            Some(Modifier {
                operation: "add".to_string(),
                value: serde_json::to_value("1").unwrap(),
                unit: "days".to_string(),
            }),
        )
        .unwrap();
        let val = generate_datetime(&spec, 10);
        assert!(val.is_some());
        assert!(spec
            .check(&val.as_ref().unwrap(), &|_e, _a| "".to_string())
            .into_iter()
            .collect::<Validated<Vec<()>, String>>()
            .is_good());

        assert_eq!("2020-09-13 04:27:27.477711492", val.clone().unwrap());
    }

    #[test]
    fn datetime_generation_with_min_max() {
        let min_dt = "2024-01-01 12:34:56";
        let max_dt = "2024-12-31 12:34:56";
        let spec = DateTimeSpecification::new(
            None,
            Some(min_dt.to_string()),
            Some(max_dt.to_string()),
            None,
            None,
        )
        .unwrap();
        let val = generate_datetime(&spec, 10);
        assert!(val.is_some());
        let dt = spec.str_to_time(val.clone().unwrap().as_str()).unwrap();
        assert!(dt > spec.str_to_time(min_dt).unwrap());
        assert!(dt < spec.str_to_time(max_dt).unwrap());
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
        let email = val.clone().unwrap();
        assert!(email.contains("@example."));
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
    fn sequence_specification_length_more_than_actual() {
        assert!(SequenceSpecification {
            length: Some(5),
            ..Default::default()
        }
        .check_length(
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
    fn sequence_specification_length_equal_to_actual() {
        assert!(SequenceSpecification {
            length: Some(5),
            ..Default::default()
        }
        .check_length(
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
    fn sequence_specification_length_less_than_actual() {
        assert!(SequenceSpecification {
            length: Some(5),
            ..Default::default()
        }
        .check_length(
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
    fn sequence_specification_length_check_no_bounds() {
        assert!(SequenceSpecification {
            ..Default::default()
        }
        .check_length(
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
            Box::from(DatumSchema::Integer {
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
            Box::from(DatumSchema::Integer {
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
            Box::from(DatumSchema::Integer {
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
            Box::from(DatumSchema::Integer {
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
        let spec =
            Specification::<Box<DatumSchema>>::NoneOf(vec![Box::from(DatumSchema::Integer {
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
    fn sequence_specification_min_max_and_length_specified() {
        vec![
            SequenceSpecification::new(None, Some(12), Some(24), Some(30)),
            SequenceSpecification::new(None, Some(12), Some(24), None),
            SequenceSpecification::new(None, Some(12), None, Some(24)),
        ]
        .into_iter()
        .for_each(|s| {
            assert_eq!(
                s.unwrap_err(),
                "length cannot be specified alongside minLength or maxLength".to_string()
            )
        });

        assert!(SequenceSpecification::new(None, Some(12), None, None).is_ok());
    }

    #[test]
    fn list_generation_sequence_specification_has_length() {
        let spec = SequenceSpecification {
            length: Some(5),
            ..Default::default()
        };
        assert!(generate_list(&spec, 1,).unwrap().as_array().unwrap().len() == 5);
    }

    #[test]
    fn list_generation_sequence_specification_has_min_and_max() {
        let spec = SequenceSpecification {
            min_length: Some(5),
            max_length: Some(5),
            ..Default::default()
        };
        assert!(generate_list(&spec, 1,).unwrap().as_array().unwrap().len() == 5);
    }

    #[test]
    fn list_generation_sequence_specification_has_max() {
        let spec = SequenceSpecification {
            max_length: Some(5),
            ..Default::default()
        };
        assert!(generate_list(&spec, 1,).unwrap().as_array().unwrap().len() <= 5);
    }

    #[test]
    fn list_generation_sequence_specification_has_min() {
        let spec = SequenceSpecification {
            min_length: Some(1),
            ..Default::default()
        };
        assert!(generate_list(&spec, 1,).unwrap().as_array().unwrap().len() >= 1);
    }

    #[test]
    fn list_generation_default_sequence_specification() {
        let spec = SequenceSpecification::default();
        assert!(generate_list(&spec, 1,).is_some());
    }
}
