//! Vendor-free batch AABB query. Hits are sorted; overflow does not write `out`.

use lumio_kernel::error::{ErrorCategory, ErrorDetail, KernelError, KernelResult};

use crate::index::{GridReferenceIndex, SpatialIndexBackend};
use crate::types::{Aabb3, SpatialObjectId};

/// One hit from a batched AABB query. Ordered by `(query_ordinal, object_id)`.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct SpatialHit {
    pub query_ordinal: u32,
    pub object_id: SpatialObjectId,
}

/// One AABB query in a batch. Ordinal is the slice index of this query.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AabbQuery {
    pub aabb: Aabb3,
}

/// Spatial index owner. Batch query is the sizing and sort surface.
pub struct SpatialContext {
    index: GridReferenceIndex,
}

impl SpatialContext {
    pub fn new() -> Self {
        Self {
            index: GridReferenceIndex::new(),
        }
    }

    pub fn upsert(&mut self, id: SpatialObjectId, aabb: Aabb3) -> KernelResult<()> {
        self.index.upsert(id, aabb)
    }

    /// Writes every matching hit into `out` and returns the number written.
    ///
    /// Hits are sorted by `(query_ordinal, object_id)`. If `out` cannot hold
    /// the full result, returns `buffer_too_small` and leaves `out` unchanged.
    pub fn query_aabb_batch(
        &self,
        queries: &[AabbQuery],
        out: &mut [SpatialHit],
    ) -> KernelResult<usize> {
        let mut hits = Vec::new();
        let mut scratch = Vec::new();
        for (ordinal, query) in queries.iter().enumerate() {
            let query_ordinal = u32::try_from(ordinal).map_err(|_| {
                KernelError::new(
                    ErrorCategory::CapacityExceeded,
                    ErrorDetail::LimitExceeded {
                        limit: u32::MAX as u64,
                        requested: queries.len() as u64,
                    },
                )
            })?;
            let n = collect_query_ids(&self.index, query.aabb, &mut scratch)?;
            hits.extend(scratch[..n].iter().copied().map(|object_id| SpatialHit {
                query_ordinal,
                object_id,
            }));
        }
        hits.sort_unstable();

        let required = hits.len();
        if required > out.len() {
            return Err(KernelError::buffer_too_small(
                required as u64,
                out.len() as u64,
            ));
        }
        out[..required].copy_from_slice(&hits);
        Ok(required)
    }
}

/// Fills `scratch` with ids for `aabb`. Grows from `buffer_too_small` then retries.
fn collect_query_ids(
    index: &GridReferenceIndex,
    aabb: Aabb3,
    scratch: &mut Vec<SpatialObjectId>,
) -> KernelResult<usize> {
    match index.query_aabb(aabb, scratch) {
        Ok(n) => return Ok(n),
        Err(err) if err.category() == ErrorCategory::BufferTooSmall => {
            let needed = match err.detail() {
                ErrorDetail::RequiredCapacity { required, .. } => {
                    usize::try_from(*required).unwrap_or(usize::MAX)
                }
                _ => return Err(err),
            };
            if needed <= scratch.len() {
                return Err(err);
            }
            scratch.resize(needed, SpatialObjectId::from_raw(0));
        }
        Err(err) => return Err(err),
    }
    index.query_aabb(aabb, scratch)
}
