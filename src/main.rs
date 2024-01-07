mod config;
mod errors;
mod executor;
mod json;
mod logger;
mod machine;
mod new;
mod telemetry;
mod test;
mod updater;

use clap::{Parser, Subcommand};
use log::{error, info, Level, LevelFilter};
use logger::SimpleLogger;
use serde::{Deserialize, Serialize};
use std::{error::Error, ffi::OsStr};
use tokio::fs;

const VERSION: &str = env!("CARGO_PKG_VERSION");

pub enum TagMode {
    AND,
    OR,
}

#[derive(Parser, Serialize, Deserialize)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Indicate which environment tests are executing against
    /// {n}This is not used unless tests are reporting to the Jikken.IO platform via an API Key
    #[arg(short, long = "env", name = "env")]
    environment: Option<String>,

    /// Enable quiet mode, suppresses all console output
    #[arg(short, long, default_value_t = false)]
    quiet: bool,

    /// Enable verbose mode, provides more detailed console output
    #[arg(short, long, default_value_t = false)]
    verbose: bool,

    /// Enable trace mode, provides exhaustive console output
    #[arg(long, default_value_t = false)]
    trace: bool,
}

#[derive(Subcommand, Serialize, Deserialize)]
pub enum Commands {
    /// Execute tests
    Run {
        /// The path(s) to search for test files
        /// {n}By default, the current path is used
        #[arg(name = "path")]
        paths: Vec<String>,

        /// Recursively search for test files
        #[arg(short)]
        recursive: bool,

        /// Select tests to run based on tags
        /// {n}By default, tests must match all given tags to be selected
        #[arg(short, long = "tag", name = "tag")]
        tags: Vec<String>,

        /// Toggle tag matching logic to select tests matching any of the given tags
        #[arg(long, default_value_t = false)]
        tags_or: bool,
    },

    /// Process tests without calling API endpoints
    #[command(name = "dryrun")]
    DryRun {
        /// The path(s) to search for test files
        /// {n}By default, the current path is used
        #[arg(name = "path")]
        paths: Vec<String>,

        /// Recursively search for test files
        #[arg(short)]
        recursive: bool,

        /// Select tests to run based on tags
        /// {n}By default, tests must match all given tags to be selected
        #[arg(short, long = "tag", name = "tag")]
        tags: Vec<String>,

        /// Toggle tag matching logic to select tests matching any of the given tags
        #[arg(long, default_value_t = false)]
        tags_or: bool,
    },

    /// Create a new test
    New {
        /// The name of the test file to be created
        name: Option<String>,

        /// Generate a test template with all available options
        #[arg(short, long = "full", name = "full")]
        full: bool,

        /// Generate a multi-stage test template
        #[arg(short = 'm', long = "multistage", name = "multistage")]
        multistage: bool,

        /// Output template to the console instead of saving to a file
        #[arg(short = 'o')]
        output: bool,
    },

    /// Update Jikken, if a newer version exists
    Update,
}

fn darius_is_jkt_test(entry: &walkdir::DirEntry) -> bool
{
    entry.metadata().map(|e|e.is_file()).unwrap_or(false) &&
    entry.file_name().to_str().map(|s| s.ends_with(".jkt")).unwrap_or(false)
}

// TODO: Add ignore and filter out hidden etc
fn is_jkt(entry: &walkdir::DirEntry) -> bool {
    entry
        .file_name()
        .to_str()
        .map(|s| entry.file_type().is_dir() || s.ends_with(".jkt"))
        .unwrap_or(false)
}

async fn search_directory(
    path : &str,
    recursive : bool,
    ignorePattern : &Option<String>,
    matchPattern : &Option<String>
) -> Result<Vec<String>, Box<dyn Error + Send + Sync>> {
    let mut ret : Vec<String> = Vec::new();
    walkdir::WalkDir::new(&path)
        .max_depth(if recursive {::std::usize::MAX} else {0})
        .into_iter()
        //.filter_entry(|e| //e.metadata().map(|e|e.is_file()).unwrap_or(false) &&
        //               darius_is_jkt_test(e.file_name()))
        .filter_map(|e| e.ok())
        .filter(darius_is_jkt_test)
        .for_each(|e|match e.path().to_str() {
            Some(s) => {ret.push(String::from(s))},
            None => {},
        });
    
    return Ok(ret);
}

async fn get_files_v2(
    paths: Vec<String>,
    recursive: bool,
) -> Result<Vec<String>, Box<dyn Error + Send + Sync>> {
    let mut results : Vec<String> = Vec::new();

    for path in paths {
        let path_metadata = fs::metadata(&path).await?;

        if (path_metadata.is_dir()) 
        {
            results.append(search_directory(path.as_str(), recursive, &None, &None)
                .await
                .unwrap_or(Vec::new())
                .as_mut());
            
        }
        else
        {
            results.push(path);
        }
    }

    for r in results.clone() {
        info!("file: {}\n", r);
    }

    Ok(results)

}

async fn get_files(
    paths: Vec<String>,
    recursive: bool,
) -> Result<Vec<String>, Box<dyn Error + Send + Sync>> {
    let mut results = Vec::new();

    for path in paths {
        let path_metadata = fs::metadata(&path).await?;

        if path_metadata.is_dir() {
            if recursive {
                walkdir::WalkDir::new(&path)
                    .into_iter()
                    .filter_entry(is_jkt)
                    .filter_map(|v| v.ok())
                    .filter(|x| !x.file_type().is_dir())
                    .for_each(|x| results.push(String::from(x.path().to_str().unwrap())));
            } else {
                let mut read_dir = tokio::fs::read_dir(&path).await?;
                while let Some(entry) = read_dir.next_entry().await? {
                    let md = fs::metadata(entry.path()).await?;
                    if md.is_file()
                        && entry
                            .file_name()
                            .to_ascii_lowercase()
                            .into_string()
                            .unwrap()
                            .ends_with(".jkt")
                    {
                        results.push(String::from(entry.path().to_str().unwrap()));
                    }
                }
            }
        } else {
            results.push(path);
        }
    }

    for r in results.clone() {
        info!("file: {}\n", r);
    }

    Ok(results)
}

async fn run_tests(
    paths: Vec<String>,
    tags: Vec<String>,
    tags_or: bool,
    dryrun_mode: bool,
    recursive: bool,
    cli_args: Box<serde_json::Value>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let mut cli_paths = paths;

    if cli_paths.is_empty() {
        cli_paths.push(".".to_string())
    }

    let cli_tag_mode = if tags_or { TagMode::OR } else { TagMode::AND };
    let config = config::get_config().await;
    let files = get_files_v2(cli_paths, recursive).await?;
    let test_plurality = if files.len() != 1 { "s" } else { "" };

    info!(
        "Jikken found {} test file{}.\n",
        files.len(),
        test_plurality
    );

    let report =
        executor::execute_tests(config, files, dryrun_mode, tags, cli_tag_mode, cli_args).await;

    info!(
        "Jikken executed {} test{} with {} passed and {} failed.\n",
        report.run, test_plurality, report.passed, report.failed
    );

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let cli = Cli::parse();
    let cli_args = Box::new(serde_json::to_value(&cli)?);

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

    match cli.command {
        Commands::Update => {
            updater::try_updating().await;
            std::process::exit(0);
        }
        Commands::New {
            full,
            multistage,
            output,
            name,
        } => {
            updater::check_for_updates().await;
            let created = new::create_test_template(full, multistage, output, name).await;
            match created {
                Ok(_) => {
                    std::process::exit(0);
                }
                Err(_) => {
                    std::process::exit(1);
                }
            }
        }
        Commands::DryRun {
            tags,
            tags_or,
            recursive,
            paths,
        } => {
            updater::check_for_updates().await;
            run_tests(
                paths,
                tags,
                tags_or,
                true,
                recursive,
                Box::new(serde_json::Value::Null),
            )
            .await?;
        }
        Commands::Run {
            tags,
            tags_or,
            recursive,
            paths,
        } => {
            updater::check_for_updates().await;
            run_tests(paths, tags, tags_or, false, recursive, cli_args).await?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests{
    #[test]
    fn darius_test() {
        
    }


} // mod tests



