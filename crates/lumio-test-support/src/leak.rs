//! Owner-count snapshot for leak assertions. Values come from the owners.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LeakSnapshot {
    pub handles_live: u64,
    pub native_bytes_charged: u64,
    pub leases_live: u64,
    pub jobs_non_terminal: u64,
}

impl LeakSnapshot {
    pub fn zero() -> Self {
        Self {
            handles_live: 0,
            native_bytes_charged: 0,
            leases_live: 0,
            jobs_non_terminal: 0,
        }
    }

    pub fn is_clean(&self) -> bool {
        *self == Self::zero()
    }
}
