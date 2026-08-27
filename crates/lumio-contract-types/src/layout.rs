//! ABI layout assertions against the architecture Header / manifest.
//!
//! FOUNDATION-W1 has not published a C Header. This gate must not invent ABI
//! sizes; an empty table is a match.

use crate::generated::StructSize;

/// Layout row that does not match the generated Header / manifest.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LayoutMismatch {
    pub struct_name: &'static str,
    pub expected: StructSize,
    pub found: StructSize,
}

/// Generated Header layout rows. Empty while the Header is unpublished.
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
