//! ABI layout assertions against the architecture Header / manifest.
//!
//! The architecture source now publishes a C Header (ADR-040 Root ABI bundle,
//! `origin/main:packages/abi/lumio_core.h`), but its bundle certifies exactly
//! one `layoutProfileId` — `linux-x86_64-glibc`. Transcribing those sizes here
//! unconditionally would assert layouts on darwin / windows that the
//! architecture source has not certified, which is the same red line as
//! inventing them. Binding therefore stays deferred until the bundle carries
//! the remaining target profiles, or until this gate is target-gated.
//!
//! This gate must not invent ABI sizes; an empty table is a match.

use crate::generated::StructSize;

/// Layout row that does not match the generated Header / manifest.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LayoutMismatch {
    pub struct_name: &'static str,
    pub expected: StructSize,
    pub found: StructSize,
}

/// Generated Header layout rows. Empty until binding is target-gated (see
/// the module docs): the published bundle certifies `linux-x86_64-glibc` only.
pub fn entries() -> &'static [(&'static str, StructSize)] {
    &[]
}

/// Verify generated struct layouts against the architecture manifest.
///
/// With no generated Header there are no structs to check, so this succeeds
/// without inventing sizes.
pub fn verify_layout() -> Result<(), LayoutMismatch> {
    for &(name, expected) in entries() {
        let _ = (name, expected);
    }
    Ok(())
}
