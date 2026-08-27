//! Static capability set, configured resource limits, and runtime status.
//! Public architecture bits remain blocked (B-ABI-002).

mod limits;
mod runtime;
mod static_set;

pub use limits::ConfiguredLimits;
pub use runtime::{RuntimeCounters, RuntimeStatus};
pub use static_set::{CapabilityKey, StaticCapabilities};
