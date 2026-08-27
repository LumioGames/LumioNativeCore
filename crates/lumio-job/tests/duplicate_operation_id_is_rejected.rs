//! T-job-04 / R-00106: OperationRegistry rejects a second kernel with the same id.

use std::sync::Arc;

use lumio_job::{OperationId, OperationRegistry, TypedKernel};
use lumio_kernel::error::ErrorCategory;

struct DummyKernel {
    id: OperationId,
}

impl TypedKernel for DummyKernel {
    fn operation_id(&self) -> OperationId {
        self.id
    }
}

#[test]
fn duplicate_operation_id_is_rejected() {
    let mut registry = OperationRegistry::new();
    let id = OperationId::test_only(1);
    let first = Arc::new(DummyKernel { id });
    let second = Arc::new(DummyKernel { id });

    registry
        .register(first)
        .expect("first registration must succeed");

    let err = registry
        .register(second)
        .expect_err("duplicate operation id must be rejected");
    assert_eq!(err.category(), ErrorCategory::InvalidArgument);

    assert!(registry.get(id).is_some());
}
