//! Static capability set, configured resource limits, and runtime status.
//! Public architecture bits remain blocked (B-ABI-002).

mod limits;
mod runtime;
mod static_set;

pub use limits::ConfiguredLimits;
pub use runtime::{RuntimeCounters, RuntimeStatus};
pub use static_set::{CapabilityKey, StaticCapabilities};

use crate::error::{ErrorCategory, ErrorDetail, KernelError};

pub struct CapabilitySource {
    static_caps: StaticCapabilities,
    limits: ConfiguredLimits,
    runtime: RuntimeCounters,
}

impl CapabilitySource {
    pub fn new(
        static_caps: StaticCapabilities,
        limits: ConfiguredLimits,
    ) -> Result<Self, KernelError> {
        limits.validate()?;
        Ok(Self {
            static_caps,
            limits,
            runtime: RuntimeCounters::new(),
        })
    }

    pub fn require(&self, key: CapabilityKey) -> Result<(), KernelError> {
        if self.static_caps.contains(key) {
            Ok(())
        } else {
            Err(KernelError::new(
                ErrorCategory::CapabilityUnavailable,
                ErrorDetail::None,
            ))
        }
    }

    pub fn limits(&self) -> ConfiguredLimits {
        self.limits
    }

    pub fn runtime_snapshot(&self) -> RuntimeStatus {
        self.runtime.snapshot()
    }

    pub fn counters(&self) -> &RuntimeCounters {
        &self.runtime
    }
}
