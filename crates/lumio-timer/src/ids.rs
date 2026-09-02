//! Opaque handles, scopes, CallbackSlot identities, and slice-trace records.

use crate::error::{TimerError, TimerResult};

pub const MAX_ACTIVE_TIMERS_PER_SCOPE: u32 = 1024;
pub const MAX_SCHEDULES_PER_TICK: u32 = 4096;
pub const MAX_SCHEDULES_PER_PUMP: u32 = 4096;
pub const DELIVERY_QUEUE_DEPTH_PER_SLOT: usize = 256;
pub const MIN_INTERVAL_TICKS: u64 = 1;
pub const MIN_INTERVAL_MS: u64 = 1;

/// Frozen by R-00352: Bot chat cadence interval in ticks.
pub const BOT_CHAT_CADENCE_TICKS: u64 = 5;
pub const BOT_CHAT_CADENCE_INTERVAL_TICKS: u64 = BOT_CHAT_CADENCE_TICKS;
pub const BOT_CHAT_CADENCE_DISPATCH: DispatchId = DispatchId::from_raw(100);

/// Server occupancy/heartbeat checkpoint period selected by R-00352.
pub const SERVER_WORLD_HEARTBEAT_TICKS: u64 = 10;
pub const SERVER_PERIODIC_INTERVAL_TICKS: u64 = SERVER_WORLD_HEARTBEAT_TICKS;
pub const SERVER_WORLD_HEARTBEAT_DISPATCH: DispatchId = DispatchId::from_raw(101);

/// Reconnect retention window (R-00350) carried by kernel:wallClock, not a second timer.
pub const RECONNECT_RETENTION_SECS: u64 = 300;
pub const RECONNECT_RETENTION_MS: u64 = RECONNECT_RETENTION_SECS * 1000;
pub const RECONNECT_RETENTION_DISPATCH: DispatchId = DispatchId::from_raw(102);

pub type SlotDispatchId = DispatchId;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TimerMode {
    WallClock = 0,
    TickFrame = 1,
}

impl TimerMode {
    pub const fn from_abi(raw: u32) -> Option<Self> {
        match raw {
            0 => Some(Self::WallClock),
            1 => Some(Self::TickFrame),
            _ => None,
        }
    }

    pub const fn to_abi(self) -> u32 {
        self as u32
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TimerLimits {
    pub max_active_timers_per_scope: u32,
    pub max_schedules_per_tick: u32,
    pub max_schedules_per_pump: u32,
    pub delivery_queue_depth_per_slot: usize,
    pub min_interval_ticks: u64,
    pub min_interval_ms: u64,
}

impl TimerLimits {
    pub const CONTRACT: Self = Self {
        max_active_timers_per_scope: MAX_ACTIVE_TIMERS_PER_SCOPE,
        max_schedules_per_tick: MAX_SCHEDULES_PER_TICK,
        max_schedules_per_pump: MAX_SCHEDULES_PER_PUMP,
        delivery_queue_depth_per_slot: DELIVERY_QUEUE_DEPTH_PER_SLOT,
        min_interval_ticks: MIN_INTERVAL_TICKS,
        min_interval_ms: MIN_INTERVAL_MS,
    };
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct TimerHandle {
    index: u32,
    generation: u32,
    context: u64,
}

impl TimerHandle {
    pub const fn from_abi(index: u32, generation: u32, context: u64) -> Self {
        Self {
            index,
            generation,
            context,
        }
    }

    pub(crate) const fn new(index: u32, generation: u32, context: u64) -> Self {
        Self::from_abi(index, generation, context)
    }

    pub const fn index(self) -> u32 {
        self.index
    }

    pub const fn generation(self) -> u32 {
        self.generation
    }

    pub const fn context(self) -> u64 {
        self.context
    }
}

/// Pre-registered dispatch destination. Integer id, never a function pointer.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct DispatchId(u32);

impl DispatchId {
    pub const BOT_CHAT_CADENCE: Self = BOT_CHAT_CADENCE_DISPATCH;
    pub const SERVER_PERIODIC_CHECKPOINT: Self = SERVER_WORLD_HEARTBEAT_DISPATCH;
    pub const RECONNECT_RETENTION: Self = RECONNECT_RETENTION_DISPATCH;

    pub const fn from_raw(raw: u32) -> Self {
        Self(raw)
    }

    pub fn from_static(id: &'static str) -> Self {
        match id {
            "test.slot" => Self(1),
            "test.slot.b" => Self(2),
            "test.other" => Self(3),
            "client.bot_chat_cadence" => Self(100),
            "server.world_authority_heartbeat" => Self(101),
            "server.reconnect_retention" => Self(102),
            _ => Self(0),
        }
    }

    pub const fn raw(self) -> u32 {
        self.0
    }

    pub const fn as_str(self) -> &'static str {
        match self.0 {
            1 => "test.slot",
            2 => "test.slot.b",
            3 => "test.other",
            100 => "client.bot_chat_cadence",
            101 => "server.world_authority_heartbeat",
            102 => "server.reconnect_retention",
            _ => "test.unknown",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DispatchTarget {
    Registered,
    BotChatCadence,
    ServerPeriodicCheckpoint,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct CallbackSlot {
    index: u32,
    generation: u32,
}

impl CallbackSlot {
    pub const fn from_abi(index: u32, generation: u32) -> Self {
        Self { index, generation }
    }

    pub(crate) const fn new(index: u32, generation: u32) -> Self {
        Self::from_abi(index, generation)
    }

    pub const fn index(self) -> u32 {
        self.index
    }

    pub const fn generation(self) -> u32 {
        self.generation
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ScopeKind {
    World,
    Session,
    Adapter,
}

impl ScopeKind {
    pub const fn from_abi(raw: u8) -> TimerResult<Self> {
        match raw {
            0 => Ok(Self::World),
            1 => Ok(Self::Session),
            2 => Ok(Self::Adapter),
            _ => Err(TimerError::ScopeInvalid),
        }
    }

    pub const fn to_abi(self) -> u8 {
        match self {
            Self::World => 0,
            Self::Session => 1,
            Self::Adapter => 2,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TimerScope {
    pub scope_id: u64,
    pub kind: ScopeKind,
    pub generation: u32,
}

impl TimerScope {
    pub const fn new(scope_id: u64, kind: ScopeKind, generation: u32) -> Self {
        Self {
            scope_id,
            kind,
            generation,
        }
    }

    pub const fn scope_id(self) -> u64 {
        self.scope_id
    }

    pub const fn kind(self) -> ScopeKind {
        self.kind
    }

    pub const fn generation(self) -> u32 {
        self.generation
    }

    pub const fn debug_unregistered(scope_id: u64, generation: u32, kind: ScopeKind) -> Self {
        Self::new(scope_id, kind, generation)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TimerKind {
    OneShot,
    Repeating,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FiringRecord {
    pub handle: TimerHandle,
    pub due_tick: u64,
    pub schedule_sequence: u64,
    pub slot_dispatch_id: DispatchId,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DrainOutcome {
    pub record: FiringRecord,
    pub result: Result<(), TimerError>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FiringRejection {
    pub handle: TimerHandle,
    pub due_tick: u64,
    pub schedule_sequence: u64,
    pub code: TimerError,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct AdvanceReport {
    firings: Vec<FiringRecord>,
    rejections: Vec<FiringRejection>,
}

impl std::ops::Deref for AdvanceReport {
    type Target = [FiringRecord];

    fn deref(&self) -> &Self::Target {
        &self.firings
    }
}

impl AdvanceReport {
    pub fn firings(&self) -> &[FiringRecord] {
        &self.firings
    }

    pub fn rejections(&self) -> &[FiringRejection] {
        &self.rejections
    }

    pub(crate) fn push_firing(&mut self, record: FiringRecord) {
        self.firings.push(record);
    }

    pub(crate) fn push_rejection(&mut self, rejection: FiringRejection) {
        self.rejections.push(rejection);
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct DrainReport {
    delivered: Vec<Delivery>,
    rejections: Vec<FiringRejection>,
    records: Vec<FiringRecord>,
}

impl DrainReport {
    pub fn delivered(&self) -> &[Delivery] {
        &self.delivered
    }

    pub fn rejections(&self) -> &[FiringRejection] {
        &self.rejections
    }

    pub fn records(&self) -> &[FiringRecord] {
        &self.records
    }

    pub(crate) fn push_delivery(&mut self, delivery: Delivery) {
        self.delivered.push(delivery);
    }

    pub(crate) fn push_record(&mut self, record: FiringRecord) {
        self.records.push(record);
    }

    pub(crate) fn push_rejection(&mut self, rejection: FiringRejection) {
        self.rejections.push(rejection);
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Delivery {
    pub dispatch_id: DispatchId,
    pub due_tick: u64,
    pub handle: TimerHandle,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TimerDiagnostic {
    pub code: TimerError,
    pub due_tick: Option<u64>,
    pub schedule_sequence: Option<u64>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SlotLifecycle {
    Unbound,
    Armed,
    Closed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TimingLayer {
    ClientTimerManager,
    ServerTimerManager,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SliceTraceEvent {
    BotUtteranceSubmit {
        due_tick: u64,
    },
    ServerPeriodicCheckpoint {
        due_tick: u64,
        live_timers: u32,
    },
    Dispatched {
        dispatch_id: DispatchId,
        due_tick: u64,
    },
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SliceTrace {
    events: Vec<SliceTraceEvent>,
}

impl SliceTrace {
    pub fn new() -> Self {
        Self { events: Vec::new() }
    }

    pub fn push(&mut self, event: SliceTraceEvent) {
        self.events.push(event);
    }

    pub fn events(&self) -> &[SliceTraceEvent] {
        &self.events
    }

    pub fn bot_utterance_ticks(&self) -> Vec<u64> {
        self.events
            .iter()
            .filter_map(|e| match e {
                SliceTraceEvent::BotUtteranceSubmit { due_tick } => Some(*due_tick),
                _ => None,
            })
            .collect()
    }

    pub fn server_checkpoint_ticks(&self) -> Vec<u64> {
        self.events
            .iter()
            .filter_map(|e| match e {
                SliceTraceEvent::ServerPeriodicCheckpoint { due_tick, .. } => Some(*due_tick),
                _ => None,
            })
            .collect()
    }

    pub fn ticks_named(&self, name: &str) -> Vec<u64> {
        match name {
            "bot_chat_cadence" => self.bot_utterance_ticks(),
            "world_authority_heartbeat" => self.server_checkpoint_ticks(),
            _ => Vec::new(),
        }
    }

    pub fn dispatched_ticks(&self, id: DispatchId) -> Vec<u64> {
        self.events
            .iter()
            .filter_map(|e| match e {
                SliceTraceEvent::Dispatched {
                    dispatch_id,
                    due_tick,
                } if *dispatch_id == id => Some(*due_tick),
                _ => None,
            })
            .collect()
    }
}

pub(crate) fn due_in_window(due: u64, committed: u64, to_tick: u64) -> bool {
    due > committed && due <= to_tick
}

pub(crate) fn bump_generation(current: u32) -> u32 {
    current
        .checked_add(1)
        .unwrap_or_else(|| panic!("timer generation overflow is fatal"))
}
