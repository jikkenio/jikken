mod config;
mod errors;
mod executor;
mod json;
mod logger;
mod test;

use chrono::Local;
use clap::{Parser, Subcommand};
use config::{Config, Settings};
use executor::runner::TestRunner;
use hyper::{body, Body, Client, Request};
use hyper_tls::HttpsConnector;
use log::{debug, error, info, trace, warn, Level, LevelFilter};
use logger::SimpleLogger;
use remove_dir_all::remove_dir_all;
use self_update;
use serde::Deserialize;
use std::collections::HashMap;
use std::collections::HashSet;
use std::env;
use std::error::Error;
use std::io::Cursor;
use std::io::{stdout, Write};
use std::path::Path;
use tempfile;
use test::definition::TestDefinition;
use test::definition::TestVariable;
use test::file::TestFile;
use test::templates::{template, template_full, template_staged};
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use walkdir::{DirEntry, WalkDir};

const UPDATE_URL: &str = "https://api.jikken.io/v1/latest_version";
const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// The environment flag can be used to indicate which environment{n}the tests are executing against.
    /// This is not used unless tests{n}are reporting to the Jikken.IO platform via an API Key{n}
    #[arg(short, long = "env", name = "env")]
    environment: Option<String>,

    /// Quiet mode suppresses all console output
    #[arg(short, long, default_value_t = false)]
    quiet: bool,

    /// Verbose mode provides more detailed console output
    #[arg(short, long, default_value_t = false)]
    verbose: bool,

    /// Trace mode provides significant console output
    #[arg(long, default_value_t = false)]
    trace: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Execute tests
    Run {
        #[arg(short, long = "tag", name = "tag")]
        tags: Vec<String>,

        #[arg(long, default_value_t = false)]
        tags_or: bool,
    },

    /// Process tests without calling api endpoints
    #[command(name = "dryrun")]
    DryRun {
        #[arg(short, long = "tag", name = "tag")]
        tags: Vec<String>,

        #[arg(long, default_value_t = false)]
        tags_or: bool,
    },
    /// Jikken updates itself if a newer version exists
    Update,
    /// Create a new test
    New {
        /// Generates a test template with all options
        #[arg(short, long = "full", name = "full")]
        full: bool,

        /// Generates a multi-stage test template
        #[arg(short = 'm', long = "multistage", name = "multistage")]
        multistage: bool,

        /// Output to console instead of saving to a file
        #[arg(short = 'o')]
        output: bool,

        /// The file name to create
        name: Option<String>,
    },
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

async fn get_config(file: &str) -> Result<Config, Box<dyn Error>> {
    let data = tokio::fs::read_to_string(file).await?;
    let config: Config = toml::from_str(&data)?;
    Ok(config)
}

fn apply_config_envvars(config: Option<Config>) -> Option<Config> {
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

fn generate_global_variables(config_opt: Option<Config>) -> Vec<TestVariable> {
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
            data_type: test::definition::VariableTypes::String,
            modifier: None,
            format: None,
        })
        .collect()
}

async fn update(url: &str) -> Result<(), Box<dyn Error + Send + Sync>> {
    info!("Jikken is updating to the latest version...");
    stdout().flush().unwrap();

    let file_name_opt = url.split("/").last();

    if file_name_opt.is_none() {
        error!("error: invalid url");
        return Ok(());
    }

    let tmp_dir = tempfile::Builder::new().tempdir_in(::std::env::current_dir()?)?;

    let tmp_tarball_path = tmp_dir.path().join(file_name_opt.unwrap());

    let mut tmp_tarball = tokio::fs::File::create(&tmp_tarball_path).await?;
    let response = reqwest::get(url).await?;
    let mut content = Cursor::new(response.bytes().await?);
    let save_file_reuslts = tmp_tarball.write_all_buf(&mut content).await;

    if let Err(error) = save_file_reuslts {
        error!("error saving downloaded file: {}", error);
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

async fn check_for_updates() -> Result<Option<ReleaseResponse>, Box<dyn Error + Send + Sync>> {
    let client = Client::builder().build::<_, Body>(HttpsConnector::new());
    let req = Request::builder()
        .uri(format!(
            "{}?channel=stable&platform={}",
            UPDATE_URL,
            env::consts::OS
        ))
        .body(Body::empty())?;

    let resp = client.request(req).await?;
    let (_, body) = resp.into_parts();
    let response_bytes = body::to_bytes(body).await?;
    if let Ok(r) = serde_json::from_slice::<ReleaseResponse>(&response_bytes.to_vec()) {
        if has_newer_version(r.version.clone()) {
            return Ok(Some(r));
        }
    }

    Ok(None)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let cli = Cli::parse();
    // TODO: Separate config class from config file deserialization class
    // TODO: Add support for arguments for extended functionality
    let mut config: Option<Config> = None;
    let mut runner = TestRunner::new();

    let mut cli_tags = &Vec::new();
    let mut cli_tags_or = false;

    match &cli.command {
        Commands::Run { tags, tags_or } | Commands::DryRun { tags, tags_or } => {
            cli_tags = tags;
            cli_tags_or = *tags_or;
        }
        _ => {}
    };

    let log_level = if cli.verbose {
        Level::Debug
    } else if cli.trace {
        Level::Trace
    } else {
        Level::Info
    };

    let my_logger = SimpleLogger {
        level: log_level,
        disabled: cli.quiet,
    };

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

    match cli.command {
        Commands::Update => {
            match latest_version_opt {
                Ok(lv_opt) => {
                    if let Some(lv) = lv_opt {
                        match update(&lv.url).await {
                            Ok(_) => {
                                info!("update completed\n");
                                std::process::exit(0);
                            }
                            Err(error) => {
                                error!(
                                    "Jikken encountered an error when trying to update itself: {}",
                                    error
                                );
                            }
                        }
                    }
                }
                Err(error) => {
                    debug!("error checking for updates: {}", error);
                }
            }

            error!("Jikken was unable to find an update for this platform and release channel");
            std::process::exit(0);
        }
        _ => match latest_version_opt {
            Ok(lv_opt) => {
                if let Some(lv) = lv_opt {
                    warn!(
                        "Jikken found new version ({}), currently running version ({})",
                        lv.version, VERSION
                    );
                    warn!("Run command: `jk --update` to update jikken or update using your package manager");
                }
            }
            Err(error) => {
                debug!("error checking for updates: {}", error);
            }
        },
    }

    match &cli.command {
        Commands::New {
            full,
            multistage,
            output,
            name,
        } => {
            let template = if *full {
                serde_yaml::to_string(&template_full()?)?
            } else if *multistage {
                serde_yaml::to_string(&template_staged()?)?
            } else {
                serde_yaml::to_string(&template()?)?
            };
            let template = template.replace("''", "");
            let mut result = "".to_string();

            for line in template.lines() {
                if !line.contains("null") {
                    result = format!("{}{}\n", result, line)
                }
            }

            if *output {
                info!("{}\n", result);
            } else {
                match name {
                    Some(n) => {
                        let filename = if !n.ends_with(".jkt") {
                            format!("{}.jkt", n)
                        } else {
                            n.clone()
                        };

                        if std::path::Path::new(&filename).exists() {
                            error!("`{}` already exists. Please pick a new name/location or delete the existing file.", filename);
                            std::process::exit(1);
                        }

                        let mut file = File::create(&filename).await?;
                        file.write_all(result.as_bytes()).await?;
                        info!("Successfully created test (`{}`).\n", filename);
                        std::process::exit(0);
                    }
                    None => {
                        error!("<NAME> is required if not outputting to screen. `jk new <NAME>`");
                        std::process::exit(1);
                    }
                }
            }

            std::process::exit(0);
        }
        _ => {}
    }

    let files = get_files();
    let mut continue_on_failure = false;

    info!("Jikken found {} tests\n", files.len());

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
        .filter_map(|filename| {
            let result = TestFile::load(filename);
            match result {
                Ok(file) => Some(file),
                Err(e) => {
                    error!("unable to load test file ({}) data: {}", filename, e);
                    None
                }
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
                        if cli_tags.len() > 0 {
                            let td_tags: HashSet<String> = HashSet::from_iter(td.clone().tags);
                            if cli_tags_or {
                                for t in cli_tags.iter() {
                                    if td_tags.contains(t) {
                                        return Some(td);
                                    }
                                }

                                tests_to_ignore.push(td.clone());

                                debug!(
                                    "test `{}` doesn't match any tags: {}",
                                    td.name.unwrap_or("".to_string()),
                                    cli_tags.join(", ")
                                );

                                return None;
                            } else {
                                for t in cli_tags.iter() {
                                    if !td_tags.contains(t) {
                                        tests_to_ignore.push(td.clone());

                                        debug!(
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
                    error!("test definition creation failed: {}", e);
                    None
                }
            }
        })
        .collect();

    if tests_to_ignore.len() > 0 {
        trace!("filtering out tests which don't match the tag pattern")
    }

    let tests_by_id: HashMap<String, TestDefinition> = tests_to_run
        .clone()
        .into_iter()
        .chain(tests_to_ignore.into_iter())
        .map(|td| (td.id.clone(), td))
        .collect();

    tests_to_run.sort_by(|a, b| a.name.partial_cmp(&b.name).unwrap());

    let mut duplicate_filter: HashSet<String> = HashSet::new();

    let mut tests_to_run_with_dependencies: Vec<TestDefinition> = Vec::new();

    trace!("determine test execution order based on dependency graph");

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

        let dry_run = match cli.command {
            Commands::DryRun {
                tags: _,
                tags_or: _,
            } => true,
            _ => false,
        };

        for iteration in 0..boxed_td.iterate {
            let passed = if dry_run {
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
