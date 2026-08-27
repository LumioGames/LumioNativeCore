//! Static capability set and configured resource limits.
//! Public architecture bits remain blocked (B-ABI-002).

mod limits;
mod static_set;

pub use limits::ConfiguredLimits;
pub use static_set::{CapabilityKey, StaticCapabilities};
