use std::fmt;

#[derive(Debug, Clone)]
pub struct ValidationError;

impl std::error::Error for ValidationError {}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "test file failed validation")
    }
}

#[derive(Debug, Clone)]
pub struct TestFailure {
    pub reason: String,
}

impl std::error::Error for TestFailure {}

impl fmt::Display for TestFailure {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.reason.len() == 0 {
            write!(f, "test failed")
        } else {
            write!(f, "{}", self.reason)
        }
    }
}
