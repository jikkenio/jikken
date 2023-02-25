use crate::test;
use chrono::Local;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::env;
use std::error::Error;

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
    pub environment: Option<String>,
}

pub async fn get_config(file: &str) -> Result<Config, Box<dyn Error>> {
    let data = tokio::fs::read_to_string(file).await?;
    let config: Config = toml::from_str(&data)?;
    Ok(config)
}

pub fn apply_config_envvars(config: Option<Config>) -> Option<Config> {
    let envvar_cof = if let Ok(cof) = env::var("JIKKEN_CONTINUE_ON_FAILURE") {
        if let Ok(b) = cof.parse::<bool>() {
            Some(b)
        } else {
            None
        }
    } else {
        None
    };

    let envvar_apikey = if let Ok(key) = env::var("JIKKEN_API_KEY") {
        Some(key)
    } else {
        None
    };

    let envvar_env = if let Ok(env) = env::var("JIKKEN_ENVIRONMENT") {
        Some(env)
    } else {
        None
    };

    let mut result_settings = Settings {
        continue_on_failure: None,
        api_key: None,
        environment: None,
    };

    if let Some(c) = config {
        if let Some(settings) = c.settings {
            result_settings.continue_on_failure = if envvar_cof.is_some() {
                envvar_cof
            } else {
                settings.continue_on_failure
            };

            result_settings.api_key = if envvar_apikey.is_some() {
                envvar_apikey
            } else {
                settings.api_key
            };

            result_settings.environment = if envvar_env.is_some() {
                envvar_env
            } else {
                settings.environment
            };
        } else {
            result_settings.continue_on_failure = envvar_cof;
            result_settings.api_key = envvar_apikey;
            result_settings.environment = envvar_env;
        }

        return Some(Config {
            settings: Some(result_settings),
            globals: c.globals,
        });
    }

    Some(Config {
        settings: Some(Settings {
            continue_on_failure: envvar_cof,
            api_key: envvar_apikey,
            environment: envvar_env,
        }),
        globals: None,
    })
}

pub fn generate_global_variables(config_opt: Option<Config>) -> Vec<test::Variable> {
    let mut global_variables = HashMap::new();
    global_variables.insert(
        "TODAY".to_string(),
        format!("{}", Local::now().format("%Y-%m-%d")),
    );

    if let Some(config) = config_opt {
        if let Some(globals) = config.globals {
            for (key, value) in globals.into_iter() {
                global_variables.insert(key, value.clone());
            }
        }
    }

    for (key, value) in env::vars() {
        if key.starts_with("JIKKEN_GLOBAL_") {
            global_variables.insert(key[14..].to_string(), value);
        }
    }

    global_variables
        .into_iter()
        .map(|i| test::Variable {
            name: i.0.to_string(),
            value: serde_yaml::Value::String(i.1),
            data_type: test::variable::Type::String,
            modifier: None,
            format: None,
        })
        .collect()
}
