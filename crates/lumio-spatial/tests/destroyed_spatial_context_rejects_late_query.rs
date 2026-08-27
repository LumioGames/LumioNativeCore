//! T-spatial-06 / R-00173: destroyed SpatialResource rejects late AABB query.

use std::sync::Arc;

use lumio_kernel::capability::ConfiguredLimits;
use lumio_kernel::context::{
    CancelReason, ContextConfig, ContextPhase, ContextResource, Deadline, KernelContext,
};
use lumio_kernel::error::{ErrorCategory, KernelError};
use lumio_spatial::{Aabb3, AabbQuery, Point3, SpatialHit, SpatialObjectId, SpatialResource};

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

fn test_config() -> ContextConfig {
    ContextConfig {
        limits: ConfiguredLimits {
            max_handles: 4,
            max_native_bytes: 64,
            max_jobs_queued: 1,
            max_jobs_running: 1,
            max_completion_items: 1,
        },
        quiesce_deadline: Deadline::NONE,
    }
}

fn assert_closing_or_destroyed(err: &KernelError, what: &str) {
    let category = err.category();
    assert!(
        category == ErrorCategory::ContextClosing || category == ErrorCategory::ContextDestroyed,
        "{what}: expected ContextClosing or ContextDestroyed, got {category:?}"
    );
}

fn assert_no_vendor(name: &str) {
    let lower = name.to_ascii_lowercase();
    for vendor in ["rstar", "kiddo", "parry", "ncollide"] {
        assert!(
            !lower.contains(vendor),
            "Spatial resource leaked vendor type `{vendor}` via {name}"
        );
    }
}

#[test]
fn destroyed_spatial_context_rejects_late_query() {
    for name in [
        std::any::type_name::<SpatialResource>(),
        std::any::type_name::<SpatialHit>(),
        std::any::type_name::<AabbQuery>(),
    ] {
        assert_no_vendor(name);
        assert!(
            name.contains("lumio_spatial"),
            "spatial resource type must stay in crate paths: {name}"
        );
    }

    let ctx = KernelContext::create_for_test(test_config());
    let _ = ctx.key();
    ctx.ensure_accepting_work()
        .expect("running context accepts work");

    let mut spatial = SpatialResource::new();
    spatial
        .upsert(id(10), aabb((0.0, 0.0, 0.0), (2.0, 2.0, 2.0)))
        .expect("upsert AABB");
    let spatial = Arc::new(spatial);

    ctx.register_resource(Arc::clone(&spatial) as Arc<dyn ContextResource>)
        .expect("register spatial");

    let queries = [AabbQuery {
        aabb: aabb((0.0, 0.0, 0.0), (2.0, 2.0, 2.0)),
    }];
    let mut out = [SpatialHit {
        query_ordinal: 0,
        object_id: id(0),
    }; 4];
    let n = spatial
        .query_aabb_batch(&queries, &mut out)
        .expect("query before close");
    assert_eq!(n, 1);
    assert_eq!(
        out[0],
        SpatialHit {
            query_ordinal: 0,
            object_id: id(10),
        }
    );

    let first = ctx
        .close(CancelReason::ContextClosing, Deadline::NONE)
        .expect("first close");
    assert_eq!(first.phase, ContextPhase::Closed);

    let late = spatial
        .query_aabb_batch(&queries, &mut out)
        .expect_err("late query after destroy must fail");
    assert_closing_or_destroyed(&late, "query_aabb_batch after close");

    let second = ctx
        .close(CancelReason::ContextClosing, Deadline::NONE)
        .expect("second close is idempotent");
    assert_eq!(second, first);
    ContextResource::destroy(spatial.as_ref()).expect("destroy is idempotent");
    let still_late = spatial
        .query_aabb_batch(&queries, &mut out)
        .expect_err("query stays rejected after extra destroy");
    assert_closing_or_destroyed(&still_late, "query_aabb_batch after second destroy");
}
