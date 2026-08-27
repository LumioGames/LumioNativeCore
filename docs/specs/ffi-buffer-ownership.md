# FFI Buffer 所有权与异步租约契约（设计现状）

> 对应决策：[`0003`](../../.spec/decisions/0003-ffi-buffer-classes-and-leases.md)。
> 来源：架构 Review `ARCH-P0-003`；解决"创建侧释放"与"提交方转移所有权"的文本矛盾。
> 状态：设计已定，ABI 参数标注进 Header 的具体形式待架构源 Schema。

## 1. 核心原则（allocator provenance）

**谁分配、谁释放；跨边界只移动"释放权凭证"（handle），不移动裸内存所有权。**

- NativeCore **永不**对调用方分配的内存调用释放函数——它无法知道对方的 allocator。
- 调用方 **永不**释放 NativeCore 分配的内存，只能通过明确的 `release` API 归还 handle。
- 由此，"所有权转移进异步 Job"只能通过两条路径实现：提交时复制进 Native 内存，
  或调用方预先向 NativeCore 申请 Native Buffer、写入后随 submit 移交（零拷贝路径）。

## 2. Buffer 三分类（V1 全集）

| 类别 | 方向 | 生命期 | 释放方 | 允许进异步 Job |
| --- | --- | --- | --- | --- |
| `BorrowedCallBuffer` | 调用方 → Native | 仅本次同步调用内 | 调用方（调用返回后自由处置） | **禁止**；submit 时内容复制进 NativeOwned |
| `CallerOutputBuffer` | Native → 调用方 | 仅本次同步调用写入 | 调用方 | **禁止**；容量不足返回 `required_length`，不隐式扩容 |
| `NativeOwnedBufferHandle` | 双向 | 从分配到 `release`/终态回收 | NativeCore（经 release API 或 reaper） | **唯一合法路径**（输入与输出都是） |

`SharedReadOnlyBufferHandle`（引用计数 + 租约的共享只读批次）**V1 不做**：
需求出现时按新 ADR 设计，约束是必须有 Context 归属与显式引用计数，不得是任意 managed 指针。

## 3. 异步租约（lease）

- Job 从 **submit 成功**那一刻起持有其全部输入/输出 `NativeOwnedBufferHandle` 的租约。
- 租约只在 Job 到达**真实终态并被 reap**（结果被消费或被回收）时释放。
- **Cancel / Timeout 只改变可见状态，不证明 Worker 已停止**——租约在此期间继续持有，
  杜绝"调用方超时后释放、Worker 仍在写"的 UAF。
- Completion Batch 本身是 NativeOwned：在声明的消费边界（上层 Barrier 消费或 `release`）前有效；
  未消费的由 Context 关闭序列或 reaper 回收，配额有界。

## 4. 终态回收矩阵（每条路径恰好回收一次）

| Job 终态路径 | 输入 Buffer | 输出 Buffer | Completion 记录 |
| --- | --- | --- | --- |
| 正常完成，结果被消费 | reap 时释放 | 随 Completion 移交调用方读取，消费后 release | 消费即回收 |
| 提交被拒（队列满/校验失败） | 从未建立租约，submit 原子失败，调用方保留 handle | 未分配 | 无记录 |
| 取消于 Queued（未运行） | reap 时释放 | 未分配 | Cancelled 记录，消费或回收 |
| 取消于 Running（协作点生效） | Worker 到达取消点后终态，reap 释放 | 部分写入的输出整体废弃回收 | Cancelled 记录 |
| 超时（TimedOut 观察态） | 同"取消于 Running"——租约持续到真实终态 | 同左 | TimedOut 记录 |
| Worker fault（panic 转换） | reap 释放 | 废弃回收 | Failed 记录 + fault code |
| Context close 排空内完成 | reap 释放 | 未消费则回收 | 回收 |
| Context close 转 Abandon | reaper 等到真实终态后释放 | reaper 回收 | reaper 回收 |

验收：故障注入在上表每一行触发，结束时 native 分配数、租约数、handle 数与期望一致；
FFI smoke 验证跨 allocator 边界零交叉释放（Rust 侧分配 ↔ C/managed 侧分配互不释放）。

## 5. ABI 参数标注（进入 Header 的元数据）

每个 ABI 参数必须标注：`borrowed / owned-in(native handle) / out / transferred`、
allocator provenance（caller / native）、对齐、`valid-until`（call-return / consume-boundary / release）、
对应 release API。标注进入版本化 Header 与绑定生成输入，具体载体待架构源 Schema 冻结。

## 6. 对既有文档的修正口径

- 根 README「内存由创建侧释放」保留，但补充口径：**"创建侧"按 provenance 判定，不按调用方向**。
- `job` README「提交方明确转移输入批次所有权」改为
  「提交方移交 `NativeOwnedBufferHandle`（其内存本就由 NativeCore 拥有），或由 submit 复制借用字节」。
- `memory` README「异步 Job 只能接收明确转移所有权的批次」改为引用本契约的三分类。
