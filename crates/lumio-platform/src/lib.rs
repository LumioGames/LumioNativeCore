//! `lumio-platform`：私有平台原语（单调时钟 port 等）。
//!
//! 决策 ADR 0004：NativeCore 只保留私有、可注入的 monotonic clock port，
//! 不拥有 Wall Clock / Tick。本 crate 永不进入稳定 ABI。
//! Ticks 为进程相对单调读数，不进入权威 Hash。

#![forbid(unsafe_code)]

mod clock;

pub use clock::{Deadline, MonotonicClock, StdMonotonicClock, Ticks};
