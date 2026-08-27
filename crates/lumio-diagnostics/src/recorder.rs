//! Bounded non-blocking recorder with drop counters.
//!
//! Kernel has no `RecordPort` yet. Local methods only; producer never waits
//! for drain.

use std::sync::atomic::{AtomicU64, Ordering};

use lumio_kernel::error::{ErrorCategory, ErrorDetail, KernelError, KernelResult};

use crate::queue::RecordQueue;
use crate::record::{KernelRecordRef, OwnedKernelRecord};

const ORDER: Ordering = Ordering::SeqCst;

/// Local field cap (kernel RecordPort unpublished).
const MAX_RECORD_FIELDS: usize = 8;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RecorderCounters {
    pub accepted: u64,
    pub dropped_full: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RecordDisposition {
    Accepted,
    DroppedFull,
    DroppedOversized,
}

pub struct BoundedRecorder {
    queue: RecordQueue,
    max_record_bytes: usize,
    accepted: AtomicU64,
    dropped_full: AtomicU64,
}

impl BoundedRecorder {
    pub fn with_capacity(capacity: usize, max_record_bytes: usize) -> KernelResult<Self> {
        if capacity == 0 {
            return Err(KernelError::new(
                ErrorCategory::InvalidArgument,
                ErrorDetail::None,
            ));
        }
        Ok(Self {
            queue: RecordQueue::with_capacity(capacity),
            max_record_bytes,
            accepted: AtomicU64::new(0),
            dropped_full: AtomicU64::new(0),
        })
    }

    /// Non-blocking. Oversized or queue-full never waits for drain.
    /// Queue-full increments `dropped_full`; oversized is `DroppedOversized`.
    pub fn try_record(&self, r: KernelRecordRef<'_>) -> RecordDisposition {
        let owned =
            match OwnedKernelRecord::try_from_ref(r, MAX_RECORD_FIELDS, self.max_record_bytes) {
                Ok(owned) => owned,
                Err(_) => return RecordDisposition::DroppedOversized,
            };
        match self.queue.try_push(owned) {
            Ok(()) => {
                self.accepted.fetch_add(1, ORDER);
                RecordDisposition::Accepted
            }
            Err(_) => {
                self.dropped_full.fetch_add(1, ORDER);
                RecordDisposition::DroppedFull
            }
        }
    }

    pub fn counters(&self) -> RecorderCounters {
        RecorderCounters {
            accepted: self.accepted.load(ORDER),
            dropped_full: self.dropped_full.load(ORDER),
        }
    }

    pub fn drain(&self, out: &mut [OwnedKernelRecord]) -> usize {
        let mut n = 0;
        for slot in out.iter_mut() {
            match self.queue.try_pop() {
                Some(record) => {
                    *slot = record;
                    n += 1;
                }
                None => break,
            }
        }
        n
    }
}
