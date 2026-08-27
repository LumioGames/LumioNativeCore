# 0005 · codec/diagnostics 维持 pending 只做私有原型，模块引入双状态标注

- 日期:2026-08-27
- 状态:生效

## 背景

架构 Review `ARCH-P1-004/005/009` 与 OPEN-002：Baseline 首批模块地图无 codec/diagnostics，
但模块 README 统一标 Baseline ID，易被误读为已获架构批准；codec 还越界吸收了
schema-aware 语义（重复字段/未知必需字段判定属生成 Serializer）。

## 决策

用户对二者去留持中立，按 Review 推荐执行：`codec`、`diagnostics` 维持 **pending**——
仓内允许 feature-gated 私有原型，不进公共 Header/export list，转正只能由架构源批准驱动。
codec 职责缩窄为纯字节 Kernel（压缩/校验/diff），schema 判定上移生成 Serializer；
diagnostics 只产 bounded records/FailureFragment，不拥有完整 Bundle/Sink。
全部模块 README 引入三行状态字段：`BaselineStatus`（只随架构源更新）、
`RepositoryDeliveryPhase`、`ImplementationPriority`（I0/I1/I2，避免与缺陷级 P0/P1 撞名），
见 [`native-core-module-map.md`](../../docs/specs/native-core-module-map.md) §4。

## 后果

- codec/diagnostics 的公共 ABI 工作全部冻结到上游批准后，接受交付顺延。
- 九个模块 README 与根 README 需要机械改写状态字段（任务卡在途）。
- lint 后续应校验 pending 模块不出现在导出面（脚手架任务内实现）。
