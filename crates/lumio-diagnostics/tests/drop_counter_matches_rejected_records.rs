//! T-diagnostics-03 / R-00122: drop counters match full-queue rejects.

use lumio_diagnostics::{
    BoundedRecorder, KernelRecordRef, OwnedKernelRecord, RecordDisposition, RecorderCounters,
};
use lumio_kernel::error::ErrorCategory;

fn try_record(rec: &BoundedRecorder, kind: &str, payload: &[u8]) -> RecordDisposition {
    let fields = [kind];
    rec.try_record(KernelRecordRef {
        fields: &fields,
        payload,
    })
}

fn empty_owned() -> OwnedKernelRecord {
    OwnedKernelRecord::try_from_ref(
        KernelRecordRef {
            fields: &[],
            payload: &[],
        },
        0,
        0,
    )
    .expect("empty placeholder")
}

#[test]
fn drop_counter_matches_rejected_records() {
    let manifest = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml"));
    for forbidden in ["tracing", "metrics", "crossbeam"] {
        assert!(
            !manifest.contains(forbidden),
            "default Cargo.toml must not list {forbidden}"
        );
    }

    match BoundedRecorder::with_capacity(0, 32) {
        Err(err) => assert_eq!(err.category(), ErrorCategory::InvalidArgument),
        Ok(_) => panic!("zero capacity must be InvalidArgument"),
    }

    let rec = match BoundedRecorder::with_capacity(1, 32) {
        Ok(rec) => rec,
        Err(_) => panic!("capacity 1 is valid"),
    };

    assert_eq!(try_record(&rec, "a", b"1"), RecordDisposition::Accepted);
    let RecorderCounters {
        accepted,
        dropped_full,
    } = rec.counters();
    assert_eq!(accepted, 1);
    assert_eq!(dropped_full, 0);

    assert_eq!(try_record(&rec, "b", b"22"), RecordDisposition::DroppedFull);
    let counters = rec.counters();
    assert_eq!(counters.accepted, 1);
    assert_eq!(counters.dropped_full, 1);

    let mut out = [empty_owned()];
    let n = rec.drain(&mut out);
    assert_eq!(n, 1, "drain pops the accepted record");
    assert_eq!(out[0].field_count(), 1);
    assert_eq!(out[0].byte_len(), 2);

    let oversized = [0u8; 64];
    let fields: [&str; 0] = [];
    let oversized_disp = rec.try_record(KernelRecordRef {
        fields: &fields,
        payload: &oversized,
    });
    assert_eq!(
        oversized_disp,
        RecordDisposition::DroppedOversized,
        "oversized must not be collapsed into queue-full"
    );

    let counters = rec.counters();
    assert_eq!(counters.accepted, 1);
    assert_eq!(
        counters.dropped_full, 1,
        "dropped_full matches the number of full-queue rejects"
    );
}
