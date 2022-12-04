use std::fmt;

#[derive(Debug, Clone)]
pub struct ValidationError;

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "test file failed validation")
    }
}
