//! T-spatial-03 / R-00109: GridReferenceIndex query hits are sorted by SpatialObjectId.

use lumio_kernel::error::{ErrorCategory, ErrorDetail};
use lumio_spatial::{Aabb3, GridReferenceIndex, Point3, SpatialIndexBackend, SpatialObjectId};

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

fn assert_no_vendor(name: &str) {
    let lower = name.to_ascii_lowercase();
    for vendor in ["rstar", "kiddo", "parry", "ncollide"] {
        assert!(
            !lower.contains(vendor),
            "GridReferenceIndex leaked vendor type `{vendor}` via {name}"
        );
    }
}

#[test]
fn reference_results_are_stably_sorted() {
    assert_no_vendor(std::any::type_name::<GridReferenceIndex>());
    assert!(
        std::any::type_name::<GridReferenceIndex>().contains("lumio_spatial"),
        "oracle type must stay in crate paths"
    );

    let overlapping = [
        (id(30), aabb((0.0, 0.0, 0.0), (2.0, 2.0, 2.0))),
        (id(10), aabb((1.0, 1.0, 1.0), (3.0, 3.0, 3.0))),
        (id(20), aabb((1.5, 1.5, 1.5), (1.5, 1.5, 1.5))),
    ];
    let outsider = (id(99), aabb((100.0, 100.0, 100.0), (101.0, 101.0, 101.0)));
    let query = aabb((0.0, 0.0, 0.0), (3.0, 3.0, 3.0));
    let expected = [id(10), id(20), id(30)];

    let mut first = GridReferenceIndex::new();
    upsert_all(&mut first, &overlapping);
    first
        .upsert(outsider.0, outsider.1)
        .expect("outsider first");

    let mut second = GridReferenceIndex::new();
    second
        .upsert(outsider.0, outsider.1)
        .expect("outsider second first");
    upsert_all(
        &mut second,
        &[overlapping[2], overlapping[1], overlapping[0]],
    );

    let hits_first = query_hits(&first, query);
    let hits_second = query_hits(&second, query);
    assert_eq!(hits_first, expected);
    assert_eq!(hits_second, expected);
    assert_eq!(hits_first, hits_second);
    assert!(
        !hits_first.contains(&outsider.0),
        "non-overlapping object must be absent"
    );

    let sentinel = id(0xDEAD);
    let mut tiny = [sentinel, sentinel];
    let err = first
        .query_aabb(query, &mut tiny[..1])
        .expect_err("undersized out must not overflow");
    assert_eq!(err.category(), ErrorCategory::BufferTooSmall);
    match err.detail() {
        ErrorDetail::RequiredCapacity { required, provided } => {
            assert_eq!(*required, 3);
            assert_eq!(*provided, 1);
        }
        other => panic!("unexpected detail: {other:?}"),
    }
    assert_eq!(tiny, [sentinel, sentinel]);

    let missing = first.remove(id(7)).expect_err("missing remove");
    assert_eq!(missing.category(), ErrorCategory::InvalidHandle);

    first
        .upsert(id(10), aabb((50.0, 50.0, 50.0), (51.0, 51.0, 51.0)))
        .expect("replace AABB");
    assert_eq!(query_hits(&first, query), [id(20), id(30)]);
}
