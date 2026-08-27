---
status: completed
---

# 核对上游 V1.1 是否覆盖 Root 归属与模块地图，缺则补 delta

上游 `LumioGameEngineArchitecture` 另一会话正在做 V1.0→V1.1 升级（2026-08-27 观察到 36 文件在途）。
其提交落地后核对是否包含本仓需要的两处公共语义；缺失部分起草 delta，经用户审阅后提交。

## 涉及范围

- `~/LumioGames/LumioGameEngineArchitecture/docs/architecture/`（基线正文 §8.1 与 §16）
- `~/LumioGames/LumioGameEngineArchitecture/.spec/decisions/ADR-006-native-managed-abi.md`

## 验收标准

- [x] §8.1 明确：跨仓 Root 符号由 CoreEngine root-abi/composition 唯一拥有导出；NativeCore/VoxelEngine 只提供 provider 契约；最终产物符号表只允许一个跨仓 Root
- [x] §8.1 明确：capability_bits 只属于 API Table 与 Capability 快照，普通导出结构以 struct_size 保护尾部扩展
- [x] §16 NativeCore 首批模块地图含 contract-types、kernel-context、native-core-ffi（abi 双重身份消除）；codec/diagnostics 标注 pending 或后续
- [x] ADR-006 修订记录上述裁决（Decision 追加 Root 符号/capability_bits 段落，Verification 追加单 Root 符号检查）
- [x] 全部四处上游已覆盖，无需起草 delta

## 依赖

- 外部事件：上游仓当前在途改动提交落地（监控中）

2026-08-27 核对记录：四处均已核实于上游提交 `1bde3cf`（v1.1 正文 L324/L483/L8、
ADR-006 L23/L43），未 push；上游工作区仍有后续在途改动（迁移 Schema 方向），
最终字节锁定与镜像重同步归 `baseline-mirror-sync`。
