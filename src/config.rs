use std::collections::BTreeMap;
use serde::Deserialize;
use super::config_settings;

#[derive(Deserialize, Clone, Debug)]
pub struct Config {
    pub settings: Option<config_settings::Settings>,
    pub globals: Option<BTreeMap<String, String>>,
}