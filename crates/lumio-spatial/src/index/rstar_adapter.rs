//! R-tree adapter seam for `SpatialIndexBackend`.
//!
//! `rstar` is not approved (`EXTERNAL_ALLOWLIST` is empty). This is the
//! blocked-supplier fallback, not a published rstar binding. Internally uses
//! `GridReferenceIndex` so behavior matches the reference oracle. When rstar
//! is approved, only this file should change. Public API does not mention
//! rstar crate types.

use lumio_kernel::error::KernelResult;

use crate::types::{Aabb3, SpatialObjectId};

use super::{GridReferenceIndex, SpatialIndexBackend};

/// Spatial index adapter. Identity is `SpatialObjectId`; insert order is ignored.
#[derive(Clone, Debug)]
pub struct RStarIndexAdapter {
    inner: GridReferenceIndex,
}

impl RStarIndexAdapter {
    pub fn new() -> Self {
        Self {
            inner: GridReferenceIndex::new(),
        }
    }
}

impl SpatialIndexBackend for RStarIndexAdapter {
    fn upsert(&mut self, id: SpatialObjectId, aabb: Aabb3) -> KernelResult<()> {
        self.inner.upsert(id, aabb)
    }

    fn remove(&mut self, id: SpatialObjectId) -> KernelResult<()> {
        self.inner.remove(id)
    }

    fn query_aabb(&self, aabb: Aabb3, out: &mut [SpatialObjectId]) -> KernelResult<usize> {
        self.inner.query_aabb(aabb, out)
    }
}
