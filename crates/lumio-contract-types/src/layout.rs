//! ABI layout assertions against the published Root ABI bundle Golden.
//!
//! The golden rows live in `generated_data.rs` (derived from the byte-pinned
//! bundle mirror by `cargo xtask gen-contracts`); this module only compares.
//! V1 publishes a Golden for exactly one layout profile —
//! `linux-x86_64-glibc` — and a consumer must not assert a layout on any
//! other target (ADR-040 §7, D-016 pending). The Rust-type comparisons are
//! therefore compile- and run-time gated to that profile; on every other
//! target `verify_layout` succeeds without asserting, and the data-vs-mirror
//! equality tests still run.

use crate::generated::StructSize;
use crate::generated_data::{ABI_MAX_ALIGNMENT, ABI_POINTER_BYTES, ABI_STRUCT_GOLDEN};

/// Published size/alignment of one named shared POD C type.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AbiTypeGolden {
    pub name: &'static str,
    pub size: u32,
    pub align: u32,
}

/// Published layout of one generated struct: declared/minimum size plus the
/// byte offset of every header field, table pointer and slot pointer.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AbiStructGolden {
    pub name: &'static str,
    pub declared_size: u32,
    pub minimum_size: u32,
    pub members: &'static [(&'static str, u32)],
}

/// Layout row that does not match the generated Header / bundle Golden.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LayoutMismatch {
    pub struct_name: &'static str,
    pub expected: StructSize,
    pub found: StructSize,
}

/// Published pointer width in bytes (`layoutProfile.pointerBytes`).
pub fn pointer_bytes() -> u32 {
    ABI_POINTER_BYTES
}

/// Published maximum alignment (`layoutProfile.maxAlignment`).
pub fn max_alignment() -> u32 {
    ABI_MAX_ALIGNMENT
}

/// Named C type golden rows from the bundle's `typeMapping`.
pub fn type_entries() -> &'static [AbiTypeGolden] {
    crate::generated_data::ABI_TYPE_GOLDEN
}

/// Struct golden rows (root table plus every published API table).
pub fn struct_entries() -> &'static [AbiStructGolden] {
    ABI_STRUCT_GOLDEN
}

/// Generated Header layout rows as (name, declared size) — POD types first,
/// then structs, in published order.
pub fn entries() -> Vec<(&'static str, StructSize)> {
    let mut rows: Vec<(&'static str, StructSize)> = crate::generated_data::ABI_TYPE_GOLDEN
        .iter()
        .map(|t| (t.name, StructSize::new(t.size)))
        .collect();
    rows.extend(
        ABI_STRUCT_GOLDEN
            .iter()
            .map(|s| (s.name, StructSize::new(s.declared_size))),
    );
    rows
}

#[cfg(all(target_arch = "x86_64", target_os = "linux", target_env = "gnu"))]
mod certified {
    //! Golden comparisons for the one published layout profile.

    use super::LayoutMismatch;
    use crate::generated::StructSize;
    use crate::generated::{LumioBuffer, LumioHandle, LumioStatus};

    // A mismatch is a build failure, never a runtime discovery (ADR-040 §4).
    const _: () = {
        assert!(size_of::<LumioStatus>() == 4);
        assert!(align_of::<LumioStatus>() == 4);
        assert!(size_of::<LumioHandle>() == 16);
        assert!(align_of::<LumioHandle>() == 8);
        assert!(size_of::<LumioBuffer>() == 24);
        assert!(align_of::<LumioBuffer>() == 8);
        assert!(size_of::<*mut core::ffi::c_void>() == 8);
    };

    pub(super) fn verify() -> Result<(), LayoutMismatch> {
        let bound: &[(&'static str, usize)] = &[
            ("lumio_status_t", size_of::<LumioStatus>()),
            ("lumio_handle_t", size_of::<LumioHandle>()),
            ("lumio_buffer_t", size_of::<LumioBuffer>()),
        ];
        for &(name, actual) in bound {
            let golden = super::type_entries()
                .iter()
                .find(|t| t.name == name)
                .unwrap_or_else(|| panic!("bundle golden missing {name}"));
            if actual as u32 != golden.size {
                return Err(LayoutMismatch {
                    struct_name: golden.name,
                    expected: StructSize::new(golden.size),
                    found: StructSize::new(actual as u32),
                });
            }
        }
        Ok(())
    }
}

/// Verify the bound Rust POD types against the published Golden.
///
/// On the certified `linux-x86_64-glibc` profile this compares every bound
/// type; on any other target it succeeds without asserting, because no other
/// Golden is published (ADR-040 §7).
pub fn verify_layout() -> Result<(), LayoutMismatch> {
    #[cfg(all(target_arch = "x86_64", target_os = "linux", target_env = "gnu"))]
    {
        certified::verify()
    }
    #[cfg(not(all(target_arch = "x86_64", target_os = "linux", target_env = "gnu")))]
    {
        Ok(())
    }
}
