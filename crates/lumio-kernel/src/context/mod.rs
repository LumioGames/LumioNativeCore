//! Kernel context configuration and resource port.

mod config;
mod resource;

pub use config::ContextConfig;
pub use resource::{CancelReason, ContextResource, QuiesceReport, QuiesceState};
