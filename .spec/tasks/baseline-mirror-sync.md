---
status: completed
---

# 上游 V1.1 落地后同步本仓基线镜像与全部基线引用

2026-08-27 完结：上游随后直接发布 `LGE-V1.2-2026-08-27`（提交 `2d7980d`，Client/CoreEngine
契约 + ADR-017~023；NativeCore 四处插入完整保留于 §8.1/§16，ADR-017 与本仓 spec 同向）。
本仓镜像从上游**已提交对象**重拷：v1.2/v1.1/v1.0 三文件字节级一致（v0.3 上游不存在，本地删除），
`.baseline.sha256` 覆盖 v1.2 正文；全部基线引用（根 README、workflow、.spec、九模块 README、
kernel-context README、spec §4、contract-types 文档注释）升级到 V1.2。
后续基线 bump 重复本卡程序即可（从上游已提交对象取字节，禁止取工作区中间态）。

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
