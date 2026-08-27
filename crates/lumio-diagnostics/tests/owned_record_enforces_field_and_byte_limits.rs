//! T-diagnostics-01 / R-00118: owned bounded record enforces field and byte caps.

use lumio_diagnostics::{KernelRecordRef, OwnedKernelRecord};
use lumio_kernel::error::{ErrorCategory, ErrorDetail, KernelError};

fn copied_byte_len(fields: &[&str], payload: &[u8]) -> usize {
    fields.iter().map(|field| field.len()).sum::<usize>() + payload.len()
}

fn assert_capacity_exceeded<T: std::fmt::Debug>(
    result: Result<T, KernelError>,
    what: &str,
    limit: u64,
    requested: u64,
) {
    let err = result.expect_err(what);
    assert_eq!(err.category(), ErrorCategory::CapacityExceeded, "{what}");
    match err.detail() {
        ErrorDetail::LimitExceeded {
            limit: got_limit,
            requested: got_requested,
        } => {
            assert_eq!(*got_limit, limit, "{what}: limit");
            assert_eq!(*got_requested, requested, "{what}: requested");
        }
        other => panic!("{what}: unexpected detail {other:?}"),
    }
}

#[test]
fn owned_record_enforces_field_and_byte_limits() {
    let manifest = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml"));
    for forbidden in ["tracing", "metrics", "crossbeam"] {
        assert!(
            !manifest.contains(forbidden),
            "default Cargo.toml must not list {forbidden}"
        );
    }

    let fields = ["kind", "tick"];
    let payload = b"ok-payload";
    let borrowed = KernelRecordRef {
        fields: &fields,
        payload,
    };
    let within_bytes = copied_byte_len(&fields, payload);

    let owned = OwnedKernelRecord::try_from_ref(borrowed, 4, 64).expect("within limits");
    assert_eq!(owned.field_count(), fields.len());
    assert_eq!(owned.byte_len(), within_bytes);

    let exact =
        OwnedKernelRecord::try_from_ref(borrowed, 2, within_bytes).expect("equal-to-limit is ok");
    assert_eq!(exact.field_count(), 2);
    assert_eq!(exact.byte_len(), within_bytes);

    assert_capacity_exceeded(
        OwnedKernelRecord::try_from_ref(borrowed, 1, 64),
        "too many fields",
        1,
        2,
    );

    let over_bytes = within_bytes - 1;
    assert_capacity_exceeded(
        OwnedKernelRecord::try_from_ref(borrowed, 4, over_bytes),
        "payload/bytes over max_bytes",
        over_bytes as u64,
        within_bytes as u64,
    );

    let payload_only = KernelRecordRef {
        fields: &[],
        payload: b"0123456789",
    };
    assert_capacity_exceeded(
        OwnedKernelRecord::try_from_ref(payload_only, 0, 9),
        "payload over max_bytes with no fields",
        9,
        10,
    );
}
