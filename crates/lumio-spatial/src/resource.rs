//! Spatial index as a KernelContext-owned ContextResource.

use std::sync::atomic::{AtomicBool, Ordering};

use lumio_kernel::context::{CancelReason, ContextResource, Deadline, QuiesceReport, QuiesceState};
use lumio_kernel::error::{ErrorCategory, ErrorDetail, KernelError, KernelResult};

use crate::query::{AabbQuery, SpatialContext, SpatialHit};
use crate::types::{Aabb3, SpatialObjectId};

/// Owns a `SpatialContext` and rejects queries after `destroy`.
pub struct SpatialResource {
    inner: SpatialContext,
    destroyed: AtomicBool,
}

impl Default for SpatialResource {
    fn default() -> Self {
        Self::new()
    }
}

impl SpatialResource {
    pub fn new() -> Self {
        Self {
            inner: SpatialContext::new(),
            destroyed: AtomicBool::new(false),
        }
    }

    pub fn upsert(&mut self, id: SpatialObjectId, aabb: Aabb3) -> KernelResult<()> {
        self.ensure_live()?;
        self.inner.upsert(id, aabb)
    }

    pub fn query_aabb_batch(
        &self,
        queries: &[AabbQuery],
        out: &mut [SpatialHit],
    ) -> KernelResult<usize> {
        self.ensure_live()?;
        self.inner.query_aabb_batch(queries, out)
    }

    fn ensure_live(&self) -> KernelResult<()> {
        if self.destroyed.load(Ordering::SeqCst) {
            return Err(KernelError::new(
                ErrorCategory::ContextDestroyed,
                ErrorDetail::None,
            ));
        }
        Ok(())
    }
}

impl ContextResource for SpatialResource {
    fn name(&self) -> &'static str {
        "spatial"
    }

    fn cancel_requested(&self, _reason: CancelReason) {}

    fn quiesce(&self, _deadline: Deadline) -> KernelResult<QuiesceReport> {
        Ok(QuiesceReport {
            state: QuiesceState::Quiesced,
        })
    }

    fn destroy(&self) -> KernelResult<()> {
        self.destroyed.store(true, Ordering::SeqCst);
        Ok(())
    }
}
