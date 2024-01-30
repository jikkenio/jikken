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
use glob::{glob_with, MatchOptions};
use log::{debug, error, info, Level, LevelFilter};
use logger::SimpleLogger;
use serde::{Deserialize, Serialize};
use std::error::Error;
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

fn glob_walk(glob_string: &String) -> Result<Vec<String>, Box<dyn Error + Send + Sync>> {
    let mut ret: Vec<String> = Vec::new();

    for entry in glob_with(glob_string.as_str(), MatchOptions::default()).unwrap() {
        if let Ok(path) = entry {
            match path.to_str() {
                Some(s) => {
                    if s.ends_with(".jkt") {
                        ret.push(String::from(s))
                    }
                }
                None => {}
            }
        }
    }

    return Ok(ret);
}

fn satisfies_potential_glob_filter(glob_pattern: &Option<glob::Pattern>, file_name: &str) -> bool {
    return match &glob_pattern {
        Some(p) => p.matches_with(file_name, MatchOptions::default()),
        None => true,
    };
}

// Consider how to approach feedback to user when supplied pattern
// is invalid
fn create_top_level_filter(glob_pattern: &Option<String>) -> impl Fn(&walkdir::DirEntry) -> bool {
    let extract_pattern = |s: &Option<String>| {
        s.clone()
            .map(|s| glob::Pattern::new(s.as_str()))
            .map(|r| r.ok())
            .unwrap_or(None)
    };

    let jikken_ignore_exists_in_dir = |d: &walkdir::DirEntry| {
        return d.path().join(".jikkenignore").try_exists().unwrap_or(false);
    };

    let pattern = extract_pattern(glob_pattern);
    return move |e: &walkdir::DirEntry| -> bool {
        e.file_name()
            .to_str()
            .map(|s| {
                (e.file_type().is_file()
                    && s.ends_with(".jkt")
                    && satisfies_potential_glob_filter(&pattern, &s))
                    || (e.file_type().is_dir() && !jikken_ignore_exists_in_dir(e))
            })
            .unwrap_or(false)
    };
}

async fn search_directory(
    path: &str,
    recursive: bool,
    glob_pattern: Option<String>,
) -> Result<Vec<String>, Box<dyn Error + Send + Sync>> {
    let mut ret: Vec<String> = Vec::new();
    let entry_is_file = |e: &walkdir::DirEntry| e.metadata().map(|e| e.is_file()).unwrap_or(false);

    walkdir::WalkDir::new(&path)
        .max_depth(if recursive { ::std::usize::MAX } else { 1 })
        .into_iter()
        .filter_entry(create_top_level_filter(&glob_pattern))
        .filter_map(|e| e.ok())
        .filter(entry_is_file)
        .for_each(|e| match e.path().to_str() {
            Some(s) => ret.push(String::from(s)),
            None => {}
        });

    return Ok(ret);
}

async fn get_files(
    paths: Vec<String>,
    recursive: bool,
) -> Result<Vec<String>, Box<dyn Error + Send + Sync>> {
    let mut results: Vec<String> = Vec::new();

    for path in paths {
        let exists = fs::try_exists(&path).await.unwrap_or(false);
        let is_file = exists && fs::metadata(&path).await?.is_file();
        let glob_pattern = if !exists {
            Some(String::from(path.as_str()))
        } else {
            None
        };

        if is_file {
            results.push(path);
        } else if !exists && !recursive {
            results.append(glob_walk(&path).unwrap_or(Vec::new()).as_mut());
        } else {
            results.append(
                search_directory(
                    if exists { path.as_str() } else { "." },
                    recursive,
                    glob_pattern,
                )
                .await
                .unwrap_or(Vec::new())
                .as_mut(),
            );
        }
    }

    for r in results.clone() {
        debug!("file: {}", r);
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
    let files = get_files(cli_paths, recursive).await?;
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

//------------------TESTS---------------------------------

//mod file_capture{
#[cfg(test)]
mod tests {
    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::*;
    use {std::fs::File, tempfile::tempdir};

    #[tokio::test]
    async fn get_files_with_one_level_of_depth_recursively() {
        let tmp_dir = tempdir().unwrap();
        let tmp_path = tmp_dir.path();
        let tmp_path_str = tmp_path.to_str().unwrap();
        {
            //Begin Scope
            let _: Vec<std::fs::File> = vec![
                "random_file",
                "my_test.jkt",
                "something_else",
                "my_test_2.jkt",
            ]
            .iter()
            .map(|f| File::create(tmp_path.join(f)).unwrap())
            .collect();
            let found_files = get_files(vec![String::from(tmp_path_str)], true).await;
            assert_eq!(2, found_files.unwrap().len());
        } //End Scope
        _ = tmp_dir.close();
    }
    #[tokio::test]
    async fn get_files_with_one_level_of_depth_recursively_with_jkignore() {
        let tmp_dir = tempdir().unwrap();
        let tmp_path = tmp_dir.path();
        let tmp_path_str = tmp_path.to_str().unwrap();
        {
            //Begin Scope
            let _: Vec<std::fs::File> = vec![
                "random_file",
                "my_test.jkt",
                "something_else",
                "my_test_2.jkt",
                ".jikkenignore",
            ]
            .iter()
            .map(|f| File::create(tmp_path.join(f)).unwrap())
            .collect();
            let found_files = get_files(vec![String::from(tmp_path_str)], true).await;
            assert_eq!(0, found_files.unwrap().len());
        } //End Scope
        _ = tmp_dir.close();
    }

    #[tokio::test]
    async fn get_files_with_one_level_of_depth_non_recursively() {
        let tmp_dir = tempdir().unwrap();
        let tmp_path = tmp_dir.path();
        let tmp_path_str = tmp_path.to_str().unwrap();
        {
            //Begin Scope
            let _: Vec<std::fs::File> = vec![
                "random_file",
                "my_test.jkt",
                "something_else",
                "my_test_2.jkt",
            ]
            .iter()
            .map(|f| File::create(tmp_path.join(f)).unwrap())
            .collect();
            let found_files = get_files(vec![String::from(tmp_path_str)], false).await;
            assert_eq!(2, found_files.unwrap().len());
        } //End Scope
        _ = tmp_dir.close();
    }

    #[tokio::test]
    async fn get_files_with_one_level_of_depth_non_recursively_globbing() {
        let tmp_dir = tempdir().unwrap();
        let tmp_path = tmp_dir.path();
        let glob_path = tmp_path.join("*_2*");
        let glob_path_str = glob_path.to_str().unwrap();

        {
            //Begin Scope
            let _: Vec<std::fs::File> = vec![
                "random_file",
                "my_test.jkt",
                "something_else",
                "my_test_2.jkt",
            ]
            .iter()
            .map(|f| File::create(tmp_path.join(f)).unwrap())
            .collect();
            let found_files = get_files(vec![String::from(glob_path_str)], false).await;
            assert_eq!(1, found_files.unwrap().len());
        } //End Scope
        _ = tmp_dir.close();
    }

    #[tokio::test]
    async fn get_files_with_recursive_globbing() {
        let tmp_dir = tempdir().unwrap();
        let tmp_path = tmp_dir.path();
        let glob_path_str = "*_2*";
        _ = std::env::set_current_dir(tmp_path);
        {
            //Begin Scope
            let _: Vec<std::fs::File> = vec![
                "random_file",
                "my_test.jkt",
                "something_else",
                "my_test_2.jkt",
            ]
            .iter()
            .map(|f| File::create(tmp_path.join(f)).unwrap())
            .collect();
            let found_files = get_files(vec![String::from(glob_path_str)], true).await;
            assert_eq!(1, found_files.unwrap().len());
        } //End Scope
        _ = tmp_dir.close();
    }
} // mod tests

//}
