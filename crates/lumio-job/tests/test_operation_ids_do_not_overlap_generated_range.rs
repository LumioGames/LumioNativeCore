//! T-job-01: crate-private OperationId values do not overlap the generated registry.

use lumio_job::{JobId, OperationId, operation_id_overlaps_generated};

#[test]
fn test_operation_ids_do_not_overlap_generated_range() {
    assert!(OperationId::test_only(0).raw() >= OperationId::TEST_RANGE_START);

    // Call the shipped seam; do not hardcode false. Generated registry is empty (B-ABI-004).
    assert!(!operation_id_overlaps_generated(OperationId::test_only(1)));
    assert!(!operation_id_overlaps_generated(OperationId::from_raw(1)));

    assert_eq!(JobId::from_raw(7).raw(), 7);
}
