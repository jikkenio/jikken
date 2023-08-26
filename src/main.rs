mod config;
mod errors;
mod executor;
mod json;
mod logger;
mod machine;
mod telemetry;
mod test;
mod updater;

use clap::{Parser, Subcommand};
use log::{debug, error, info, warn, Level, LevelFilter};
use logger::SimpleLogger;
use serde::{Deserialize, Serialize};
use std::error::Error;
use test::template;
use tokio::fs;
use tokio::io::AsyncWriteExt;

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

        #[arg(short)]
        recursive: bool,

        #[arg(name = "path")]
        paths: Vec<String>,
    },

    /// Process tests without calling api endpoints
    #[command(name = "dryrun")]
    DryRun {
        #[arg(short, long = "tag", name = "tag")]
        tags: Vec<String>,

        #[arg(long, default_value_t = false)]
        tags_or: bool,

        #[arg(short)]
        recursive: bool,

        #[arg(name = "path")]
        paths: Vec<String>,
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

async fn get_files(paths: Vec<String>, recursive: bool) -> Result<Vec<String>, Box<dyn Error + Send + Sync>> {
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
                  if md.is_file() && entry.file_name().to_ascii_lowercase().into_string().unwrap().ends_with(".jkt") {
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

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let cli = Cli::parse();
    let (cli_tags, cli_tag_mode, cli_recursive, mut cli_paths) = match &cli.command {
        Commands::Run { tags, tags_or, recursive, paths } | Commands::DryRun { tags, tags_or, recursive, paths } => (
            tags.clone(),
            if *tags_or { TagMode::OR } else { TagMode::AND },
            *recursive,
            paths.clone(),
        ),
        _ => (Vec::new(), TagMode::AND, false, Vec::new()),
    };

    if cli_paths.is_empty() {
        cli_paths.push(".".to_string())
    }

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
                    warn!("Run command: `jk update` to update jikken or update using your package manager");
                }
            }
            Err(error) => {
                debug!("error checking for updates: {}", error);
            }
        },
    }

    if let Commands::New {
        full,
        multistage,
        output,
        name,
    } = &cli.command
    {
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

    let files = get_files(cli_paths, cli_recursive).await?;

    let test_plurality = if files.len() != 1 { "s" } else { "" };

    info!(
        "Jikken found {} test file{}.\n",
        files.len(),
        test_plurality
    );

    let report = executor::execute_tests(config, files, &cli, cli_tags, cli_tag_mode).await;

    info!(
        "Jikken executed {} test{} with {} passed and {} failed.\n",
        report.run, test_plurality, report.passed, report.failed
    );

    Ok(())
}
