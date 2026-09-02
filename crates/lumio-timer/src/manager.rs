//! Native Tick/Frame Timer Manager core plus in-process adapter surface.

use std::collections::{BTreeMap, BTreeSet};

use crate::error::{TimerError, TimerResult};
use crate::ids::{
    AdvanceReport, CallbackSlot, Delivery, DispatchId, DispatchTarget, DrainReport, FiringRecord,
    FiringRejection, ScopeKind, SliceTrace, SliceTraceEvent, SlotLifecycle, TimerDiagnostic,
    TimerHandle, TimerKind, TimerLimits, TimerMode, TimerScope, bump_generation, due_in_window,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SlotState {
    Unbound,
    Armed,
    Closed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum LiveState {
    Active,
    Queued,
}

struct ScopeRecord {
    kind: ScopeKind,
    generation: u32,
    active: u32,
    /// Tombstone after destroy: id stays mapped so the next register cannot wrap to generation 1.
    alive: bool,
}

struct QueueItem {
    record: FiringRecord,
    valid: bool,
    live_timers_at_fire: u32,
}

struct SlotRecord {
    generation: u32,
    state: SlotState,
    dispatch_id: Option<DispatchId>,
    queue: Vec<QueueItem>,
}

struct TimerRecord {
    scope_id: u64,
    kind: TimerKind,
    due_tick: u64,
    interval_ticks: Option<u64>,
    schedule_sequence: u64,
    slot: CallbackSlot,
    dispatch_id: DispatchId,
    state: LiveState,
}

struct TimerSlot {
    generation: u32,
    record: Option<TimerRecord>,
}

pub struct TimerManager {
    context: u64,
    mode: TimerMode,
    limits: TimerLimits,
    running: bool,
    committed_tick: u64,
    schedules_this_tick: u32,
    next_sequence: u64,
    scopes: BTreeMap<u64, ScopeRecord>,
    timers: Vec<TimerSlot>,
    timer_free: Vec<u32>,
    slots: Vec<SlotRecord>,
    dispatch_table: BTreeSet<DispatchId>,
    trace: SliceTrace,
    diagnostics: Vec<TimerDiagnostic>,
}

impl TimerManager {
    pub fn new(context: u64) -> Self {
        Self::with_mode(context, TimerMode::TickFrame)
    }

    pub fn with_mode(context: u64, mode: TimerMode) -> Self {
        Self::with_mode_and_limits(context, mode, TimerLimits::CONTRACT)
    }

    pub fn with_limits(context: u64, limits: TimerLimits) -> Self {
        Self::with_mode_and_limits(context, TimerMode::TickFrame, limits)
    }

    pub fn with_mode_and_limits(context: u64, mode: TimerMode, limits: TimerLimits) -> Self {
        Self {
            context,
            mode,
            limits,
            running: true,
            committed_tick: 0,
            schedules_this_tick: 0,
            next_sequence: 1,
            scopes: BTreeMap::new(),
            timers: Vec::new(),
            timer_free: Vec::new(),
            slots: Vec::new(),
            dispatch_table: BTreeSet::new(),
            trace: SliceTrace::new(),
            diagnostics: Vec::new(),
        }
    }

    pub fn diagnostics(&self) -> &[TimerDiagnostic] {
        &self.diagnostics
    }

    fn diagnose(&mut self, code: TimerError, record: Option<FiringRecord>) {
        self.diagnostics.push(TimerDiagnostic {
            code,
            due_tick: record.map(|r| r.due_tick),
            schedule_sequence: record.map(|r| r.schedule_sequence),
        });
    }

    pub fn create_slot(&mut self) -> TimerResult<CallbackSlot> {
        self.ensure_running()?;
        Ok(self.allocate_slot())
    }

    pub fn bind_slot(&mut self, slot: CallbackSlot, dispatch_id: DispatchId) -> TimerResult<()> {
        self.register_dispatch(dispatch_id, DispatchTarget::Registered);
        self.arm_slot(slot, dispatch_id)
    }

    pub fn remove_dispatch_binding(&mut self, id: DispatchId) {
        self.remove_dispatch(id);
    }

    pub fn register_scope_from_u8(&mut self, kind: u8, scope_id: u64) -> TimerResult<TimerScope> {
        let kind = ScopeKind::from_abi(kind)?;
        self.register_scope(scope_id, kind)
    }

    pub fn committed_tick(&self) -> u64 {
        self.committed_tick
    }

    pub fn mode(&self) -> TimerMode {
        self.mode
    }

    pub fn is_running(&self) -> bool {
        self.running
    }

    pub fn is_scope_alive(&self, scope_id: u64) -> bool {
        self.scopes.get(&scope_id).is_some_and(|s| s.alive)
    }

    pub fn is_dispatch_registered(&self, id: DispatchId) -> bool {
        self.dispatch_table.contains(&id)
    }

    pub fn slot_lifecycle(&self, slot: CallbackSlot) -> TimerResult<SlotLifecycle> {
        let rec = self.slot(slot)?;
        Ok(match rec.state {
            SlotState::Unbound => SlotLifecycle::Unbound,
            SlotState::Armed => SlotLifecycle::Armed,
            SlotState::Closed => SlotLifecycle::Closed,
        })
    }

    pub fn live_timer_count(&self) -> u32 {
        self.timers.iter().filter(|s| s.record.is_some()).count() as u32
    }

    pub fn trace(&self) -> &SliceTrace {
        &self.trace
    }

    pub fn shutdown(&mut self) {
        self.running = false;
        for slot in &mut self.slots {
            for item in &mut slot.queue {
                item.valid = false;
            }
        }
    }

    fn ensure_running(&self) -> TimerResult<()> {
        if self.running {
            Ok(())
        } else {
            Err(TimerError::ManagerShutdown)
        }
    }

    pub fn register_dispatch(&mut self, id: DispatchId, _target: DispatchTarget) {
        self.dispatch_table.insert(id);
    }

    pub fn remove_dispatch(&mut self, id: DispatchId) {
        self.dispatch_table.remove(&id);
    }

    pub fn register_scope(&mut self, scope_id: u64, kind: ScopeKind) -> TimerResult<TimerScope> {
        self.ensure_running()?;
        if let Some(existing) = self.scopes.get_mut(&scope_id) {
            if existing.alive {
                return Ok(TimerScope::new(
                    scope_id,
                    existing.kind,
                    existing.generation,
                ));
            }
            existing.kind = kind;
            existing.generation = bump_generation(existing.generation);
            existing.active = 0;
            existing.alive = true;
            return Ok(TimerScope::new(scope_id, kind, existing.generation));
        }
        self.scopes.insert(
            scope_id,
            ScopeRecord {
                kind,
                generation: 1,
                active: 0,
                alive: true,
            },
        );
        Ok(TimerScope::new(scope_id, kind, 1))
    }

    pub fn teardown_scope(&mut self, scope_id: u64) -> TimerResult<TimerScope> {
        self.ensure_running()?;
        let next = {
            let scope = self.live_scope_mut(scope_id)?;
            scope.generation = bump_generation(scope.generation);
            scope.active = 0;
            TimerScope::new(scope_id, scope.kind, scope.generation)
        };
        self.retire_scope_timers(scope_id);
        Ok(next)
    }

    pub fn destroy_scope(&mut self, scope_id: u64) -> TimerResult<()> {
        self.ensure_running()?;
        {
            let scope = self.live_scope_mut(scope_id)?;
            scope.alive = false;
            scope.active = 0;
        }
        self.retire_scope_timers(scope_id);
        Ok(())
    }

    fn live_scope_mut(&mut self, scope_id: u64) -> TimerResult<&mut ScopeRecord> {
        let scope = self
            .scopes
            .get_mut(&scope_id)
            .ok_or(TimerError::ScopeInvalid)?;
        if !scope.alive {
            return Err(TimerError::ScopeInvalid);
        }
        Ok(scope)
    }

    fn retire_scope_timers(&mut self, scope_id: u64) {
        let mut dead = Vec::new();
        for (index, slot) in self.timers.iter().enumerate() {
            if let Some(record) = slot.record.as_ref()
                && record.scope_id == scope_id
            {
                dead.push(TimerHandle::new(
                    index as u32,
                    slot.generation,
                    self.context,
                ));
            }
        }
        for handle in &dead {
            self.retire(*handle);
        }
        for slot in &mut self.slots {
            for item in &mut slot.queue {
                if dead.contains(&item.record.handle) {
                    item.valid = false;
                }
            }
        }
    }

    pub fn allocate_slot(&mut self) -> CallbackSlot {
        let index = u32::try_from(self.slots.len()).expect("slot index fits u32");
        self.slots.push(SlotRecord {
            generation: 1,
            state: SlotState::Unbound,
            dispatch_id: None,
            queue: Vec::new(),
        });
        CallbackSlot::new(index, 1)
    }

    pub fn arm_slot(&mut self, slot: CallbackSlot, dispatch_id: DispatchId) -> TimerResult<()> {
        self.ensure_running()?;
        let rec = self.slot_mut(slot)?;
        match rec.state {
            SlotState::Unbound => {
                rec.state = SlotState::Armed;
                rec.dispatch_id = Some(dispatch_id);
                Ok(())
            }
            SlotState::Armed if rec.dispatch_id == Some(dispatch_id) => Ok(()),
            SlotState::Closed | SlotState::Armed => Err(TimerError::SlotClosed),
        }
    }

    pub fn close_slot(&mut self, slot: CallbackSlot) -> TimerResult<()> {
        self.ensure_running()?;
        let rec = self.slot_mut(slot)?;
        match rec.state {
            SlotState::Unbound => Err(TimerError::SlotUnbound),
            SlotState::Closed => Err(TimerError::SlotClosed),
            SlotState::Armed => {
                rec.state = SlotState::Closed;
                for item in &mut rec.queue {
                    item.valid = false;
                }
                Ok(())
            }
        }
    }

    fn slot(&self, slot: CallbackSlot) -> TimerResult<&SlotRecord> {
        let rec = self
            .slots
            .get(slot.index() as usize)
            .ok_or(TimerError::SlotClosed)?;
        if rec.generation != slot.generation() {
            Err(TimerError::SlotClosed)
        } else {
            Ok(rec)
        }
    }

    fn slot_mut(&mut self, slot: CallbackSlot) -> TimerResult<&mut SlotRecord> {
        let rec = self
            .slots
            .get_mut(slot.index() as usize)
            .ok_or(TimerError::SlotClosed)?;
        if rec.generation != slot.generation() {
            Err(TimerError::SlotClosed)
        } else {
            Ok(rec)
        }
    }

    fn resolve_scope(&self, scope: TimerScope) -> TimerResult<&ScopeRecord> {
        let Some(rec) = self.scopes.get(&scope.scope_id()) else {
            return Err(TimerError::ScopeInvalid);
        };
        if !rec.alive {
            return Err(TimerError::ScopeInvalid);
        }
        if rec.generation != scope.generation() {
            return Err(TimerError::ScopeGenerationMismatch);
        }
        Ok(rec)
    }

    fn slot_for_schedule(&self, slot: CallbackSlot) -> TimerResult<DispatchId> {
        let rec = self.slot(slot)?;
        match rec.state {
            SlotState::Unbound => Err(TimerError::SlotUnbound),
            SlotState::Closed => Err(TimerError::SlotClosed),
            SlotState::Armed => rec.dispatch_id.ok_or(TimerError::SlotUnbound),
        }
    }

    pub fn schedule_one_shot(
        &mut self,
        scope: TimerScope,
        due_tick: u64,
        slot: CallbackSlot,
    ) -> TimerResult<TimerHandle> {
        self.schedule(scope, due_tick, None, slot)
    }

    pub fn schedule_repeating(
        &mut self,
        scope: TimerScope,
        first_due_tick: u64,
        interval_ticks: u64,
        slot: CallbackSlot,
    ) -> TimerResult<TimerHandle> {
        self.ensure_running()?;
        let min_interval = match self.mode {
            TimerMode::TickFrame => self.limits.min_interval_ticks,
            TimerMode::WallClock => self.limits.min_interval_ms,
        };
        if interval_ticks < min_interval {
            return Err(TimerError::InvalidInterval);
        }
        self.schedule(scope, first_due_tick, Some(interval_ticks), slot)
    }

    fn schedule(
        &mut self,
        scope: TimerScope,
        due_tick: u64,
        interval_ticks: Option<u64>,
        slot: CallbackSlot,
    ) -> TimerResult<TimerHandle> {
        self.ensure_running()?;
        self.resolve_scope(scope)?;
        let dispatch_id = self.slot_for_schedule(slot)?;
        if due_tick <= self.committed_tick {
            return Err(TimerError::InvalidDueTick);
        }
        let max_schedules = match self.mode {
            TimerMode::TickFrame => self.limits.max_schedules_per_tick,
            TimerMode::WallClock => self.limits.max_schedules_per_pump,
        };
        if self.schedules_this_tick >= max_schedules {
            return Err(TimerError::ScheduleBudgetExceeded);
        }
        let active = self
            .scopes
            .get(&scope.scope_id())
            .map(|s| s.active)
            .unwrap_or(0);
        if active >= self.limits.max_active_timers_per_scope {
            return Err(TimerError::ScheduleBudgetExceeded);
        }
        let handle = self.insert_timer(TimerRecord {
            scope_id: scope.scope_id(),
            kind: if interval_ticks.is_some() {
                TimerKind::Repeating
            } else {
                TimerKind::OneShot
            },
            due_tick,
            interval_ticks,
            schedule_sequence: self.next_sequence,
            slot,
            dispatch_id,
            state: LiveState::Active,
        });
        self.next_sequence = self.next_sequence.saturating_add(1);
        self.schedules_this_tick += 1;
        if let Some(scope_rec) = self.scopes.get_mut(&scope.scope_id()) {
            scope_rec.active += 1;
        }
        Ok(handle)
    }

    fn insert_timer(&mut self, record: TimerRecord) -> TimerHandle {
        if let Some(index) = self.timer_free.pop() {
            let slot = &mut self.timers[index as usize];
            slot.record = Some(record);
            TimerHandle::new(index, slot.generation, self.context)
        } else {
            let index = u32::try_from(self.timers.len()).expect("timer index fits u32");
            self.timers.push(TimerSlot {
                generation: 1,
                record: Some(record),
            });
            TimerHandle::new(index, 1, self.context)
        }
    }

    fn get_timer(&self, handle: TimerHandle) -> TimerResult<&TimerRecord> {
        if handle.context() != self.context {
            return Err(TimerError::StaleHandle);
        }
        let slot = self
            .timers
            .get(handle.index() as usize)
            .ok_or(TimerError::StaleHandle)?;
        if slot.generation != handle.generation() || slot.record.is_none() {
            return Err(TimerError::StaleHandle);
        }
        Ok(slot.record.as_ref().expect("occupied"))
    }

    fn retire(&mut self, handle: TimerHandle) {
        if handle.context() == self.context {
            self.retire_index(handle.index());
        }
    }

    fn retire_index(&mut self, index: u32) {
        let Some(slot) = self.timers.get_mut(index as usize) else {
            return;
        };
        let Some(record) = slot.record.take() else {
            return;
        };
        if let Some(scope) = self.scopes.get_mut(&record.scope_id) {
            scope.active = scope.active.saturating_sub(1);
        }
        slot.generation = bump_generation(slot.generation);
        self.timer_free.push(index);
    }

    pub fn cancel(&mut self, handle: TimerHandle) -> TimerResult<bool> {
        self.ensure_running()?;
        self.get_timer(handle)?;
        self.invalidate_queued(handle);
        self.retire(handle);
        Ok(true)
    }

    fn invalidate_queued(&mut self, handle: TimerHandle) {
        for slot in &mut self.slots {
            for item in &mut slot.queue {
                if item.record.handle == handle {
                    item.valid = false;
                }
            }
        }
    }

    pub fn pump(&mut self, now_ms: u64) -> TimerResult<AdvanceReport> {
        self.advance(now_ms)
    }

    pub fn advance(&mut self, to_tick: u64) -> TimerResult<AdvanceReport> {
        self.ensure_running()?;
        if to_tick < self.committed_tick {
            return Err(TimerError::InvalidDueTick);
        }
        if to_tick == self.committed_tick {
            return Ok(AdvanceReport::default());
        }

        let committed = self.committed_tick;
        let mut collected: Vec<FiringRecord> = Vec::new();
        let mut next_due: BTreeMap<u32, u64> = BTreeMap::new();

        for (index, slot) in self.timers.iter().enumerate() {
            let Some(record) = slot.record.as_ref() else {
                continue;
            };
            if record.state != LiveState::Active {
                continue;
            }
            let handle = TimerHandle::new(index as u32, slot.generation, self.context);
            match record.kind {
                TimerKind::OneShot => {
                    if due_in_window(record.due_tick, committed, to_tick) {
                        collected.push(FiringRecord {
                            handle,
                            due_tick: record.due_tick,
                            schedule_sequence: record.schedule_sequence,
                            slot_dispatch_id: record.dispatch_id,
                        });
                    }
                }
                TimerKind::Repeating => {
                    let interval = record
                        .interval_ticks
                        .unwrap_or(self.limits.min_interval_ticks);
                    let mut due = record.due_tick;
                    let mut last = due;
                    let mut fired = false;
                    while due_in_window(due, committed, to_tick) {
                        collected.push(FiringRecord {
                            handle,
                            due_tick: due,
                            schedule_sequence: record.schedule_sequence,
                            slot_dispatch_id: record.dispatch_id,
                        });
                        last = due;
                        fired = true;
                        match due.checked_add(interval) {
                            Some(next) => due = next,
                            None => break,
                        }
                    }
                    if fired && let Some(interval) = record.interval_ticks {
                        next_due.insert(index as u32, last.saturating_add(interval));
                    }
                }
            }
        }

        collected.sort_by_key(|r| (r.due_tick, r.schedule_sequence, r.handle.index()));

        let mut report = AdvanceReport::default();
        let mut retired = BTreeSet::new();
        for record in collected {
            if retired.contains(&record.handle.index()) || self.get_timer(record.handle).is_err() {
                continue;
            }
            let timer_slot = self.get_timer(record.handle).expect("live").slot;
            match self.try_enqueue(record, timer_slot) {
                Ok(()) => {
                    report.push_firing(record);
                    if let Some(slot) = self.timers.get_mut(record.handle.index() as usize)
                        && let Some(rec) = slot.record.as_mut()
                        && rec.kind == TimerKind::OneShot
                    {
                        rec.state = LiveState::Queued;
                    }
                }
                Err(code) => {
                    report.push_rejection(FiringRejection {
                        handle: record.handle,
                        due_tick: record.due_tick,
                        schedule_sequence: record.schedule_sequence,
                        code,
                    });
                    self.diagnose(code, Some(record));
                    self.retire(record.handle);
                    retired.insert(record.handle.index());
                }
            }
        }

        for (index, due) in next_due {
            if retired.contains(&index) {
                continue;
            }
            if let Some(slot) = self.timers.get_mut(index as usize)
                && let Some(record) = slot.record.as_mut()
            {
                record.due_tick = due;
            }
        }

        self.committed_tick = to_tick;
        self.schedules_this_tick = 0;
        Ok(report)
    }

    fn try_enqueue(&mut self, record: FiringRecord, slot: CallbackSlot) -> TimerResult<()> {
        let depth = self.limits.delivery_queue_depth_per_slot;
        let live = self.live_timer_count();
        let rec = match self.slot_mut(slot) {
            Ok(r) => r,
            Err(_) => return Err(TimerError::SlotClosed),
        };
        match rec.state {
            SlotState::Closed => return Err(TimerError::SlotClosed),
            SlotState::Unbound => return Err(TimerError::SlotUnbound),
            SlotState::Armed => {}
        }
        if rec.queue.iter().filter(|i| i.valid).count() >= depth {
            return Err(TimerError::SlotQueueFull);
        }
        rec.queue.push(QueueItem {
            record,
            valid: true,
            live_timers_at_fire: live,
        });
        Ok(())
    }

    pub fn pending_record_count(&self) -> u32 {
        self.slots
            .iter()
            .map(|slot| {
                if slot.state == SlotState::Closed {
                    0
                } else {
                    slot.queue.iter().filter(|item| item.valid).count()
                }
            })
            .sum::<usize>() as u32
    }

    pub fn drain_records(&mut self) -> TimerResult<Vec<FiringRecord>> {
        self.ensure_running()?;
        Ok(self.drain_internal(false).records().to_vec())
    }

    pub fn drain(&mut self) -> DrainReport {
        self.drain_internal(true)
    }

    fn drain_internal(&mut self, emit_trace: bool) -> DrainReport {
        let mut report = DrainReport::default();
        let mut pending: Vec<(DispatchId, QueueItem)> = Vec::new();
        let slot_count = self.slots.len();
        for index in 0..slot_count {
            let queue = std::mem::take(&mut self.slots[index].queue);
            let dispatch_id = self.slots[index].dispatch_id;
            let closed = self.slots[index].state == SlotState::Closed;
            for item in queue {
                if !item.valid || closed || !self.running {
                    report.push_rejection(FiringRejection {
                        handle: item.record.handle,
                        due_tick: item.record.due_tick,
                        schedule_sequence: item.record.schedule_sequence,
                        code: TimerError::LateCompletion,
                    });
                    if self.get_timer(item.record.handle).is_ok() {
                        self.retire(item.record.handle);
                    }
                    continue;
                }
                let Some(id) = dispatch_id else {
                    report.push_rejection(FiringRejection {
                        handle: item.record.handle,
                        due_tick: item.record.due_tick,
                        schedule_sequence: item.record.schedule_sequence,
                        code: TimerError::SlotDispatchMismatch,
                    });
                    self.retire(item.record.handle);
                    continue;
                };
                if !self.dispatch_table.contains(&id) {
                    report.push_rejection(FiringRejection {
                        handle: item.record.handle,
                        due_tick: item.record.due_tick,
                        schedule_sequence: item.record.schedule_sequence,
                        code: TimerError::SlotDispatchMismatch,
                    });
                    self.retire(item.record.handle);
                    continue;
                }
                pending.push((id, item));
            }
        }
        pending.sort_by_key(|(_, item)| {
            (
                item.record.due_tick,
                item.record.schedule_sequence,
                item.record.handle.index(),
            )
        });
        for (id, item) in pending {
            report.push_record(item.record);
            report.push_delivery(Delivery {
                dispatch_id: id,
                due_tick: item.record.due_tick,
                handle: item.record.handle,
            });
            if emit_trace {
                self.trace.push(SliceTraceEvent::Dispatched {
                    dispatch_id: id,
                    due_tick: item.record.due_tick,
                });
                if id == DispatchId::BOT_CHAT_CADENCE {
                    self.trace.push(SliceTraceEvent::BotUtteranceSubmit {
                        due_tick: item.record.due_tick,
                    });
                }
                if id == DispatchId::SERVER_PERIODIC_CHECKPOINT {
                    self.trace.push(SliceTraceEvent::ServerPeriodicCheckpoint {
                        due_tick: item.record.due_tick,
                        live_timers: item.live_timers_at_fire,
                    });
                }
            }
            let one_shot = self
                .get_timer(item.record.handle)
                .map(|t| t.kind == TimerKind::OneShot)
                .unwrap_or(false);
            if one_shot {
                self.retire(item.record.handle);
            }
        }
        report
    }

    #[doc(hidden)]
    pub fn force_timer_generation(&mut self, handle: TimerHandle, generation: u32) -> TimerHandle {
        if let Some(slot) = self.timers.get_mut(handle.index() as usize) {
            slot.generation = generation;
        }
        TimerHandle::new(handle.index(), generation, handle.context())
    }

    #[doc(hidden)]
    pub fn force_scope_generation(&mut self, scope_id: u64, generation: u32) {
        if let Some(scope) = self.scopes.get_mut(&scope_id) {
            scope.generation = generation;
        }
    }
}
