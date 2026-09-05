# 0001 · abi 拆为 contract-types 叶子与 native-core-ffi 门面，NativeCore 不导出跨仓 Root 符号

- 日期:2026-08-27
- 状态:被 0009 取代

## 背景

架构 Review `ARCH-P0-001`：原 `abi` 模块同时是零依赖类型层和依赖 Error/Capability/Handle 的
Root API 门面，按文档建 crate 必成依赖环；且本仓与 CoreEngine 对 `lumio_core_get_api_v1`
的所有权表述冲突（Baseline §2.1/§2.3 已把统一 Root ABI 判给 CoreEngine）。

## 决策

用户裁决（2026-08-27）：跨仓 Root 符号由 CoreEngine `root-abi`/composition 层唯一拥有并导出；
NativeCore 只提供 provider 契约。本仓将 `abi` 拆为 `contract-types`（零依赖叶子，只消费架构源
生成物）与 `native-core-ffi`（顶层唯一导出面）两层；错误码/能力位以生成常量进叶子层，
`capability_bits` 只属于 API Table 与 Capability 快照，不复制进普通导出结构。
分层、依赖图与 crate 映射的完整设计见 [`native-core-module-map.md`](../../docs/specs/native-core-module-map.md)。

## 后果

- 跨仓归属的正式冻结依赖架构源 ADR-006/Baseline 修订（在途，另一会话执行中）；
  上游落地前本仓不冻结任何公共 Header 字段。
- 模块文档需要重写 `abi` README 并新增两层口径；根 README 依赖图需同步（任务卡在途）。
- NativeCore 发布产物做 symbol dump 断言，接受由此增加的 CI 成本。
