//! Instantaneous runtime resource status. Not part of the capability snapshot.

use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};

const ORDER: Ordering = Ordering::SeqCst;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RuntimeStatus {
    pub accepting_work: bool,
    pub queued_jobs: u32,
    pub running_jobs: u32,
    pub allocated_native_bytes: u64,
}

pub struct RuntimeCounters {
    accepting_work: AtomicBool,
    queued_jobs: AtomicU32,
    running_jobs: AtomicU32,
    allocated_native_bytes: AtomicU64,
}

impl RuntimeCounters {
    pub fn new() -> Self {
        Self {
            accepting_work: AtomicBool::new(true),
            queued_jobs: AtomicU32::new(0),
            running_jobs: AtomicU32::new(0),
            allocated_native_bytes: AtomicU64::new(0),
        }
    }

    pub fn set_accepting_work(&self, v: bool) {
        self.accepting_work.store(v, ORDER);
    }

    pub fn add_queued(&self, d: i32) {
        saturating_add_u32(&self.queued_jobs, d);
    }

    pub fn add_running(&self, d: i32) {
        saturating_add_u32(&self.running_jobs, d);
    }

    pub fn add_bytes(&self, d: i64) {
        saturating_add_u64(&self.allocated_native_bytes, d);
    }

    pub fn snapshot(&self) -> RuntimeStatus {
        RuntimeStatus {
            accepting_work: self.accepting_work.load(ORDER),
            queued_jobs: self.queued_jobs.load(ORDER),
            running_jobs: self.running_jobs.load(ORDER),
            allocated_native_bytes: self.allocated_native_bytes.load(ORDER),
        }
    }
}

impl Default for RuntimeCounters {
    fn default() -> Self {
        Self::new()
    }
}

fn saturating_add_u32(atom: &AtomicU32, d: i32) {
    let _ = atom.fetch_update(ORDER, ORDER, |cur| {
        Some(if d >= 0 {
            cur.saturating_add(d as u32)
        } else {
            cur.saturating_sub(d.unsigned_abs())
        })
    });
}

fn saturating_add_u64(atom: &AtomicU64, d: i64) {
    let _ = atom.fetch_update(ORDER, ORDER, |cur| {
        Some(if d >= 0 {
            cur.saturating_add(d as u64)
        } else {
            cur.saturating_sub(d.unsigned_abs())
        })
    });
}
