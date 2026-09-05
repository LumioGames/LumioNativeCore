//! Kernel error facade: internal categories, bounded detail, and constructors.
//!
//! `ErrorCategory` is an **internal** enum: it carries no cross-boundary
//! numeric and projects onto no published registry. Mapping a failure onto a
//! status code that managed callers see belongs to the SDK plug in the
//! architecture repository, against `engine/abi/native-abi.json` (ADR 0009).

mod category;

pub use category::{ErrorCategory, ErrorDetail, KernelError};

pub type KernelResult<T> = Result<T, KernelError>;

impl KernelError {
    pub const fn buffer_too_small(required: u64, provided: u64) -> Self {
        Self::new(
            ErrorCategory::BufferTooSmall,
            ErrorDetail::RequiredCapacity { required, provided },
        )
    }
}
