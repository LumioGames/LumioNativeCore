//! Architecture ErrorCode mapping seam.
//!
//! The generated registry is empty and `ArchitectureErrorCode` has no public
//! constructor, so every category is `Err(MappingBlocked)`. That is the
//! blocked-ABI seam; public numeric mapping is not complete.

use lumio_contract_types::ArchitectureErrorCode;

use super::{ErrorCategory, KernelError};

/// Architecture ErrorCode values are unpublished; mapping cannot complete.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MappingBlocked;

/// Maps a kernel error to an architecture ErrorCode, or the blocked seam.
pub fn to_architecture_error_code(
    error: &KernelError,
) -> Result<ArchitectureErrorCode, MappingBlocked> {
    match error.category() {
        ErrorCategory::InvalidArgument => Err(MappingBlocked),
        ErrorCategory::InvalidHandle => Err(MappingBlocked),
        ErrorCategory::WrongContext => Err(MappingBlocked),
        ErrorCategory::AlreadyReleased => Err(MappingBlocked),
        ErrorCategory::BufferTooSmall => Err(MappingBlocked),
        ErrorCategory::CapacityExceeded => Err(MappingBlocked),
        ErrorCategory::CapabilityUnavailable => Err(MappingBlocked),
        ErrorCategory::Cancelled => Err(MappingBlocked),
        ErrorCategory::TimedOut => Err(MappingBlocked),
        ErrorCategory::ContextClosing => Err(MappingBlocked),
        ErrorCategory::ContextDestroyed => Err(MappingBlocked),
        ErrorCategory::PanicBoundary => Err(MappingBlocked),
        ErrorCategory::InternalInvariant => Err(MappingBlocked),
    }
}
