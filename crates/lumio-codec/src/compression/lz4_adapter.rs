//! LZ4 bounded-decompress adapter seam.
//!
//! `lz4` / `lz4_flex` is not approved (`EXTERNAL_ALLOWLIST` is empty). This is the
//! blocked-supplier fallback, not a published lz4 binding. Truncated frames are
//! rejected before any vendor decode. When lz4 is approved, only this file should
//! change. Public API does not mention lz4 crate types.

use lumio_kernel::error::{ErrorCategory, ErrorDetail, KernelError, KernelResult};

use crate::bounds::CodecLimits;

/// LZ4 frame magic is 4 bytes; shorter input cannot be a complete frame.
const LZ4_FRAME_HEADER_LEN: usize = 4;

pub struct Lz4Adapter;

impl Lz4Adapter {
    pub fn decompress_bounded(input: &[u8], limits: &CodecLimits) -> KernelResult<Vec<u8>> {
        limits.validate()?;
        if input.len() < LZ4_FRAME_HEADER_LEN {
            return Err(KernelError::new(
                ErrorCategory::InvalidArgument,
                ErrorDetail::None,
            ));
        }
        Err(KernelError::new(
            ErrorCategory::CapabilityUnavailable,
            ErrorDetail::None,
        ))
    }
}
