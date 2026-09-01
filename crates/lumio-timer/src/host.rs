//! Host timer service: monotonic wall-clock deadlines and typed command delivery.
//!
//! This is not the Tick/Frame Timer Manager. Reconnect retention stays here.

use std::collections::{BTreeMap, VecDeque};
use std::sync::Arc;
use std::time::Duration;

use lumio_platform::{MonotonicClock, Ticks};

use crate::ids::{RECONNECT_RETENTION_SECS, SliceTrace, SliceTraceEvent};

const HOST_PORT_CAPACITY: usize = 32;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HostTimerError {
    PortFull,
    DeadlineOverflow,
    StaleKey,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct HostTimerKey {
    id: u64,
}

impl HostTimerKey {
    pub const fn id(self) -> u64 {
        self.id
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HostCommandKind {
    ReconnectRetentionExpired { retention_id: u64 },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HostCommand {
    pub kind: HostCommandKind,
    pub due: Ticks,
}

struct HostDeadline {
    due: Ticks,
    kind: HostCommandKind,
}

/// Process-local monotonic deadline port. No CallbackSlot, no Tick/Frame.
pub struct HostTimerService {
    clock: Arc<dyn MonotonicClock>,
    next_id: u64,
    deadlines: BTreeMap<u64, HostDeadline>,
    port: VecDeque<HostCommand>,
}

impl HostTimerService {
    pub fn new(clock: Arc<dyn MonotonicClock>) -> Self {
        Self {
            clock,
            next_id: 1,
            deadlines: BTreeMap::new(),
            port: VecDeque::new(),
        }
    }

    pub fn schedule_reconnect_retention(
        &mut self,
        retention_id: u64,
    ) -> Result<HostTimerKey, HostTimerError> {
        let due = self
            .clock
            .now()
            .checked_add(Duration::from_secs(RECONNECT_RETENTION_SECS))
            .ok_or(HostTimerError::DeadlineOverflow)?;
        let id = self.next_id;
        self.next_id = self.next_id.saturating_add(1);
        self.deadlines.insert(
            id,
            HostDeadline {
                due,
                kind: HostCommandKind::ReconnectRetentionExpired { retention_id },
            },
        );
        Ok(HostTimerKey { id })
    }

    pub fn cancel(&mut self, key: HostTimerKey) -> Result<(), HostTimerError> {
        self.deadlines
            .remove(&key.id)
            .map(|_| ())
            .ok_or(HostTimerError::StaleKey)
    }

    pub fn poll(&mut self) -> Result<Vec<HostCommand>, HostTimerError> {
        let now = self.clock.now();
        let mut due: Vec<(Ticks, u64, HostCommandKind)> = self
            .deadlines
            .iter()
            .filter(|(_, d)| now >= d.due)
            .map(|(id, d)| (d.due, *id, d.kind))
            .collect();
        due.sort_by_key(|(t, id, _)| (*t, *id));
        if self.port.len() + due.len() > HOST_PORT_CAPACITY {
            return Err(HostTimerError::PortFull);
        }
        for (due_ticks, id, kind) in due {
            self.deadlines.remove(&id);
            self.port.push_back(HostCommand {
                kind,
                due: due_ticks,
            });
        }
        let mut out = Vec::with_capacity(self.port.len());
        while let Some(cmd) = self.port.pop_front() {
            out.push(cmd);
        }
        Ok(out)
    }

    pub fn poll_into(
        &mut self,
        trace: &mut SliceTrace,
    ) -> Result<Vec<HostCommand>, HostTimerError> {
        let commands = self.poll()?;
        for cmd in &commands {
            trace.push(SliceTraceEvent::HostReconnectExpired {
                host_nanos: cmd.due.as_nanos(),
            });
        }
        Ok(commands)
    }
}
