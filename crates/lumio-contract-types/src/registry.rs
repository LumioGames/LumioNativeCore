//! Read-only registry queries over the published ID Registry.
//!
//! `ids/index.json` is the sole numeric authority (ADR-040 §7); the table
//! consumed here is generated from its byte-pinned mirror by
//! `cargo xtask gen-contracts`, so no numeric in this crate is hand-written.
//! Only the `ErrorCode` namespace is bound:
//!
//! - `Capability` numerics are CoreEngine package-capability enumeration
//!   ordinals, not bit positions; deriving any kernel capability key from
//!   them is forbidden until D-015 lands, so they stay unbound here.
//! - No `OperationId` namespace exists or is reserved (B-ABI-004 adjudicated
//!   not applicable): the public identity of a callable operation is the
//!   published (`apiTable[].name`, `slots[].slotIndex`) pair.
//! - `MessageType` / `FaultClass` are GameRuntime-owned and outside this
//!   repository's consumption surface.

use crate::generated::{ArchitectureErrorCode, ArchitectureOperationId, CapabilityBits};
use crate::generated_data::ERROR_CODES;

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

/// Capability bit entries. Empty until D-015 freezes the `capability_bits`
/// semantics and bit assignment (see module docs).
pub fn capability_bits() -> impl Iterator<Item = CapabilityBits> {
    core::iter::empty()
}
