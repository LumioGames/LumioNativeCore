//! `lumio-kernel`：error / capability / handle / memory / kernel-context 模块的编译承载。
//!
//! kernel-context 是生命周期根（ADR 0002）：定义 ContextResource port，
//! job/spatial/codec 实现该 port 并注册进 Context——编译期依赖方向是它们指向本 crate，
//! 本 crate 不得依赖 lumio-job / lumio-diagnostics（xtask check-dep-dag 强制）。
//! 契约见 `docs/specs/kernel-context-lifecycle.md` 与 `docs/specs/ffi-buffer-ownership.md`。

pub mod capability;
pub mod error;
pub mod handle;

pub use error::{ErrorCategory, ErrorDetail, KernelError, KernelResult};
pub use handle::{ContextKey, Generation, Handle, HandleKey, SlotIndex};
