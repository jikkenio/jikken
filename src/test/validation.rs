use crate::test;
use std::fmt;

#[derive(Debug, Clone)]
pub struct Error {
    pub reason: String,
}

impl std::error::Error for Error {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.reason.len() == 0 {
            write!(f, "test validation")
        } else {
            write!(f, "{}", self.reason)
        }
    }
}

// this method is intended to do basic validation of the test file and convert it into a TestDefinition if it passes
pub fn _validate_file(_test_file: &test::File) -> Result<test::Definition, Error> {
    Err(Error {
        reason: "blah".to_string(),
    })
}

// this method is intended to do a thorough validation of rules and logic in the resolved test definition
pub fn _validate_definition(_test_definition: &test::Definition) -> Result<bool, Error> {
    Ok(true)
}
