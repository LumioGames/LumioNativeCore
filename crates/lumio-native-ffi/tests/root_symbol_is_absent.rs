//! T-ffi-05 / R-00180: NativeCore must not export the CoreEngine Root symbol.
//!
//! `lumio-native-ffi` is `cdylib`+`staticlib` only, so Cargo does not pass
//! `--extern lumio_native_ffi` (no rlib) to integration tests on this host.
//! The same `symbol_guard` source is compiled via `#[path]`; the crate-root
//! `pub use` is covered by the `#[cfg(test)]` module in `symbol_guard.rs`.

#[path = "../src/symbol_guard.rs"]
mod symbol_guard;

use std::fs;
use std::path::{Path, PathBuf};
use symbol_guard::{crate_sources_contain_root_symbol, forbidden_root_symbol_name};

#[test]
fn root_symbol_is_absent() {
    let name = forbidden_root_symbol_name();
    assert_eq!(name, "lumio_core_get_api_v1");
    assert!(
        !crate_sources_contain_root_symbol(),
        "{name} must not appear as an identifier in crate sources"
    );

    let src_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let files = rust_sources(&src_dir);
    assert!(
        !files.is_empty(),
        "expected .rs files under {}",
        src_dir.display()
    );
    for path in &files {
        let text =
            fs::read_to_string(path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
        assert_no_root_export(path, &text, name);
    }

    // Host gap: `cargo xtask dump-symbols` needs `nm`. This Windows runner
    // does not provide it — do not fail the named test on nm. Source scan
    // above is the enforceable Root-absence gate.
    let _ = std::process::Command::new("nm").arg("--version").output();
}

fn rust_sources(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    collect_rs(dir, &mut files);
    files.sort();
    files
}

fn collect_rs(dir: &Path, out: &mut Vec<PathBuf>) {
    let entries = fs::read_dir(dir).unwrap_or_else(|e| panic!("read_dir {}: {e}", dir.display()));
    for entry in entries {
        let path = entry.unwrap_or_else(|e| panic!("src entry: {e}")).path();
        if path.is_dir() {
            collect_rs(&path, out);
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("rs") {
            out.push(path);
        }
    }
}

fn assert_no_root_export(path: &Path, text: &str, name: &str) {
    let fn_pat = format!("fn {name}");
    let static_pat = format!("static {name}");
    let export_pat = format!("#[export_name = \"{name}\"]");
    let mut saw_no_mangle = false;
    for (idx, line) in text.lines().enumerate() {
        let loc = format!("{}:{}", path.display(), idx + 1);
        assert!(!line.contains(&fn_pat), "{loc} must not contain `{fn_pat}`");
        assert!(
            !line.contains(&static_pat),
            "{loc} must not contain `{static_pat}`"
        );
        assert!(
            !line.contains(&export_pat),
            "{loc} must not contain `{export_pat}`"
        );
        if saw_no_mangle && (line.contains(&fn_pat) || line.contains(&static_pat)) {
            panic!("{loc}: #[no_mangle] export of {name} is forbidden");
        }
        let trimmed = line.trim();
        if trimmed.contains("#[no_mangle]") && trimmed.contains(name) {
            panic!("{loc}: #[no_mangle] must not appear with {name}");
        }
        saw_no_mangle = trimmed.contains("#[no_mangle]");
    }
}
