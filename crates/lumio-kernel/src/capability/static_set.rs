//! Sorted unique static capability set.
//!
//! Keys come from the generated architecture registry (ADR-040 §7.1, D-015):
//! `ids/index.json` owns the key space and the architecture generator is its
//! sole emitter, so this module maps published keys and never keeps a
//! repository-private key table. The numerics it carries are 1-based
//! enumeration ordinals, **not** bit positions — `capability_bits`
//! mask-vs-count semantics and every bit assignment stay unfrozen.

use lumio_contract_types::{ArchitectureCapabilityKey, registry};

use crate::error::{ErrorCategory, ErrorDetail, KernelError};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct CapabilityKey(u32);

impl CapabilityKey {
    /// The only way to obtain a key: project a registered architecture
    /// capability. There is no raw constructor, so no caller — inside this
    /// crate or outside it — can mint a key the registry does not publish.
    pub const fn from_registered(key: ArchitectureCapabilityKey) -> Self {
        Self(key.numeric())
    }

    /// Look up a registered capability by its published id string, e.g.
    /// `"VoxelSpatial"`. Unregistered ids have no key.
    pub fn from_registry_id(id: &str) -> Option<Self> {
        registry::capability_key(id).map(Self::from_registered)
    }

    /// The registered enumeration ordinal this key projects.
    pub const fn as_registry_numeric(self) -> u32 {
        self.0
    }
}

#[derive(Debug)]
pub struct StaticCapabilities {
    enabled: Box<[CapabilityKey]>,
}

impl StaticCapabilities {
    pub fn from_keys(keys: impl IntoIterator<Item = CapabilityKey>) -> Result<Self, KernelError> {
        let mut enabled: Vec<CapabilityKey> = keys.into_iter().collect();
        enabled.sort_unstable();
        if enabled.windows(2).any(|pair| pair[0] == pair[1]) {
            return Err(KernelError::new(
                ErrorCategory::InvalidArgument,
                ErrorDetail::None,
            ));
        }
        Ok(Self {
            enabled: enabled.into_boxed_slice(),
        })
    }

    pub fn contains(&self, k: CapabilityKey) -> bool {
        self.enabled.binary_search(&k).is_ok()
    }

    pub fn iter(&self) -> impl Iterator<Item = CapabilityKey> + '_ {
        self.enabled.iter().copied()
    }
}
