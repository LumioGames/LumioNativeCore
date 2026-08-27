//! `lumio-platform`：私有平台原语（单调时钟 port 等）。
//!
//! 决策 ADR 0004：NativeCore 只保留私有、可注入的 monotonic clock port，
//! 不拥有 Wall Clock / Tick。本 crate 永不进入稳定 ABI。
//! 当前为脚手架，公共 API 面为空。

#![forbid(unsafe_code)]
