---
status: completed
---

# 按 crate 映射搭 Cargo workspace 脚手架（不冻结任何公共契约）

Review §7.2 允许与文档整改并行的工程化部分：只建骨架与检查，不写公共 Header、不实现契约语义。

## 涉及范围

- `Cargo.toml`（workspace，成员按 `docs/specs/native-core-module-map.md` §3 的九个 crate）
- `crates/lumio-contract-types/`、`crates/lumio-kernel/`、`crates/lumio-job/`、`crates/lumio-spatial/`、`crates/lumio-codec/`、`crates/lumio-diagnostics/`、`crates/lumio-native-ffi/`、`crates/lumio-platform/`、`crates/lumio-test-support/`（各含最小 lib.rs 与模块级 doc 注释）
- `xtask/`（子命令骨架：`check-dep-dag`——按 spec §2 禁止方向断言 cargo metadata 依赖图；`dump-symbols`——对 lumio-native-ffi 产物做导出符号断言）
- `.github/workflows/repository-policy.yml`（追加 fmt/clippy/test/xtask 检查步骤）

## 验收标准

- [x] `cargo build --workspace` 与 `cargo test --workspace` 通过（空实现即可）
- [x] 唯一 cdylib/staticlib 是 lumio-native-ffi；lumio-codec/lumio-diagnostics 标 experimental feature-gate，默认不参与 ffi 依赖
- [x] `cargo xtask check-dep-dag` 能拒绝 spec §2 列出的每个禁止方向（xtask 单测 9 例覆盖全部禁边）
- [x] `cargo xtask dump-symbols` 断言 ffi 产物无跨仓 Root 符号（`lumio_core_get_api_v1` 不出现）
- [x] 公共 API 面为空或仅占位私有项：`cargo doc` 无任何声称契约已定的公开文档
- [x] CI 工作流本地复现通过（`native` job 已追加：fmt --check / clippy -D warnings / build / test / xtask 两命令，全部本地绿）

2026-08-27 执行记录：kernel-context 若编译期依赖 job 会在 crate 层成环，已按
ContextResource port 反转（编译期方向 job -> kernel-context），
`docs/specs/native-core-module-map.md` §2 已同步修正。

## 依赖

- 无（与 modules-doc-fixes 文件集不重叠，可并行）
