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
mod validated;

use clap::{Parser, Subcommand};
use glob::{glob_with, MatchOptions};
use log::{debug, error, info, warn, Level, LevelFilter};
use logger::SimpleLogger;
use serde::{Deserialize, Serialize};
use std::{
    error::Error,
    path::{Path, PathBuf},
};
use telemetry::PlatformIdFailure;
use tokio::{fs, io::AsyncWriteExt};
use ulid::Ulid;

const VERSION: &str = env!("CARGO_PKG_VERSION");
const IGNORE_FILE: &str = ".jikkenignore";

#[derive(PartialEq, Eq)]
pub enum ExecutionMode {
    Run,
    Dryrun,
    List,
    Format,
    Validate(bool),
}

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

    /// Indicate which project tests belong to
    /// {n}This is not used unless tests are reporting to the Jikken.IO platform via an API Key
    #[arg(short, long = "project", name = "proj")]
    project: Option<String>,

    /// Specify path to a Jikken configuration file
    /// {n}By default, optional ".jikken" files can be placed in the current directory
    /// {n}and the user's home directory. This option instructs jikken to use the
    /// {n}provided path instead of the optional .jikken file in the current directory
    #[arg(short, long = "config_file", name = "config_file")]
    config_file: Option<String>,

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
    #[command(name = "run", alias = "r")]
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

        /// Output results in junit format to specified file
        #[arg(long = "junit", name = "junit_file")]
        junit: Option<String>,
    },

    /// Execute tests without calling API endpoints
    #[command(name = "dryrun", alias = "dr")]
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

        /// Output results in junit format to specified file
        #[arg(long = "junit", name = "junit_file")]
        junit: Option<String>,
    },

    /// List test files
    #[command(name = "list")]
    List {
        /// The path(s) to search for test files
        /// {n}By default, the current path is used
        #[arg(name = "path")]
        paths: Vec<String>,

        /// Recursively search for test files
        #[arg(short)]
        recursive: bool,

        /// Select tests to list based on tags
        /// {n}By default, tests must match all given tags to be selected
        #[arg(short, long = "tag", name = "tag")]
        tags: Vec<String>,

        /// Toggle tag matching logic to select tests matching any of the given tags
        #[arg(long, default_value_t = false)]
        tags_or: bool,
    },

    /// Format test files
    #[command(name = "format", alias = "fmt")]
    Format {
        /// The path(s) to search for test files
        /// {n}By default, the current path is used
        #[arg(name = "path")]
        paths: Vec<String>,

        /// Recursively search for test files
        #[arg(short)]
        recursive: bool,

        /// Select tests to format based on tags
        /// {n}By default, tests must match all given tags to be selected
        #[arg(short, long = "tag", name = "tag")]
        tags: Vec<String>,

        /// Toggle tag matching logic to select tests matching any of the given tags
        #[arg(long, default_value_t = false)]
        tags_or: bool,
    },

    /// Validate test files
    Validate {
        /// The path(s) to search for test files
        /// {n}By default, the current path is used
        #[arg(name = "path")]
        paths: Vec<String>,

        /// Recursively search for test files
        #[arg(short)]
        recursive: bool,

        /// Select tests to validate based on tags
        /// {n}By default, tests must match all given tags to be selected
        #[arg(short, long = "tag", name = "tag")]
        tags: Vec<String>,

        /// Toggle tag matching logic to select tests matching any of the given tags
        #[arg(long, default_value_t = false)]
        tags_or: bool,

        /// Automatically generate and insert platform IDs in tests that don't have one
        #[arg(long, default_value_t = false)]
        generate_platform_ids: bool,
    },

    /// Create a new test
    New {
        /// The name of the test file to be created
        name: Option<String>,

        /// OpenApi spec to derive tests from
        #[arg(long = "from_openapi", name = "from_openapi")]
        openapi_spec_path: Option<String>,

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

    /// Update Jikken (if a newer version exists)
    Update,
}

fn glob_walk(glob_string: &str) -> Result<Vec<String>, Box<dyn Error + Send + Sync>> {
    let mut ret: Vec<String> = Vec::new();

    for path in glob_with(glob_string, MatchOptions::default())
        .unwrap()
        .flatten()
    {
        if let Some(s) = path.to_str() {
            if s.ends_with(".jkt") {
                ret.push(String::from(s))
            }
        }
    }

    Ok(ret)
}

fn satisfies_potential_glob_filter(glob_pattern: &Option<glob::Pattern>, file_name: &str) -> bool {
    match &glob_pattern {
        Some(p) => p.matches_with(file_name, MatchOptions::default()),
        None => true,
    }
}

fn check_supplied_config_file_existence(config_file: &Option<String>) {
    if let Some(file) = config_file {
        check_supplied_file_existence("config", &PathBuf::from(file));
    }
}

fn check_supplied_file_existence(file_description: &str, path: &Path) {
    if !std::path::Path::try_exists(path).unwrap_or(false) {
        warn!(
            "Supplied {} file does not exist: {}",
            file_description,
            path.as_os_str().to_str().unwrap_or_default()
        );
    }
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
    let pattern = extract_pattern(glob_pattern);
    return move |e: &walkdir::DirEntry| -> bool {
        e.file_name()
            .to_str()
            .map(|s| {
                (e.file_type().is_file()
                    && s.ends_with(".jkt")
                    && satisfies_potential_glob_filter(&pattern, s))
                    || e.file_type().is_dir()
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

    walkdir::WalkDir::new(path)
        .max_depth(if recursive { usize::MAX } else { 1 })
        .into_iter()
        .filter_entry(create_top_level_filter(&glob_pattern))
        .filter_map(|e| e.ok())
        .filter(entry_is_file)
        .for_each(|e| {
            if let Some(s) = e.path().to_str() {
                ret.push(String::from(s))
            }
        });

    Ok(ret)
}

async fn get_ignore_files(ignore_file: &std::path::Path) -> Vec<String> {
    tokio::fs::read_to_string(ignore_file)
        .await
        .ok()
        .map(|s| {
            s.split('\n')
                .map(|s| s.to_string())
                .collect::<Vec<String>>()
        })
        .unwrap_or_default()
}

fn ignore_matches<'a>(ignore_pattern: &'a str, file: &'a str) -> bool {
    let ignore_pattern_path = std::path::Path::new(ignore_pattern);
    let file_path = std::path::Path::new(file);

    let dirname_extractor =
        |s: &'a Path| -> &'a str { s.parent().and_then(|s| s.to_str()).unwrap_or_default() };

    if ignore_pattern_path.is_file() {
        return file_path == ignore_pattern_path;
    } else if ignore_pattern_path.is_dir() {
        return dirname_extractor(file_path) == ignore_pattern_path.to_str().unwrap_or_default();
    }

    glob::Pattern::new(ignore_pattern)
        .map(|p| p.matches_path(file_path))
        .unwrap_or_default()
}

async fn get_files(
    paths: Vec<String>,
    ignore_file: &std::path::Path,
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
            results.append(glob_walk(&path).unwrap_or_default().as_mut());
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

    let ignore_patterns = get_ignore_files(ignore_file).await;
    results.retain(|f| {
        !ignore_patterns
            .iter()
            .any(|ignore| ignore_matches(ignore.as_str(), f.as_str()))
    });

    for r in results.clone() {
        debug!("file: {}", r);
    }

    Ok(results)
}

fn print_test_info(mut tests: Vec<test::Definition>) {
    let mut path_column = vec!["PATH".to_string()];
    let mut name_column = vec!["TEST NAME".to_string()];
    let mut tags_column = vec!["TAGS".to_string()];

    tests.sort_by_key(|td| td.file_data.filename.clone());
    tests.into_iter().for_each(|td| {
        name_column.push(td.name.unwrap_or("<none>".to_string()));
        tags_column.push(if td.tags.is_empty() {
            "<none>".to_string()
        } else {
            td.tags.join(", ")
        });
        path_column.push(td.file_data.filename)
    });

    let get_column_width = |v: &Vec<String>| v.iter().fold(0, |max, s| std::cmp::max(max, s.len()));
    let max_name_size = get_column_width(&name_column);
    let max_tags_size = get_column_width(&tags_column);
    let max_path_size = get_column_width(&path_column);
    name_column
        .into_iter()
        .zip(tags_column)
        .zip(path_column)
        .for_each(|((n, t), p)| {
            info!(
                "{: <max_path_size$}    {: <max_name_size$}    {: <max_tags_size$} \n",
                p, n, t
            );
        });
}

async fn run_tests(
    paths: Vec<String>,
    tags: Vec<String>,
    tags_or: bool,
    execution_mode: ExecutionMode,
    recursive: bool,
    project: Option<String>,
    environment: Option<String>,
    config_file: Option<String>,
    junit_file: Option<String>,
    cli_args: Box<serde_json::Value>,
) -> Result<executor::Report, Box<dyn Error + Send + Sync>> {
    let mut cli_paths = paths;

    if cli_paths.is_empty() {
        cli_paths.push(".".to_string())
    }

    let cli_tag_mode = if tags_or { TagMode::OR } else { TagMode::AND };
    let config = config::get_config(config_file).await;
    let files = get_files(cli_paths, std::path::Path::new(IGNORE_FILE), recursive).await?;
    let plurality_policy = |count: usize| match count {
        1 => "",
        _ => "s",
    };

    let project = project.or(config.clone().settings.project);
    let environment = environment.or(config.clone().settings.environment);

    if config.settings.bypass_cert_verification {
        warn!("WARNING: SSL certificate verification is disabled.\nIf this is not intentional please adjust your config settings.\nFor more information please check our docs: https://www.jikken.io/docs/configuration/");
        log::logger().flush();
    }

    info!(
        "Jikken found {} test file{}.\n\n",
        files.len(),
        plurality_policy(files.len())
    );

    if files.is_empty() {
        return Ok(executor::Report::default());
    }

    let filters_specified = !tags.is_empty();

    let (tests_to_run, tests_to_ignore) =
        executor::tests_from_files(&config, files, tags, project, environment, cli_tag_mode);

    if execution_mode == ExecutionMode::List {
        let number_of_tests_to_run = tests_to_run.len();
        print_test_info(tests_to_run);
        if filters_specified {
            info!(
                "\n{} test{} matched provided filters.\n",
                number_of_tests_to_run,
                plurality_policy(number_of_tests_to_run)
            );
        }
        return Ok(executor::Report::default());
    }

    if execution_mode == ExecutionMode::Format {
        for td in &tests_to_run {
            let mut file = fs::File::create(&td.file_data.filename).await?;
            let file_data = serde_yaml::to_string(&td.file_data).unwrap();
            file.write_all(file_data.as_bytes()).await?;
        }

        info!(
            "Successfully formatted {} test files.\n",
            &tests_to_run.len()
        );
        return Ok(executor::Report::default());
    }

    if let ExecutionMode::Validate(generate) = execution_mode {
        if let Some(token) = &config.settings.api_key {
            if uuid::Uuid::parse_str(token).is_ok() {
                let validation_results =
                    telemetry::validate_platform_ids(tests_to_run.iter().collect());
                let mut has_missing = false;

                if let Err(failures) = validation_results {
                    for failure in failures {
                        if generate && failure.1 == PlatformIdFailure::Missing {
                            let platform_id = Ulid::new().to_string();
                            let mut test = tests_to_run[failure.0.index].clone();
                            test.file_data.platform_id = Some(platform_id.clone());

                            let mut file = fs::File::create(&test.file_data.filename).await?;
                            let file_data = serde_yaml::to_string(&test.file_data).unwrap();
                            file.write_all(file_data.as_bytes()).await?;

                            info!(
                                "Successfully updated test at path \"{}\" with platform ID {}.\n",
                                test.file_data.filename, platform_id
                            );
                        } else {
                            if !has_missing && failure.1 == PlatformIdFailure::Missing {
                                has_missing = true;
                            }
                            executor::print_validation_failures(vec![failure], true);
                        }
                    }

                    if has_missing {
                        warn!("\nRun the validate command with the --generate-platform-ids option to automatically generate and insert missing platform IDs.");
                    }
                }
            } else {
                error!("API Key {} is invalid", &token);
            }
        }

        return Ok(executor::Report::default());
    }

    let report = executor::execute_tests(
        config,
        tests_to_run,
        execution_mode == ExecutionMode::Dryrun,
        tests_to_ignore,
        junit_file,
        cli_args,
    )
    .await;

    let runtime_label = executor::runtime_formatter(report.runtime);

    if report.skipped > 0 {
        info!(
            "Jikken executed {} test{} from {} file{} with {} passed, {} skipped, and {} failed in {}.\n",
            report.run,
            plurality_policy(report.run.into()),
            report.test_files,
            plurality_policy(report.test_files as usize),
            report.passed,
            report.skipped,
            report.failed,
            runtime_label,
        );
    } else {
        info!(
            "Jikken executed {} test{} from {} file{} with {} passed and {} failed in {}.\n",
            report.run,
            plurality_policy(report.run.into()),
            report.test_files,
            plurality_policy(report.test_files as usize),
            report.passed,
            report.failed,
            runtime_label,
        );
    }
    Ok(report)
}

/*
    Result is converted to an exit code implicitly,
    but it prints a message we don't like. So we're
    forced to resort to this
*/
fn result_to_exit_code<T>(
    res: Result<T, Box<dyn Error + Send + Sync>>,
    print: bool,
) -> std::process::ExitCode {
    match res {
        Err(e) => {
            if print {
                eprintln!("Error: {}", e);
            }
            std::process::ExitCode::FAILURE
        }
        Ok(_) => std::process::ExitCode::SUCCESS,
    }
}

fn result_report_to_exit_code(
    res: Result<executor::Report, Box<dyn Error + Send + Sync>>,
) -> std::process::ExitCode {
    match res {
        Err(_) => result_to_exit_code(res, true),
        Ok(r) => report_to_exit_code(r),
    }
}

fn report_to_exit_code(report: executor::Report) -> std::process::ExitCode {
    if report.failed > 0 {
        return std::process::ExitCode::FAILURE;
    }

    std::process::ExitCode::SUCCESS
}

#[tokio::main]
async fn main() -> std::process::ExitCode {
    let _ = enable_ansi_support::enable_ansi_support();

    let cli = Cli::parse();
    let cli_args = Box::new(serde_json::to_value(&cli).unwrap());

    let log_level = if cli.verbose {
        Level::Debug
    } else if cli.trace {
        Level::Trace
    } else {
        Level::Info
    };

    let my_logger = SimpleLogger::new(
        log_level,
        cli.quiet,
        matches!(cli.command, Commands::Run { .. } | Commands::DryRun { .. }),
    );

    if let Err(e) = log::set_boxed_logger(Box::new(my_logger)) {
        error!("Error creating logger: {}", e);
        panic!("unable to create logger");
    }

    log::set_max_level(LevelFilter::Trace);

    let cli_project = cli.project;
    let cli_environment = cli.environment;

    let exit_code: std::process::ExitCode = match cli.command {
        Commands::Run {
            tags,
            tags_or,
            recursive,
            paths,
            junit,
        } => {
            updater::check_for_updates().await;
            log::logger().flush();
            check_supplied_config_file_existence(&cli.config_file);
            result_report_to_exit_code(
                run_tests(
                    paths,
                    tags,
                    tags_or,
                    ExecutionMode::Run,
                    recursive,
                    cli_project,
                    cli_environment,
                    cli.config_file,
                    junit,
                    cli_args,
                )
                .await,
            )
        }
        Commands::DryRun {
            tags,
            tags_or,
            recursive,
            paths,
            junit,
        } => {
            // \todo create a runner function that takes an Fn trait and
            // eliminates the duplicated code
            updater::check_for_updates().await;
            log::logger().flush();
            check_supplied_config_file_existence(&cli.config_file);
            result_report_to_exit_code(
                run_tests(
                    paths,
                    tags,
                    tags_or,
                    ExecutionMode::Dryrun,
                    recursive,
                    cli_project,
                    cli_environment,
                    cli.config_file,
                    junit,
                    Box::new(serde_json::Value::Null),
                )
                .await,
            )
        }
        Commands::List {
            tags,
            tags_or,
            recursive,
            paths,
        } => {
            updater::check_for_updates().await;
            check_supplied_config_file_existence(&cli.config_file);
            result_report_to_exit_code(
                run_tests(
                    paths,
                    tags,
                    tags_or,
                    ExecutionMode::List,
                    recursive,
                    cli_project,
                    cli_environment,
                    cli.config_file,
                    None,
                    cli_args,
                )
                .await,
            )
        }
        Commands::Format {
            tags,
            tags_or,
            recursive,
            paths,
        } => {
            updater::check_for_updates().await;
            check_supplied_config_file_existence(&cli.config_file);
            result_report_to_exit_code(
                run_tests(
                    paths,
                    tags,
                    tags_or,
                    ExecutionMode::Format,
                    recursive,
                    cli_project,
                    cli_environment,
                    cli.config_file,
                    None,
                    cli_args,
                )
                .await,
            )
        }
        Commands::Validate {
            tags,
            tags_or,
            recursive,
            paths,
            generate_platform_ids,
        } => {
            updater::check_for_updates().await;
            check_supplied_config_file_existence(&cli.config_file);
            result_report_to_exit_code(
                run_tests(
                    paths,
                    tags,
                    tags_or,
                    ExecutionMode::Validate(generate_platform_ids),
                    recursive,
                    cli_project,
                    cli_environment,
                    cli.config_file,
                    None,
                    cli_args,
                )
                .await,
            )
        }
        Commands::New {
            full,
            openapi_spec_path,
            multistage,
            output,
            name,
        } => {
            updater::check_for_updates().await;
            match openapi_spec_path {
                Some(path) => result_to_exit_code(
                    new::create_tests_from_openapi_spec(path.as_str(), full, multistage, name),
                    false,
                ),
                None => result_to_exit_code(
                    new::create_test_template(full, multistage, output, name).await,
                    false,
                ),
            }
        }
        Commands::Update => {
            updater::try_updating().await;
            std::process::ExitCode::SUCCESS
        }
    };

    log::logger().flush();
    exit_code
}

//------------------TESTS---------------------------------

//mod file_capture{
#[cfg(test)]
mod tests {
    use std::io::Write;

    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::*;
    use std::fs::File;
    use tempfile::tempdir;

    struct DirectoryFixture {
        pub temp_dir: tempfile::TempDir,
    }

    //todo: support arbitrary depth
    impl DirectoryFixture {
        fn new(file_names: &[&str]) -> DirectoryFixture {
            let tmp_dir = tempdir().unwrap();
            let tmp_path = tmp_dir.path();
            let _: Vec<std::fs::File> = file_names
                .into_iter()
                .map(|f| File::create(tmp_path.join(f)).unwrap())
                .collect();

            DirectoryFixture { temp_dir: tmp_dir }
        }

        fn path_string(&self) -> String {
            self.temp_dir.path().to_str().unwrap().to_string()
        }

        fn create_ignore_file(&self, file_names: &[&str]) {
            let ignore_file = self.temp_dir.path().join(".jikkenignore");
            let str_to_write = file_names.join("\n");
            _ = File::create(&ignore_file)
                .unwrap()
                .write(str_to_write.as_bytes());
        }

        async fn get_files(&self, recursive: bool, path: Option<String>) -> Vec<String> {
            get_files(
                vec![path.unwrap_or(self.path_string())],
                self.temp_dir.path().join(".jikkenignore").as_path(),
                recursive,
            )
            .await
            .unwrap()
        }
    }

    #[tokio::test]
    async fn get_files_with_glob_ignore() {
        let dir_fixture = DirectoryFixture::new(
            vec![
                "random_file",
                "my_test.jkt",
                "something_else",
                "my_test_2.jkt",
            ]
            .as_slice(),
        );

        dir_fixture.create_ignore_file(
            vec![dir_fixture
                .temp_dir
                .path()
                .join("my_test*")
                .to_str()
                .unwrap_or_default()]
            .as_slice(),
        );

        assert_eq!(0, dir_fixture.get_files(true, None).await.len());
    }

    #[tokio::test]
    async fn get_files_with_dir_ignore() {
        let dir_fixture = DirectoryFixture::new(
            vec![
                "random_file",
                "my_test.jkt",
                "something_else",
                "my_test_2.jkt",
            ]
            .as_slice(),
        );

        dir_fixture.create_ignore_file(
            vec![dir_fixture.temp_dir.path().to_str().unwrap_or_default()].as_slice(),
        );

        assert_eq!(0, dir_fixture.get_files(true, None).await.len());
    }

    #[tokio::test]
    async fn get_files_with_filename_ignore() {
        let dir_fixture = DirectoryFixture::new(
            vec![
                "random_file",
                "my_test.jkt",
                "something_else",
                "my_test_2.jkt",
            ]
            .as_slice(),
        );

        dir_fixture.create_ignore_file(
            vec![
                dir_fixture
                    .temp_dir
                    .path()
                    .join("my_test.jkt")
                    .to_str()
                    .unwrap_or_default(),
                dir_fixture
                    .temp_dir
                    .path()
                    .join("my_test_2.jkt")
                    .to_str()
                    .unwrap_or_default(),
            ]
            .as_slice(),
        );

        assert_eq!(0, dir_fixture.get_files(true, None).await.len());
    }

    #[tokio::test]
    async fn get_files_with_one_level_of_depth_recursively() {
        let dir_fixture = DirectoryFixture::new(
            vec![
                "random_file",
                "my_test.jkt",
                "something_else",
                "my_test_2.jkt",
            ]
            .as_slice(),
        );

        assert_eq!(2, dir_fixture.get_files(true, None).await.len());
    }

    #[tokio::test]
    async fn get_files_with_one_level_of_depth_non_recursively() {
        let dir_fixture = DirectoryFixture::new(
            vec![
                "random_file",
                "my_test.jkt",
                "something_else",
                "my_test_2.jkt",
            ]
            .as_slice(),
        );

        assert_eq!(2, dir_fixture.get_files(false, None).await.len());
    }

    #[tokio::test]
    async fn get_files_with_one_level_of_depth_non_recursively_globbing() {
        let dir_fixture = DirectoryFixture::new(
            vec![
                "random_file",
                "my_test.jkt",
                "something_else",
                "my_test_2.jkt",
            ]
            .as_slice(),
        );

        assert_eq!(
            1,
            dir_fixture
                .get_files(
                    false,
                    Some(
                        dir_fixture
                            .temp_dir
                            .path()
                            .join("*_2*")
                            .to_str()
                            .unwrap_or_default()
                            .to_string()
                    )
                )
                .await
                .len()
        );
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
            let found_files = get_files(
                vec![String::from(glob_path_str)],
                std::path::Path::new(".does_not_exist"),
                true,
            )
            .await;
            assert_eq!(1, found_files.unwrap().len());
        } //End Scope
        _ = tmp_dir.close();
    }
} // mod tests

//}
