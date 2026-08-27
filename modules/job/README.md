# job

> 提供有界 Worker、Typed Job、取消、超时和 Completion Batch 的 Native 调度原语。

**优先级**：P0  
**实施阶段**：Foundation  
**架构基线**：`LGE-V1.0-2026-08-27`

Job 结果由上层在规定 Barrier 消费；Native Worker 不回调托管 Hot Gameplay。

## 负责范围

- 管理有界队列、Worker 生命周期和 Typed Job 状态。
- 支持提交、取消、Deadline/Timeout、完成批次和结果查询。
- 明确队列满载、取消竞态、超时和 Worker 故障的结果。
- 为 `spatial`、`codec` 等纯 Kernel 提供可选调度承载。

## 不负责范围

- 不定义 Tick Phase、Processor 依赖、Session 调度或业务重试。
- 不调用 C#、保存托管 Delegate、跨 FFI 持有 Rust 锁或写入 World。
- 不把无界线程、无界队列或隐式后台任务作为默认策略。

## 输入、输出与所有权

提交方明确转移 Job 输入批次的所有权，并通过不透明 Job Handle 查询结果。Completion Batch 在声明的消费边界前保持有效；取消或超时后不得自动写入已销毁的 World。队列满载应返回稳定错误和可观测的容量信息。

## 依赖与约束

依赖 `abi`、`handle`、`error` 和 `memory`。Job 不强制依赖具体 Kernel；调用方可在 `NativeJobBarrier` 或之后应用结果。线程亲和、重入性、最大并发和资源预算必须写进契约。

## 线程、错误与观测

Worker 只执行 Native 闭包或 Typed Kernel，不执行未知回调。状态转换必须可线性化；重复取消、超时后完成、结果丢失和 Worker 关闭都要可区分。队列指标、耗时和取消原因以批量 Diagnostic Event 输出。

## 测试与性能

- 提交、完成、取消、超时、队列满载和关闭期间提交。
- 取消与完成竞态、Worker 故障、结果批次顺序和资源释放。
- 固定 Worker/队列配置测量吞吐、等待 p95/p99、峰值内存和取消延迟。

## 版本演进

改变 Job 状态机、结果可见时机、取消/超时语义或线程亲和性必须经过 ABI/契约变更流程。Worker 实现、队列算法和线程数可优化，但不能破坏有界和 Barrier 约束。

## 相关

- [ABI 模块](../abi/README.md)
- [Handle 模块](../handle/README.md)
- [Memory 模块](../memory/README.md)
- [根 README](../../README.md)
