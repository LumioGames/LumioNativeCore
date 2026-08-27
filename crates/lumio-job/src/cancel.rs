//! Cooperative cancellation. The flag is sticky: once true, it stays true.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

const ORDER: Ordering = Ordering::SeqCst;

/// Owner of a shared cancellation flag.
pub struct CancellationSource {
    flag: Arc<AtomicBool>,
}

/// Cheap clone that observes the same flag as its source.
#[derive(Clone)]
pub struct CancellationView {
    flag: Arc<AtomicBool>,
}

impl CancellationSource {
    pub fn new() -> Self {
        Self {
            flag: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn view(&self) -> CancellationView {
        CancellationView {
            flag: Arc::clone(&self.flag),
        }
    }

    pub fn cancel(&self) {
        self.flag.store(true, ORDER);
    }

    pub fn is_cancelled(&self) -> bool {
        self.flag.load(ORDER)
    }
}

impl CancellationView {
    pub fn is_cancelled(&self) -> bool {
        self.flag.load(ORDER)
    }
}
