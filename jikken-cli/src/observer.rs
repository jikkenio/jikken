use jikken_core::observer::{ExecutionEvent, ExecutionObserver};
use log::error;

/// Format runtime in milliseconds to a human-readable string
fn format_runtime(runtime_ms: u32) -> String {
    if runtime_ms < 1000 {
        format!("{}ms", runtime_ms)
    } else if runtime_ms < 60000 {
        let seconds = runtime_ms as f64 / 1000.0;
        format!("{:.2}s", seconds)
    } else {
        let minutes = runtime_ms / 60000;
        let seconds = (runtime_ms % 60000) as f64 / 1000.0;
        format!("{}m{:.2}s", minutes, seconds)
    }
}

/// CLI observer that produces the same text output as the current CLI
pub struct CliObserver {
    current_test_index: usize,
    total_tests: usize,
    test_name: String,
    policy_name: String,
}

impl CliObserver {
    pub fn new() -> Self {
        Self {
            current_test_index: 0,
            total_tests: 0,
            test_name: String::new(),
            policy_name: String::new(),
        }
    }

    pub fn with_policy_name(policy_name: String) -> Self {
        Self {
            current_test_index: 0,
            total_tests: 0,
            test_name: String::new(),
            policy_name,
        }
    }
}

impl ExecutionObserver for CliObserver {
    fn on_event(&mut self, event: ExecutionEvent) {
        match event {
            ExecutionEvent::SuiteStart { total_tests, .. } => {
                self.total_tests = total_tests;
                self.current_test_index = 0;
            }

            ExecutionEvent::TestStart {
                test,
                iteration,
                total_iterations,
            } => {
                // Only increment test index on first iteration
                if iteration == 0 {
                    self.current_test_index += 1;
                }
                self.test_name = test
                    .name
                    .clone()
                    .unwrap_or_else(|| format!("Test{}", self.current_test_index));

                // Print test start message
                if total_iterations > 1 {
                    print!(
                        "{} Test ({}/{}) `{}` Iteration({}/{})",
                        self.policy_name,
                        self.current_test_index,
                        self.total_tests,
                        &self.test_name,
                        iteration + 1,
                        total_iterations,
                    );
                } else {
                    print!(
                        "{} Test ({}/{}) `{}`",
                        self.policy_name,
                        self.current_test_index,
                        self.total_tests,
                        &self.test_name,
                    );
                }
            }

            ExecutionEvent::TestComplete {
                passed, runtime_ms, ..
            } => {
                // Format runtime
                let runtime_label = format_runtime(runtime_ms);

                if passed {
                    println!(" Runtime({}) ... \x1b[32mPASSED\x1b[0m", runtime_label);
                } else {
                    println!(" Runtime({}) ... \x1b[31mFAILED\x1b[0m", runtime_label);
                }
            }

            ExecutionEvent::TestSkipped { test, reason } => {
                self.current_test_index += 1;
                let test_name = test
                    .name
                    .clone()
                    .unwrap_or_else(|| format!("Test{}", self.current_test_index));

                // Determine skip reason and color
                let (color, reason_text) =
                    if reason.contains("disabled") || reason.contains("DISABLED") {
                        ("\x1b[33m", "DISABLED")
                    } else {
                        ("\x1b[33m", "SKIPPED")
                    };

                println!(
                    "{} Test ({}/{}) `{}` ... {}{}\x1b[0m",
                    self.policy_name,
                    self.current_test_index,
                    self.total_tests,
                    test_name,
                    color,
                    reason_text
                );
            }

            ExecutionEvent::Error { error, .. } => {
                error!("{}", error);
            }

            ExecutionEvent::ExecutionCancelled { reason, .. } => {
                error!("Execution cancelled: {}", reason);
            }

            _ => {
                // For now, ignore other events like stage start/complete, variable extraction, etc.
                // These could be added later for more detailed logging
            }
        }
    }
}

impl Default for CliObserver {
    fn default() -> Self {
        Self::new()
    }
}
