//! Bounded job/completion queue adapter.
//!
//! `crossbeam-channel` is not approved (`EXTERNAL_ALLOWLIST` is empty).
//! Fallback: `std::sync::mpsc::sync_channel`. `Receiver` is not `Sync`, so
//! `try_pop` serializes through a mutex; `try_send` / `try_recv` never block.

use std::sync::Mutex;
use std::sync::mpsc::{self, Receiver, SyncSender, TrySendError};

use lumio_kernel::error::{ErrorCategory, ErrorDetail, KernelError};

pub struct BoundedJobQueue<T> {
    tx: SyncSender<T>,
    rx: Mutex<Receiver<T>>,
    cap: usize,
}

impl<T: Send> BoundedJobQueue<T> {
    pub fn with_capacity(cap: usize) -> Self {
        let (tx, rx) = mpsc::sync_channel(cap);
        Self {
            tx,
            rx: Mutex::new(rx),
            cap,
        }
    }

    /// Non-blocking push. Full → `CapacityExceeded` immediately.
    pub fn try_push(&self, item: T) -> Result<(), KernelError> {
        match self.tx.try_send(item) {
            Ok(()) => Ok(()),
            Err(TrySendError::Full(_)) => Err(KernelError::new(
                ErrorCategory::CapacityExceeded,
                ErrorDetail::LimitExceeded {
                    limit: self.cap as u64,
                    requested: self.cap as u64 + 1,
                },
            )),
            Err(TrySendError::Disconnected(_)) => Err(KernelError::new(
                ErrorCategory::InternalInvariant,
                ErrorDetail::None,
            )),
        }
    }

    pub fn try_pop(&self) -> Option<T> {
        let rx = self.rx.lock().unwrap_or_else(|p| p.into_inner());
        rx.try_recv().ok()
    }

    pub fn cap(&self) -> usize {
        self.cap
    }
}
