---
status: in_progress
---

# 上游 V1.1 落地后同步本仓基线镜像与全部基线引用

2026-08-27 进展：上游镜像同步会话已直接写入本仓（v1.1 镜像、v1.0/v0.3 降级指针、
`.baseline.sha256`、根 README 基线行、workflow 断言），本仓验证内部一致（sha256 OK、
workflow grep 全过）。**遗留**：镜像字节与上游已提交版 `1bde3cf` 不一致（同步自中间态工作区），
且上游仍在继续修改 v1.1 正文——待上游静默并提交最终版后，重拷镜像、重算 hash、终验。
模块 README 与 `.spec` 的基线引用已由 `modules-doc-fixes` 对齐 V1.1。

## 涉及范围

- `docs/architecture/LumioGameEngine_Architecture_v1.0.md`（按上游处置：替换为新版镜像或降级为指针文件）
- `docs/architecture/.baseline.sha256`（重算）
- `README.md`、`AGENTS.md`、`.spec/AGENTS.md`、`.spec/knowledge/standards/repository-architecture.md`（基线 ID 字符串）
- `modules/*/README.md`（「架构基线」行与 BaselineStatus 字段值）
- `.github/workflows/repository-policy.yml`（grep 的基线 ID 与文件名断言）

## 验收标准

- [ ] 镜像文件与上游提交后的基线正文逐字节一致（sha256 相同）
- [ ] `shasum -a 256 -c docs/architecture/.baseline.sha256` 通过
- [ ] 全仓 `rg "LGE-V1\.0-2026-08-27"` 无残留（历史评审文档 docs/ 下的引用除外，逐个确认后豁免）
- [ ] workflow 中的 grep/test 断言与新文件名、新基线 ID 一致，CI 全绿
- [ ] `node .spec/tools/spec-lint.mjs` 通过

## 依赖

- upstream-root-modulemap-delta（上游内容定稿后才能锁镜像）
