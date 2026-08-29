//! Architecture ErrorCode mapping — the single category→code conversion.
//!
//! ADR-046 (Draft; values published in `ids/index.json` on `origin/main`)
//! allocates the kernel status band 1044–1053 and adjudicates three
//! categories onto existing values (1020/1029/1030), so every frozen
//! `ErrorCategory` now maps to a registered numeric. Codes are resolved
//! through the generated registry by id string — no numeric is written here,
//! and an unregistered non-zero status cannot originate from this mapping
//! (ADR-046 §4).
//!
//! `AlreadyReleased` maps to `HandleDoubleRelease` (1030), the §2 release-path
//! ruling. Known gap: the arena also reports empty-slot hits on *use* paths
//! as `AlreadyReleased`, which §2 expects to surface as `InvalidHandle`
//! (1029); resolving that is a handle-module check-order question outside
//! this mapping (tracked in the R-00079 delivery notes).

use lumio_contract_types::{ArchitectureErrorCode, registry};

use super::{ErrorCategory, KernelError};

/// Panics only if the generated registry lost a registered id — that is a
/// broken build of the generated tables, not a runtime condition.
fn registered(id: &str) -> ArchitectureErrorCode {
    registry::error_code(id).unwrap_or_else(|| panic!("generated ids registry is missing `{id}`"))
}

/// Maps a kernel error to its registered architecture ErrorCode.
pub fn to_architecture_error_code(error: &KernelError) -> ArchitectureErrorCode {
    let id = match error.category() {
        ErrorCategory::InvalidArgument => "InvalidArgument",
        ErrorCategory::InvalidHandle => "InvalidHandle",
        ErrorCategory::WrongContext => "WrongContext",
        ErrorCategory::AlreadyReleased => "HandleDoubleRelease",
        ErrorCategory::BufferTooSmall => "BufferTooSmall",
        ErrorCategory::CapacityExceeded => "CapacityExceeded",
        ErrorCategory::CapabilityUnavailable => "CapabilityMissing",
        ErrorCategory::Cancelled => "Cancelled",
        ErrorCategory::TimedOut => "TimedOut",
        ErrorCategory::ContextClosing => "ContextClosing",
        ErrorCategory::ContextDestroyed => "ContextDestroyed",
        ErrorCategory::PanicBoundary => "PanicBoundary",
        ErrorCategory::InternalInvariant => "InternalInvariant",
    };
    registered(id)
}
