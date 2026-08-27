# abi（contract-types + native-core-ffi 两层）

> 原单一 `abi` 模块按 ADR 0001 拆为两层：`contract-types` 零依赖类型叶子与
> `native-core-ffi` 唯一导出门面。本文档同时描述两层的本地边界。

**BaselineStatus**：approved（`LGE-V1.4` §16 模块地图）  
**RepositoryDeliveryPhase**：Architecture Gate / Foundation  
**ImplementationPriority**：I0  
**架构基线**：`LGE-V1.4-2026-08-27`

公共 ABI Schema、结构字段和 Fixture 的规范来源是 `LumioGameEngineArchitecture`；本文只描述本地模块边界。分层与 crate 映射见 [`native-core-module-map.md`](../../docs/specs/native-core-module-map.md)。

## contract-types（叶子）

- 承载架构源生成的固定宽度 POD、布尔/枚举宽度、字节布局约束与 Buffer view。
- 承载版本标量、opaque handle 表示与生成的错误码/能力位**常量**（行为逻辑归 `error`/`capability`）。
- **零依赖**：不依赖任何其他 NativeCore 模块，只消费架构源生成物。

## native-core-ffi（门面）

- NativeCore 唯一符号导出面：组装 provider API Table、实现 panic 边界与入口 smoke。
- **不导出跨仓 Root 符号**：`lumio_core_get_api_v1` 一类 Root 由 CoreEngine `root-abi`/composition
  唯一拥有并导出（Baseline v1.1 §8.1）；本仓产物的符号表由 `cargo xtask dump-symbols` 断言。
- 依赖全部公开实现模块；pending 模块（codec/diagnostics）只能经 experimental feature 进入本地实验构建。

## 不负责范围

- 不维护公共 Error Code、Capability Bit、ID Registry 或领域 Schema。
- 不生成最终 C# Binding，不传 Rust/C# 容器、对象引用或异常。
- 不决定 Host、World、Session、网络或持久化格式。

## 输入、输出与所有权

调用方提供请求结构和可选输出 Buffer；释放方按 allocator provenance 判定（谁分配谁释放，
见 [`ffi-buffer-ownership.md`](../../docs/specs/ffi-buffer-ownership.md)）。Buffer 不足返回所需长度，
不隐式扩容。ABI 入口不得跨调用保存调用方裸指针；异步结果必须转为版本化批次后由调用方消费。

## 版本化结构约定

导出结构以 `struct_size` 保护尾部扩展（必要时加独立结构版本字段）；`capability_bits`
只出现在 Root API Table 与 Capability 快照中，不复制进 payload/batch/error 结构。

## 线程、错误与观测

布局描述和版本查询应保持无状态、可重入且不阻塞。未知版本、结构过短、结构过长策略和
Buffer 不足必须映射到 `error` 的稳定类别；panic 不得穿过导出边界。模块只提供诊断字段承载，
不拥有日志 Sink。

## 测试与性能

- C Header 编译、结构大小、对齐、字段偏移和字节序检查。
- provider 表版本协商、短结构、未知尾字段和 Buffer 不足 smoke test。
- 符号导出断言（无跨仓 Root、无 lumio_* 泄漏）；记录 ABI 调用批次、复制字节和布局检查耗时。

## 版本演进

可向后兼容的尾部扩展必须由 `struct_size` 保护；破坏布局、宽度或所有权语义时提升 ABI 主版本。
模块 README 不记录具体字段数值，具体数值以架构源生成物为准。

## 相关

- [契约分层与符号面决策（ADR 0001）](../../.spec/decisions/0001-contract-layering-and-symbol-surface.md)
- [仓库边界与架构契约](../../.spec/knowledge/standards/repository-architecture.md)
- [根 README](../../README.md)
- [架构镜像](../../docs/architecture/LumioGameEngine_Architecture_v1.4.md)
