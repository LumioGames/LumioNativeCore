//! `lumio-diagnostics`：bounded records 与 FailureFragment 生产（不拥有 Bundle/Sink）。
//!
//! BaselineStatus = pending（ADR 0005）：仅限私有原型，不进公共 Header/export list。
//! 核心模块经 lumio-kernel 的 record port 接入，本 crate 只做 port 实现；
//! 任何核心 crate 不得反向依赖本 crate（xtask check-dep-dag 强制）。
//! 当前为脚手架，公共 API 面为空。

#![forbid(unsafe_code)]
