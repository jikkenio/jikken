mod config;
mod errors;
mod json_extractor;
mod json_filter;
mod logger;
mod test_definition;
mod test_file;
mod test_runner;

use chrono::Local;
use clap::Parser;
use hyper::{body, Body, Client, Request};
use hyper_tls::HttpsConnector;
use log::{error, info, trace, Level, LevelFilter};
use remove_dir_all::remove_dir_all;
use self_update;
use serde::Deserialize;
use std::collections::HashMap;
use std::collections::HashSet;
use std::error::Error;
use std::io::Cursor;
use std::io::{stdout, Write};
use std::path::Path;
use std::{env, fs};
use tempfile;
use test_definition::TestDefinition;
use test_definition::TestVariable;
use tokio::io::AsyncWriteExt;
use walkdir::{DirEntry, WalkDir};

const UPDATE_URL: &str = "https://api.jikken.io/v1/latest_version";
const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long = "tag", name = "tag")]
    tags: Vec<String>,

    #[arg(long, default_value_t = false)]
    tags_or: bool,

    #[arg(short, long, default_value_t = false)]
    verbose: bool,

    #[arg(short, long, default_value_t = false)]
    dry_run: bool,

    #[arg(short, long, default_value_t = false)]
    update: bool,
}

#[derive(Deserialize)]
struct ReleaseResponse {
    version: String,
    url: String,
}

// TODO: Add ignore and filter out hidden etc
fn is_jkt(entry: &DirEntry) -> bool {
    entry
        .file_name()
        .to_str()
        .map(|s| entry.file_type().is_dir() || s.ends_with(".jkt"))
        .unwrap_or(false)
}

fn get_files() -> Vec<String> {
    let mut results = Vec::new();

    WalkDir::new(".")
        .into_iter()
        .filter_entry(is_jkt)
        .filter_map(|v| v.ok())
        .filter(|x| !x.file_type().is_dir())
        .for_each(|x| results.push(String::from(x.path().to_str().unwrap())));

    results
}

async fn get_config(file: &str) -> Result<config::Config, Box<dyn Error>> {
    let data = tokio::fs::read_to_string(file).await?;
    let config: config::Config = toml::from_str(&data)?;
    Ok(config)
}

fn apply_config_envvars(config: Option<config::Config>) -> Option<config::Config> {
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

    let mut result_settings = config::Settings {
        continue_on_failure: None,
        api_key: None,
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
        } else {
            result_settings.continue_on_failure = envvar_cof;
            result_settings.api_key = envvar_apikey;
        }

        return Some(config::Config {
            settings: Some(result_settings),
            globals: c.globals,
        });
    }

    Some(config::Config {
        settings: Some(config::Settings {
            continue_on_failure: envvar_cof,
            api_key: envvar_apikey,
        }),
        globals: None,
    })
}

fn generate_global_variables(config_opt: Option<config::Config>) -> Vec<TestVariable> {
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
        .map(|i| TestVariable {
            name: i.0.to_string(),
            value: serde_yaml::Value::String(i.1),
            data_type: test_definition::VariableTypes::String,
            modifier: None,
            format: None,
        })
        .collect()
}

async fn update(url: &str) -> Result<(), Box<dyn Error + Send + Sync>> {
    print!("Jikken is updating to the latest version...");
    stdout().flush().unwrap();

    let file_name_opt = url.split("/").last();

    if file_name_opt.is_none() {
        println!("error: invalid url");
        return Ok(());
    }

    let tmp_dir = tempfile::Builder::new().tempdir_in(::std::env::current_dir()?)?;

    let tmp_tarball_path = tmp_dir.path().join(file_name_opt.unwrap());

    let mut tmp_tarball = tokio::fs::File::create(&tmp_tarball_path).await?;
    let response = reqwest::get(url).await?;
    let mut content = Cursor::new(response.bytes().await?);
    let save_file_reuslts = tmp_tarball.write_all_buf(&mut content).await;

    if let Err(error) = save_file_reuslts {
        println!("error saving downloaded file: {}", error);
        return Ok(());
    }

    if env::consts::OS == "windows" {
        self_update::Extract::from_source(&tmp_tarball_path)
            .archive(self_update::ArchiveKind::Zip)
            .extract_into(&tmp_dir.path())?;
    } else {
        self_update::Extract::from_source(&tmp_tarball_path)
            .archive(self_update::ArchiveKind::Tar(Some(
                self_update::Compression::Gz,
            )))
            .extract_into(&tmp_dir.path())?;
    }

    let tmp_file = tmp_dir.path().join("replacement_tmp");
    let bin_path = match env::consts::OS {
        "windows" => tmp_dir.path().join("jk.exe"),
        _ => tmp_dir.path().join("jk"),
    };
    self_update::Move::from_source(&bin_path)
        .replace_using_temp(&tmp_file)
        .to_dest(&::std::env::current_exe()?)?;

    drop(tmp_tarball);
    _ = remove_dir_all(tmp_dir);

    Ok(())
}

fn has_newer_version(new_version: String) -> bool {
    let new_version_segments: Vec<&str> = new_version.split(".").collect();
    let my_version_segments: Vec<&str> = VERSION.split(".").collect();

    let segment_length = std::cmp::min(new_version_segments.len(), my_version_segments.len());

    for i in 0..segment_length {
        let new_segment_opt = new_version_segments[i].parse::<u32>();
        let my_segment_opt = my_version_segments[i].parse::<u32>();

        if new_segment_opt.is_err() || my_segment_opt.is_err() {
            return false;
        } else {
            if new_segment_opt.unwrap() > my_segment_opt.unwrap() {
                return true;
            }
        }
    }

    false
}

async fn check_for_updates() -> Option<ReleaseResponse> {
    let client = Client::builder().build::<_, Body>(HttpsConnector::new());
    let req_opt = Request::builder()
        .uri(format!(
            "{}?channel=stable&platform={}",
            UPDATE_URL,
            env::consts::OS
        ))
        .body(Body::empty());
    match req_opt {
        Ok(req) => {
            let resp_opt = client.request(req).await;
            match resp_opt {
                Ok(resp) => {
                    let (_, body) = resp.into_parts();
                    let response_bytes_opt = body::to_bytes(body).await;
                    match response_bytes_opt {
                        Ok(response_bytes) => {
                            match serde_json::from_slice::<ReleaseResponse>(
                                &response_bytes.to_vec(),
                            ) {
                                Ok(r) => {
                                    if has_newer_version(r.version.clone()) {
                                        return Some(r);
                                    }
                                }
                                Err(error) => {
                                    trace!("unable to deserialize response: {}", error)
                                }
                            }
                        }
                        Err(error) => {
                            trace!("unable to read response from update server: {}", error)
                        }
                    }
                }
                Err(error) => {
                    trace!("unable to contact update server: {}", error)
                }
            }
        }
        Err(error) => {
            trace!("unable to contact update server: {}", error)
        }
    }

    None
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let args = Args::parse();
    // TODO: Separate config class from config file deserialization class
    // TODO: Add support for arguments for extended functionality
    let mut config: Option<config::Config> = None;
    let mut runner = test_runner::TestRunner::new();

    let log_level = if args.verbose {
        Level::Trace
    } else {
        Level::Info
    };

    let my_logger = logger::SimpleLogger { level: log_level };

    if let Err(e) = log::set_boxed_logger(Box::new(my_logger)) {
        error!("Error creating logger: {}", e);
        panic!("unable to create logger");
    }

    log::set_max_level(LevelFilter::Trace);

    if Path::new(".jikken").exists() {
        let config_raw = get_config(".jikken").await;
        match config_raw {
            Ok(c) => {
                config = Some(c);
            }
            Err(e) => {
                error!("invalid configuration file: {}", e);
                std::process::exit(exitcode::CONFIG);
            }
        }
    }
    config = apply_config_envvars(config);

    let latest_version_opt = check_for_updates().await;

    if !args.update {
        if let Some(lv) = latest_version_opt {
            info!(
                "\x1b[33mJikken found new version ({}), currently running version ({})\x1b[0m",
                lv.version, VERSION
            );
            info!("\x1b[33mRun command: `jk --update` to update jikken or update using your package manager\x1b[0m");
        }
    } else {
        if let Some(lv) = latest_version_opt {
            match update(&lv.url).await {
                Ok(_) => {
                    info!("update completed");
                    std::process::exit(0);
                }
                Err(error) => {
                    error!(
                        "Jikken encountered an error when trying to update itself: {}",
                        error
                    );
                }
            }
        } else {
            error!("Jikken was unable to find an update for this platform and release channel");
        }
        std::process::exit(1);
    }

    let files = get_files();
    let mut continue_on_failure = false;

    info!("Jikken found {} tests", files.len());

    if let Some(c) = config.as_ref() {
        if let Some(settings) = c.settings.as_ref() {
            if let Some(cof) = settings.continue_on_failure {
                continue_on_failure = cof;
            }
        }
    }

    let global_variables = generate_global_variables(config);
    let mut tests_to_ignore: Vec<TestDefinition> = Vec::new();
    let mut tests_to_run: Vec<TestDefinition> = files
        .iter()
        .map(|f| (f, fs::read_to_string(f)))
        .filter_map(|(filename, f)| match f {
            Ok(file_data) => {
                let result: Result<test_file::UnvalidatedTest, serde_yaml::Error> =
                    serde_yaml::from_str(&file_data);
                match result {
                    Ok(file) => Some(file),
                    Err(e) => {
                        println!("unable to parse file ({}) data: {}", filename, e);
                        None
                    }
                }
            }
            Err(err) => {
                println!("error loading file: {}", err);
                None
            }
        })
        .filter_map(|f| {
            let result = TestDefinition::new(f, global_variables.clone());
            match result {
                Ok(td) => {
                    if !td.validate() {
                        error!(
                            "test failed validation: {}",
                            td.name.unwrap_or("unnamed test".to_string())
                        );
                        None
                    } else {
                        if args.tags.len() > 0 {
                            let td_tags: HashSet<String> = HashSet::from_iter(td.clone().tags);
                            if args.tags_or {
                                for t in args.tags.iter() {
                                    if td_tags.contains(t) {
                                        return Some(td);
                                    }
                                }

                                tests_to_ignore.push(td.clone());

                                trace!(
                                    "test `{}` doesn't match any tags: {}",
                                    td.name.unwrap_or("".to_string()),
                                    args.tags.join(", ")
                                );

                                return None;
                            } else {
                                for t in args.tags.iter() {
                                    if !td_tags.contains(t) {
                                        tests_to_ignore.push(td.clone());

                                        trace!(
                                            "test `{}` is missing tag: {}",
                                            td.name.unwrap_or("".to_string()),
                                            t
                                        );
                                        return None;
                                    }
                                }
                            }
                        }

                        Some(td)
                    }
                }
                Err(e) => {
                    trace!("test definition creation failed: {}", e);
                    None
                }
            }
        })
        .collect();

    let tests_by_id: HashMap<String, TestDefinition> = tests_to_run
        .clone()
        .into_iter()
        .chain(tests_to_ignore.into_iter())
        .map(|td| (td.id.clone(), td))
        .collect();

    tests_to_run.sort_by(|a, b| a.name.partial_cmp(&b.name).unwrap());

    let mut duplicate_filter: HashSet<String> = HashSet::new();

    let mut tests_to_run_with_dependencies: Vec<TestDefinition> = Vec::new();

    for td in tests_to_run.into_iter() {
        match &td.requires {
            Some(req) => {
                if tests_by_id.contains_key(req) {
                    if !duplicate_filter.contains(req) {
                        duplicate_filter.insert(req.clone());
                        tests_to_run_with_dependencies
                            .push(tests_by_id.get(req).unwrap().to_owned());
                    }
                }
            }
            _ => {}
        }

        if !duplicate_filter.contains(&td.id) {
            duplicate_filter.insert(td.id.clone());
            tests_to_run_with_dependencies.push(td);
        }
    }

    let total_count = tests_to_run_with_dependencies.len();

    for (i, td) in tests_to_run_with_dependencies.into_iter().enumerate() {
        let boxed_td: Box<TestDefinition> = Box::from(td);

        for iteration in 0..boxed_td.iterate {
            let passed = if args.dry_run {
                runner
                    .dry_run(boxed_td.as_ref(), i, total_count, iteration)
                    .await
            } else {
                runner
                    .run(boxed_td.as_ref(), i, total_count, iteration)
                    .await
            };

            if !continue_on_failure && !passed {
                std::process::exit(1);
            }
        }
    }

    Ok(())
}
