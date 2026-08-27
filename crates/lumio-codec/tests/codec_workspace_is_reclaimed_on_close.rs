//! T-codec-05 / R-00175: codec workspace reclaimed on KernelContext close.

use std::sync::Arc;

use lumio_codec::{CodecResource, CodecWorkspace};
use lumio_kernel::capability::ConfiguredLimits;
use lumio_kernel::context::{
    CancelReason, ContextConfig, ContextPhase, ContextResource, Deadline, KernelContext,
};
use lumio_kernel::error::{ErrorCategory, KernelError};

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
    for vendor in ["zstd", "lz4", "lz4_flex", "flate2", "brotli", "snap"] {
        assert!(
            !lower.contains(vendor),
            "codec resource leaked vendor type `{vendor}` via {name}"
        );
    }
}

#[test]
fn codec_workspace_is_reclaimed_on_close() {
    for name in [
        std::any::type_name::<CodecResource>(),
        std::any::type_name::<CodecWorkspace>(),
    ] {
        assert_no_vendor(name);
        assert!(
            name.contains("lumio_codec"),
            "codec resource type must stay in crate paths: {name}"
        );
    }

    let ctx = KernelContext::create_for_test(test_config());
    let _ = ctx.key();
    ctx.ensure_accepting_work()
        .expect("running context accepts work");

    let resource = Arc::new(CodecResource::new());
    resource.reserve(16).expect("reserve scratch");
    assert_eq!(resource.workspace_len(), 16);
    resource.try_use().expect("workspace usable before close");

    let registration = ctx
        .register_resource(Arc::clone(&resource) as Arc<dyn ContextResource>)
        .expect("register codec");
    assert_eq!(registration.name, "codec");

    let first = ctx
        .close(CancelReason::ContextClosing, Deadline::NONE)
        .expect("first close");
    assert_eq!(first.phase, ContextPhase::Closed);

    assert_eq!(
        resource.workspace_len(),
        0,
        "workspace live bytes must be 0 after close"
    );
    let late = resource
        .try_use()
        .expect_err("late try_use after destroy must fail");
    assert_closing_or_destroyed(&late, "try_use after close");
    let late_reserve = resource
        .reserve(16)
        .expect_err("late reserve after destroy must fail");
    assert_closing_or_destroyed(&late_reserve, "reserve after close");

    let second = ctx
        .close(CancelReason::ContextClosing, Deadline::NONE)
        .expect("second close is idempotent");
    assert_eq!(second, first);
    ContextResource::destroy(resource.as_ref()).expect("destroy is idempotent");
    assert_eq!(resource.workspace_len(), 0);
    let still_late = resource
        .try_use()
        .expect_err("try_use stays rejected after extra destroy");
    assert_closing_or_destroyed(&still_late, "try_use after second destroy");
}
