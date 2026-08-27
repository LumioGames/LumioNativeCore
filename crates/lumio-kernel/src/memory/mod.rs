//! Call-scoped buffers, native-owned buffer handles, allocator provenance, and budget.

mod budget;
mod buffer;
mod native_buffers;
mod provenance;

pub use budget::MemoryBudget;
pub use buffer::{
    BorrowedCallBuffer, CallerOutputBuffer, NativeBufferTag, NativeOwnedBufferHandle,
};
pub use native_buffers::NativeBufferOwner;
pub use provenance::{AllocationClass, AllocationProvenance, AllocatorId};
