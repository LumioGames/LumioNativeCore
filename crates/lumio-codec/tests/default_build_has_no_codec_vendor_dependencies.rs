//! T-codec-01 / R-00113: default build exposes CodecLimits and names no codec vendors.

use std::fs;
use std::path::Path;

use lumio_codec::CodecLimits;
use lumio_kernel::error::ErrorCategory;

const FORBIDDEN_VENDORS: &[&str] = &["zstd", "lz4", "lz4_flex", "flate2", "brotli", "snap"];

fn assert_zero_rejected(limits: CodecLimits, what: &str) {
    let err = limits.validate().expect_err(what);
    assert_eq!(err.category(), ErrorCategory::InvalidArgument);
}

#[test]
fn default_build_has_no_codec_vendor_dependencies() {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
    let text = fs::read_to_string(&manifest)
        .unwrap_or_else(|e| panic!("read {}: {e}", manifest.display()));
    let lower = text.to_ascii_lowercase();
    for vendor in FORBIDDEN_VENDORS {
        assert!(
            !lower.contains(vendor),
            "{} must not mention `{vendor}`",
            manifest.display()
        );
    }

    let ok = CodecLimits {
        max_input_bytes: 1024,
        max_output_bytes: 2048,
        max_expansion_ratio: 2,
    };
    ok.validate().expect("positive triple must be accepted");

    let mut zero_input = ok;
    zero_input.max_input_bytes = 0;
    assert_zero_rejected(zero_input, "zero max_input_bytes must fail");

    let mut zero_output = ok;
    zero_output.max_output_bytes = 0;
    assert_zero_rejected(zero_output, "zero max_output_bytes must fail");

    let mut zero_ratio = ok;
    zero_ratio.max_expansion_ratio = 0;
    assert_zero_rejected(zero_ratio, "zero max_expansion_ratio must fail");

    assert!(
        ok.expansion_would_exceed(10, 21),
        "ratio 2: 10 → 21 must exceed"
    );
    assert!(
        !ok.expansion_would_exceed(10, 20),
        "ratio 2: 10 → 20 is the exact cap"
    );
}
