//! Read-only registry queries over the published ID Registry.
//!
//! `ids/index.json` is the sole numeric authority (ADR-040 §7); the tables
//! consumed here are generated from its byte-pinned mirror by
//! `cargo xtask gen-contracts`, so no numeric in this crate is hand-written.
//! The `ErrorCode` and `Capability` namespaces are bound:
//!
//! - `Capability` keys are bound since D-015 (ADR-040 §7.1): the registry is
//!   the key-space authority and the architecture generator its sole
//!   emitter, so a repository-private key table is a violation. The numerics
//!   are 1-based enumeration ordinals, **not** bit positions — reading a key
//!   is allowed, deriving a bit is not, and `capability_bits` stays unbound.
//! - No `OperationId` namespace exists or is reserved (B-ABI-004 adjudicated
//!   not applicable): the public identity of a callable operation is the
//!   published (`apiTable[].name`, `slots[].slotIndex`) pair.
//! - `MessageType` / `FaultClass` are GameRuntime-owned and outside this
//!   repository's consumption surface.

use crate::generated::{
    ArchitectureCapabilityKey, ArchitectureErrorCode, ArchitectureOperationId, CapabilityBits,
};
use crate::generated_data::{CAPABILITY_KEYS, ERROR_CODES};

/// Architecture error codes from the generated registry, in published order
/// (includes the ADR-046 kernel status band).
pub fn error_codes() -> impl Iterator<Item = ArchitectureErrorCode> {
    ERROR_CODES.iter().copied()
}

/// Look up one registered error code by its published id string.
pub fn error_code(id: &str) -> Option<ArchitectureErrorCode> {
    ERROR_CODES.iter().copied().find(|code| code.id() == id)
}

/// Architecture operation ids. Permanently empty: the namespace does not
/// exist and none is reserved (see module docs); kept for `lumio-job`'s
/// non-overlap negative gate.
pub fn operation_ids() -> impl Iterator<Item = ArchitectureOperationId> {
    core::iter::empty()
}

/// Registered capability keys from the generated registry, in published
/// order (ADR-040 §7.1); includes `Reserved` values, whose status callers
/// must honour rather than infer.
pub fn capability_keys() -> impl Iterator<Item = ArchitectureCapabilityKey> {
    CAPABILITY_KEYS.iter().copied()
}

/// Look up one registered capability key by its published id string.
pub fn capability_key(id: &str) -> Option<ArchitectureCapabilityKey> {
    CAPABILITY_KEYS.iter().copied().find(|key| key.id() == id)
}

/// Capability bit entries. Still empty after D-015: the adjudication froze
/// the key space only, leaving `capability_bits` mask-vs-count semantics and
/// every bit position unbound (see module docs).
pub fn capability_bits() -> impl Iterator<Item = CapabilityBits> {
    core::iter::empty()
}
