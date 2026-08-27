//! Vendor-free spatial index backend seam. Adapters stay behind this trait.

use lumio_kernel::error::KernelResult;

use crate::types::{Aabb3, SpatialObjectId};

/// Object-safe index port. Signatures use crate POD types and kernel results only.
pub trait SpatialIndexBackend: Send + Sync + 'static {
    fn upsert(&mut self, id: SpatialObjectId, aabb: Aabb3) -> KernelResult<()>;
    fn remove(&mut self, id: SpatialObjectId) -> KernelResult<()>;

    /// Writes matching ids into `out` and returns the number written.
    /// If `out` cannot hold every hit, returns `KernelError::buffer_too_small`.
    fn query_aabb(&self, aabb: Aabb3, out: &mut [SpatialObjectId]) -> KernelResult<usize>;
}
