# Native Timer ABI 只读镜像

本目录镜像 `LumioGameEngineArchitecture` 已冻结的进程内 Timer API 契约（ADR-055）。本仓只消费，不另写语义真值，不把该契约并入 Root ABI / `engine/abi/native-abi.json`。

## 钉住的上游 revision（钉 revision，不钉分支名）

- 上游仓库：`LumioGameEngineArchitecture`（`https://github.com/LumioGames/LumioGameEngineArchitecture.git`）
- 镜像 revision：`2b7e321`（`origin/main` merge of wave0 contracts，含 ADR-055 与 repeating 窗口 `(committedTick, toTick]`）
- SHA-256：`f2eff09b44f9ebddec9dc0b5a31228fd1ec85073e399c42079d5564a61e16ba7`
- 架构基线：`LGE-V1.4-2026-08-27`（模块地图不含 Timer；Timer 由 ADR-055 冻结）

## 文件清单与上游路径

| 本目录文件 | 上游路径 | 完整性保证 |
| --- | --- | --- |
| `native-timer-abi-v1.json` | `engine/wire/native-timer-abi-v1.json` | 钉 revision 对象身份 + `.baseline.sha256` |

字节级镜像，不得手改。更新时 `git show <rev>:engine/wire/native-timer-abi-v1.json` 覆盖并重算 SHA-256。
