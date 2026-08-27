//! T-memory-02 / R-00092: AllocationProvenance preserves allocator, context, class.

use lumio_kernel::handle::ContextKey;
use lumio_kernel::memory::{AllocationClass, AllocationProvenance, AllocatorId};

#[test]
fn provenance_preserves_allocator_context_class() {
    let provenance = AllocationProvenance {
        allocator: AllocatorId::new(7),
        context: ContextKey::new(9),
        class: AllocationClass::NativeOwnedBuffer,
        requested_bytes: 16,
        charged_bytes: 32,
    };

    assert_eq!(provenance.allocator, AllocatorId::new(7));
    assert_eq!(provenance.allocator.raw(), 7);
    assert_eq!(provenance.context, ContextKey::new(9));
    assert_eq!(provenance.context.raw(), 9);
    assert_eq!(provenance.class, AllocationClass::NativeOwnedBuffer);
    assert_eq!(provenance.requested_bytes, 16);
    assert_eq!(provenance.charged_bytes, 32);
}
