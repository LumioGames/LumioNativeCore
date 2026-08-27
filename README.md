# LumioNativeCore

> 跨项目复用的 Rust Native Kernel、内存/Job 原语和领域无关高性能计算层。

## 架构基线

- Baseline：`LGE-V1.0-2026-08-27`
- 唯一架构源：`LumioGameEngineArchitecture`
- 本地镜像：[`docs/architecture/LumioGameEngine_Architecture_v1.0.md`](docs/architecture/LumioGameEngine_Architecture_v1.0.md)

`LumioNativeCore` 位于依赖图最底层。它提供可被多个产品复用、与 Voxel/Gameplay/网络/Host 无关的 Rust Kernel 和稳定 C ABI 基础。它不是 VoxelWorld、ECS Runtime 或游戏内容运行时。

## Architecture Gate

公共 ABI/Capability/Error Schema、ID Registry、正向/失败 Fixture 和契约校验器只维护在 `LumioGameEngineArchitecture`。本仓库在提交 ABI 或布局变更前，必须消费已发布 Baseline 和生成物，并在架构源执行 `python3 tools/lumio_contract.py validate`；不在这里复制第二套 Schema 或绑定生成器。

## 拥有的状态与生命周期

- Kernel 内部有界 Worker、Job、内存池、索引和临时批次状态。
- Native Handle 的 `Index + Generation` 生命周期、Context 校验和 Capability 状态。
- 调用期 Buffer、空间索引、通用 Diff/压缩任务和诊断计数器。

调用方创建并销毁 Handle/Buffer；进程、World、Session 和 Gameplay 生命周期由上层 Host 管理。NativeCore 不保存托管对象、业务实体、回调地址或跨调用的领域引用。

## 子模块

| 子模块 | 责任 | 首批状态 |
| --- | --- | --- |
| `abi` | 固定宽度类型、结构版本、Buffer、Root API Table | P0 |
| `handle` | Index/Generation/Context Handle、重复释放和失效检查 | P0 |
| `error` | 稳定 Error Code、错误类别和诊断载荷 | P0 |
| `capability` | 平台/编译 Feature/能力位声明 | P0 |
| `memory` | 调用方 Buffer、Allocator 边界、内存池和统计 | P0 |
| `job` | 有界 Worker、Typed Job、取消、超时和 Completion Batch | P0 |
| `spatial` | Grid、Hash、BVH、邻域和批量距离 Kernel | P1 |
| `codec` | 领域无关的批量 Diff、压缩和序列化热路径 | P1 |
| `diagnostics` | Native Metrics、Trace Event 和 Failure Bundle 片段 | P1 |

`codec` 只提供机械编码能力，不定义 RPC、Gameplay Schema、Voxel Revision 或权限语义。

## 职责

- 冻结 `NativeManagedAbiV1` 所需的基础类型、结构长度、对齐、Error Code 和 Capability。
- 提供线程安全、可取消、有界的 Native Job 和批处理 API。
- 提供可复用空间、碰撞基础计算、压缩和 Diff Kernel；领域策略由上层组合。
- 输出平台静态/动态库、ABI Header、符号、SBOM 和 Benchmark 结果。
- 为 ABI Layout、失效 Handle、内存泄漏、重复加载和异常转换提供 Conformance Fixture。

## 明确不负责什么

- 不创建或保存 `VoxelWorld`、Chunk、Block、Revision、Mutation 或 Mesh 权威数据。
- 不创建、销毁或访问 C# ECS Entity、Component、GAS 或 Gameplay 状态。
- 不定义 Connection、Session、RPC、Release Pool、CoreCLR、ALC 或 Host Profile。
- 不保存 C# Delegate、托管对象、热更方法地址、裸指针或跨线程回调。
- 不把通用算法包装成含有玩家、技能、权限、阵营或产品名称的领域 API。

## Native/Managed ABI 契约

公共入口采用单一 Root API Table（例如 `lumio_core_get_api_v1`）。每个导出结构包含 `abi_version`、`struct_size` 和 `capability_bits`；跨边界只传固定宽度 POD、版本化 Buffer 和不透明 Handle。

- 不跨边界传 Rust/C# 容器、对象引用、异常或函数 Delegate。
- 内存由创建侧释放，优先使用调用方提供的 Buffer，并返回所需长度。
- Rust 在 FFI 边界捕获 panic 并转为稳定错误；不让 panic 穿过 ABI。
- Native Worker 不回调托管代码；异步结果在规定 Barrier 由上层消费。
- 句柄必须校验 Generation、Context 和释放状态；重复释放返回稳定错误。
- ABI 结构布局、枚举/布尔宽度、字节序、字符串编码、取消、线程亲和和可重入性都进入版本化 Header。

## 线程、资源与故障

Kernel Worker 使用有界队列和明确 Deadline；队列满载返回可诊断错误，不无限分配。长时间 Job 必须可取消或超时，超时结果不得自动写入已销毁 World。NativeCore 只报告 Fault Code，由 Server/Runtime 决定 Session、World 或进程级处置。

## 日志与观测

本仓库只产生 Native Diagnostic Event、Metrics 和 Trace，不拥有服务器日志 Sink。事件通过稳定批次输出，至少带 `ProductId、GameReleaseId、SessionId、WorldId、TickId、TraceId`（若上下文可用）。Rust 使用成熟日志生态接入异步 Sink；上层负责轮转、审计、保留和外部导出。

## 序列化与持久化边界

NativeCore 可以提供 Canonical Buffer、Diff、压缩和校验 Kernel，但不决定 Snapshot/WAL 文件格式。Voxel、Runtime 和 Game 各自拥有领域 Schema；CoreEngine 负责把 ABI 产物纳入发布包。任何需要持久化的 Buffer 都必须带 SchemaVersion、Length、Hash/Checksum 和明确的拥有者。

## Source / Compile-Time Dependencies

- Rust toolchain、平台 SDK 和经过供应链审查的通用 Rust crates。
- 不依赖任何 Lumio 上层仓库源码，不允许路径依赖反向引用 Voxel、Runtime、Server、Client 或 Game。
- 领域无关的第三方库通过内部 Adapter 隔离，不能把第三方类型暴露在 ABI。

## Generated Contract Dependencies

本仓库维护 Native ABI 源 Schema、Header、Error/Capability 清单和布局测试输入。`LumioCoreEngine` 消费这些只读产物并生成统一 Root 包和托管绑定；本仓库不再独立生成第二套 C# 最终绑定。

## Runtime Loading Relationships

```text
LumioCoreEngine platform package
  -> Host Loader
  -> one NativeCore instance per process
  -> Runtime/Server/Voxel adapters
```

同一进程拒绝加载第二套不兼容 NativeCore；LocalEmbedded 的 Server/Client Role 共享一个 Native 包，但不共享 World 数据。

## Release Composition Relationships

NativeCore 发布独立版本、Commit、平台 Artifact Hash、ABI 主版本、Feature、Compiler 和 SBOM，由 CoreEngine 锁定并签名。它不决定 `ProductId` 或 `GameReleaseId`，也不直接组成游戏发布。

## Room Modes / Host Profiles

同一 Kernel API 可用于 `PureHeadless`、`NativeHeadless`、`LocalEmbedded`、`LocalSplitProcess`、`RemoteDS` 和 `MobileLocal`。模式、网络、Role 和资源预算不进入 Kernel；差异由调用方 Capability 和调度层表达。

## Headless Test Surface

- ABI Layout/Smoke、Handle 生命周期、错误码、并发、取消、重复释放和内存泄漏。
- Miri/Sanitizer/线程模型检查、跨平台构建和符号检查。
- Spatial/Codec/Job Benchmark，记录吞吐、p95/p99、分配和峰值内存。
- Fault Fixture：失效 Handle、Buffer 不足、Job 超时、panic 转换、重复加载和 Capability 缺失。

测试结果使用统一 Failure Bundle 字段；State Hash 只覆盖明确标记为确定性的 Kernel 输出，不把缓存地址或线程时序纳入权威 Hash。

## Version / Manifest

Manifest 至少记录 Commit、平台/架构、Compiler、Feature、ABI、Capability、Artifact Hash、符号、SBOM 和许可证。ABI 主版本变化必须拒绝旧组合的静默加载；所有组合由 CoreEngine Loader 校验。

## 开源优先与供应链

满足需求时优先采用成熟开源 Kernel、并发、编码和诊断方案；选择必须通过许可证、维护状态、漏洞、性能、确定性和目标平台验证。默认优先 MIT、Apache-2.0、BSD、Zlib 等宽松许可证；依赖通过 Adapter、锁定 Commit、SBOM 和扫描流程管理。

## 开发规范

- 公共 API 先写结构布局、所有权、线程、取消、错误和 Benchmark，再实现。
- 任何逐 Entity/逐 Voxel/逐包 FFI 设计必须先证明批处理不足，不得把调用开销扩散到上层。
- Native 优化不得引入 Voxel、Gameplay、Session 或网络语义。
- 结果必须可诊断、可取消、可重复；不以“调用方自行保证”替代契约。

## 当前阶段与开发节奏

1. **Architecture Gate**：冻结 ABI/Header、Error、Capability、Handle 和布局/故障 Fixture。
2. **Foundation**：实现 `abi/handle/error/memory/job` 最小闭环和 CI。
3. **NativeHeadless**：加入 spatial/codec Kernel、跨平台 Benchmark 和 CoreEngine 包加载。
4. **Production Hardening**：补齐 Sanitizer、可复现构建、符号/SBOM、性能曲线和崩溃证据。
5. **P2**：更深 SIMD、更多空间算法和可选后端；不得改变已发布 ABI 主版本语义。
