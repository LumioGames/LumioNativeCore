# NativeCore 契约分层、模块依赖图与符号面（设计现状）

> 对应决策：[`0001`](../../.spec/decisions/0001-contract-layering-and-symbol-surface.md)、
> [`0005`](../../.spec/decisions/0005-codec-diagnostics-pending-and-dual-status.md)。
> 来源：架构 Review `ARCH-P0-001`、`ARCH-P1-006`；用户裁决 Root 归 CoreEngine、批准 abi 拆分与
> kernel-context 新增（2026-08-27）。跨仓 Root 归属的正式冻结在架构源 ADR-006/Baseline（在途）。

## 1. 契约分层：拆掉双重身份的 `abi`

原 `abi` 模块同时承担"最底层类型"与"Root API 门面"，不可实现。拆为两层：

| 层 | 职责 | 依赖 |
| --- | --- | --- |
| `contract-types`（叶子） | 消费架构源生成的固定宽度 POD、Buffer view、version scalar、opaque handle 表示、错误码/能力位常量 | **零依赖**（只依赖生成物） |
| `native-core-ffi`（门面） | NativeCore 唯一导出面：provider API Table 组装、panic 边界、ABI 入口 smoke | 依赖全部实现模块 |

- 错误类别、能力位以**生成契约常量**的形式进 `contract-types`，`error`/`capability` 模块
  实现行为逻辑——原文档「abi 的错误映射到 error 模块」的循环由此消解。
- **Root 符号归属**：`lumio_core_get_api_v1` 一类跨仓 Root 由 **CoreEngine `root-abi`/composition 唯一拥有并导出**；
  NativeCore 提供 provider API Table（源契约 + 组合入口），自身发布产物的符号表中
  **不出现跨仓 Root 符号**。每个导出结构携带 `struct_size`（必要时加结构版本字段）；
  `capability_bits` 只属于 API Table 与 Capability 快照，不机械复制进 payload/batch/error 结构。

## 2. 模块依赖图（编译期，无环）

```text
架构源发布物（只读）
        │
  contract-types（叶子）
        │
 ┌──────┼──────────┬─────────────┐
error  capability  handle       memory
                     │             │
                 context-token  buffer/lease 原语
                       \          /
                    （私有 clock port）
                          job
                           │
spatial ──> contract-types + error + handle + memory
codec   ──> contract-types + error + memory（有状态字典/工作区才加 handle）
diagnostics ──> contract-types + error（其余模块经可选 record port 接入，不编译期依赖其实现）

kernel-context ──> capability + handle + memory（定义 ContextResource port）
job / spatial / codec ──实现并注册──> kernel-context 的 ContextResource port
        （编译期方向是 job -> kernel-context；Context 经 port 拥有 Worker 集/索引/工作区，避免 crate 层环）
native-core-ffi ──> kernel-context + 各公开模块（唯一导出面）
```

禁止方向（lint 拒绝）：`spatial/codec -> job`、`error -> diagnostics`、`memory -> job/spatial/codec`、
核心模块 -> diagnostics 实现、NativeCore -> CoreEngine/任何领域仓、生成物反向依赖。

## 3. 文档模块 ↔ crate 映射（生产布局基线）

`modules/` 目录保持**文档分类**职责；编译边界按下表，后续调整须新 ADR：

| crate | 承载文档模块 | 类型 |
| --- | --- | --- |
| `lumio-contract-types` | abi（类型半边） | rlib，叶子 |
| `lumio-kernel` | error、capability、handle、memory、kernel-context | rlib |
| `lumio-job` | job | rlib |
| `lumio-spatial` | spatial | rlib |
| `lumio-codec` | codec | rlib，**experimental/private**（见 §4） |
| `lumio-diagnostics` | diagnostics | rlib，**experimental/private**（见 §4） |
| `lumio-native-ffi` | abi（门面半边） | **唯一 `cdylib`/`staticlib`**，唯一符号导出面 |
| `lumio-platform` | —（monotonic clock port 等） | rlib，private，不进稳定 ABI |
| `lumio-test-support` | — | dev-only |

- 只有 `lumio-native-ffi` 配置符号导出列表；CI 做 symbol dump 断言（除 provider 入口外无泄漏）。
- `modules/` 需随之新增 `kernel-context/README.md`，`abi/README.md` 改写为两层口径（任务卡在途）。

## 4. 模块双状态标注（Baseline 批准 ≠ 本仓排期）

每个模块 README 头部字段从单一「架构基线」扩展为三行：

```text
**BaselineStatus**：approved | pending | not-applicable   ← 只能由架构源 Baseline 更新
**RepositoryDeliveryPhase**：Foundation | NativeHeadless | Production Hardening
**ImplementationPriority**：I0 | I1 | I2                  ← 实施优先级，改用 I 系避免与缺陷级 P0/P1 撞名
```

当前判定：`contract-types/error/capability/handle/memory/job/kernel-context/native-core-ffi/spatial`
= approved（随上游 V1.1 模块地图落地确认）；`codec`、`diagnostics` = **pending**——
仓内允许 feature-gated 私有原型，**不进公共 Header/export list**，转 approved 只能由架构源批准驱动。
