#![allow(dead_code)]
mod test_descriptor;
mod test_runner;
mod config;
mod config_settings;

use walkdir::{DirEntry, WalkDir};
use std::error::Error;
use std::path::Path;
// use indicatif::ProgressBar;

// use clap::Parser;

// #[path = "parsertest_descriptor.rs"] mod

// #[derive(Parser, Debug)]
// #[clap(author, version, about, long_about = None)]
// struct Args {
//     #[clap(short, long, default_value = ".")]
//     file: String
// }



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
        .filter_entry(|e| is_jkt(e))
        .filter_map(|v| v.ok())
        .filter(|x| !x.file_type().is_dir())
        .for_each(|x| results.push(String::from(x.path().to_str().unwrap())));

    results
}

fn get_config(file: &str) -> Result<config::Config, Box<dyn Error>> {
    let data = std::fs::read_to_string(file)?;
    let config: config::Config = toml::from_str(&data)?;
    Ok(config)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut config: Option<config::Config> = None;

    let runner = test_runner::TestRunner::new();

    if Path::new(".jikken").exists() {
        let config_raw = get_config(".jikken");
        match config_raw {
            Ok(c) => {
                config = Some(c);
            },
            Err(e) => {
                println!("invalid configuration file: {}", e);
                std::process::exit(exitcode::CONFIG);
            }
        }
    }

    let files = get_files();

    // let bar = ProgressBar::new(files.len() as u64);

    println!("Jikken found {} tests.", files.len());

    let mut continue_on_failure = false;

    match config {
        Some(ref c) => {
            if let Some(settings) = c.settings.as_ref() {
                match settings.continue_on_failure {
                    Some(cof) => {
                        continue_on_failure = cof;
                    },
                    _ => {}
                }
            }
        }
        _ => {}
    }

    for (i, file) in files.iter().enumerate() {
        let mut td = test_descriptor::TestDescriptor::new(file.to_string());
        td.load(config.clone());    
        let passed = runner.run(td, i + 1).await?;
        // bar.inc(1);
        
        if !continue_on_failure {
            if !passed {
                std::process::exit(1);
            }
        }
    }

    // bar.finish();

    Ok(())
}