//! Single-point CAS linearization of job execution state.
//!
//! `TimedOut` is a completion observation (ADR 0004), not a CAS target.

use std::sync::atomic::{AtomicU8, Ordering};

const ORDER: Ordering = Ordering::SeqCst;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum JobState {
    Queued = 0,
    Running = 1,
    Succeeded = 2,
    Failed = 3,
    Cancelled = 4,
    TimedOut = 5,
}

/// Slot CAS cell. Same type as [`JobStateMachine`].
pub type JobStateCell = JobStateMachine;

pub struct JobStateMachine {
    state: AtomicU8,
}

impl JobStateMachine {
    pub fn queued() -> Self {
        Self {
            state: AtomicU8::new(JobState::Queued as u8),
        }
    }

    pub fn snapshot(&self) -> JobState {
        decode(self.state.load(ORDER))
    }

    /// Queued → Running.
    pub fn cas_start(&self) -> Result<(), JobState> {
        self.state
            .compare_exchange(
                JobState::Queued as u8,
                JobState::Running as u8,
                ORDER,
                ORDER,
            )
            .map(|_| ())
            .map_err(decode)
    }

    /// Queued or Running → Cancelled. `Ok` is the pre-CAS state.
    pub fn cas_cancel(&self) -> Result<JobState, JobState> {
        let current = self.snapshot();
        match current {
            JobState::Queued | JobState::Running => self
                .state
                .compare_exchange(current as u8, JobState::Cancelled as u8, ORDER, ORDER)
                .map(|_| current)
                .map_err(decode),
            other => Err(other),
        }
    }

    /// Running → Succeeded or Failed.
    pub fn cas_complete(&self, to: JobState) -> Result<(), JobState> {
        if !matches!(to, JobState::Succeeded | JobState::Failed) {
            return Err(self.snapshot());
        }
        self.state
            .compare_exchange(JobState::Running as u8, to as u8, ORDER, ORDER)
            .map(|_| ())
            .map_err(decode)
    }
}

fn decode(raw: u8) -> JobState {
    match raw {
        0 => JobState::Queued,
        1 => JobState::Running,
        2 => JobState::Succeeded,
        3 => JobState::Failed,
        4 => JobState::Cancelled,
        5 => JobState::TimedOut,
        _ => unreachable!("invalid job state encoding"),
    }
}
