//! Cancellation primitives for runtime execution.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

/// Cooperative cancellation token for long-running tasks.
#[derive(Clone, Debug, Default)]
pub struct CancellationToken {
    cancelled: Arc<AtomicBool>,
    reason: Arc<Mutex<Option<String>>>,
}

impl CancellationToken {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cancel(&self, reason: impl Into<String>) {
        self.cancelled.store(true, Ordering::SeqCst);
        let mut guard = self.reason.lock().unwrap();
        *guard = Some(reason.into());
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }

    pub fn reason(&self) -> Option<String> {
        self.reason.lock().unwrap().clone()
    }

    pub fn abort_reason(&self) -> String {
        self.reason().unwrap_or_else(|| "cancelled".to_string())
    }
}
