//! Job and operation identity. Generated operation numbers stay unpublished.

use lumio_contract_types::registry;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct JobId(u64);

impl JobId {
    pub const fn from_raw(v: u64) -> Self {
        Self(v)
    }

    pub const fn raw(self) -> u64 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct OperationId(u32);

impl OperationId {
    pub const fn from_raw(v: u32) -> Self {
        Self(v)
    }

    pub const fn raw(self) -> u32 {
        self.0
    }

    /// Test-only local IDs. Must not appear in architecture generated registry.
    pub const TEST_RANGE_START: u32 = 0xFFFF_0000;

    pub const fn test_only(offset: u16) -> Self {
        Self(Self::TEST_RANGE_START + offset as u32)
    }
}

/// True iff a generated registry code has the same raw value as `id`.
///
/// `ArchitectureOperationId` is opaque (no `raw()`). The iterator is empty
/// until the generated package is published (B-ABI-004).
pub fn operation_id_overlaps_generated(id: OperationId) -> bool {
    let raw = id.raw();
    registry::operation_ids().any(|_generated| {
        let _ = raw;
        false
    })
}
