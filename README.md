# LumioNativeCore

> 跨项目复用的 Rust Native Kernel、内存/Job 原语和领域无关高性能计算层。

<!-- lumio-community:start -->
<div align="center">
<table>
<tr>
<td align="center" width="50%" valign="top">
<a href="https://qm.qq.com/q/PGkXh4tCyQ"><img src="https://raw.githubusercontent.com/LumioGames/.github/main/profile/assets/qr-qq.svg" width="170" alt="QQ 交流群 972220164"></a><br>
<a href="https://qm.qq.com/q/PGkXh4tCyQ"><img src="https://img.shields.io/badge/QQ%20%E4%BA%A4%E6%B5%81%E7%BE%A4-972220164-6171F0?style=for-the-badge&logo=tencentqq&logoColor=white" alt="QQ 交流群 972220164"></a><br>
<sub>什么都能聊</sub>
</td>
<td align="center" width="50%" valign="top">
<a href="https://applink.feishu.cn/client/chat/chatter/add_by_link?link_token=fffn1ae7-fd83-4315-96ac-6fa3aba3968e"><img src="https://raw.githubusercontent.com/LumioGames/.github/main/profile/assets/qr-engine.svg" width="170" alt="LumioEngine 开发者社区"></a><br>
<a href="https://applink.feishu.cn/client/chat/chatter/add_by_link?link_token=fffn1ae7-fd83-4315-96ac-6fa3aba3968e"><img src="https://img.shields.io/badge/%E9%A3%9E%E4%B9%A6%E7%BE%A4-LumioEngine%20%E5%BC%80%E5%8F%91%E8%80%85%E7%A4%BE%E5%8C%BA-5DE2C6?style=for-the-badge&logoColor=1E2A3A" alt="LumioEngine 开发者社区"></a><br>
<sub>飞书话题群 · Rust / C# 引擎层</sub>
</td>
</tr>
</table>
<sub>先进群再看代码。其它群和整体介绍见 <a href="https://github.com/LumioGames">LumioGames 主页</a>。</sub>
</div>
<!-- lumio-community:end -->
## 这个仓是什么

`LumioNativeCore` 位于依赖图最底层，是一组**纯 Rust 内核 crate**：通用 Handle/Buffer/Allocator、有界 Job、KernelContext 生命周期、空间与碰撞计算、单一定时内核。它与 Voxel、Gameplay、Session、网络和 Host 无关，不是 VoxelWorld、ECS Runtime 或游戏内容运行时。

本仓**不导出 C 符号、不发布独立平台产物、不维护任何 ABI Schema**。跨语言边界与其唯一真值 `engine/abi/native-abi.json` 都在架构仓 `LumioGameEngine`（Living Architecture，ADR-059）；本仓只以 Rust 源码形态被那边的 SDK 编入。

## crate 一览

| crate | 责任 | 状态 | 模块文档 |
| --- | --- | --- | --- |
| `lumio-platform` | 可注入 monotonic clock port 与 tick 标量 | 已交付 | — |
| `lumio-kernel` | error / capability / handle / memory / kernel-context 五个模块的编译承载，生命周期根 | 已交付 | [`error`](modules/error/README.md) · [`capability`](modules/capability/README.md) · [`handle`](modules/handle/README.md) · [`memory`](modules/memory/README.md) · [`kernel-context`](modules/kernel-context/README.md) |
| `lumio-job` | 有界 Worker、Typed Job、取消、超时与 Completion Batch | 已交付 | [`job`](modules/job/README.md) |
| `lumio-timer` | 单一定时内核：`wallClock` + `tickFrame` 双模式、CallbackSlot | 已交付 | [`timer`](modules/timer/README.md) |
| `lumio-spatial` | Grid / BVH / 邻域 / 批量距离与碰撞基础计算 | 已交付 | [`spatial`](modules/spatial/README.md) |
| `lumio-codec` | 纯字节压缩、校验和 Diff Kernel | 私有原型，feature 默认关 | [`codec`](modules/codec/README.md) |
| `lumio-diagnostics` | Native Metrics / Trace Event / FailureFragment | 私有原型，feature 默认关 | [`diagnostics`](modules/diagnostics/README.md) |
| `lumio-test-support` | dev-only 测试辅助（fake clock、泄漏计数、交错回放） | 已交付 | — |
| `xtask` | 仓库工程检查（依赖 DAG、产物形态断言） | 已交付 | — |

依赖方向沿下图形成无环图；箭头是编译期概念依赖，不是运行时调用方向（完整口径见 [`native-core-module-map.md`](docs/specs/native-core-module-map.md)）：

```text
platform（零依赖叶子：clock port）
├── kernel -> platform（error / capability / handle / memory / kernel-context）
├── job -> kernel + platform
├── timer（零依赖：自带确定性刻度与单调毫秒，不读墙钟）
├── spatial -> kernel
├── codec -> kernel（pending，feature-gated）
└── diagnostics -> kernel + platform（pending，feature-gated）
```

`job` / `spatial` / `codec` 的跨调用资源实现 `ContextResource` port 并注册进 `kernel-context`（编译期方向指向 `lumio-kernel`，避免成环）。禁止方向由 `cargo xtask check-dep-dag` 强制。任何第三方类型都必须停留在 Adapter 内。

## SDK 如何编入

架构仓 `LumioGameEngine` 的 `engine/native/modules/sdk-native`（crate `lumio-engine-native`）以 **Cargo 路径依赖**直接引用本仓 crate 源码，编进它自己的 `cdylib`：

```toml
lumio-kernel = { path = ".../LumioNativeCore/crates/lumio-kernel" }
lumio-timer  = { path = ".../LumioNativeCore/crates/lumio-timer" }
```

托管侧可达面由那边的 `engine/abi/native-abi.json` 单独定义，插头代码也在那边。定时内核是当前唯一有完整消费链的模块：**内核 `lumio-timer` 在 NativeCore；C ABI 插头在架构仓 `engine/native/modules/sdk-native/src/timer.rs`；经 `engine/abi/native-abi.json` 的 `timer_*` 槽到达托管侧。**

因此改本仓 crate 的公开 Rust API 即是改 SDK 的编译输入：改完必须在架构仓 `engine/native` 复跑 `cargo build -p lumio-engine-native` 与 `cargo test -p lumio-engine-native`。

## 职责

- 提供线程安全、可取消、有界的 Native Job 和批处理 API。
- 提供 Native Handle 的 `Index + Generation` 生命周期、Context 校验与重复释放检查。
- 提供可复用空间、碰撞基础计算、压缩和 Diff Kernel；领域策略由上层组合。
- 提供单一定时内核，确定性 tick 调度与单调墙钟到期共用一份实现。
- 内存所有权用三个术语区分：resource owner（跨调用资源唯一登记在所属 KernelContext）、handle holder（调用方持有 handle 并发起释放）、allocator（谁分配谁释放，见 [`ffi-buffer-ownership.md`](docs/specs/ffi-buffer-ownership.md)）。

## 明确不负责什么

- 不创建或保存 `VoxelWorld`、Chunk、Block、Revision、Mutation 或 Mesh 权威数据。
- 不创建、销毁或访问 C# ECS Entity、Component、GAS 或 Gameplay 状态。
- 不定义 Connection、Session、RPC、Release Pool、CoreCLR、ALC 或 Host Profile。
- 不保存 C# Delegate、托管对象、热更方法地址、裸指针或跨线程回调。
- 不定义、不镜像、不校验任何跨语言 ABI Schema、Header、ID Registry 或状态码——那是架构仓 `native-abi.json` 的事。
- 不把通用算法包装成含有玩家、技能、权限、阵营或产品名称的领域 API。
- 不依赖任何 Lumio 上层仓库源码，不允许路径依赖反向引用 Voxel、Runtime、Server、Client 或 Game。

## 线程、资源与故障

Kernel Worker 使用有界队列和明确 Deadline；队列满载返回可诊断错误，不无限分配。长时间 Job 必须可取消或超时。`ErrorCategory` 是内部枚举，跨边界的状态码由架构仓插头对 `native-abi.json` 决定；本仓不持有跨边界数值。NativeCore 只报告失败类别，由上层决定 Session、World 或进程级处置。

## 测试面

- Handle 生命周期、错误类别、并发、取消、重复释放与内存泄漏计数。
- 确定性回放：定时内核的到期顺序、交错命名回放、参考实现对拍。
- Spatial / Job Benchmark，记录吞吐、p95/p99、分配和峰值内存。
- 故障用例：失效 Handle、Buffer 不足、Job 超时、panic 转换、Capability 缺失。

State Hash 只覆盖明确标记为确定性的 Kernel 输出，不把缓存地址或线程时序纳入权威 Hash。

## 收口门槛

改动提交前，下列命令必须全部 exit 0：

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo build --workspace
cargo test --workspace
cargo xtask check-dep-dag
cargo xtask assert-no-native-artifacts
node .spec/tools/spec-lint.mjs
node --test .spec/tools/spec-lint.test.mjs
```

触及本仓公开 Rust API 时，还必须在架构仓 `engine/native` 复跑 `cargo build -p lumio-engine-native` 与 `cargo test -p lumio-engine-native`。

## 开源优先与供应链

满足需求时优先采用成熟开源 Kernel、并发、编码和诊断方案；选择必须通过许可证、维护状态、漏洞、性能、确定性和目标平台验证。默认优先 MIT、Apache-2.0、BSD、Zlib 等宽松许可证；依赖通过 Adapter、锁定版本和扫描流程管理，并登记在 `cargo xtask check-dep-dag` 的外部依赖白名单里。

## 开发规范

- 公共 Rust API 先写结构布局、所有权、线程、取消、错误和 Benchmark，再实现。
- 任何逐 Entity/逐 Voxel/逐包的细粒度设计必须先证明批处理不足。
- Native 优化不得引入 Voxel、Gameplay、Session 或网络语义。
- 结果必须可诊断、可取消、可重复；不以“调用方自行保证”替代契约。
