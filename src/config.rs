use crate::test;
use chrono::Local;
use log::error;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::env;
use std::path::Path;

#[derive(PartialEq, Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    pub settings: Settings,
    pub globals: BTreeMap<String, String>,
}

#[derive(PartialEq, Serialize, Deserialize, Clone, Debug)]
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

impl Default for Config {
    fn default() -> Config {
        Config {
            settings: Settings {
                continue_on_failure: false,
                api_key: None,
                environment: None,
            },
            globals: BTreeMap::new(),
        }
    }
}

pub async fn get_config() -> Config {
    let config_sources_ascending_priority = vec![
        load_home_file().await,
        load_config_file(".jikken").await,
        Some(load_config_from_environment_variables()),
    ];

    return get_config_impl(config_sources_ascending_priority);
}

fn get_config_impl(config_sources_ascending_priority: Vec<Option<File>>) -> Config {
    return config_sources_ascending_priority
        .into_iter()
        .fold(Config::default(), apply_config_file);
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
    let cfg_file = dirs::home_dir().map(|pb| pb.join(".jikken"));
    return load_config_file(cfg_file?.as_path().to_str()?).await;
}

//Treat environment as just another config file
fn load_config_from_environment_variables() -> File {
    let envvar_cof = env::var("JIKKEN_CONTINUE_ON_FAILURE")
        .ok()
        .and_then(|cfg| cfg.parse::<bool>().ok());

    let envvar_apikey = env::var("JIKKEN_API_KEY").ok();

    let envvar_env = env::var("JIKKEN_ENVIRONMENT").ok();

    let mut global_variables = BTreeMap::new();

    for (key, value) in env::vars() {
        if let Some(stripped) = key.strip_prefix("JIKKEN_GLOBAL_") {
            global_variables.insert(stripped.to_string(), value);
        }
    }

    return File {
        settings: Some(FileSettings {
            api_key: envvar_apikey,
            continue_on_failure: envvar_cof,
            environment: envvar_env,
        }),
        globals: Some(global_variables),
    };
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

#[cfg(test)]
mod tests {
    use super::*;
    use {std::fs::OpenOptions, std::io::Write, tempfile::tempdir};

    #[test]
    fn no_overrides_yields_default_config() {
        let sources: Vec<Option<File>> = vec![None, None];
        let actual = get_config_impl(sources);
        assert_eq!(Config::default(), actual);
    }

    #[tokio::test]
    async fn one_overrides_yields_correct_combination() {
        let tmp_dir = tempdir().unwrap();
        let tmp_path = tmp_dir.path();
        let override_file_path = tmp_path.join("foo.jikken");
        let override_file_path_str = override_file_path.to_str().unwrap();
        _ = std::fs::File::create(override_file_path_str);
        let mut f = OpenOptions::new()
            .write(true)
            .open(override_file_path_str)
            .expect("create failed");
        f.write_all(
            r#"
            [settings]
            continueOnFailure=true
            
            [globals]
            my_override_global="foo"
            "#
            .as_bytes(),
        )
        .unwrap();
        let sources: Vec<Option<File>> = vec![
            load_config_file(override_file_path.to_str().unwrap()).await,
            None,
        ];
        let actual = get_config_impl(sources);
        assert_eq!(
            Config {
                settings: Settings {
                    continue_on_failure: true,
                    api_key: None,
                    environment: None,
                },
                globals: BTreeMap::from([(
                    String::from("my_override_global"),
                    String::from("foo")
                )])
            },
            actual
        );
    }

    #[tokio::test]
    async fn two_overrides_yields_correct_combination() {
        let tmp_dir = tempdir().unwrap();
        let tmp_path = tmp_dir.path();
        let override_file_path = tmp_path.join("foo.jikken");
        let override_file_path_str = override_file_path.to_str().unwrap();
        let override_file_path2 = tmp_path.join("foo2.jikken");
        let override_file_path_str2 = override_file_path2.to_str().unwrap();

        _ = std::fs::File::create(override_file_path_str);
        let mut f = OpenOptions::new()
            .write(true)
            .open(override_file_path_str)
            .expect("create failed");
        f.write_all(
            r#"
            [settings]
            continueOnFailure=true
            apiKey="key"
            
            [globals]
            my_override_global="foo"
            my_override_global2="bar"
            "#
            .as_bytes(),
        )
        .unwrap();

        _ = std::fs::File::create(override_file_path_str2);
        f = OpenOptions::new()
            .write(true)
            .open(override_file_path_str2)
            .expect("create failed");
        f.write_all(
            r#"
            [settings]
            continueOnFailure=false
            environment="magic"

            [globals]
            my_override_global="bar"
            my_override_global3="car"
            "#
            .as_bytes(),
        )
        .unwrap();

        let sources: Vec<Option<File>> = vec![
            load_config_file(override_file_path_str).await,
            load_config_file(override_file_path_str2).await,
        ];
        let actual = get_config_impl(sources);
        assert_eq!(
            Config {
                settings: Settings {
                    continue_on_failure: false,
                    api_key: Some(String::from("key")),
                    environment: Some(String::from("magic")),
                },
                globals: BTreeMap::from([
                    (String::from("my_override_global"), String::from("bar")),
                    (String::from("my_override_global2"), String::from("bar")),
                    (String::from("my_override_global3"), String::from("car"))
                ]),
            },
            actual
        );
    }
} // mod tests
