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
use log::{error, info, trace, Level, LevelFilter};
use std::collections::HashMap;
use std::collections::HashSet;
use std::error::Error;
use std::fs;
use std::path::Path;
use test_definition::TestDefinition;
use test_definition::TestVariable;
use walkdir::{DirEntry, WalkDir};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long = "tag", name = "tag")]
    tags: Vec<String>,

    #[arg(long, default_value_t = false)]
    tags_or: bool,

    #[arg(short, long, default_value_t = false)]
    verbose: bool,
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

fn get_config(file: &str) -> Result<config::Config, Box<dyn Error>> {
    let data = fs::read_to_string(file)?;
    let config: config::Config = toml::from_str(&data)?;
    Ok(config)
}

fn generate_global_variables(config_opt: Option<config::Config>) -> Vec<TestVariable> {
    let mut global_variables = HashMap::new();
    global_variables.insert("TODAY".to_string(), format!("{}", Local::now().format("%Y-%m-%d")));

    if let Some(config) = config_opt {
        if let Some(globals) = config.globals {
            for (key, value) in globals.into_iter() {
                global_variables.insert(key, value.clone());
            }
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

    let global_variables = generate_global_variables(config);

    let mut tests_to_ignore: Vec<TestDefinition> = Vec::new();
    let mut tests_to_run: Vec<TestDefinition> = files
        .iter()
        .map(|f| fs::read_to_string(f))
        .filter_map(|f| match f {
            Ok(file_data) => {
                let result: Result<test_file::UnvalidatedTest, serde_yaml::Error> =
                    serde_yaml::from_str(&file_data);
                match result {
                    Ok(file) => Some(file),
                    Err(e) => {
                        trace!("unable to parse file data: {}", e);
                        None
                    }
                }
            }
            Err(err) => {
                println!("error loading file: {}", err);
                None
            },
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
        // let json_string = serde_json::to_string(&td)?;
        // println!("json: {}", json_string);
        let boxed_td: Box<TestDefinition> = Box::from(td);

        for iteration in 0..boxed_td.iterate {
            let passed = runner
                .run(boxed_td.as_ref(), i, total_count, iteration)
                .await;

            if !continue_on_failure && !passed {
                std::process::exit(1);
            }
        }
    }

    Ok(())
}
