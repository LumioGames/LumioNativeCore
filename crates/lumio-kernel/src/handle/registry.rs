//! Context-scoped typed handle registry.

use std::sync::RwLock;

use super::{ContextKey, Handle, HandleArena, HandleGuard};
use crate::error::{ErrorCategory, ErrorDetail, KernelError};

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

    pub fn context(&self) -> ContextKey {
        self.context
    }

    #[allow(clippy::len_without_is_empty)]
    pub fn len(&self) -> u32 {
        self.arena.read().expect("handle registry lock").len()
    }
}
