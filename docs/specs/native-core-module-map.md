# NativeCore 模块依赖图与 crate 映射（设计现状）

> 对应决策：[`0005`](../../.spec/decisions/0005-codec-diagnostics-pending-and-dual-status.md)、
> [`0009`](../../.spec/decisions/0009-exit-legacy-contract-regime.md)（取代 [`0001`](../../.spec/decisions/0001-contract-layering-and-symbol-surface.md) 的分层与符号面结论）。
> 本仓不定义任何跨语言 ABI：唯一 ABI 真值是架构仓 `LumioGameEngine` 的 `engine/abi/native-abi.json`，
> 插头代码也在那边。本文只描述本仓内部的编译边界。

## 1. 对外形态

NativeCore 只产出 rlib。仓内**没有 `cdylib` / `staticlib` 目标，不导出任何 C 符号，不组装 API Table**。
架构仓 `engine/native/modules/sdk-native`（crate `lumio-engine-native`）以 Cargo 路径依赖引用本仓 crate 源码，
编进它自己的动态库；跨语言可达面、状态码与槽位全部由 `native-abi.json` 定义。

因此本仓的稳定边界是**crate 的公开 Rust API**，不是 C ABI：改公开 API 即改 SDK 的编译输入，
必须在架构仓 `engine/native` 复跑 `cargo build -p lumio-engine-native` 与 `cargo test -p lumio-engine-native`。
`cargo xtask assert-no-native-artifacts` 机械守住「仓内不出现 cdylib / staticlib 目标」这条线。

## 2. 模块依赖图（编译期，无环）

```text
lumio-platform（零依赖叶子：monotonic clock port、tick 标量）
        │
     lumio-kernel ──> lumio-platform
        │  （error / capability / handle / memory / kernel-context 五个文档模块）
 ┌──────┼──────────┬─────────────┐
job    spatial    codec       diagnostics
 │                              （codec / diagnostics 为 pending，feature 默认关）
 └── job ──> kernel + platform

lumio-timer：零依赖内核，自带确定性刻度与单调毫秒到期，不读墙钟，不依赖 kernel / platform

kernel-context（在 lumio-kernel 内）定义 ContextResource port
job / spatial / codec ──实现并注册──> ContextResource port
        （编译期方向是 job -> lumio-kernel；Context 经 port 拥有 Worker 集 / 索引 / 工作区，避免 crate 层环）
```

禁止方向（`cargo xtask check-dep-dag` 拒绝）：`spatial/codec -> job`、`error -> diagnostics`、
`memory -> job/spatial/codec`、核心模块 -> diagnostics 实现、`kernel -> job`、
NativeCore -> 任何 Lumio 上层仓库、未登记的外部依赖。

## 3. 文档模块 ↔ crate 映射

`modules/` 目录保持**文档分类**职责；编译边界按下表，后续调整须新 ADR：

| crate | 承载文档模块 | 类型 |
| --- | --- | --- |
| `lumio-platform` | —（monotonic clock port 等） | rlib，private |
| `lumio-kernel` | error、capability、handle、memory、kernel-context | rlib |
| `lumio-job` | job | rlib |
| `lumio-spatial` | spatial | rlib |
| `lumio-timer` | timer | rlib |
| `lumio-codec` | codec | rlib，**experimental/private**（见 §4） |
| `lumio-diagnostics` | diagnostics | rlib，**experimental/private**（见 §4） |
| `lumio-test-support` | — | dev-only |

timer 的口径固定为一句话：**内核 `lumio-timer` 在 NativeCore；C ABI 插头在架构仓
`engine/native/modules/sdk-native/src/timer.rs`；经 `engine/abi/native-abi.json` 的 `timer_*` 槽到达托管侧。**

## 4. 模块状态标注

每个模块 README 头部保留两行：

```text
**RepositoryDeliveryPhase**：Foundation | NativeHeadless | Production Hardening
**ImplementationPriority**：I0 | I1 | I2                  ← 实施优先级，用 I 系避免与缺陷级 P0/P1 撞名
```

`codec`、`diagnostics` 仍是 feature-gated 私有原型（ADR 0005），默认不参与依赖图；
转正需要架构源先给出公共语义，本仓不自行发布。
