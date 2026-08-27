# KernelContext 生命周期契约（设计现状）

> 对应决策：[`0002`](../../.spec/decisions/0002-kernel-context-lifecycle-root.md)。
> 来源：架构 Review `ARCH-P0-002`；用户批准新增 `kernel-context` 模块（2026-08-27）。
> 状态：设计已定，公共 ABI 字段（ContextId 表示、错误码）待架构源发布后回填。

## 1. 定位

`kernel-context` 是 NativeCore 的**生命周期根**：一个 Context 代表一个 NativeCore 资源域，
统一拥有该域内所有跨调用资源，并唯一裁决关闭时序。它不是 World、Session 或 Host——
上层如何把 World/Session 映射到 Context 属跨仓 handoff 契约（Review OPEN-005，待上游）。

Context 统一拥有：

| 资源 | 说明 |
| --- | --- |
| Handle Arena namespace | 本 Context 的全部 Handle 槽位；跨 Context 使用被 `handle` 拒绝 |
| 内存预算与池 | `memory` 的有界 Allocator、临时批次、池配额 |
| Worker 集与队列 | `job` 的有界队列、Worker、Completion Queue |
| 索引与工作区 registry | `spatial` 索引、`codec` 字典/工作区等有状态资源 |
| 可选诊断 recorder | 非阻塞 record port；无 recorder 时零开销 |

每个资源在所有权矩阵中**只有一个 owner**；调用方持有的只是 opaque handle。

## 2. 状态机

```text
Creating -> Running -> Quiescing -> Closed
任一活动状态 -> Faulted -> (资源回收后) Closed
```

| 状态 | 允许的 API | 拒绝的 API（稳定错误） |
| --- | --- | --- |
| Creating | 无外部 API（创建调用内部） | 一切调用 → `ContextNotReady` |
| Running | 全部 | — |
| Quiescing | resolve 只读、结果消费、release、状态查询 | 新建资源 / submit → `ContextClosing` |
| Closed | 状态查询（幂等） | 其余一切 → `ContextClosed` |
| Faulted | 状态查询、证据导出 | 其余一切 → `ContextFaulted` |

- 状态迁移由 `kernel-context` 单点线性化；任何模块不得自行判断"Context 大概还活着"。
- `Creating` 中任一步失败 → 直接 `Faulted`，已获取资源按获取的逆序回收，不留半初始化 Context。

## 3. 关闭协议（固定顺序，不可重排）

`context_close` 幂等；首个调用者驱动以下序列，重复调用返回当前进度状态：

1. **拒绝新工作**：进入 `Quiescing`；新 submit / 新建资源立即返回 `ContextClosing`。
2. **失效新解析**：Handle Arena 标记 closing；新的写 resolve 拒绝，已借出的同步视图在调用返回时自然结束（借用不跨 FFI 调用，见 `handle` 契约）。
3. **请求取消**：对全部在途 Job 置 CancelRequested；不假设 Worker 立即停止。
4. **排空或放弃**：等待在途 Job 到达真实终态，超过 `close_deadline`（配置项）转入 Abandon——
   Job 标记 `Abandoned`，交后台 reaper 继续等待 Worker 到达可回收点；reaper 容量有界，超限即 `Faulted`。
5. **回收批次**：未消费的 Completion Batch、输入/输出租约按
   [Buffer 契约](ffi-buffer-ownership.md)的终态矩阵恰好回收一次。
6. **销毁资源**：索引/工作区 → 池/Allocator → Handle Arena，逆依赖顺序销毁。
7. **退休 Epoch**：ContextId 永久退休（见 §4），进入 `Closed`；分配器、Handle、Job 计数必须归零，
   或进入文档化的 retained-evidence 状态（Faulted 路径）。

关闭与并发操作的线性化裁决：

| 竞态 | 赢家 | 输家可见结果 |
| --- | --- | --- |
| close vs submit | 先到达 Context 状态锁者 | submit 输 → `ContextClosing`；close 输 → 该 Job 进入排空集合 |
| close vs resolve | 同上 | resolve 输 → `ContextClosing`；已返回的视图在本次调用内仍有效 |
| close vs complete | complete 总是允许落入 Completion Queue（排空期） | 未被消费的结果按终态矩阵回收，绝不写入调用方内存 |
| close vs close | 首个调用者驱动，其余观察进度 | 幂等返回 |

## 4. 标识与复用规则

- `ContextId`：进程内单调递增（u64），**永不回卷、永不复用**；跨 ABI 的公开表示待架构源冻结。
- Handle `Generation` 溢出：该槽位永久退休，不回收复用；Arena 容量有界，槽位耗尽返回容量错误。
- 迟到结果（World/Session 已亡）：Context 仍在 → 结果留在 Completion Queue 由上层丢弃或消费（上层策略）；
  Context 已 Closed → 结果在 reaper 中回收，绝不投递。NativeCore 只保证"不写入已销毁资源域"，
  不理解 World 语义。

## 5. Conformance Fixture（进入 Foundation 的验收面）

每条至少一个正向 + 一个失败样例；并发类用固定 interleaving + 随机压力双跑：

- close vs resolve / submit / complete 三组竞态，验证赢家表与错误码。
- 重复 close 幂等；close_deadline 触发 Abandon；reaper 超限进入 Faulted。
- Creating 半初始化失败 → Faulted，无泄漏（分配计数归零）。
- ContextId 不复用：关闭 N 个 Context 后新建，ID 严格递增。
- Generation 溢出槽位退休；容量耗尽错误可区分。
- Closed 后全部 API 返回稳定错误；计数归零或 retained-evidence 明示。
