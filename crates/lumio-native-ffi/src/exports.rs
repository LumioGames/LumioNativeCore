//! Provider `lumio_core_api` table, bound to the published Root ABI Header.
//!
//! The generated Header (`docs/architecture/abi/lumio_core.h`, ADR-040) is
//! the source of the table layout and slot signatures transcribed here; the
//! layout tests compare against the bundle Golden through
//! `lumio_contract_types::layout`. Per ADR-006 the entry symbol belongs to
//! CoreEngine `root-abi` — this crate assembles the provider table as a Rust
//! value and exports **no** C symbol (enforced by `cargo xtask dump-symbols`
//! and the source-text guard in this module's tests; do not spell that
//! symbol here). Still blocked, recorded on R-00179:
//!
//! - how CoreEngine obtains this table (provider composition contract /
//!   symbol list is unpublished), so nothing is `#[no_mangle]`;
//! - `lumio_core_init`: the `lumio_core_config_v1` body is opaque by
//!   contract and the published slot returns the context handle through a
//!   by-value `out_context` parameter, which cannot carry a result — raised
//!   upstream; the slot stays unpopulated rather than invented.

use core::ffi::c_void;

use lumio_contract_types::{LumioBuffer, LumioCoreConfigV1, LumioHandle, LumioStatus, layout};
use lumio_kernel::error::{ErrorCategory, ErrorDetail, KernelError, to_architecture_error_code};
use lumio_kernel::handle::{ContextKey, HandleKey};

use crate::boundary::ffi_boundary;
use crate::handles::decode_handle_for_context;

/// `lumio_core_api` with the published layout (48 bytes on the certified
/// profile: header fields at 0/4/8, slots at 16/24/32, one reserved pointer
/// word). Field names and slot signatures follow the generated Header
/// verbatim; `Option<extern "C" fn>` has the guaranteed nullable-pointer
/// representation, so an unpopulated slot is a null function pointer.
#[repr(C)]
pub struct LumioCoreApi {
    pub version: u32,
    pub struct_size: u32,
    pub reserved0: u64,
    pub lumio_core_init:
        Option<extern "C" fn(*const LumioCoreConfigV1, LumioHandle) -> LumioStatus>,
    pub lumio_core_shutdown: Option<extern "C" fn(LumioHandle) -> LumioStatus>,
    pub lumio_core_last_error_detail:
        Option<extern "C" fn(LumioHandle, LumioBuffer) -> LumioStatus>,
    pub reserved: [*mut c_void; 1],
}

fn status_of(result: Result<(), KernelError>) -> LumioStatus {
    match result {
        Ok(()) => LumioStatus::SUCCESS,
        Err(error) => LumioStatus::from_error_code(to_architecture_error_code(&error)),
    }
}

/// With `lumio_core_init` unpopulated no context can exist, so every handle
/// fails as the registered `InvalidHandle` code — through the panic boundary
/// and the single mapping, never as an invented numeric.
extern "C" fn core_shutdown(_context: LumioHandle) -> LumioStatus {
    status_of(ffi_boundary(|| {
        Err(KernelError::new(
            ErrorCategory::InvalidHandle,
            ErrorDetail::None,
        ))
    }))
}

/// Same degenerate state as [`core_shutdown`]: the context cannot exist, so
/// the buffer is left untouched and the registered `InvalidHandle` returns.
extern "C" fn core_last_error_detail(
    _context: LumioHandle,
    _out_detail: LumioBuffer,
) -> LumioStatus {
    status_of(ffi_boundary(|| {
        Err(KernelError::new(
            ErrorCategory::InvalidHandle,
            ErrorDetail::None,
        ))
    }))
}

/// Assemble the provider table with published header fields (version and
/// struct_size come from the bundle Golden, not literals).
pub fn provider_core_api_table() -> LumioCoreApi {
    let golden = layout::struct_entries()
        .iter()
        .find(|s| s.name == "lumio_core_api")
        .expect("bundle golden carries lumio_core_api");
    LumioCoreApi {
        version: layout::table_version("lumio_core_api").expect("published table version"),
        struct_size: golden.declared_size,
        reserved0: 0,
        lumio_core_init: None,
        lumio_core_shutdown: Some(core_shutdown),
        lumio_core_last_error_detail: Some(core_last_error_detail),
        reserved: [core::ptr::null_mut(); 1],
    }
}

/// Decode `key` for `expected` inside the FFI panic/error boundary.
///
/// Wrong-context handles return `ErrorCategory::WrongContext`, which maps to
/// the registered `WrongContext` ErrorCode (ADR-046).
pub fn smoke_decode_handle(key: HandleKey, expected: ContextKey) -> Result<(), KernelError> {
    ffi_boundary(move || decode_handle_for_context(key, expected).map(|_| ()))
}

#[cfg(test)]
mod tests {
    use super::{provider_core_api_table, smoke_decode_handle};
    use crate::boundary::ffi_boundary;
    use crate::handles::decode_handle_for_context;
    use lumio_contract_types::{LumioHandle, layout, registry};
    use lumio_kernel::error::{ErrorCategory, to_architecture_error_code};
    use lumio_kernel::handle::{ContextKey, Generation, HandleKey, SlotIndex};

    fn wrong_context_key() -> HandleKey {
        HandleKey {
            context: ContextKey::new(1),
            slot: SlotIndex::new(0),
            generation: Generation::new(1),
        }
    }

    #[test]
    fn c_smoke_invalid_handle_returns_stable_code() {
        let key = wrong_context_key();
        let expected = ContextKey::new(2);

        let err = match smoke_decode_handle(key, expected) {
            Err(e) => e,
            Ok(()) => panic!("wrong-context smoke must fail"),
        };
        assert_eq!(err.category(), ErrorCategory::WrongContext);
        assert_ne!(err.category(), ErrorCategory::InvalidHandle);
        assert_ne!(err.category(), ErrorCategory::AlreadyReleased);
        assert_eq!(to_architecture_error_code(&err).id(), "WrongContext");

        let via_boundary =
            match ffi_boundary(|| decode_handle_for_context(key, expected).map(|_| ())) {
                Err(e) => e,
                Ok(()) => panic!("wrong-context decode through ffi_boundary must fail"),
            };
        assert_eq!(via_boundary.category(), ErrorCategory::WrongContext);
        assert_eq!(via_boundary.category(), err.category());
        assert_eq!(
            to_architecture_error_code(&via_boundary),
            to_architecture_error_code(&err)
        );

        assert!(smoke_decode_handle(key, ContextKey::new(1)).is_ok());

        // 经槽位函数指针的 C ABI 调用：init 未发布即无 context，任何 handle
        // 都必须返回注册的 InvalidHandle numeric（经注册表取值，不写字面量）。
        let table = provider_core_api_table();
        assert!(table.lumio_core_init.is_none(), "init stays blocked");
        let shutdown = table.lumio_core_shutdown.expect("shutdown slot populated");
        let status = shutdown(LumioHandle {
            index: 7,
            generation: 3,
            context: 999,
        });
        let invalid = registry::error_code("InvalidHandle").expect("registered InvalidHandle");
        assert_eq!(status.raw(), invalid.numeric());
        assert!(!status.is_success());

        let last_error = table
            .lumio_core_last_error_detail
            .expect("last_error_detail slot populated");
        let status = last_error(
            LumioHandle {
                index: 0,
                generation: 1,
                context: 1,
            },
            lumio_contract_types::LumioBuffer {
                ptr: core::ptr::null_mut(),
                len: 0,
                capacity: 0,
            },
        );
        assert_eq!(status.raw(), invalid.numeric());

        let exports_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/exports.rs");
        let exports_src = std::fs::read_to_string(&exports_path)
            .unwrap_or_else(|e| panic!("read {}: {e}", exports_path.display()));
        assert!(
            !exports_src.contains(concat!("lumio_core_get_api_", "v1")),
            "exports.rs must not mention the Root API symbol"
        );
    }

    /// Table header fields carry the published values (data vs golden; no
    /// platform layout claim).
    #[test]
    fn provider_table_header_fields_match_golden() {
        let table = provider_core_api_table();
        let golden = layout::struct_entries()
            .iter()
            .find(|s| s.name == "lumio_core_api")
            .expect("golden lumio_core_api");
        assert_eq!(table.struct_size, golden.declared_size);
        assert_eq!(Some(table.version), layout::table_version("lumio_core_api"));
        assert_eq!(table.reserved0, 0);
    }

    /// Rust-side table layout equals the bundle Golden — asserted only on
    /// the one certified profile (ADR-040 §7, D-016 pending).
    #[cfg(all(target_arch = "x86_64", target_os = "linux", target_env = "gnu"))]
    #[test]
    fn provider_table_layout_matches_golden_on_certified_profile() {
        use super::LumioCoreApi;

        let golden = layout::struct_entries()
            .iter()
            .find(|s| s.name == "lumio_core_api")
            .expect("golden lumio_core_api");
        assert_eq!(size_of::<LumioCoreApi>() as u32, golden.declared_size);
        let offsets: &[(&str, usize)] = &[
            ("version", core::mem::offset_of!(LumioCoreApi, version)),
            (
                "struct_size",
                core::mem::offset_of!(LumioCoreApi, struct_size),
            ),
            ("reserved0", core::mem::offset_of!(LumioCoreApi, reserved0)),
            (
                "lumio_core_init",
                core::mem::offset_of!(LumioCoreApi, lumio_core_init),
            ),
            (
                "lumio_core_shutdown",
                core::mem::offset_of!(LumioCoreApi, lumio_core_shutdown),
            ),
            (
                "lumio_core_last_error_detail",
                core::mem::offset_of!(LumioCoreApi, lumio_core_last_error_detail),
            ),
        ];
        for &(name, actual) in offsets {
            let (_, expected) = golden
                .members
                .iter()
                .find(|(member, _)| *member == name)
                .unwrap_or_else(|| panic!("golden member {name}"));
            assert_eq!(actual as u32, *expected, "offset mismatch for {name}");
        }
    }
}
