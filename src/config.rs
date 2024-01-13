use crate::test;
use chrono::Local;
use log::error;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::env;
use std::path::Path;

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    pub settings: Settings,
    pub globals: BTreeMap<String, String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub continue_on_failure: bool,
    pub environment: Option<String>,
    #[serde(skip_serializing)]
    pub api_key: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct File {
    pub settings: Option<FileSettings>,
    pub globals: Option<BTreeMap<String, String>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct FileSettings {
    pub continue_on_failure: Option<bool>,
    pub api_key: Option<String>,
    pub environment: Option<String>,
}

impl Config {
    pub fn generate_global_variables(&self) -> Vec<test::Variable> {
        let mut global_variables = BTreeMap::new();
        global_variables.insert(
            "TODAY".to_string(),
            format!("{}", Local::now().format("%Y-%m-%d")),
        );

        global_variables
            .iter()
            .chain(self.globals.iter())
            .map(|i| test::Variable {
                name: i.0.to_string(),
                value: serde_yaml::Value::String(i.1.to_string()),
                data_type: test::variable::Type::String,
                modifier: None,
                format: None,
                file: None,
                source_path: "./".to_string(),
            })
            .collect()
    }
}

pub async fn get_config() -> Config {
    let mut config = get_default_config();

    config = apply_config_file(config, load_home_file().await);
    config = apply_config_file(config, load_config_file(".jikken").await);

    apply_envvars(config)
}

async fn load_config_file(file: &str) -> Option<File> {
    if !Path::new(file).exists() {
        return None;
    }

    match tokio::fs::read_to_string(file).await {
        Ok(data) => {
            let config_result: Result<File, _> = toml::from_str(&data);
            match config_result {
                Ok(config) => Some(config),
                Err(e) => {
                    error!("unable to load config file ({}): {}", file, e);
                    None
                }
            }
        }
        Err(e) => {
            error!("unable to load config file ({}): {}", file, e);
            None
        }
    }
}

async fn load_home_file() -> Option<File> {
    let home_dir_opt = dirs::home_dir();

    if let Some(home_dir_path) = home_dir_opt {
        if let Some(home_dir) = home_dir_path.to_str() {
            let resolved_home_file = if home_dir.contains('/') {
                format!("{}/.jikken", home_dir)
            } else {
                format!("{}\\.jikken", home_dir)
            };

            return load_config_file(&resolved_home_file).await;
        }
    }

    None
}

fn apply_config_file(config: Config, file_opt: Option<File>) -> Config {
    if let Some(file) = file_opt {
        let merged_globals: BTreeMap<String, String> = config
            .globals
            .into_iter()
            .chain(file.globals.unwrap_or_default())
            .collect();

        if let Some(settings) = file.settings {
            return Config {
                settings: Settings {
                    continue_on_failure: settings
                        .continue_on_failure
                        .unwrap_or(config.settings.continue_on_failure),
                    api_key: settings.api_key.or(config.settings.api_key),
                    environment: settings.environment.or(config.settings.environment),
                },
                globals: merged_globals,
            };
        }

        return Config {
            settings: config.settings,
            globals: merged_globals,
        };
    }

    config
}

fn get_default_config() -> Config {
    Config {
        settings: Settings {
            continue_on_failure: false,
            api_key: None,
            environment: None,
        },
        globals: BTreeMap::new(),
    }
}

fn apply_envvars(config: Config) -> Config {
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

    let mut global_variables = config.globals.clone();
    for (key, value) in env::vars() {
        if let Some(stripped) = key.strip_prefix("JIKKEN_GLOBAL_") {
            global_variables.insert(stripped.to_string(), value);
        }
    }

    Config {
        settings: Settings {
            continue_on_failure: envvar_cof.unwrap_or(config.settings.continue_on_failure),
            api_key: envvar_apikey.or(config.settings.api_key),
            environment: envvar_env.or(config.settings.environment),
        },
        globals: global_variables,
    }
}
