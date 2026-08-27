//! T-context-06 / R-00168: KernelContext close-race conformance.

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

use lumio_kernel::capability::ConfiguredLimits;
use lumio_kernel::context::{
    CancelReason, ContextConfig, ContextPhase, ContextResource, KernelContext, QuiesceReport,
    QuiesceState,
};
use lumio_kernel::error::{ErrorCategory, KernelError, KernelResult};
use lumio_platform::Deadline;
use lumio_test_support::Interleaving;

struct RecordingResource {
    name: &'static str,
    destroy_count: Arc<AtomicU32>,
}

impl RecordingResource {
    fn new(name: &'static str) -> (Arc<Self>, Arc<AtomicU32>) {
        let destroy_count = Arc::new(AtomicU32::new(0));
        (
            Arc::new(Self {
                name,
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
    }

    fn quiesce(&self, _deadline: Deadline) -> KernelResult<QuiesceReport> {
        Ok(QuiesceReport {
            state: QuiesceState::Quiesced,
        })
    }

    fn destroy(&self) -> KernelResult<()> {
        self.destroy_count.fetch_add(1, Ordering::SeqCst);
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

fn assert_closing_or_destroyed(err: &KernelError, what: &str) {
    let category = err.category();
    assert!(
        category == ErrorCategory::ContextClosing || category == ErrorCategory::ContextDestroyed,
        "{what}: expected ContextClosing or ContextDestroyed, got {category:?}"
    );
}

#[test]
fn late_resource_cannot_revive_context() {
    let ctx = KernelContext::create_for_test(test_config());
    let _ = ctx.key();
    ctx.ensure_accepting_work()
        .expect("running context accepts work");

    let (original, destroy_original) = RecordingResource::new("original");
    ctx.register_resource(original).expect("register original");

    let first = ctx
        .close(CancelReason::ContextClosing, Deadline::NONE)
        .expect("first close");
    assert_eq!(first.phase, ContextPhase::Closed);
    assert_eq!(destroy_original.load(Ordering::SeqCst), 1);

    let (late, destroy_late) = RecordingResource::new("late");
    let late_err = ctx
        .register_resource(late)
        .expect_err("late resource must not register after close");
    assert_closing_or_destroyed(&late_err, "register_resource after close");
    assert_eq!(destroy_late.load(Ordering::SeqCst), 0);

    let work_err = ctx
        .ensure_accepting_work()
        .expect_err("closed context rejects new work");
    assert_closing_or_destroyed(&work_err, "ensure_accepting_work after close");

    let second = ctx
        .close(CancelReason::ContextClosing, Deadline::NONE)
        .expect("second close is idempotent");
    assert_eq!(second, first);
    assert_eq!(second.phase, ContextPhase::Closed);
    assert_eq!(destroy_original.load(Ordering::SeqCst), 1);
    assert_eq!(destroy_late.load(Ordering::SeqCst), 0);

    let still_closed = ctx
        .ensure_accepting_work()
        .expect_err("second close must not revive the context");
    assert_closing_or_destroyed(&still_closed, "ensure_accepting_work after second close");
}

#[test]
fn close_vs_register_has_one_linearization() {
    let ctx = KernelContext::create_for_test(test_config());
    let (original, destroy_original) = RecordingResource::new("original");
    ctx.register_resource(original).expect("register original");

    let (late, destroy_late) = RecordingResource::new("late");
    let late_registered = AtomicBool::new(false);
    let close_phase = Mutex::new(None);
    let interleaving = Interleaving::new(&["close", "register"]);

    thread::scope(|scope| {
        scope.spawn(|| {
            interleaving.arrive_and_wait("close");
            let report = ctx
                .close(CancelReason::ContextClosing, Deadline::NONE)
                .expect("close");
            *close_phase.lock().expect("close phase") = Some(report.phase);
            interleaving.arrive_and_wait("register");
        });
        scope.spawn(|| {
            interleaving.arrive_and_wait("close");
            interleaving.arrive_and_wait("register");
            match ctx.register_resource(Arc::clone(&late) as Arc<dyn ContextResource>) {
                Ok(_) => late_registered.store(true, Ordering::SeqCst),
                Err(err) => assert_closing_or_destroyed(&err, "register after close has begun"),
            }
        });
    });

    assert!(
        !late_registered.load(Ordering::SeqCst),
        "register after close has begun must lose; context must not revive"
    );
    assert_eq!(
        *close_phase.lock().expect("close phase"),
        Some(ContextPhase::Closed)
    );
    assert_eq!(destroy_original.load(Ordering::SeqCst), 1);
    assert_eq!(destroy_late.load(Ordering::SeqCst), 0);

    let second = ctx
        .close(CancelReason::ContextClosing, Deadline::NONE)
        .expect("second close is idempotent");
    assert_eq!(second.phase, ContextPhase::Closed);
    assert_eq!(destroy_original.load(Ordering::SeqCst), 1);

    let after = ctx
        .ensure_accepting_work()
        .expect_err("closed context never returns to Running");
    assert_closing_or_destroyed(&after, "ensure_accepting_work after close/register race");
}
