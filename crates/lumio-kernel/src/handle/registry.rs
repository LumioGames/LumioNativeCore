//! Context-scoped typed handle registry.

use std::sync::RwLock;

use super::{ContextKey, Handle, HandleArena, HandleGuard};
use crate::error::{ErrorCategory, ErrorDetail, KernelError};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HandleRetireReport {
    pub dropped: u32,
    pub already_empty: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HandleArenaSnapshot {
    pub live: u32,
    pub capacity: u32,
    pub retired_slots: u32,
}

pub struct TypedHandleRegistry<T> {
    arena: RwLock<HandleArena<T>>,
    context: ContextKey,
}

impl<T> TypedHandleRegistry<T> {
    pub fn new(context: ContextKey, capacity: u32) -> Self {
        Self {
            arena: RwLock::new(HandleArena::with_capacity(context, capacity)),
            context,
        }
    }

    pub fn insert(&mut self, v: T) -> Result<Handle<T>, KernelError> {
        self.arena.write().expect("handle registry lock").insert(v)
    }

    pub fn get(&self, h: Handle<T>) -> Result<HandleGuard<'_, T>, KernelError> {
        self.borrow(h)
    }

    pub fn borrow(&self, h: Handle<T>) -> Result<HandleGuard<'_, T>, KernelError> {
        let key = h.key();
        if key.context != self.context {
            return Err(KernelError::new(
                ErrorCategory::WrongContext,
                ErrorDetail::None,
            ));
        }
        let handle = Handle::from_key(key);
        let arena = self.arena.read().expect("handle registry lock");
        arena.get(handle)?;
        Ok(HandleGuard::new(arena, handle))
    }

    pub fn remove(&self, h: Handle<T>) -> Result<T, KernelError> {
        let key = h.key();
        if key.context != self.context {
            return Err(KernelError::new(
                ErrorCategory::WrongContext,
                ErrorDetail::None,
            ));
        }
        self.arena
            .write()
            .expect("handle registry lock")
            .remove(Handle::from_key(key))
    }

    pub fn retire_all(&self) -> HandleRetireReport {
        let (report, payloads) = {
            let mut arena = self.arena.write().expect("handle registry lock");
            let payloads = arena.drain_occupied();
            let dropped = payloads.len() as u32;
            let already_empty = arena.capacity().saturating_sub(dropped);
            (
                HandleRetireReport {
                    dropped,
                    already_empty,
                },
                payloads,
            )
        };
        drop(payloads);
        report
    }

    pub fn snapshot(&self) -> HandleArenaSnapshot {
        let arena = self.arena.read().expect("handle registry lock");
        HandleArenaSnapshot {
            live: arena.len(),
            capacity: arena.capacity(),
            retired_slots: arena.retired_slots(),
        }
    }

    pub fn context(&self) -> ContextKey {
        self.context
    }

    #[allow(clippy::len_without_is_empty)]
    pub fn len(&self) -> u32 {
        self.arena.read().expect("handle registry lock").len()
    }

    pub(crate) fn take_all(&self) -> Vec<T> {
        self.arena.write().expect("handle registry lock").take_all()
    }
}
