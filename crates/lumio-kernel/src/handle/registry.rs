//! Context-scoped typed handle registry.

use super::{ContextKey, Handle, HandleArena};
use crate::error::{ErrorCategory, ErrorDetail, KernelError};

pub struct TypedHandleRegistry<T> {
    arena: HandleArena<T>,
    context: ContextKey,
}

impl<T> TypedHandleRegistry<T> {
    pub fn new(context: ContextKey, capacity: u32) -> Self {
        Self {
            arena: HandleArena::with_capacity(context, capacity),
            context,
        }
    }

    pub fn insert(&mut self, v: T) -> Result<Handle<T>, KernelError> {
        self.arena.insert(v)
    }

    pub fn get(&self, h: Handle<T>) -> Result<&T, KernelError> {
        let key = h.key();
        if key.context != self.context {
            return Err(KernelError::new(
                ErrorCategory::WrongContext,
                ErrorDetail::None,
            ));
        }
        self.arena.get(Handle::from_key(key))
    }

    pub fn remove(&mut self, h: Handle<T>) -> Result<T, KernelError> {
        let key = h.key();
        if key.context != self.context {
            return Err(KernelError::new(
                ErrorCategory::WrongContext,
                ErrorDetail::None,
            ));
        }
        self.arena.remove(Handle::from_key(key))
    }

    pub fn context(&self) -> ContextKey {
        self.context
    }

    #[allow(clippy::len_without_is_empty)]
    pub fn len(&self) -> u32 {
        self.arena.len()
    }
}
