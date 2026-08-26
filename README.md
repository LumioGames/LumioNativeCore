# LumioNativeCore

> LumioGameEngine v0.2 架构中的 Rust 原生公共基础库与高性能计算层。

## 定位

`LumioNativeCore` 承载不属于具体体素实现、服务器宿主或游戏业务的 Rust 公共能力。它与 `LumioVoxelEngine` 共同组成 v0.2 定义的 Rust Core Engine。

这里的 HPC（High-Performance Computing）是指游戏运行时需要的高性能批量计算能力，不是独立的云计算或超算平台。

## 职责

- 通用固定宽度 ID、Index + Generation Handle、Error Code、ABI Header 和 Capability。
- AOI 候选计算、Octree、BVH、Spatial Hash、Grid/Cell Index 等空间基础设施。
- 可复用碰撞查询、寻路与导航计算内核。
- Snapshot Diff、Delta Encoding、压缩和序列化热路径。
- 有界 Worker Pool、Typed Native Job、批处理、内存池和 SIMD Kernel。
- 为 Native 能力发布版本化 Rust API、C ABI 与平台产物。

## 依赖关系

### 上游依赖

- 无其他 Lumio 仓库依赖。它是依赖图最底层的原生基础库。

### 下游使用者

- [`LumioVoxelEngine`](https://github.com/LumioGames/LumioVoxelEngine)：复用空间、碰撞、压缩和 Job 能力。
- [`LumioServer`](https://github.com/LumioGames/LumioServer)：直接链接原生计算能力。
- [`LumioClient`](https://github.com/LumioGames/LumioClient)：通过平台原生库和托管适配层使用计算能力。
- [`LumioGame`](https://github.com/LumioGames/LumioGame)：只通过锁定的底层发布产物间接组合，不复制源码。

```text
LumioNativeCore
├─> LumioVoxelEngine
├─> LumioServer
└─> LumioClient
```

## 契约所有权

本仓库是通用 Native ID、Handle、Error、Capability 和 ABI 基础结构的唯一事实源。跨语言结构必须显式携带版本和结构大小；破坏性修改必须产生新的 ABI 版本。

## 禁止事项

- 禁止实现或保存 Voxel World、Chunk 和体素权威数据。
- 禁止创建、销毁或直接访问 C# ECS Entity、Component Storage 和 System 生命周期。
- 禁止包含 Gameplay、技能、任务、经济、建造权限等业务语义。
- 禁止承担网络连接、Session、服务器进程或 CoreCLR 生命周期。
- 禁止让 Native Worker 保存或调用 C# Delegate、托管对象或热更方法地址。
- 禁止依赖 `LumioVoxelEngine`、`LumioGameRuntime`、`LumioServer`、`LumioClient` 或 `LumioGame`，避免依赖反转。

## 当前状态

`v0.1.0` 仅冻结仓库职责与依赖边界；尚未发布代码或软件包。

