//! T-spatial-04 / R-00110: RStarIndexAdapter matches GridReferenceIndex on a fixed corpus.

use lumio_kernel::error::{ErrorCategory, ErrorDetail};
use lumio_spatial::{
    Aabb3, GridReferenceIndex, Point3, RStarIndexAdapter, SpatialIndexBackend, SpatialObjectId,
};

fn aabb(min: (f32, f32, f32), max: (f32, f32, f32)) -> Aabb3 {
    Aabb3::new(
        Point3::new(min.0, min.1, min.2).expect("finite min"),
        Point3::new(max.0, max.1, max.2).expect("finite max"),
    )
    .expect("finite AABB")
}

fn id(v: u64) -> SpatialObjectId {
    SpatialObjectId::from_raw(v)
}

fn upsert_all(index: &mut dyn SpatialIndexBackend, items: &[(SpatialObjectId, Aabb3)]) {
    for (oid, box_) in items {
        index.upsert(*oid, *box_).expect("upsert");
    }
}

fn query_hits(index: &dyn SpatialIndexBackend, query: Aabb3) -> Vec<SpatialObjectId> {
    let mut out = [id(u64::MAX); 8];
    let n = index.query_aabb(query, &mut out).expect("query");
    out[..n].to_vec()
}

fn required_capacity(err: &lumio_kernel::error::KernelError) -> u64 {
    match err.detail() {
        ErrorDetail::RequiredCapacity { required, .. } => *required,
        other => panic!("unexpected detail: {other:?}"),
    }
}

#[test]
fn rstar_matches_reference_corpus() {
    let adapter_name = std::any::type_name::<RStarIndexAdapter>();
    assert!(
        adapter_name.contains("lumio_spatial"),
        "adapter type must stay in crate paths: {adapter_name}"
    );
    assert!(
        !adapter_name.contains("::rstar::"),
        "adapter must not leak rstar crate types: {adapter_name}"
    );

    let overlapping = [
        (id(30), aabb((0.0, 0.0, 0.0), (2.0, 2.0, 2.0))),
        (id(10), aabb((1.0, 1.0, 1.0), (3.0, 3.0, 3.0))),
        (id(20), aabb((1.5, 1.5, 1.5), (1.5, 1.5, 1.5))),
    ];
    let miss = (id(99), aabb((100.0, 100.0, 100.0), (101.0, 101.0, 101.0)));
    let query = aabb((0.0, 0.0, 0.0), (3.0, 3.0, 3.0));
    let miss_query = aabb((50.0, 50.0, 50.0), (51.0, 51.0, 51.0));
    let expected = [id(10), id(20), id(30)];

    let mut reference = GridReferenceIndex::new();
    upsert_all(&mut reference, &overlapping);
    reference.upsert(miss.0, miss.1).expect("miss on reference");

    let mut adapter = RStarIndexAdapter::new();
    adapter
        .upsert(miss.0, miss.1)
        .expect("miss on adapter first");
    upsert_all(
        &mut adapter,
        &[overlapping[2], overlapping[1], overlapping[0]],
    );

    let hits_reference = query_hits(&reference, query);
    let hits_adapter = query_hits(&adapter, query);
    assert_eq!(hits_reference, expected);
    assert_eq!(hits_adapter, expected);
    assert_eq!(hits_reference, hits_adapter);
    assert!(
        !hits_reference.contains(&miss.0) && !hits_adapter.contains(&miss.0),
        "non-overlapping object must be absent from both"
    );
    assert_eq!(
        query_hits(&reference, miss_query),
        query_hits(&adapter, miss_query)
    );
    assert!(query_hits(&adapter, miss_query).is_empty());

    let sentinel = id(0xDEAD);
    let mut tiny_reference = [sentinel, sentinel];
    let mut tiny_adapter = [sentinel, sentinel];
    let err_reference = reference
        .query_aabb(query, &mut tiny_reference[..1])
        .expect_err("reference undersized out");
    let err_adapter = adapter
        .query_aabb(query, &mut tiny_adapter[..1])
        .expect_err("adapter undersized out");
    assert_eq!(err_reference.category(), ErrorCategory::BufferTooSmall);
    assert_eq!(err_adapter.category(), ErrorCategory::BufferTooSmall);
    assert_eq!(
        required_capacity(&err_reference),
        required_capacity(&err_adapter)
    );
    assert_eq!(err_reference, err_adapter);
    assert_eq!(tiny_reference, [sentinel, sentinel]);
    assert_eq!(tiny_adapter, [sentinel, sentinel]);
}
