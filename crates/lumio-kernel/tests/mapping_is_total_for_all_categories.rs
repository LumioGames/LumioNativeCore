//! T-error-03 / R-00079: every frozen `ErrorCategory` maps to a registered
//! architecture ErrorCode (ADR-046 kernel band plus 1020/1029/1030).
//!
//! Numerics are never written here: expectations go through the generated
//! registry, whose values the contract-types tests verify against the
//! byte-pinned `ids/index.json` mirror.

use lumio_contract_types::registry;
use lumio_kernel::error::{ErrorCategory, ErrorDetail, KernelError, to_architecture_error_code};

#[test]
fn mapping_is_total_for_all_categories() {
    // (kernel category, adjudicated registry id) — ADR-046 §1–§2.
    let expected = [
        (ErrorCategory::InvalidArgument, "InvalidArgument"),
        (ErrorCategory::InvalidHandle, "InvalidHandle"),
        (ErrorCategory::WrongContext, "WrongContext"),
        // §2 release-path ruling; the use-path nuance is a known gap
        // recorded on the card, not silently resolved here.
        (ErrorCategory::AlreadyReleased, "HandleDoubleRelease"),
        (ErrorCategory::BufferTooSmall, "BufferTooSmall"),
        (ErrorCategory::CapacityExceeded, "CapacityExceeded"),
        (ErrorCategory::CapabilityUnavailable, "CapabilityMissing"),
        (ErrorCategory::Cancelled, "Cancelled"),
        (ErrorCategory::TimedOut, "TimedOut"),
        (ErrorCategory::ContextClosing, "ContextClosing"),
        (ErrorCategory::ContextDestroyed, "ContextDestroyed"),
        (ErrorCategory::PanicBoundary, "PanicBoundary"),
        (ErrorCategory::InternalInvariant, "InternalInvariant"),
    ];

    let mut mapped = Vec::new();
    for (category, registry_id) in expected {
        let err = KernelError::new(category, ErrorDetail::None);
        assert_eq!(err.category(), category);

        let code = to_architecture_error_code(&err);
        assert_eq!(code.id(), registry_id, "wrong mapping for {category:?}");
        assert_eq!(
            Some(code),
            registry::error_code(registry_id),
            "mapped code must be the registered instance for {registry_id}"
        );
        assert!(code.numeric() > 0, "0 is reserved for success");
        mapped.push(code);
    }

    // 13 类映射到 13 个互不相同的注册值（单射）。
    for (i, a) in mapped.iter().enumerate() {
        for b in &mapped[i + 1..] {
            assert_ne!(a.numeric(), b.numeric(), "mapping must stay injective");
        }
    }
}
