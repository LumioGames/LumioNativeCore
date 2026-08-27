//! Creation-time context configuration.

use crate::capability::ConfiguredLimits;
use crate::error::KernelResult;
use lumio_platform::Deadline;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ContextConfig {
    pub limits: ConfiguredLimits,
    pub quiesce_deadline: Deadline,
}

impl ContextConfig {
    pub fn validate(&self) -> KernelResult<()> {
        self.limits.validate()
    }
}
