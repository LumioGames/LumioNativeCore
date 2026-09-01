//! Server/Client adapters over the single Timer Manager core.

use crate::error::TimerResult;
use crate::ids::{
    BOT_CHAT_CADENCE_DISPATCH, BOT_CHAT_CADENCE_TICKS, CallbackSlot, Delivery,
    SERVER_WORLD_HEARTBEAT_DISPATCH, SERVER_WORLD_HEARTBEAT_TICKS, ScopeKind, SliceTrace,
    TimerHandle, TimerScope,
};
use crate::manager::TimerManager;

pub struct ClientTimerManager {
    manager: TimerManager,
    bot_slot: Option<CallbackSlot>,
}

impl ClientTimerManager {
    pub fn new(context: u64) -> Self {
        let mut manager = TimerManager::new(context);
        manager.register_dispatch(
            BOT_CHAT_CADENCE_DISPATCH,
            crate::ids::DispatchTarget::Registered,
        );
        Self {
            manager,
            bot_slot: None,
        }
    }

    pub fn trace(&self) -> &SliceTrace {
        self.manager.trace()
    }

    pub fn register_scope(&mut self, kind: ScopeKind, scope_id: u64) -> TimerResult<TimerScope> {
        self.manager.register_scope(scope_id, kind)
    }

    pub fn schedule_bot_chat_cadence(&mut self, scope: TimerScope) -> TimerResult<TimerHandle> {
        let slot = match self.bot_slot {
            Some(existing) => existing,
            None => {
                let slot = self.manager.create_slot()?;
                self.manager.bind_slot(slot, BOT_CHAT_CADENCE_DISPATCH)?;
                self.bot_slot = Some(slot);
                slot
            }
        };
        let first_due = self
            .manager
            .committed_tick()
            .saturating_add(BOT_CHAT_CADENCE_TICKS);
        self.manager
            .schedule_repeating(scope, first_due, BOT_CHAT_CADENCE_TICKS, slot)
    }

    pub fn pump(&mut self, to_tick: u64) -> TimerResult<Vec<Delivery>> {
        let _report = self.manager.advance(to_tick)?;
        Ok(self.manager.drain().delivered().to_vec())
    }
}

pub struct ServerTimerManager {
    manager: TimerManager,
    heartbeat_slot: Option<CallbackSlot>,
}

impl ServerTimerManager {
    pub fn new(context: u64) -> Self {
        let mut manager = TimerManager::new(context);
        manager.register_dispatch(
            SERVER_WORLD_HEARTBEAT_DISPATCH,
            crate::ids::DispatchTarget::Registered,
        );
        Self {
            manager,
            heartbeat_slot: None,
        }
    }

    pub fn trace(&self) -> &SliceTrace {
        self.manager.trace()
    }

    pub fn register_scope(&mut self, kind: ScopeKind, scope_id: u64) -> TimerResult<TimerScope> {
        self.manager.register_scope(scope_id, kind)
    }

    pub fn schedule_world_heartbeat(&mut self, scope: TimerScope) -> TimerResult<TimerHandle> {
        let slot = match self.heartbeat_slot {
            Some(existing) => existing,
            None => {
                let slot = self.manager.create_slot()?;
                self.manager
                    .bind_slot(slot, SERVER_WORLD_HEARTBEAT_DISPATCH)?;
                self.heartbeat_slot = Some(slot);
                slot
            }
        };
        let first_due = self
            .manager
            .committed_tick()
            .saturating_add(SERVER_WORLD_HEARTBEAT_TICKS);
        self.manager
            .schedule_repeating(scope, first_due, SERVER_WORLD_HEARTBEAT_TICKS, slot)
    }

    pub fn pump(&mut self, to_tick: u64) -> TimerResult<Vec<Delivery>> {
        let _report = self.manager.advance(to_tick)?;
        Ok(self.manager.drain().delivered().to_vec())
    }
}
