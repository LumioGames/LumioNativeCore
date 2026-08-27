//! Typed handles and internal keys. No ABI opaque encoding.

mod arena;
mod key;

pub use arena::HandleArena;
pub use key::{ContextKey, Generation, Handle, HandleKey, SlotIndex};
