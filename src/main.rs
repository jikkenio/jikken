mod config;
mod errors;
mod logger;
mod test_definition;
mod test_file;
mod test_runner;

use chrono::Local;
use log::{error, info, Level, LevelFilter};
use std::collections::HashMap;
use std::error::Error;
use std::path::Path;
use std::{env, fs};
use test_definition::TestDefinition;
use walkdir::{DirEntry, WalkDir};

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

fn get_config(file: &str) -> Result<config::Config, Box<dyn Error>> {
    let data = fs::read_to_string(file)?;
    let config: config::Config = toml::from_str(&data)?;
    Ok(config)
}

fn get_file_with_modifications(file: &str, config_opt: Option<config::Config>) -> Option<String> {
    let original_data_opt = fs::read_to_string(file);

    let mut built_in_globals = HashMap::new();

    built_in_globals.insert("#TODAY#", format!("{}", Local::now().format("%Y-%m-%d")));

    match original_data_opt {
        Ok(original_data) => {
            if let Some(config) = config_opt {
                if let Some(globals) = config.globals.as_ref() {
                    let mut modified_data = original_data;
                    for (key, value) in globals {
                        let key_pattern = format!("#{}#", key);
                        modified_data = modified_data.replace(&key_pattern, value);
                    }

                    for (key, value) in built_in_globals {
                        modified_data = modified_data.replace(key, value.as_str());
                    }
                    return Some(modified_data);
                }
            }
            Some(original_data)
        }
        Err(err) => {
            println!("error loading file: {}", err);
            None
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let args: Vec<String> = env::args().collect();
    // TODO: Separate config class from config file deserialization class
    // TODO: Add support for arguments for extended functionality
    let mut config: Option<config::Config> = None;
    let mut runner = test_runner::TestRunner::new();

    let tag_pattern_opt = if let Some(p) = args.iter().position(|a| a == "-t") {
        Some(args[p + 1].to_lowercase())
    } else {
        None
    };

    let log_level = if args.contains(&String::from("-v")) {
        Level::Trace
    } else {
        Level::Error
    };

    let my_logger = logger::SimpleLogger { level: log_level };

    if let Err(e) = log::set_boxed_logger(Box::new(my_logger)) {
        error!("Error creating logger: {}", e);
        panic!("unable to create logger");
    }

    log::set_max_level(LevelFilter::Trace);

    if Path::new(".jikken").exists() {
        let config_raw = get_config(".jikken");
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

    let files = get_files();
    info!("Jikken found {} tests.", files.len());

    let mut continue_on_failure = false;

    if let Some(ref c) = config {
        if let Some(settings) = c.settings.as_ref() {
            if let Some(cof) = settings.continue_on_failure {
                continue_on_failure = cof;
            }
        }
    }

    let total_count = match tag_pattern_opt {
        Some(_) => 0,
        None => files.len(),
    };

    for (i, file) in files.iter().enumerate() {
        let file_opt = get_file_with_modifications(file, config.clone());
        if let Some(f) = file_opt {
            let file_opt: Result<test_file::UnvalidatedTest, serde_yaml::Error> =
                serde_yaml::from_str(&f);
            match file_opt {
                Ok(test_file) => {
                    let td_opt = test_definition::TestDefinition::new(test_file);

                    match td_opt {
                        Ok(td) => {
                            if !td.validate() {
                                error!("Invalid Test Definition File: {}", file);
                                continue;
                            }

                            if let Some(tag) = tag_pattern_opt.as_ref() {
                                if !td.tags.contains(&tag) {
                                    continue;
                                }
                            }

                            let boxed_td: Box<TestDefinition> = Box::from(td);

                            // td.process_variables();
                            for iteration in 0..boxed_td.iterate {
                                match runner
                                    .run(boxed_td.as_ref(), i + 1, total_count, iteration + 1)
                                    .await
                                {
                                    Ok(passed) => {
                                        if !continue_on_failure && !passed {
                                            std::process::exit(1);
                                        }
                                    }
                                    Err(e) => {
                                        error!("Test failed to run: {}", e)
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            error!("Test Definition parsing error: {}", e);
                        }
                    }
                }
                Err(e) => {
                    error!("File parsing error: {}, ({})", e, file);
                }
            }
        } else {
            error!("file failed to load"); // TODO: Add meaningful output
        }
    }

    Ok(())
}
