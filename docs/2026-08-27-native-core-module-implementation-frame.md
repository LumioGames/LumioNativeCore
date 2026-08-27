# LumioNativeCore 模块实现框架设计

> 仓库目标路径：`docs/2026-08-27-native-core-module-implementation-frame.md`
> 架构基线：`LGE-V1.4-2026-08-27`
> 文档性质：实现级框架设计与可执行任务；不包含生产实现，不发布新公共 ABI。
> 日期：2026-08-27

本文只在冻结框架上细化实现。公共 ABI、Capability、Error Schema、ID Registry、Fixture 仍以 `LumioGameEngineArchitecture` 发布物为唯一来源。未发布的公共字段、数值、Operation ID、`ContextId` 跨 ABI 表示与 Header 布局统一标记 `BLOCKED_ABI`，不得在本仓手写第二套 Schema。`codec`、`diagnostics` 只允许 feature-gated 私有原型。

---

# 0. 总览与实现节奏

**一句话：** Cargo workspace、crate 映射、依赖 Gate、Context/Buffer/Job 规范和 ADR 已冻结；实现从无第三方的 Error/Clock/Capability/Handle/Memory/Context 基础开始，随后接有界 Job，再做 NativeHeadless Spatial，最后完成 FFI hardening 与受控私有原型。

## 0.1 阶段

- Architecture Gate 残留：绑定架构源生成物，关闭公共数值、布局、ID 与 Fixture 缺口；第三方依赖逐项准入。
- Foundation (I0)：contract-types、platform、error、capability、handle、memory、kernel-context、Job 单线程/有界闭环。
- NativeHeadless (I1)：多 Worker Job、spatial、C/Rust headless conformance、故障与确定性验证。
- Production Hardening：panic/符号/ABI/供应链/内存/并发/性能 hardening；codec/diagnostics 仍为私有 feature。
- I2：经架构源批准后的更多算法、平台与公共契约；本文不提前冻结。

## 0.2 模块矩阵

| 模块 | crate | BaselineStatus | 阶段 | 本轮实现 | 阻塞 |
|---|---|---|---|---|---|
| contract-types | `lumio-contract-types` | crate 映射冻结；生成值待绑定 | Gate→I0 | 生成物 adapter/layout/registry | ABI-001..006 |
| error | `lumio-kernel::error` | 边界冻结；数值外部 | I0 | 内部错误模型与唯一映射出口 | ABI-001 |
| capability | `lumio-kernel::capability` | 三层模型冻结；bits 外部 | I0 | Static/Configured/Runtime | ABI-002 |
| handle | `lumio-kernel::handle` | 语义冻结 | I0 | 有界代际槽位、Context 隔离 | ABI-003/005 仅影响 FFI |
| memory | `lumio-kernel::memory` | Buffer 三类冻结 | I0 | 预算、provenance、Native buffer | ABI-006 仅影响 Header |
| kernel-context | `lumio-kernel::context` | 七步关闭冻结 | I0 | Context owner、resource registry | ABI-005 仅影响 FFI |
| platform | `lumio-platform` | 私有实现 | I0 | 单调时钟 port | 无 |
| job | `lumio-job` | 状态机冻结 | I0→I1 | 有界队列、Typed Kernel、取消/超时 | ABI-004 |
| spatial | `lumio-spatial` | 领域无关边界冻结 | I1 | AABB/index/batch/determinism | Operation ID |
| codec | `lumio-codec` | ADR 0005 pending | Hardening prototype | 私有 feature | ABI-007 |
| diagnostics | `lumio-diagnostics` | ADR 0005 pending | Hardening prototype | 私有 feature | ABI-008 |
| native-core-ffi | `lumio-native-ffi` | 唯一导出 crate 冻结 | I0 smoke→Hardening | guard/validation/generated exports | ABI-001..006 |
| test-support | `lumio-test-support` | dev-only 冻结 | Gate→Hardening | FakeClock/interleaving/leak/fault | Fixture corpus |

## 0.3 不可越过的 Gate

- `lumio-kernel` 不得依赖 `lumio-job` 或 `lumio-diagnostics`。
- `spatial`、`codec` 不得依赖 `job`；Operation 组合放在 `lumio-native-ffi::operations` 或未来 CoreEngine composition。
- 只有 `lumio-native-ffi` 可配置 `cdylib/staticlib`；本仓不得导出 `lumio_core_get_api_v1`。
- 第三方类型不得进入稳定 Rust port、C Header 或跨仓契约。
- Worker 只执行 Rust 内部实现或 Registry 中的 Typed Kernel，不接收托管函数指针、delegate 或回调。
- 所有外部 crate 在改 Cargo allowlist 前必须完成许可证、维护、漏洞、平台、静态链接/AOT 和退出路径审查。

---

# 1. 全局骨架（先于模块）

## 1.1 内部类型与 Port

### 1.1.1 `MonotonicClock`（`lumio-platform::clock`）

```rust
use core::time::Duration;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Ticks(u64);
impl Ticks {
    pub const ZERO: Self = Self(0);
    pub const fn from_nanos(v: u64) -> Self { Self(v) }
    pub const fn as_nanos(self) -> u64 { self.0 }
    pub fn checked_add(self, d: Duration) -> Option<Self>;
    pub fn saturating_duration_since(self, earlier: Self) -> Duration;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Deadline(Option<Ticks>);
impl Deadline {
    pub const NONE: Self = Self(None);
    pub const fn at(t: Ticks) -> Self { Self(Some(t)) }
    pub fn is_expired(self, now: Ticks) -> bool;
}

pub trait MonotonicClock: Send + Sync + 'static { fn now(&self) -> Ticks; }
pub struct StdMonotonicClock { epoch: std::time::Instant }
impl StdMonotonicClock { pub fn new() -> Self; }
impl MonotonicClock for StdMonotonicClock { fn now(&self) -> Ticks; }
```
不变式：同一 clock 的 `now()` 非递减；`Ticks` 不表示 Wall Clock/TickId，不跨进程比较，不进入权威 Hash。测试实现 `FakeClock` 位于 `lumio-test-support`。

### 1.1.2 `ContextResource` 与七步关闭

```rust
use lumio_platform::Deadline;
use crate::error::KernelResult;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CancelReason { ContextClosing, ContextFaulted, OwnerRequested }
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum QuiesceState { Quiesced, Pending { remaining: u32 } }
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct QuiesceReport { pub state: QuiesceState }

pub trait ContextResource: Send + Sync + 'static {
    fn name(&self) -> &'static str;
    fn cancel_requested(&self, reason: CancelReason);
    fn quiesce(&self, deadline: Deadline) -> KernelResult<QuiesceReport>;
    fn destroy(&self) -> KernelResult<()>;
}
```
关闭由 `KernelContext` 唯一驱动：拒绝新工作 → 广播取消 → quiesce → 等待 in-flight/lease → 逆序 destroy → 退休 Context 下 Handle/Native Buffer → 发布 terminal。合法状态和竞态赢家直接服从 `kernel-context-lifecycle.md`，不得另建旁路。

### 1.1.3 `RecordPort`（定义在 kernel，diagnostics 实现）

```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RecordLevel { Trace, Debug, Info, Warn, Error }
#[derive(Clone, Copy, Debug)]
pub struct KernelRecordRef<'a> {
    pub kind: &'static str,
    pub level: RecordLevel,
    pub context: Option<crate::handle::ContextKey>,
    pub fields: &'a [RecordFieldRef<'a>],
}
#[derive(Clone, Copy, Debug)]
pub struct RecordFieldRef<'a> { pub key: &'static str, pub value: RecordValueRef<'a> }
#[derive(Clone, Copy, Debug)]
pub enum RecordValueRef<'a> { U64(u64), I64(i64), Bool(bool), Bytes(&'a [u8]), Str(&'a str) }
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RecordDisposition { Accepted, DroppedFull, Disabled }
pub trait RecordPort: Send + Sync + 'static {
    fn try_record(&self, record: KernelRecordRef<'_>) -> RecordDisposition;
}
pub struct NoopRecordPort;
```
核心路径只允许 non-blocking `try_record`；无 recorder 使用静态 Noop，不分配、不格式化、不等待 Sink。`lumio-kernel` 不编译依赖 diagnostics。

### 1.1.4 Buffer 三分类

```rust
pub struct BorrowedCallBuffer<'a>(&'a [u8]);
pub struct CallerOutputBuffer<'a> { bytes: &'a mut [u8], written: usize }
pub enum NativeBufferTag {}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct NativeOwnedBufferHandle(crate::handle::Handle<NativeBufferTag>);

impl<'a> BorrowedCallBuffer<'a> {
    pub fn new(bytes: &'a [u8]) -> Self;
    pub fn as_slice(&self) -> &'a [u8];
}
impl<'a> CallerOutputBuffer<'a> {
    pub fn new(bytes: &'a mut [u8]) -> Self;
    pub fn capacity(&self) -> usize;
    pub fn written(&self) -> usize;
    pub fn write_all(&mut self, src: &[u8]) -> crate::error::KernelResult<()>;
    pub fn finish(self) -> &'a mut [u8];
}
```
`BorrowedCallBuffer` 不跨调用保存；`CallerOutputBuffer` 由调用方分配且不足时返回 required length；`NativeOwnedBufferHandle` 由创建侧 allocator 回收。V1 不做 `SharedReadOnlyBufferHandle`。

### 1.1.5 Handle 内部视图与 ABI opaque 值

```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)] pub struct ContextKey(u64);
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)] pub struct SlotIndex(u32);
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)] pub struct Generation(u32);
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct HandleKey { pub context: ContextKey, pub slot: SlotIndex, pub generation: Generation }
pub struct Handle<T> { key: HandleKey, _tag: core::marker::PhantomData<fn() -> T> }
impl<T> Handle<T> { pub(crate) const fn from_key(k: HandleKey) -> Self; pub const fn key(self) -> HandleKey; }

// BLOCKED_ABI: opaque value width/layout and ContextId representation.
#[cfg(feature = "architecture-contracts")]
pub fn decode_abi_handle<T>(raw: lumio_contract_types::AbiOpaqueHandle, expected: ContextKey)
    -> crate::error::KernelResult<Handle<T>>;
```
跨 Context 拒绝；generation 溢出永久退休，不回绕复用；opaque 编解码只在 FFI facade。

### 1.1.6 Error、Capability、Operation 与 allocator provenance

```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ErrorCategory {
    InvalidArgument, InvalidHandle, WrongContext, AlreadyReleased, BufferTooSmall,
    CapacityExceeded, CapabilityUnavailable, Cancelled, TimedOut,
    ContextClosing, ContextDestroyed, PanicBoundary, InternalInvariant,
}
pub struct KernelError { category: ErrorCategory, detail: ErrorDetail }
pub type KernelResult<T> = Result<T, KernelError>;

pub struct StaticCapabilities { enabled: Box<[CapabilityKey]> }
pub struct ConfiguredLimits {
    pub max_handles: u32, pub max_native_bytes: u64, pub max_jobs_queued: u32,
    pub max_jobs_running: u32, pub max_completion_items: u32,
}
pub struct RuntimeStatus {
    pub accepting_work: bool, pub queued_jobs: u32, pub running_jobs: u32,
    pub allocated_native_bytes: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct OperationId(u32);
pub trait TypedKernel: Send + Sync + 'static {
    fn operation_id(&self) -> OperationId;
    fn execute(&self, request: KernelRequest<'_>, output: &mut CallerOutputBuffer<'_>,
               cancel: &CancellationView) -> KernelResult<KernelExecutionResult>;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)] pub struct AllocatorId(u32);
pub enum AllocationClass { CallScratch, NativeOwnedBuffer, HandlePayload, JobPayload,
    SpatialIndex, CodecWorkspace, DiagnosticsQueue }
pub struct AllocationProvenance {
    pub allocator: AllocatorId, pub context: ContextKey, pub class: AllocationClass,
    pub requested_bytes: u64, pub charged_bytes: u64,
}
```
Error 数值只由 `error/mapping.rs` 映射生成常量；Capability 分 Static/Configured/Runtime 三层；Operation 数值来自架构 Registry。`spatial`/`codec` 不依赖 job，薄 `TypedKernel` adapter 放在 FFI/composition。所有 allocation 可追溯 allocator、Context、class 与 charged bytes。

## 1.2 推荐文件树

| crate | `.rs` 文件 | 职责 |
|---|---|---|
| `lumio-contract-types` | `generated.rs / layout.rs / registry.rs` | 生成物绑定、布局断言、只读 Registry |
| `lumio-platform` | `clock.rs` | Ticks/Deadline/MonotonicClock/std adapter |
| `lumio-kernel` | `record.rs; error/{mod,category,detail,mapping}.rs; capability/{mod,static_set,limits,runtime}.rs; handle/{mod,key,arena,registry,guard}.rs; memory/{mod,buffer,budget,provenance,native_buffers,call_scratch}.rs; context/{mod,config,state,resource,registry,lifecycle}.rs` | Foundation primitives 与 Context owner |
| `lumio-job` | `id.rs / state.rs / cancel.rs / request.rs / operation.rs / queue.rs / scheduler.rs / worker.rs / completion.rs / resource.rs` | 有界 Typed Job |
| `lumio-spatial` | `types.rs / validation.rs / query.rs / index/{mod,rstar_adapter,grid_reference}.rs / resource.rs` | 领域无关 spatial |
| `lumio-codec` | `bounds.rs / options.rs / compression/{mod,zstd_adapter,lz4_adapter}.rs / checksum.rs / diff.rs / resource.rs` | 私有 codec prototype |
| `lumio-diagnostics` | `record.rs / queue.rs / recorder.rs / drain.rs / tracing_adapter.rs / resource.rs` | 私有 recorder prototype |
| `lumio-native-ffi` | `boundary.rs / validation.rs / handles.rs / buffers.rs / error.rs / exports.rs / operations/{mod,spatial,codec}.rs / symbol_guard.rs` | 唯一 C facade |
| `lumio-test-support` | `clock.rs / interleaving.rs / leak.rs / fault.rs / fixtures.rs / panic.rs` | dev-only helpers |

所有现有空 `lib.rs` 只做模块声明和经审查的 re-export。禁止 `common.rs`、`utils.rs`、`globals.rs`、`everything.rs` 或隐藏运行时 event bus。

## 1.3 外部依赖与 Adapter 总表

| 能力/候选 | 裁决 | 许可证门 | Adapter | 理由与退出路径 |
|---|---|---|---|---|
| `slotmap` / `generational-arena` | Handle V1 不采用 | MIT/Apache-2.0 或 MIT | 无 | Context 与永久退休语义不足；自研最小 bounded arena，并保留 backend seam |
| `bumpalo` | 仅 CallScratch 候选 | MIT/Apache-2.0 | `memory/call_scratch.rs` | 调用期整体释放；可回退 Vec scratch |
| 系统 allocator + budget wrapper | 采用 | Rust std | `memory/budget.rs` | 标准行为 + 本仓计费/provenance；backend 可替换 |
| `crossbeam-channel` | Job 候选采用 | MIT/Apache-2.0 | `lumio-job/queue.rs` | bounded MPMC；可退 std sync_channel |
| `rayon` / `tokio` | 排除 Kernel scheduler | MIT/Apache-2.0 或 MIT | 无 | 默认池/runtime 不匹配 Context 有界 Typed Job |
| `rstar` | Spatial I1 候选采用 | MIT/Apache-2.0 | `index/rstar_adapter.rs` | 成熟 R-tree；结果稳定二次排序；可切 reference grid/BVH |
| `kiddo` | 首期不采用 | MIT/Apache-2.0 | 无 | 更偏 point nearest，非动态 AABB 首选 |
| `parry3d` | I2 候选 | Apache-2.0 | 未来 adapter | 依赖面较大；碰撞 response 不属本模块 |
| `zstd` / `lz4_flex` | 私有 codec 候选 | BSD-3-Clause / MIT | compression adapters | 有界解压；公共算法 ID 待架构源；adapter 可替换 |
| `xxhash-rust` / `blake3` | 私有 checksum 候选 | Boost-1.0 / CC0+Apache-2.0 | `checksum.rs` | 安全语义分开；公共格式待批准 |
| `tracing` | 仅 diagnostics bridge | MIT | `tracing_adapter.rs` | 核心不依赖 subscriber/Sink；删除 bridge 不影响 RecordPort |
| `metrics` | 首期不采用 | MIT | 无 | global recorder/labels 扩大稳定面 |
| `crossbeam-queue` | 私有 recorder 候选 | MIT/Apache-2.0 | `diagnostics/queue.rs` | ArrayQueue bounded non-blocking；可退 mutex ring |
| `loom` / `proptest` / `criterion` | dev-only 采用 | MIT / MIT+Apache-2.0 | tests/benches | 模型并发、property、统计 benchmark；生产零影响 |

版本策略：manifest 只允许受控 minor range，`Cargo.lock` 与供应链 manifest 锁 exact version/commit；升级重跑 license、RustSec、Windows/macOS/Linux build、静态链接/AOT、public type leak 和 benchmark。

---

# 2.1 `contract-types`

## A. 一句话定位 + 边界

**定位：** 只消费并薄封装架构源生成的固定宽度 ABI 类型、Error/Capability/Operation/ID Registry 与布局元数据；不定义新数值，不拥有 Root API，不含运行时状态。

**输入 / 输出 / 所有权：** 输入是架构源生成 package/header manifest；输出是受控 Rust re-export、layout 与 registry 查询。架构源是契约 owner，本 crate 只是只读消费者；无 Buffer/Handle 生命周期。

**线程模型：** 纯值类型与静态表；可重入、无锁、无回调。

## B. 内部子模块怎么切

| 单元 | 文件 | 依赖 | 可见性 | 切分理由 |
|---|---|---|---|---|
| generated adapter | `crates/lumio-contract-types/src/generated.rs` | 无 | crate 内；受控 re-export | 生成路径变更只改一处 |
| layout | `.../layout.rs` | generated | crate 外验证函数 | 布局验证独立 |
| registry | `.../registry.rs` | generated | crate 外只读 | 映射不散落 |
| facade | `.../lib.rs` | 前三者 | crate 外最小 | 防止全量泄漏 |

## C. 代码面

```rust
mod generated;
pub mod layout;
pub mod registry;

pub use generated::{AbiVersion, ArchitectureErrorCode, ArchitectureOperationId,
                    CapabilityBits, StructSize};
// BLOCKED_ABI: exact generated names/layout.
#[cfg(feature = "architecture-contracts")]
pub use generated::{AbiOpaqueHandle, AbiReadBuffer, AbiWriteBuffer};

pub fn architecture_baseline_id() -> &'static str;
pub fn verify_generated_contract_revision() -> Result<(), ContractMismatch>;
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContractMismatch { pub expected: &'static str, pub found: &'static str }
```

**关键不变式：**

- 源码中不存在手写 error/capability/operation 数值。
- 零 workspace/第三方依赖叶子。
- `generated` 私有，只 re-export NativeCore 必需项。
- layout test 以架构 manifest 为期望，不从本仓反向生成契约。

**失败与稳定类别：** Contract revision/layout mismatch；在 FFI 初始化映射为架构源规定类别，数值待生成物。

**相邻模块接口：** Consumes：架构生成物。Produces：生成类型的最小 Rust 面、`verify_generated_contract_revision`、layout/registry queries。

## D. 成熟方案选型

| 候选 | 许可证 | 满足 | 不满足 | 裁决 |
|---|---|---|---|---|
| 架构源生成 crate | 项目契约 | 唯一权威、可锁版本 | 发布前会阻塞 | 采用 |
| bindgen 现场生成 | BSD-3-Clause | 可解析 Header | 环境漂移、方向错误 | 不采用默认流程 |
| 手写 `repr(C)` | 无 | 立即可编译 | 双重 Schema | 禁止 |

V1 生产路径不新增第三方依赖；标准库、生成物和小型 typed model 已足够，dev dependency 不进入生产 graph。

## E. 测试与 Fixture

- **unit：** `generated_struct_sizes_match_manifest`、`all_registry_values_are_unique`。
- **concurrency：** 不适用：纯只读。
- **fault：** `wrong_baseline_is_rejected`。
- **bench：** 不适用。
- **共同要求：** 固定 baseline/build/target/seed；可由 FakeClock/Interleaving 表达的竞态不得只依赖 `sleep`；bench 报告 throughput、p50/p95/p99/max、alloc、peak bytes、queue depth，时钟读数不进入权威 Hash。

## F. 本模块任务列表

### T-contract-types-01: 建立生成契约 adapter

**Files:**
- Create: `crates/lumio-contract-types/src/generated.rs`
- Modify: `crates/lumio-contract-types/src/lib.rs`
- Test: `crates/lumio-contract-types/tests/generated_contract_revision_is_readable.rs`

**Consumes:** 架构源生成 package
**Produces:** 受控 re-export 与 `architecture_baseline_id`
**成熟方案:** 唯一采用架构生成物

**步骤（TDD）:**
1. 先写失败测试 `generated_contract_revision_is_readable`，固定输入、预期状态/错误类别与 owner/handle/allocator 计数。
2. 写只覆盖本任务的最小实现；不得扩大稳定 API、写公共数值或绕过 Adapter。
3. 运行 `cargo test -p lumio-contract-types generated_contract_revision_is_readable`、`cargo xtask check-dep-dag`；期望测试和依赖 Gate 同时通过。

**验收:**
- [ ] `crates/lumio-contract-types/src/generated.rs` 存在，`lib.rs` 只增加必要声明/re-export。
- [ ] `generated_contract_revision_is_readable` 在实现前失败、实现后可重复通过，不靠 wall-clock sleep 碰运气。
- [ ] `受控 re-export 与 `architecture_baseline_id`` 不泄漏第三方类型或未批准公共数值。
- [ ] `cargo fmt --check`、目标 crate tests、`xtask check-dep-dag` 通过。

**依赖:** 无
**Blocked:** 架构源 package 名/路径

### T-contract-types-02: 建立 Registry 只读适配

**Files:**
- Create: `crates/lumio-contract-types/src/registry.rs`
- Modify: `crates/lumio-contract-types/src/lib.rs`
- Test: `crates/lumio-contract-types/tests/registry_values_are_unique.rs`

**Consumes:** T-contract-types-01
**Produces:** Error/Capability/Operation registry 查询
**成熟方案:** 生成 Registry

**步骤（TDD）:**
1. 先写失败测试 `registry_values_are_unique`，固定输入、预期状态/错误类别与 owner/handle/allocator 计数。
2. 写只覆盖本任务的最小实现；不得扩大稳定 API、写公共数值或绕过 Adapter。
3. 运行 `cargo test -p lumio-contract-types registry_values_are_unique`、`cargo xtask check-dep-dag`；期望测试和依赖 Gate 同时通过。

**验收:**
- [ ] `crates/lumio-contract-types/src/registry.rs` 存在，`lib.rs` 只增加必要声明/re-export。
- [ ] `registry_values_are_unique` 在实现前失败、实现后可重复通过，不靠 wall-clock sleep 碰运气。
- [ ] `Error/Capability/Operation registry 查询` 不泄漏第三方类型或未批准公共数值。
- [ ] `cargo fmt --check`、目标 crate tests、`xtask check-dep-dag` 通过。

**依赖:** T-contract-types-01
**Blocked:** 公共 Registry 生成物

### T-contract-types-03: 建立 ABI layout 断言

**Files:**
- Create: `crates/lumio-contract-types/src/layout.rs`
- Modify: `crates/lumio-contract-types/src/lib.rs`
- Test: `crates/lumio-contract-types/tests/generated_layout_matches_manifest.rs`

**Consumes:** T-contract-types-01
**Produces:** `verify_layout()`
**成熟方案:** 生成 manifest

**步骤（TDD）:**
1. 先写失败测试 `generated_layout_matches_manifest`，固定输入、预期状态/错误类别与 owner/handle/allocator 计数。
2. 写只覆盖本任务的最小实现；不得扩大稳定 API、写公共数值或绕过 Adapter。
3. 运行 `cargo test -p lumio-contract-types generated_layout_matches_manifest`、`cargo xtask check-dep-dag`；期望测试和依赖 Gate 同时通过。

**验收:**
- [ ] `crates/lumio-contract-types/src/layout.rs` 存在，`lib.rs` 只增加必要声明/re-export。
- [ ] `generated_layout_matches_manifest` 在实现前失败、实现后可重复通过，不靠 wall-clock sleep 碰运气。
- [ ] ``verify_layout()`` 不泄漏第三方类型或未批准公共数值。
- [ ] `cargo fmt --check`、目标 crate tests、`xtask check-dep-dag` 通过。

**依赖:** T-contract-types-01
**Blocked:** Header/layout manifest

### T-contract-types-04: 建立契约漂移 Gate

**Files:**
- Create: `crates/lumio-contract-types/src/lib.rs`
- Modify: `crates/lumio-contract-types/src/lib.rs`
- Test: `crates/lumio-contract-types/tests/wrong_baseline_is_rejected.rs`

**Consumes:** T-contract-types-02/03
**Produces:** `verify_generated_contract_revision()`
**成熟方案:** 本章 D

**步骤（TDD）:**
1. 先写失败测试 `wrong_baseline_is_rejected`，固定输入、预期状态/错误类别与 owner/handle/allocator 计数。
2. 写只覆盖本任务的最小实现；不得扩大稳定 API、写公共数值或绕过 Adapter。
3. 运行 `cargo test -p lumio-contract-types wrong_baseline_is_rejected`、`cargo xtask check-dep-dag`；期望测试和依赖 Gate 同时通过。

**验收:**
- [ ] `crates/lumio-contract-types/src/lib.rs` 存在，`lib.rs` 只增加必要声明/re-export。
- [ ] `wrong_baseline_is_rejected` 在实现前失败、实现后可重复通过，不靠 wall-clock sleep 碰运气。
- [ ] ``verify_generated_contract_revision()`` 不泄漏第三方类型或未批准公共数值。
- [ ] `cargo fmt --check`、目标 crate tests、`xtask check-dep-dag` 通过。

**依赖:** T-contract-types-02, T-contract-types-03
**Blocked:** baseline ID

---

# 2.2 `error`

## A. 一句话定位 + 边界

**定位：** 拥有内部失败分类、bounded detail 和唯一外部映射出口；不拥有日志、恢复策略、进程处置或公共错误码数值。

**输入 / 输出 / 所有权：** 发生失败的模块提供事实；error 模块拥有表示与映射；FFI 只消费 `KernelError`。无资源所有权。

**线程模型：** immutable 值可跨线程；构造无全局锁，不格式化任意字符串，不触发 recorder。

## B. 内部子模块怎么切

| 单元 | 文件 | 依赖 | 可见性 | 切分理由 |
|---|---|---|---|---|
| category | `.../error/category.rs` | 无 | crate 外 | 稳定名称 |
| detail | `.../error/detail.rs` | category | crate 外只读 | bounded context |
| mapping | `.../error/mapping.rs` | contract-types | crate 内/FFI 间接 | 数值唯一出口 |
| facade | `.../error/mod.rs` | 全部 | crate 外 | 限制 re-export |

## C. 代码面

```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ErrorCategory { InvalidArgument, InvalidHandle, WrongContext, AlreadyReleased,
    BufferTooSmall, CapacityExceeded, CapabilityUnavailable, Cancelled, TimedOut,
    ContextClosing, ContextDestroyed, PanicBoundary, InternalInvariant }
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ErrorDetail { None, RequiredCapacity { required: u64, provided: u64 },
    LimitExceeded { limit: u64, requested: u64 }, StaticMessage(&'static str) }
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct KernelError { category: ErrorCategory, detail: ErrorDetail }
pub type KernelResult<T> = Result<T, KernelError>;
impl KernelError { pub const fn new(c: ErrorCategory, d: ErrorDetail) -> Self;
    pub const fn category(&self) -> ErrorCategory; pub fn detail(&self) -> &ErrorDetail; }
#[cfg(feature = "architecture-contracts")]
pub fn to_architecture_error_code(e: &KernelError)
    -> lumio_contract_types::ArchitectureErrorCode;
```

**关键不变式：**

- 每个失败一个主 category。
- detail 不持任意 `String`、backtrace 或第三方 error。
- panic 只在 FFI boundary 转 `PanicBoundary`。
- 映射集中且 exhaustiveness test 完整。

**失败与稳定类别：** InvalidArgument、BufferTooSmall、CapacityExceeded 等内部名称；公共数值 BLOCKED_ABI。

**相邻模块接口：** Consumes：`ArchitectureErrorCode`。Produces：`KernelError`、`KernelResult<T>`、唯一 mapping。

## D. 成熟方案选型

| 候选 | 许可证 | 满足 | 不满足 | 裁决 |
|---|---|---|---|---|
| `thiserror` | MIT/Apache-2.0 | derive 便利 | 收益小且鼓励 Display/source 链 | 不采用 |
| `anyhow` | MIT/Apache-2.0 | 应用上下文 | 类型擦除 | 禁止进入 Kernel API |
| typed enum | 本仓 | 精确有界 | 维护 mapping tests | 采用 |

V1 生产路径不新增第三方依赖；标准库、生成物和小型 typed model 已足够，dev dependency 不进入生产 graph。

## E. 测试与 Fixture

- **unit：** `kernel_error_preserves_category_and_detail`、`mapping_is_total`。
- **concurrency：** 不适用。
- **fault：** panic detail bounded、required length。
- **bench：** construct error zero-allocation。
- **共同要求：** 固定 baseline/build/target/seed；可由 FakeClock/Interleaving 表达的竞态不得只依赖 `sleep`；bench 报告 throughput、p50/p95/p99/max、alloc、peak bytes、queue depth，时钟读数不进入权威 Hash。

## F. 本模块任务列表

### T-error-01: 定义错误类别与 bounded detail

**Files:**
- Create: `crates/lumio-kernel/src/error/category.rs`
- Modify: `crates/lumio-kernel/src/lib.rs`
- Test: `crates/lumio-kernel/tests/kernel_error_detail_is_bounded.rs`

**Consumes:** 无
**Produces:** `ErrorCategory`, `ErrorDetail`, `KernelError`
**成熟方案:** typed enum

**步骤（TDD）:**
1. 先写失败测试 `kernel_error_detail_is_bounded`，固定输入、预期状态/错误类别与 owner/handle/allocator 计数。
2. 写只覆盖本任务的最小实现；不得扩大稳定 API、写公共数值或绕过 Adapter。
3. 运行 `cargo test -p lumio-kernel kernel_error_detail_is_bounded`、`cargo xtask check-dep-dag`；期望测试和依赖 Gate 同时通过。

**验收:**
- [ ] `crates/lumio-kernel/src/error/category.rs` 存在，`lib.rs` 只增加必要声明/re-export。
- [ ] `kernel_error_detail_is_bounded` 在实现前失败、实现后可重复通过，不靠 wall-clock sleep 碰运气。
- [ ] ``ErrorCategory`, `ErrorDetail`, `KernelError`` 不泄漏第三方类型或未批准公共数值。
- [ ] `cargo fmt --check`、目标 crate tests、`xtask check-dep-dag` 通过。

**依赖:** 无
**Blocked:** 无

### T-error-02: 实现 KernelResult constructors

**Files:**
- Create: `crates/lumio-kernel/src/error/mod.rs`
- Modify: `crates/lumio-kernel/src/lib.rs`
- Test: `crates/lumio-kernel/tests/buffer_too_small_reports_required.rs`

**Consumes:** T-error-01
**Produces:** `KernelResult<T>` 与 constructors
**成熟方案:** std

**步骤（TDD）:**
1. 先写失败测试 `buffer_too_small_reports_required`，固定输入、预期状态/错误类别与 owner/handle/allocator 计数。
2. 写只覆盖本任务的最小实现；不得扩大稳定 API、写公共数值或绕过 Adapter。
3. 运行 `cargo test -p lumio-kernel buffer_too_small_reports_required`、`cargo xtask check-dep-dag`；期望测试和依赖 Gate 同时通过。

**验收:**
- [ ] `crates/lumio-kernel/src/error/mod.rs` 存在，`lib.rs` 只增加必要声明/re-export。
- [ ] `buffer_too_small_reports_required` 在实现前失败、实现后可重复通过，不靠 wall-clock sleep 碰运气。
- [ ] ``KernelResult<T>` 与 constructors` 不泄漏第三方类型或未批准公共数值。
- [ ] `cargo fmt --check`、目标 crate tests、`xtask check-dep-dag` 通过。

**依赖:** T-error-01
**Blocked:** 无

### T-error-03: 建立架构错误码映射

**Files:**
- Create: `crates/lumio-kernel/src/error/mapping.rs`
- Modify: `crates/lumio-kernel/src/lib.rs`
- Test: `crates/lumio-kernel/tests/mapping_is_total_for_all_categories.rs`

**Consumes:** T-contract-types-02, T-error-01
**Produces:** `to_architecture_error_code`
**成熟方案:** 生成 Registry

**步骤（TDD）:**
1. 先写失败测试 `mapping_is_total_for_all_categories`，固定输入、预期状态/错误类别与 owner/handle/allocator 计数。
2. 写只覆盖本任务的最小实现；不得扩大稳定 API、写公共数值或绕过 Adapter。
3. 运行 `cargo test -p lumio-kernel mapping_is_total_for_all_categories`、`cargo xtask check-dep-dag`；期望测试和依赖 Gate 同时通过。

**验收:**
- [ ] `crates/lumio-kernel/src/error/mapping.rs` 存在，`lib.rs` 只增加必要声明/re-export。
- [ ] `mapping_is_total_for_all_categories` 在实现前失败、实现后可重复通过，不靠 wall-clock sleep 碰运气。
- [ ] ``to_architecture_error_code`` 不泄漏第三方类型或未批准公共数值。
- [ ] `cargo fmt --check`、目标 crate tests、`xtask check-dep-dag` 通过。

**依赖:** T-contract-types-02, T-error-01
**Blocked:** ErrorCode 数值

### T-error-04: 建立错误无分配/负向测试

**Files:**
- Create: `crates/lumio-kernel/tests/error_contract.rs`
- Modify: `crates/lumio-kernel/src/lib.rs`
- Test: `crates/lumio-kernel/tests/error_contract.rs`

**Consumes:** T-error-02
**Produces:** error conformance suite
**成熟方案:** dev test tools

**步骤（TDD）:**
1. 先写失败测试 `error_hot_path_does_not_allocate`，固定输入、预期状态/错误类别与 owner/handle/allocator 计数。
2. 写只覆盖本任务的最小实现；不得扩大稳定 API、写公共数值或绕过 Adapter。
3. 运行 `cargo test -p lumio-kernel error_hot_path_does_not_allocate`、`cargo xtask check-dep-dag`；期望测试和依赖 Gate 同时通过。

**验收:**
- [ ] `crates/lumio-kernel/tests/error_contract.rs` 存在，`lib.rs` 只增加必要声明/re-export。
- [ ] `error_hot_path_does_not_allocate` 在实现前失败、实现后可重复通过，不靠 wall-clock sleep 碰运气。
- [ ] `error conformance suite` 不泄漏第三方类型或未批准公共数值。
- [ ] `cargo fmt --check`、目标 crate tests、`xtask check-dep-dag` 通过。

**依赖:** T-error-02
**Blocked:** 无

---

# 2.3 `capability`

## A. 一句话定位 + 边界

**定位：** 提供 Static/Configured/Runtime 三层能力视图与 `require` gate；不拥有 Host role、RoomMode、权限、Release 路由或动态协商。

**输入 / 输出 / 所有权：** 输入是编译/平台事实、Context limits 和运行计数；输出 snapshot/require。Context 拥有配置，Capability 模块只表示。

**线程模型：** snapshot 可并发读；runtime counters 用原子或短锁；无回调。

## B. 内部子模块怎么切

| 单元 | 文件 | 依赖 | 可见性 | 切分理由 |
|---|---|---|---|---|
| static set | `.../capability/static_set.rs` | contract-types | crate 外 | bits 转换集中 |
| limits | `.../capability/limits.rs` | error | crate 外 | 创建期验证 |
| runtime | `.../capability/runtime.rs` | 无 | crate 内/快照外 | 瞬时状态独立 |
| facade | `.../capability/mod.rs` | 三者 | crate 外 | 统一查询 |

## C. 代码面

```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct CapabilityKey(u16);
pub struct StaticCapabilities { keys: Box<[CapabilityKey]> }
pub struct ConfiguredLimits { pub max_handles: u32, pub max_native_bytes: u64,
    pub max_jobs_queued: u32, pub max_jobs_running: u32, pub max_completion_items: u32 }
pub struct RuntimeStatus { pub accepting_work: bool, pub queued_jobs: u32,
    pub running_jobs: u32, pub allocated_native_bytes: u64 }
pub trait CapabilitySource: Send + Sync {
    fn static_capabilities(&self) -> &StaticCapabilities;
    fn configured_limits(&self) -> &ConfiguredLimits;
    fn runtime_status(&self) -> RuntimeStatus;
    fn require(&self, key: CapabilityKey) -> crate::error::KernelResult<()>;
}
```

**关键不变式：**

- Static 在 Context 生命周期内不变。
- ConfiguredLimits 创建后冻结。
- RuntimeStatus 不是 capability bits。
- `CapabilityKey` constructor crate-private，公共 bits 只经生成 Registry。

**失败与稳定类别：** InvalidArgument（limits）、CapabilityUnavailable；snapshot 不失败。

**相邻模块接口：** Consumes：生成 Capability Registry、KernelError。Produces：`CapabilitySource` 与三层模型。

## D. 成熟方案选型

| 候选 | 许可证 | 满足 | 不满足 | 裁决 |
|---|---|---|---|---|
| `bitflags` | MIT/Apache-2.0 | 成熟 bitset | bits 已由生成物定义 | 不用于内部模型 |
| `enumset` | MIT/Apache-2.0 | 紧凑 | 第三方 API 泄漏风险 | 不采用 |
| 排序 boxed slice | std | 确定、简单 | 仅创建时插入 | 采用 |

V1 生产路径不新增第三方依赖；标准库、生成物和小型 typed model 已足够，dev dependency 不进入生产 graph。

## E. 测试与 Fixture

- **unit：** limits validation、require missing。
- **concurrency：** runtime snapshot consistency。
- **fault：** limit overflow。
- **bench：** require p99。
- **共同要求：** 固定 baseline/build/target/seed；可由 FakeClock/Interleaving 表达的竞态不得只依赖 `sleep`；bench 报告 throughput、p50/p95/p99/max、alloc、peak bytes、queue depth，时钟读数不进入权威 Hash。

## F. 本模块任务列表

### T-capability-01: 定义 StaticCapabilities 与生成转换

**Files:**
- Create: `crates/lumio-kernel/src/capability/static_set.rs`
- Modify: `crates/lumio-kernel/src/lib.rs`
- Test: `crates/lumio-kernel/tests/static_capabilities_are_sorted_unique.rs`

**Consumes:** T-contract-types-02, T-error-02
**Produces:** `CapabilityKey`, `StaticCapabilities`
**成熟方案:** std sorted slice

**步骤（TDD）:**
1. 先写失败测试 `static_capabilities_are_sorted_unique`，固定输入、预期状态/错误类别与 owner/handle/allocator 计数。
2. 写只覆盖本任务的最小实现；不得扩大稳定 API、写公共数值或绕过 Adapter。
3. 运行 `cargo test -p lumio-kernel static_capabilities_are_sorted_unique`、`cargo xtask check-dep-dag`；期望测试和依赖 Gate 同时通过。

**验收:**
- [ ] `crates/lumio-kernel/src/capability/static_set.rs` 存在，`lib.rs` 只增加必要声明/re-export。
- [ ] `static_capabilities_are_sorted_unique` 在实现前失败、实现后可重复通过，不靠 wall-clock sleep 碰运气。
- [ ] ``CapabilityKey`, `StaticCapabilities`` 不泄漏第三方类型或未批准公共数值。
- [ ] `cargo fmt --check`、目标 crate tests、`xtask check-dep-dag` 通过。

**依赖:** T-contract-types-02, T-error-02
**Blocked:** Capability bits

### T-capability-02: 定义 ConfiguredLimits 验证

**Files:**
- Create: `crates/lumio-kernel/src/capability/limits.rs`
- Modify: `crates/lumio-kernel/src/lib.rs`
- Test: `crates/lumio-kernel/tests/invalid_limits_are_rejected.rs`

**Consumes:** T-error-02
**Produces:** `ConfiguredLimits::validate`
**成熟方案:** std

**步骤（TDD）:**
1. 先写失败测试 `invalid_limits_are_rejected`，固定输入、预期状态/错误类别与 owner/handle/allocator 计数。
2. 写只覆盖本任务的最小实现；不得扩大稳定 API、写公共数值或绕过 Adapter。
3. 运行 `cargo test -p lumio-kernel invalid_limits_are_rejected`、`cargo xtask check-dep-dag`；期望测试和依赖 Gate 同时通过。

**验收:**
- [ ] `crates/lumio-kernel/src/capability/limits.rs` 存在，`lib.rs` 只增加必要声明/re-export。
- [ ] `invalid_limits_are_rejected` 在实现前失败、实现后可重复通过，不靠 wall-clock sleep 碰运气。
- [ ] ``ConfiguredLimits::validate`` 不泄漏第三方类型或未批准公共数值。
- [ ] `cargo fmt --check`、目标 crate tests、`xtask check-dep-dag` 通过。

**依赖:** T-error-02
**Blocked:** 无

### T-capability-03: 定义 RuntimeStatus 原子快照

**Files:**
- Create: `crates/lumio-kernel/src/capability/runtime.rs`
- Modify: `crates/lumio-kernel/src/lib.rs`
- Test: `crates/lumio-kernel/tests/runtime_snapshot_is_consistent.rs`

**Consumes:** T-capability-02
**Produces:** `RuntimeCounters`, `RuntimeStatus`
**成熟方案:** std atomics

**步骤（TDD）:**
1. 先写失败测试 `runtime_snapshot_is_consistent`，固定输入、预期状态/错误类别与 owner/handle/allocator 计数。
2. 写只覆盖本任务的最小实现；不得扩大稳定 API、写公共数值或绕过 Adapter。
3. 运行 `cargo test -p lumio-kernel runtime_snapshot_is_consistent`、`cargo xtask check-dep-dag`；期望测试和依赖 Gate 同时通过。

**验收:**
- [ ] `crates/lumio-kernel/src/capability/runtime.rs` 存在，`lib.rs` 只增加必要声明/re-export。
- [ ] `runtime_snapshot_is_consistent` 在实现前失败、实现后可重复通过，不靠 wall-clock sleep 碰运气。
- [ ] ``RuntimeCounters`, `RuntimeStatus`` 不泄漏第三方类型或未批准公共数值。
- [ ] `cargo fmt --check`、目标 crate tests、`xtask check-dep-dag` 通过。

**依赖:** T-capability-02
**Blocked:** 无

### T-capability-04: 组合 CapabilitySource

**Files:**
- Create: `crates/lumio-kernel/src/capability/mod.rs`
- Modify: `crates/lumio-kernel/src/lib.rs`
- Test: `crates/lumio-kernel/tests/require_missing_capability_fails.rs`

**Consumes:** T-capability-01/02/03
**Produces:** `CapabilitySource`, snapshot
**成熟方案:** 本章 D

**步骤（TDD）:**
1. 先写失败测试 `require_missing_capability_fails`，固定输入、预期状态/错误类别与 owner/handle/allocator 计数。
2. 写只覆盖本任务的最小实现；不得扩大稳定 API、写公共数值或绕过 Adapter。
3. 运行 `cargo test -p lumio-kernel require_missing_capability_fails`、`cargo xtask check-dep-dag`；期望测试和依赖 Gate 同时通过。

**验收:**
- [ ] `crates/lumio-kernel/src/capability/mod.rs` 存在，`lib.rs` 只增加必要声明/re-export。
- [ ] `require_missing_capability_fails` 在实现前失败、实现后可重复通过，不靠 wall-clock sleep 碰运气。
- [ ] ``CapabilitySource`, snapshot` 不泄漏第三方类型或未批准公共数值。
- [ ] `cargo fmt --check`、目标 crate tests、`xtask check-dep-dag` 通过。

**依赖:** T-capability-01, T-capability-02, T-capability-03
**Blocked:** 无

---

# 2.4 `handle`

## A. 一句话定位 + 边界

**定位：** 拥有 Context-scoped Index+Generation 槽位、typed Handle、释放与永久退休；不决定 payload allocator 或资源业务生命周期，不编码公共 opaque 字段。

**输入 / 输出 / 所有权：** resource owner 插入/移除 payload；handle holder 只持 token；allocator 由 payload 模块负责。输出 typed Handle 与受控访问。

**线程模型：** 并发验证/访问；写操作短锁；持锁期间不得调用 resource destroy、recorder 或外部 closure。

## B. 内部子模块怎么切

| 单元 | 文件 | 依赖 | 可见性 | 切分理由 |
|---|---|---|---|---|
| key | `.../handle/key.rs` | 无 | crate 外 | 内部视图独立 |
| arena | `.../handle/arena.rs` | key+error | crate 内 | slot/reuse/retire 唯一 |
| registry | `.../handle/registry.rs` | arena | crate 外 | typed API |
| guard | `.../handle/guard.rs` | registry | crate 外受控 | 借用线性化 |
| ABI seam | `lumio-native-ffi/src/handles.rs` | generated+handle | FFI 内 | opaque 隔离 |

## C. 代码面

```rust
pub struct HandleArena<T> { context: ContextKey, capacity: u32, _private: () }
impl<T> HandleArena<T> {
    pub fn with_capacity(context: ContextKey, capacity: u32) -> crate::error::KernelResult<Self>;
    pub fn insert(&self, value: T) -> crate::error::KernelResult<Handle<T>>;
    pub fn with<R>(&self, h: Handle<T>, f: impl FnOnce(&T)->R) -> crate::error::KernelResult<R>;
    pub fn with_mut<R>(&self, h: Handle<T>, f: impl FnOnce(&mut T)->R) -> crate::error::KernelResult<R>;
    pub fn remove(&self, h: Handle<T>) -> crate::error::KernelResult<T>;
    pub fn retire_all(&self) -> HandleRetireReport;
    pub fn snapshot(&self) -> HandleArenaSnapshot;
}
```

**关键不变式：**

- 校验顺序 Context→bounds→occupied→generation。
- 重复释放不 double-drop。
- generation 溢出后 slot 永久 retired。
- Context destroy 后 token 永久失效。
- closure 不得递归操作同 arena；debug/test 检测。

**失败与稳定类别：** WrongContext、InvalidHandle、AlreadyReleased、CapacityExceeded、ContextDestroyed、InternalInvariant。

**相邻模块接口：** Consumes：ContextKey、KernelError。Produces：`Handle<T>`、`HandleArena<T>`、retire/snapshot。

## D. 成熟方案选型

| 候选 | 许可证 | 满足 | 不满足 | 裁决 |
|---|---|---|---|---|
| `slotmap` | MIT/Apache-2.0 | 成熟 key arena | retirement/Context 不满足 | 不采用 |
| `generational-arena` | MIT | 代际索引 | 同上且容量外包 | 不采用 |
| `slab`+metadata | MIT | free-list | 仍需全部安全语义 | 不采用首期 |
| 最小 bounded arena | 本仓 | 精确满足 | 需 loom/property/ADR | 采用 |

采用自研部分只限现成方案无法满足的最小语义；实现前必须提交 ADR 候选表，含维护责任、benchmark、退出 seam。

## E. 测试与 Fixture

- **unit：** insert/get/remove、overflow retire。
- **concurrency：** remove vs borrow、retire vs insert。
- **fault：** wrong Context、double release。
- **bench：** lookup/insert/remove、retired density。
- **共同要求：** 固定 baseline/build/target/seed；可由 FakeClock/Interleaving 表达的竞态不得只依赖 `sleep`；bench 报告 throughput、p50/p95/p99/max、alloc、peak bytes、queue depth，时钟读数不进入权威 Hash。

## F. 本模块任务列表

### T-handle-01: 定义 Context/Slot/Generation/HandleKey

**Files:**
- Create: `crates/lumio-kernel/src/handle/key.rs`
- Modify: `crates/lumio-kernel/src/lib.rs`
- Test: `crates/lumio-kernel/tests/handle_key_orders_stably.rs`

**Consumes:** T-error-02
**Produces:** `ContextKey`, `HandleKey`, `Handle<T>`
**成熟方案:** 自研最小类型

**步骤（TDD）:**
1. 先写失败测试 `handle_key_orders_stably`，固定输入、预期状态/错误类别与 owner/handle/allocator 计数。
2. 写只覆盖本任务的最小实现；不得扩大稳定 API、写公共数值或绕过 Adapter。
3. 运行 `cargo test -p lumio-kernel handle_key_orders_stably`、`cargo xtask check-dep-dag`；期望测试和依赖 Gate 同时通过。

**验收:**
- [ ] `crates/lumio-kernel/src/handle/key.rs` 存在，`lib.rs` 只增加必要声明/re-export。
- [ ] `handle_key_orders_stably` 在实现前失败、实现后可重复通过，不靠 wall-clock sleep 碰运气。
- [ ] ``ContextKey`, `HandleKey`, `Handle<T>`` 不泄漏第三方类型或未批准公共数值。
- [ ] `cargo fmt --check`、目标 crate tests、`xtask check-dep-dag` 通过。

**依赖:** T-error-02
**Blocked:** 无

### T-handle-02: 实现有界 slot/free-list

**Files:**
- Create: `crates/lumio-kernel/src/handle/arena.rs`
- Modify: `crates/lumio-kernel/src/lib.rs`
- Test: `crates/lumio-kernel/tests/arena_rejects_capacity_exhaustion.rs`

**Consumes:** T-handle-01
**Produces:** `HandleArena::with_capacity/insert`
**成熟方案:** 自研最小 arena，见 D

**步骤（TDD）:**
1. 先写失败测试 `arena_rejects_capacity_exhaustion`，固定输入、预期状态/错误类别与 owner/handle/allocator 计数。
2. 写只覆盖本任务的最小实现；不得扩大稳定 API、写公共数值或绕过 Adapter。
3. 运行 `cargo test -p lumio-kernel arena_rejects_capacity_exhaustion`、`cargo xtask check-dep-dag`；期望测试和依赖 Gate 同时通过。

**验收:**
- [ ] `crates/lumio-kernel/src/handle/arena.rs` 存在，`lib.rs` 只增加必要声明/re-export。
- [ ] `arena_rejects_capacity_exhaustion` 在实现前失败、实现后可重复通过，不靠 wall-clock sleep 碰运气。
- [ ] ``HandleArena::with_capacity/insert`` 不泄漏第三方类型或未批准公共数值。
- [ ] `cargo fmt --check`、目标 crate tests、`xtask check-dep-dag` 通过。

**依赖:** T-handle-01
**Blocked:** 无

### T-handle-03: 实现 generation 校验与永久退休

**Files:**
- Create: `crates/lumio-kernel/src/handle/arena.rs`
- Modify: `crates/lumio-kernel/src/lib.rs`
- Test: `crates/lumio-kernel/tests/generation_overflow_retires_slot_permanently.rs`

**Consumes:** T-handle-02
**Produces:** `remove` 与 retired slots
**成熟方案:** 自研最小 arena，见 D

**步骤（TDD）:**
1. 先写失败测试 `generation_overflow_retires_slot_permanently`，固定输入、预期状态/错误类别与 owner/handle/allocator 计数。
2. 写只覆盖本任务的最小实现；不得扩大稳定 API、写公共数值或绕过 Adapter。
3. 运行 `cargo test -p lumio-kernel generation_overflow_retires_slot_permanently`、`cargo xtask check-dep-dag`；期望测试和依赖 Gate 同时通过。

**验收:**
- [ ] `crates/lumio-kernel/src/handle/arena.rs` 存在，`lib.rs` 只增加必要声明/re-export。
- [ ] `generation_overflow_retires_slot_permanently` 在实现前失败、实现后可重复通过，不靠 wall-clock sleep 碰运气。
- [ ] ``remove` 与 retired slots` 不泄漏第三方类型或未批准公共数值。
- [ ] `cargo fmt --check`、目标 crate tests、`xtask check-dep-dag` 通过。

**依赖:** T-handle-02
**Blocked:** 无

### T-handle-04: 实现 Context-scoped typed registry

**Files:**
- Create: `crates/lumio-kernel/src/handle/registry.rs`
- Modify: `crates/lumio-kernel/src/lib.rs`
- Test: `crates/lumio-kernel/tests/wrong_context_is_rejected_first.rs`

**Consumes:** T-handle-03
**Produces:** `TypedHandleRegistry<T>`
**成熟方案:** 本章 D

**步骤（TDD）:**
1. 先写失败测试 `wrong_context_is_rejected_first`，固定输入、预期状态/错误类别与 owner/handle/allocator 计数。
2. 写只覆盖本任务的最小实现；不得扩大稳定 API、写公共数值或绕过 Adapter。
3. 运行 `cargo test -p lumio-kernel wrong_context_is_rejected_first`、`cargo xtask check-dep-dag`；期望测试和依赖 Gate 同时通过。

**验收:**
- [ ] `crates/lumio-kernel/src/handle/registry.rs` 存在，`lib.rs` 只增加必要声明/re-export。
- [ ] `wrong_context_is_rejected_first` 在实现前失败、实现后可重复通过，不靠 wall-clock sleep 碰运气。
- [ ] ``TypedHandleRegistry<T>`` 不泄漏第三方类型或未批准公共数值。
- [ ] `cargo fmt --check`、目标 crate tests、`xtask check-dep-dag` 通过。

**依赖:** T-handle-03
**Blocked:** 无

### T-handle-05: 实现 borrow/remove 线性化

**Files:**
- Create: `crates/lumio-kernel/src/handle/guard.rs`
- Modify: `crates/lumio-kernel/src/lib.rs`
- Test: `crates/lumio-kernel/tests/remove_vs_borrow_has_single_winner.rs`

**Consumes:** T-handle-04, T-test-support-02
**Produces:** borrow guards
**成熟方案:** std lock + loom

**步骤（TDD）:**
1. 先写失败测试 `remove_vs_borrow_has_single_winner`，固定输入、预期状态/错误类别与 owner/handle/allocator 计数。
2. 写只覆盖本任务的最小实现；不得扩大稳定 API、写公共数值或绕过 Adapter。
3. 运行 `cargo test -p lumio-kernel remove_vs_borrow_has_single_winner`、`cargo xtask check-dep-dag`；期望测试和依赖 Gate 同时通过。

**验收:**
- [ ] `crates/lumio-kernel/src/handle/guard.rs` 存在，`lib.rs` 只增加必要声明/re-export。
- [ ] `remove_vs_borrow_has_single_winner` 在实现前失败、实现后可重复通过，不靠 wall-clock sleep 碰运气。
- [ ] `borrow guards` 不泄漏第三方类型或未批准公共数值。
- [ ] `cargo fmt --check`、目标 crate tests、`xtask check-dep-dag` 通过。

**依赖:** T-handle-04, T-test-support-02
**Blocked:** 无

### T-handle-06: 实现 retire_all 与泄漏报告

**Files:**
- Create: `crates/lumio-kernel/src/handle/registry.rs`
- Modify: `crates/lumio-kernel/src/lib.rs`
- Test: `crates/lumio-kernel/tests/retire_all_drops_each_payload_once.rs`

**Consumes:** T-handle-05, T-test-support-03
**Produces:** `HandleRetireReport`, snapshot
**成熟方案:** 本章 D

**步骤（TDD）:**
1. 先写失败测试 `retire_all_drops_each_payload_once`，固定输入、预期状态/错误类别与 owner/handle/allocator 计数。
2. 写只覆盖本任务的最小实现；不得扩大稳定 API、写公共数值或绕过 Adapter。
3. 运行 `cargo test -p lumio-kernel retire_all_drops_each_payload_once`、`cargo xtask check-dep-dag`；期望测试和依赖 Gate 同时通过。

**验收:**
- [ ] `crates/lumio-kernel/src/handle/registry.rs` 存在，`lib.rs` 只增加必要声明/re-export。
- [ ] `retire_all_drops_each_payload_once` 在实现前失败、实现后可重复通过，不靠 wall-clock sleep 碰运气。
- [ ] ``HandleRetireReport`, snapshot` 不泄漏第三方类型或未批准公共数值。
- [ ] `cargo fmt --check`、目标 crate tests、`xtask check-dep-dag` 通过。

**依赖:** T-handle-05, T-test-support-03
**Blocked:** 无

---

# 2.5 `memory`

## A. 一句话定位 + 边界

**定位：** 实现三类 Buffer、预算计费、allocator provenance、Native-owned buffer owner 与 call scratch；不决定 Job 调度、Codec 格式或领域对象布局。

**输入 / 输出 / 所有权：** 输入 Context budget、调用方 byte slices 与 allocation request；输出 Buffer wrappers、Native handle、allocation snapshot。resource owner、handle holder、allocator 必须可区分。

**线程模型：** 预算线程安全；CallerOutput 独占借用；Native buffer handle 可跨线程，payload 访问经 registry。

## B. 内部子模块怎么切

| 单元 | 文件 | 依赖 | 可见性 | 切分理由 |
|---|---|---|---|---|
| buffer | `.../memory/buffer.rs` | error | crate 外 | 三分类唯一 |
| budget | `.../memory/budget.rs` | capability | crate 外 | reserve-before-allocate |
| provenance | `.../memory/provenance.rs` | handle | crate 外只读 | 跨 allocator 检测 |
| native owner | `.../memory/native_buffers.rs` | handle+budget | crate 外 | 句柄/分配联结 |
| scratch | `.../memory/call_scratch.rs` | budget | crate 内 | vendor adapter 可替换 |

## C. 代码面

```rust
pub struct MemoryBudget { context: crate::handle::ContextKey, limit_bytes: u64, _private: () }
impl MemoryBudget {
    pub fn new(context: crate::handle::ContextKey, limit: u64) -> crate::error::KernelResult<Self>;
    pub fn reserve(&self, class: AllocationClass, bytes: usize)
        -> crate::error::KernelResult<AllocationReservation>;
    pub fn snapshot(&self) -> AllocationSnapshot;
}
pub struct NativeBufferOwner { handles: crate::handle::HandleArena<NativeBuffer>,
    budget: std::sync::Arc<MemoryBudget> }
impl NativeBufferOwner {
    pub fn allocate(&self, len: usize) -> crate::error::KernelResult<NativeOwnedBufferHandle>;
    pub fn with_bytes<R>(&self, h: NativeOwnedBufferHandle, f: impl FnOnce(&[u8])->R)
        -> crate::error::KernelResult<R>;
    pub fn release(&self, h: NativeOwnedBufferHandle) -> crate::error::KernelResult<()>;
    pub fn release_all(&self) -> NativeBufferReleaseReport;
}
```

**关键不变式：**

- 先 reserve 后 allocate，失败回滚 reservation。
- CallerOutput `write_all` 不部分写后才报不足。
- Native buffer 只由原 `AllocatorId` 回收。
- Context close 后拒绝新 allocation，第六步统一退休。
- 统计以 charged bytes 为准。

**失败与稳定类别：** BufferTooSmall、CapacityExceeded、InvalidHandle、WrongContext、AlreadyReleased、ContextClosing、InternalInvariant。

**相邻模块接口：** Consumes：HandleArena、ConfiguredLimits。Produces：Buffer 三类、MemoryBudget、NativeBufferOwner、provenance。

## D. 成熟方案选型

| 候选 | 许可证 | 满足 | 不满足 | 裁决 |
|---|---|---|---|---|
| `bumpalo` | MIT/Apache-2.0 | call scratch | 不支持单对象释放/完整统计 | 仅 scratch adapter |
| `slab` | MIT | 池化 | 与 HandleArena 重叠 | 不采用 |
| system allocator+budget | std | 标准行为 | 需本仓计费 | 采用 |

采用项只能存在于指定 Adapter；供应商类型不得穿过 crate 公共 port 或 C ABI。

## E. 测试与 Fixture

- **unit：** output all-or-nothing、reservation rollback。
- **concurrency：** reservations never exceed limit、release vs read。
- **fault：** double release、wrong allocator、close reclaim。
- **bench：** alloc/release、copy bytes、peak charged。
- **共同要求：** 固定 baseline/build/target/seed；可由 FakeClock/Interleaving 表达的竞态不得只依赖 `sleep`；bench 报告 throughput、p50/p95/p99/max、alloc、peak bytes、queue depth，时钟读数不进入权威 Hash。

## F. 本模块任务列表

### T-memory-01: 实现三类 Buffer newtype

**Files:**
- Create: `crates/lumio-kernel/src/memory/buffer.rs`
- Modify: `crates/lumio-kernel/src/lib.rs`
- Test: `crates/lumio-kernel/tests/caller_output_write_is_atomic.rs`

**Consumes:** T-error-02, T-handle-01
**Produces:** 三类 Buffer Rust API
**成熟方案:** std

**步骤（TDD）:**
1. 先写失败测试 `caller_output_write_is_atomic`，固定输入、预期状态/错误类别与 owner/handle/allocator 计数。
2. 写只覆盖本任务的最小实现；不得扩大稳定 API、写公共数值或绕过 Adapter。
3. 运行 `cargo test -p lumio-kernel caller_output_write_is_atomic`、`cargo xtask check-dep-dag`；期望测试和依赖 Gate 同时通过。

**验收:**
- [ ] `crates/lumio-kernel/src/memory/buffer.rs` 存在，`lib.rs` 只增加必要声明/re-export。
- [ ] `caller_output_write_is_atomic` 在实现前失败、实现后可重复通过，不靠 wall-clock sleep 碰运气。
- [ ] `三类 Buffer Rust API` 不泄漏第三方类型或未批准公共数值。
- [ ] `cargo fmt --check`、目标 crate tests、`xtask check-dep-dag` 通过。

**依赖:** T-error-02, T-handle-01
**Blocked:** 无

### T-memory-02: 定义 allocator provenance

**Files:**
- Create: `crates/lumio-kernel/src/memory/provenance.rs`
- Modify: `crates/lumio-kernel/src/lib.rs`
- Test: `crates/lumio-kernel/tests/provenance_preserves_allocator_context_class.rs`

**Consumes:** T-handle-01
**Produces:** `AllocatorId`, `AllocationProvenance`
**成熟方案:** std typed values

**步骤（TDD）:**
1. 先写失败测试 `provenance_preserves_allocator_context_class`，固定输入、预期状态/错误类别与 owner/handle/allocator 计数。
2. 写只覆盖本任务的最小实现；不得扩大稳定 API、写公共数值或绕过 Adapter。
3. 运行 `cargo test -p lumio-kernel provenance_preserves_allocator_context_class`、`cargo xtask check-dep-dag`；期望测试和依赖 Gate 同时通过。

**验收:**
- [ ] `crates/lumio-kernel/src/memory/provenance.rs` 存在，`lib.rs` 只增加必要声明/re-export。
- [ ] `provenance_preserves_allocator_context_class` 在实现前失败、实现后可重复通过，不靠 wall-clock sleep 碰运气。
- [ ] ``AllocatorId`, `AllocationProvenance`` 不泄漏第三方类型或未批准公共数值。
- [ ] `cargo fmt --check`、目标 crate tests、`xtask check-dep-dag` 通过。

**依赖:** T-handle-01
**Blocked:** 无

### T-memory-03: 实现预算 reserve/release ledger

**Files:**
- Create: `crates/lumio-kernel/src/memory/budget.rs`
- Modify: `crates/lumio-kernel/src/lib.rs`
- Test: `crates/lumio-kernel/tests/concurrent_reservations_never_exceed_limit.rs`

**Consumes:** T-capability-02, T-memory-02
**Produces:** `MemoryBudget`, reservation
**成熟方案:** std atomics

**步骤（TDD）:**
1. 先写失败测试 `concurrent_reservations_never_exceed_limit`，固定输入、预期状态/错误类别与 owner/handle/allocator 计数。
2. 写只覆盖本任务的最小实现；不得扩大稳定 API、写公共数值或绕过 Adapter。
3. 运行 `cargo test -p lumio-kernel concurrent_reservations_never_exceed_limit`、`cargo xtask check-dep-dag`；期望测试和依赖 Gate 同时通过。

**验收:**
- [ ] `crates/lumio-kernel/src/memory/budget.rs` 存在，`lib.rs` 只增加必要声明/re-export。
- [ ] `concurrent_reservations_never_exceed_limit` 在实现前失败、实现后可重复通过，不靠 wall-clock sleep 碰运气。
- [ ] ``MemoryBudget`, reservation` 不泄漏第三方类型或未批准公共数值。
- [ ] `cargo fmt --check`、目标 crate tests、`xtask check-dep-dag` 通过。

**依赖:** T-capability-02, T-memory-02
**Blocked:** 无

### T-memory-04: 实现 NativeBufferOwner

**Files:**
- Create: `crates/lumio-kernel/src/memory/native_buffers.rs`
- Modify: `crates/lumio-kernel/src/lib.rs`
- Test: `crates/lumio-kernel/tests/double_release_frees_once.rs`

**Consumes:** T-handle-04, T-memory-01/03
**Produces:** `NativeBufferOwner`
**成熟方案:** system allocator wrapper

**步骤（TDD）:**
1. 先写失败测试 `double_release_frees_once`，固定输入、预期状态/错误类别与 owner/handle/allocator 计数。
2. 写只覆盖本任务的最小实现；不得扩大稳定 API、写公共数值或绕过 Adapter。
3. 运行 `cargo test -p lumio-kernel double_release_frees_once`、`cargo xtask check-dep-dag`；期望测试和依赖 Gate 同时通过。

**验收:**
- [ ] `crates/lumio-kernel/src/memory/native_buffers.rs` 存在，`lib.rs` 只增加必要声明/re-export。
- [ ] `double_release_frees_once` 在实现前失败、实现后可重复通过，不靠 wall-clock sleep 碰运气。
- [ ] ``NativeBufferOwner`` 不泄漏第三方类型或未批准公共数值。
- [ ] `cargo fmt --check`、目标 crate tests、`xtask check-dep-dag` 通过。

**依赖:** T-handle-04, T-memory-01, T-memory-03
**Blocked:** 无

### T-memory-05: 封装 CallScratch adapter

**Files:**
- Create: `crates/lumio-kernel/src/memory/call_scratch.rs`
- Modify: `crates/lumio-kernel/src/lib.rs`
- Test: `crates/lumio-kernel/tests/scratch_reset_releases_all_charge.rs`

**Consumes:** T-memory-03
**Produces:** `CallScratch`
**成熟方案:** `bumpalo` adapter

**步骤（TDD）:**
1. 先写失败测试 `scratch_reset_releases_all_charge`，固定输入、预期状态/错误类别与 owner/handle/allocator 计数。
2. 写只覆盖本任务的最小实现；不得扩大稳定 API、写公共数值或绕过 Adapter。
3. 运行 `cargo test -p lumio-kernel scratch_reset_releases_all_charge`、`cargo xtask check-dep-dag`；期望测试和依赖 Gate 同时通过。

**验收:**
- [ ] `crates/lumio-kernel/src/memory/call_scratch.rs` 存在，`lib.rs` 只增加必要声明/re-export。
- [ ] `scratch_reset_releases_all_charge` 在实现前失败、实现后可重复通过，不靠 wall-clock sleep 碰运气。
- [ ] ``CallScratch`` 不泄漏第三方类型或未批准公共数值。
- [ ] `cargo fmt --check`、目标 crate tests、`xtask check-dep-dag` 通过。

**依赖:** T-memory-03
**Blocked:** bumpalo supplier approval

### T-memory-06: 实现批量回收与统计

**Files:**
- Create: `crates/lumio-kernel/src/memory/native_buffers.rs`
- Modify: `crates/lumio-kernel/src/lib.rs`
- Test: `crates/lumio-kernel/tests/release_all_returns_zero_live_bytes.rs`

**Consumes:** T-memory-04, T-test-support-03
**Produces:** `NativeBufferReleaseReport`
**成熟方案:** 本章 D

**步骤（TDD）:**
1. 先写失败测试 `release_all_returns_zero_live_bytes`，固定输入、预期状态/错误类别与 owner/handle/allocator 计数。
2. 写只覆盖本任务的最小实现；不得扩大稳定 API、写公共数值或绕过 Adapter。
3. 运行 `cargo test -p lumio-kernel release_all_returns_zero_live_bytes`、`cargo xtask check-dep-dag`；期望测试和依赖 Gate 同时通过。

**验收:**
- [ ] `crates/lumio-kernel/src/memory/native_buffers.rs` 存在，`lib.rs` 只增加必要声明/re-export。
- [ ] `release_all_returns_zero_live_bytes` 在实现前失败、实现后可重复通过，不靠 wall-clock sleep 碰运气。
- [ ] ``NativeBufferReleaseReport`` 不泄漏第三方类型或未批准公共数值。
- [ ] `cargo fmt --check`、目标 crate tests、`xtask check-dep-dag` 通过。

**依赖:** T-memory-04, T-test-support-03
**Blocked:** 无

---

# 2.6 `kernel-context`

## A. 一句话定位 + 边界

**定位：** 是 NativeCore Context 生命周期唯一 owner，冻结 limits/capabilities，注册 `ContextResource` 并执行七步关闭；不拥有 World/Session/Host/Wall Clock 语义。

**输入 / 输出 / 所有权：** 输入 ContextConfig、clock、RecordPort、resource registrations；输出 ContextKey、admission gate 与 close report。Context 是 resource owner；上层只是 handle holder。

**线程模型：** 工作路径用原子 gate；close 幂等且单一赢家；registry lock 内不调用 resource。

## B. 内部子模块怎么切

| 单元 | 文件 | 依赖 | 可见性 | 切分理由 |
|---|---|---|---|---|
| config | `.../context/config.rs` | capability | crate 外 | 创建验证集中 |
| state | `.../context/state.rs` | platform | crate 内/快照外 | 线性化点 |
| resource | `.../context/resource.rs` | error+platform | crate 外 | 跨 crate 契约 |
| registry | `.../context/registry.rs` | resource | crate 内 | 快照后无锁调用 |
| lifecycle | `.../context/lifecycle.rs` | 前述+handle+memory | crate 外 | 七步唯一驱动 |
| facade | `.../context/mod.rs` | 全部 | crate 外 | 组合入口 |

## C. 代码面

```rust
pub struct ContextConfig { pub limits: crate::capability::ConfiguredLimits,
    pub quiesce_deadline: lumio_platform::Deadline }
pub struct KernelContext { key: crate::handle::ContextKey, _private: () }
impl KernelContext {
    pub fn create(config: ContextConfig, caps: crate::capability::StaticCapabilities,
        clock: std::sync::Arc<dyn lumio_platform::MonotonicClock>,
        records: std::sync::Arc<dyn crate::record::RecordPort>)
        -> crate::error::KernelResult<std::sync::Arc<Self>>;
    pub fn key(&self) -> crate::handle::ContextKey;
    pub fn state(&self) -> ContextStateSnapshot;
    pub fn ensure_accepting_work(&self) -> crate::error::KernelResult<()>;
    pub fn register_resource(&self, r: std::sync::Arc<dyn ContextResource>)
        -> crate::error::KernelResult<ResourceRegistration>;
    pub fn close(&self, reason: CancelReason, deadline: lumio_platform::Deadline)
        -> crate::error::KernelResult<ContextCloseReport>;
}
```

**关键不变式：**

- ContextKey 仅本进程隔离，不暗示 ABI 表示。
- close 一个驱动者；并发调用观察同一结果。
- Closing 后 register/submit/allocate 拒绝。
- resource callback 在 registry lock 外，destroy 逆序。
- quiesce 后才退休 handle/buffer，terminal 不可复活。

**失败与稳定类别：** InvalidArgument、ContextClosing、ContextDestroyed、TimedOut、InternalInvariant；Host 处置不在本模块。

**相邻模块接口：** Consumes：Capability、Handle、Memory、Clock、RecordPort。Produces：KernelContext、ContextResource、close report/gate。

## D. 成熟方案选型

| 候选 | 许可证 | 满足 | 不满足 | 裁决 |
|---|---|---|---|---|
| tokio CancellationToken | MIT | 取消成熟 | 引入 runtime 生态且不解决七步 | 不采用 |
| crossbeam epoch | MIT/Apache-2.0 | 并发 reclamation | 非生命周期状态机 | 不采用 |
| atomic gate+snapshot registry | std | 精确、依赖小 | 需 loom | 采用 |

V1 生产路径不新增第三方依赖；标准库、生成物和小型 typed model 已足够，dev dependency 不进入生产 graph。

## E. 测试与 Fixture

- **unit：** 七步顺序、逆序 destroy。
- **concurrency：** close/register、close/allocate、double close、deadline。
- **fault：** quiesce/destroy failure、late completion、leaked lease。
- **bench：** admission ns、close N resources。
- **共同要求：** 固定 baseline/build/target/seed；可由 FakeClock/Interleaving 表达的竞态不得只依赖 `sleep`；bench 报告 throughput、p50/p95/p99/max、alloc、peak bytes、queue depth，时钟读数不进入权威 Hash。

## F. 本模块任务列表

### T-context-01: 定义 ContextConfig 与创建验证

**Files:**
- Create: `crates/lumio-kernel/src/context/config.rs`
- Modify: `crates/lumio-kernel/src/lib.rs`
- Test: `crates/lumio-kernel/tests/context_config_rejects_invalid_limits.rs`

**Consumes:** T-capability-04, T-platform-01
**Produces:** `ContextConfig`
**成熟方案:** std

**步骤（TDD）:**
1. 先写失败测试 `context_config_rejects_invalid_limits`，固定输入、预期状态/错误类别与 owner/handle/allocator 计数。
2. 写只覆盖本任务的最小实现；不得扩大稳定 API、写公共数值或绕过 Adapter。
3. 运行 `cargo test -p lumio-kernel context_config_rejects_invalid_limits`、`cargo xtask check-dep-dag`；期望测试和依赖 Gate 同时通过。

**验收:**
- [ ] `crates/lumio-kernel/src/context/config.rs` 存在，`lib.rs` 只增加必要声明/re-export。
- [ ] `context_config_rejects_invalid_limits` 在实现前失败、实现后可重复通过，不靠 wall-clock sleep 碰运气。
- [ ] ``ContextConfig`` 不泄漏第三方类型或未批准公共数值。
- [ ] `cargo fmt --check`、目标 crate tests、`xtask check-dep-dag` 通过。

**依赖:** T-capability-04, T-platform-01
**Blocked:** 无

### T-context-02: 实现 admission/closing 原子 gate

**Files:**
- Create: `crates/lumio-kernel/src/context/state.rs`
- Modify: `crates/lumio-kernel/src/lib.rs`
- Test: `crates/lumio-kernel/tests/close_vs_admit_has_one_linearization.rs`

**Consumes:** T-context-01, T-test-support-02
**Produces:** `ContextStateGate`
**成熟方案:** std atomics+loom

**步骤（TDD）:**
1. 先写失败测试 `close_vs_admit_has_one_linearization`，固定输入、预期状态/错误类别与 owner/handle/allocator 计数。
2. 写只覆盖本任务的最小实现；不得扩大稳定 API、写公共数值或绕过 Adapter。
3. 运行 `cargo test -p lumio-kernel close_vs_admit_has_one_linearization`、`cargo xtask check-dep-dag`；期望测试和依赖 Gate 同时通过。

**验收:**
- [ ] `crates/lumio-kernel/src/context/state.rs` 存在，`lib.rs` 只增加必要声明/re-export。
- [ ] `close_vs_admit_has_one_linearization` 在实现前失败、实现后可重复通过，不靠 wall-clock sleep 碰运气。
- [ ] ``ContextStateGate`` 不泄漏第三方类型或未批准公共数值。
- [ ] `cargo fmt --check`、目标 crate tests、`xtask check-dep-dag` 通过。

**依赖:** T-context-01, T-test-support-02
**Blocked:** state names 与 spec 对齐

### T-context-03: 定义 ContextResource port

**Files:**
- Create: `crates/lumio-kernel/src/context/resource.rs`
- Modify: `crates/lumio-kernel/src/lib.rs`
- Test: `crates/lumio-kernel/tests/resource_port_is_object_safe.rs`

**Consumes:** T-error-02, T-platform-01
**Produces:** `ContextResource`, reports
**成熟方案:** std trait

**步骤（TDD）:**
1. 先写失败测试 `resource_port_is_object_safe`，固定输入、预期状态/错误类别与 owner/handle/allocator 计数。
2. 写只覆盖本任务的最小实现；不得扩大稳定 API、写公共数值或绕过 Adapter。
3. 运行 `cargo test -p lumio-kernel resource_port_is_object_safe`、`cargo xtask check-dep-dag`；期望测试和依赖 Gate 同时通过。

**验收:**
- [ ] `crates/lumio-kernel/src/context/resource.rs` 存在，`lib.rs` 只增加必要声明/re-export。
- [ ] `resource_port_is_object_safe` 在实现前失败、实现后可重复通过，不靠 wall-clock sleep 碰运气。
- [ ] ``ContextResource`, reports` 不泄漏第三方类型或未批准公共数值。
- [ ] `cargo fmt --check`、目标 crate tests、`xtask check-dep-dag` 通过。

**依赖:** T-error-02, T-platform-01
**Blocked:** 无

### T-context-04: 实现资源登记与顺序快照

**Files:**
- Create: `crates/lumio-kernel/src/context/registry.rs`
- Modify: `crates/lumio-kernel/src/lib.rs`
- Test: `crates/lumio-kernel/tests/registration_order_is_stable.rs`

**Consumes:** T-context-02/03
**Produces:** `ResourceRegistry`
**成熟方案:** std Arc/lock

**步骤（TDD）:**
1. 先写失败测试 `registration_order_is_stable`，固定输入、预期状态/错误类别与 owner/handle/allocator 计数。
2. 写只覆盖本任务的最小实现；不得扩大稳定 API、写公共数值或绕过 Adapter。
3. 运行 `cargo test -p lumio-kernel registration_order_is_stable`、`cargo xtask check-dep-dag`；期望测试和依赖 Gate 同时通过。

**验收:**
- [ ] `crates/lumio-kernel/src/context/registry.rs` 存在，`lib.rs` 只增加必要声明/re-export。
- [ ] `registration_order_is_stable` 在实现前失败、实现后可重复通过，不靠 wall-clock sleep 碰运气。
- [ ] ``ResourceRegistry`` 不泄漏第三方类型或未批准公共数值。
- [ ] `cargo fmt --check`、目标 crate tests、`xtask check-dep-dag` 通过。

**依赖:** T-context-02, T-context-03
**Blocked:** 无

### T-context-05: 实现七步关闭驱动

**Files:**
- Create: `crates/lumio-kernel/src/context/lifecycle.rs`
- Modify: `crates/lumio-kernel/src/lib.rs`
- Test: `crates/lumio-kernel/tests/close_executes_seven_steps_in_order.rs`

**Consumes:** T-context-04, T-handle-06, T-memory-06
**Produces:** `KernelContext::close`, report
**成熟方案:** 现有 lifecycle spec

**步骤（TDD）:**
1. 先写失败测试 `close_executes_seven_steps_in_order`，固定输入、预期状态/错误类别与 owner/handle/allocator 计数。
2. 写只覆盖本任务的最小实现；不得扩大稳定 API、写公共数值或绕过 Adapter。
3. 运行 `cargo test -p lumio-kernel close_executes_seven_steps_in_order`、`cargo xtask check-dep-dag`；期望测试和依赖 Gate 同时通过。

**验收:**
- [ ] `crates/lumio-kernel/src/context/lifecycle.rs` 存在，`lib.rs` 只增加必要声明/re-export。
- [ ] `close_executes_seven_steps_in_order` 在实现前失败、实现后可重复通过，不靠 wall-clock sleep 碰运气。
- [ ] ``KernelContext::close`, report` 不泄漏第三方类型或未批准公共数值。
- [ ] `cargo fmt --check`、目标 crate tests、`xtask check-dep-dag` 通过。

**依赖:** T-context-04, T-handle-06, T-memory-06
**Blocked:** 精确 state names

### T-context-06: 实现关闭竞态/超时矩阵

**Files:**
- Create: `crates/lumio-kernel/tests/context_lifecycle.rs`
- Modify: `crates/lumio-kernel/src/lib.rs`
- Test: `crates/lumio-kernel/tests/context_lifecycle.rs`

**Consumes:** T-context-05, T-test-support-01/02/03
**Produces:** Context conformance suite
**成熟方案:** loom+FakeClock

**步骤（TDD）:**
1. 先写失败测试 `late_resource_cannot_revive_context`，固定输入、预期状态/错误类别与 owner/handle/allocator 计数。
2. 写只覆盖本任务的最小实现；不得扩大稳定 API、写公共数值或绕过 Adapter。
3. 运行 `cargo test -p lumio-kernel late_resource_cannot_revive_context`、`cargo xtask check-dep-dag`；期望测试和依赖 Gate 同时通过。

**验收:**
- [ ] `crates/lumio-kernel/tests/context_lifecycle.rs` 存在，`lib.rs` 只增加必要声明/re-export。
- [ ] `late_resource_cannot_revive_context` 在实现前失败、实现后可重复通过，不靠 wall-clock sleep 碰运气。
- [ ] `Context conformance suite` 不泄漏第三方类型或未批准公共数值。
- [ ] `cargo fmt --check`、目标 crate tests、`xtask check-dep-dag` 通过。

**依赖:** T-context-05, T-test-support-01, T-test-support-02, T-test-support-03
**Blocked:** 架构 Fixture

---

# 2.7 `platform`

## A. 一句话定位 + 边界

**定位：** 只提供 NativeCore 私有单调时钟最小 port；不提供 Wall Clock、TickId、线程池、文件系统、网络或 Host pacing。

**输入 / 输出 / 所有权：** 输入 `std::time::Instant`；输出 process-relative Ticks/Deadline。无跨 ABI 对象。

**线程模型：** clock 可并发调用，可重入；无全局可变状态。

## B. 内部子模块怎么切

| 单元 | 文件 | 依赖 | 可见性 | 切分理由 |
|---|---|---|---|---|
| clock | `crates/lumio-platform/src/clock.rs` | std | crate 外 | 屏蔽 Instant |
| facade | `.../lib.rs` | clock | crate 外 | 保持小面 |
| fake | `lumio-test-support/src/clock.rs` | platform | dev-only | 生产不含可变 clock |

## C. 代码面

```rust
pub use clock::{Deadline, MonotonicClock, StdMonotonicClock, Ticks};
mod clock;
// No wall-clock or logical TickId APIs in this crate.
```

**关键不变式：**

- Ticks 仅相对同实例。
- duration 溢出 checked/saturating，不 wrap。
- FakeClock 仅 dev-only。

**失败与稳定类别：** 标准 clock 创建不失败；overflow 由 checked API 表达。

**相邻模块接口：** Produces：Ticks、Deadline、MonotonicClock；无 workspace consumes。

## D. 成熟方案选型

| 候选 | 许可证 | 满足 | 不满足 | 裁决 |
|---|---|---|---|---|
| std::time::Instant | Rust std | 三平台成熟 | 不可直接跨 ABI | 采用包装 |
| quanta | MIT/Apache-2.0 | 高性能 | 额外校准/依赖无证据 | 不采用 |
| web-time | MIT/Apache-2.0 | wasm | 当前非目标 | I2 |

V1 生产路径不新增第三方依赖；标准库、生成物和小型 typed model 已足够，dev dependency 不进入生产 graph。

## E. 测试与 Fixture

- **unit：** deadline/overflow。
- **concurrency：** multi-thread non-decreasing。
- **fault：** FakeClock backward reject。
- **bench：** now() distribution。
- **共同要求：** 固定 baseline/build/target/seed；可由 FakeClock/Interleaving 表达的竞态不得只依赖 `sleep`；bench 报告 throughput、p50/p95/p99/max、alloc、peak bytes、queue depth，时钟读数不进入权威 Hash。

## F. 本模块任务列表

### T-platform-01: 定义 Ticks/Deadline/MonotonicClock

**Files:**
- Create: `crates/lumio-platform/src/clock.rs`
- Modify: `crates/lumio-platform/src/lib.rs`
- Test: `crates/lumio-platform/tests/ticks_checked_add_does_not_wrap.rs`

**Consumes:** 无
**Produces:** clock port types
**成熟方案:** std

**步骤（TDD）:**
1. 先写失败测试 `ticks_checked_add_does_not_wrap`，固定输入、预期状态/错误类别与 owner/handle/allocator 计数。
2. 写只覆盖本任务的最小实现；不得扩大稳定 API、写公共数值或绕过 Adapter。
3. 运行 `cargo test -p lumio-platform ticks_checked_add_does_not_wrap`、`cargo xtask check-dep-dag`；期望测试和依赖 Gate 同时通过。

**验收:**
- [ ] `crates/lumio-platform/src/clock.rs` 存在，`lib.rs` 只增加必要声明/re-export。
- [ ] `ticks_checked_add_does_not_wrap` 在实现前失败、实现后可重复通过，不靠 wall-clock sleep 碰运气。
- [ ] `clock port types` 不泄漏第三方类型或未批准公共数值。
- [ ] `cargo fmt --check`、目标 crate tests、`xtask check-dep-dag` 通过。

**依赖:** 无
**Blocked:** 无

### T-platform-02: 实现 StdMonotonicClock

**Files:**
- Create: `crates/lumio-platform/src/clock.rs`
- Modify: `crates/lumio-platform/src/lib.rs`
- Test: `crates/lumio-platform/tests/std_clock_is_non_decreasing.rs`

**Consumes:** T-platform-01
**Produces:** `StdMonotonicClock`
**成熟方案:** std Instant

**步骤（TDD）:**
1. 先写失败测试 `std_clock_is_non_decreasing`，固定输入、预期状态/错误类别与 owner/handle/allocator 计数。
2. 写只覆盖本任务的最小实现；不得扩大稳定 API、写公共数值或绕过 Adapter。
3. 运行 `cargo test -p lumio-platform std_clock_is_non_decreasing`、`cargo xtask check-dep-dag`；期望测试和依赖 Gate 同时通过。

**验收:**
- [ ] `crates/lumio-platform/src/clock.rs` 存在，`lib.rs` 只增加必要声明/re-export。
- [ ] `std_clock_is_non_decreasing` 在实现前失败、实现后可重复通过，不靠 wall-clock sleep 碰运气。
- [ ] ``StdMonotonicClock`` 不泄漏第三方类型或未批准公共数值。
- [ ] `cargo fmt --check`、目标 crate tests、`xtask check-dep-dag` 通过。

**依赖:** T-platform-01
**Blocked:** 无

### T-platform-03: 建立 clock benchmark/三平台 smoke

**Files:**
- Create: `crates/lumio-platform/benches/clock.rs`
- Modify: `crates/lumio-platform/src/lib.rs`
- Test: `crates/lumio-platform/benches/clock.rs`

**Consumes:** T-platform-02
**Produces:** clock benchmark report
**成熟方案:** criterion dev-only

**步骤（TDD）:**
1. 先写失败测试 `clock_benchmark_reports_distribution`，固定输入、预期状态/错误类别与 owner/handle/allocator 计数。
2. 写只覆盖本任务的最小实现；不得扩大稳定 API、写公共数值或绕过 Adapter。
3. 运行 `cargo test -p lumio-platform clock_benchmark_reports_distribution`、`cargo xtask check-dep-dag`；期望测试和依赖 Gate 同时通过。

**验收:**
- [ ] `crates/lumio-platform/benches/clock.rs` 存在，`lib.rs` 只增加必要声明/re-export。
- [ ] `clock_benchmark_reports_distribution` 在实现前失败、实现后可重复通过，不靠 wall-clock sleep 碰运气。
- [ ] `clock benchmark report` 不泄漏第三方类型或未批准公共数值。
- [ ] `cargo fmt --check`、目标 crate tests、`xtask check-dep-dag` 通过。

**依赖:** T-platform-02
**Blocked:** 三平台 CI runner

---

# 2.8 `job`

## A. 一句话定位 + 边界

**定位：** 拥有 Context-scoped 有界 Worker、Typed Kernel Registry、Job 状态机、取消/超时和 Completion Batch；不运行 C#/delegate，不拥有 Spatial/Codec 实现，不杀线程。

**输入 / 输出 / 所有权：** 输入 OperationId、input lease、output policy、deadline；输出 JobHandle/Completion。JobSystem 是 resource owner；调用方是 handle holder；Memory 是 allocator。

**线程模型：** 多 producer、有界 worker、CAS 状态、pull completion；锁内不执行 kernel，无托管回调。

## B. 内部子模块怎么切

| 单元 | 文件 | 依赖 | 可见性 | 切分理由 |
|---|---|---|---|---|
| ids/ops | `.../id.rs`, `operation.rs` | kernel | crate 外 | runtime binding |
| state | `.../state.rs` | error | crate 内/快照外 | 竞态唯一 |
| cancel | `.../cancel.rs` | platform | crate 外 | 协作取消 |
| queue | `.../queue.rs` | crossbeam adapter | crate 内 | vendor 隔离 |
| scheduler | `.../scheduler.rs` | state+queue+op | crate 外 | submit/poll/cancel |
| worker | `.../worker.rs` | queue+op | crate 内 | 锁外执行 |
| completion | `.../completion.rs` | memory+handle | crate 外 | 一次消费 |
| resource | `.../resource.rs` | context | crate 外 | 七步接入 |

## C. 代码面

```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum JobState { Queued, Running, Succeeded, Failed, Cancelled, TimedOut }
pub struct JobRequest { pub operation: OperationId, pub input: JobInput,
    pub output: JobOutputPolicy, pub deadline: lumio_platform::Deadline }
pub struct JobSystem { _private: () }
impl JobSystem {
    pub fn create(context: std::sync::Arc<lumio_kernel::context::KernelContext>,
        config: JobSystemConfig, registry: std::sync::Arc<dyn OperationRegistry>,
        clock: std::sync::Arc<dyn lumio_platform::MonotonicClock>)
        -> lumio_kernel::error::KernelResult<std::sync::Arc<Self>>;
    pub fn submit(&self, r: JobRequest) -> lumio_kernel::error::KernelResult<JobHandle>;
    pub fn poll(&self, h: JobHandle) -> lumio_kernel::error::KernelResult<JobSnapshot>;
    pub fn cancel(&self, h: JobHandle) -> lumio_kernel::error::KernelResult<CancelOutcome>;
    pub fn drain_completions(&self, out: &mut [JobCompletion])
        -> lumio_kernel::error::KernelResult<usize>;
    pub fn release(&self, h: JobHandle) -> lumio_kernel::error::KernelResult<()>;
}
```

**关键不变式：**

- queue 满立即稳定失败，不无限等待/分配。
- 状态转移服从 `job-state-machine.md`。
- timeout 不 kill thread；迟到结果不可见。
- completion 发布一次，release/complete 不 double-free。
- Registry 拒绝重复 ID。
- ABI 不接收函数指针/delegate。

**失败与稳定类别：** InvalidHandle、WrongContext、CapacityExceeded、CapabilityUnavailable、Cancelled、TimedOut、ContextClosing、kernel propagated category。

**相邻模块接口：** Consumes：KernelContext、Memory、Handle、Clock。Produces：JobSystem、TypedKernel、OperationRegistry、Completion。

## D. 成熟方案选型

| 候选 | 许可证 | 满足 | 不满足 | 裁决 |
|---|---|---|---|---|
| crossbeam-channel | MIT/Apache-2.0 | bounded MPMC | 需 wrapper | 采用候选 |
| rayon | MIT/Apache-2.0 | 成熟池 | Context/state/queue 不匹配 | 排除 |
| tokio | MIT | 异步生态 | 不是 Kernel scheduler | 排除 |
| std sync_channel | std | 依赖少 | 编排能力弱 | fallback |

采用项只能存在于指定 Adapter；供应商类型不得穿过 crate 公共 port 或 C ABI。

## E. 测试与 Fixture

- **unit：** state/registry/queue/completion。
- **concurrency：** cancel/start、cancel/complete、timeout/complete、release/complete、close/submit。
- **fault：** kernel error、shutdown、late result。
- **bench：** submit/queue/execute/drain/RSS/alloc。
- **共同要求：** 固定 baseline/build/target/seed；可由 FakeClock/Interleaving 表达的竞态不得只依赖 `sleep`；bench 报告 throughput、p50/p95/p99/max、alloc、peak bytes、queue depth，时钟读数不进入权威 Hash。

## F. 本模块任务列表

### T-job-01: 定义 Job/Operation IDs 与生成 seam

**Files:**
- Create: `crates/lumio-job/src/id.rs`
- Modify: `crates/lumio-job/src/lib.rs`
- Test: `crates/lumio-job/tests/test_operation_ids_do_not_overlap_generated_range.rs`

**Consumes:** T-contract-types-02, T-error-02
**Produces:** `JobId`, `OperationId`
**成熟方案:** 生成 Registry seam

**步骤（TDD）:**
1. 先写失败测试 `test_operation_ids_do_not_overlap_generated_range`，固定输入、预期状态/错误类别与 owner/handle/allocator 计数。
2. 写只覆盖本任务的最小实现；不得扩大稳定 API、写公共数值或绕过 Adapter。
3. 运行 `cargo test -p lumio-job test_operation_ids_do_not_overlap_generated_range`、`cargo xtask check-dep-dag`；期望测试和依赖 Gate 同时通过。

**验收:**
- [ ] `crates/lumio-job/src/id.rs` 存在，`lib.rs` 只增加必要声明/re-export。
- [ ] `test_operation_ids_do_not_overlap_generated_range` 在实现前失败、实现后可重复通过，不靠 wall-clock sleep 碰运气。
- [ ] ``JobId`, `OperationId`` 不泄漏第三方类型或未批准公共数值。
- [ ] `cargo fmt --check`、目标 crate tests、`xtask check-dep-dag` 通过。

**依赖:** T-contract-types-02, T-error-02
**Blocked:** Operation Registry

### T-job-02: 实现 CancellationSource/View

**Files:**
- Create: `crates/lumio-job/src/cancel.rs`
- Modify: `crates/lumio-job/src/lib.rs`
- Test: `crates/lumio-job/tests/cancel_is_monotonic.rs`

**Consumes:** T-platform-01
**Produces:** cancel types
**成熟方案:** std atomics

**步骤（TDD）:**
1. 先写失败测试 `cancel_is_monotonic`，固定输入、预期状态/错误类别与 owner/handle/allocator 计数。
2. 写只覆盖本任务的最小实现；不得扩大稳定 API、写公共数值或绕过 Adapter。
3. 运行 `cargo test -p lumio-job cancel_is_monotonic`、`cargo xtask check-dep-dag`；期望测试和依赖 Gate 同时通过。

**验收:**
- [ ] `crates/lumio-job/src/cancel.rs` 存在，`lib.rs` 只增加必要声明/re-export。
- [ ] `cancel_is_monotonic` 在实现前失败、实现后可重复通过，不靠 wall-clock sleep 碰运气。
- [ ] `cancel types` 不泄漏第三方类型或未批准公共数值。
- [ ] `cargo fmt --check`、目标 crate tests、`xtask check-dep-dag` 通过。

**依赖:** T-platform-01
**Blocked:** 无

### T-job-03: 实现 CAS JobStateMachine

**Files:**
- Create: `crates/lumio-job/src/state.rs`
- Modify: `crates/lumio-job/src/lib.rs`
- Test: `crates/lumio-job/tests/every_legal_transition_has_single_winner.rs`

**Consumes:** T-job-02, T-test-support-02
**Produces:** `JobStateCell`
**成熟方案:** std atomics+loom

**步骤（TDD）:**
1. 先写失败测试 `every_legal_transition_has_single_winner`，固定输入、预期状态/错误类别与 owner/handle/allocator 计数。
2. 写只覆盖本任务的最小实现；不得扩大稳定 API、写公共数值或绕过 Adapter。
3. 运行 `cargo test -p lumio-job every_legal_transition_has_single_winner`、`cargo xtask check-dep-dag`；期望测试和依赖 Gate 同时通过。

**验收:**
- [ ] `crates/lumio-job/src/state.rs` 存在，`lib.rs` 只增加必要声明/re-export。
- [ ] `every_legal_transition_has_single_winner` 在实现前失败、实现后可重复通过，不靠 wall-clock sleep 碰运气。
- [ ] ``JobStateCell`` 不泄漏第三方类型或未批准公共数值。
- [ ] `cargo fmt --check`、目标 crate tests、`xtask check-dep-dag` 通过。

**依赖:** T-job-02, T-test-support-02
**Blocked:** state names 与 spec

### T-job-04: 实现 TypedKernel/OperationRegistry

**Files:**
- Create: `crates/lumio-job/src/operation.rs`
- Modify: `crates/lumio-job/src/lib.rs`
- Test: `crates/lumio-job/tests/duplicate_operation_id_is_rejected.rs`

**Consumes:** T-job-01, T-memory-01
**Produces:** `TypedKernel`, Registry
**成熟方案:** std Arc/map

**步骤（TDD）:**
1. 先写失败测试 `duplicate_operation_id_is_rejected`，固定输入、预期状态/错误类别与 owner/handle/allocator 计数。
2. 写只覆盖本任务的最小实现；不得扩大稳定 API、写公共数值或绕过 Adapter。
3. 运行 `cargo test -p lumio-job duplicate_operation_id_is_rejected`、`cargo xtask check-dep-dag`；期望测试和依赖 Gate 同时通过。

**验收:**
- [ ] `crates/lumio-job/src/operation.rs` 存在，`lib.rs` 只增加必要声明/re-export。
- [ ] `duplicate_operation_id_is_rejected` 在实现前失败、实现后可重复通过，不靠 wall-clock sleep 碰运气。
- [ ] ``TypedKernel`, Registry` 不泄漏第三方类型或未批准公共数值。
- [ ] `cargo fmt --check`、目标 crate tests、`xtask check-dep-dag` 通过。

**依赖:** T-job-01, T-memory-01
**Blocked:** Operation Registry

### T-job-05: 封装 bounded crossbeam 队列

**Files:**
- Create: `crates/lumio-job/src/queue.rs`
- Modify: `crates/lumio-job/src/lib.rs`
- Test: `crates/lumio-job/tests/queue_full_returns_immediately.rs`

**Consumes:** T-job-03
**Produces:** job/completion queue adapters
**成熟方案:** crossbeam-channel

**步骤（TDD）:**
1. 先写失败测试 `queue_full_returns_immediately`，固定输入、预期状态/错误类别与 owner/handle/allocator 计数。
2. 写只覆盖本任务的最小实现；不得扩大稳定 API、写公共数值或绕过 Adapter。
3. 运行 `cargo test -p lumio-job queue_full_returns_immediately`、`cargo xtask check-dep-dag`；期望测试和依赖 Gate 同时通过。

**验收:**
- [ ] `crates/lumio-job/src/queue.rs` 存在，`lib.rs` 只增加必要声明/re-export。
- [ ] `queue_full_returns_immediately` 在实现前失败、实现后可重复通过，不靠 wall-clock sleep 碰运气。
- [ ] `job/completion queue adapters` 不泄漏第三方类型或未批准公共数值。
- [ ] `cargo fmt --check`、目标 crate tests、`xtask check-dep-dag` 通过。

**依赖:** T-job-03
**Blocked:** supplier approval

### T-job-06: 实现 Worker 与 Scheduler

**Files:**
- Create: `crates/lumio-job/src/worker.rs`
- Modify: `crates/lumio-job/src/lib.rs`
- Test: `crates/lumio-job/tests/worker_never_executes_under_scheduler_lock.rs`

**Consumes:** T-job-04/05, T-context-03
**Produces:** `JobSystem::create/submit/poll/cancel`
**成熟方案:** 具名 std threads+adapter

**步骤（TDD）:**
1. 先写失败测试 `worker_never_executes_under_scheduler_lock`，固定输入、预期状态/错误类别与 owner/handle/allocator 计数。
2. 写只覆盖本任务的最小实现；不得扩大稳定 API、写公共数值或绕过 Adapter。
3. 运行 `cargo test -p lumio-job worker_never_executes_under_scheduler_lock`、`cargo xtask check-dep-dag`；期望测试和依赖 Gate 同时通过。

**验收:**
- [ ] `crates/lumio-job/src/worker.rs` 存在，`lib.rs` 只增加必要声明/re-export。
- [ ] `worker_never_executes_under_scheduler_lock` 在实现前失败、实现后可重复通过，不靠 wall-clock sleep 碰运气。
- [ ] ``JobSystem::create/submit/poll/cancel`` 不泄漏第三方类型或未批准公共数值。
- [ ] `cargo fmt --check`、目标 crate tests、`xtask check-dep-dag` 通过。

**依赖:** T-job-04, T-job-05, T-context-03
**Blocked:** 无

### T-job-07: 实现 CompletionBatch/Release

**Files:**
- Create: `crates/lumio-job/src/completion.rs`
- Modify: `crates/lumio-job/src/lib.rs`
- Test: `crates/lumio-job/tests/completion_is_published_and_released_once.rs`

**Consumes:** T-job-06, T-memory-04
**Produces:** completion/drain/release
**成熟方案:** 本章 D

**步骤（TDD）:**
1. 先写失败测试 `completion_is_published_and_released_once`，固定输入、预期状态/错误类别与 owner/handle/allocator 计数。
2. 写只覆盖本任务的最小实现；不得扩大稳定 API、写公共数值或绕过 Adapter。
3. 运行 `cargo test -p lumio-job completion_is_published_and_released_once`、`cargo xtask check-dep-dag`；期望测试和依赖 Gate 同时通过。

**验收:**
- [ ] `crates/lumio-job/src/completion.rs` 存在，`lib.rs` 只增加必要声明/re-export。
- [ ] `completion_is_published_and_released_once` 在实现前失败、实现后可重复通过，不靠 wall-clock sleep 碰运气。
- [ ] `completion/drain/release` 不泄漏第三方类型或未批准公共数值。
- [ ] `cargo fmt --check`、目标 crate tests、`xtask check-dep-dag` 通过。

**依赖:** T-job-06, T-memory-04
**Blocked:** 无

### T-job-08: 实现全部竞态/超时/关闭矩阵

**Files:**
- Create: `crates/lumio-job/tests/job_state_machine.rs`
- Modify: `crates/lumio-job/src/lib.rs`
- Test: `crates/lumio-job/tests/job_state_machine.rs`

**Consumes:** T-job-07, T-context-05, T-test-support-01/02
**Produces:** Job conformance suite
**成熟方案:** loom+FakeClock

**步骤（TDD）:**
1. 先写失败测试 `timeout_vs_complete_matches_spec_winner`，固定输入、预期状态/错误类别与 owner/handle/allocator 计数。
2. 写只覆盖本任务的最小实现；不得扩大稳定 API、写公共数值或绕过 Adapter。
3. 运行 `cargo test -p lumio-job timeout_vs_complete_matches_spec_winner`、`cargo xtask check-dep-dag`；期望测试和依赖 Gate 同时通过。

**验收:**
- [ ] `crates/lumio-job/tests/job_state_machine.rs` 存在，`lib.rs` 只增加必要声明/re-export。
- [ ] `timeout_vs_complete_matches_spec_winner` 在实现前失败、实现后可重复通过，不靠 wall-clock sleep 碰运气。
- [ ] `Job conformance suite` 不泄漏第三方类型或未批准公共数值。
- [ ] `cargo fmt --check`、目标 crate tests、`xtask check-dep-dag` 通过。

**依赖:** T-job-07, T-context-05, T-test-support-01, T-test-support-02
**Blocked:** 架构 Fixture

---

# 2.9 `spatial`

## A. 一句话定位 + 边界

**定位：** 提供领域无关 2D/3D AABB/object index、邻域/交叠/nearest batch 与确定性排序；不拥有 Voxel/Entity/Player 语义，不做碰撞响应、积分或导航策略。

**输入 / 输出 / 所有权：** 输入有限坐标、稳定 ObjectId、批量 update/query 与 output buffer；输出稳定排序 hits。SpatialContext 是 resource owner。

**线程模型：** 查询可并发读，更新短写锁/串行化；不依赖 job、不回调。

## B. 内部子模块怎么切

| 单元 | 文件 | 依赖 | 可见性 | 切分理由 |
|---|---|---|---|---|
| types | `.../types.rs` | kernel | crate 外 | vendor-free POD |
| validation | `.../validation.rs` | types+error | crate 内 | NaN/bounds 一处 |
| backend | `.../index/mod.rs` | types | crate 内 seam | adapter 可替换 |
| rstar | `.../index/rstar_adapter.rs` | vendor | crate 内 | 第三方隔离 |
| reference | `.../index/grid_reference.rs` | types | tests/可选 | differential oracle |
| query | `.../query.rs` | backend+memory | crate 外 | batch/sort/sizing |
| resource | `.../resource.rs` | context | crate 外 | 关闭 |

## C. 代码面

```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct SpatialObjectId(u64);
#[derive(Clone, Copy, Debug, PartialEq)] pub struct Point3 { pub x:f32,pub y:f32,pub z:f32 }
#[derive(Clone, Copy, Debug, PartialEq)] pub struct Aabb3 { pub min:Point3,pub max:Point3 }
pub struct SpatialContext { _private: () }
impl SpatialContext {
    pub fn create(context: std::sync::Arc<lumio_kernel::context::KernelContext>,
        config: SpatialConfig) -> lumio_kernel::error::KernelResult<std::sync::Arc<Self>>;
    pub fn upsert_batch(&self, items:&[SpatialItem]) -> lumio_kernel::error::KernelResult<SpatialUpdateReport>;
    pub fn remove_batch(&self, ids:&[SpatialObjectId]) -> lumio_kernel::error::KernelResult<SpatialUpdateReport>;
    pub fn query_aabb_batch(&self, q:&[AabbQuery], out:&mut [SpatialHit])
        -> lumio_kernel::error::KernelResult<BatchWriteReport>;
    pub fn nearest_batch(&self, q:&[NearestQuery], out:&mut [SpatialHit])
        -> lumio_kernel::error::KernelResult<BatchWriteReport>;
}
```

**关键不变式：**

- 拒绝 NaN/Inf/反向 bounds/超预算 batch。
- 输出按 `(query_ordinal,distance_key,object_id)` 稳定排序。
- 插入/线程时序不影响结果序列。
- output 不足返回 required，不越界。
- destroy 后 index handle 失效。

**失败与稳定类别：** InvalidArgument、BufferTooSmall、CapacityExceeded、InvalidHandle、WrongContext、ContextClosing。

**相邻模块接口：** Consumes：Kernel Context/Memory/Error。Produces：SpatialContext/batch types；Job adapter 位于 FFI。

## D. 成熟方案选型

| 候选 | 许可证 | 满足 | 不满足 | 裁决 |
|---|---|---|---|---|
| rstar | MIT/Apache-2.0 | 动态 R-tree/AABB | 原生迭代序非契约 | 采用+stable sort |
| kiddo | MIT/Apache-2.0 | point nearest | 非动态 AABB 首选 | 不采用首期 |
| parry/ncollide | Apache/BSD | 碰撞宽相 | 依赖重/response 越界 | I2 |
| grid reference | 本仓 | 确定性 oracle | 性能未必最佳 | 最小自研+benchmark |

采用自研部分只限现成方案无法满足的最小语义；实现前必须提交 ADR 候选表，含维护责任、benchmark、退出 seam。

## E. 测试与 Fixture

- **unit：** validation/tie sort/sizing/idempotence。
- **concurrency：** query/update、close/query、seeded oracle。
- **fault：** NaN、duplicates、capacity、destroyed、undersized。
- **bench：** update/query/RSS/bytes/object/sort cost。
- **共同要求：** 固定 baseline/build/target/seed；可由 FakeClock/Interleaving 表达的竞态不得只依赖 `sleep`；bench 报告 throughput、p50/p95/p99/max、alloc、peak bytes、queue depth，时钟读数不进入权威 Hash。

## F. 本模块任务列表

### T-spatial-01: 定义 vendor-free types/validation

**Files:**
- Create: `crates/lumio-spatial/src/types.rs`
- Modify: `crates/lumio-spatial/src/lib.rs`
- Test: `crates/lumio-spatial/tests/non_finite_coordinates_are_rejected.rs`

**Consumes:** T-error-02, T-memory-01
**Produces:** Spatial POD/query types
**成熟方案:** std

**步骤（TDD）:**
1. 先写失败测试 `non_finite_coordinates_are_rejected`，固定输入、预期状态/错误类别与 owner/handle/allocator 计数。
2. 写只覆盖本任务的最小实现；不得扩大稳定 API、写公共数值或绕过 Adapter。
3. 运行 `cargo test -p lumio-spatial non_finite_coordinates_are_rejected`、`cargo xtask check-dep-dag`；期望测试和依赖 Gate 同时通过。

**验收:**
- [ ] `crates/lumio-spatial/src/types.rs` 存在，`lib.rs` 只增加必要声明/re-export。
- [ ] `non_finite_coordinates_are_rejected` 在实现前失败、实现后可重复通过，不靠 wall-clock sleep 碰运气。
- [ ] `Spatial POD/query types` 不泄漏第三方类型或未批准公共数值。
- [ ] `cargo fmt --check`、目标 crate tests、`xtask check-dep-dag` 通过。

**依赖:** T-error-02, T-memory-01
**Blocked:** 无

### T-spatial-02: 定义 backend seam

**Files:**
- Create: `crates/lumio-spatial/src/index/mod.rs`
- Modify: `crates/lumio-spatial/src/lib.rs`
- Test: `crates/lumio-spatial/tests/backend_trait_does_not_expose_vendor_types.rs`

**Consumes:** T-spatial-01
**Produces:** `SpatialIndexBackend`
**成熟方案:** trait adapter

**步骤（TDD）:**
1. 先写失败测试 `backend_trait_does_not_expose_vendor_types`，固定输入、预期状态/错误类别与 owner/handle/allocator 计数。
2. 写只覆盖本任务的最小实现；不得扩大稳定 API、写公共数值或绕过 Adapter。
3. 运行 `cargo test -p lumio-spatial backend_trait_does_not_expose_vendor_types`、`cargo xtask check-dep-dag`；期望测试和依赖 Gate 同时通过。

**验收:**
- [ ] `crates/lumio-spatial/src/index/mod.rs` 存在，`lib.rs` 只增加必要声明/re-export。
- [ ] `backend_trait_does_not_expose_vendor_types` 在实现前失败、实现后可重复通过，不靠 wall-clock sleep 碰运气。
- [ ] ``SpatialIndexBackend`` 不泄漏第三方类型或未批准公共数值。
- [ ] `cargo fmt --check`、目标 crate tests、`xtask check-dep-dag` 通过。

**依赖:** T-spatial-01
**Blocked:** 无

### T-spatial-03: 实现确定性 grid reference

**Files:**
- Create: `crates/lumio-spatial/src/index/grid_reference.rs`
- Modify: `crates/lumio-spatial/src/lib.rs`
- Test: `crates/lumio-spatial/tests/reference_results_are_stably_sorted.rs`

**Consumes:** T-spatial-02
**Produces:** `GridReferenceIndex`
**成熟方案:** 最小自研，见 D

**步骤（TDD）:**
1. 先写失败测试 `reference_results_are_stably_sorted`，固定输入、预期状态/错误类别与 owner/handle/allocator 计数。
2. 写只覆盖本任务的最小实现；不得扩大稳定 API、写公共数值或绕过 Adapter。
3. 运行 `cargo test -p lumio-spatial reference_results_are_stably_sorted`、`cargo xtask check-dep-dag`；期望测试和依赖 Gate 同时通过。

**验收:**
- [ ] `crates/lumio-spatial/src/index/grid_reference.rs` 存在，`lib.rs` 只增加必要声明/re-export。
- [ ] `reference_results_are_stably_sorted` 在实现前失败、实现后可重复通过，不靠 wall-clock sleep 碰运气。
- [ ] ``GridReferenceIndex`` 不泄漏第三方类型或未批准公共数值。
- [ ] `cargo fmt --check`、目标 crate tests、`xtask check-dep-dag` 通过。

**依赖:** T-spatial-02
**Blocked:** ADR 候选表

### T-spatial-04: 实现 rstar adapter

**Files:**
- Create: `crates/lumio-spatial/src/index/rstar_adapter.rs`
- Modify: `crates/lumio-spatial/src/lib.rs`
- Test: `crates/lumio-spatial/tests/rstar_matches_reference_corpus.rs`

**Consumes:** T-spatial-03
**Produces:** `RStarIndexAdapter`
**成熟方案:** rstar adapter

**步骤（TDD）:**
1. 先写失败测试 `rstar_matches_reference_corpus`，固定输入、预期状态/错误类别与 owner/handle/allocator 计数。
2. 写只覆盖本任务的最小实现；不得扩大稳定 API、写公共数值或绕过 Adapter。
3. 运行 `cargo test -p lumio-spatial rstar_matches_reference_corpus`、`cargo xtask check-dep-dag`；期望测试和依赖 Gate 同时通过。

**验收:**
- [ ] `crates/lumio-spatial/src/index/rstar_adapter.rs` 存在，`lib.rs` 只增加必要声明/re-export。
- [ ] `rstar_matches_reference_corpus` 在实现前失败、实现后可重复通过，不靠 wall-clock sleep 碰运气。
- [ ] ``RStarIndexAdapter`` 不泄漏第三方类型或未批准公共数值。
- [ ] `cargo fmt --check`、目标 crate tests、`xtask check-dep-dag` 通过。

**依赖:** T-spatial-03
**Blocked:** rstar supplier approval

### T-spatial-05: 实现 batch query/output sizing

**Files:**
- Create: `crates/lumio-spatial/src/query.rs`
- Modify: `crates/lumio-spatial/src/lib.rs`
- Test: `crates/lumio-spatial/tests/undersized_output_reports_required_without_overwrite.rs`

**Consumes:** T-spatial-04, T-memory-01
**Produces:** SpatialContext batch APIs
**成熟方案:** 本章 D

**步骤（TDD）:**
1. 先写失败测试 `undersized_output_reports_required_without_overwrite`，固定输入、预期状态/错误类别与 owner/handle/allocator 计数。
2. 写只覆盖本任务的最小实现；不得扩大稳定 API、写公共数值或绕过 Adapter。
3. 运行 `cargo test -p lumio-spatial undersized_output_reports_required_without_overwrite`、`cargo xtask check-dep-dag`；期望测试和依赖 Gate 同时通过。

**验收:**
- [ ] `crates/lumio-spatial/src/query.rs` 存在，`lib.rs` 只增加必要声明/re-export。
- [ ] `undersized_output_reports_required_without_overwrite` 在实现前失败、实现后可重复通过，不靠 wall-clock sleep 碰运气。
- [ ] `SpatialContext batch APIs` 不泄漏第三方类型或未批准公共数值。
- [ ] `cargo fmt --check`、目标 crate tests、`xtask check-dep-dag` 通过。

**依赖:** T-spatial-04, T-memory-01
**Blocked:** 无

### T-spatial-06: 接入 ContextResource/differential/bench

**Files:**
- Create: `crates/lumio-spatial/src/resource.rs`
- Modify: `crates/lumio-spatial/src/lib.rs`
- Test: `crates/lumio-spatial/tests/destroyed_spatial_context_rejects_late_query.rs`

**Consumes:** T-spatial-05, T-context-05, T-test-support-02
**Produces:** Spatial resource/I1 suite
**成熟方案:** rstar+reference

**步骤（TDD）:**
1. 先写失败测试 `destroyed_spatial_context_rejects_late_query`，固定输入、预期状态/错误类别与 owner/handle/allocator 计数。
2. 写只覆盖本任务的最小实现；不得扩大稳定 API、写公共数值或绕过 Adapter。
3. 运行 `cargo test -p lumio-spatial destroyed_spatial_context_rejects_late_query`、`cargo xtask check-dep-dag`；期望测试和依赖 Gate 同时通过。

**验收:**
- [ ] `crates/lumio-spatial/src/resource.rs` 存在，`lib.rs` 只增加必要声明/re-export。
- [ ] `destroyed_spatial_context_rejects_late_query` 在实现前失败、实现后可重复通过，不靠 wall-clock sleep 碰运气。
- [ ] `Spatial resource/I1 suite` 不泄漏第三方类型或未批准公共数值。
- [ ] `cargo fmt --check`、目标 crate tests、`xtask check-dep-dag` 通过。

**依赖:** T-spatial-05, T-context-05, T-test-support-02
**Blocked:** 无

---

# 2.10 `codec`

## A. 一句话定位 + 边界

**定位：** 仅提供领域无关 byte compression/decompression/checksum/diff 私有 prototype；不定义 Voxel/RPC/Persistence 格式、Schema、Revision 或公共算法 ID。

**输入 / 输出 / 所有权：** 输入 BorrowedCallBuffer、private options、CallerOutput/Native output policy；输出机械 bytes/report。Codec workspace 是 ContextResource。

**线程模型：** workspace 采用独占租约或池；不依赖 job；所有输出和解压均有界。

## B. 内部子模块怎么切

| 单元 | 文件 | 依赖 | 可见性 | 切分理由 |
|---|---|---|---|---|
| bounds | `.../bounds.rs` | kernel | 私有 feature 外可见 | zip bomb 防线 |
| options | `.../options.rs` | 无 | 私有 feature | 不使用公共 ID |
| compression | `.../compression/*` | vendor | crate 内 | 供应商隔离 |
| checksum | `.../checksum.rs` | vendor | crate 内 | 安全语义分开 |
| diff | `.../diff.rs` | memory | 私有 feature | reference first |
| resource | `.../resource.rs` | context | crate 外私有 | workspace close |

## C. 代码面

```rust
#![cfg(feature = "private-codec-prototype")]
pub enum PrivateCompressionAlgorithm { ZstdPrototype, Lz4Prototype }
pub struct CodecLimits { pub max_input_bytes:u64, pub max_output_bytes:u64,
    pub max_expansion_ratio:u32 }
pub struct CodecService { _private: () }
impl CodecService {
    pub fn compress(&self, alg:PrivateCompressionAlgorithm,
        input:lumio_kernel::memory::BorrowedCallBuffer<'_>,
        output:&mut lumio_kernel::memory::CallerOutputBuffer<'_>)
        -> lumio_kernel::error::KernelResult<CodecReport>;
    pub fn decompress_bounded(&self, alg:PrivateCompressionAlgorithm,
        input:lumio_kernel::memory::BorrowedCallBuffer<'_>, expected_max:usize,
        output:&mut lumio_kernel::memory::CallerOutputBuffer<'_>)
        -> lumio_kernel::error::KernelResult<CodecReport>;
    pub fn checksum(&self, input:lumio_kernel::memory::BorrowedCallBuffer<'_>,
        output:&mut [u8]) -> lumio_kernel::error::KernelResult<usize>;
}
```

**关键不变式：**

- default features 不编译 vendor adapter/exports。
- 解压前与过程中均执行 output/ratio 上限。
- 算法/level/checksum 不映射公共 ID。
- vendor error 只映射 KernelError，不外泄。

**失败与稳定类别：** InvalidArgument、BufferTooSmall、CapacityExceeded、CapabilityUnavailable、InternalInvariant；损坏输入的公共类别待架构源。

**相邻模块接口：** Consumes：Kernel Memory/Context/Error。Produces：private CodecService；Job adapter 位于 FFI private feature。

## D. 成熟方案选型

| 候选 | 许可证 | 满足 | 不满足 | 裁决 |
|---|---|---|---|---|
| zstd | BSD-3-Clause | 成熟高压缩比 | native build/移动供应链 | 私有候选 |
| lz4_flex | MIT | 纯 Rust/快速 | 格式未批准 | 私有候选 |
| flate2 | MIT/Apache-2.0 | 兼容性 | 首期无 workload 优势 | 不采用 |
| xdelta/bsdiff crates | 多种 | 成熟思想 | 维护/许可证/内存边界不统一 | reference+benchmark |

采用项只能存在于指定 Adapter；供应商类型不得穿过 crate 公共 port 或 C ABI。

## E. 测试与 Fixture

- **unit：** roundtrip/bounds/checksum/diff。
- **concurrency：** workspace exhaustion、close/running。
- **fault：** corrupt/truncated/zip bomb/undersized。
- **bench：** MB/s/ratio/p99/RSS/corpus hash。
- **共同要求：** 固定 baseline/build/target/seed；可由 FakeClock/Interleaving 表达的竞态不得只依赖 `sleep`；bench 报告 throughput、p50/p95/p99/max、alloc、peak bytes、queue depth，时钟读数不进入权威 Hash。

## F. 本模块任务列表

### T-codec-01: 建立 private feature 与 limits

**Files:**
- Create: `crates/lumio-codec/src/bounds.rs`
- Modify: `crates/lumio-codec/src/lib.rs`
- Test: `crates/lumio-codec/tests/default_build_has_no_codec_vendor_dependencies.rs`

**Consumes:** T-error-02, T-memory-01
**Produces:** `CodecLimits`, feature gate
**成熟方案:** std

**步骤（TDD）:**
1. 先写失败测试 `default_build_has_no_codec_vendor_dependencies`，固定输入、预期状态/错误类别与 owner/handle/allocator 计数。
2. 写只覆盖本任务的最小实现；不得扩大稳定 API、写公共数值或绕过 Adapter。
3. 运行 `cargo test -p lumio-codec default_build_has_no_codec_vendor_dependencies`、`cargo xtask check-dep-dag`；期望测试和依赖 Gate 同时通过。

**验收:**
- [ ] `crates/lumio-codec/src/bounds.rs` 存在，`lib.rs` 只增加必要声明/re-export。
- [ ] `default_build_has_no_codec_vendor_dependencies` 在实现前失败、实现后可重复通过，不靠 wall-clock sleep 碰运气。
- [ ] ``CodecLimits`, feature gate` 不泄漏第三方类型或未批准公共数值。
- [ ] `cargo fmt --check`、目标 crate tests、`xtask check-dep-dag` 通过。
- [ ] default features 不启用私有 prototype 依赖或符号。

**依赖:** T-error-02, T-memory-01
**Blocked:** ADR 0005/public status

### T-codec-02: 实现 zstd bounded adapter

**Files:**
- Create: `crates/lumio-codec/src/compression/zstd_adapter.rs`
- Modify: `crates/lumio-codec/src/lib.rs`
- Test: `crates/lumio-codec/tests/zstd_decompress_rejects_expansion_limit.rs`

**Consumes:** T-codec-01
**Produces:** private `ZstdAdapter`
**成熟方案:** zstd adapter

**步骤（TDD）:**
1. 先写失败测试 `zstd_decompress_rejects_expansion_limit`，固定输入、预期状态/错误类别与 owner/handle/allocator 计数。
2. 写只覆盖本任务的最小实现；不得扩大稳定 API、写公共数值或绕过 Adapter。
3. 运行 `cargo test -p lumio-codec zstd_decompress_rejects_expansion_limit`、`cargo xtask check-dep-dag`；期望测试和依赖 Gate 同时通过。

**验收:**
- [ ] `crates/lumio-codec/src/compression/zstd_adapter.rs` 存在，`lib.rs` 只增加必要声明/re-export。
- [ ] `zstd_decompress_rejects_expansion_limit` 在实现前失败、实现后可重复通过，不靠 wall-clock sleep 碰运气。
- [ ] `private `ZstdAdapter`` 不泄漏第三方类型或未批准公共数值。
- [ ] `cargo fmt --check`、目标 crate tests、`xtask check-dep-dag` 通过。
- [ ] default features 不启用私有 prototype 依赖或符号。

**依赖:** T-codec-01
**Blocked:** zstd supplier approval

### T-codec-03: 实现 lz4 bounded adapter

**Files:**
- Create: `crates/lumio-codec/src/compression/lz4_adapter.rs`
- Modify: `crates/lumio-codec/src/lib.rs`
- Test: `crates/lumio-codec/tests/lz4_decompress_rejects_truncated_input.rs`

**Consumes:** T-codec-01
**Produces:** private `Lz4Adapter`
**成熟方案:** lz4_flex adapter

**步骤（TDD）:**
1. 先写失败测试 `lz4_decompress_rejects_truncated_input`，固定输入、预期状态/错误类别与 owner/handle/allocator 计数。
2. 写只覆盖本任务的最小实现；不得扩大稳定 API、写公共数值或绕过 Adapter。
3. 运行 `cargo test -p lumio-codec lz4_decompress_rejects_truncated_input`、`cargo xtask check-dep-dag`；期望测试和依赖 Gate 同时通过。

**验收:**
- [ ] `crates/lumio-codec/src/compression/lz4_adapter.rs` 存在，`lib.rs` 只增加必要声明/re-export。
- [ ] `lz4_decompress_rejects_truncated_input` 在实现前失败、实现后可重复通过，不靠 wall-clock sleep 碰运气。
- [ ] `private `Lz4Adapter`` 不泄漏第三方类型或未批准公共数值。
- [ ] `cargo fmt --check`、目标 crate tests、`xtask check-dep-dag` 通过。
- [ ] default features 不启用私有 prototype 依赖或符号。

**依赖:** T-codec-01
**Blocked:** lz4 supplier approval

### T-codec-04: 实现 checksum/diff reference

**Files:**
- Create: `crates/lumio-codec/src/checksum.rs`
- Modify: `crates/lumio-codec/src/lib.rs`
- Test: `crates/lumio-codec/tests/checksum_vectors_are_stable.rs`

**Consumes:** T-codec-01
**Produces:** private checksum/diff APIs
**成熟方案:** xxhash/blake3 adapters

**步骤（TDD）:**
1. 先写失败测试 `checksum_vectors_are_stable`，固定输入、预期状态/错误类别与 owner/handle/allocator 计数。
2. 写只覆盖本任务的最小实现；不得扩大稳定 API、写公共数值或绕过 Adapter。
3. 运行 `cargo test -p lumio-codec checksum_vectors_are_stable`、`cargo xtask check-dep-dag`；期望测试和依赖 Gate 同时通过。

**验收:**
- [ ] `crates/lumio-codec/src/checksum.rs` 存在，`lib.rs` 只增加必要声明/re-export。
- [ ] `checksum_vectors_are_stable` 在实现前失败、实现后可重复通过，不靠 wall-clock sleep 碰运气。
- [ ] `private checksum/diff APIs` 不泄漏第三方类型或未批准公共数值。
- [ ] `cargo fmt --check`、目标 crate tests、`xtask check-dep-dag` 通过。
- [ ] default features 不启用私有 prototype 依赖或符号。

**依赖:** T-codec-01
**Blocked:** 算法 ID 不公开

### T-codec-05: 接入 ContextResource/corpus/bench

**Files:**
- Create: `crates/lumio-codec/src/resource.rs`
- Modify: `crates/lumio-codec/src/lib.rs`
- Test: `crates/lumio-codec/tests/codec_workspace_is_reclaimed_on_close.rs`

**Consumes:** T-codec-02/03/04, T-context-05
**Produces:** private CodecService suite
**成熟方案:** 本章 D

**步骤（TDD）:**
1. 先写失败测试 `codec_workspace_is_reclaimed_on_close`，固定输入、预期状态/错误类别与 owner/handle/allocator 计数。
2. 写只覆盖本任务的最小实现；不得扩大稳定 API、写公共数值或绕过 Adapter。
3. 运行 `cargo test -p lumio-codec codec_workspace_is_reclaimed_on_close`、`cargo xtask check-dep-dag`；期望测试和依赖 Gate 同时通过。

**验收:**
- [ ] `crates/lumio-codec/src/resource.rs` 存在，`lib.rs` 只增加必要声明/re-export。
- [ ] `codec_workspace_is_reclaimed_on_close` 在实现前失败、实现后可重复通过，不靠 wall-clock sleep 碰运气。
- [ ] `private CodecService suite` 不泄漏第三方类型或未批准公共数值。
- [ ] `cargo fmt --check`、目标 crate tests、`xtask check-dep-dag` 通过。
- [ ] default features 不启用私有 prototype 依赖或符号。

**依赖:** T-codec-02, T-codec-03, T-codec-04, T-context-05
**Blocked:** 公共 codec contract

---

# 2.11 `diagnostics`

## A. 一句话定位 + 边界

**定位：** 实现 Kernel `RecordPort` 的有界 non-blocking recorder、drop counters、pull drain 与可选 tracing bridge；不拥有 Sink、日志文件、审计、事务日志或错误码。

**输入 / 输出 / 所有权：** 输入 borrowed KernelRecordRef；输出 owned bounded batch 供上层 pull。DiagnosticsContext 是 resource owner，上层拥有 Sink。

**线程模型：** producer `try_record` 不阻塞；受控 consumer drain；满载丢弃并计数。

## B. 内部子模块怎么切

| 单元 | 文件 | 依赖 | 可见性 | 切分理由 |
|---|---|---|---|---|
| owned record | `.../record.rs` | kernel record | crate 内 | 复制上限 |
| queue | `.../queue.rs` | crossbeam-queue | crate 内 | vendor 隔离 |
| recorder | `.../recorder.rs` | queue | 私有 feature 外 | RecordPort 实现 |
| drain | `.../drain.rs` | queue+memory | 私有 feature 外 | pull only |
| tracing bridge | `.../tracing_adapter.rs` | tracing | crate 内可选 | 生态边界 |
| resource | `.../resource.rs` | context | crate 外私有 | 关闭 |

## C. 代码面

```rust
#![cfg(feature = "private-diagnostics-prototype")]
pub struct BoundedRecorder { _private: () }
impl BoundedRecorder {
    pub fn with_capacity(capacity:usize, max_record_bytes:usize)
        -> lumio_kernel::error::KernelResult<std::sync::Arc<Self>>;
    pub fn drain(&self, output:&mut [OwnedKernelRecord]) -> DrainReport;
    pub fn counters(&self) -> RecorderCounters;
}
impl lumio_kernel::record::RecordPort for BoundedRecorder {
    fn try_record(&self, r:lumio_kernel::record::KernelRecordRef<'_>)
        -> lumio_kernel::record::RecordDisposition;
}
```

**关键不变式：**

- producer 不等待 consumer/Sink。
- queue full 返回 DroppedFull 且计数。
- 单 record 有 max bytes/fields。
- error mapping 不依赖 recorder。
- core default graph 无 tracing/metrics。

**失败与稳定类别：** 创建参数非法→InvalidArgument；预算不足→CapacityExceeded；热路径返回 disposition，不返回 KernelError。

**相邻模块接口：** Consumes：RecordPort/Context/Memory。Produces：BoundedRecorder/drain batch；上层拥有 Sink。

## D. 成熟方案选型

| 候选 | 许可证 | 满足 | 不满足 | 裁决 |
|---|---|---|---|---|
| tracing | MIT | 成熟生态 | 若进 core 会硬依赖 | 仅 bridge |
| metrics | MIT | metrics facade | global recorder 扩大面 | 不采用 |
| crossbeam ArrayQueue | MIT/Apache-2.0 | bounded non-blocking | 需 owned copy | 候选采用 |
| 自研 ring | 本仓 | 可定制 | 并发维护成本高 | 无阻塞不做 |

采用项只能存在于指定 Adapter；供应商类型不得穿过 crate 公共 port 或 C ABI。

## E. 测试与 Fixture

- **unit：** full/drop counter、size cap、sequence。
- **concurrency：** MP/SC stress、close/record、loom。
- **fault：** budget exhausted、oversized、no consumer。
- **bench：** try_record p99/drop/copy/drain。
- **共同要求：** 固定 baseline/build/target/seed；可由 FakeClock/Interleaving 表达的竞态不得只依赖 `sleep`；bench 报告 throughput、p50/p95/p99/max、alloc、peak bytes、queue depth，时钟读数不进入权威 Hash。

## F. 本模块任务列表

### T-diagnostics-01: 定义 owned bounded record

**Files:**
- Create: `crates/lumio-diagnostics/src/record.rs`
- Modify: `crates/lumio-diagnostics/src/lib.rs`
- Test: `crates/lumio-diagnostics/tests/owned_record_enforces_field_and_byte_limits.rs`

**Consumes:** T-error-02
**Produces:** `OwnedKernelRecord` consuming KernelRecordRef
**成熟方案:** typed owned copy

**步骤（TDD）:**
1. 先写失败测试 `owned_record_enforces_field_and_byte_limits`，固定输入、预期状态/错误类别与 owner/handle/allocator 计数。
2. 写只覆盖本任务的最小实现；不得扩大稳定 API、写公共数值或绕过 Adapter。
3. 运行 `cargo test -p lumio-diagnostics owned_record_enforces_field_and_byte_limits`、`cargo xtask check-dep-dag`；期望测试和依赖 Gate 同时通过。

**验收:**
- [ ] `crates/lumio-diagnostics/src/record.rs` 存在，`lib.rs` 只增加必要声明/re-export。
- [ ] `owned_record_enforces_field_and_byte_limits` 在实现前失败、实现后可重复通过，不靠 wall-clock sleep 碰运气。
- [ ] ``OwnedKernelRecord` consuming KernelRecordRef` 不泄漏第三方类型或未批准公共数值。
- [ ] `cargo fmt --check`、目标 crate tests、`xtask check-dep-dag` 通过。
- [ ] default features 不启用私有 prototype 依赖或符号。

**依赖:** T-error-02
**Blocked:** 公共 record schema pending

### T-diagnostics-02: 实现 bounded queue adapter

**Files:**
- Create: `crates/lumio-diagnostics/src/queue.rs`
- Modify: `crates/lumio-diagnostics/src/lib.rs`
- Test: `crates/lumio-diagnostics/tests/full_queue_never_blocks_producer.rs`

**Consumes:** T-diagnostics-01
**Produces:** `RecordQueue`
**成熟方案:** crossbeam-queue adapter

**步骤（TDD）:**
1. 先写失败测试 `full_queue_never_blocks_producer`，固定输入、预期状态/错误类别与 owner/handle/allocator 计数。
2. 写只覆盖本任务的最小实现；不得扩大稳定 API、写公共数值或绕过 Adapter。
3. 运行 `cargo test -p lumio-diagnostics full_queue_never_blocks_producer`、`cargo xtask check-dep-dag`；期望测试和依赖 Gate 同时通过。

**验收:**
- [ ] `crates/lumio-diagnostics/src/queue.rs` 存在，`lib.rs` 只增加必要声明/re-export。
- [ ] `full_queue_never_blocks_producer` 在实现前失败、实现后可重复通过，不靠 wall-clock sleep 碰运气。
- [ ] ``RecordQueue`` 不泄漏第三方类型或未批准公共数值。
- [ ] `cargo fmt --check`、目标 crate tests、`xtask check-dep-dag` 通过。
- [ ] default features 不启用私有 prototype 依赖或符号。

**依赖:** T-diagnostics-01
**Blocked:** supplier approval

### T-diagnostics-03: 实现 RecordPort recorder/drain

**Files:**
- Create: `crates/lumio-diagnostics/src/recorder.rs`
- Modify: `crates/lumio-diagnostics/src/lib.rs`
- Test: `crates/lumio-diagnostics/tests/drop_counter_matches_rejected_records.rs`

**Consumes:** T-diagnostics-02, T-memory-03
**Produces:** `BoundedRecorder`, DrainReport
**成熟方案:** 本章 D

**步骤（TDD）:**
1. 先写失败测试 `drop_counter_matches_rejected_records`，固定输入、预期状态/错误类别与 owner/handle/allocator 计数。
2. 写只覆盖本任务的最小实现；不得扩大稳定 API、写公共数值或绕过 Adapter。
3. 运行 `cargo test -p lumio-diagnostics drop_counter_matches_rejected_records`、`cargo xtask check-dep-dag`；期望测试和依赖 Gate 同时通过。

**验收:**
- [ ] `crates/lumio-diagnostics/src/recorder.rs` 存在，`lib.rs` 只增加必要声明/re-export。
- [ ] `drop_counter_matches_rejected_records` 在实现前失败、实现后可重复通过，不靠 wall-clock sleep 碰运气。
- [ ] ``BoundedRecorder`, DrainReport` 不泄漏第三方类型或未批准公共数值。
- [ ] `cargo fmt --check`、目标 crate tests、`xtask check-dep-dag` 通过。
- [ ] default features 不启用私有 prototype 依赖或符号。

**依赖:** T-diagnostics-02, T-memory-03
**Blocked:** 无

### T-diagnostics-04: 接入 ContextResource/bridge/stress

**Files:**
- Create: `crates/lumio-diagnostics/src/resource.rs`
- Modify: `crates/lumio-diagnostics/src/lib.rs`
- Test: `crates/lumio-diagnostics/tests/core_default_graph_has_no_diagnostics_dependency.rs`

**Consumes:** T-diagnostics-03, T-context-05, T-test-support-02
**Produces:** private diagnostics suite
**成熟方案:** optional tracing bridge

**步骤（TDD）:**
1. 先写失败测试 `core_default_graph_has_no_diagnostics_dependency`，固定输入、预期状态/错误类别与 owner/handle/allocator 计数。
2. 写只覆盖本任务的最小实现；不得扩大稳定 API、写公共数值或绕过 Adapter。
3. 运行 `cargo test -p lumio-diagnostics core_default_graph_has_no_diagnostics_dependency`、`cargo xtask check-dep-dag`；期望测试和依赖 Gate 同时通过。

**验收:**
- [ ] `crates/lumio-diagnostics/src/resource.rs` 存在，`lib.rs` 只增加必要声明/re-export。
- [ ] `core_default_graph_has_no_diagnostics_dependency` 在实现前失败、实现后可重复通过，不靠 wall-clock sleep 碰运气。
- [ ] `private diagnostics suite` 不泄漏第三方类型或未批准公共数值。
- [ ] `cargo fmt --check`、目标 crate tests、`xtask check-dep-dag` 通过。
- [ ] default features 不启用私有 prototype 依赖或符号。

**依赖:** T-diagnostics-03, T-context-05, T-test-support-02
**Blocked:** ADR 0005/public schema

---

# 2.12 `native-core-ffi`

## A. 一句话定位 + 边界

**定位：** 唯一 C symbol facade，负责 raw 参数/layout/handle/buffer 校验、panic 捕获与 Error 映射；不拥有 Root API `lumio_core_get_api_v1`，不定义公共字段。

**输入 / 输出 / 所有权：** 输入生成 Header 的 POD/opaque/buffer；输出生成 ErrorCode、written/required、opaque handle。调用方拥有输入/CallerOutput；Native-owned 返回 handle。

**线程模型：** 按服务线程契约；boundary 可重入，无全局锁；catch_unwind 只包 extern body。

## B. 内部子模块怎么切

| 单元 | 文件 | 依赖 | 可见性 | 切分理由 |
|---|---|---|---|---|
| boundary | `.../boundary.rs` | kernel error | crate 内 | 统一 panic guard |
| validation | `.../validation.rs` | contract-types | crate 内 | layout/null/alias |
| handles/buffers | `.../handles.rs`, `buffers.rs` | kernel+generated | crate 内 | ownership conversion |
| error | `.../error.rs` | mapping | crate 内 | 单一返回 |
| exports | `.../exports.rs` | 全部 | C public generated-only | 公共面集中 |
| operations | `.../operations/*` | job+service | crate 内 | 打断依赖 |
| symbol guard | `.../symbol_guard.rs` | manifest | tests | 负向 Gate |

## C. 代码面

```rust
pub(crate) fn ffi_boundary<F>(body:F)
    -> lumio_contract_types::ArchitectureErrorCode
where F: FnOnce()->lumio_kernel::error::KernelResult<()> + std::panic::UnwindSafe;

// BLOCKED_ABI: export names/signatures generated from Architecture Header.
#[cfg(feature = "architecture-contracts")]
mod exports;
// Forbidden here: lumio_core_get_api_v1
```

**关键不变式：**

- 所有 extern 经 `ffi_boundary`，panic 不穿 ABI。
- null/length/alignment/size/version 先校验。
- raw handle 必须 decode+expected Context。
- symbol 只含批准 provider exports 且排除 Root。
- Rust/vendor types、bool、usize、slice/String 不出 ABI。

**失败与稳定类别：** KernelError 经唯一 mapping；panic→PanicBoundary；version/layout 类别 BLOCKED_ABI。

**相邻模块接口：** Consumes：selected services+generated types。Produces：C provider symbols、ABI smoke library。

## D. 成熟方案选型

| 候选 | 许可证 | 满足 | 不满足 | 裁决 |
|---|---|---|---|---|
| catch_unwind | Rust std | 标准 boundary | abort profile 需发布策略 | 采用 |
| safer-ffi | MIT/Apache-2.0 | 生成便利 | 与唯一 Header 链冲突 | 不采用公共面 |
| cbindgen | MPL-2.0 | Rust→C | 事实方向相反 | 禁止 |
| 手写 exports | 无 | 快 | 漂移 | 仅生成模板 |

V1 生产路径不新增第三方依赖；标准库、生成物和小型 typed model 已足够，dev dependency 不进入生产 graph。

## E. 测试与 Fixture

- **unit：** raw validation/mapping。
- **concurrency：** parallel smoke、close/call lease。
- **FFI smoke：** C create/query/release/undersized/invalid/panic。
- **fault：** panic/double release/cap missing/wrong Context。
- **bench：** FFI overhead/copy/calls/symbol report。
- **共同要求：** 固定 baseline/build/target/seed；可由 FakeClock/Interleaving 表达的竞态不得只依赖 `sleep`；bench 报告 throughput、p50/p95/p99/max、alloc、peak bytes、queue depth，时钟读数不进入权威 Hash。

## F. 本模块任务列表

### T-ffi-01: 实现统一 panic/error boundary

**Files:**
- Create: `crates/lumio-native-ffi/src/boundary.rs`
- Modify: `crates/lumio-native-ffi/src/lib.rs`
- Test: `crates/lumio-native-ffi/tests/panic_is_converted_and_does_not_unwind.rs`

**Consumes:** T-error-03
**Produces:** `ffi_boundary`
**成熟方案:** std catch_unwind

**步骤（TDD）:**
1. 先写失败测试 `panic_is_converted_and_does_not_unwind`，固定输入、预期状态/错误类别与 owner/handle/allocator 计数。
2. 写只覆盖本任务的最小实现；不得扩大稳定 API、写公共数值或绕过 Adapter。
3. 运行 `cargo test -p lumio-native-ffi panic_is_converted_and_does_not_unwind`、`cargo xtask check-dep-dag`；期望测试和依赖 Gate 同时通过。

**验收:**
- [ ] `crates/lumio-native-ffi/src/boundary.rs` 存在，`lib.rs` 只增加必要声明/re-export。
- [ ] `panic_is_converted_and_does_not_unwind` 在实现前失败、实现后可重复通过，不靠 wall-clock sleep 碰运气。
- [ ] ``ffi_boundary`` 不泄漏第三方类型或未批准公共数值。
- [ ] `cargo fmt --check`、目标 crate tests、`xtask check-dep-dag` 通过。
- [ ] `xtask dump-symbols` 不含 `lumio_core_get_api_v1` 或未批准符号。

**依赖:** T-error-03
**Blocked:** Panic ErrorCode

### T-ffi-02: 实现 raw/Buffer/alias 校验

**Files:**
- Create: `crates/lumio-native-ffi/src/validation.rs`
- Modify: `crates/lumio-native-ffi/src/lib.rs`
- Test: `crates/lumio-native-ffi/tests/null_nonzero_length_is_rejected.rs`

**Consumes:** T-contract-types-03, T-memory-01
**Produces:** validation helpers
**成熟方案:** std checked conversions

**步骤（TDD）:**
1. 先写失败测试 `null_nonzero_length_is_rejected`，固定输入、预期状态/错误类别与 owner/handle/allocator 计数。
2. 写只覆盖本任务的最小实现；不得扩大稳定 API、写公共数值或绕过 Adapter。
3. 运行 `cargo test -p lumio-native-ffi null_nonzero_length_is_rejected`、`cargo xtask check-dep-dag`；期望测试和依赖 Gate 同时通过。

**验收:**
- [ ] `crates/lumio-native-ffi/src/validation.rs` 存在，`lib.rs` 只增加必要声明/re-export。
- [ ] `null_nonzero_length_is_rejected` 在实现前失败、实现后可重复通过，不靠 wall-clock sleep 碰运气。
- [ ] `validation helpers` 不泄漏第三方类型或未批准公共数值。
- [ ] `cargo fmt --check`、目标 crate tests、`xtask check-dep-dag` 通过。
- [ ] `xtask dump-symbols` 不含 `lumio_core_get_api_v1` 或未批准符号。

**依赖:** T-contract-types-03, T-memory-01
**Blocked:** Buffer Header layout

### T-ffi-03: 实现 opaque Handle seam

**Files:**
- Create: `crates/lumio-native-ffi/src/handles.rs`
- Modify: `crates/lumio-native-ffi/src/lib.rs`
- Test: `crates/lumio-native-ffi/tests/wrong_context_opaque_handle_is_rejected.rs`

**Consumes:** T-handle-04, T-contract-types-01
**Produces:** encode/decode helpers
**成熟方案:** generated adapter

**步骤（TDD）:**
1. 先写失败测试 `wrong_context_opaque_handle_is_rejected`，固定输入、预期状态/错误类别与 owner/handle/allocator 计数。
2. 写只覆盖本任务的最小实现；不得扩大稳定 API、写公共数值或绕过 Adapter。
3. 运行 `cargo test -p lumio-native-ffi wrong_context_opaque_handle_is_rejected`、`cargo xtask check-dep-dag`；期望测试和依赖 Gate 同时通过。

**验收:**
- [ ] `crates/lumio-native-ffi/src/handles.rs` 存在，`lib.rs` 只增加必要声明/re-export。
- [ ] `wrong_context_opaque_handle_is_rejected` 在实现前失败、实现后可重复通过，不靠 wall-clock sleep 碰运气。
- [ ] `encode/decode helpers` 不泄漏第三方类型或未批准公共数值。
- [ ] `cargo fmt --check`、目标 crate tests、`xtask check-dep-dag` 通过。
- [ ] `xtask dump-symbols` 不含 `lumio_core_get_api_v1` 或未批准符号。

**依赖:** T-handle-04, T-contract-types-01
**Blocked:** Opaque/ContextId representation

### T-ffi-04: 实现 generated exports 与 C smoke

**Files:**
- Create: `crates/lumio-native-ffi/src/exports.rs`
- Modify: `crates/lumio-native-ffi/src/lib.rs`
- Test: `crates/lumio-native-ffi/tests/c_smoke_invalid_handle_returns_stable_code.rs`

**Consumes:** T-ffi-01/02/03, T-context-05, T-job-07
**Produces:** approved provider exports
**成熟方案:** 架构生成 Header

**步骤（TDD）:**
1. 先写失败测试 `c_smoke_invalid_handle_returns_stable_code`，固定输入、预期状态/错误类别与 owner/handle/allocator 计数。
2. 写只覆盖本任务的最小实现；不得扩大稳定 API、写公共数值或绕过 Adapter。
3. 运行 `cargo test -p lumio-native-ffi c_smoke_invalid_handle_returns_stable_code`、`cargo xtask check-dep-dag`；期望测试和依赖 Gate 同时通过。

**验收:**
- [ ] `crates/lumio-native-ffi/src/exports.rs` 存在，`lib.rs` 只增加必要声明/re-export。
- [ ] `c_smoke_invalid_handle_returns_stable_code` 在实现前失败、实现后可重复通过，不靠 wall-clock sleep 碰运气。
- [ ] `approved provider exports` 不泄漏第三方类型或未批准公共数值。
- [ ] `cargo fmt --check`、目标 crate tests、`xtask check-dep-dag` 通过。
- [ ] `xtask dump-symbols` 不含 `lumio_core_get_api_v1` 或未批准符号。

**依赖:** T-ffi-01, T-ffi-02, T-ffi-03, T-context-05, T-job-07
**Blocked:** 正式 Header/symbol list

### T-ffi-05: 建立 symbol/dependency 负向 Gate

**Files:**
- Create: `crates/lumio-native-ffi/src/symbol_guard.rs`
- Modify: `crates/lumio-native-ffi/src/lib.rs`
- Test: `crates/lumio-native-ffi/tests/root_symbol_is_absent.rs`

**Consumes:** T-ffi-04
**Produces:** dump-symbols assertions
**成熟方案:** xtask

**步骤（TDD）:**
1. 先写失败测试 `root_symbol_is_absent`，固定输入、预期状态/错误类别与 owner/handle/allocator 计数。
2. 写只覆盖本任务的最小实现；不得扩大稳定 API、写公共数值或绕过 Adapter。
3. 运行 `cargo test -p lumio-native-ffi root_symbol_is_absent`、`cargo xtask check-dep-dag`；期望测试和依赖 Gate 同时通过。

**验收:**
- [ ] `crates/lumio-native-ffi/src/symbol_guard.rs` 存在，`lib.rs` 只增加必要声明/re-export。
- [ ] `root_symbol_is_absent` 在实现前失败、实现后可重复通过，不靠 wall-clock sleep 碰运气。
- [ ] `dump-symbols assertions` 不泄漏第三方类型或未批准公共数值。
- [ ] `cargo fmt --check`、目标 crate tests、`xtask check-dep-dag` 通过。
- [ ] `xtask dump-symbols` 不含 `lumio_core_get_api_v1` 或未批准符号。

**依赖:** T-ffi-04
**Blocked:** 无

---

# 2.13 `test-support`

## A. 一句话定位 + 边界

**定位：** dev-only deterministic helpers、Fixture loader、fault/leak/interleaving harness；不进入生产依赖，不成为第二规范，不提供产品 mock 语义。

**输入 / 输出 / 所有权：** 输入架构 Fixture、seed、fault plan；输出 FakeClock、barriers、owner counters 和 expected reports。

**线程模型：** 测试显式驱动；不得用 sleep 代替可表达 interleaving。

## B. 内部子模块怎么切

| 单元 | 文件 | 依赖 | 可见性 | 切分理由 |
|---|---|---|---|---|
| clock | `.../clock.rs` | platform | dev-only public | 时间统一 |
| interleaving | `.../interleaving.rs` | std/loom | dev-only public | 竞态可重放 |
| leak | `.../leak.rs` | snapshots | dev-only public | owner counts |
| fault | `.../fault.rs` | 无 | dev-only public | 命名 fault point |
| fixtures | `.../fixtures.rs` | contract-types | dev-only public | 架构 corpus |
| panic | `.../panic.rs` | ffi tests | dev-only public | boundary probe |

## C. 代码面

```rust
pub struct FakeClock { now_nanos: std::sync::atomic::AtomicU64 }
impl FakeClock {
    pub fn new(initial:lumio_platform::Ticks)->Self;
    pub fn advance(&self,d:std::time::Duration);
    pub fn set_forward(&self,t:lumio_platform::Ticks)->Result<(),FakeClockError>;
}
impl lumio_platform::MonotonicClock for FakeClock { fn now(&self)->lumio_platform::Ticks; }
pub struct LeakSnapshot { pub handles_live:u64, pub native_bytes_charged:u64,
    pub leases_live:u64, pub jobs_non_terminal:u64 }
pub struct Interleaving { _private: () }
impl Interleaving { pub fn arrive_and_wait(&self, step:&'static str); }
```

**关键不变式：**

- 只允许 dev-dependency，xtask 检查生产 graph。
- expected values 来自架构源，不在 helper 自创。
- FakeClock 不倒退。
- 随机压力记录 seed/platform/build/minimized corpus。

**失败与稳定类别：** Fixture mismatch/test helper misuse 只作为 typed test failure，不映射公共 ErrorCode。

**相邻模块接口：** Consumes：各 crate Rust test seam。Produces：dev helpers；无生产消费者。

## D. 成熟方案选型

| 候选 | 许可证 | 满足 | 不满足 | 裁决 |
|---|---|---|---|---|
| loom | MIT | 模型并发 | 只覆盖小状态 | 采用 |
| proptest | MIT/Apache-2.0 | 生成/最小化 | 不代替 Fixture | 采用 |
| sleep tests | 无 | 简单 | 脆弱 | 禁止主方法 |
| 自研 named barriers | 本仓 | 精确 | 范围小 | 采用+loom |

V1 生产路径不新增第三方依赖；标准库、生成物和小型 typed model 已足够，dev dependency 不进入生产 graph。

## E. 测试与 Fixture

- **unit：** FakeClock/fault/leak。
- **concurrency：** harness replay/seed。
- **fault：** corrupt fixture/wrong baseline/leak。
- **bench：** 不适用。
- **共同要求：** 固定 baseline/build/target/seed；可由 FakeClock/Interleaving 表达的竞态不得只依赖 `sleep`；bench 报告 throughput、p50/p95/p99/max、alloc、peak bytes、queue depth，时钟读数不进入权威 Hash。

## F. 本模块任务列表

### T-test-support-01: 实现 FakeClock

**Files:**
- Create: `crates/lumio-test-support/src/clock.rs`
- Modify: `crates/lumio-test-support/src/lib.rs`
- Test: `crates/lumio-test-support/tests/fake_clock_cannot_move_backwards.rs`

**Consumes:** T-platform-01
**Produces:** `FakeClock`
**成熟方案:** std atomics

**步骤（TDD）:**
1. 先写失败测试 `fake_clock_cannot_move_backwards`，固定输入、预期状态/错误类别与 owner/handle/allocator 计数。
2. 写只覆盖本任务的最小实现；不得扩大稳定 API、写公共数值或绕过 Adapter。
3. 运行 `cargo test -p lumio-test-support fake_clock_cannot_move_backwards`、`cargo xtask check-dep-dag`；期望测试和依赖 Gate 同时通过。

**验收:**
- [ ] `crates/lumio-test-support/src/clock.rs` 存在，`lib.rs` 只增加必要声明/re-export。
- [ ] `fake_clock_cannot_move_backwards` 在实现前失败、实现后可重复通过，不靠 wall-clock sleep 碰运气。
- [ ] ``FakeClock`` 不泄漏第三方类型或未批准公共数值。
- [ ] `cargo fmt --check`、目标 crate tests、`xtask check-dep-dag` 通过。

**依赖:** T-platform-01
**Blocked:** 无

### T-test-support-02: 实现 deterministic Interleaving

**Files:**
- Create: `crates/lumio-test-support/src/interleaving.rs`
- Modify: `crates/lumio-test-support/src/lib.rs`
- Test: `crates/lumio-test-support/tests/named_interleaving_is_replayable.rs`

**Consumes:** 无
**Produces:** `Interleaving`
**成熟方案:** loom+named barriers

**步骤（TDD）:**
1. 先写失败测试 `named_interleaving_is_replayable`，固定输入、预期状态/错误类别与 owner/handle/allocator 计数。
2. 写只覆盖本任务的最小实现；不得扩大稳定 API、写公共数值或绕过 Adapter。
3. 运行 `cargo test -p lumio-test-support named_interleaving_is_replayable`、`cargo xtask check-dep-dag`；期望测试和依赖 Gate 同时通过。

**验收:**
- [ ] `crates/lumio-test-support/src/interleaving.rs` 存在，`lib.rs` 只增加必要声明/re-export。
- [ ] `named_interleaving_is_replayable` 在实现前失败、实现后可重复通过，不靠 wall-clock sleep 碰运气。
- [ ] ``Interleaving`` 不泄漏第三方类型或未批准公共数值。
- [ ] `cargo fmt --check`、目标 crate tests、`xtask check-dep-dag` 通过。

**依赖:** 无
**Blocked:** loom approval

### T-test-support-03: 实现 LeakCounter/Snapshot

**Files:**
- Create: `crates/lumio-test-support/src/leak.rs`
- Modify: `crates/lumio-test-support/src/lib.rs`
- Test: `crates/lumio-test-support/tests/leak_snapshot_detects_unreleased_handle_and_bytes.rs`

**Consumes:** T-handle-04, T-memory-03
**Produces:** `LeakSnapshot` assertions
**成熟方案:** std

**步骤（TDD）:**
1. 先写失败测试 `leak_snapshot_detects_unreleased_handle_and_bytes`，固定输入、预期状态/错误类别与 owner/handle/allocator 计数。
2. 写只覆盖本任务的最小实现；不得扩大稳定 API、写公共数值或绕过 Adapter。
3. 运行 `cargo test -p lumio-test-support leak_snapshot_detects_unreleased_handle_and_bytes`、`cargo xtask check-dep-dag`；期望测试和依赖 Gate 同时通过。

**验收:**
- [ ] `crates/lumio-test-support/src/leak.rs` 存在，`lib.rs` 只增加必要声明/re-export。
- [ ] `leak_snapshot_detects_unreleased_handle_and_bytes` 在实现前失败、实现后可重复通过，不靠 wall-clock sleep 碰运气。
- [ ] ``LeakSnapshot` assertions` 不泄漏第三方类型或未批准公共数值。
- [ ] `cargo fmt --check`、目标 crate tests、`xtask check-dep-dag` 通过。

**依赖:** T-handle-04, T-memory-03
**Blocked:** 无

### T-test-support-04: 实现 Fixture/Fault/Panic helpers

**Files:**
- Create: `crates/lumio-test-support/src/fixtures.rs`
- Modify: `crates/lumio-test-support/src/lib.rs`
- Test: `crates/lumio-test-support/tests/fixture_loader_rejects_wrong_baseline.rs`

**Consumes:** T-contract-types-04, T-test-support-01/02/03
**Produces:** fixture harness
**成熟方案:** proptest+architecture fixtures

**步骤（TDD）:**
1. 先写失败测试 `fixture_loader_rejects_wrong_baseline`，固定输入、预期状态/错误类别与 owner/handle/allocator 计数。
2. 写只覆盖本任务的最小实现；不得扩大稳定 API、写公共数值或绕过 Adapter。
3. 运行 `cargo test -p lumio-test-support fixture_loader_rejects_wrong_baseline`、`cargo xtask check-dep-dag`；期望测试和依赖 Gate 同时通过。

**验收:**
- [ ] `crates/lumio-test-support/src/fixtures.rs` 存在，`lib.rs` 只增加必要声明/re-export。
- [ ] `fixture_loader_rejects_wrong_baseline` 在实现前失败、实现后可重复通过，不靠 wall-clock sleep 碰运气。
- [ ] `fixture harness` 不泄漏第三方类型或未批准公共数值。
- [ ] `cargo fmt --check`、目标 crate tests、`xtask check-dep-dag` 通过。

**依赖:** T-contract-types-04, T-test-support-01, T-test-support-02, T-test-support-03
**Blocked:** 架构 Fixture corpus

---

# 3. 全仓任务索引与 Wave

任务总数由文档中的 `### T-...` 标题机械校验，目标为 **65**。

## 3.1 Wave 划分

| Wave | Tasks | 退出条件 |
|---|---|---|
| Gate-0 | T-contract-types-01..04；T-error-01..02；T-platform-01 | 生成契约 seam、内部 error/clock 可编译；公共数值可继续 Blocked |
| Foundation-1 | T-capability-01..04；T-handle-01..04；T-memory-01..03；T-test-support-01..03 | limits、typed handle、budget、deterministic harness |
| Foundation-2 | T-handle-05..06；T-memory-04..06；T-context-01..06 | 所有权与七步 Context lifecycle 闭合 |
| Foundation-3 | T-job-01..07 | 有界 Typed Job 完成 submit→execute→drain→release |
| NativeHeadless-4 | T-job-08；T-spatial-01..06；T-test-support-04 | 竞态 conformance、Spatial differential、Fixture loader |
| Hardening-5 | T-error-03..04；T-platform-02..03；T-ffi-01..03 | mapping、三平台 clock、FFI guard/validation seams |
| PrivatePrototype-6 | T-codec-01..05；T-diagnostics-01..04 | 仅 private feature，无 default symbols/deps |
| ReleaseGate-7 | T-ffi-04..05 | 正式生成 Header 可用后，C smoke 与 symbol manifest 通过 |

同一 Wave 的实际并行需再按 Files 做机械重叠检查；共同修改同一 `lib.rs` 的卡必须串行合并模块声明，不能制造 merge race。

## 3.2 Foundation 第一条可运行垂直切片

- 创建 `KernelContext`，冻结 `ConfiguredLimits`，注入 `NoopRecordPort` 与 `FakeClock`。
- 注册 Memory 与 JobSystem 两个 `ContextResource`。
- 注册 test-only Typed Kernel；向 bounded queue 提交一个 Job。
- Worker 写 CallerOutput/NativeOwnedBuffer，terminal 与 completion 只发布一次。
- 调用方 drain、release；LeakSnapshot 回到 handles=0、bytes=0、leases=0、non-terminal jobs=0。
- 并发 close，验证拒绝新工作、cancel、quiesce、destroy、retire、terminal 与迟到 completion 失效。

该切片不需要 Spatial/Codec/Diagnostics 公共 ABI，也不需要 `lumio_core_get_api_v1`。

---

# 4. 并发与故障矩阵

## 4.1 Context 竞态

| Case | 固定 interleaving | 唯一允许结果 | 资源断言 | 测试 |
|---|---|---|---|---|
| CTX-R-01 close/register | register 读 open；close CAS；register commit | late register→ContextClosing | resource count 不变 | `close_vs_register_rejects_late_registration` |
| CTX-R-02 close/allocate | reserve 前 close 线性化 | allocate→ContextClosing | charged bytes 不变 | `close_vs_allocate_rolls_back_reservation` |
| CTX-R-03 double close | 两个线程同时 CAS | 一个驱动；另一观察同 report/pending | destroy 每资源一次 | `double_close_has_single_driver` |
| CTX-R-04 quiesce/deadline | resource Pending；FakeClock 到期 | 按 spec 返回 timeout/pending；不得提前 destroy | live lease 保留并报告 | `deadline_does_not_destroy_live_resource` |
| CTX-R-05 late completion/destroy | kernel 计算结束；Context 已 retire | completion 不可发布为有效 | 最终 job/buffer 归零 | `late_completion_after_destroy_is_stale` |
| CTX-R-06 destroy failure | 前项成功；后项失败 | 完整 report；不得复活 | 每项最多 destroy 一次 | `destroy_failure_produces_complete_report` |

## 4.2 Job 竞态

| Case | 固定 interleaving | 终态 | 输出 | 测试 |
|---|---|---|---|---|
| JOB-R-01 cancel/dequeue | Queued；cancel CAS；worker dequeue | 按 spec Cancelled 赢 | 无 output | `cancel_before_dequeue_wins` |
| JOB-R-02 start/cancel | start token 与 cancel CAS | 单一合法赢家 | Running 赢则 cooperative cancel | `start_vs_cancel_single_winner` |
| JOB-R-03 complete/cancel | kernel 结束与 cancel CAS | 按状态矩阵 | Succeeded 赢才发布一次 | `complete_vs_cancel_matches_matrix` |
| JOB-R-04 timeout/complete | FakeClock deadline 与 completion | 按线性化点 | TimedOut 赢则迟到 bytes 不可见 | `timeout_vs_complete_matches_matrix` |
| JOB-R-05 release/complete | holder release 与 worker complete | payload drop 一次 | 无 UAF/double completion | `release_vs_complete_drops_once` |
| JOB-R-06 close/submit | submit 过/未过 Context gate | 已接纳按关闭规则；否则拒绝 | 计数守恒 | `close_vs_submit_preserves_admission_boundary` |
| JOB-R-07 completion queue full | terminal ready；completion queue 满 | 终态不回滚，overflow 有界可诊断 | 不无限分配 | `completion_queue_full_is_bounded` |
| JOB-R-08 non-cooperative kernel | deadline 后仍执行 | 线程不 kill；terminal TimedOut/stale | 返回后才回收 | `non_cooperative_kernel_cannot_publish_late_result` |

---

# 5. Architecture Gate Blocked Register

| ID | 缺口 | 影响 | 临时内部处理 | 解锁证据 |
|---|---|---|---|---|
| B-ABI-001 | ErrorCode 名称/数值及 panic/version mapping | T-error-03, T-ffi-01/04 | 仅 `ErrorCategory` | generated registry + fixtures |
| B-ABI-002 | Capability bits/名称 | T-capability-01, FFI query | crate-private CapabilityKey | generated capability registry |
| B-ABI-003 | opaque Handle width/bit layout | T-ffi-03/04 | 内部 HandleKey | generated Header/layout |
| B-ABI-004 | Operation ID Registry/test range | T-job-01/04, adapters | crate-private test IDs | generated operation registry |
| B-ABI-005 | ContextId ABI 表示 | T-ffi-03/04 | ContextKey 仅内部 | generated Context type |
| B-ABI-006 | Buffer Header 字段/对齐/required length | T-contract-types-03, T-ffi-02/04 | Rust Buffer 三类 | generated Header fixture |
| B-ABI-007 | Codec 公共格式/算法/operation/error | T-codec-* | private feature | ADR 0005 resolved + artifacts |
| B-ABI-008 | Diagnostics record schema/ID/batch | T-diagnostics-* | private owned record | ADR 0005 resolved + artifacts |
| B-FIX-001 | 正向/失败 Fixture corpus | T-test-support-04, context/job/ffi suites | 只实现 loader | architecture fixture package |

Blocked 未解锁时可完成内部 seam、负向 Gate 与测试 harness，但不得把对应任务标记为“公共 ABI 已完成”。

---

# 6. 供应链、平台与验证

## 6.1 外部 crate 准入证据

- crates.io/upstream 身份一致，release tag/commit 可重现，Cargo.lock exact。
- 许可证属于默认集合；其他许可证单独法务 Gate。
- 维护活跃度、最近发布、Bus Factor、issue 响应写入供应链 manifest；不能只写下载量。
- RustSec/advisory 无未豁免高风险；豁免含 owner、expiry、替换计划。
- Windows x86_64、macOS arm64/x86_64、Linux x86_64 build/test；移动后续单列。
- 静态链接、AOT/IL2CPP 间接加载、build.rs 与 native dependency 审计。
- public API/rustdoc 检查第三方类型无泄漏。
- backend seam + differential corpus 证明退出路径。

## 6.2 必跑命令

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo test --workspace --no-default-features
cargo xtask check-dep-dag
cargo xtask dump-symbols
cargo deny check
cargo audit
cargo tree -p lumio-kernel | rg 'lumio-job|lumio-diagnostics' && exit 1 || true
cargo tree -p lumio-spatial | rg 'lumio-job' && exit 1 || true
cargo tree -p lumio-codec | rg 'lumio-job' && exit 1 || true
```

Release hardening 追加 Miri（可支持 crate）、ASan/TSan、C smoke、symbol/ABI layout diff、Fixture replay 与 benchmark baseline。

---

# 7. 负向验收清单

- [ ] 无手写公共 ErrorCode/Capability bit/Operation ID 数值。
- [ ] `lumio-kernel` normal dependencies 不含 `lumio-job`/`lumio-diagnostics`。
- [ ] `lumio-spatial`/`lumio-codec` normal dependencies 不含 `lumio-job`。
- [ ] 核心 crate 不依赖 tracing/metrics/diagnostics implementation。
- [ ] default features 不启用 codec/diagnostics vendor prototype。
- [ ] public Rust/C API 不含 slotmap/rstar/crossbeam/zstd/lz4/tracing/metrics 类型。
- [ ] 仅 `lumio-native-ffi` 是 cdylib/staticlib。
- [ ] 导出中不存在 `lumio_core_get_api_v1`。
- [ ] Worker ABI 不接受 function pointer/delegate/managed callback。
- [ ] 无 Voxel/ECS/Gameplay/Session/Network/Host/Wall Clock/TickId 语义。
- [ ] Buffer 类别只有三种冻结术语。
- [ ] Context close 后 fixtures 要求的 owner/handle/allocator/job 计数归零。

---

# 8. 文档级完成定义

- [ ] 13 个模块均有 A–F 小节。
- [ ] 全局 port 均有 crate/路径/签名/实现者/调用者/不变式。
- [ ] 65 张任务卡均有 Files、Consumes、Produces、成熟方案、TDD、验收、依赖、Blocked。
- [ ] 所有自研项有候选表、最小范围、维护责任、退出 seam。
- [ ] 所有未冻结项进入 Blocked Register，未伪装成公共完成。
- [ ] codec/diagnostics 只有 private prototype。
- [ ] FFI 唯一导出面和 Root symbol 禁令可机器验证。

第一批可领取：`T-platform-01`、`T-error-01`、`T-test-support-02`。遇到生成契约缺口，只实现内部 seam/失败测试并保持 Blocked，不得手写替代 Header。

本文不要求改 ADR、模块 README、架构镜像、公共 Header 或 crate 映射；不创建生产业务语义；不提交 Git。

