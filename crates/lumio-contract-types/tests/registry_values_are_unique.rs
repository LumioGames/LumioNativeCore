mod common;

use lumio_contract_types::registry;
use lumio_contract_types::{ArchitectureErrorCode, ArchitectureOperationId, CapabilityBits};

/// The bound `ErrorCode` table must equal the published registry mirror
/// entry for entry (ids unique, numerics unique and status-range safe), and
/// the namespaces that stay unbound must stay empty.
#[test]
fn registry_values_are_unique() {
    let ids = common::parse_mirror("ids-index.json");
    let error_ns = ids
        .get("namespaces")
        .as_arr()
        .iter()
        .find(|ns| ns.get("namespace").as_str() == "ErrorCode")
        .expect("mirror ErrorCode namespace");
    assert_eq!(error_ns.get("owner").as_str(), "Architecture");

    let mirror: Vec<(&str, i64)> = error_ns
        .get("values")
        .as_arr()
        .iter()
        .map(|v| {
            assert_eq!(v.get("status").as_str(), "Active");
            (v.get("id").as_str(), v.get("numeric").as_i64())
        })
        .collect();

    let bound: Vec<ArchitectureErrorCode> = registry::error_codes().collect();
    assert_eq!(
        bound.len(),
        mirror.len(),
        "bound table must carry every published ErrorCode value"
    );
    for (code, (mirror_id, mirror_numeric)) in bound.iter().zip(&mirror) {
        assert_eq!(code.id(), *mirror_id);
        assert_eq!(i64::from(code.numeric()), *mirror_numeric);
        // ADR-040 §3 / ADR-046：status 值域 (0, i32::MAX]，0 只留给成功。
        assert!(code.numeric() > 0);
    }

    for (i, a) in bound.iter().enumerate() {
        for b in &bound[i + 1..] {
            assert_ne!(a.id(), b.id(), "duplicate ErrorCode id");
            assert_ne!(a.numeric(), b.numeric(), "duplicate ErrorCode numeric");
        }
    }

    for code in &bound {
        assert_eq!(registry::error_code(code.id()), Some(*code));
    }
    assert_eq!(registry::error_code("NotARegisteredId"), None);

    // OperationId 不存在（B-ABI-004 不适用）；Capability 绑定待 D-015。
    let operation_ids: Vec<ArchitectureOperationId> = registry::operation_ids().collect();
    let capability_bits: Vec<CapabilityBits> = registry::capability_bits().collect();
    assert_eq!(operation_ids.len(), 0);
    assert_eq!(capability_bits.len(), 0);
}

/// The ADR-046 kernel status band must be present through the registry —
/// numerics compared against the mirror, never hard-coded here.
#[test]
fn kernel_status_band_is_bound() {
    let ids = common::parse_mirror("ids-index.json");
    let error_ns = ids
        .get("namespaces")
        .as_arr()
        .iter()
        .find(|ns| ns.get("namespace").as_str() == "ErrorCode")
        .expect("mirror ErrorCode namespace");

    for band_id in [
        "InvalidArgument",
        "WrongContext",
        "BufferTooSmall",
        "CapacityExceeded",
        "Cancelled",
        "TimedOut",
        "ContextClosing",
        "ContextDestroyed",
        "PanicBoundary",
        "InternalInvariant",
        // ADR-046 §2：由既有值承担的三类。
        "InvalidHandle",
        "HandleDoubleRelease",
        "CapabilityMissing",
    ] {
        let mirror_numeric = error_ns
            .get("values")
            .as_arr()
            .iter()
            .find(|v| v.get("id").as_str() == band_id)
            .unwrap_or_else(|| panic!("mirror missing ErrorCode {band_id}"))
            .get("numeric")
            .as_i64();
        let code = registry::error_code(band_id)
            .unwrap_or_else(|| panic!("registry missing ErrorCode {band_id}"));
        assert_eq!(i64::from(code.numeric()), mirror_numeric);
    }
}
