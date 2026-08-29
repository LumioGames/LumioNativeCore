//! T-ffi-04 / R-00179: provider-table C smoke — registered status codes
//! through the published slot signatures, plus a standalone compile of the
//! mirrored generated Header (its static asserts are the layout Golden).
//!
//! `lumio-native-ffi` is `cdylib`+`staticlib` only, so Cargo does not pass
//! `--extern lumio_native_ffi` (no rlib) to integration tests on this host.
//! The smoke helper composes `ffi_boundary` and `decode_handle_for_context`,
//! so those sources are compiled via `#[path]` together with `exports.rs`.
//! The crate-root `pub use` is covered by the `#[cfg(test)]` module in
//! `exports.rs`.

#[path = "../src/boundary.rs"]
mod boundary;
#[path = "../src/exports.rs"]
mod exports;
#[path = "../src/handles.rs"]
mod handles;

use boundary::ffi_boundary;
use exports::smoke_decode_handle;
use handles::decode_handle_for_context;
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

    let via_boundary = match ffi_boundary(|| decode_handle_for_context(key, expected).map(|_| ())) {
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

    let exports_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/exports.rs");
    let exports_src = std::fs::read_to_string(&exports_path)
        .unwrap_or_else(|e| panic!("read {}: {e}", exports_path.display()));
    assert!(
        !exports_src.contains("lumio_core_get_api_v1"),
        "exports.rs must not mention the Root API symbol"
    );

    // 经 provider 表函数指针的 C ABI 调用：无 context 存在，任何 handle 都返回
    // 注册的 InvalidHandle numeric（经注册表取值，不写字面量）。
    let table = exports::provider_core_api_table();
    assert!(table.lumio_core_init.is_none(), "init stays blocked");
    let shutdown = table.lumio_core_shutdown.expect("shutdown slot populated");
    let status = shutdown(lumio_contract_types::LumioHandle {
        index: 7,
        generation: 3,
        context: 999,
    });
    let invalid = lumio_contract_types::registry::error_code("InvalidHandle")
        .expect("registered InvalidHandle");
    assert_eq!(status.raw(), invalid.numeric());
    assert!(!status.is_success());
}

/// C smoke: the mirrored generated Header must compile standalone — its
/// `LUMIO_STATIC_ASSERT` rows are the layout Golden, so a successful compile
/// is the C-side layout check. Skips (with a log line) when the host has no
/// C compiler, mirroring the `nm` host-gap precedent in `symbol_guard`.
#[test]
fn c_header_compile_smoke_asserts_published_layout() {
    use std::process::Command;

    let header_dir =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/architecture/abi");
    assert!(
        header_dir.join("lumio_core.h").is_file(),
        "mirrored lumio_core.h must exist"
    );

    let compiler = ["cc", "gcc", "clang"].into_iter().find(|c| {
        Command::new(c)
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    });
    let Some(compiler) = compiler else {
        eprintln!("skip: no C compiler (cc/gcc/clang) on this host");
        return;
    };

    let tu = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("lumio_core_smoke.c");
    std::fs::write(&tu, "#include \"lumio_core.h\"\nint lumio_smoke_unused;\n")
        .unwrap_or_else(|e| panic!("write {}: {e}", tu.display()));

    let out = Command::new(compiler)
        .arg("-fsyntax-only")
        .arg("-I")
        .arg(&header_dir)
        .arg(&tu)
        .output()
        .unwrap_or_else(|e| panic!("run {compiler}: {e}"));
    assert!(
        out.status.success(),
        "{compiler} rejected the published Header (layout Golden failed?):\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
}
