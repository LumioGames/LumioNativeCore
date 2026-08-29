//! Negative gate for the CoreEngine-owned Root symbol (T-ffi-05 / R-00180).
//!
//! NativeCore must never export the forbidden Root entry; CoreEngine
//! `root-abi` owns it (ADR 0001). Identifier uses in `src` fail the gate.
//! Comments/docs that only name the forbidden symbol do not. Binary
//! `nm` / `dump-symbols` is a separate host gap.

use std::fs;
use std::path::{Path, PathBuf};

/// Cross-crate Root symbol owned by CoreEngine `root-abi` (ADR 0001).
///
/// The published bundle's `entrySymbol` is the authority; the tests assert
/// this hardcoded copy equals [`mirror_entry_symbol`] so neither can drift.
pub fn forbidden_root_symbol_name() -> &'static str {
    "lumio_core_get_api_v1"
}

/// `abi.entrySymbol` extracted textually from the mirrored bundle
/// (`docs/architecture/abi/root-abi-bundle.json`), keeping this guard free
/// of crate dependencies.
pub fn mirror_entry_symbol() -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/architecture/abi/root-abi-bundle.json");
    let text = fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    // 镜像可能是 minified 或 pretty 格式：定位键名后跳过 `:` 与空白再取值。
    let key = "\"entrySymbol\"";
    let after_key = text
        .find(key)
        .unwrap_or_else(|| panic!("{} has no entrySymbol field", path.display()))
        + key.len();
    let rest = text[after_key..]
        .trim_start()
        .strip_prefix(':')
        .map(str::trim_start);
    let Some(value) = rest.and_then(|r| r.strip_prefix('"')) else {
        panic!("{} entrySymbol is not a string", path.display());
    };
    let end = value
        .find('"')
        .unwrap_or_else(|| panic!("{} entrySymbol unterminated", path.display()));
    value[..end].to_string()
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
        // 字符/字节字面量（含引号内容如 `'"'` / `b'\''`）会让字符串识别失步，
        // 必须整体跳过；`'ident`（无闭合引号）是生命周期，原样保留。
        if let Some(end) = char_literal_end(b, i) {
            i = end;
            out.push(' ');
            continue;
        }
        let ch = src[i..].chars().next().expect("utf-8");
        out.push(ch);
        i += ch.len_utf8();
    }
    out
}

/// `Some(end)` when `b[i..]` starts a char/byte-char literal (optionally
/// `b`-prefixed); `None` for lifetimes and everything else.
fn char_literal_end(b: &[u8], i: usize) -> Option<usize> {
    let mut j = i;
    if b[j] == b'b' && j + 1 < b.len() && b[j + 1] == b'\'' {
        j += 1;
    }
    if b[j] != b'\'' {
        return None;
    }
    let content = j + 1;
    if content >= b.len() {
        return None;
    }
    if b[content] == b'\\' {
        // 转义字面量：从转义序列后找最近的闭合引号。
        let mut k = content + 2;
        while k < b.len() && b[k] != b'\'' {
            k += 1;
        }
        return (k < b.len()).then_some(k + 1);
    }
    // 单字符字面量 `'x'`；`'a`（无闭合）是生命周期。
    let ch_len = core::str::from_utf8(&b[content..])
        .ok()?
        .chars()
        .next()?
        .len_utf8();
    let close = content + ch_len;
    (close < b.len() && b[close] == b'\'').then_some(close + 1)
}

#[cfg(test)]
mod tests {
    use super::{
        contains_ident, crate_sources_contain_root_symbol, forbidden_root_symbol_name,
        mirror_entry_symbol, rust_sources, strip_comments_and_strings,
    };
    use std::fs;
    use std::path::Path;

    /// Regression: a quote inside a char/byte-char literal (`'"'`, `b'"'`)
    /// must not desync the string stripper — a desynced scanner can both
    /// miss real identifiers and report string contents as code.
    #[test]
    fn scanner_survives_quote_char_literals() {
        let ident = forbidden_root_symbol_name();
        let in_string_only = format!("let q = b'\"'; let s = \"{ident}\"; let c = '\"';");
        assert!(
            !contains_ident(&strip_comments_and_strings(&in_string_only), ident),
            "string contents after a quote char literal must stay stripped"
        );
        let in_code = format!("let q = '\"'; let bad = {ident};");
        assert!(
            contains_ident(&strip_comments_and_strings(&in_code), ident),
            "identifiers after a quote char literal must stay visible"
        );
        let lifetime = "fn f<'a>(x: &'a str) -> &'a str { x }";
        assert!(!contains_ident(
            &strip_comments_and_strings(lifetime),
            ident
        ));
    }

    /// The hardcoded Root symbol must equal the published `entrySymbol`, and
    /// it must sit under the published `symbolPrefix` — the gate binds the
    /// mirror, it does not merely repeat a string.
    #[test]
    fn forbidden_symbol_matches_published_entry_symbol() {
        let published = mirror_entry_symbol();
        assert_eq!(published, forbidden_root_symbol_name());
        assert!(
            published.starts_with("lumio_"),
            "published entry symbol must carry the published symbolPrefix"
        );
    }

    /// Dependency half of the negative gate: only this crate may declare a
    /// C artifact crate-type (`cdylib`/`staticlib`), so no other workspace
    /// crate can grow a symbol surface.
    #[test]
    fn only_native_ffi_declares_a_c_artifact_crate_type() {
        let crates_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
        let mut checked = 0usize;
        for entry in fs::read_dir(&crates_dir).expect("read crates/") {
            let dir = entry.expect("crates entry").path();
            let manifest = dir.join("Cargo.toml");
            if !manifest.is_file() {
                continue;
            }
            checked += 1;
            let text = fs::read_to_string(&manifest)
                .unwrap_or_else(|e| panic!("read {}: {e}", manifest.display()));
            let declares_c_artifact = text.contains("cdylib") || text.contains("staticlib");
            let is_ffi = dir.file_name().and_then(|n| n.to_str()) == Some("lumio-native-ffi");
            assert_eq!(
                declares_c_artifact,
                is_ffi,
                "{} crate-type violates the single-symbol-surface rule",
                manifest.display()
            );
        }
        assert!(checked >= 2, "expected multiple crates under crates/");
    }

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
