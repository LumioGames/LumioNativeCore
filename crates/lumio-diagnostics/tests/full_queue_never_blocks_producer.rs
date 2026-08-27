//! T-diagnostics-02 / R-00120: a full bounded RecordQueue fails immediately, never blocks.

use lumio_diagnostics::{KernelRecordRef, OwnedKernelRecord, RecordQueue};
use lumio_kernel::error::{ErrorCategory, ErrorDetail};

fn small_record(kind: &str, payload: &[u8]) -> OwnedKernelRecord {
    let fields = [kind];
    OwnedKernelRecord::try_from_ref(
        KernelRecordRef {
            fields: &fields,
            payload,
        },
        4,
        32,
    )
    .expect("small record within field and byte limits")
}

#[test]
fn full_queue_never_blocks_producer() {
    let manifest = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml"));
    for forbidden in ["tracing", "metrics", "crossbeam"] {
        assert!(
            !manifest.contains(forbidden),
            "default Cargo.toml must not list {forbidden}"
        );
    }

    let q = RecordQueue::with_capacity(1);

    let first = small_record("a", b"1");
    let first_fields = first.field_count();
    let first_bytes = first.byte_len();
    assert!(q.try_push(first).is_ok());

    let second = small_record("b", b"22");
    let err = q
        .try_push(second)
        .expect_err("full queue must return CapacityExceeded immediately");
    assert_eq!(err.category(), ErrorCategory::CapacityExceeded);
    match err.detail() {
        ErrorDetail::LimitExceeded { limit, requested } => {
            assert_eq!(*limit, 1);
            assert_eq!(*requested, 2);
        }
        other => panic!("unexpected detail: {other:?}"),
    }

    let popped = q.try_pop().expect("first record still queued");
    assert_eq!(popped.field_count(), first_fields);
    assert_eq!(popped.byte_len(), first_bytes);

    let third = small_record("c", b"3");
    assert!(q.try_push(third).is_ok());
}
