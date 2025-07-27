use crate::executor::StageResult;
use crate::test::{self};
use std::sync::Arc;
use std::time::Duration;

/// Events emitted during test execution
#[derive(Debug, Clone)]
pub enum ExecutionEvent {
    /// Test execution is starting
    TestStart {
        test: Arc<test::Definition>,
        iteration: usize,
        total_iterations: usize,
    },
    /// Test execution completed
    TestComplete {
        test: Arc<test::Definition>,
        test_name: String,
        passed: bool,
        iteration_count: u32,
        runtime_ms: u32,
    },
    /// Test was skipped
    TestSkipped {
        test: Arc<test::Definition>,
        reason: String,
    },
    /// Stage execution is starting
    StageStart {
        test: Arc<test::Definition>,
        stage: Arc<test::definition::StageDescriptor>,
        stage_type: StageType,
        iteration: usize,
    },
    /// Stage execution completed
    StageComplete {
        test: Arc<test::Definition>,
        stage: Arc<test::definition::StageDescriptor>,
        result: StageResult,
    },
    /// HTTP request is about to be sent
    RequestPrepared {
        test: Arc<test::Definition>,
        stage: Arc<test::definition::StageDescriptor>,
        request: serde_json::Value, // Simplified for now
    },
    /// HTTP request was sent
    RequestSent {
        test: Arc<test::Definition>,
        stage: Arc<test::definition::StageDescriptor>,
    },
    /// HTTP response received
    ResponseReceived {
        test: Arc<test::Definition>,
        stage: Arc<test::definition::StageDescriptor>,
        response: serde_json::Value, // Simplified for now
        duration: Duration,
    },
    /// Variable was extracted from response
    VariableExtracted {
        test: Arc<test::Definition>,
        stage: Arc<test::definition::StageDescriptor>,
        name: String,
        value: String,
        source: String,
    },
    /// Variable resolution occurred
    VariableResolved {
        name: String,
        value: String,
        source: VariableSource,
    },
    /// Validation was performed
    ValidationPerformed {
        test: Arc<test::Definition>,
        stage: Arc<test::definition::StageDescriptor>,
        validation: serde_json::Value, // Simplified for now
    },
    /// Error occurred during execution
    Error {
        test: Option<Arc<test::Definition>>,
        stage: Option<Arc<test::definition::StageDescriptor>>,
        error: String, // Simplified for now
    },
    /// Execution was cancelled
    ExecutionCancelled {
        test: Option<Arc<test::Definition>>,
        stage: Option<Arc<test::definition::StageDescriptor>>,
        reason: String,
    },
    /// Execution suite started
    SuiteStart {
        total_tests: usize,
        tags: Vec<String>,
    },
    /// Execution suite completed
    SuiteComplete {
        total_tests: usize,
        passed: usize,
        failed: usize,
        skipped: usize,
        duration: Duration,
    },
}

/// Type of stage being executed
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StageType {
    Setup,
    Normal,
    Cleanup,
}

/// Source of a resolved variable
#[derive(Debug, Clone)]
pub enum VariableSource {
    Environment,
    Config,
    Extracted,
    Default,
    Generated,
}

/// Trait for observing test execution events
pub trait ExecutionObserver: Send + Sync {
    /// Called when an execution event occurs
    fn on_event(&mut self, event: ExecutionEvent);
    
    /// Called to check if execution should be cancelled
    fn should_cancel(&self) -> bool {
        false
    }
}

/// A no-op observer that does nothing
pub struct NoOpObserver;

impl ExecutionObserver for NoOpObserver {
    fn on_event(&mut self, _event: ExecutionEvent) {
        // Do nothing
    }
}

/// An observer that collects all events
/// Note: Disabled due to thread-safety issues with test definitions containing Cell<bool>
// #[derive(Default)]
// pub struct CollectingObserver {
//     pub events: Vec<ExecutionEvent>,
// }

// impl ExecutionObserver for CollectingObserver {
//     fn on_event(&mut self, event: ExecutionEvent) {
//         self.events.push(event);
//     }
// }

/// An observer that delegates to multiple observers
pub struct CompositeObserver {
    observers: Vec<Box<dyn ExecutionObserver>>,
}

impl CompositeObserver {
    pub fn new() -> Self {
        Self {
            observers: Vec::new(),
        }
    }
    
    pub fn add(&mut self, observer: Box<dyn ExecutionObserver>) {
        self.observers.push(observer);
    }
}

impl ExecutionObserver for CompositeObserver {
    fn on_event(&mut self, event: ExecutionEvent) {
        for observer in &mut self.observers {
            observer.on_event(event.clone());
        }
    }
    
    fn should_cancel(&self) -> bool {
        self.observers.iter().any(|o| o.should_cancel())
    }
}

/// Thread-safe wrapper for observers
pub struct ThreadSafeObserver {
    inner: Arc<std::sync::Mutex<Box<dyn ExecutionObserver>>>,
}

impl ThreadSafeObserver {
    pub fn new(observer: Box<dyn ExecutionObserver>) -> Self {
        Self {
            inner: Arc::new(std::sync::Mutex::new(observer)),
        }
    }
}

impl ExecutionObserver for ThreadSafeObserver {
    fn on_event(&mut self, event: ExecutionEvent) {
        if let Ok(mut observer) = self.inner.lock() {
            observer.on_event(event);
        }
    }
    
    fn should_cancel(&self) -> bool {
        self.inner.lock().map(|o| o.should_cancel()).unwrap_or(false)
    }
}

impl Clone for ThreadSafeObserver {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}