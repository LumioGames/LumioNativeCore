//! Native Tick/Frame Timer Manager (ADR-055 / `lumio.native-timer-abi.v1`).
//!
//! In-process API. Not a C ABI export. Wall-clock reconnect stays on the Host
//! timer service.

#![forbid(unsafe_code)]

mod adapter;
mod error;
mod host;
mod ids;
mod manager;

pub use adapter::{ClientTimerManager, ServerTimerManager};
pub use error::{TimerError, TimerResult};
pub use host::{HostCommand, HostCommandKind, HostTimerError, HostTimerKey, HostTimerService};
pub use ids::{
    AdvanceReport, BOT_CHAT_CADENCE_DISPATCH, BOT_CHAT_CADENCE_INTERVAL_TICKS,
    BOT_CHAT_CADENCE_TICKS, CallbackSlot, DELIVERY_QUEUE_DEPTH_PER_SLOT, Delivery, DispatchId,
    DispatchTarget, DrainOutcome, DrainReport, FiringRecord, FiringRejection,
    MAX_ACTIVE_TIMERS_PER_SCOPE, MAX_SCHEDULES_PER_TICK, MIN_INTERVAL_TICKS,
    RECONNECT_RETENTION_SECS, SERVER_PERIODIC_INTERVAL_TICKS, SERVER_WORLD_HEARTBEAT_DISPATCH,
    SERVER_WORLD_HEARTBEAT_TICKS, ScopeKind, SliceTrace, SliceTraceEvent, SlotDispatchId,
    TimerDiagnostic, TimerHandle, TimerKind, TimerLimits, TimerScope, TimingLayer,
};
pub use manager::TimerManager;

pub const CONTRACT_ID: &str = "lumio.native-timer-abi.v1";
pub const CONTRACT_REVISION: &str = "2b7e321";
