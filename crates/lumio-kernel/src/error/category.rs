//! Internal error category and bounded diagnostic detail.
//!
//! Detail holds only scalars and `&'static str`. It must not own `String`,
//! a backtrace, or a third-party error type. Public numeric ErrorCodes are
//! out of scope (Blocked ABI / T-error-03).

/// Primary failure class. Each `KernelError` carries exactly one.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ErrorCategory {
    InvalidArgument,
    InvalidHandle,
    WrongContext,
    AlreadyReleased,
    BufferTooSmall,
    CapacityExceeded,
    CapabilityUnavailable,
    Cancelled,
    TimedOut,
    ContextClosing,
    ContextDestroyed,
    PanicBoundary,
    InternalInvariant,
}

/// Bounded context attached to a category. Closed set: no owned `String`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ErrorDetail {
    None,
    RequiredCapacity { required: u64, provided: u64 },
    LimitExceeded { limit: u64, requested: u64 },
    StaticMessage(&'static str),
}

/// Immutable kernel failure: one category plus bounded detail.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct KernelError {
    category: ErrorCategory,
    detail: ErrorDetail,
}

impl KernelError {
    pub const fn new(category: ErrorCategory, detail: ErrorDetail) -> Self {
        Self { category, detail }
    }

    pub const fn category(&self) -> ErrorCategory {
        self.category
    }

    pub fn detail(&self) -> &ErrorDetail {
        &self.detail
    }
}
