//! Kernel context configuration, admission/closing gate, resource port, and registry.

mod config;
mod lifecycle;
mod registry;
mod resource;
mod state;

pub use config::ContextConfig;
pub use lifecycle::{ContextCloseReport, KernelContext};
pub use registry::{ResourceRegistration, ResourceRegistry};
pub use resource::{CancelReason, ContextResource, Deadline, QuiesceReport, QuiesceState};
pub use state::{ContextPhase, ContextStateGate, ContextStateSnapshot};
