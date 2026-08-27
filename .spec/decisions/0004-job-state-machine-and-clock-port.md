# 0004 · Job 状态机 CAS 线性化 + 私有单调时钟 port，超时是观察不是终止

- 日期:2026-08-27
- 状态:生效

## 背景

架构 Review `ARCH-P0-004` 与 `ARCH-P1-008`：job 只有要求清单，无状态集合、竞态赢家与
Deadline 时钟域；本仓标准写拥有「时间与 Diagnostic Kernel」，与 Baseline RACI
（Wall Clock 归 Host、Tick 归 Runtime）冲突。

## 决策

状态集 `Created/Queued/Running/{Completed|Failed|Cancelled}/Reaped`（+ Abandoned 关闭态），
CancelRequested 是标志不是状态，`TimedOut` 是 Completion 记录上的观察结果；转移在槽位上
单点 CAS 线性化，竞态裁决表唯一赢家。NativeCore 只保留私有可注入 monotonic clock port，
跨 ABI 只收相对 duration，不拥有 Wall Clock/TickId；时钟读数只进 Diagnostics 不进权威 Hash。
Worker 只执行 Rust 内部闭包或架构源注册的 Typed Kernel，公共 ABI 不收任何回调。
完整契约见 [`job-state-machine.md`](../../docs/specs/job-state-machine.md)。

## 后果

- 超时不终止线程：长核必须有协作取消点并声明粒度预算，无取消点的核不能上 Job。
- `repository-architecture.md` 的「时间与 Diagnostic Kernel」表述需改为私有时钟 port 口径（任务卡在途）。
- 公开状态枚举与 operation ID registry 待架构源冻结，之前不发布公共 Header。
