# Root ABI 发布物只读镜像

本目录镜像 `LumioGameEngineArchitecture` 已发布的 Root ABI 消费面（ADR-040 Root ABI Generated Bundle；该 ADR 当前状态为 **Draft**，但发布物本身已在上游 `origin/main`）。本仓在 `packages/index.json` 的 `rootAbi.consumers` 中登记为消费方，按 ADR-040 §7 只消费 C Header 与索引，不消费 Rust/C# 生成包。

## 钉住的上游 revision（钉 revision，不钉分支名）

- 上游仓库：`LumioGameEngineArchitecture`（`https://github.com/LumioGames/LumioGameEngineArchitecture.git`）
- 镜像 revision：`origin/main` 提交 `1f2ead332b3dfc3042e1495bfbe6febb8699df7e`（含内容提交 `5c222c4`，2026-08-28）
- 架构基线：`LGE-V1.4-2026-08-27`

## 文件清单与上游路径

| 本目录文件 | 上游路径 | 完整性保证 |
| --- | --- | --- |
| `lumio_core.h` | `packages/abi/lumio_core.h` | bundle `outputFiles[].digest`（自校验）+ `.baseline.sha256` |
| `root-abi-bundle.json` | `packages/abi/root-abi-bundle.json` | `packages/index.json` 的 `rootAbi.bundleDigest`（自校验）+ `.baseline.sha256` |
| `ids-index.json` | `ids/index.json` | V1 无 per-file digest；钉住的镜像 revision 对象身份 + `.baseline.sha256` |
| `packages-index.json` | `packages/index.json` | 同上 |

`.baseline.sha256`（`docs/architecture/.baseline.sha256`）钉住上表四个镜像文件的 SHA-256（本 README 是本仓维护的说明文档，不入 pin），由 Repository Policy CI 的 `sha256sum -c` 与 `cargo xtask check-baseline` 共同校验；`root-abi-bundle.json` 与 `lumio_core.h` 另有上游自发布 digest 交叉校验（见 `crates/lumio-contract-types` 测试）。

## 消费纪律（ADR-040 §7）

- 数值权威只有 `ids/index.json`；生成包只发布 id 字符串，从生成包读 ordinal 等于读未发布之物。
- V1 布局 Golden 只发布 `linux-x86_64-glibc` 一档，其余平台布局不得断言（D-016 待裁决）。
- `capability_bits` 是掩码还是计数、以及任何 bit 位指派，V1 均未冻结；`Capability` 命名空间 numeric 是枚举序号，不是 bit 位（D-015 待裁决）。
- 不存在 `OperationId` 命名空间；公共操作身份是 (`apiTable[].name`, `slots[].slotIndex`)。
- 跨仓 Root 符号（`LUMIO_ENTRY_SYMBOL`）由 CoreEngine `root-abi`/`composition` 独占导出；本仓发布物不得导出。

## 更新流程

1. 在上游仓核实目标提交已在 `origin/main`（`git branch -r --contains <rev>`）。
2. `git show <rev>:<上游路径>` 覆盖本目录对应文件（字节级，不得手改）。
3. 重算四个文件的 SHA-256 更新 `docs/architecture/.baseline.sha256`，并更新本文件的 revision 记录。
4. 运行 `cargo xtask gen-contracts` 重新生成 `crates/lumio-contract-types/src/registry_data.rs`，与镜像一起提交。
5. 跑收口门槛（workspace 测试 + clippy + `cargo xtask check-baseline`）确认绑定测试全绿。
