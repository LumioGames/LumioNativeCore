# 0002 · 新增 kernel-context 模块作为 NativeCore 生命周期根

- 日期:2026-08-27
- 状态:生效

## 背景

架构 Review `ARCH-P0-002`：Handle 用 Context 校验、Spatial 索引随 Context 失效、Job 结果
不得写入已销毁 World，但九个模块没有任何一个拥有 Context 的创建、排空、关闭与 Epoch 失效——
资源关闭无法形成闭环，UAF/泄漏由实现者即兴决定。

## 决策

用户批准（2026-08-27）：新增领域无关的 `kernel-context` 模块，统一拥有 Handle Arena、
内存预算/池、Worker 集、Completion Queue、索引/工作区 registry 与可选诊断 recorder；
状态机 `Creating -> Running -> Quiescing -> Closed`（活动态可入 `Faulted`），
关闭顺序固定为：拒新 → 失效新解析 → 请求取消 → 排空/Abandon → 回收批次 → 销毁资源 → 退休 Epoch。
ContextId 单调不复用，Generation 溢出槽位永久退休。
完整契约见 [`kernel-context-lifecycle.md`](../../docs/specs/kernel-context-lifecycle.md)。

## 后果

- 所有跨调用资源必须登记唯一 owner，模块获得资源前先接入 Context registry，增加接入成本。
- World/Session 如何映射到 Context 属跨仓 handoff（Review OPEN-005），仍待上游定义；
  ContextId 的跨 ABI 公开表示待架构源冻结。
- 关闭竞态的 Conformance Fixture 成为 Foundation 硬验收面。
