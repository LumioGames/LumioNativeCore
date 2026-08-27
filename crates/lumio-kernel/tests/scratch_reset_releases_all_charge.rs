//! T-memory-05 / R-00097: CallScratch reset releases all charged bytes.

use lumio_kernel::memory::{CallScratch, MemoryBudget};

#[test]
fn scratch_reset_releases_all_charge() {
    let mut scratch = CallScratch::new(MemoryBudget::new(100));

    scratch.alloc(40).expect("alloc 40");
    assert_eq!(scratch.charged(), 40);

    scratch.alloc(30).expect("alloc 30");
    assert_eq!(scratch.charged(), 70);

    scratch.reset();
    assert_eq!(scratch.charged(), 0);

    scratch.alloc(40).expect("alloc 40 after reset");
}
