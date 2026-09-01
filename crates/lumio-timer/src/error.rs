//! Stable Timer ABI error vocabulary from `lumio.native-timer-abi.v1`.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TimerError {
    StaleHandle,
    ScopeInvalid,
    ScopeGenerationMismatch,
    InvalidDueTick,
    InvalidInterval,
    ScheduleBudgetExceeded,
    SlotClosed,
    SlotUnbound,
    SlotDispatchMismatch,
    SlotQueueFull,
    LateCompletion,
    ManagerShutdown,
}

impl TimerError {
    pub const fn as_str(self) -> &'static str {
        self.as_code()
    }

    pub const fn as_code(self) -> &'static str {
        match self {
            Self::StaleHandle => "stale_handle",
            Self::ScopeInvalid => "scope_invalid",
            Self::ScopeGenerationMismatch => "scope_generation_mismatch",
            Self::InvalidDueTick => "invalid_due_tick",
            Self::InvalidInterval => "invalid_interval",
            Self::ScheduleBudgetExceeded => "schedule_budget_exceeded",
            Self::SlotClosed => "slot_closed",
            Self::SlotUnbound => "slot_unbound",
            Self::SlotDispatchMismatch => "slot_dispatch_mismatch",
            Self::SlotQueueFull => "slot_queue_full",
            Self::LateCompletion => "late_completion",
            Self::ManagerShutdown => "manager_shutdown",
        }
    }
}

pub type TimerResult<T> = Result<T, TimerError>;
