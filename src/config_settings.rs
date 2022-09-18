use serde::Deserialize;

#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub continue_on_failure: Option<bool>, 
}