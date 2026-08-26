# LumioNativeCore

> 跨项目复用的 Rust Native Kernel 与高性能通用计算层。

## 定位

`LumioNativeCore` 是依赖图最底层的通用 Native 库。它只提供与具体 Voxel 领域、网络协议、CoreCLR Host 和游戏业务无关的内存安全 Rust Kernel、批量计算和稳定 C ABI。它不是 Voxel World，也不是一个可直接被游戏内容引用的运行时。

总架构基线见 [`docs/architecture/LumioGameEngine_Architecture_v0.3.md`](docs/architecture/LumioGameEngine_Architecture_v0.3.md)。

本仓库的底层代码和高性能模块统一使用 Rust；C ABI 是给 C# Runtime/Host 使用的边界，不在本仓库承载 C# 热更代码。

## 拥有的状态与生命周期

- Kernel 内部的有界 Worker、内存池、索引和临时 Job 状态。
- Native Handle 的 Index/Generation 生命周期与 Capability 状态。
- Snapshot/Delta 编码器、空间索引和计算任务的调用期状态。

所有权由调用方显式创建和销毁；不得跨边界保存托管对象或业务实体。进程生命周期由 `LumioServer`、`LumioClient` 或测试 Host 管理。

## 职责

- 固定宽度 ID、Index + Generation Handle、Error Code、Capability 和 ABI 基础类型。
- 通用 Spatial Hash、Grid/Cell Index、Octree、BVH、批量距离/邻域查询等空间 Kernel。
- 可复用碰撞查询、导航计算、Snapshot Diff、Delta Encoding、压缩和序列化热路径。
- 有界 Worker Pool、Typed Native Job、批处理、内存池、SIMD Kernel 和诊断计数器。
- 发布版本化 Rust API、统一 C ABI Header、平台静态/动态库和 Native Benchmark。

AOI、Streaming、Collision 等若需要体素语义，只能由 `LumioVoxelEngine` 在上层组合这些通用 Kernel。

## 明确不负责什么

- 不创建或保存 Voxel World、Chunk、Block、Revision、Voxel Mutation 或 Mesh 权威数据。
- 不创建、销毁或访问 C# ECS Entity、Component Storage、GAS 或 Gameplay 状态。
- 不定义 Connection、Session、RPC Payload、Server 进程、CoreCLR 生命周期或 Host Profile。
- 不保存 C# Delegate、托管对象、热更方法地址或跨线程裸指针。
- 不依赖 `LumioVoxelEngine`、`LumioGameRuntime`、`LumioServer`、`LumioClient` 或 `LumioGame`。

## 对外产物与契约

- `liblumio_native_core`：各平台 `.a`/`.so`/`.dylib`/`.dll` 或等价产物。
- `lumio_native_core.h`：带 ABI 版本、结构大小、对齐、错误码和 Capability 的 C ABI Header。
- Rust crate、API 文档、Benchmark 结果、符号包和 SBOM。

破坏性 ABI 修改必须提升 ABI 主版本；所有跨语言结构必须校验版本与 `size_of`，并通过 `LumioCoreEngine` 统一发布。

## Source / Compile-Time Dependencies

- Rust toolchain、平台 SDK 和经审核的通用 Rust crates。
- 无其他 Lumio 仓库的源码依赖；不得通过路径依赖反向引用领域仓库。

## Generated Contract Dependencies

由本仓库生成 C ABI Header、Rust bindgen 元数据、Capability 表和错误码清单。`LumioCoreEngine` 读取这些生成物并将其与 `LumioVoxelEngine` 组合；上层不自行复制定义。

## Runtime Loading Relationships

```text
LumioCoreEngine platform package
  -> LumioServer / LumioClient native loader
  -> NativeCore symbols selected by Capability + ABI version
```

Server 和 Client 不应各自再加载一份独立 NativeCore；同一进程由 `LumioCoreEngine` 统一解析和加载。GameRuntime 通过稳定托管契约间接使用。

## Release Composition Relationships

`LumioNativeCore` 先发布可复用版本，由 `LumioCoreEngine` 锁定 Commit、平台产物 Hash 和 ABI 版本。它不直接组成游戏发行包，也不决定 Server/Client 的 Game Release。

## Room Modes / Host Profiles

对 `PureHeadless`、`NativeHeadless`、`LocalEmbedded`、`LocalSplitProcess`、`RemoteDS` 和 `MobileLocal` 提供相同 Kernel API。模式、传输和 Role 不进入 Kernel；差异只体现在调用方的调度与资源预算。

## Headless Test Surface

- Kernel 单元测试：Handle、边界、错误码、并发和内存生命周期。
- Native Benchmark：空间查询、碰撞、编码压缩、Job 调度和分配热点。
- Sanitizer/Miri/线程模型检查，以及跨平台 ABI Smoke Test。
- 测试输出包含吞吐、延迟、分配次数、峰值内存、State Hash（适用时）和符号化日志。

## Version / Manifest

- Rust crate 与 ABI 遵循 SemVer；ABI 主版本变化不得静默覆盖旧产物。
- 发布清单必须包含 Commit、平台/架构、编译器、Feature、ABI 版本、Artifact Hash、符号包和 SBOM。
- `LumioCoreEngine` 和上层 Host 启动时校验 ABI/Capability；不匹配即拒绝加载。

## 开发规范

- 公共 API 先定义错误码、所有权、线程约束、结构版本和 Benchmark，再实现 Kernel。
- 所有句柄 API 必须支持显式 Drop/Destroy、Generation 校验和重复释放检测。
- Native Job 不得回调托管代码；结果通过 Typed Buffer 或版本化批次返回。
- 性能优化必须有可重复基准和最小数据集，不能以引入业务语义为代价。
- 仅将确实跨项目复用且与领域无关的能力下沉到本仓库。

## 当前阶段任务

- 冻结 v0.3 C ABI 基础类型、Capability 表和错误码清单。
- 建立 Handle/空间 Kernel/编码器的最小实现与 Benchmark CI。
- 提供可被 `LumioCoreEngine` 消费的多平台构建、签名和 Manifest 产物。
