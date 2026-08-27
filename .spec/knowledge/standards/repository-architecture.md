---
name: repository-architecture
description: 仓库边界与架构契约——Native Kernel、FFI 所有权和 Architecture Gate;改 ABI、内存或 Job 原语前查
metadata:
  type: doc
  status: 已交付
---

# 仓库边界与架构契约

## 规范来源与优先级

- Agent 的开发流程、测试政策和交付规则以 `.spec/` 为权威。
- 模块边界以根 [`README.md`](../../../README.md) 为本仓入口；共享架构以 `LumioGameEngineArchitecture` 的 `LGE-V1.0-2026-08-27` 为唯一来源，本仓 [`架构镜像`](../../../docs/architecture/LumioGameEngine_Architecture_v1.0.md) 只读。
- 冲突时不得在 Kernel 内自行改写公共 ABI/Capability；先在架构源完成 ADR、Schema、Fixture 和新 Baseline。

## 所有权边界

- 本仓拥有通用 Handle/Buffer、Allocator、Job/Queue、空间/碰撞、Codec/Compression、时间与 Diagnostic Kernel。
- 本仓不拥有 VoxelWorld/Chunk、ECS、Gameplay、Session、网络、Host 或产品语义；任何优化都必须保持领域无关。
- 公共边界是批处理、版本化、可取消的 C ABI；托管调用只能消费生成 Binding，不得持有裸指针或内部 Rust 引用。
- 第三方 crate 经 Adapter 隔离并锁定版本/Commit，不能把供应商类型写进稳定 ABI。

## Architecture Gate

- 公共 API 先定义结构布局、所有权、线程、取消、错误、资源上限与 Benchmark，再实现。
- 逐 Entity/逐 Voxel/逐包 FFI 必须先证明批处理不足；不得把跨边界调用开销扩散到上层。
- 结果必须可诊断、可取消、可重复；不能以“调用方自行保证”替代契约，panic 必须在 ABI 边界转换为稳定错误。
- ABI/Capability/Error Schema、ID 与 Fixture 只在架构源维护；本仓消费已发布 Baseline，不复制生成器或第二套 Schema。
- 性能改动记录吞吐、p95/p99、分配、峰值内存、硬件/构建配置和结果确定性范围。
