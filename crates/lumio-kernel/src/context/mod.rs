//! Kernel context configuration, admission/closing gate, and resource port.

mod config;
mod resource;
mod state;

pub use config::ContextConfig;
pub use resource::{CancelReason, ContextResource, QuiesceReport, QuiesceState};
pub use state::{ContextPhase, ContextStateGate, ContextStateSnapshot};
