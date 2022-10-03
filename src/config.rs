use super::config_settings;
use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(Deserialize, Clone, Debug)]
pub struct Config {
    pub settings: Option<config_settings::Settings>,
    pub globals: Option<BTreeMap<String, String>>,
}
