//! T-codec-03 / R-00115: Lz4Adapter rejects truncated frames without a vendor crate.

use std::fs;
use std::path::Path;

use lumio_codec::{CodecLimits, Lz4Adapter};
use lumio_kernel::error::ErrorCategory;

#[test]
fn lz4_decompress_rejects_truncated_input() {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
    let text = fs::read_to_string(&manifest)
        .unwrap_or_else(|e| panic!("read {}: {e}", manifest.display()));
    let lower = text.to_ascii_lowercase();
    assert!(
        !lower.contains("lz4"),
        "{} must not mention `lz4`",
        manifest.display()
    );

    let limits = CodecLimits {
        max_input_bytes: 1024,
        max_output_bytes: 2048,
        max_expansion_ratio: 2,
    };
    let err = Lz4Adapter::decompress_bounded(&[0x01], &limits)
        .expect_err("truncated 1-byte input must fail");
    assert_eq!(err.category(), ErrorCategory::InvalidArgument);
}
