//! T-memory-01: CallerOutputBuffer::write_all is all-or-nothing.

use lumio_kernel::error::{ErrorCategory, ErrorDetail};
use lumio_kernel::memory::CallerOutputBuffer;

#[test]
fn caller_output_write_is_atomic() {
    let mut dest = [0u8; 4];
    let mut out = CallerOutputBuffer::new(&mut dest);

    out.write_all(&[1, 2, 3])
        .expect("3 bytes fit in 4-byte dest");
    assert_eq!(out.written(), 3);
    assert_eq!(out.capacity(), 4);

    let err = out
        .write_all(&[4, 5])
        .expect_err("2 more bytes must not fit");
    assert_eq!(err.category(), ErrorCategory::BufferTooSmall);
    match err.detail() {
        ErrorDetail::RequiredCapacity { required, provided } => {
            assert_eq!(*required, 5);
            assert_eq!(*provided, 4);
        }
        other => panic!("unexpected detail: {other:?}"),
    }
    assert_eq!(out.written(), 3);
    assert_eq!(out.finish(), &[1, 2, 3]);
    assert_eq!(dest, [1, 2, 3, 0]);
}
