use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(Deserialize, Clone, Debug)]
pub struct Config {
    pub settings: Option<Settings>,
    pub globals: Option<BTreeMap<String, String>>,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub continue_on_failure: Option<bool>,
    pub api_key: Option<String>,
    pub environment: Option<String>
}
