# Job 状态机、取消线性化与时钟域契约（设计现状）

> 对应决策：[`0004`](../../.spec/decisions/0004-job-state-machine-and-clock-port.md)。
> 来源：架构 Review `ARCH-P0-004` 与 `ARCH-P1-008`。
> 状态：设计已定；公开状态枚举值、operation ID registry 待架构源冻结。

## 1. 状态集合与合法转移

```text
            submit
Created ────────────> Queued ──────> Running ──────> Completed
(内部瞬态)               │               │        └──> Failed
                        │               │        └──> Cancelled   （协作取消点生效）
                        │               └─ CancelRequested 标志（不新增状态，见 §2）
                        └────────────> Cancelled                  （出队前取消，立即终态）

任一终态（Completed / Failed / Cancelled）──consume 或回收──> Reaped
Context 关闭排空超时：任何非终态 Job ──> Abandoned（交 reaper 等真实终态后 Reaped）
```

- `TimedOut` 不是执行状态，是 **Completion 记录上的观察结果**（见 §3）；
  底层执行终态仍是 Cancelled/Completed/Failed 之一。
- 状态转移在 Job 槽位上单点 CAS 线性化；每个转移恰好发生一次。
- `Reaped` 后 Job Handle 失效（Generation 递增），租约释放（见 [Buffer 契约](ffi-buffer-ownership.md) §3）。

## 2. 竞态裁决表（唯一赢家 + 双方可见结果）

| 竞态 | 裁决 | API 可见结果 |
| --- | --- | --- |
| cancel vs 出队执行 | 谁先 CAS Queued 谁赢 | 取消赢 → 立即 `Cancelled`；执行赢 → cancel 变为置 CancelRequested |
| cancel vs complete | Running 中 Worker 到达终态点先 CAS 者赢 | complete 赢 → cancel 返回 `AlreadyTerminal`；cancel 生效 → 结果为 `Cancelled` |
| 重复 cancel | 首次置标志，其余幂等 | 均返回当前状态，不报错 |
| timeout vs complete | 见 §3——timeout 只是观察，complete 永远按实际终态记录 | Deadline 已过但已完成 → 记录 `Completed`（不伪造 TimedOut） |
| close vs submit | Context 状态锁先到者赢（见 [KernelContext 契约](kernel-context-lifecycle.md) §3） | submit 输 → `ContextClosing` |
| 结果丢失（消费边界后再查询） | Reaped 即终 | 查询返回 `JobReaped`，与 `UnknownJob` 可区分 |

## 3. Deadline 与时钟域

- **NativeCore 只拥有一个私有、可注入的单调时钟 port**（`monotonic-clock port`）：
  用于 Deadline 判定、队列等待计量与测试注入（fake clock）。
- 跨 ABI 只接受**相对 duration**；Wall Clock 归 Host，Logical Tick/Phase 归 Runtime——
  NativeCore 不读取、不换算、不存储日历时间与 TickId（诊断字段中的 TickId 由调用方传入，原样承载）。
- Deadline 语义：到期时置 CancelRequested 并在 Completion 记录标注 `deadline_exceeded`；
  **不终止线程**。Worker 在协作取消点检查标志；无取消点的长核必须声明最大粒度预算。
- 确定性边界：任何时钟读数、等待时长、完成时刻**只进 Diagnostics**，
  不进入权威结果与 State Hash；确定性 Kernel 的输出内容与 canonical 顺序在不同调度下必须逐位一致，
  Completion 消费顺序按 JobId canonical 排序提供（调用方可选按到达序，但该序不参与 Hash）。

## 4. 执行体边界

- Worker 只执行 **Rust 内部闭包或架构源注册的 Typed Kernel（operation ID + 版本化参数）**。
- 公共 ABI **不接受**调用方函数指针、managed delegate 或任何回调形式的执行体；
  「闭包」仅是 Rust 实现内部形态，不出现在 Header。
- Job 不编译期依赖 `spatial`/`codec`；两者作为 operation 经 registry 运行时绑定。

## 5. 关闭与排空

Worker 关闭只由 Context 关闭序列驱动：拒新 → 置取消 → 排空至 deadline → Abandon 交 reaper。
Worker 线程 join 发生在资源销毁步之前；join 超时 → `Faulted`（证据保留）。

## 6. Conformance Fixture

- §2 每行竞态：固定 interleaving（注入调度点）+ 随机并发双跑，验证唯一赢家与错误码。
- fake clock 注入：Deadline 到期前/后完成两分支；`deadline_exceeded` 标注正确且不影响权威结果。
- 取消协作点延迟释放：cancel 后租约仍在，Worker 到达取消点才 Reaped。
- 队列满载返回容量错误且不建立租约；关闭期间 submit 稳定拒绝。
- 确定性 Kernel 在 1/4/16 Worker 配置下输出逐位一致；等待时间只出现在 Diagnostics。
