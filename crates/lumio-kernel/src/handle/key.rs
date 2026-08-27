//! Internal handle identity: context, slot, generation.
//!
//! This is not an ABI opaque encoding (Blocked B-ABI-003).

use core::cmp::Ordering;
use core::marker::PhantomData;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct ContextKey(u64);

impl ContextKey {
    pub const fn new(v: u64) -> Self {
        Self(v)
    }

    pub const fn raw(self) -> u64 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct SlotIndex(u32);

impl SlotIndex {
    pub const fn new(v: u32) -> Self {
        Self(v)
    }

    pub const fn raw(self) -> u32 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Generation(u32);

impl Generation {
    pub const fn new(v: u32) -> Self {
        Self(v)
    }

    pub const fn raw(self) -> u32 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct HandleKey {
    pub context: ContextKey,
    pub slot: SlotIndex,
    pub generation: Generation,
}

impl PartialOrd for HandleKey {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for HandleKey {
    fn cmp(&self, other: &Self) -> Ordering {
        self.context
            .cmp(&other.context)
            .then(self.slot.cmp(&other.slot))
            .then(self.generation.cmp(&other.generation))
    }
}

pub struct Handle<T> {
    key: HandleKey,
    _tag: PhantomData<fn() -> T>,
}

impl<T> Clone for Handle<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for Handle<T> {}

impl<T> Handle<T> {
    pub const fn from_key(k: HandleKey) -> Self {
        Self {
            key: k,
            _tag: PhantomData,
        }
    }

    pub const fn key(self) -> HandleKey {
        self.key
    }
}
