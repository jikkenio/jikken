#[cfg(test)]
mod tests {
    use crate::config::Config;
    use crate::executor::execute_tests_with_observer;
    use crate::observer::{ExecutionEvent, ExecutionObserver, NoOpObserver};
    use std::sync::{Arc, Mutex};

    // Test observer that collects events for verification
    struct TestObserver {
        events: Arc<Mutex<Vec<String>>>,
    }

    impl TestObserver {
        fn new() -> (Self, Arc<Mutex<Vec<String>>>) {
            let events = Arc::new(Mutex::new(Vec::new()));
            (
                Self {
                    events: events.clone(),
                },
                events,
            )
        }
    }

    impl ExecutionObserver for TestObserver {
        fn on_event(&mut self, event: ExecutionEvent) {
            if let Ok(mut events) = self.events.lock() {
                match event {
                    ExecutionEvent::SuiteStart { total_tests, .. } => {
                        events.push(format!("SuiteStart: {} tests", total_tests));
                    }
                    ExecutionEvent::TestStart {
                        test, iteration, ..
                    } => {
                        events.push(format!(
                            "TestStart: {} iteration {}",
                            test.name.clone().unwrap_or_default(),
                            iteration
                        ));
                    }
                    ExecutionEvent::TestComplete { test, .. } => {
                        events.push(format!(
                            "TestComplete: {}",
                            test.name.clone().unwrap_or_default()
                        ));
                    }
                    ExecutionEvent::TestSkipped { test, reason } => {
                        events.push(format!(
                            "TestSkipped: {} ({})",
                            test.name.clone().unwrap_or_default(),
                            reason
                        ));
                    }
                    ExecutionEvent::SuiteComplete {
                        total_tests,
                        passed,
                        failed,
                        ..
                    } => {
                        events.push(format!(
                            "SuiteComplete: {}/{} passed, {} failed",
                            passed, total_tests, failed
                        ));
                    }
                    ExecutionEvent::Error { error, .. } => {
                        events.push(format!("Error: {}", error));
                    }
                    ExecutionEvent::ExecutionCancelled { reason, .. } => {
                        events.push(format!("Cancelled: {}", reason));
                    }
                    _ => {
                        events.push("OtherEvent".to_string());
                    }
                }
            }
        }
    }

    #[tokio::test]
    async fn test_observer_integration_with_empty_tests() {
        // Create test observer
        let (observer, events) = TestObserver::new();

        // Create minimal config
        let config = Config::default();

        // Execute with empty test list
        let report = execute_tests_with_observer(
            config,
            vec![],                          // empty tests
            true,                            // dry run
            vec![],                          // no ignored tests
            None,                            // no junit file
            Box::new(serde_json::json!({})), // empty cli args
            Some(Box::new(observer)),
        )
        .await;

        // Verify report
        assert_eq!(report.run, 0);
        assert_eq!(report.passed, 0);
        assert_eq!(report.failed, 0);

        // Verify events were emitted
        let captured_events = events.lock().unwrap();
        assert!(
            !captured_events.is_empty(),
            "Should have emitted some events"
        );

        // Should at least have suite start event
        assert!(captured_events.iter().any(|e| e.starts_with("SuiteStart")));
        // Note: SuiteComplete is not emitted in the current architecture since observer is consumed
    }

    #[test]
    fn test_no_op_observer() {
        let mut observer = NoOpObserver;

        // Should not panic or do anything
        observer.on_event(ExecutionEvent::SuiteStart {
            total_tests: 5,
            tags: vec![],
        });

        assert!(!observer.should_cancel());
    }

    #[test]
    fn test_execution_events_are_cloneable() {
        let event = ExecutionEvent::SuiteStart {
            total_tests: 10,
            tags: vec!["test".to_string()],
        };

        let cloned_event = event.clone();

        // Should be able to clone events (needed for CompositeObserver)
        match (event, cloned_event) {
            (
                ExecutionEvent::SuiteStart {
                    total_tests: t1, ..
                },
                ExecutionEvent::SuiteStart {
                    total_tests: t2, ..
                },
            ) => {
                assert_eq!(t1, t2);
            }
            _ => panic!("Event clone failed"),
        }
    }
}
