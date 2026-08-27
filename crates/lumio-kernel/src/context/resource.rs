//! Object-safe port for resources owned by a KernelContext.

use lumio_platform::Deadline;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CancelReason {
    ContextClosing,
    ContextFaulted,
    OwnerRequested,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum QuiesceState {
    Quiesced,
    Pending { remaining: u32 },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct QuiesceReport {
    pub state: QuiesceState,
}

pub trait ContextResource: Send + Sync + 'static {
    fn name(&self) -> &'static str;
    fn cancel_requested(&self, reason: CancelReason);
    fn quiesce(&self, deadline: Deadline) -> crate::error::KernelResult<QuiesceReport>;
    fn destroy(&self) -> crate::error::KernelResult<()>;
}
