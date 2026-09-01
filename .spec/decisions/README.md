# Decisions(决策记录 · ADR)

用 ADR(Architecture Decision Record)记录决策:为什么这样调度、为什么定这种结构、为什么划这条边界。**本目录是全仓决策记录的唯一落点**——功能内决策与框架级决策都记这里,feature 文档只描述设计现状,不留决策记录。

> 跨仓公共语义的决策只在 `LumioGameEngineArchitecture` 维护；本目录仅记录 NativeCore 内部实现决策，并从 `0001` 开始编号。

## 怎么写一条 ADR

- 一个决策 = 一个文件 `NNNN-<slug>.md`,编号从 `0001` 递增;写完在下方索引加一行。
- **一旦记录不改写**:被推翻就新增一条,把旧的状态标成「被 NNNN 取代」,历史留痕。
- 无 frontmatter。格式照抄:

      # NNNN · <一句话决策>

      - 日期:YYYY-MM-DD
      - 状态:生效 | 被 NNNN 取代

      ## 背景
      面对什么问题。

      ## 决策
      定了什么。

      ## 后果
      接受了什么代价。

## 索引

| 编号 | 决策 | 状态 |
|------|------|------|
| [0001](0001-contract-layering-and-symbol-surface.md) | abi 拆 contract-types 叶子与 native-core-ffi 门面,不导出跨仓 Root 符号 | 生效 |
| [0002](0002-kernel-context-lifecycle-root.md) | 新增 kernel-context 生命周期根 | 生效 |
| [0003](0003-ffi-buffer-classes-and-leases.md) | FFI Buffer 三分类 + 异步租约,按 provenance 定释放方 | 生效 |
| [0004](0004-job-state-machine-and-clock-port.md) | Job 状态机 CAS 线性化 + 私有单调时钟 port | 生效 |
| [0005](0005-codec-diagnostics-pending-and-dual-status.md) | codec/diagnostics 维持 pending,模块引入双状态标注 | 生效 |
| [0006](0006-capability-keys-have-no-raw-constructor.md) | Capability key 只由生成注册表投影,不保留裸构造器 | 生效 |
| [0007](0007-timer-manager-in-process-api.md) | Timer Manager 以独立 crate 提供进程内 Tick/Frame API，不进入 C ABI | 生效 |
