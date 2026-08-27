//! ContextResource wrapping BoundedRecorder. Destroy closes late records.

use std::sync::atomic::{AtomicBool, Ordering};

use lumio_kernel::context::{CancelReason, ContextResource, QuiesceReport, QuiesceState};
use lumio_kernel::error::KernelResult;
use lumio_platform::Deadline;

use crate::record::KernelRecordRef;
use crate::recorder::{BoundedRecorder, RecordDisposition};

const ORDER: Ordering = Ordering::SeqCst;

pub struct DiagnosticsResource {
    recorder: BoundedRecorder,
    closed: AtomicBool,
}

impl DiagnosticsResource {
    pub fn new(recorder: BoundedRecorder) -> Self {
        Self {
            recorder,
            closed: AtomicBool::new(false),
        }
    }

    /// Non-blocking. After `destroy`, returns `DroppedFull` without enqueueing.
    pub fn try_record(&self, r: KernelRecordRef<'_>) -> RecordDisposition {
        if self.closed.load(ORDER) {
            return RecordDisposition::DroppedFull;
        }
        self.recorder.try_record(r)
    }
}

impl ContextResource for DiagnosticsResource {
    fn name(&self) -> &'static str {
        "diagnostics"
    }

    fn cancel_requested(&self, _reason: CancelReason) {}

    fn quiesce(&self, _deadline: Deadline) -> KernelResult<QuiesceReport> {
        Ok(QuiesceReport {
            state: QuiesceState::Quiesced,
        })
    }

    fn destroy(&self) -> KernelResult<()> {
        self.closed.store(true, ORDER);
        Ok(())
    }
}
