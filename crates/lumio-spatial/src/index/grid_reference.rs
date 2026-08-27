//! Deterministic brute-force AABB oracle. Query hits are ordered by `SpatialObjectId`.

use std::collections::BTreeMap;

use lumio_kernel::error::{ErrorCategory, ErrorDetail, KernelError, KernelResult};

use crate::types::{Aabb3, SpatialObjectId};

use super::SpatialIndexBackend;

/// In-tree reference index. Identity is `SpatialObjectId`; insert order is ignored.
#[derive(Clone, Debug)]
pub struct GridReferenceIndex {
    objects: BTreeMap<SpatialObjectId, Aabb3>,
}

impl GridReferenceIndex {
    pub fn new() -> Self {
        Self {
            objects: BTreeMap::new(),
        }
    }
}

/// Closed AABB overlap. Degenerate `min == max` boxes are allowed and may hit.
fn aabb_overlaps(a: Aabb3, b: Aabb3) -> bool {
    !(a.max.x < b.min.x
        || b.max.x < a.min.x
        || a.max.y < b.min.y
        || b.max.y < a.min.y
        || a.max.z < b.min.z
        || b.max.z < a.min.z)
}

impl SpatialIndexBackend for GridReferenceIndex {
    fn upsert(&mut self, id: SpatialObjectId, aabb: Aabb3) -> KernelResult<()> {
        self.objects.insert(id, aabb);
        Ok(())
    }

    fn remove(&mut self, id: SpatialObjectId) -> KernelResult<()> {
        if self.objects.remove(&id).is_none() {
            return Err(KernelError::new(
                ErrorCategory::InvalidHandle,
                ErrorDetail::None,
            ));
        }
        Ok(())
    }

    fn query_aabb(&self, aabb: Aabb3, out: &mut [SpatialObjectId]) -> KernelResult<usize> {
        let required = self
            .objects
            .values()
            .filter(|item| aabb_overlaps(**item, aabb))
            .count();
        if required > out.len() {
            return Err(KernelError::buffer_too_small(
                required as u64,
                out.len() as u64,
            ));
        }
        let mut written = 0;
        for (id, item) in &self.objects {
            if aabb_overlaps(*item, aabb) {
                out[written] = *id;
                written += 1;
            }
        }
        Ok(written)
    }
}
