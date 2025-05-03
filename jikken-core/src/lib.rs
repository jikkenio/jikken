pub mod config;
pub mod errors;
pub mod executor;
pub mod json;
pub mod logger;
pub mod machine;
pub mod telemetry;
pub mod test;
pub mod validated;

const VERSION: &str = env!("CARGO_PKG_VERSION");

pub enum TagMode {
    AND,
    OR,
}
