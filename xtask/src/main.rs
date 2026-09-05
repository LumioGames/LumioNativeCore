//! 仓库工程检查入口：`cargo xtask <command>`。
//!
//! - `check-dep-dag`：按 `docs/specs/native-core-module-map.md` §2 的白名单断言
//!   workspace crate 间的编译期依赖方向（normal 依赖，dev 依赖不计）。
//! - `assert-no-native-artifacts`：断言 workspace 内不存在 `cdylib` / `staticlib` 目标。
//!   本仓只产 rlib：跨语言边界与其唯一真值 `native-abi.json` 都在架构仓，本仓以 Rust
//!   路径依赖被那边的 SDK 编入（ADR 0009）。

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask is a workspace member")
        .to_path_buf()
}

/// crate -> 允许的 workspace 直接依赖（normal 图）。白名单以外的一切 workspace 边都算违规；
/// 非 workspace 的外部依赖必须出现在 `EXTERNAL_ALLOWLIST`。
fn allowed_deps() -> BTreeMap<&'static str, Vec<&'static str>> {
    BTreeMap::from([
        ("lumio-platform", vec![]),
        ("lumio-kernel", vec!["lumio-platform"]),
        ("lumio-job", vec!["lumio-kernel", "lumio-platform"]),
        ("lumio-spatial", vec!["lumio-kernel"]),
        ("lumio-timer", vec![]),
        ("lumio-codec", vec!["lumio-kernel"]),
        ("lumio-diagnostics", vec!["lumio-kernel", "lumio-platform"]),
        (
            "lumio-test-support",
            vec!["lumio-kernel", "lumio-job", "lumio-platform"],
        ),
        ("xtask", vec![]),
    ])
}

/// 当前批准的外部依赖（供应链任务批准前保持为空）。
const EXTERNAL_ALLOWLIST: &[&str] = &[];

/// 纯函数：对 (crate -> 直接依赖) 图做白名单校验，返回违规描述。
fn check_graph(
    graph: &BTreeMap<String, Vec<String>>,
    allowed: &BTreeMap<&str, Vec<&str>>,
    external_allowlist: &[&str],
) -> Vec<String> {
    let mut violations = Vec::new();
    for (krate, deps) in graph {
        let Some(allow) = allowed.get(krate.as_str()) else {
            violations.push(format!("未登记的 workspace crate: {krate}"));
            continue;
        };
        for dep in deps {
            let is_workspace = allowed.contains_key(dep.as_str());
            if is_workspace {
                if !allow.contains(&dep.as_str()) {
                    violations.push(format!("禁止的依赖方向: {krate} -> {dep}"));
                }
            } else if !external_allowlist.contains(&dep.as_str()) {
                violations.push(format!("未批准的外部依赖: {krate} -> {dep}"));
            }
        }
    }
    violations
}

/// 用 `cargo tree --depth 1` 取一个 crate 的直接 normal 依赖。
fn direct_deps(krate: &str, features: &[&str]) -> Result<Vec<String>, String> {
    let mut args = vec![
        "tree", "-p", krate, "-e", "normal", "--depth", "1", "--prefix", "depth",
    ];
    let feature_arg;
    if !features.is_empty() {
        feature_arg = features.join(",");
        args.push("--features");
        args.push(&feature_arg);
    }
    let out = Command::new("cargo")
        .args(&args)
        .output()
        .map_err(|e| format!("cargo tree 启动失败: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "cargo tree -p {krate} 失败: {}",
            String::from_utf8_lossy(&out.stderr)
        ));
    }
    let stdout = String::from_utf8_lossy(&out.stdout);
    let mut deps = Vec::new();
    for line in stdout.lines() {
        // `--prefix depth` 行如 `1lumio-kernel v0.0.0 (path)`；深度 1 即直接依赖。
        if let Some(name) = line
            .strip_prefix('1')
            .and_then(|rest| rest.split_whitespace().next())
        {
            deps.push(name.to_string());
        }
    }
    Ok(deps)
}

fn cmd_check_dep_dag() -> ExitCode {
    let allowed = allowed_deps();
    let mut graph: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for krate in allowed.keys() {
        match direct_deps(krate, &[]) {
            Ok(deps) => {
                graph.insert((*krate).to_string(), deps);
            }
            Err(err) => {
                eprintln!("error: {err}");
                return ExitCode::from(2);
            }
        }
    }
    let violations = check_graph(&graph, &allowed, EXTERNAL_ALLOWLIST);
    if violations.is_empty() {
        println!(
            "check-dep-dag OK：{} 个 crate，依赖方向全部符合白名单。",
            graph.len()
        );
        ExitCode::SUCCESS
    } else {
        for v in &violations {
            eprintln!("FAIL {v}");
        }
        ExitCode::FAILURE
    }
}

/// workspace 根 `Cargo.toml` 的 members 列表（相对路径，按声明顺序）。
fn workspace_members(root: &Path) -> Result<Vec<String>, String> {
    let text =
        fs::read_to_string(root.join("Cargo.toml")).map_err(|e| format!("read Cargo.toml: {e}"))?;
    let body = text
        .split_once("members = [")
        .ok_or("Cargo.toml 中找不到 workspace members")?
        .1;
    let body = body.split_once(']').ok_or("members 列表未闭合")?.0;
    Ok(body
        .lines()
        .filter_map(|line| {
            let line = line.trim().trim_end_matches(',');
            line.strip_prefix('"')?
                .strip_suffix('"')
                .map(str::to_string)
        })
        .collect())
}

/// 一个成员清单声明的 lib `crate-type` 值；未声明 `[lib] crate-type` 时为空（即默认 rlib）。
fn declared_crate_types(manifest: &str) -> Vec<String> {
    let Some(rest) = manifest.split_once("crate-type") else {
        return Vec::new();
    };
    let Some(rest) = rest.1.split_once('[') else {
        return Vec::new();
    };
    let Some((list, _)) = rest.1.split_once(']') else {
        return Vec::new();
    };
    list.split(',')
        .map(|kind| kind.trim().trim_matches('"').to_string())
        .filter(|kind| !kind.is_empty())
        .collect()
}

/// 违反「本仓只产 rlib」的目标：返回 `成员: crate-type` 描述列表。
fn native_artifact_violations(members: &[(String, Vec<String>)]) -> Vec<String> {
    const FORBIDDEN: [&str; 2] = ["cdylib", "staticlib"];
    let mut hits = Vec::new();
    for (member, kinds) in members {
        for kind in kinds {
            if FORBIDDEN.contains(&kind.as_str()) {
                hits.push(format!("{member}: {kind}"));
            }
        }
    }
    hits
}

fn cmd_assert_no_native_artifacts() -> ExitCode {
    let root = workspace_root();
    let members = match workspace_members(&root) {
        Ok(m) => m,
        Err(err) => {
            eprintln!("error: {err}");
            return ExitCode::from(2);
        }
    };
    if members.is_empty() {
        eprintln!("error: workspace members 解析为空");
        return ExitCode::from(2);
    }
    let mut declared = Vec::new();
    for member in &members {
        let path = root.join(member).join("Cargo.toml");
        match fs::read_to_string(&path) {
            Ok(text) => declared.push((member.clone(), declared_crate_types(&text))),
            Err(e) => {
                eprintln!("error: read {}: {e}", path.display());
                return ExitCode::from(2);
            }
        }
    }
    let violations = native_artifact_violations(&declared);
    if violations.is_empty() {
        println!(
            "assert-no-native-artifacts OK：{} 个 workspace 成员，无 cdylib / staticlib 目标。",
            declared.len()
        );
        ExitCode::SUCCESS
    } else {
        for v in &violations {
            eprintln!("FAIL 本仓不得产原生库目标（ADR 0009）：{v}");
        }
        ExitCode::FAILURE
    }
}

fn main() -> ExitCode {
    let arg = std::env::args().nth(1);
    match arg.as_deref() {
        Some("check-dep-dag") => cmd_check_dep_dag(),
        Some("assert-no-native-artifacts") => cmd_assert_no_native_artifacts(),
        _ => {
            eprintln!("用法: cargo xtask <check-dep-dag|assert-no-native-artifacts>");
            ExitCode::from(2)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn graph(edges: &[(&str, &[&str])]) -> BTreeMap<String, Vec<String>> {
        edges
            .iter()
            .map(|(k, deps)| {
                (
                    (*k).to_string(),
                    deps.iter().map(|d| (*d).to_string()).collect(),
                )
            })
            .collect()
    }

    fn assert_rejects(edge_from: &str, edge_to: &str) {
        let g = graph(&[(edge_from, &[edge_to])]);
        let violations = check_graph(&g, &allowed_deps(), EXTERNAL_ALLOWLIST);
        assert!(
            violations
                .iter()
                .any(|v| v.contains(edge_from) && v.contains(edge_to)),
            "禁边 {edge_from} -> {edge_to} 未被拒绝: {violations:?}"
        );
    }

    #[test]
    fn rejects_kernel_to_job_cycle_direction() {
        assert_rejects("lumio-kernel", "lumio-job");
    }

    #[test]
    fn rejects_core_modules_depending_on_diagnostics_impl() {
        assert_rejects("lumio-kernel", "lumio-diagnostics");
        assert_rejects("lumio-job", "lumio-diagnostics");
    }

    #[test]
    fn rejects_spatial_and_codec_to_job() {
        assert_rejects("lumio-spatial", "lumio-job");
        assert_rejects("lumio-codec", "lumio-job");
    }

    #[test]
    fn rejects_leaf_gaining_dependencies() {
        assert_rejects("lumio-platform", "lumio-kernel");
    }

    #[test]
    fn rejects_test_support_in_normal_graph() {
        assert_rejects("lumio-job", "lumio-test-support");
    }

    #[test]
    fn rejects_unapproved_external_dependency() {
        assert_rejects("lumio-kernel", "serde");
    }

    #[test]
    fn rejects_unregistered_workspace_crate() {
        let g = graph(&[("lumio-unknown", &[])]);
        let violations = check_graph(&g, &allowed_deps(), EXTERNAL_ALLOWLIST);
        assert!(!violations.is_empty());
    }

    #[test]
    fn declared_crate_types_reads_the_lib_section() {
        let manifest = "[package]\nname = \"x\"\n\n[lib]\ncrate-type = [\"cdylib\", \"staticlib\", \"rlib\"]\n";
        assert_eq!(
            declared_crate_types(manifest),
            vec!["cdylib", "staticlib", "rlib"]
        );
        assert!(declared_crate_types("[package]\nname = \"x\"\n").is_empty());
    }

    #[test]
    fn rejects_a_cdylib_or_staticlib_target() {
        let rows = vec![
            ("crates/lumio-kernel".to_string(), vec!["rlib".to_string()]),
            ("crates/rogue".to_string(), vec!["cdylib".to_string()]),
            ("crates/rogue2".to_string(), vec!["staticlib".to_string()]),
        ];
        assert_eq!(
            native_artifact_violations(&rows),
            vec!["crates/rogue: cdylib", "crates/rogue2: staticlib"]
        );
    }

    /// 真实 workspace 必须满足这条线，而不只是纯函数满足。
    #[test]
    fn live_workspace_has_no_native_artifacts() {
        let root = workspace_root();
        let members = workspace_members(&root).expect("parse workspace members");
        assert!(!members.is_empty(), "workspace members parsed empty");
        let declared: Vec<(String, Vec<String>)> = members
            .iter()
            .map(|m| {
                let text = std::fs::read_to_string(root.join(m).join("Cargo.toml"))
                    .unwrap_or_else(|e| panic!("read {m}/Cargo.toml: {e}"));
                (m.clone(), declared_crate_types(&text))
            })
            .collect();
        let hits = native_artifact_violations(&declared);
        assert!(hits.is_empty(), "unexpected native artifacts: {hits:?}");
    }

    #[test]
    fn accepts_the_specified_dag() {
        let g = graph(&[
            ("lumio-platform", &[] as &[&str]),
            ("lumio-kernel", &["lumio-platform"]),
            ("lumio-job", &["lumio-kernel", "lumio-platform"]),
            ("lumio-spatial", &["lumio-kernel"]),
            ("lumio-timer", &[]),
            ("lumio-codec", &["lumio-kernel"]),
            ("lumio-diagnostics", &["lumio-kernel"]),
            (
                "lumio-test-support",
                &["lumio-kernel", "lumio-job", "lumio-platform"],
            ),
            ("xtask", &[]),
        ]);
        let violations = check_graph(&g, &allowed_deps(), EXTERNAL_ALLOWLIST);
        assert!(violations.is_empty(), "{violations:?}");
    }
}
