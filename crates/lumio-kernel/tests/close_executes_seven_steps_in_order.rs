//! T-context-05 / R-00165: KernelContext::close drives seven steps in order.

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

use lumio_kernel::capability::ConfiguredLimits;
use lumio_kernel::context::{
    CancelReason, ContextCloseReport, ContextConfig, ContextPhase, ContextResource, KernelContext,
    QuiesceReport, QuiesceState,
};
use lumio_kernel::error::{ErrorCategory, KernelResult};
use lumio_platform::Deadline;

struct RecordingResource {
    name: &'static str,
    cancel_label: &'static str,
    quiesce_label: &'static str,
    destroy_label: &'static str,
    log: Arc<Mutex<Vec<&'static str>>>,
    destroy_count: Arc<AtomicU32>,
}

impl RecordingResource {
    fn new(
        name: &'static str,
        cancel_label: &'static str,
        quiesce_label: &'static str,
        destroy_label: &'static str,
        log: Arc<Mutex<Vec<&'static str>>>,
    ) -> (Arc<Self>, Arc<AtomicU32>) {
        let destroy_count = Arc::new(AtomicU32::new(0));
        (
            Arc::new(Self {
                name,
                cancel_label,
                quiesce_label,
                destroy_label,
                log,
                destroy_count: Arc::clone(&destroy_count),
            }),
            destroy_count,
        )
    }
}

impl ContextResource for RecordingResource {
    fn name(&self) -> &'static str {
        self.name
    }

    fn cancel_requested(&self, reason: CancelReason) {
        assert_eq!(reason, CancelReason::ContextClosing);
        self.log.lock().expect("log").push(self.cancel_label);
    }

    fn quiesce(&self, _deadline: Deadline) -> KernelResult<QuiesceReport> {
        self.log.lock().expect("log").push(self.quiesce_label);
        Ok(QuiesceReport {
            state: QuiesceState::Quiesced,
        })
    }

    fn destroy(&self) -> KernelResult<()> {
        self.destroy_count.fetch_add(1, Ordering::SeqCst);
        self.log.lock().expect("log").push(self.destroy_label);
        Ok(())
    }
}

fn test_config() -> ContextConfig {
    ContextConfig {
        limits: ConfiguredLimits {
            max_handles: 4,
            max_native_bytes: 64,
            max_jobs_queued: 1,
            max_jobs_running: 1,
            max_completion_items: 1,
        },
        quiesce_deadline: Deadline::NONE,
    }
}

#[test]
fn close_executes_seven_steps_in_order() {
    let ctx = KernelContext::create_for_test(test_config());
    let _ = ctx.key();
    ctx.ensure_accepting_work()
        .expect("running context accepts work");

    let log = Arc::new(Mutex::new(Vec::new()));
    let (res_a, destroy_a) =
        RecordingResource::new("A", "cancel:A", "quiesce:A", "destroy:A", Arc::clone(&log));
    let (res_b, destroy_b) =
        RecordingResource::new("B", "cancel:B", "quiesce:B", "destroy:B", Arc::clone(&log));

    ctx.register_resource(res_a).expect("register A");
    ctx.register_resource(res_b).expect("register B");

    let report = ctx
        .close(CancelReason::ContextClosing, Deadline::NONE)
        .expect("first close");

    assert_eq!(
        report,
        ContextCloseReport {
            steps: &[
                "reject_new_work",
                "cancel_requested",
                "quiesce",
                "wait_quiesce",
                "drain",
                "destroy",
                "mark_closed",
            ],
            phase: ContextPhase::Closed,
        }
    );

    let calls = log.lock().expect("log").clone();
    assert_eq!(
        calls,
        [
            "cancel:A",
            "cancel:B",
            "quiesce:A",
            "quiesce:B",
            "destroy:B",
            "destroy:A",
        ]
    );
    assert_eq!(destroy_a.load(Ordering::SeqCst), 1);
    assert_eq!(destroy_b.load(Ordering::SeqCst), 1);

    let after = ctx
        .ensure_accepting_work()
        .expect_err("closed context rejects new work");
    assert!(
        after.category() == ErrorCategory::ContextClosing
            || after.category() == ErrorCategory::ContextDestroyed,
        "expected ContextClosing or ContextDestroyed, got {:?}",
        after.category()
    );

    let second = ctx
        .close(CancelReason::ContextClosing, Deadline::NONE)
        .expect("second close is idempotent");
    assert_eq!(second, report);
    assert_eq!(destroy_a.load(Ordering::SeqCst), 1);
    assert_eq!(destroy_b.load(Ordering::SeqCst), 1);
    assert_eq!(*log.lock().expect("log"), calls);
}
