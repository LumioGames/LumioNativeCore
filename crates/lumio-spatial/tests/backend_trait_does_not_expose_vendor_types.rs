//! T-spatial-02 / R-00108: SpatialIndexBackend is object-safe and vendor-free.

use lumio_kernel::error::{ErrorCategory, ErrorDetail, KernelError, KernelResult};
use lumio_spatial::{Aabb3, Point3, SpatialIndexBackend, SpatialObjectId};

fn assert_obj(_: &dyn SpatialIndexBackend) {}

fn aabb(min: (f32, f32, f32), max: (f32, f32, f32)) -> Aabb3 {
    Aabb3::new(
        Point3::new(min.0, min.1, min.2).expect("finite min"),
        Point3::new(max.0, max.1, max.2).expect("finite max"),
    )
    .expect("finite AABB")
}

fn overlaps(a: Aabb3, b: Aabb3) -> bool {
    a.min.x <= b.max.x
        && a.max.x >= b.min.x
        && a.min.y <= b.max.y
        && a.max.y >= b.min.y
        && a.min.z <= b.max.z
        && a.max.z >= b.min.z
}

/// Test-only Vec store. Production adapters land in later spatial cards.
struct VecIndex {
    items: Vec<(SpatialObjectId, Aabb3)>,
}

impl SpatialIndexBackend for VecIndex {
    fn upsert(&mut self, id: SpatialObjectId, aabb: Aabb3) -> KernelResult<()> {
        if let Some((_, existing)) = self.items.iter_mut().find(|(existing, _)| *existing == id) {
            *existing = aabb;
        } else {
            self.items.push((id, aabb));
        }
        Ok(())
    }

    fn remove(&mut self, id: SpatialObjectId) -> KernelResult<()> {
        self.items.retain(|(existing, _)| *existing != id);
        Ok(())
    }

    fn query_aabb(&self, aabb: Aabb3, out: &mut [SpatialObjectId]) -> KernelResult<usize> {
        let required = self
            .items
            .iter()
            .filter(|(_, item)| overlaps(*item, aabb))
            .count();
        if required > out.len() {
            return Err(KernelError::buffer_too_small(
                required as u64,
                out.len() as u64,
            ));
        }
        let mut written = 0;
        for (id, item) in &self.items {
            if overlaps(*item, aabb) {
                out[written] = *id;
                written += 1;
            }
        }
        Ok(written)
    }
}

fn assert_no_vendor(name: &str) {
    let lower = name.to_ascii_lowercase();
    for vendor in ["rstar", "kiddo", "parry", "ncollide"] {
        assert!(
            !lower.contains(vendor),
            "spatial seam leaked vendor type `{vendor}` via {name}"
        );
    }
}

#[test]
fn backend_trait_does_not_expose_vendor_types() {
    for name in [
        std::any::type_name::<dyn SpatialIndexBackend>(),
        std::any::type_name::<SpatialObjectId>(),
        std::any::type_name::<Point3>(),
        std::any::type_name::<Aabb3>(),
        std::any::type_name::<KernelResult<usize>>(),
    ] {
        assert_no_vendor(name);
        assert!(
            name.contains("lumio_spatial") || name.contains("lumio_kernel"),
            "seam type must stay in crate/kernel paths: {name}"
        );
    }

    assert!(
        Point3::new(f32::NAN, 0.0, 0.0).is_err(),
        "non-finite points must fail at Aabb3/Point3 construction"
    );
    assert!(
        Aabb3::new(
            Point3 {
                x: f32::INFINITY,
                y: 0.0,
                z: 0.0
            },
            Point3::new(1.0, 1.0, 1.0).expect("finite max"),
        )
        .is_err(),
        "non-finite AABB must be rejected before a backend sees it"
    );

    let a = SpatialObjectId::from_raw(1);
    let b = SpatialObjectId::from_raw(2);
    let mut index = VecIndex { items: Vec::new() };
    let backend: &mut dyn SpatialIndexBackend = &mut index;
    assert_obj(backend);

    backend
        .upsert(a, aabb((0.0, 0.0, 0.0), (1.0, 1.0, 1.0)))
        .expect("upsert a");
    backend
        .upsert(b, aabb((10.0, 10.0, 10.0), (11.0, 11.0, 11.0)))
        .expect("upsert b");

    let mut out = [SpatialObjectId::from_raw(0); 2];
    let n = backend
        .query_aabb(aabb((0.0, 0.0, 0.0), (1.0, 1.0, 1.0)), &mut out)
        .expect("query a");
    assert_eq!(n, 1);
    assert_eq!(out[0], a);

    let err = backend
        .query_aabb(aabb((0.0, 0.0, 0.0), (11.0, 11.0, 11.0)), &mut [])
        .expect_err("short output must not overflow");
    assert_eq!(err.category(), ErrorCategory::BufferTooSmall);
    match err.detail() {
        ErrorDetail::RequiredCapacity { required, provided } => {
            assert_eq!(*required, 2);
            assert_eq!(*provided, 0);
        }
        other => panic!("unexpected detail: {other:?}"),
    }

    backend.remove(a).expect("remove a");
    let n = backend
        .query_aabb(aabb((0.0, 0.0, 0.0), (1.0, 1.0, 1.0)), &mut out)
        .expect("query after remove");
    assert_eq!(n, 0);
}
