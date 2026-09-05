//! Sorted unique static capability set.
//!
//! A key is an opaque `u32` whose meaning is defined by the embedder (the SDK
//! in the architecture repository). This module owns no key names and no key
//! table: it only stores a sorted unique set and answers membership. Ordering
//! is by raw value and carries no semantics of its own (ADR 0009, superseding
//! ADR 0006's registry projection).

use crate::error::{ErrorCategory, ErrorDetail, KernelError};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct CapabilityKey(u32);

impl CapabilityKey {
    pub const fn from_raw(value: u32) -> Self {
        Self(value)
    }

    pub const fn raw(self) -> u32 {
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
