# diagnostics

> 产生 Native Metrics、Trace Event 和 Failure Bundle 片段，供上层观测与故障重建使用。

**优先级**：P1  
**实施阶段**：NativeHeadless / Production Hardening  
**架构基线**：`LGE-V1.0-2026-08-27`

`diagnostics` 是本仓规划模块；共享事件字段和 Failure Bundle Schema 以架构源发布物为准，本文不复制字段清单。

## 负责范围

- 组装带 `ProductId`、`GameReleaseId`、`SessionId`、`WorldId`、`TickId`、`TraceId`（可用时）和 `EventSeq` 的 Native 事件。
- 输出批量 Metrics、Trace Event 和可校验的 Failure Bundle 片段。
- 提供采样、级别、队列容量和丢弃原因等结构化元数据。
- 让 ABI、Handle、Memory、Job、Spatial 和 Codec 可以报告本地状态，而不依赖外部 Sink。

## 不负责范围

- 不拥有文件/控制台/外部日志 Sink、轮转、保留、权限或脱敏策略的最终编排。
- 不替代 Audit Log、Txn Journal、Command Log 或 Snapshot 存储。
- 不决定 Session、World 或进程级故障处置，也不把诊断事件当作权威状态。

## 输入、输出与所有权

调用方提供上下文和批次 Buffer；诊断模块不得保存托管对象、回调地址或跨线程裸指针。事件内容应优先使用固定宽度字段和受限字节载荷，Failure Bundle 必须带长度、Hash/Checksum 和明确的拥有者。

## 依赖与约束

依赖 `abi` 和 `error` 的稳定承载结构；其他模块通过数据记录接入，而不是让核心计算路径等待诊断 Sink。外部日志生态只能通过 Adapter 接入，供应商类型不能进入 ABI。

## 线程、错误与观测

Diagnostic 队列有界，普通事件可按级别、类别和采样策略丢弃并记录原因；Error/Fatal 的应急落盘由上层 Sink 决定。事件必须保留 Producer 的 `EventSeq` 和 Tick 关联，但不承诺跨线程实时全局顺序。敏感字段在进入模块前由调用方按策略脱敏。

## 测试与性能

- 事件字段完整性、版本/长度校验、序列化、Hash/Checksum 和损坏 Bundle 拒绝。
- 队列满载、采样、并发生产、事件序号和 Failure Bundle 重放。
- 记录事件吞吐、队列等待、丢弃率、批次大小、分配和 Simulation Thread 额外延迟。

## 版本演进

共享字段、事件类别和 Failure Bundle Schema 的变更必须回到架构源；本地实现只能扩展非权威诊断字段并保持未知字段策略。改变采样、丢弃或同步落盘语义时要补故障 Fixture 和运行手册。

## 相关

- [ABI 模块](../abi/README.md)
- [错误模块](../error/README.md)
- [仓库边界与架构契约](../../.spec/knowledge/standards/repository-architecture.md)
- [根 README](../../README.md)
