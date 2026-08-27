//! Bounded diagnostics record queue adapter.
//!
//! `crossbeam-queue` is not approved (`EXTERNAL_ALLOWLIST` is empty).
//! Fallback: `std::sync::mpsc::sync_channel`. `Receiver` is not `Sync`, so
//! `try_pop` serializes through a mutex; `try_send` / `try_recv` never block.

use std::sync::Mutex;
use std::sync::mpsc::{self, Receiver, SyncSender, TrySendError};

use lumio_kernel::error::{ErrorCategory, ErrorDetail, KernelError, KernelResult};

use crate::record::OwnedKernelRecord;

pub struct RecordQueue {
    tx: SyncSender<OwnedKernelRecord>,
    rx: Mutex<Receiver<OwnedKernelRecord>>,
    cap: usize,
}

impl RecordQueue {
    pub fn with_capacity(cap: usize) -> Self {
        let (tx, rx) = mpsc::sync_channel(cap);
        Self {
            tx,
            rx: Mutex::new(rx),
            cap,
        }
    }

    /// Non-blocking. Full → Err CapacityExceeded immediately, item not queued.
    pub fn try_push(&self, r: OwnedKernelRecord) -> KernelResult<()> {
        match self.tx.try_send(r) {
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

    pub fn try_pop(&self) -> Option<OwnedKernelRecord> {
        let rx = self.rx.lock().unwrap_or_else(|p| p.into_inner());
        rx.try_recv().ok()
    }
}
