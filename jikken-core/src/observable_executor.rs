use crate::{
    config,
    executor::{ExecutionPolicy, StageResult, State},
    observer::{ExecutionEvent, ExecutionObserver},
    telemetry, test,
};
use std::{error::Error, sync::Arc};

/// Wrapper that adds observer capabilities to any ExecutionPolicy
pub struct ObservableExecutionPolicy<P: ExecutionPolicy> {
    inner: P,
    observer: Option<Box<dyn ExecutionObserver>>,
}

impl<P: ExecutionPolicy> ObservableExecutionPolicy<P> {
    pub fn new(policy: P, observer: Option<Box<dyn ExecutionObserver>>) -> Self {
        Self {
            inner: policy,
            observer,
        }
    }

    fn emit_event(&mut self, event: ExecutionEvent) {
        if let Some(observer) = &mut self.observer {
            observer.on_event(event);
        }
    }

    fn should_cancel(&self) -> bool {
        self.observer
            .as_ref()
            .map(|o| o.should_cancel())
            .unwrap_or(false)
    }
}

impl<P: ExecutionPolicy> ExecutionPolicy for ObservableExecutionPolicy<P> {
    fn name(&self) -> String {
        self.inner.name()
    }

    fn new_line(&self) -> bool {
        self.inner.new_line()
    }

    async fn execute(
        &mut self,
        state: &mut State,
        telemetry: &Option<telemetry::Session>,
        test: &test::Definition,
        iteration: u32,
        config: &config::Config,
    ) -> Result<(bool, Vec<StageResult>), Box<dyn Error + Send + Sync>> {
        // Check for cancellation before starting
        if self.should_cancel() {
            self.emit_event(ExecutionEvent::ExecutionCancelled {
                test: Some(Arc::new(test.clone())),
                stage: None,
                reason: "Execution cancelled by user".to_string(),
            });
            return Err("Execution cancelled".into());
        }

        // Emit test start event
        let test_arc = Arc::new(test.clone());
        self.emit_event(ExecutionEvent::TestStart {
            test: test_arc.clone(),
            iteration: iteration as usize,
            total_iterations: test.iterate as usize,
        });
        
        // Execute the inner policy
        match self.inner.execute(state, telemetry, test, iteration, config).await {
            Ok((passed, stage_results)) => {
                // Emit stage complete events for each stage
                for (stage_index, stage_result) in stage_results.iter().enumerate() {
                    // Use the actual stage from the test definition if available
                    let stage_descriptor = if let Some(stage) = test.stages.get(stage_index) {
                        Arc::new(stage.clone())
                    } else {
                        // Create a minimal stage descriptor as fallback
                        Arc::new(crate::test::definition::StageDescriptor {
                            request: crate::test::definition::RequestDescriptor {
                                method: crate::test::http::Verb::Get,
                                url: "".to_string(),
                                params: Vec::new(),
                                headers: Vec::new(),
                                body: None,
                            },
                            compare: None,
                            response: None,
                            variables: Vec::new(),
                            name: stage_result.stage_name.clone(),
                            delay: None,
                        })
                    };

                    self.emit_event(ExecutionEvent::StageComplete {
                        test: test_arc.clone(),
                        stage: stage_descriptor,
                        result: stage_result.clone(),
                    });
                }

                // Calculate total runtime
                let total_runtime: u32 = stage_results.iter().map(|r| r.total_runtime).sum();
                
                // Emit test complete event
                self.emit_event(ExecutionEvent::TestComplete {
                    test: test_arc,
                    test_name: test.name.clone().unwrap_or_default(),
                    passed,
                    iteration_count: test.iterate,
                    runtime_ms: total_runtime,
                });
                
                Ok((passed, stage_results))
            }
            Err(e) => {
                self.emit_event(ExecutionEvent::Error {
                    test: Some(test_arc.clone()),
                    stage: None,
                    error: e.to_string(),
                });
                
                // Also emit test complete with failed status
                self.emit_event(ExecutionEvent::TestComplete {
                    test: test_arc,
                    test_name: test.name.clone().unwrap_or_default(),
                    passed: false,
                    iteration_count: test.iterate,
                    runtime_ms: 0,
                });
                
                Err(e)
            }
        }
    }

    async fn skip(
        &mut self,
        telemetry: &Option<telemetry::Session>,
        test: &test::Definition,
        config: &config::Config,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        self.emit_event(ExecutionEvent::TestSkipped {
            test: Arc::new(test.clone()),
            reason: "Test skipped due to unmet requirements".to_string(),
        });
        
        self.inner.skip(telemetry, test, config).await
    }
}

/// Extension trait to make any ExecutionPolicy observable
pub trait ExecutionPolicyExt: ExecutionPolicy + Sized {
    fn with_observer(self, observer: Option<Box<dyn ExecutionObserver>>) -> ObservableExecutionPolicy<Self> {
        ObservableExecutionPolicy::new(self, observer)
    }
}

// Implement the extension trait for all ExecutionPolicy implementations
impl<T: ExecutionPolicy> ExecutionPolicyExt for T {}