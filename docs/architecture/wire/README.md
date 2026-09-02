# Native Timer ABI 只读镜像

本目录镜像 `LumioGameEngineArchitecture` 已冻结的 Native 单一定时内核契约（ADR-056 §7 修订 ADR-055：单内核双模式 wallClock + tickFrame）。本仓只消费，不另写语义真值。托管可达面以架构源 `engine/abi/native-abi.json` 的 `timer_*` 槽为准。

## 钉住的上游 revision（钉 revision，不钉分支名）

- 上游仓库：`LumioGameEngineArchitecture`（`https://github.com/LumioGames/LumioGameEngineArchitecture.git`）
- 镜像 revision：`936046a64a7fd75fc1672ceb5d458c7195052fb7`（Arch `origin/main`；C-4′ 合入 `1b25573`，后续 C-1/C-2 不改 ABI 槽序）
- SHA-256：`f1a766daf912e4c52eea8922fcce290c73641695abc47ade9f7762a177e1ff71`
- `native-abi.json` DEFINITION_SHA256：`ee2f6c6dc2e73a58561ba82325bc1c7c12fbfee52e94e9466642bd0a38510a41`
- 架构基线：`LGE-V1.4-2026-08-27`（Timer 由 ADR-055 冻结，分层由 ADR-056 §7 修订为进 ABI）

## 文件清单与上游路径

| 本目录文件 | 上游路径 | 完整性保证 |
| --- | --- | --- |
| `native-timer-abi-v1.json` | `engine/wire/native-timer-abi-v1.json` | 钉 revision 对象身份 + SHA-256 |

字节级镜像，不得手改。更新时 `git show <rev>:engine/wire/native-timer-abi-v1.json` 覆盖并重算 SHA-256。
