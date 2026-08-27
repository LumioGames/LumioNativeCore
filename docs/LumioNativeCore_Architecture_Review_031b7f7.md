# LumioNativeCore 架构 Review

- Review 类型：只读、对抗式、证据驱动
- Review 基线：`LGE-V1.0-2026-08-27`
- 被审提交：`031b7f75b96c84a9dd9aafd5d410363fa293e706`（`docs(modules): add native core module READMEs`）
- Review 日期：2026-08-27
- 范围：仓库文档、九个模块边界、依赖图、ABI/生命周期/线程/资源契约、阶段与可实现性
- 不包含：代码修改、分支、提交、推送、实现代码

> 证据限制：本次运行环境无法直接 checkout 用户本机 `/Users/cui/LumioGames/LumioNativeCore`，因此 `git status`、Node 校验器和 `sha256sum -c` 未在本地工作树实际执行。报告读取并核对了 GitHub 上指定提交的根 README、九个模块 README、本地 v1.0 架构镜像、标准/Agent/Workflow 文件和 baseline digest；GitHub Workflow 声明的校验命令不等于本次已执行通过。结构校验结果应在开发机上补跑后附真实退出码。

# 1. 总体结论

## 结论

**REQUEST_REDESIGN**

- **结论置信度：High**
- **是否建议开始 Foundation 实现：否。** 当前可做 Cargo/workspace、CI、测试工具目录等不冻结公共契约的脚手架；不应开始稳定 ABI、Handle、Memory、Job 的功能实现或发布字段冻结。
- **当前最重要的三个风险：**
  1. `abi` 同时扮演最底层 POD/Buffer 类型层、NativeCore Root API 层和跨仓统一 Root ABI，和 CoreEngine 的 Root ABI 所有权以及模块依赖方向冲突，容易形成循环依赖、重复导出符号或第二套绑定入口。
  2. `Context` 被 Handle、Spatial 和异步结果反复引用，但没有模块拥有 Context 的创建、Quiesce、Drain、关闭、Generation/Epoch 和迟到结果处置；Handle/Memory/Job 无法组成安全销毁闭环。
  3. Caller Buffer、Native Buffer 和 Job 输入的“所有权转移”只有原则，没有 allocator provenance、终态回收和取消/超时/关闭竞态协议，存在 Use-after-free、双重释放和泄漏风险。

## 总体判断

九个模块的**领域边界方向大体正确**：没有明显把 Voxel、ECS、Gameplay、网络、Session 或 CoreCLR 语义写入 Kernel；`spatial`、`codec`、`diagnostics` 的“不负责范围”也有意识地保持领域无关。

但当前设计还不是可实施架构。核心问题不是缺少更多算法模块，而是缺少：

1. 一个精确的 **`contract-types` 叶子层**，承载架构源生成的固定宽度 ABI 类型，而不同时拥有 Root API；
2. 一个领域无关的 **`kernel-context` 生命周期根**，统一拥有 Handle Arena、Worker、Pool、索引和关闭屏障；
3. 一套可执行的 **Buffer/Job/Context 状态机与 Conformance Fixture**；
4. 一个清楚的 **NativeCore provider 与 CoreEngine unified Root ABI 的组合契约**；
5. 不进入稳定 ABI 的 **private platform/monotonic-clock port** 与工程工具体系。

不建议新增泛化 `common` 或 `tools` 运行时模块。`common` 很容易成为无边界依赖汇聚点；工具应当是 `tools/xtask`、conformance、fuzz、bench、ABI smoke、contract-diff 等开发/验证目标，不应成为稳定 C ABI 模块。

## 验证状态

| 检查项 | 本次状态 | 证据/说明 |
| --- | --- | --- |
| 指定提交存在 | 已核对 | GitHub commit `031b7f7`，10 files changed，新增九个模块 README 并修改根 README |
| `git status --short --branch` | 未执行 | 无本地 checkout；不能断言用户工作树干净 |
| `git show --stat --oneline 031b7f7` | 等价内容已核对 | GitHub commit 页面显示提交信息和变更统计；不是本地命令输出 |
| `rg --files modules docs .spec` | 主要文件已逐个读取 | GitHub 文件树与 raw 文件；不是本地命令输出 |
| `node .spec/tools/spec-lint.mjs` | 未执行 | Workflow 声明会执行；本次不能声称通过 |
| `node --test .spec/tools/spec-lint.test.mjs` | 未执行 | 同上；测试中预期 fixture 错误输出未被误判 |
| `sha256sum -c docs/architecture/.baseline.sha256` | 未执行 | digest 文件期望值为 `6abd53d0...2075`；未在本地重新计算 v1.0 文件 hash |

# 2. Findings

## P0

### ARCH-P0-001 — `abi` 同时位于依赖图底部和顶部，Root ABI 所有权与 CoreEngine 冲突

- **严重级别：P0**
- **文件路径：**
  - `README.md`：模块依赖方向、Native/Managed ABI 契约、Generated Contract Dependencies
  - `modules/abi/README.md`：负责范围、依赖与约束、线程/错误
  - `modules/error/README.md`：依赖与约束
  - `modules/capability/README.md`：依赖与约束
  - `docs/architecture/LumioGameEngine_Architecture_v1.0.md`：2.1、2.3、8.1、16
- **具体章节或行号：**
  - `README.md:L35-L51, L71-L80, L97-L110`
  - `modules/abi/README.md:L10-L15, L26-L31`
  - `modules/error/README.md:L26-L32`
  - `modules/capability/README.md:L26-L32`
  - `...Architecture_v1.0.md:L40-L42, L83-L92, L288-L300, L422-L428`
- **事实证据：**
  1. 根依赖图把 `abi` 画在所有模块之上/之前，并声称该图是“编译期概念依赖”。
  2. `abi` README 又声明自己不依赖其他 NativeCore 模块，但同一文件要求版本和 Buffer 错误映射到 `error`。
  3. `error`、`capability` 反过来声明依赖 `abi`。
  4. NativeCore README 与 `abi` README 都声称本模块拥有单一 Root API Table，示例符号为 `lumio_core_get_api_v1`；架构 Baseline 则把“统一 Root ABI、Native 聚合构建”明确归给 CoreEngine，NativeCore/VoxelEngine只提供源契约。
  5. 本仓写“每个导出结构包含 `abi_version/struct_size/capability_bits`”，Baseline 只明确要求 **API Table** 包含这三项。两者不是同一条规则。
- **为什么这是架构问题：**
  一个模块不能既是所有公共类型的无依赖叶子层，又是依赖 Error、Capability、Handle 等子系统的 Root API facade。若直接按当前文档建 crate，要么产生 `abi <-> error/capability` 环，要么把 Error/Capability 退化为裸整数并让文档失真。跨仓 Root 符号不明确还可能造成 NativeCore 和 CoreEngine 各导出一个 Root、两个绑定生成入口或链接符号冲突。
- **影响范围：** 全部九模块、CoreEngine 聚合、C Header、C# Binding、Loader、符号检查、ABI 版本演进。
- **最小修正建议：**
  1. 在架构源明确：跨仓唯一外部 Root 符号由谁导出；推荐由 CoreEngine 的 `root-abi`/composition 层拥有。
  2. NativeCore 拆成两个概念层：
     - `contract-types`：无依赖叶子层，只消费架构源生成的固定宽度 POD、Buffer view、version scalar、opaque handle representation；
     - `native-core-ffi`/`provider-api`：顶层 facade，依赖 Error/Capability/Handle/Memory/Job 等模块，并由 CoreEngine 组合进入统一 Root。
  3. Root API 与“每个导出结构”的 header 字段规则回到架构源逐类冻结；不要把 `capability_bits` 机械复制到所有 payload/batch/error 结构。
  4. 公开错误码和能力位作为生成契约类型被消费，不能通过 Rust crate 的反向依赖解决。
- **修正后的验收条件：**
  - 一份 ADR 和更新后的 Baseline 明确 `lumio_core_get_api_v1` 唯一所有者、NativeCore provider 的符号/组合方式。
  - Cargo metadata/依赖图无环；`contract-types` 不依赖任何实现模块。
  - 最终发布产物的 symbol dump 只出现一个跨仓 Root 入口。
  - C Header/C# binding 只由架构源/CoreEngine 工具链生成一次。
  - ABI Fixture 覆盖最小结构、长结构未知尾部、错误/能力未知值和主版本拒绝。
- **归属：** 架构源 Baseline + 本仓 ADR + 后续实现。

### ARCH-P0-002 — 缺失 `KernelContext` 生命周期所有者，资源关闭无法形成闭环

- **严重级别：P0**
- **文件路径：**
  - `README.md`
  - `modules/handle/README.md`
  - `modules/memory/README.md`
  - `modules/job/README.md`
  - `modules/spatial/README.md`
  - `docs/architecture/LumioGameEngine_Architecture_v1.0.md`
- **具体章节或行号：**
  - `README.md:L15-L21, L81-L83`
  - `handle:L10-L32`
  - `memory:L22-L32`
  - `job:L10-L32`
  - `spatial:L22-L32`
  - Baseline `L135-L164, L288-L300`
- **事实证据：**
  1. NativeCore 声称拥有 Worker、Job、内存池、索引和临时批次。
  2. Handle 使用 `Context` 校验；Spatial 索引要在 Context 销毁时失效；Job 结果在 World 销毁后不得写入。
  3. 九模块中没有模块定义 Context 的创建、状态、拥有资源、停止接收、排空、取消、关闭、故障和 Epoch 失效。
  4. `handle` 明确说“不决定上层资源销毁顺序”，`job`/`memory` 也没有接管整个关闭序列。
- **为什么这是架构问题：**
  Handle 的 Context 字段不是单纯校验位，它必须对应一个真实生命周期根。没有所有者，任何模块都无法回答“Context Close 与 Handle Resolve/Job Complete/Pool Reclaim 谁先线性化”，因此会出现旧 Context 句柄复用、迟到结果进入新实例、资源永不回收或正在执行时被销毁。
- **影响范围：** Handle、Memory、Job、Spatial 索引、Codec workspace/dictionary、Diagnostics queue、未来所有 Native 资源。
- **最小修正建议：**
  新增或明确一个领域无关的 `kernel-context`（名称也可为 `kernel-instance`）模块/聚合层：
  - 不等于 World、Session 或 Host；只代表 NativeCore 资源域。
  - 状态机至少为 `Creating -> Running -> Quiescing -> Closed`，任意活动状态可进入 `Faulted`。
  - 统一拥有 Handle Arena namespace、内存预算/池、Worker 集、Completion Queue、索引/workspace registry 和 optional diagnostic recorder。
  - Close 顺序固定为：拒绝新提交 → 标记资源 Closing/失效新 resolve → 请求取消 → Drain 或进入明确 Abandon policy → 回收 completion/input/output → 销毁资源 → 退休 Context Epoch。
  - Generation 溢出时永久退休 slot；Context Epoch 不得静默回卷复用。
- **修正后的验收条件：**
  - ADR 给出 Context 状态机、每个状态允许的 API、关闭线性化点和资源销毁顺序。
  - 所有跨调用资源在 ownership matrix 中只有一个 owner。
  - 并发 Fixture 覆盖 Close vs Resolve、Close vs Submit、Close vs Complete、ContextId 复用、迟到结果和部分初始化失败。
  - Context 关闭后所有 API 返回稳定、可区分错误，且 allocator/handle/job 计数归零或进入文档化的 retained evidence 状态。
- **归属：** 本仓 ADR；若 Context Handle/字段跨 ABI，则公共表示归架构源 Baseline。

### ARCH-P0-003 — Caller Buffer 与异步 Job 的所有权转移没有 allocator provenance 和终态回收协议

- **严重级别：P0**
- **文件路径：**
  - `README.md`
  - `modules/abi/README.md`
  - `modules/memory/README.md`
  - `modules/job/README.md`
- **具体章节或行号：**
  - `README.md:L71-L83`
  - `abi:L22-L31`
  - `memory:L22-L32`
  - `job:L22-L32`
- **事实证据：**
  1. ABI 原则是“创建侧释放”“优先调用方提供 Buffer”。
  2. Memory 说异步 Job 只能接收“明确转移所有权”的批次。
  3. Job 又说提交方转移输入批次所有权，Completion Batch 在某个“声明的消费边界”前有效。
  4. 文档没有定义调用方分配的内存如何由 NativeCore 安全释放、取消/超时是否立即返还、Job 已运行但 Handle 被释放时谁持有 lease、Completion 未消费时谁回收。
- **为什么这是架构问题：**
  “转移所有权”不是可执行 ABI。若 Buffer 来自 managed/native caller allocator，NativeCore 不能凭裸指针推断释放函数；若 caller 在 timeout 后释放而 Worker 仍运行，则 UAF；若双方都等待对方释放，则泄漏；若双方都释放，则 double free。
- **影响范围：** 所有异步 Job、批量 Spatial/Codec、错误载荷、Completion Batch、Context 关闭、OOM 与取消路径。
- **最小修正建议：**
  在 ABI/Memory ADR 中只允许清楚的 Buffer 类别：
  1. `BorrowedCallBuffer`：仅同步调用期间有效，绝不进入异步 Worker；
  2. `CallerOutputBuffer`：仅本次调用写入，容量不足返回 required length；
  3. `NativeOwnedBufferHandle`：由 NativeCore 分配或在 submit 时复制，适用于异步输入/输出，并通过明确 release API 回收；
  4. 可选 `SharedReadOnlyBufferHandle`：必须有引用计数/lease 和 Context 归属，不能是任意 managed pointer。

  Timeout/Cancel 只改变可见状态，不自动证明 Worker 已停止；底层 lease 直到真实 terminal/reap 才释放。Abandon 必须有后台回收和容量上限。
- **修正后的验收条件：**
  - 每个 ABI 参数标注 `borrowed/owned/transferred/out`、allocator provenance、alignment、valid-until、release API。
  - 所有 Job terminal path（完成、拒绝、队列满、取消前、取消中、超时、Worker fault、Context close、未消费 completion）都证明输入/输出恰好回收一次。
  - 故障注入测试在每个状态点触发，最终 native allocation、lease 和 handle 数量符合期望。
  - FFI smoke 验证不同 allocator 边界不会交叉释放。
- **归属：** 本仓 ADR + 架构源 ABI Schema/Fixture + 后续实现。

### ARCH-P0-004 — Job 只有要求清单，没有可实现的状态机、取消线性化与 Deadline 时钟域

- **严重级别：P0**
- **文件路径：**
  - `modules/job/README.md`
  - `modules/handle/README.md`
  - `modules/memory/README.md`
  - `.spec/knowledge/standards/repository-architecture.md`
  - `docs/architecture/LumioGameEngine_Architecture_v1.0.md`
- **具体章节或行号：**
  - `job:L10-L41`
  - `handle:L30-L41`
  - `memory:L22-L41`
  - standard `L15-L25`
  - Baseline `L143-L188, L288-L300`
- **事实证据：**
  Job 文档要求状态转换可线性化，并列出重复取消、超时后完成、结果丢失和 Worker 关闭，但没有给出状态集合、合法转移、竞态赢家、结果保留/消费/销毁规则；也没有定义 Deadline 使用 Host wall clock、Native monotonic clock、相对 duration 还是 Runtime logical Tick。`Worker 只执行 Native 闭包或 Typed Kernel` 也没有声明“闭包仅限 Rust 内部”，容易被误实现为 ABI callback。
- **为什么这是架构问题：**
  取消与超时不会自动终止线程。没有状态机和线性化点，Handle release、Buffer 回收和 Completion 可见性都会由实现者自行解释。Deadline 若进入权威结果，还会把机器调度时序带入确定性状态。
- **影响范围：** Job、Memory、Handle、Runtime Barrier、确定性、性能降级、Context close。
- **最小修正建议：**
  1. 写 Job ADR，至少定义：`Created/Queued/Running/CancelRequested/Completed/Failed/TimedOut/Abandoned/Reaped`，并明确 Timeout 是观测状态还是执行终止。
  2. 指定每一对竞态的单一赢家和 API 返回，例如 Cancel-vs-Complete、Timeout-vs-Complete、Close-vs-Submit。
  3. 公共 ABI 只接受架构源定义的 Typed Kernel/operation ID 和版本化参数；任意 Rust closure 仅可作为内部实现，不得成为 C callback 或托管回调。
  4. Job 使用私有、可注入的 monotonic clock port 或相对 duration；Wall Clock 仍归 Host，Logical Tick/Apply Barrier 仍归 Runtime。Operational timeout 不进入 authoritative hash。
- **修正后的验收条件：**
  - 有状态转移表、终态定义、所有权转移表、关闭协议和时钟域说明。
  - Property/concurrency test 对每个竞态重复运行并验证唯一结果。
  - Public header 中不存在 caller function pointer/managed callback。
  - 不同线程调度下 deterministic kernel 的内容结果和 canonical ordering 相同；等待时间/完成时间只进入 diagnostics。
- **归属：** 本仓 ADR；跨 ABI 的状态/错误/operation ID 归架构源 Baseline。

## P1

### ARCH-P1-001 — 架构源生成契约只被“声明消费”，没有可复现的锁定/同步入口

- **严重级别：P1**
- **文件路径：** `README.md`、`.baseline.sha256`、`code-style.md`、`testing.md`、Workflow
- **具体章节或行号：** `README.md:L11-L13, L97-L100`；`code-style:L22-L30`；`testing:L32-L47`；Workflow `L15-L37`
- **事实证据：** 仓库声明必须消费架构源发布的 ABI/Capability/Error/Header/Fixture，但当前可见树没有 contract lock、生成物目录、版本清单或 fetch/verify 命令；CI 只显式检查本地镜像 hash 和通用 spec lint。
- **为什么这是架构问题：** 实现者无法从仓库独立重建“究竟使用了哪一个 schema/compiler/fixture”，容易从 prose 临时造字段或依赖浮动上游。
- **影响范围：** ABI 可复现性、CI、Header/Binding、Fixture、供应链。
- **最小修正建议：** 增加只读 `contracts.lock`/manifest（BaselineId、source commit、compiler version、input/output hash、artifact package digest），并用 `tools/xtask contract verify` 消费上游产物；本仓不得拥有第二套 schema generator。
- **修正后的验收条件：** 清空缓存后能用固定命令拉取/验证相同产物；任一 hash/版本漂移 CI 失败；本仓修改生成文件会被检测。
- **归属：** 架构源 Baseline + 本仓工具/CI。

### ARCH-P1-002 — Capability 混合静态能力、配置上限和动态资源状态

- **严重级别：P1**
- **文件路径：** `modules/capability/README.md`
- **具体章节或行号：** `L10-L15, L22-L32`
- **事实证据：** 同一 Capability Snapshot 同时包含平台/编译 Feature/后端、资源上限，并用“新快照或失效通知”表达动态资源变化；同时把资源预算不足纳入能力匹配错误。
- **为什么这是架构问题：** 静态 ABI/feature negotiation 应稳定，当前内存/队列可用量是运行状态；混合后会导致 capability bits 随负载变化、Loader 结果与运行时 admission 混为一谈。
- **影响范围：** CoreEngine Loader、Host admission、Context 配置、测试组合、缓存和诊断。
- **最小修正建议：** 区分：
  - `Build/StaticCapabilities`：ABI feature、平台、可选 backend；
  - `ConfiguredLimits`：Context 创建时的最大 worker、queue、memory；
  - `RuntimeStatus`：当前使用量/余量，仅 Diagnostics/查询，不参与静态 capability bit negotiation。
- **修正后的验收条件：** 相同 binary 的 static capability snapshot 在运行期间不可变；动态容量变化不改变 ABI capability bits；Host policy 只消费稳定的 required/provided capability 和独立预算数据。
- **归属：** 公共字段归架构源；拆分行为归本仓 ADR。

### ARCH-P1-003 — Spatial 领域边界清楚，但确定性、精度和并发可见性仍是占位要求

- **严重级别：P1**
- **文件路径：** `modules/spatial/README.md`
- **具体章节或行号：** `L10-L15, L22-L41`
- **事实证据：** 文档要求声明排序、重复输入、坐标、精度、读写并发和重建可见性，却未选择具体规则。
- **为什么这是架构问题：** 这些不是算法内部细节，而是调用方可见行为和 State Hash 输入。不同 Grid/BVH/backend 若各自解释，将导致结果集合、顺序和跨平台语义漂移。
- **影响范围：** Runtime/Voxel Adapter、差分测试、SIMD backend、State Hash、Batch ABI。
- **最小修正建议：** 在进入 public ABI 前冻结：坐标 scalar/单位解释、NaN/Inf/-0/溢出、距离比较、重复 key、tie-breaker、结果排序、query snapshot generation、update 原子性和 backend determinism class。
- **修正后的验收条件：** Reference Kernel + Golden fixtures；所有 backend 对结果集合/顺序满足同一 contract；并发 update/query 明确看到旧 snapshot 或新 snapshot，不出现部分可见。
- **归属：** 本仓 ADR；跨语言字段/排序规则归架构源 Fixture。

### ARCH-P1-004 — Codec 把机械字节 Kernel 与领域 Schema/Canonical Serializer 规则混在一起

- **严重级别：P1**
- **文件路径：** `modules/codec/README.md`、Baseline 11.2、Baseline 16
- **具体章节或行号：** `codec:L10-L15, L22-L41`；Baseline `L318-L329, L422-L428`
- **事实证据：** Codec 宣称不拥有领域字段/Schema，但又负责 Canonical 顺序、Magic、SchemaVersion、重复字段、未知必需/可选字段。Baseline 把字段级 Canonical Serializer 明确描述为“生成的 Serializer”，NativeCore 首批模块地图没有 `codec`，只在后续写“压缩优化”。
- **为什么这是架构问题：** 字节压缩/校验无法判断“未知必需字段”或“重复字段”；这是 schema-aware serializer 的责任。若 Codec 冻结这些语义，就会产生第二套 Serializer 和 Snapshot/Wire 规则。
- **影响范围：** Runtime/Voxel/Game serializer、Snapshot/WAL、Wire、Replay、迁移、公共格式 ID。
- **最小修正建议：**
  - 将 Codec 缩窄为 byte-level compression/decompression、checksum/hash、明确算法的 byte diff/patch、bounded workspace。
  - `SchemaVersion` 等仅作为 opaque metadata 传递或由上层先验证；Codec 不解释 field presence。
  - “Canonical Buffer”改为“由上层 Canonical Serializer 产生并交给 Codec 的 canonical bytes”，或从模块职责中删除 canonicalization。
  - 在架构源批准前只允许 private/feature-gated prototype，不发布跨仓公共 ABI。
- **修正后的验收条件：** Codec Fixture 不包含 unknown field/repeated field 语义；只验证 bytes、长度、算法/version、base hash、解压上限和损坏数据；架构源明确 public format IDs 与模块状态。
- **归属：** 架构源 Baseline + 本仓文档/ADR。

### ARCH-P1-005 — Diagnostics 边界方向正确，但脱敏、Failure Bundle 和热路径接入仍不闭合

- **严重级别：P1**
- **文件路径：** `modules/diagnostics/README.md`、Baseline 12
- **具体章节或行号：** `diagnostics:L10-L41`；Baseline `L334-L350`
- **事实证据：** Diagnostics 正确排除 Sink/Audit/Journal，但把敏感字段脱敏完全交给“进入模块前的调用方”；同时测试写“Failure Bundle 重放”，容易让 NativeCore 拥有完整 Bundle/replay，而 Baseline 把 Failure Bundle 定义为跨系统故障重建类别。共享 schema 也尚未进入 NativeCore 模块地图。
- **为什么这是架构问题：** 任一调用者漏脱敏都可能把秘密进入 bounded queue；“fragment”与完整 bundle 的 owner 不清会重复持久化/重放职责。所有核心模块直接依赖 Diagnostics implementation 还会把热路径与队列实现耦合。
- **影响范围：** Error、Job、Memory、Spatial/Codec、Server/Runtime observability、隐私与故障证据。
- **最小修正建议：**
  - 只接受 allowlisted fixed-width fields 与受限 payload class；默认拒绝任意 raw bytes/字符串，调用方脱敏之外再做 schema-level 限制。
  - NativeCore 只产生 `FailureFragment`；完整 bundle 组装、持久化、下载和 replay 属上层。
  - 核心模块通过 optional non-blocking recorder port 或批量 snapshot/records 接入，不编译依赖具体 sink/queue 实现。
  - 架构源确认 shared event/fragment schema 后再发布公共 ABI。
- **修正后的验收条件：** 敏感字段 canary 不得出现在输出；queue full 不阻塞 Simulation Thread；每 producer sequence 可重建；fragment encode/validate/reassembly fixture 与完整 bundle ownership 分开。
- **归属：** 架构源 Baseline + 本仓 ADR/实现。

### ARCH-P1-006 — `modules/<name>/README.md` 适合作为文档分类，但不能直接等同未来 crate 边界

- **严重级别：P1**
- **文件路径：** 根 README、九个模块 README、`code-style.md`
- **具体章节或行号：** `README.md:L52-L63`；`code-style:L25-L30`
- **事实证据：** 当前没有 Cargo 工程，九目录只含 README；没有 workspace/crate/public-symbol/feature boundary ADR。
- **为什么这是架构问题：** “一个 README 一个 crate”会制造过多 crate、公开内部类型和依赖环；“全部一个 crate”又会失去边界检查。特别是 `abi`、`error`、`capability` 的当前位置已显示文档模块和编译模块不是一一对应。
- **影响范围：** Cargo workspace、feature graph、编译时间、visibility、发布产物和 symbol exports。
- **最小修正建议：** 保留 `modules/` 作为架构文档；另写 crate-map ADR。建议最小生产布局：
  - `lumio-contract-types`（leaf，生成物 adapter）
  - `lumio-kernel`（error/handle/memory/capability/context primitives）
  - `lumio-job`
  - `lumio-spatial`
  - `lumio-codec`（baseline 确认前 experimental/private）
  - `lumio-diagnostics`（baseline 确认前 experimental/private）
  - `lumio-native-ffi`（唯一 NativeCore export facade）
  - private `lumio-platform`、dev-only `lumio-test-support`

  具体 crate 数量可调整，但只允许 `lumio-native-ffi` 产出公共 C symbols。
- **修正后的验收条件：** ADR 映射每个文档模块到 crate/module；Cargo graph 无环；forbidden dependency lint 阻止领域仓倒灌；只有 facade crate 配置 `cdylib/staticlib` 和 symbol export list。
- **归属：** 本仓 ADR + 后续实现。

### ARCH-P1-007 — 测试与 Benchmark 当前主要是类别清单，不足以作为 Architecture Gate Fixture

- **严重级别：P1**
- **文件路径：** 根 README、九个模块 README、`testing.md`、Baseline 15/16
- **具体章节或行号：** `README.md:L118-L139`；`testing:L32-L48`；Baseline `L396-L436`
- **事实证据：** 文档列出了 ABI/Handle/并发/Fuzz/Benchmark 名称，但多数没有固定输入、预期输出、失败条件、工作负载 ID、硬件配置和阈值。Baseline 要求 Architecture Gate 每个 P0 有正向和失败 Fixture 设计。
- **为什么这是架构问题：** 对生命周期/ABI 这类契约，仅写“测试取消竞态”不能约束实现；不同实现都可能声称已测试，但对竞态赢家、内存计数和错误码给出不同结果。
- **影响范围：** Architecture Gate、CI、跨平台一致性、性能回归、故障重现。
- **最小修正建议：** 为每个 P0 模块建立 fixture manifest：case ID、前置状态、输入 bytes、并发 interleaving/fault point、预期状态、error category、owner counts、output/hash。Benchmark 增加 workload ID、dataset hash、build/profile/hardware、warmup、iteration、p50/p95/p99/max、RSS/alloc/copy/queue。
- **修正后的验收条件：** Architecture Gate 文档中每个 P0 至少一个 positive 和一个 failure fixture，且可被未来 Rust/C/C# harness 自动消费；故障路径有明确 expected terminal state 和资源计数。
- **归属：** 架构源 Fixture + 本仓 test-support/benchmark manifest。

### ARCH-P1-008 — “时间与 Diagnostic Kernel”标准与 Baseline RACI 不一致

- **严重级别：P1**
- **文件路径：** `.spec/knowledge/standards/repository-architecture.md`、Baseline 2.3/4、模块列表
- **具体章节或行号：** standard `L15-L18`；Baseline `L83-L92, L143-L188, L422-L428`
- **事实证据：** 本仓标准写 NativeCore 拥有“时间与 Diagnostic Kernel”，但九模块没有 `time`；Baseline 把 Wall Clock/节拍/暂停归 Host，把 Logical Tick/Phase/Determinism 归 Runtime，NativeCore 首批模块也没有 `time`。
- **为什么这是架构问题：** “时间”若不限定，会让 NativeCore误拥有 wall clock/tick pacing；完全没有 clock abstraction 又让 Job Deadline 无法测试和统一。
- **影响范围：** Job timeout、determinism、Host pacing、Runtime Tick、测试注入。
- **最小修正建议：** 不新增公共 `time` 模块。修改标准为：NativeCore 只可拥有 private `monotonic-clock/deadline port`，用于 operational timeout 和测试；不拥有 Wall Clock、TickId、Phase 或模拟时间。若需要跨 ABI 暴露 clock domain，必须回架构源定义 clock ID/unit/overflow。
- **修正后的验收条件：** 标准、Job ADR 和 Baseline 口径一致；authoritative hash 不包含 wall/monotonic timestamps；可用 fake clock deterministic 测试 timeout。
- **归属：** 本仓文档/ADR；跨 ABI clock 字段则归架构源。

### ARCH-P1-009 — 本仓规划状态与跨仓 Baseline 状态虽有说明，但缺少机器可判的双状态记录

- **严重级别：P1**
- **文件路径：** `README.md`、`spatial/codec/diagnostics` README、Baseline 16
- **具体章节或行号：** `README.md:L34, L140-L146`；`spatial:L4-L8`；`codec:L4-L8`；`diagnostics:L4-L8`；Baseline `L422-L436`
- **事实证据：** 根 README 已正确声明优先级/阶段不替代 Baseline，也指出 codec/diagnostics 待上游确认；但每个模块仍统一写同一个 Baseline ID，容易被误解为该模块已进入 Baseline。Spatial 在 Baseline 是“首批”，本仓却放到 NativeHeadless。
- **为什么这是架构问题：** 实施计划与契约批准状态混在同一 metadata，会误导开发者把本仓 P1 当成架构已批准，或把 Baseline 首批误当 Foundation 必做。
- **影响范围：** 排期、Architecture Gate、public ABI、CI gating。
- **最小修正建议：** 模块 README 增加独立字段：`BaselineStatus: approved/pending/not-applicable`、`RepositoryDeliveryPhase`、`ImplementationPriority`；codec/diagnostics 标 pending，spatial 标 approved + planned NativeHeadless。
- **修正后的验收条件：** lint 检查每个公共模块都有双状态；pending 模块不能进入 public header/export list；状态只能由上游 Baseline manifest 更新。
- **归属：** 本仓文档/tooling；批准状态归架构源。

## P2

### ARCH-P2-001 — “调用方创建并销毁 Handle/Buffer”措辞掩盖真实资源创建者

- **严重级别：P2**
- **文件路径：** `README.md:L15-L21`、`handle:L22-L24`、`memory:L22-L24`
- **事实证据：** 根 README 说调用方创建 Handle/Buffer；Handle 模块却负责分配/释放 opaque Handle，资源模块才创建底层资源，调用方只是请求创建并发起 release。
- **为什么这是架构问题：** 会混淆 resource owner、handle owner 和 allocator owner。
- **影响范围：** 文档、API 命名、所有权矩阵。
- **最小修正建议：** 改为“调用方请求创建并持有 opaque handle，资源所属模块创建/销毁底层资源；调用方必须调用 release；Buffer 按 provenance 决定释放方”。
- **验收条件：** 根 README 与各模块使用统一术语：resource owner、handle holder、buffer allocator、release initiator。
- **归属：** 本仓文档。

### ARCH-P2-002 — `P0/P1/P2` 容易与缺陷严重级别混淆

- **严重级别：P2**
- **文件路径：** 根 README 与九个模块 README
- **事实证据：** 模块实施优先级使用 P0/P1，而 Review findings 也使用 P0/P1。
- **为什么这是架构问题：** 讨论中“P0 module”可能被误读为“P0 defect”。
- **影响范围：** 项目管理与评审沟通。
- **最小修正建议：** 实施优先级改为 `I0/I1/I2` 或 `FoundationPriority`；保留 P0/P1 仅表示 defect severity。
- **验收条件：** 文档和任务系统可无歧义区分 implementation priority、delivery phase、baseline status 和 defect severity。
- **归属：** 本仓文档。

### ARCH-P2-003 — Benchmark 指标缺少 p50/max、工作负载身份和环境可比性

- **严重级别：P2**
- **文件路径：** 根 README、模块 README、Baseline 15.3
- **事实证据：** 模块多写 p95/p99；Baseline 还要求固定 workload/hardware 和 p50/p95/p99/max、RSS、复制字节等。
- **为什么这是架构问题：** 无环境/数据集身份的数值不可比较，不能作为性能回归门槛。
- **影响范围：** Job/Spatial/Codec/Handle/Memory 性能基线。
- **最小修正建议：** 统一 benchmark result schema，加入 workload/dataset hash、target triple、CPU feature、compiler/profile、p50/max、RSS、alloc count、copy bytes、queue depth。
- **验收条件：** 两次结果可验证是否同一 workload/build/environment；回归阈值按指标类别定义。
- **归属：** 架构源结果 Schema或本仓 ADR/工具。

### ARCH-P2-004 — 当前 CI 可见规则主要验证文档存在/Hash，缺少模块 DAG 与禁用语义的显式检查

- **严重级别：P2**
- **文件路径：** `.github/workflows/repository-policy.yml`、`.spec/tools/spec-lint*`
- **事实证据：** Workflow 显式调用通用 spec lint，并通过 `test/grep/sha256sum` 检查 README 标题、BaselineId 和镜像 hash；Workflow 中没有单独可见的模块依赖 DAG、唯一 owner、pending module export 或 forbidden domain token 检查。由于本次无法读取/执行 script body，不能断言现有 spec-lint 一定没有这些能力。
- **为什么这是架构问题：** 当前发现的 `abi` 依赖矛盾和 module-map drift 可以在所有文件均存在、hash 正确时通过结构检查。
- **影响范围：** 文档一致性、未来 Cargo dependency drift、public export gate。
- **最小修正建议：** 扩展 lint manifest：模块 owner、allowed deps、baseline status、delivery phase、public export status；自动拓扑排序并拒绝环；扫描禁止的 Voxel/ECS/Session/managed callback 类型进入 public contracts。
- **验收条件：** 对依赖环、重复 owner、pending 模块导出、第二 Root symbol、非法 domain dependency 注入 mutation fixture，lint 必须失败。
- **归属：** 本仓 tooling。

### ARCH-P2-005 — `codec` 名称过宽，容易继续吸收 Serializer/Storage 责任

- **严重级别：P2**
- **文件路径：** `modules/codec/README.md`
- **事实证据：** 当前模块同时写 Diff、压缩、Canonical Buffer、字段级解码错误。
- **为什么这是架构问题：** 宽泛名称会推动职责膨胀。
- **影响范围：** 模块边界、API 命名、上层 serializer。
- **最小修正建议：** 在职责缩窄后考虑 `byte-codec`、`compression-diff`，或在 `codec` 下明确 `compression/checksum/byte-diff` 子边界；不必为了命名强制拆 crate。
- **验收条件：** 模块 README 不再声明字段级 schema validation/canonical serializer ownership。
- **归属：** 本仓文档/ADR。

# 3. 模块职责审查矩阵

| 模块 | 内聚性 | 边界清晰度 | 依赖合理性 | 契约完整性 | 主要风险 | 结论 |
| --- | --- | --- | --- | --- | --- | --- |
| `abi` | 低 | 低 | 不合理 | 缺失 | 同时是 leaf types 与 top Root facade；与 CoreEngine Root 所有权冲突 | **必须重设计/拆分概念层** |
| `handle` | 高 | 中 | 部分合理 | 部分完整 | Context owner、Arena/resource owner、generation overflow、close race 未定义 | 保留模块，先补生命周期 ADR |
| `error` | 高 | 高 | 受 ABI 分层冲突影响 | 部分完整 | machine code 与 payload lifetime 有原则无具体 schema；不能依赖 top ABI | 保留，改为依赖 contract-types |
| `capability` | 中高 | 中 | 受 ABI 分层冲突影响 | 部分完整 | 静态 feature、配置上限、动态资源状态混合 | 保留，拆静态能力/限制/状态 |
| `memory` | 高 | 中 | 基本合理 | 缺失关键部分 | async transfer、allocator provenance、terminal reclaim 不清 | 保留，P0 补所有权协议 |
| `job` | 高 | 中 | 不完整 | 缺失 | 无状态机/时钟域/关闭语义；closure 可能被误做 public callback | 保留但需核心重设计 |
| `spatial` | 高 | 高 | 基本合理 | 部分完整 | 排序/精度/单位/并发可见性尚未契约化；缺显式 handle dep | 领域边界通过，public ABI 暂缓 |
| `codec` | 中 | 中低 | 基本合理 | 部分完整 | 泄漏 schema/canonical serializer 语义；上游状态 pending | 缩窄职责并等待 Baseline |
| `diagnostics` | 中高 | 中 | 运行时关系未定 | 部分完整 | 脱敏前置、完整 Failure Bundle/replay 边界、热路径耦合 | 保留为 fragment/record producer，等待 Baseline |

# 4. 模块间边界审查

| 边界 | 判断 | 审查结论 | 最小调整 |
| --- | --- | --- | --- |
| `abi / error / capability` | **需要拆分** | `abi` 同时是底层类型与上层 Root；Error/Capability 依赖它，它又需要它们表达 Root 行为 | 新增精确 `contract-types` leaf；`native-core-ffi` 位于顶部；CoreEngine 拥有最终 Root |
| `handle / memory` | **需要补充契约** | Handle token/slot 与底层资源/内存 owner 未区分；释放先失效还是先 drop 未定义 | Typed arena/lease 规则；先关闭新 resolve，再 drain borrow/job，最后 drop/recycle；generation exhaustion 退休 slot |
| `error / diagnostics` | **需要补充契约** | Error 是同步 machine outcome；Diagnostics 是异步 evidence。当前仅说可引用，没有 copy/retention/redaction 规则 | Diagnostics 只复制稳定 code/correlation，不保存 error buffer pointer；恢复策略不属于两者 |
| `capability / Host/CoreEngine` | **需要补充契约** | 业务模式排除得当；但 static feature 与 dynamic budget 混合 | CoreEngine 验 ABI/static capability；Host/Context admission 使用独立 limits/status |
| `memory / job` | **需要补充契约** | async input ownership、completion lifetime、cancel/timeout reclaim 不可执行 | 四类 Buffer + lease + terminal/reap state；Memory 不知道调度策略，只提供 ownership primitives |
| `job / Runtime Scheduler` | **清晰但需收紧** | Tick Phase/Processor/owner thread 正确留在 Runtime；Job 是 native kernel scheduler | Public 只接受 typed operation；无 managed/function callback；job 时间不决定模拟提交 |
| `spatial / Voxel/ECS/AOI` | **清晰，契约待补** | 没有领域语义泄漏 | 冻结坐标/排序/精度/snapshot visibility；AOI/collision policy 留上层 |
| `codec / Schema/Serialization/Storage` | **需要拆分/上移** | 字段、unknown required/repeated field 属生成 serializer；Snapshot/WAL owner 已排除 | Codec 只做 bytes/diff/compression/checksum；schema validation 上移；public formats 回 Baseline |
| `diagnostics / Logging/Audit/Journal` | **需要补充契约/上移** | Sink/Audit/Journal 排除正确；完整 Failure Bundle/replay 与脱敏边界仍模糊 | NativeCore 只产 bounded records/fragments；完整 bundle/sink/replay 上移；schema allowlist |

# 5. 依赖图审查

## 5.1 当前依赖图的问题

1. 画法没有明确箭头方向；根图把 `abi` 表示成所有模块的父节点，而模块 README 又写“依赖 abi”。
2. `abi` 自称零依赖，但其 Root/错误行为必然需要 Error/Capability；这不是可实现 DAG。
3. `spatial` 使用 opaque index Handle，却未声明依赖 `handle`；如果重复实现一套 Handle，会破坏唯一生命周期原语。
4. `diagnostics` 既依赖 Error，又被其他模块“报告状态”，但未说明是编译期依赖、record port 还是运行时聚合。
5. 没有 Context 生命周期节点；所有资源图缺少 teardown root。
6. 没有区分“架构源生成契约依赖”和“Rust 实现 crate 依赖”。

## 5.2 推荐的无环依赖图

```text
LumioGameEngineArchitecture released artifacts (read-only, external authority)
                              |
                      contract-types (leaf)
                _________|___________
               /         |           \
            error      handle       memory      capability-static
                         |              |
                 context-token      buffer/lease primitives
                         \             /
                   private clock-port
                          \   |   /
                             job

spatial  -> contract-types + error + handle + memory
codec    -> contract-types + error + memory
            (+ handle only for stateful dictionary/workspace resources)

diagnostic-record-types (leaf, only after upstream approval)
                |
          diagnostics implementation

kernel-context / kernel-instance
  -> capability-static + handle + memory + job
  -> selected spatial/codec resources
  -> optional diagnostics recorder

native-core-ffi / provider facade
  -> kernel-context + selected public modules
  -> sole NativeCore public symbol surface

CoreEngine composition/root-abi
  -> combines NativeCore/Voxel provider contracts
  -> sole cross-repository Root API and managed binding package
```

## 5.3 每条调整的理由

1. **拆 `abi`：** 固定宽度类型必须是 leaf；Root facade 必须在顶部。用一个模块承担两者必然成环。
2. **新增 `kernel-context`：** Handle 的 Context、Worker/Pool/Index 生命周期必须有唯一 owner；它不拥有 World/Session 语义。
3. **Spatial 显式使用 Handle：** 跨调用索引不能私造第二套 generation/context 机制。
4. **Job 不依赖具体 Spatial/Codec：** 调度和 kernel 保持可组合；可以通过 operation registry/facade 运行时绑定。
5. **Diagnostics 不成为所有核心模块的硬依赖：** 热路径通过 optional record port、counter snapshot 或 facade 聚合；Sink 永不上移到 Kernel。
6. **CoreEngine 只向下组合，不被 NativeCore 反向依赖：** NativeCore 发布 provider artifact，CoreEngine 负责最终 Root/package/loader。

## 5.4 是否需要新增模块

- **需要：`kernel-context`/`kernel-instance`。** 这是缺失的生命周期根，属于 Foundation。
- **需要拆出：`contract-types`。** 它是精确的 ABI 叶子层，不是泛化 `common`。
- **不建议新增公共 `common`。** 容易成为无边界依赖桶。
- **不建议新增公共 `time`。** Host 拥有 Wall Clock/Pacing，Runtime 拥有 Logical Tick；NativeCore 只需 private monotonic deadline port。
- **建议新增但不属于稳定运行时模块：**
  - `tools/xtask`：contract sync/verify、header/layout、symbol、manifest、reproducibility；
  - `lumio-test-support` / `tests/conformance`：Fixture harness、fault injection、reference kernel；
  - `tests/ffi-c` 与未来 managed smoke；
  - `fuzz/`、`benches/`；
  - dependency/forbidden-boundary lint、contract-diff、ABI dump/symbol check。

## 5.5 只能是运行时可选关系、不能成为编译期依赖

- `spatial/codec -> job`：不允许；Job 可在运行时调度它们。
- `job -> Runtime Scheduler/Processor Graph`：不允许。
- 核心算法模块 `-> diagnostics implementation/sink`：不允许；只允许轻量 record port 或 facade 聚合。
- `NativeCore -> CoreEngine`：不允许；只能由 CoreEngine 组合 NativeCore artifact。
- `error -> diagnostics`：不允许。
- `memory -> job/spatial/codec`：不允许。

# 6. 契约完整性审查

| 契约项 | 结论 | 证据与判断 |
| --- | --- | --- |
| ABI | **部分完整** | 已规定 POD/Buffer/Handle、panic conversion、单 Root、批处理；但 Root owner、leaf/top 分层、每结构 header、长结构兼容和生成物锁定冲突/缺失 |
| 所有权/生命周期 | **缺失** | 有“创建侧释放”和 borrowed scope 原则；没有 Context owner、async lease、resource/slot/drop 顺序和 terminal reclaim |
| 线程/取消/超时 | **部分完整** | 有 bounded worker/queue、no managed callback、需线性化；没有状态机、竞态赢家、clock domain、shutdown/drain/abandon |
| 错误 | **部分完整** | Error code 归上游、文本非机器判定、panic/bounds/OOM 分类方向正确；payload retention、unknown code forward compatibility、与 diagnostics copy 边界待定 |
| Capability | **部分完整** | 正确排除 RoomMode/Role/权限/签名；static capabilities 与 limits/runtime status 混合 |
| 确定性 | **部分完整** | 明确不把地址/线程时序入 hash，要求 stable ordering；Spatial/Codec tie-breaker、float、completion order、backend determinism 尚未冻结 |
| 资源限制 | **部分完整** | 各模块要求 bounded queue/pool/ratio/budget；具体上限来源、failure atomicity、degrade policy、retained completion quota 缺失 |
| Diagnostics | **部分完整** | 正确区分 Sink/Audit/Journal，要求 bounded queue/EventSeq；redaction、fragment owner、hot-path port、Error/Fatal evidence 不闭合 |
| 版本演进 | **部分完整** | 有 Baseline/struct_size/main version 原则；缺完整兼容矩阵、contract lock、format/operation version policy |
| 测试和 Benchmark | **缺失（作为开工 Gate）** | 有类别清单，没有足够的机器可判输入/输出/失败状态和固定 workload manifest；尚不能证明 Architecture Gate 退出 |

# 7. 开始实现前必须完成的事项

## 7.1 必须先修改的文档（按顺序）

1. 修正 Root ABI 所有权和依赖图；明确 `contract-types` leaf、NativeCore provider facade、CoreEngine unified Root。
2. 增加 `kernel-context` 的职责、状态机、资源拥有表和 shutdown 顺序。
3. 把 Memory/Job 输入输出改成明确 Buffer classes、allocator provenance、lease 和 terminal reclaim。
4. 写出 Job 状态机、竞态表、Deadline clock domain、Worker close 与 completion reaping。
5. 修正“每个导出结构都有 capability_bits”与结构扩展规则；给出 version negotiation matrix。
6. 缩窄 Codec；将字段级 schema/canonical serializer 语义移回生成 Serializer。
7. 收紧 Diagnostics：allowlist、fragment-only、redaction defence-in-depth、optional recorder port。
8. 将 `repository-architecture.md` 的“时间”改为 private monotonic deadline port，不宣称拥有 wall/tick time。
9. 为模块增加 `BaselineStatus` 与 `RepositoryDeliveryPhase` 两套状态。

## 7.2 必须新增的 ADR

1. `ADR: NativeCore crate/module DAG and public symbol ownership`
2. `ADR: KernelContext lifecycle and teardown protocol`
3. `ADR: FFI Buffer classes, allocator provenance and async leases`
4. `ADR: Job state machine, cancellation, timeout and shutdown`
5. `ADR: ABI version negotiation and struct extension compatibility`
6. `ADR: Determinism contract for Spatial/Codec/Job completion`
7. `ADR: Diagnostics record port, bounded queues and FailureFragment ownership`
8. `ADR: Cargo workspace/crate/public-export map`

## 7.3 必须回到架构源确认的项目

1. `lumio_core_get_api_v1` 的唯一 owner、NativeCore provider 与 CoreEngine composition 方式。
2. Root table 与普通 request/result/batch/error 结构中 `abi_version/struct_size/capability_bits` 的精确规则。
3. Error/Capability/Handle/Buffer/Job 状态的公开 Schema、IDs 和正向/失败 Fixtures。
4. `codec` 是否成为独立 public module，以及 format/algorithm IDs。
5. `diagnostics` 是否成为独立 public module，以及 event/FailureFragment Schema。
6. 生成契约 artifact 的发布形式、source commit/compiler/hash/lock manifest。

## 7.4 可以延后到实现阶段的项目

- 具体 Worker queue 算法、allocator/pool 实现、BVH/Grid backend。
- SIMD/非 SIMD、更多压缩 backend 和缓存策略。
- Miri/Sanitizer/Soak 的具体 CI runner 配置，但命令/目标必须在首次 Cargo 提交同步建立。
- Diagnostics 具体日志生态 Adapter；稳定契约只定义 record/batch 边界。
- 性能阈值的最终数值，可先建立 workload/result schema 与 reference baseline。

## 7.5 可以明确不做的项目

- 不做泛化 `common` god crate。
- 不让 NativeCore 拥有 Wall Clock、Tick Phase、World/Session 生命周期。
- 不做第二套 ABI/Schema/Binding generator。
- 不让 Job 接受 managed delegate 或 caller function pointer 回调 Hot Gameplay。
- 不让 Codec 解释 Gameplay/Voxel/RPC/Snapshot 字段 Schema。
- 不让 Diagnostics 拥有文件 Sink、Audit、Txn Journal、Command Log 或完整 Failure Bundle replay。
- 不让 Spatial/Codec 对 Job 形成编译期依赖。

# 8. 残余风险和开放问题

### OPEN-001 — 最终 Root ABI 由 CoreEngine 还是 NativeCore 导出

- **为什么本地无法回答：** 本仓 README 与 Baseline RACI/模块地图表述不一致；只有公共架构源能裁决跨仓 ownership。
- **需要确认方：** `LumioGameEngineArchitecture` owner + `LumioCoreEngine` owner。
- **不确认会阻塞什么：** C symbol、Root API table、Header/Binding、static/dynamic composition 和 Loader 实现。

### OPEN-002 — `codec`/`diagnostics` 是否已经进入最新外部 Baseline

- **为什么本地无法回答：** 本地 v1.0 镜像明确未把二者列入 NativeCore 首批模块；v0.3 只是 deprecated pointer。外部架构仓最新状态未在本地证明。
- **需要确认方：** `LumioGameEngineArchitecture` owner。
- **不确认会阻塞什么：** 二者的公共 ABI、ID/Schema、NativeHeadless export；不阻塞其 private prototype。

### OPEN-003 — 公共 Error/Capability/ABI Fixture 的实际发布载体

- **为什么本地无法回答：** 本仓只声明消费生成物，没有 artifact package/lock/bootstrap 入口。
- **需要确认方：** Architecture contract-toolchain owner。
- **不确认会阻塞什么：** ABI/Header/Binding 的可复现实现和 Architecture Gate 自动验收。

### OPEN-004 — CoreEngine 的 Native composition 模式

- **为什么本地无法回答：** Baseline允许 static/dynamic linking 由 Manifest 唯一声明，但本仓没有 provider API 或 composition manifest 细节。
- **需要确认方：** CoreEngine/Loader owner。
- **不确认会阻塞什么：** crate type、symbol visibility、one Native instance、平台 package layout。

### OPEN-005 — Host/Runtime 如何映射 World/Session 销毁到 NativeCore Context/Epoch

- **为什么本地无法回答：** NativeCore 不应拥有 World/Session，但迟到结果必须由上层映射到逻辑 owner；本仓没有跨仓 handoff contract。
- **需要确认方：** Server Host + GameRuntime owner + Architecture owner。
- **不确认会阻塞什么：** late completion discard/apply policy、NativeJobBarrier 和 Context close handshake。

### OPEN-006 — 本地校验命令真实结果

- **为什么本地无法回答：** 本次环境没有用户本机 checkout，且无法执行仓库 Node 脚本/sha256 命令。
- **需要确认方：** 仓库维护者在 `/Users/cui/LumioGames/LumioNativeCore` 运行指定只读命令并保留退出码。
- **不确认会阻塞什么：** 只阻塞“当前工作树/结构校验已通过”的声明，不改变本报告对架构内容的 P0/P1 结论。

# 9. 最终放行判断

1. **九模块拆分是否合理？**
   - **部分合理，但不是最终可实现拆分。** Error、Handle、Memory、Job、Spatial 的方向正确；`abi` 必须拆分概念层；缺少 `kernel-context`；Codec/Diagnostics 需缩窄并完成上游批准。

2. **每个模块职责是否足够清晰？**
   - **否。** Error/Spatial 的领域边界较清楚；ABI、Handle/Memory/Job 生命周期、Capability 动静态边界、Codec schema 边界、Diagnostics fragment/redaction 边界不够完整。

3. **当前内容是否足以开始 Foundation？**
   - **不足。** 不建议实现稳定 ABI/Handle/Memory/Job。可先做不冻结契约的 workspace、CI、xtask、test-support、fixture manifest 脚手架，但必须先关闭所有 P0 文档/ADR问题。

4. **哪些模块需要重设计？**
   - `abi`：必须拆为 leaf contract types 与 top FFI/provider facade，并解决 CoreEngine Root ownership。
   - `job`：必须补完整状态机、clock、cancel/timeout/shutdown/reap。
   - `memory`：必须重写 async buffer ownership/provenance。
   - `handle`：保留，但必须接入 Context owner 和 typed arena/lease。
   - `codec`：缩窄为机械 byte kernel。
   - `diagnostics`：缩窄为 bounded records/fragments，不拥有完整 bundle/sink。

5. **是否存在遗漏的核心能力？**
   - **存在。** `kernel-context/kernel-instance` 生命周期根、精确 `contract-types` leaf、private monotonic clock/platform port，以及 conformance/contract-sync/ABI-smoke/fuzz/benchmark 工具体系。
   - 不应补一个泛化 `common`，也不应补公共 `time`。

6. **当前设计最大的问题是什么？**
   - **没有一份单一、可执行的生命周期与所有权协议，把 Context、Handle、Buffer、Job、取消/超时、Worker Close 和迟到 Completion 串成闭环。** 当前文档把义务分别写在多个模块中，但没有任何模块拥有终态裁决和资源回收，因此在真正写 Rust/C ABI 时，最危险的行为仍会由实现者自行解释。

---

## 推荐的 Foundation 放行门槛（一句话）

只有当“唯一 Root/无环依赖 + KernelContext 状态机 + Buffer/Job 所有权/终态 + 上游契约锁定 + P0 正向/失败 Fixture”全部成为机器可验证输入后，LumioNativeCore 才应从 Architecture Gate 进入 Foundation。
