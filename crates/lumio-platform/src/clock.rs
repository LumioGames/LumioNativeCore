//! Process-relative monotonic ticks. Not a wall clock and not TickId.

use core::time::Duration;
use std::time::Instant;

/// Nanoseconds since a clock's process-local epoch. Not comparable across processes
/// and not an input to authoritative hashes.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Ticks(u64);

impl Ticks {
    pub const ZERO: Self = Self(0);

    pub const fn from_nanos(v: u64) -> Self {
        Self(v)
    }

    pub const fn as_nanos(self) -> u64 {
        self.0
    }

    /// `None` when `self + d` exceeds `u64::MAX` nanos. Never wraps.
    pub fn checked_add(self, d: Duration) -> Option<Self> {
        let add = u64::try_from(d.as_nanos()).ok()?;
        self.0.checked_add(add).map(Self)
    }

    /// Saturates at `u64::MAX` nanos instead of wrapping.
    pub fn saturating_add(self, d: Duration) -> Self {
        self.checked_add(d).unwrap_or(Self(u64::MAX))
    }

    /// `Duration::ZERO` when `earlier` is not before `self`.
    pub fn saturating_duration_since(self, earlier: Self) -> Duration {
        Duration::from_nanos(self.0.saturating_sub(earlier.0))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Deadline(Option<Ticks>);

impl Deadline {
    /// Sentinel: never expires.
    pub const NONE: Self = Self(None);

    pub const fn at(t: Ticks) -> Self {
        Self(Some(t))
    }

    /// `NONE` never expires. A finite deadline expires at `now >= t`.
    pub fn is_expired(self, now: Ticks) -> bool {
        match self.0 {
            None => false,
            Some(t) => now >= t,
        }
    }
}

pub trait MonotonicClock: Send + Sync + 'static {
    fn now(&self) -> Ticks;
}

/// Monotonic clock whose epoch is `Instant::now()` at construction.
pub struct StdMonotonicClock {
    epoch: Instant,
}

impl Default for StdMonotonicClock {
    fn default() -> Self {
        Self::new()
    }
}

impl StdMonotonicClock {
    pub fn new() -> Self {
        Self {
            epoch: Instant::now(),
        }
    }
}

impl MonotonicClock for StdMonotonicClock {
    fn now(&self) -> Ticks {
        Ticks::ZERO.saturating_add(self.epoch.elapsed())
    }
}
