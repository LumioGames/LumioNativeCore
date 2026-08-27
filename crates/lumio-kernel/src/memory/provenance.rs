//! Allocator provenance: who allocated, in which context, for which class.
//!
//! Native buffers are released only by the original `AllocatorId`.

use crate::handle::ContextKey;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct AllocatorId(u32);

impl AllocatorId {
    pub const fn new(v: u32) -> Self {
        Self(v)
    }

    pub const fn raw(self) -> u32 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum AllocationClass {
    CallScratch,
    NativeOwnedBuffer,
    HandlePayload,
    JobPayload,
    SpatialIndex,
    CodecWorkspace,
    DiagnosticsQueue,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct AllocationProvenance {
    pub allocator: AllocatorId,
    pub context: ContextKey,
    pub class: AllocationClass,
    pub requested_bytes: u64,
    pub charged_bytes: u64,
}
