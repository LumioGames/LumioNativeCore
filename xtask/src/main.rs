//! 仓库工程检查入口：`cargo xtask <command>`。
//!
//! - `check-dep-dag`：按 `docs/specs/native-core-module-map.md` §2 的白名单断言
//!   workspace crate 间的编译期依赖方向（normal 依赖，dev 依赖不计）。
//! - `dump-symbols`：构建 `lumio-native-ffi` cdylib 并断言符号表不含跨仓 Root 符号
//!   （ADR 0001：`lumio_core_get_api_v1` 归 CoreEngine root-abi）。
//! - `check-baseline`：活动入口、模块 README、CI 与镜像 Hash 对齐 `LGE-V1.4-2026-08-27`。
//! - `gen-contracts`：从 `docs/architecture/abi/` 镜像重新生成
//!   `crates/lumio-contract-types/src/generated_data.rs`（生成物不手改）。

mod baseline;
mod contracts;

use std::collections::BTreeMap;
use std::process::{Command, ExitCode};

/// 跨仓 Root 符号：NativeCore 产物中出现即失败（ADR 0001）。
/// `dump-symbols` 会与镜像 bundle 的 `entrySymbol` 交叉校验，防止硬编码漂移。
const FORBIDDEN_ROOT_SYMBOL: &str = "lumio_core_get_api_v1";

/// 已批准导出的 provider 符号。provider 组合契约未发布（T-ffi-04 blocked
/// 半边），列表保持为空：任何 `symbolPrefix` 前缀导出都视为违规。
const APPROVED_PROVIDER_EXPORTS: &[&str] = &[];

/// crate -> 允许的 workspace 直接依赖（normal 图）。白名单以外的一切 workspace 边都算违规；
/// 非 workspace 的外部依赖必须出现在 `EXTERNAL_ALLOWLIST`。
fn allowed_deps() -> BTreeMap<&'static str, Vec<&'static str>> {
    BTreeMap::from([
        ("lumio-contract-types", vec![]),
        ("lumio-platform", vec![]),
        (
            "lumio-kernel",
            vec!["lumio-contract-types", "lumio-platform"],
        ),
        (
            "lumio-job",
            vec!["lumio-contract-types", "lumio-kernel", "lumio-platform"],
        ),
        (
            "lumio-spatial",
            vec!["lumio-contract-types", "lumio-kernel"],
        ),
        ("lumio-timer", vec!["lumio-platform"]),
        ("lumio-codec", vec!["lumio-contract-types", "lumio-kernel"]),
        (
            "lumio-diagnostics",
            vec!["lumio-contract-types", "lumio-kernel", "lumio-platform"],
        ),
        (
            "lumio-native-ffi",
            vec![
                "lumio-contract-types",
                "lumio-kernel",
                "lumio-job",
                "lumio-spatial",
                "lumio-platform",
                // 仅 experimental feature 下可出现（pending 模块，ADR 0005）：
                "lumio-codec",
                "lumio-diagnostics",
            ],
        ),
        (
            "lumio-test-support",
            vec![
                "lumio-contract-types",
                "lumio-kernel",
                "lumio-job",
                "lumio-platform",
            ],
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
    // 默认图中 pending 模块不得挂在 ffi 上（ADR 0005：默认发布面不含 experimental）。
    if let Some(ffi_deps) = graph.get("lumio-native-ffi") {
        for pending in ["lumio-codec", "lumio-diagnostics"] {
            if ffi_deps.iter().any(|d| d == pending) {
                eprintln!("FAIL 默认 feature 下 lumio-native-ffi 不得依赖 {pending}");
                return ExitCode::FAILURE;
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

/// 从 `cargo metadata` 解析 target 目录（CI/沙箱可能重定向 CARGO_TARGET_DIR，不可硬编码）。
fn target_directory() -> Result<String, String> {
    let out = Command::new("cargo")
        .args(["metadata", "--format-version", "1", "--no-deps"])
        .output()
        .map_err(|e| format!("cargo metadata 启动失败: {e}"))?;
    if !out.status.success() {
        return Err("cargo metadata 失败".to_string());
    }
    let text = String::from_utf8_lossy(&out.stdout);
    let key = "\"target_directory\":\"";
    let start = text.find(key).ok_or("metadata 中无 target_directory")? + key.len();
    let rest = &text[start..];
    let end = rest.find('"').ok_or("target_directory 未闭合")?;
    Ok(rest[..end].replace("\\\\", "\\"))
}

fn cmd_dump_symbols() -> ExitCode {
    let build = Command::new("cargo")
        .args(["build", "-p", "lumio-native-ffi"])
        .status();
    match build {
        Ok(s) if s.success() => {}
        other => {
            eprintln!("error: 构建 lumio-native-ffi 失败: {other:?}");
            return ExitCode::from(2);
        }
    }
    let target_dir = match target_directory() {
        Ok(dir) => dir,
        Err(err) => {
            eprintln!("error: {err}");
            return ExitCode::from(2);
        }
    };
    let file = if cfg!(target_os = "macos") {
        "liblumio_native_ffi.dylib"
    } else if cfg!(target_os = "windows") {
        "lumio_native_ffi.dll"
    } else {
        "liblumio_native_ffi.so"
    };
    let artifact = format!("{target_dir}/debug/{file}");
    let out = Command::new("nm").args(["-gU", &artifact]).output();
    let out = match out {
        Ok(o) if o.status.success() => o,
        other => {
            eprintln!("error: nm -gU {artifact} 失败: {other:?}");
            return ExitCode::from(2);
        }
    };
    let stdout = String::from_utf8_lossy(&out.stdout);
    let mut exported: Vec<String> = Vec::new();
    for line in stdout.lines() {
        // nm 行如 `0000000000001234 T _lumio_xxx`；取符号列并去掉 macOS 下划线前缀。
        if let Some(sym) = line.split_whitespace().last() {
            exported.push(sym.trim_start_matches('_').to_string());
        }
    }

    // 符号策略来自镜像 bundle；与硬编码交叉校验，防止两边各自漂移。
    let (entry_symbol, symbol_prefix) =
        match contracts::abi_symbol_policy(&baseline::workspace_root()) {
            Ok(policy) => policy,
            Err(err) => {
                eprintln!("error: 读取镜像符号策略失败: {err}");
                return ExitCode::from(2);
            }
        };
    let mut failed = false;
    if entry_symbol != FORBIDDEN_ROOT_SYMBOL {
        eprintln!(
            "FAIL 镜像 entrySymbol `{entry_symbol}` 与硬编码 Root 符号 `{FORBIDDEN_ROOT_SYMBOL}` 不一致"
        );
        failed = true;
    }
    if exported.iter().any(|s| s == FORBIDDEN_ROOT_SYMBOL) {
        eprintln!(
            "FAIL 产物导出了跨仓 Root 符号 {FORBIDDEN_ROOT_SYMBOL}（归 CoreEngine root-abi，ADR 0001）"
        );
        failed = true;
    }
    let unapproved: Vec<&String> = exported
        .iter()
        .filter(|s| s.starts_with(&symbol_prefix))
        .filter(|s| !APPROVED_PROVIDER_EXPORTS.contains(&s.as_str()))
        .collect();
    if unapproved.is_empty() {
        println!(
            "dump-symbols：{symbol_prefix}* 导出 0 个未批准符号（批准列表 {} 项）",
            APPROVED_PROVIDER_EXPORTS.len()
        );
    } else {
        eprintln!(
            "FAIL 未批准的 {symbol_prefix}* 导出：{unapproved:?}（provider 符号列表未发布，批准列表为空）"
        );
        failed = true;
    }
    if failed {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

fn cmd_check_baseline() -> ExitCode {
    match baseline::audit_v14_activity_refs(&baseline::workspace_root()) {
        Ok(()) => {
            println!(
                "check-baseline OK：活动入口对齐 {}，镜像 {}",
                baseline::BASELINE_ID,
                baseline::MIRROR_REL
            );
            ExitCode::SUCCESS
        }
        Err(errors) => {
            for e in errors {
                eprintln!("FAIL {e}");
            }
            ExitCode::FAILURE
        }
    }
}

fn cmd_gen_contracts() -> ExitCode {
    let root = baseline::workspace_root();
    match contracts::write_generated_data(&root) {
        Ok(true) => {
            println!("gen-contracts：已更新 {}", contracts::GENERATED_DATA_REL);
            ExitCode::SUCCESS
        }
        Ok(false) => {
            println!("gen-contracts：{} 已是最新", contracts::GENERATED_DATA_REL);
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

fn main() -> ExitCode {
    let arg = std::env::args().nth(1);
    match arg.as_deref() {
        Some("check-dep-dag") => cmd_check_dep_dag(),
        Some("dump-symbols") => cmd_dump_symbols(),
        Some("check-baseline") => cmd_check_baseline(),
        Some("gen-contracts") => cmd_gen_contracts(),
        _ => {
            eprintln!(
                "用法: cargo xtask <check-dep-dag|dump-symbols|check-baseline|gen-contracts>"
            );
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
        assert_rejects("lumio-contract-types", "lumio-kernel");
        assert_rejects("lumio-platform", "lumio-contract-types");
    }

    #[test]
    fn rejects_reverse_edge_into_ffi_facade() {
        assert_rejects("lumio-kernel", "lumio-native-ffi");
        assert_rejects("lumio-job", "lumio-native-ffi");
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
    fn v14_activity_refs_match_live_execution_truth() {
        let root = crate::baseline::workspace_root();
        crate::baseline::audit_v14_activity_refs(&root).unwrap_or_else(|errors| {
            panic!(
                "R-00010 activity-entry audit failed:\n{}",
                errors.join("\n")
            );
        });
    }

    /// 生成物不得手改：从镜像重推导必须与已提交的 generated_data.rs 逐字节一致。
    #[test]
    fn generated_data_matches_mirror_derivation() {
        let root = crate::baseline::workspace_root();
        let derived = crate::contracts::derive_generated_data(&root)
            .unwrap_or_else(|e| panic!("derive generated_data: {e}"));
        let path = crate::contracts::generated_data_path(&root);
        let committed = std::fs::read_to_string(&path).unwrap_or_else(|e| {
            panic!(
                "read {}: {e}（先跑 cargo xtask gen-contracts）",
                path.display()
            )
        });
        assert_eq!(
            committed, derived,
            "generated_data.rs 与镜像推导不一致：重跑 `cargo xtask gen-contracts` 并与镜像一起提交"
        );
    }

    #[test]
    fn v14_mirror_digest_in_baseline_file_matches_hashed_bytes() {
        let root = crate::baseline::workspace_root();
        let sha_path = root.join(crate::baseline::BASELINE_SHA_REL);
        let body = std::fs::read_to_string(&sha_path).expect("read .baseline.sha256");
        let rows = crate::baseline::parse_baseline_sha_file(&body).expect("parse .baseline.sha256");
        assert!(
            rows.iter()
                .any(|(_, rel)| rel == crate::baseline::MIRROR_REL)
        );
        for required in crate::baseline::ABI_MIRROR_RELS {
            assert!(
                rows.iter().any(|(_, rel)| rel == required),
                ".baseline.sha256 must pin {required}"
            );
        }
        for (expected, rel) in &rows {
            let actual = crate::baseline::file_sha256_hex(
                &root.join(rel.replace('/', std::path::MAIN_SEPARATOR_STR)),
            )
            .unwrap_or_else(|e| panic!("hash {rel}: {e}"));
            assert_eq!(&actual, expected, "pinned digest mismatch for {rel}");
        }
    }

    #[test]
    fn accepts_the_specified_dag() {
        let g = graph(&[
            ("lumio-contract-types", &[] as &[&str]),
            ("lumio-platform", &[]),
            ("lumio-kernel", &["lumio-contract-types", "lumio-platform"]),
            (
                "lumio-job",
                &["lumio-contract-types", "lumio-kernel", "lumio-platform"],
            ),
            ("lumio-spatial", &["lumio-contract-types", "lumio-kernel"]),
            ("lumio-timer", &["lumio-platform"]),
            ("lumio-codec", &["lumio-contract-types", "lumio-kernel"]),
            (
                "lumio-diagnostics",
                &["lumio-contract-types", "lumio-kernel"],
            ),
            (
                "lumio-native-ffi",
                &[
                    "lumio-contract-types",
                    "lumio-kernel",
                    "lumio-job",
                    "lumio-spatial",
                    "lumio-platform",
                ],
            ),
            (
                "lumio-test-support",
                &[
                    "lumio-contract-types",
                    "lumio-kernel",
                    "lumio-job",
                    "lumio-platform",
                ],
            ),
            ("xtask", &[]),
        ]);
        let violations = check_graph(&g, &allowed_deps(), EXTERNAL_ALLOWLIST);
        assert!(violations.is_empty(), "{violations:?}");
    }
}
