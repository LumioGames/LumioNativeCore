//! `lumio-test-support`：dev-only 测试辅助（fixture 装载、fake clock、泄漏计数断言）。
//!
//! 只允许出现在其他 crate 的 dev-dependencies；normal 依赖图中不得出现
//! （xtask check-dep-dag 强制）。

#![forbid(unsafe_code)]

mod clock;
mod fixtures;
mod interleaving;
mod leak;

pub use clock::{FakeClock, FakeClockError};
pub use fixtures::{FaultPlan, FixtureLoader, PanicProbe};
pub use interleaving::Interleaving;
pub use leak::LeakSnapshot;
