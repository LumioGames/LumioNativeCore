//! Atomic admission/closing gate. Close vs admit share one linearization point.

use std::sync::atomic::{AtomicU32, Ordering};

use crate::error::{ErrorCategory, ErrorDetail, KernelError};

const ORDER: Ordering = Ordering::SeqCst;

const CREATING: u32 = 0;
const RUNNING: u32 = 1;
const QUIESCING: u32 = 2;
const CLOSED: u32 = 3;
const FAULTED: u32 = 4;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ContextPhase {
    Creating,
    Running,
    Quiescing,
    Closed,
    Faulted,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ContextStateSnapshot {
    pub phase: ContextPhase,
}

pub struct ContextStateGate {
    phase: AtomicU32,
}

impl ContextStateGate {
    pub fn new_running() -> Self {
        Self {
            phase: AtomicU32::new(RUNNING),
        }
    }

    pub fn snapshot(&self) -> ContextStateSnapshot {
        ContextStateSnapshot {
            phase: decode(self.phase.load(ORDER)),
        }
    }

    pub fn try_admit(&self) -> Result<(), KernelError> {
        match decode(self.phase.load(ORDER)) {
            ContextPhase::Running => Ok(()),
            ContextPhase::Quiescing => Err(KernelError::new(
                ErrorCategory::ContextClosing,
                ErrorDetail::None,
            )),
            ContextPhase::Creating | ContextPhase::Closed | ContextPhase::Faulted => Err(
                KernelError::new(ErrorCategory::ContextDestroyed, ErrorDetail::None),
            ),
        }
    }

    pub fn begin_close(&self) -> bool {
        self.phase
            .compare_exchange(RUNNING, QUIESCING, ORDER, ORDER)
            .is_ok()
    }

    pub fn mark_closed(&self) {
        self.phase.store(CLOSED, ORDER);
    }
}

fn decode(raw: u32) -> ContextPhase {
    match raw {
        CREATING => ContextPhase::Creating,
        RUNNING => ContextPhase::Running,
        QUIESCING => ContextPhase::Quiescing,
        CLOSED => ContextPhase::Closed,
        FAULTED => ContextPhase::Faulted,
        _ => ContextPhase::Faulted,
    }
}
