//! T-codec-02 / R-00114: bounded zstd adapter rejects expansion before output.

use lumio_codec::{CodecLimits, ZstdAdapter};
use lumio_kernel::error::{ErrorCategory, ErrorDetail};

#[test]
fn zstd_decompress_rejects_expansion_limit() {
    let limits = CodecLimits {
        max_input_bytes: 100,
        max_output_bytes: 10,
        max_expansion_ratio: 1,
    };
    // 20 dummy frame-looking bytes. The seam must not need a decoder to reject.
    let input = [
        0x28, 0xB5, 0x2F, 0xFD, 0x00, 0x61, 0x00, 0x00, 0x00, 0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF,
        0x01, 0x02, 0x03, 0x04, 0x05,
    ];
    assert_eq!(input.len(), 20);

    let err = ZstdAdapter::decompress_bounded(&input, &limits)
        .expect_err("expansion/output cap must fail before any payload");
    assert_eq!(err.category(), ErrorCategory::CapacityExceeded);
    match err.detail() {
        ErrorDetail::LimitExceeded { limit, requested } => {
            assert_eq!(*limit, 10, "tighter cap is max_output_bytes");
            assert_eq!(*requested, 20, "claimed stand-in is input length");
        }
        other => panic!("expected LimitExceeded, got {other:?}"),
    }

    let name = std::any::type_name::<ZstdAdapter>();
    assert!(
        name.contains("lumio_codec"),
        "adapter type must stay in crate paths: {name}"
    );
    assert!(
        !name.contains("::zstd::"),
        "adapter must not leak zstd crate types: {name}"
    );
}
