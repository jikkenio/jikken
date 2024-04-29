use crate::test;
use chrono::{Local, Utc};
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
    pub project: Option<String>,
    pub environment: Option<String>,
    #[serde(skip_serializing)]
    pub api_key: Option<String>,
    pub dev_mode: Option<bool>,
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
    pub dev_mode: Option<bool>,
    pub project: Option<String>,
    pub environment: Option<String>,
}

impl Config {
    pub fn generate_global_variables(&self) -> Vec<test::Variable> {
        let mut global_variables = BTreeMap::new();
        global_variables.insert(
            "TODAY".to_string(),
            format!("{}", Local::now().format("%Y-%m-%d")),
        );

        global_variables.insert(
            "TODAY_UTC".to_string(),
            format!("{}", Utc::now().format("%Y-%m-%d")),
        );

        global_variables
            .iter()
            .chain(self.globals.iter())
            .map(|i| test::Variable {
                name: i.0.to_string(),
                value: test::file::StringOrDatumOrFile::Value(i.1.to_string()),
                //value: serde_yaml::Value::String(i.1.to_string()),
                //data_type: test::variable::Type::String,
                //modifier: None,
                //format: None,
                //file: None,
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
                dev_mode: None,
                project: None,
                environment: None,
            },
            globals: BTreeMap::new(),
        }
    }
}

pub async fn get_config(file: Option<String>) -> Config {
    let config_sources_ascending_priority = vec![
        load_home_file().await,
        load_config_file(file.unwrap_or(".jikken".to_string()).as_str()).await,
        Some(load_config_from_environment_variables_as_file()),
    ];

    get_config_impl(config_sources_ascending_priority)
}

fn get_config_impl(config_sources_ascending_priority: Vec<Option<File>>) -> Config {
    let specified_config = config_sources_ascending_priority
        .into_iter()
        .fold(None, combine_config_files);
    apply_config_file(Config::default(), specified_config)
}

async fn load_config_file(file: &str) -> Option<File> {
    if !Path::new(file).exists() || !Path::new(file).is_file() {
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
    load_config_file(cfg_file?.as_path().to_str()?).await
}

fn load_config_from_environment_variables_as_file() -> File {
    let envvar_cof = env::var("JIKKEN_CONTINUE_ON_FAILURE")
        .ok()
        .and_then(|cfg| cfg.parse::<bool>().ok());

    let envvar_apikey = env::var("JIKKEN_API_KEY").ok();
    let envvar_devmode = env::var("JIKKEN_DEV_MODE")
        .ok()
        .and_then(|cfg| cfg.parse::<bool>().ok());

    let envvar_project = env::var("JIKKEN_PROJECT").ok();
    let envvar_env = env::var("JIKKEN_ENVIRONMENT").ok();

    let mut global_variables = BTreeMap::new();

    for (key, value) in env::vars() {
        if let Some(stripped) = key.strip_prefix("JIKKEN_GLOBAL_") {
            global_variables.insert(stripped.to_string(), value);
        }
    }

    File {
        settings: Some(FileSettings {
            api_key: envvar_apikey,
            dev_mode: envvar_devmode,
            continue_on_failure: envvar_cof,
            project: envvar_project,
            environment: envvar_env,
        }),
        globals: Some(global_variables),
    }
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
                    dev_mode: settings.dev_mode.or(config.settings.dev_mode),
                    project: settings.project.or(config.settings.project),
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

//rhs priority
fn combine_config_files(lhs: Option<File>, rhs: Option<File>) -> Option<File> {
    match (lhs, rhs) {
        (None, None) => None,
        (Some(x), None) => Some(x),
        (None, Some(x)) => Some(x),
        (Some(existing_file), Some(file_to_apply)) => {
            let merged_globals: BTreeMap<String, String> = existing_file
                .globals
                .unwrap_or_default()
                .into_iter()
                .chain(file_to_apply.globals.unwrap_or_default())
                .collect();

            if let Some(settings) = file_to_apply.settings {
                return Some(File {
                    settings: Some(FileSettings {
                        continue_on_failure: settings.continue_on_failure.or(existing_file
                            .settings
                            .as_ref()
                            .and_then(|s| s.continue_on_failure)),
                        api_key: settings.api_key.or(existing_file
                            .settings
                            .as_ref()
                            .and_then(|s| s.api_key.clone())),
                        dev_mode: settings
                            .dev_mode
                            .or(existing_file.settings.as_ref().and_then(|s| s.dev_mode)),
                        project: settings.project.or(existing_file
                            .settings
                            .as_ref()
                            .and_then(|s| s.project.clone())),
                        environment: settings.environment.or(existing_file
                            .settings
                            .as_ref()
                            .and_then(|s| s.environment.clone())),
                    }),
                    globals: Some(merged_globals),
                });
            }

            Some(File {
                settings: existing_file.settings,
                globals: Some(merged_globals),
            })
        }
    }
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
                    dev_mode: None,
                    project: None,
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
            devMode=true
            
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
            project="my_proj"
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
                    dev_mode: Some(true),
                    project: Some(String::from("my_proj")),
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
