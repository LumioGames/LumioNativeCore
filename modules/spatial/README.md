# spatial

> 提供领域无关的 Grid、Hash、BVH、邻域查询和批量距离 Kernel。

**优先级**：P1  
**实施阶段**：NativeHeadless  
**架构基线**：`LGE-V1.0-2026-08-27`

当前阶段是本仓实现规划；架构镜像将 `spatial` 列入 NativeCore 首批模块，具体交付批次以架构源和本仓计划的同步结果为准。

## 负责范围

- 接收通用坐标、半径、索引键和批量数据，执行空间插入、删除、邻域和距离计算。
- 提供 Grid、Hash、BVH 等可替换实现的统一 Kernel 边界。
- 声明结果排序、重复输入、边界坐标和数值精度规则。
- 输出可被上层 Voxel、Runtime 或 Gameplay Adapter 组合的纯计算结果。

## 不负责范围

- 不创建或保存 VoxelWorld、Chunk、ECS Entity、AOI 或碰撞业务状态。
- 不决定权限、可见性、阵营、玩家或产品语义。
- 不直接访问网络、托管对象或上层 World Storage。

## 输入、输出与所有权

输入和输出均为调用方拥有的版本化批次；索引内部存储不得泄漏地址。查询结果不足时返回所需长度或明确截断状态。跨调用保留的索引必须通过不透明 Handle 管理，并在 Context 销毁时失效。

## 依赖与约束

依赖 `abi`、`error` 和 `memory`；可由 `job` 调度但不依赖 `job` 的具体实现。第三方空间库通过 Adapter 隔离。确定性输出必须使用显式排序规则，不把线程时序或地址纳入权威 Hash。

## 线程、错误与观测

读写并发、快照查询和重建期间的可见性必须明确。容量不足、非法坐标、重复键、版本不匹配和索引损坏都返回稳定错误。重建耗时、候选数量、分配和缓存命中率进入 Diagnostic Event/Metric。

## 测试与性能

- Grid/Hash/BVH 的插入、删除、边界、重复项和空查询。
- 与简单 Reference Kernel 的差分测试、确定性排序和并发读写。
- 固定点分布和批次大小测量吞吐、候选数、p95/p99、分配和峰值内存。

## 版本演进

算法和内部布局可以替换；改变坐标单位、结果排序、精度、溢出或 Handle 语义时必须更新契约和 Fixture。SIMD、分层索引和新后端作为后续扩展，不改变已发布 ABI 主版本。

## 相关

- [Memory 模块](../memory/README.md)
- [Job 模块](../job/README.md)
- [仓库边界与架构契约](../../.spec/knowledge/standards/repository-architecture.md)
- [根 README](../../README.md)
