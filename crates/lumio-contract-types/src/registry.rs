//! Read-only Error / Capability / Operation registry queries.
//!
//! Tables stay empty until the architecture source publishes a generated
//! package. This crate must not hand-write public numeric ids.

use crate::generated::{ArchitectureErrorCode, ArchitectureOperationId, CapabilityBits};

/// Architecture error codes from the generated registry.
pub fn error_codes() -> impl Iterator<Item = ArchitectureErrorCode> {
    core::iter::empty()
}

/// Architecture operation ids from the generated registry.
pub fn operation_ids() -> impl Iterator<Item = ArchitectureOperationId> {
    core::iter::empty()
}

/// Capability bit entries from the generated registry.
pub fn capability_bits() -> impl Iterator<Item = CapabilityBits> {
    core::iter::empty()
}
