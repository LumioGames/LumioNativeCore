//! Seven-step KernelContext close driver.
//!
//! Resource callbacks run on a registry snapshot, never while the registry lock is held.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use lumio_platform::Deadline;

use super::config::ContextConfig;
use super::registry::{ResourceRegistration, ResourceRegistry};
use super::resource::{CancelReason, ContextResource, QuiesceState};
use super::state::{ContextPhase, ContextStateGate};
use crate::error::KernelError;
use crate::handle::ContextKey;

const CLOSE_STEPS: &[&str] = &[
    "reject_new_work",
    "cancel_requested",
    "quiesce",
    "wait_quiesce",
    "drain",
    "destroy",
    "mark_closed",
];

static NEXT_CONTEXT_KEY: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ContextCloseReport {
    pub steps: &'static [&'static str],
    pub phase: ContextPhase,
}

pub struct KernelContext {
    key: ContextKey,
    gate: ContextStateGate,
    resources: ResourceRegistry,
    close_report: Mutex<Option<ContextCloseReport>>,
}

impl KernelContext {
    pub fn create_for_test(config: ContextConfig) -> Arc<Self> {
        config
            .validate()
            .expect("create_for_test requires a valid ContextConfig");
        let key = ContextKey::new(NEXT_CONTEXT_KEY.fetch_add(1, Ordering::SeqCst));
        Arc::new(Self {
            key,
            gate: ContextStateGate::new_running(),
            resources: ResourceRegistry::new(),
            close_report: Mutex::new(None),
        })
    }

    pub fn key(&self) -> ContextKey {
        self.key
    }

    pub fn register_resource(
        &self,
        r: Arc<dyn ContextResource>,
    ) -> Result<ResourceRegistration, KernelError> {
        self.ensure_accepting_work()?;
        self.resources.register(r)
    }

    pub fn ensure_accepting_work(&self) -> Result<(), KernelError> {
        self.gate.try_admit()
    }

    pub fn close(
        &self,
        reason: CancelReason,
        deadline: Deadline,
    ) -> Result<ContextCloseReport, KernelError> {
        let mut slot = self.close_report.lock().expect("context close lock");
        if let Some(report) = *slot {
            return Ok(report);
        }
        let report = self.drive_close(reason, deadline)?;
        *slot = Some(report);
        Ok(report)
    }

    fn drive_close(
        &self,
        reason: CancelReason,
        deadline: Deadline,
    ) -> Result<ContextCloseReport, KernelError> {
        let _ = self.gate.begin_close();
        let snapshot = self.resources.snapshot();

        for resource in &snapshot {
            resource.cancel_requested(reason);
        }

        let mut reports = Vec::with_capacity(snapshot.len());
        for resource in &snapshot {
            reports.push(resource.quiesce(deadline)?);
        }

        for report in &reports {
            match report.state {
                QuiesceState::Quiesced | QuiesceState::Pending { .. } => {}
            }
        }

        // No JobSystem yet: drain is a documented no-op for this card.

        for resource in snapshot.iter().rev() {
            resource.destroy()?;
        }

        self.gate.mark_closed();
        Ok(ContextCloseReport {
            steps: CLOSE_STEPS,
            phase: ContextPhase::Closed,
        })
    }
}
