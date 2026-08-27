//! Call-scoped buffers, native-owned buffer handles, and allocator provenance.

mod buffer;
mod provenance;

pub use buffer::{
    BorrowedCallBuffer, CallerOutputBuffer, NativeBufferTag, NativeOwnedBufferHandle,
};
pub use provenance::{AllocationClass, AllocationProvenance, AllocatorId};
