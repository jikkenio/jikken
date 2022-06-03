#![allow(dead_code)]
mod test_descriptor;
mod test_runner;

use clap::Parser;

// #[path = "parsertest_descriptor.rs"] mod

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[clap(short, long)]
    name: String,

    #[clap(short, long, default_value_t = 1)]
    count: u8,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // let args = Args::parse();

    // for _ in 0..args.count {
    //     println!("Hello {}!", args.name)
    // }

    let runner = test_runner::TestRunner::new();

    let mut td = test_descriptor::TestDescriptor::new(String::from("/home/lrusso/projects/jikken/test.jkt"));
    td.load();

    runner.run(td).await?;

    Ok(())
}