//! Bounded handle slots and a free list.
//!
//! Generation overflow retire is T-handle-03.

use super::{ContextKey, Generation, Handle, HandleKey, SlotIndex};
use crate::error::{ErrorCategory, ErrorDetail, KernelError};

struct Slot<T> {
    generation: Generation,
    value: Option<T>,
}

pub struct HandleArena<T> {
    context: ContextKey,
    slots: Vec<Slot<T>>,
    free: Vec<u32>,
}

impl<T> HandleArena<T> {
    pub fn with_capacity(context: ContextKey, capacity: u32) -> Self {
        let cap = capacity as usize;
        let mut slots = Vec::with_capacity(cap);
        slots.extend((0..capacity).map(|_| Slot {
            generation: Generation::new(1),
            value: None,
        }));
        let mut free = Vec::with_capacity(cap);
        free.extend((0..capacity).rev());
        Self {
            context,
            slots,
            free,
        }
    }

    pub fn insert(&mut self, value: T) -> Result<Handle<T>, KernelError> {
        let Some(index) = self.free.pop() else {
            let limit = u64::from(self.capacity());
            return Err(KernelError::new(
                ErrorCategory::CapacityExceeded,
                ErrorDetail::LimitExceeded {
                    limit,
                    requested: limit + 1,
                },
            ));
        };

        let slot = &mut self.slots[index as usize];
        slot.value = Some(value);
        Ok(Handle::from_key(HandleKey {
            context: self.context,
            slot: SlotIndex::new(index),
            generation: slot.generation,
        }))
    }

    #[allow(clippy::len_without_is_empty)]
    pub fn len(&self) -> u32 {
        (self.slots.len() - self.free.len()) as u32
    }

    pub fn capacity(&self) -> u32 {
        self.slots.len() as u32
    }
}
