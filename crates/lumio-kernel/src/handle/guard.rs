//! Shared borrow of a live handle slot.
//!
//! The guard holds the registry read lock so `remove` cannot take the write
//! lock until every guard is dropped (T-handle-05 / R-00129).

use std::ops::Deref;
use std::sync::RwLockReadGuard;

use super::{Handle, HandleArena};

pub struct HandleGuard<'a, T> {
    arena: RwLockReadGuard<'a, HandleArena<T>>,
    handle: Handle<T>,
}

impl<'a, T> HandleGuard<'a, T> {
    pub(crate) fn new(arena: RwLockReadGuard<'a, HandleArena<T>>, handle: Handle<T>) -> Self {
        Self { arena, handle }
    }
}

impl<T> Deref for HandleGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        self.arena
            .get(self.handle)
            .expect("borrowed handle remains occupied while the guard lives")
    }
}
