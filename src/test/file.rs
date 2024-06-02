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
use num::Num;
use rand::distributions::uniform::SampleUniform;
use rand::rngs::ThreadRng;
use rand::Rng;
use regex::Regex;
use rnglib::Language;
use rnglib::RNG;
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

#[derive(Serialize, Debug, Clone, Deserialize, PartialEq, PartialOrd, Default)]
#[serde(rename_all = "camelCase")]
pub struct Specification<T> {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub one_of: Option<Vec<T>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub none_of: Option<Vec<T>>,
}

#[derive(Serialize, Debug, Clone, Deserialize, PartialEq, PartialOrd, Default)]
#[serde(rename_all = "camelCase")]
pub struct NumericSpecification<T: std::fmt::Display> {
    #[serde(flatten)]
    pub specification: Specification<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max: Option<T>,
}

type BooleanSpecification = Specification<bool>;
type FloatSpecification = NumericSpecification<f64>;
type IntegerSpecification = NumericSpecification<i64>;

#[derive(Hash, Serialize, Debug, Clone, Deserialize, PartialEq, PartialOrd, Default)]
#[serde(rename_all = "camelCase")]
pub struct StringSpecification {
    #[serde(flatten)]
    pub specification: Specification<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_length: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_length: Option<i64>,
}

#[derive(Hash, Default, Serialize, Debug, Clone, Deserialize, PartialEq)]
pub struct DateSpecification {
    #[serde(flatten)]
    pub specification: Specification<String>,
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
    pub specification: Specification<String>,
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
        match &self.value {
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

impl<T> Hash for Specification<T>
where
    T: PartialEq,
    T: Display,
    T: PartialOrd,
    T: fmt::Debug,
    T: Display,
    T: Serialize,
{
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
{
    fn hash<H: Hasher>(&self, state: &mut H) {
        serde_json::to_string(self).unwrap().hash(state)
    }
}

impl<T> NumericSpecification<T>
where
    T: PartialEq,
    T: Display,
    T: PartialOrd,
    T: fmt::Debug,
{
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
            self.check_val(val, formatter),
            self.check_none_of(val, formatter),
            self.check_one_of(val, formatter),
        ]
    }
}

impl<T> Checker for NumericSpecification<T>
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
        let mut ret = vec![
            self.check_min(val, formatter),
            self.check_max(val, formatter),
        ];
        ret.append(self.specification.check(val, formatter).as_mut());
        ret
    }
}

impl StringSpecification {
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
                        format!("{}", actual).as_str(),
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
                        format!("{}", actual).as_str(),
                    ))
                }
            }
            None => Good(()),
        }
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
        ];
        ret.append(self.specification.check(val, formatter).as_mut());
        ret
    }
}

impl DateSpecification {
    const DEFAULT_FORMAT: &'static str = "%Y-%m-%d";
    fn check_val(
        &self,
        actual: &String,
        formatter: &impl Fn(&str, &str) -> String,
    ) -> Validated<(), String> {
        self.specification.check_val(actual, formatter)
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

    fn check_one_of(
        &self,
        actual: &String,
        formatter: &impl Fn(&str, &str) -> String,
    ) -> Validated<(), String> {
        self.specification.check_one_of(actual, formatter)
    }

    fn check_none_of(
        &self,
        actual: &String,
        formatter: &impl Fn(&str, &str) -> String,
    ) -> Validated<(), String> {
        self.specification.check_none_of(actual, formatter)
    }

    fn get_format(&self) -> String {
        self.format
            .clone()
            .unwrap_or(Self::DEFAULT_FORMAT.to_string())
    }

    fn str_to_time(&self, string_val: &str) -> Result<DateTime<Local>, ParseError> {
        let format = self.get_format();

        NaiveDate::parse_from_str(string_val, format.as_str()).map(|d| {
            Local
                .from_local_datetime(&d.and_hms_opt(0, 0, 0).unwrap())
                .unwrap()
        })
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

        vec![
            self.check_val(val, formatter),
            self.check_min(&time, formatter),
            self.check_max(&time, formatter),
            self.check_none_of(val, formatter),
            self.check_one_of(val, formatter),
        ]
    }
}

impl DateTimeSpecification {
    const DEFAULT_FORMAT: &'static str = "%Y-%m-%d %H:%M:%S%.f";
    fn check_val(
        &self,
        actual: &String,
        formatter: &impl Fn(&str, &str) -> String,
    ) -> Validated<(), String> {
        self.specification.check_val(actual, formatter)
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

    fn check_one_of(
        &self,
        actual: &String,
        formatter: &impl Fn(&str, &str) -> String,
    ) -> Validated<(), String> {
        self.specification.check_one_of(actual, formatter)
    }

    fn check_none_of(
        &self,
        actual: &String,
        formatter: &impl Fn(&str, &str) -> String,
    ) -> Validated<(), String> {
        self.specification.check_none_of(actual, formatter)
    }

    fn get_format(&self) -> String {
        self.format
            .clone()
            .unwrap_or(Self::DEFAULT_FORMAT.to_string())
    }

    fn str_to_time(&self, string_val: &str) -> Result<DateTime<Local>, ParseError> {
        let format = self.get_format();

        NaiveDateTime::parse_from_str(string_val, format.as_str())
            .map(|d| Local.from_local_datetime(&d).unwrap())
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

        vec![
            self.check_val(val, formatter),
            self.check_min(&time, formatter),
            self.check_max(&time, formatter),
            self.check_none_of(val, formatter),
            self.check_one_of(val, formatter),
        ]
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
    EmailSpecification {
        #[serde(flatten)]
        specification: Option<EmailSpecification>,
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
        trace!("validating datum");
        match self {
            DatumSchema::Date { specification } => {
                Self::check_date(specification, actual, formatter)
            }
            DatumSchema::DateTime { specification } => {
                Self::check_datetime(specification, actual, formatter)
            }
            DatumSchema::EmailSpecification { specification } => {
                Self::check_email(specification, actual, formatter)
            }
            DatumSchema::Float { specification } => {
                Self::check_float(specification, actual, formatter)
            }
            DatumSchema::Boolean { specification } => {
                Self::check_bool(specification, actual, formatter)
            }
            DatumSchema::Int { specification } => Self::check_int(specification, actual, formatter),
            DatumSchema::List { schema } => Self::check_list(schema, actual, formatter),
            DatumSchema::Object { schema } => Self::check_object(schema, actual, formatter),
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
        schema: &Option<Box<DatumSchema>>,
        actual: &serde_json::Value,
        formatter: &impl Fn(&str, &str) -> String,
    ) -> Vec<Validated<(), String>> {
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
                    .flat_map(|v| s.check(v, formatter))
                    .collect()
            })
            .unwrap_or(vec![Good(())])
    }

    fn check_object(
        schema: &Option<BTreeMap<String, DatumSchema>>,
        actual: &serde_json::Value,
        formatter: &impl Fn(&str, &str) -> String,
    ) -> Vec<Validated<(), String>> {
        if !actual.is_object() {
            return vec![Validated::fail(formatter("object type", "different type"))];
        }

        let vals = actual.as_object().unwrap();
        schema
            .as_ref()
            .map(|bt| {
                bt.iter()
                    .flat_map(|(k, datum)| {
                        vals.get(k)
                            .map(|v| datum.check(v, formatter))
                            .unwrap_or(vec![Validated::fail(formatter(
                                format!(r#"member "{k}""#).as_str(),
                                format!(r#"object with "{k}" missing"#).as_str(),
                            ))])
                    })
                    .collect()
            })
            .unwrap_or(vec![Good(())])
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnvalidatedRequest {
    pub method: Option<http::Verb>,
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Vec<http::Parameter>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<Vec<http::Header>>,
    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    pub body: Option<BodyOrSchema>,
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
    pub body: Option<BodyOrSchema>,
    pub strict: Option<bool>,
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

#[derive(Hash, Debug, Serialize, Clone, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum ValueOrNumericSpecification<
    T: std::fmt::Display + std::fmt::Debug + std::cmp::PartialOrd + Serialize,
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

/**
    We expose variables to the user as things
    that are either:
        - Strings
        - Datums
        - Files
    However, our implementation type also treats Secrets as
    obfuscated variables by leveraging SecretValue's.

    This requires us to use 2 different types for the implementation
    and the data file (jk::test::File) interface.
**/
#[derive(Hash, Debug, Serialize, Clone, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum StringOrDatumOrFile {
    File { file: String },
    Schema(DatumSchema),
    Value { value: String },
}

pub struct BodyOrSchemaChecker<'a> {
    pub value_or_schema: &'a BodyOrSchema,
    pub ignore_values: &'a [String],
    pub strict: bool,
}

impl<'a> BodyOrSchemaChecker<'a> {
    pub fn check_schema(
        &self,
        actual: &serde_json::Value,
        schema: &DatumSchema,
        formatter: &impl Fn(&str, &str) -> String,
    ) -> Result<Vec<Validated<(), String>>, Box<dyn Error + Send + Sync>> {
        trace!("validating response body using schema");
        //\todo How do I apply ignore in this case?
        //Or does it even make sense? If not, we would need to
        //factor it out of response and into the "Value" part of ValueOrSchema
        Ok(schema.check(actual, formatter))
    }

    pub fn check_expected_value(
        &self,
        actual: &serde_json::Value,
        expected: &serde_json::Value,
        formatter: &impl Fn(&str, &str) -> String,
    ) -> Result<Vec<Validated<(), String>>, Box<dyn Error + Send + Sync>> {
        trace!("validating response body");
        let mut modified_actual = actual.clone();
        let mut modified_expected = expected.clone();

        // TODO: make this more efficient, with a single pass filter
        for path in self.ignore_values.iter() {
            trace!("stripping path({}) from response", path);
            modified_actual = filter_json(path, 0, modified_actual)?;
            modified_expected = filter_json(path, 0, modified_expected)?;
        }

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
            BodyOrSchema::Schema(s) => BodyOrSchemaChecker::check_schema(self, val, s, formatter),
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

#[derive(Hash, Debug, Clone, Serialize, Deserialize)]
pub struct UnvalidatedResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<ValueOrNumericSpecification<u16>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<Vec<http::Header>>,
    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    pub body: Option<BodyOrSchema>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ignore: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extract: Option<Vec<definition::ResponseExtraction>>,
    pub strict: Option<bool>,
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
        }
    }
}

#[derive(Hash, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnvalidatedVariable {
    pub name: String,
    #[serde(flatten)]
    pub value: StringOrDatumOrFile,
}

#[derive(Hash, Debug, Clone, Serialize, Deserialize)]
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
pub struct UnvalidatedRequestResponse {
    pub request: UnvalidatedRequest,
    pub response: Option<UnvalidatedResponse>,
}

#[derive(Hash, Debug, Clone, Serialize, Deserialize)]
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
    if spec.specification.value.is_some() {
        return spec.specification.value;
    }

    let mut rng = rand::thread_rng();
    (0..max_attempts)
        .map(|_| {
            return match spec.specification.one_of.as_ref() {
                Some(vals) => *vals
                    .get(rng.gen_range(0..vals.len()))
                    .unwrap_or(&T::default()),
                None => generate_number_in_range(
                    spec.min.unwrap_or(T::min_value()),
                    spec.max.unwrap_or(T::max_value()),
                    &mut rng,
                ),
            };
        })
        .find(|v| {
            spec.check(v, &|_e, _a| "".to_string())
                .into_iter()
                .collect::<Validated<Vec<()>, String>>()
                .is_good()
        })
}

pub fn generate_bool(spec: &BooleanSpecification, max_attempts: u16) -> Option<bool> {
    if spec.value.is_some() {
        return spec.value.clone();
    }

    let mut rng = rand::thread_rng();

    for _ in 0..max_attempts {
        let ret: bool = match spec.one_of.as_ref() {
            Some(vals) => vals
                .get(rng.gen_range(0..vals.len()))
                .unwrap_or(&bool::default())
                .clone(),
            None => generate_number_in_range(0, 100, &mut rng) % 2 == 0,
        };

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
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ\
                            abcdefghijklmnopqrstuvwxyz\
                            "; // 0123456789)(*&^%$#@!~";

    if spec.specification.value.is_some() {
        return spec.specification.value.clone();
    }

    let mut rng = rand::thread_rng();
    let string_length: usize = rng.gen_range(1..50);

    for _ in 0..max_attempts {
        let ret: String = match spec.specification.one_of.as_ref() {
            Some(vals) => vals
                .get(rng.gen_range(0..vals.len()))
                .unwrap_or(&String::default())
                .clone(),
            None => (0..string_length)
                .map(|_| {
                    let idx = rng.gen_range(0..CHARSET.len());
                    CHARSET[idx] as char
                })
                .collect::<String>(),
        };

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
    if let Some(val) = &spec.specification.value {
        return spec.get(val).ok();
    }

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
            return match spec.specification.one_of.as_ref() {
                Some(vals) => vals
                    .get(rng.gen_range(0..vals.len()))
                    .unwrap_or(&String::default())
                    .clone(),
                None => {
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

                    return chrono::NaiveDate::default()
                        .with_year(year)
                        .and_then(|d| d.with_month(month))
                        .and_then(|d| d.with_day(day))
                        .map(|d| Local.from_local_datetime(&d.and_hms_opt(0, 0, 0).unwrap()))
                        .map(|d| spec.time_to_str(&d.unwrap()))
                        .unwrap_or_default();
                }
            };
        })
        .find(|date_str| {
            spec.check(date_str, &|_e, _a| "".to_string())
                .into_iter()
                .collect::<Validated<Vec<()>, String>>()
                .is_good()
        })
}

pub fn generate_datetime(spec: &DateTimeSpecification, max_attempts: u16) -> Option<String> {
    if let Some(val) = &spec.specification.value {
        return spec.get(val).ok();
    }

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
            return match spec.specification.one_of.as_ref() {
                Some(vals) => vals
                    .get(rng.gen_range(0..vals.len()))
                    .unwrap_or(&String::default())
                    .clone(),
                None => {
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

                    return chrono::NaiveDateTime::default()
                        .with_year(year)
                        .and_then(|d| d.with_month(month))
                        .and_then(|d| d.with_day(day))
                        .map(|d| Local.from_local_datetime(&d))
                        .map(|d| spec.time_to_str(&d.unwrap()))
                        .unwrap_or_default();
                }
            };
        })
        .find(|date_str| {
            spec.check(date_str, &|_e, _a| "".to_string())
                .into_iter()
                .collect::<Validated<Vec<()>, String>>()
                .is_good()
        })
}

pub fn generate_name(spec: &NameSpecification, max_attempts: u16) -> Option<String> {
    let rng = RNG::from(&Language::Fantasy);
    (0..max_attempts)
        .map(|_| {
            return match spec.specification.specification.one_of.as_ref() {
                Some(_) => generate_string(&spec.specification, max_attempts).unwrap_or_default(),
                None => rng.generate_name(),
            };
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

pub fn generate_value_from_schema(
    schema: &DatumSchema,
    max_attempts: u16,
) -> Option<serde_json::Value> {
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
                .unwrap_or(&&StringSpecification::default()),
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
        DatumSchema::EmailSpecification {
            specification: email,
        } => generate_email(
            email.as_ref().unwrap_or(&EmailSpecification::default()),
            max_attempts,
        )
        .map(serde_json::Value::from),
        DatumSchema::List { schema } => Some(serde_json::Value::Array(
            schema
                .as_ref()
                .map(|s| {
                    (0..3)
                        .filter_map(|_| generate_value_from_schema(s, max_attempts))
                        .collect::<Vec<Value>>()
                })
                .unwrap_or_default(),
        )),
        DatumSchema::Object { schema } => {
            let f = schema
                .as_ref()
                .map(|s| {
                    s.iter()
                        .filter_map(|(k, v)| {
                            let ret = generate_value_from_schema(v, max_attempts);
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
        let spec = Specification::<u16> {
            value: None,
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
    fn specification_val_checker() {
        let spec = Specification::<u16> {
            value: Some(12),
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
    fn specification_one_of_checker() {
        let spec = Specification::<u16> {
            value: None,
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
    fn numeric_specification_min_checker() {
        let spec = NumericSpecification::<u16> {
            specification: Specification::default(),
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
            specification: Specification::default(),
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
            specification: Specification::default(),
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
            specification: Specification::default(),
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
    fn specification_errors_accumulate() {
        let spec = NumericSpecification::<u16> {
            specification: Specification {
                value: Some(1),
                one_of: Some(vec![1, 2, 4]),
                none_of: Some(vec![101]),
            },
            min: Some(200),
            max: Some(100),
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
                .get()
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
    fn datum_date_type_validation() {
        assert_eq!(
            true,
            DatumSchema::Date {
                specification: None
            }
            .check(&serde_json::json!({}), &|_e, _a| "".to_string())
            .into_iter()
            .collect::<Validated<Vec<()>, String>>()
            .is_fail(),
        );

        assert_eq!(
            false,
            DatumSchema::Date {
                specification: None
            }
            .check(&serde_json::json!("2024-12-08"), &|_e, _a| "".to_string())
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
                        specification: Some(StringSpecification {
                            specification: Specification {
                                value: None,
                                one_of: Some(vec!["foo".to_string(), "bar".to_string()]),
                                none_of: None,
                            },
                            min_length: None,
                            max_length: None,
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
        let foo = serde_json::json!({ "bodySchema" : {
            "type" : "object"
        }});

        let again: UnvalidatedResponse = serde_json::from_value(foo).unwrap();

        assert_eq!(
            true,
            match again.body.unwrap() {
                BodyOrSchema::Schema(..) => true,
                BodyOrSchema::Body(..) => false,
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
                BodyOrSchema::Schema(..) => false,
                BodyOrSchema::Body(..) => true,
            }
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
            specification: Specification {
                none_of: Some(vec!["foo".to_string(), "bar".to_string()]),
                ..Default::default()
            },
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
            .check(&val.unwrap(), &|_e, _a| "".to_string())
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
}
