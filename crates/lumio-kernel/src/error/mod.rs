//! Kernel error facade: stable categories, bounded detail, and constructors.

mod category;
mod mapping;

pub use category::{ErrorCategory, ErrorDetail, KernelError};
pub use mapping::{MappingBlocked, to_architecture_error_code};

pub type KernelResult<T> = Result<T, KernelError>;

impl KernelError {
    pub const fn buffer_too_small(required: u64, provided: u64) -> Self {
        Self::new(
            ErrorCategory::BufferTooSmall,
            ErrorDetail::RequiredCapacity { required, provided },
        )
    }
}
