//! `lumio-job`：有界 Worker、Typed Job、取消/超时与 Completion Batch。
//!
//! 状态机与竞态裁决见 `docs/specs/job-state-machine.md`（ADR 0004）；
//! Worker 集作为 ContextResource 注册进 kernel-context。

#![forbid(unsafe_code)]

mod cancel;
mod id;
mod state;

pub use cancel::{CancellationSource, CancellationView};
pub use id::{JobId, OperationId, operation_id_overlaps_generated};
pub use state::{JobState, JobStateCell, JobStateMachine};
