//! Native-owned buffers: handle registry plus charged-byte budget.

use crate::error::KernelError;
use crate::handle::{ContextKey, Handle, TypedHandleRegistry};

use super::budget::MemoryBudget;
use super::buffer::NativeOwnedBufferHandle;
use super::provenance::AllocatorId;

pub struct NativeBufferOwner {
    registry: TypedHandleRegistry<Vec<u8>>,
    budget: MemoryBudget,
    #[allow(dead_code)]
    allocator: AllocatorId,
}

impl NativeBufferOwner {
    pub fn new(
        context: ContextKey,
        max_handles: u32,
        budget: MemoryBudget,
        allocator: AllocatorId,
    ) -> Self {
        Self {
            registry: TypedHandleRegistry::new(context, max_handles),
            budget,
            allocator,
        }
    }

    pub fn allocate(&mut self, bytes: usize) -> Result<NativeOwnedBufferHandle, KernelError> {
        self.budget.try_reserve(bytes as u64)?;
        match self.registry.insert(vec![0u8; bytes]) {
            Ok(handle) => Ok(NativeOwnedBufferHandle::wrap(Handle::from_key(
                handle.key(),
            ))),
            Err(err) => {
                self.budget.release(bytes as u64);
                Err(err)
            }
        }
    }

    pub fn release(&mut self, h: NativeOwnedBufferHandle) -> Result<(), KernelError> {
        let payload = self.registry.remove(Handle::from_key(h.unwrap().key()))?;
        self.budget.release(payload.len() as u64);
        Ok(())
    }

    pub fn charged(&self) -> u64 {
        self.budget.charged()
    }

    #[allow(clippy::len_without_is_empty)]
    pub fn len(&self) -> u32 {
        self.registry.len()
    }
}
