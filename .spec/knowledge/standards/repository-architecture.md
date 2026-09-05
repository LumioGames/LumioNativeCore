---
name: repository-architecture
description: 仓库边界与架构契约——Native Kernel 所有权、稳定边界是公开 Rust API、SDK 路径依赖;改公开 API、内存或 Job 原语前查
metadata:
  type: doc
  status: 已交付
---

# 仓库边界与架构契约

## 规范来源与优先级

- Agent 的开发流程、测试政策和交付规则以 `.spec/` 为权威。
- 模块边界以根 [`README.md`](../../../README.md) 为本仓入口，编译边界见 [`native-core-module-map.md`](../../../docs/specs/native-core-module-map.md)。
- 跨语言可达面、状态码与槽位的唯一真值是架构仓 `LumioGameEngine` 的 `engine/abi/native-abi.json`，插头代码也在那边；本仓不保存镜像、不复制 Schema、不为内核类型申请状态码（[ADR 0009](../../decisions/0009-exit-legacy-contract-regime.md)）。
- 冲突时不得在 Kernel 内自行定义跨语言合同；先在架构仓改 `native-abi.json` 与插头。

## 所有权边界

- 本仓拥有通用 Handle/Buffer、Allocator、Job/Queue、KernelContext 生命周期、空间/碰撞 Kernel、单一定时内核（ADR-056 §7 / ADR 0008），以及处于 pending 的 Codec/Diagnostics 原型（ADR 0005）。
- 定时内核的口径固定为一句话：**内核 `lumio-timer` 在 NativeCore；C ABI 插头在架构仓 `engine/native/modules/sdk-native/src/timer.rs`；经 `engine/abi/native-abi.json` 的 `timer_*` 槽到达托管侧。**
- 时间：私有可注入 monotonic clock port（ADR 0004）供 Job deadline；定时内核 wallClock 模式拥有单调毫秒到期，tickFrame 模式只认确定性问题刻度；不拥有 TickId（归 Runtime）；时钟读数只进诊断，不进权威 Hash。
- 本仓不拥有 VoxelWorld/Chunk、ECS、Gameplay、Session、网络、Host 或产品语义；任何优化都必须保持领域无关。Capability 键是不透明数值，键空间由嵌入方定义，本仓不保存键名表。
- **稳定边界是 crate 的公开 Rust API，不是 C ABI。** 本仓只产 rlib，仓内不得出现 `cdylib` / `staticlib` 目标（`cargo xtask assert-no-native-artifacts` 强制）；架构仓 `engine/native/modules/sdk-native` 以 Cargo 路径依赖把本仓源码编进它自己的动态库。
- 第三方 crate 经 Adapter 隔离并锁定版本/Commit，供应商类型不得出现在公开 API 上。

## 变更顺序

- 公开 Rust API 先定义结构布局、所有权、线程、取消、错误、资源上限与 Benchmark，再实现。
- 改到公开 Rust API 即改 SDK 的编译输入：必须在架构仓 `engine/native` 复跑 `cargo build -p lumio-engine-native` 与 `cargo test -p lumio-engine-native`，且**不得为了让它编过而改架构仓文件**——编不过就停下上报。
- 逐 Entity/逐 Voxel/逐包的细粒度设计必须先证明批处理不足。
- 结果必须可诊断、可取消、可重复；不能以“调用方自行保证”替代契约，panic 不得穿过 SDK 的 FFI 边界（捕获在架构仓插头一侧）。
- 不恢复任何已删的 Schema / Fixture / 镜像工具链：那套 Baseline 复印制度已由 ADR-059 与本仓 ADR 0009 废止。
- 性能改动记录吞吐、p95/p99、分配、峰值内存、硬件/构建配置和结果确定性范围。
