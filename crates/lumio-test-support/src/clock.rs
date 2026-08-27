//! Deterministic monotonic clock for tests. Never sleeps; never moves backwards.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use lumio_platform::{MonotonicClock, Ticks};

/// Process-local fake clock driven only by `advance` / `set_forward`.
pub struct FakeClock {
    now_nanos: AtomicU64,
}

/// Error from an attempted non-monotonic `set_forward`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FakeClockError {
    Backward,
}

impl FakeClock {
    pub fn new(initial: Ticks) -> Self {
        Self {
            now_nanos: AtomicU64::new(initial.as_nanos()),
        }
    }

    /// Saturating add; never wraps and never sleeps.
    pub fn advance(&self, d: Duration) {
        let _ = self
            .now_nanos
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |now| {
                Some(Ticks::from_nanos(now).saturating_add(d).as_nanos())
            });
    }

    /// Rejects `t` strictly before the current reading.
    pub fn set_forward(&self, t: Ticks) -> Result<(), FakeClockError> {
        let target = t.as_nanos();
        let mut current = self.now_nanos.load(Ordering::SeqCst);
        loop {
            if target < current {
                return Err(FakeClockError::Backward);
            }
            match self.now_nanos.compare_exchange_weak(
                current,
                target,
                Ordering::SeqCst,
                Ordering::SeqCst,
            ) {
                Ok(_) => return Ok(()),
                Err(actual) => current = actual,
            }
        }
    }
}

impl MonotonicClock for FakeClock {
    fn now(&self) -> Ticks {
        Ticks::from_nanos(self.now_nanos.load(Ordering::SeqCst))
    }
}
