//! Configured resource limits, frozen at Context creation.

use crate::error::{ErrorCategory, ErrorDetail, KernelError};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ConfiguredLimits {
    pub max_handles: u32,
    pub max_native_bytes: u64,
    pub max_jobs_queued: u32,
    pub max_jobs_running: u32,
    pub max_completion_items: u32,
}

impl ConfiguredLimits {
    pub fn validate(&self) -> Result<(), KernelError> {
        if self.max_handles == 0
            || self.max_native_bytes == 0
            || self.max_jobs_queued == 0
            || self.max_jobs_running == 0
            || self.max_completion_items == 0
        {
            return Err(KernelError::new(
                ErrorCategory::InvalidArgument,
                ErrorDetail::None,
            ));
        }
        Ok(())
    }
}
