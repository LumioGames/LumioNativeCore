//! Call-scoped buffers, native-owned buffer handles, allocator provenance, and budget.

mod budget;
mod buffer;
mod call_scratch;
mod native_buffers;
mod provenance;

pub use budget::MemoryBudget;
pub use buffer::{
    BorrowedCallBuffer, CallerOutputBuffer, NativeBufferTag, NativeOwnedBufferHandle,
};
pub use call_scratch::CallScratch;
pub use native_buffers::{NativeBufferOwner, NativeBufferReleaseReport};
pub use provenance::{AllocationClass, AllocationProvenance, AllocatorId};
