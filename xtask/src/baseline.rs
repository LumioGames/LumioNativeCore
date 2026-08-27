//! Activity-entry audit for architecture baseline `LGE-V1.4-2026-08-27`.
//!
//! Historical mirrors (`v1.0`–`v1.3`) and completed task records stay untouched;
//! this module only checks the files that R-00010 names as live execution truth.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

pub const BASELINE_ID: &str = "LGE-V1.4-2026-08-27";
pub const MIRROR_REL: &str = "docs/architecture/LumioGameEngine_Architecture_v1.4.md";
pub const BASELINE_SHA_REL: &str = "docs/architecture/.baseline.sha256";
pub const FRAME_REL: &str = "docs/2026-08-27-native-core-module-implementation-frame.md";
const STALE_MIRROR: &str = "LumioGameEngine_Architecture_v1.2.md";
const STALE_BASELINE: &str = "LGE-V1.2-2026-08-27";

pub fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask is a workspace member")
        .to_path_buf()
}

fn read(root: &Path, rel: &str) -> Result<String, String> {
    fs::read_to_string(root.join(rel)).map_err(|e| format!("read {rel}: {e}"))
}

fn must_contain(errors: &mut Vec<String>, rel: &str, body: &str, needle: &str) {
    if !body.contains(needle) {
        errors.push(format!("{rel} missing `{needle}`"));
    }
}

fn must_not_contain(errors: &mut Vec<String>, rel: &str, body: &str, needle: &str) {
    if body.contains(needle) {
        errors.push(format!(
            "{rel} still contains stale activity text `{needle}`"
        ));
    }
}

/// SHA-256 hex of `path` via `sha256sum` (CI) or `certutil` (Windows).
pub fn file_sha256_hex(path: &Path) -> Result<String, String> {
    if let Ok(out) = Command::new("sha256sum").arg(path).output() {
        if out.status.success() {
            let text = String::from_utf8_lossy(&out.stdout);
            if let Some(hex) = text.split_whitespace().next() {
                return Ok(hex.to_ascii_lowercase());
            }
        }
    }
    let out = Command::new("certutil")
        .args(["-hashfile", &path.to_string_lossy(), "SHA256"])
        .output()
        .map_err(|e| format!("certutil/sha256sum failed for {}: {e}", path.display()))?;
    if !out.status.success() {
        return Err(format!(
            "hash failed for {}: {}",
            path.display(),
            String::from_utf8_lossy(&out.stderr)
        ));
    }
    let text = String::from_utf8_lossy(&out.stdout);
    for line in text.lines() {
        let compact: String = line.chars().filter(|c| !c.is_whitespace()).collect();
        if compact.len() == 64 && compact.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Ok(compact.to_ascii_lowercase());
        }
    }
    Err(format!(
        "no SHA-256 hex in hasher output for {}",
        path.display()
    ))
}

pub fn parse_baseline_sha_file(body: &str) -> Result<(String, String), String> {
    let line = body
        .lines()
        .find(|l| !l.trim().is_empty())
        .ok_or_else(|| ".baseline.sha256 is empty".to_string())?;
    let mut parts = line.split_whitespace();
    let hex = parts
        .next()
        .ok_or_else(|| ".baseline.sha256 missing digest".to_string())?
        .to_ascii_lowercase();
    let rel = parts
        .next()
        .ok_or_else(|| ".baseline.sha256 missing path".to_string())?
        .replace('\\', "/");
    if hex.len() != 64 {
        return Err(format!(".baseline.sha256 digest length {}", hex.len()));
    }
    Ok((hex, rel))
}

fn audit_agents(root: &Path, errors: &mut Vec<String>) {
    let rel = ".spec/AGENTS.md";
    match read(root, rel) {
        Ok(body) => {
            must_contain(errors, rel, &body, BASELINE_ID);
            if body.contains("当前架构基线是 `LGE-V1.2") {
                errors.push(format!("{rel} still declares V1.2 as the current baseline"));
            }
        }
        Err(e) => errors.push(e),
    }
}

fn audit_repository_architecture(root: &Path, errors: &mut Vec<String>) {
    let rel = ".spec/knowledge/standards/repository-architecture.md";
    match read(root, rel) {
        Ok(body) => {
            must_contain(errors, rel, &body, BASELINE_ID);
            must_contain(errors, rel, &body, "LumioGameEngine_Architecture_v1.4.md");
            must_not_contain(errors, rel, &body, "LumioGameEngine_Architecture_v1.2.md");
        }
        Err(e) => errors.push(e),
    }
}

fn audit_readme(root: &Path, errors: &mut Vec<String>) {
    let rel = "README.md";
    match read(root, rel) {
        Ok(body) => {
            must_contain(errors, rel, &body, BASELINE_ID);
            must_contain(
                errors,
                rel,
                &body,
                "docs/architecture/LumioGameEngine_Architecture_v1.4.md",
            );
            if body.contains("本地镜像：[`docs/architecture/LumioGameEngine_Architecture_v1.2.md`]")
            {
                errors.push(format!("{rel} activity mirror still points at v1.2"));
            }
            if body.contains("`LGE-V1.2` §16") {
                errors.push(format!("{rel} still cites V1.2 §16 as the live module map"));
            }
        }
        Err(e) => errors.push(e),
    }
}

fn audit_module_readmes(root: &Path, errors: &mut Vec<String>) {
    let modules = root.join("modules");
    let entries = match fs::read_dir(&modules) {
        Ok(e) => e,
        Err(e) => {
            errors.push(format!("read modules/: {e}"));
            return;
        }
    };
    let mut found = 0usize;
    for entry in entries.flatten() {
        let readme = entry.path().join("README.md");
        if !readme.is_file() {
            continue;
        }
        found += 1;
        let rel = format!("modules/{}/README.md", entry.file_name().to_string_lossy());
        match fs::read_to_string(&readme) {
            Ok(body) => {
                must_contain(errors, &rel, &body, "**架构基线**：`LGE-V1.4-2026-08-27`");
                if body.contains("LGE-V1.2") {
                    errors.push(format!(
                        "{rel} still uses V1.2 in BaselineStatus or 架构基线"
                    ));
                }
            }
            Err(e) => errors.push(format!("read {rel}: {e}")),
        }
    }
    if found == 0 {
        errors.push("no modules/*/README.md found".to_string());
    }
}

fn audit_frame_header(root: &Path, errors: &mut Vec<String>) {
    match read(root, FRAME_REL) {
        Ok(body) => {
            let header: String = body.lines().take(8).collect::<Vec<_>>().join("\n");
            if header.contains("docs/specs/2026-08-27-native-core-module-implementation-frame.md") {
                errors.push(format!(
                    "{FRAME_REL} header still names the non-existent docs/specs/ path"
                ));
            }
            if !header.contains(FRAME_REL) {
                errors.push(format!("{FRAME_REL} header does not name its actual path"));
            }
            if !header.contains(BASELINE_ID) {
                errors.push(format!("{FRAME_REL} header missing {BASELINE_ID}"));
            }
            if header.contains(STALE_BASELINE) {
                errors.push(format!(
                    "{FRAME_REL} header still declares {STALE_BASELINE}"
                ));
            }
        }
        Err(e) => errors.push(e),
    }
}

fn audit_repository_policy(root: &Path, errors: &mut Vec<String>) {
    let rel = ".github/workflows/repository-policy.yml";
    match read(root, rel) {
        Ok(body) => {
            must_contain(errors, rel, &body, MIRROR_REL);
            must_contain(errors, rel, &body, "# LumioGameEngine V3 (v1.4)");
            must_contain(errors, rel, &body, BASELINE_ID);
            must_contain(errors, rel, &body, ".baseline.sha256");
            if body.contains(&format!("grep -q '{STALE_BASELINE}' README.md")) {
                errors.push(format!("{rel} still asserts V1.2 BaselineId in README.md"));
            }
            if body.contains(&format!("grep -q '^# LumioGameEngine V3 (v1.2)' {STALE_MIRROR}"))
                || body.contains("grep -q '^# LumioGameEngine V3 (v1.2)' docs/architecture/LumioGameEngine_Architecture_v1.2.md")
            {
                errors.push(format!("{rel} still treats v1.2 as the activity mirror"));
            }
        }
        Err(e) => errors.push(e),
    }
}

fn audit_mirror_hash(root: &Path, errors: &mut Vec<String>) {
    let sha_body = match read(root, BASELINE_SHA_REL) {
        Ok(b) => b,
        Err(e) => {
            errors.push(e);
            return;
        }
    };
    let (expected, rel) = match parse_baseline_sha_file(&sha_body) {
        Ok(v) => v,
        Err(e) => {
            errors.push(e);
            return;
        }
    };
    if rel != MIRROR_REL {
        errors.push(format!(
            ".baseline.sha256 path `{rel}` is not the v1.4 activity mirror"
        ));
    }
    let path = root.join(rel.replace('/', std::path::MAIN_SEPARATOR_STR));
    match file_sha256_hex(&path) {
        Ok(actual) if actual == expected => {}
        Ok(actual) => errors.push(format!(
            "v1.4 mirror SHA-256 {actual} != .baseline.sha256 {expected}"
        )),
        Err(e) => errors.push(e),
    }
}

/// Returns Ok(()) when every R-00010 activity-entry check holds.
pub fn audit_v14_activity_refs(root: &Path) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();
    audit_agents(root, &mut errors);
    audit_repository_architecture(root, &mut errors);
    audit_readme(root, &mut errors);
    audit_module_readmes(root, &mut errors);
    audit_frame_header(root, &mut errors);
    audit_repository_policy(root, &mut errors);
    audit_mirror_hash(root, &mut errors);
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}
