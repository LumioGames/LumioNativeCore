//! Negative gate for the CoreEngine-owned Root symbol (T-ffi-05 / R-00180).
//!
//! NativeCore must never export the forbidden Root entry; CoreEngine
//! `root-abi` owns it (ADR 0001). Identifier uses in `src` fail the gate.
//! Comments/docs that only name the forbidden symbol do not. Binary
//! `nm` / `dump-symbols` is a separate host gap.

use std::fs;
use std::path::{Path, PathBuf};

/// Cross-crate Root symbol owned by CoreEngine `root-abi` (ADR 0001).
pub fn forbidden_root_symbol_name() -> &'static str {
    "lumio_core_get_api_v1"
}

/// True when any `src/**/*.rs` file uses the Root symbol as a Rust identifier.
///
/// Comments and string literals do not count, so naming the forbidden symbol
/// in this gate (or in docs that forbid it) is not a hit.
pub fn crate_sources_contain_root_symbol() -> bool {
    let ident = forbidden_root_symbol_name();
    for path in rust_sources(&Path::new(env!("CARGO_MANIFEST_DIR")).join("src")) {
        let text =
            fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
        if contains_ident(&strip_comments_and_strings(&text), ident) {
            return true;
        }
    }
    false
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

fn is_ident_continue(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

fn contains_ident(src: &str, ident: &str) -> bool {
    let bytes = src.as_bytes();
    let needle = ident.as_bytes();
    let mut i = 0;
    while i + needle.len() <= bytes.len() {
        if &bytes[i..i + needle.len()] == needle {
            let before_ok = i == 0 || !is_ident_continue(bytes[i - 1]);
            let after = i + needle.len();
            let after_ok = after == bytes.len() || !is_ident_continue(bytes[after]);
            if before_ok && after_ok {
                return true;
            }
        }
        i += 1;
    }
    false
}

fn is_raw_string_at(b: &[u8], i: usize) -> bool {
    let mut j = i;
    if j < b.len() && (b[j] == b'b' || b[j] == b'c') {
        j += 1;
    }
    if j >= b.len() || b[j] != b'r' {
        return false;
    }
    j += 1;
    while j < b.len() && b[j] == b'#' {
        j += 1;
    }
    j < b.len() && b[j] == b'"'
}

fn skip_quoted(b: &[u8], quote_at: usize) -> usize {
    let mut i = quote_at + 1;
    while i < b.len() {
        if b[i] == b'\\' {
            i = i.saturating_add(2);
            continue;
        }
        if b[i] == b'"' {
            return i + 1;
        }
        i += 1;
    }
    b.len()
}

fn skip_raw_string(b: &[u8], start: usize) -> usize {
    let mut i = start;
    if i < b.len() && (b[i] == b'b' || b[i] == b'c') {
        i += 1;
    }
    i += 1;
    let mut hashes = 0;
    while i < b.len() && b[i] == b'#' {
        hashes += 1;
        i += 1;
    }
    if i < b.len() && b[i] == b'"' {
        i += 1;
    }
    while i < b.len() {
        if b[i] == b'"' {
            let mut n = 0;
            while n < hashes && i + 1 + n < b.len() && b[i + 1 + n] == b'#' {
                n += 1;
            }
            if n == hashes {
                return i + 1 + hashes;
            }
        }
        i += 1;
    }
    b.len()
}

fn strip_comments_and_strings(src: &str) -> String {
    let b = src.as_bytes();
    let mut out = String::with_capacity(src.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'/' && i + 1 < b.len() && b[i + 1] == b'/' {
            i += 2;
            while i < b.len() && b[i] != b'\n' {
                i += 1;
            }
            continue;
        }
        if b[i] == b'/' && i + 1 < b.len() && b[i + 1] == b'*' {
            i += 2;
            while i + 1 < b.len() && !(b[i] == b'*' && b[i + 1] == b'/') {
                i += 1;
            }
            i = i.saturating_add(2).min(b.len());
            out.push(' ');
            continue;
        }
        if is_raw_string_at(b, i) {
            i = skip_raw_string(b, i);
            out.push(' ');
            continue;
        }
        if (b[i] == b'b' || b[i] == b'c') && i + 1 < b.len() && b[i + 1] == b'"' {
            i = skip_quoted(b, i + 1);
            out.push(' ');
            continue;
        }
        if b[i] == b'"' {
            i = skip_quoted(b, i);
            out.push(' ');
            continue;
        }
        let ch = src[i..].chars().next().expect("utf-8");
        out.push(ch);
        i += ch.len_utf8();
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{crate_sources_contain_root_symbol, forbidden_root_symbol_name, rust_sources};
    use std::fs;
    use std::path::Path;

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
}
