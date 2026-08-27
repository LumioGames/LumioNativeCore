//! Call-scoped buffers, native-owned buffer handles, allocator provenance, and budget.

mod budget;
mod buffer;
mod provenance;

pub use budget::MemoryBudget;
pub use buffer::{
    BorrowedCallBuffer, CallerOutputBuffer, NativeBufferTag, NativeOwnedBufferHandle,
};
pub use provenance::{AllocationClass, AllocationProvenance, AllocatorId};
