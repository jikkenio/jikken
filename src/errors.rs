use std::fmt;

#[derive(Debug, Clone)]
pub struct ValidationError {
    pub reason: String,
}

impl std::error::Error for ValidationError {}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.reason.is_empty() {
            write!(f, "test validation")
        } else {
            write!(f, "{}", self.reason)
        }
    }
}

#[derive(Debug, Clone)]
pub struct TestFailure {
    pub reason: String,
}

impl std::error::Error for TestFailure {}

impl fmt::Display for TestFailure {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.reason.is_empty() {
            write!(f, "test failed")
        } else {
            write!(f, "{}", self.reason)
        }
    }
}

#[derive(Debug, Clone)]
pub struct TelemetryError {
    pub reason: String,
}

impl std::error::Error for TelemetryError {}

impl fmt::Display for TelemetryError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.reason.is_empty() {
            write!(f, "telemetry failed")
        } else {
            write!(f, "{}", self.reason)
        }
    }
}
