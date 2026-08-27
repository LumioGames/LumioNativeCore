//! T-context-04 / R-00132: ResourceRegistry snapshot_names preserves insert order.

use std::sync::Arc;

use lumio_kernel::context::{
    CancelReason, ContextResource, QuiesceReport, QuiesceState, ResourceRegistry,
};
use lumio_kernel::error::KernelResult;
use lumio_platform::Deadline;

struct DummyA;

impl ContextResource for DummyA {
    fn name(&self) -> &'static str {
        "A"
    }

    fn cancel_requested(&self, _reason: CancelReason) {}

    fn quiesce(&self, _deadline: Deadline) -> KernelResult<QuiesceReport> {
        Ok(QuiesceReport {
            state: QuiesceState::Quiesced,
        })
    }

    fn destroy(&self) -> KernelResult<()> {
        Ok(())
    }
}

struct DummyB;

impl ContextResource for DummyB {
    fn name(&self) -> &'static str {
        "B"
    }

    fn cancel_requested(&self, _reason: CancelReason) {}

    fn quiesce(&self, _deadline: Deadline) -> KernelResult<QuiesceReport> {
        Ok(QuiesceReport {
            state: QuiesceState::Quiesced,
        })
    }

    fn destroy(&self) -> KernelResult<()> {
        Ok(())
    }
}

#[test]
fn registration_order_is_stable() {
    let registry = ResourceRegistry::new();
    registry.register(Arc::new(DummyA)).expect("register A");
    registry.register(Arc::new(DummyB)).expect("register B");
    assert_eq!(registry.snapshot_names(), ["A", "B"]);
    assert_ne!(registry.snapshot_names(), ["B", "A"]);
}
