//! Bounded zstd decompress adapter seam.
//!
//! `zstd` is not approved (`EXTERNAL_ALLOWLIST` is empty). This is the
//! blocked-supplier fallback, not a published zstd binding. Expansion
//! (output cap / ratio) is rejected before any output is produced. When
//! zstd is approved, only this file should change.

use lumio_kernel::error::{ErrorCategory, ErrorDetail, KernelError, KernelResult};

use crate::bounds::CodecLimits;

/// Zip-bomb seam. Does not call a zstd decoder.
pub struct ZstdAdapter;

impl ZstdAdapter {
    pub fn decompress_bounded(input: &[u8], limits: &CodecLimits) -> KernelResult<Vec<u8>> {
        limits.validate()?;
        if input.is_empty() {
            return Err(KernelError::new(
                ErrorCategory::InvalidArgument,
                ErrorDetail::None,
            ));
        }

        let input_len = input.len() as u64;
        if input_len > limits.max_input_bytes {
            return Err(capacity_exceeded(limits.max_input_bytes, input_len));
        }

        // No frame parser: claimed size stand-in is the input length.
        // ratio == 1 forbids growth, so identity sitting on the output cap
        // is treated as would-expand (decompression grows).
        let claimed = if limits.max_expansion_ratio == 1 && input_len == limits.max_output_bytes {
            input_len.saturating_add(1)
        } else {
            input_len
        };

        if limits.expansion_would_exceed(input_len, claimed) {
            return Err(capacity_exceeded(output_limit(limits, input_len), claimed));
        }

        // Supplier blocked: never allocate decompressed output.
        Err(KernelError::new(
            ErrorCategory::CapabilityUnavailable,
            ErrorDetail::None,
        ))
    }
}

fn output_limit(limits: &CodecLimits, input_len: u64) -> u64 {
    match input_len.checked_mul(u64::from(limits.max_expansion_ratio)) {
        Some(cap) => limits.max_output_bytes.min(cap),
        None => limits.max_output_bytes,
    }
}

fn capacity_exceeded(limit: u64, requested: u64) -> KernelError {
    KernelError::new(
        ErrorCategory::CapacityExceeded,
        ErrorDetail::LimitExceeded { limit, requested },
    )
}
