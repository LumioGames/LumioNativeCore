//! Native single timer kernel (ADR-056 §7 / `lumio.native-timer-abi.v1`).
//!
//! Dual mode: `wallClock` (`pump`) and `tickFrame` (`advance`). Shared
//! handle / slot / error vocabulary. C ABI is exported by `lumio-native-ffi`.

#![forbid(unsafe_code)]

mod adapter;
mod error;
mod ids;
mod manager;

pub use adapter::{ClientTimerManager, ServerTimerManager};
pub use error::{TimerError, TimerResult};
pub use ids::{
    AdvanceReport, BOT_CHAT_CADENCE_DISPATCH, BOT_CHAT_CADENCE_INTERVAL_TICKS,
    BOT_CHAT_CADENCE_TICKS, CallbackSlot, DELIVERY_QUEUE_DEPTH_PER_SLOT, Delivery, DispatchId,
    DispatchTarget, DrainOutcome, DrainReport, FiringRecord, FiringRejection,
    MAX_ACTIVE_TIMERS_PER_SCOPE, MAX_SCHEDULES_PER_PUMP, MAX_SCHEDULES_PER_TICK, MIN_INTERVAL_MS,
    MIN_INTERVAL_TICKS, RECONNECT_RETENTION_DISPATCH, RECONNECT_RETENTION_MS,
    RECONNECT_RETENTION_SECS, SERVER_PERIODIC_INTERVAL_TICKS, SERVER_WORLD_HEARTBEAT_DISPATCH,
    SERVER_WORLD_HEARTBEAT_TICKS, ScopeKind, SliceTrace, SliceTraceEvent, SlotDispatchId,
    SlotLifecycle, TimerDiagnostic, TimerHandle, TimerKind, TimerLimits, TimerMode, TimerScope,
    TimingLayer,
};
pub use manager::TimerManager;

pub const CONTRACT_ID: &str = "lumio.native-timer-abi.v1";
pub const CONTRACT_REVISION: &str = "936046a";
