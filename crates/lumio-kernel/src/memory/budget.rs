//! Thread-safe charged-byte ledger. Reserve before allocate.

use std::sync::atomic::{AtomicU64, Ordering};

use crate::error::{ErrorCategory, ErrorDetail, KernelError};

/// Atomic charged-bytes counter bounded by `max_native_bytes`.
pub struct MemoryBudget {
    charged: AtomicU64,
    limit: u64,
}

impl MemoryBudget {
    pub fn new(max_native_bytes: u64) -> Self {
        Self {
            charged: AtomicU64::new(0),
            limit: max_native_bytes,
        }
    }

    /// CAS loop: concurrent reserves cannot push charged past `limit`.
    pub fn try_reserve(&self, bytes: u64) -> Result<(), KernelError> {
        let mut current = self.charged.load(Ordering::Acquire);
        loop {
            let Some(next) = current
                .checked_add(bytes)
                .filter(|next| *next <= self.limit)
            else {
                return Err(KernelError::new(
                    ErrorCategory::CapacityExceeded,
                    ErrorDetail::LimitExceeded {
                        limit: self.limit,
                        requested: current.saturating_add(bytes),
                    },
                ));
            };
            match self.charged.compare_exchange_weak(
                current,
                next,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => return Ok(()),
                Err(actual) => current = actual,
            }
        }
    }

    /// Saturates at zero so extra release cannot wrap the ledger.
    pub fn release(&self, bytes: u64) {
        let mut current = self.charged.load(Ordering::Acquire);
        loop {
            let next = current.saturating_sub(bytes);
            match self.charged.compare_exchange_weak(
                current,
                next,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => return,
                Err(actual) => current = actual,
            }
        }
    }

    pub fn charged(&self) -> u64 {
        self.charged.load(Ordering::Acquire)
    }

    pub fn limit(&self) -> u64 {
        self.limit
    }
}
