---
status: completed
---

# 按 ADR 0001-0005 修正模块文档的越权声明与矛盾表述

架构 Review §7.1 的本地文档整改：不涉及上游基线字符串（那部分归 `baseline-mirror-sync` 卡）。

## 涉及范围

- `README.md`（根：依赖图加 contract-types/kernel-context/native-core-ffi；「创建侧释放」补 provenance 口径；「调用方创建并销毁 Handle/Buffer」改为 resource owner / handle holder / allocator 三术语；实施优先级 P0/P1 改 I0/I1；碰撞归属并入 spatial 的一句话）
- `modules/abi/README.md`（改写为 contract-types + native-core-ffi 两层口径；删除 Root API Table 拥有权声明，改为 provider 契约；删除「每个导出结构包含 capability_bits」）
- `modules/kernel-context/README.md`（新建，口径来自 `docs/specs/kernel-context-lifecycle.md`）
- `modules/handle/README.md`（Context 校验引用 kernel-context；Generation 溢出退休规则）
- `modules/memory/README.md`（「异步 Job 只能接收明确转移所有权的批次」改为引用 Buffer 三分类）
- `modules/job/README.md`（「提交方转移输入所有权」改为 NativeOwnedBufferHandle 移交或复制；状态机/竞态表引用 `docs/specs/job-state-machine.md`；闭包限定 Rust 内部）
- `modules/capability/README.md`（拆 StaticCapabilities / ConfiguredLimits / RuntimeStatus 三层口径）
- `modules/spatial/README.md`（依赖行去掉对 job 的编译期依赖表述；补碰撞归属一句）
- `modules/codec/README.md`（缩窄为纯字节 Kernel；schema 判定移交生成 Serializer；标 pending）
- `modules/diagnostics/README.md`（只产 fragment/bounded records；脱敏 allowlist 口径；标 pending）
- `.spec/knowledge/standards/repository-architecture.md`（「时间与 Diagnostic Kernel」改为私有单调时钟 port；不拥有 Wall Clock/Tick）
- 九个模块 README 头部加三行状态字段（BaselineStatus / RepositoryDeliveryPhase / ImplementationPriority）

## 验收标准

- [x] 上述每个文件的指定条目全部落实，且未夹带其他改动
- [x] 全文无「NativeCore 拥有/导出 Root API Table」表述；无「每个导出结构包含 capability_bits」
- [x] 所有权术语全仓统一为 resource owner / handle holder / allocator
- [x] `rg -n "P0|P1" README.md modules/` 中不再有实施优先级含义的 P 系用法（缺陷级引用除外）
- [x] `node .spec/tools/spec-lint.mjs` 通过

2026-08-27 执行记录：模块 README 的「架构基线」行与 BaselineStatus 字段已一并对齐 V1.1
（上游镜像同步会话只改了根 README 与 docs/architecture，.spec 与 modules 由本卡补齐，
含 `.spec/AGENTS.md` 的基线字符串）。镜像字节的最终锁定仍归 `baseline-mirror-sync`。

## 依赖

- 无（ADR 0001-0005 已生效；BaselineStatus 字段值先按 spec §4 填，上游落地后由 `baseline-mirror-sync` 复核）
