//! `lumio-test-support`：dev-only 测试辅助（fixture 装载、fake clock、泄漏计数断言）。
//!
//! 只允许出现在其他 crate 的 dev-dependencies；normal 依赖图中不得出现
//! （xtask check-dep-dag 强制）。当前为脚手架，公共 API 面为空。

#![forbid(unsafe_code)]
