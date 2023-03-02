mod config;
mod errors;
mod executor;
mod json;
mod logger;
mod test;
mod updater;

use clap::{Parser, Subcommand};
use executor::TestRunner;
use log::{debug, error, info, trace, warn, Level, LevelFilter};
use logger::SimpleLogger;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::error::Error;
use test::{template, validation};
use tokio::fs;
use tokio::io::AsyncWriteExt;

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Parser, Serialize, Deserialize)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
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

#[derive(Subcommand, Serialize, Deserialize)]
pub enum Commands {
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

// TODO: Add ignore and filter out hidden etc
fn is_jkt(entry: &walkdir::DirEntry) -> bool {
    entry
        .file_name()
        .to_str()
        .map(|s| entry.file_type().is_dir() || s.ends_with(".jkt"))
        .unwrap_or(false)
}

fn get_files() -> Vec<String> {
    let mut results = Vec::new();

    walkdir::WalkDir::new(".")
        .into_iter()
        .filter_entry(is_jkt)
        .filter_map(|v| v.ok())
        .filter(|x| !x.file_type().is_dir())
        .for_each(|x| results.push(String::from(x.path().to_str().unwrap())));

    results
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let cli = Cli::parse();
    // TODO: Add support for arguments for extended functionality
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

    let config = config::get_config().await;
    let latest_version_opt = updater::check_for_updates().await;

    match cli.command {
        Commands::Update => {
            match latest_version_opt {
                Ok(lv_opt) => {
                    if let Some(lv) = lv_opt {
                        match updater::update(&lv.url).await {
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
                serde_yaml::to_string(&template::template_full()?)?
            } else if *multistage {
                serde_yaml::to_string(&template::template_staged()?)?
            } else {
                serde_yaml::to_string(&template::template()?)?
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

                        let mut file = fs::File::create(&filename).await?;
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

    info!("Jikken found {} tests\n", files.len());

    let global_variables = config.generate_global_variables();
    let mut tests_to_ignore: Vec<test::Definition> = Vec::new();
    let mut tests_to_run: Vec<test::Definition> = files
        .iter()
        .filter_map(|filename| {
            let result = test::file::load(filename);
            match result {
                Ok(file) => Some(file),
                Err(e) => {
                    error!("unable to load test file ({}) data: {}", filename, e);
                    None
                }
            }
        })
        .filter_map(|f| {
            let name = f.name.clone().unwrap_or(f.filename.clone());
            let result = validation::validate_file(f, &global_variables);
            match result {
                Ok(td) => {
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
                                name,
                                cli_tags.join(", ")
                            );

                            return None;
                        } else {
                            for t in cli_tags.iter() {
                                if !td_tags.contains(t) {
                                    tests_to_ignore.push(td.clone());

                                    debug!("test `{}` is missing tag: {}", name, t);
                                    return None;
                                }
                            }
                        }
                    }

                    Some(td)
                }
                Err(e) => {
                    error!("test ({}) failed validation: {}", name, e);
                    None
                }
            }
        })
        .collect();

    if tests_to_ignore.len() > 0 {
        trace!("filtering out tests which don't match the tag pattern")
    }

    let tests_by_id: HashMap<String, test::Definition> = tests_to_run
        .clone()
        .into_iter()
        .chain(tests_to_ignore.into_iter())
        .map(|td| (td.id.clone(), td))
        .collect();

    tests_to_run.sort_by(|a, b| a.name.partial_cmp(&b.name).unwrap());

    let mut duplicate_filter: HashSet<String> = HashSet::new();

    let mut tests_to_run_with_dependencies: Vec<test::Definition> = Vec::new();

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

    let dry_run = match cli.command {
        Commands::DryRun {
            tags: _,
            tags_or: _,
        } => true,
        _ => false,
    };

    for (i, td) in tests_to_run_with_dependencies.into_iter().enumerate() {
        let boxed_td: Box<test::Definition> = Box::from(td);

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

            if !config.settings.continue_on_failure && !passed {
                std::process::exit(1);
            }
        }
    }

    Ok(())
}
