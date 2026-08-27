//! T-context-03 / R-00100: ContextResource is usable as `dyn ContextResource`.

use lumio_kernel::context::{CancelReason, ContextResource, QuiesceReport, QuiesceState};
use lumio_kernel::error::KernelResult;
use lumio_platform::Deadline;

fn assert_object_safe(_: &dyn ContextResource) {}

struct DummyResource;

impl ContextResource for DummyResource {
    fn name(&self) -> &'static str {
        "dummy"
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
fn resource_port_is_object_safe() {
    let dummy = DummyResource;
    let resource: &dyn ContextResource = &dummy;
    assert_object_safe(resource);

    resource.cancel_requested(CancelReason::OwnerRequested);
    let report = resource.quiesce(Deadline::NONE).expect("dummy quiesce");
    assert_eq!(
        report,
        QuiesceReport {
            state: QuiesceState::Quiesced
        }
    );
    resource.destroy().expect("dummy destroy");
}
