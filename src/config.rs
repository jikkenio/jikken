use std::collections::BTreeMap;
use serde::Deserialize;

#[derive(Deserialize, Clone, Debug)]
pub struct Config {
    pub globals: Option<BTreeMap<String, String>>,
}