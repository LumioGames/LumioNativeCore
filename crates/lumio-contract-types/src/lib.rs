//! `lumio-contract-types`：零依赖类型叶子。
//!
//! 只承载架构源生成的固定宽度 POD、Buffer view、版本标量、opaque handle 表示与
//! 错误码/能力位常量；不含任何行为逻辑。边界与依赖图见
//! `docs/specs/native-core-module-map.md`。当前为脚手架，公共 API 面为空，
//! 等待架构源当前基线生成物接入（contracts.lock 任务）。

#![forbid(unsafe_code)]
