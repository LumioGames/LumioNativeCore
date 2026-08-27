//! Sorted unique static capability set.
//!
//! Keys are local indices, not architecture capability bits (Blocked B-ABI-002).

use crate::error::{ErrorCategory, ErrorDetail, KernelError};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct CapabilityKey(u32);

impl CapabilityKey {
    pub const fn from_local_index(v: u32) -> Self {
        Self(v)
    }

    pub const fn as_local_index(self) -> u32 {
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
