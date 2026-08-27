//! Zip-bomb I/O bounds. No vendor types or public algorithm IDs.

use lumio_kernel::error::{ErrorCategory, ErrorDetail, KernelError, KernelResult};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CodecLimits {
    pub max_input_bytes: u64,
    pub max_output_bytes: u64,
    pub max_expansion_ratio: u32,
}

impl CodecLimits {
    pub fn validate(&self) -> KernelResult<()> {
        if self.max_input_bytes == 0 || self.max_output_bytes == 0 || self.max_expansion_ratio == 0
        {
            return Err(KernelError::new(
                ErrorCategory::InvalidArgument,
                ErrorDetail::None,
            ));
        }
        Ok(())
    }

    pub fn expansion_would_exceed(&self, input_len: u64, output_len: u64) -> bool {
        if output_len > self.max_output_bytes {
            return true;
        }
        match input_len.checked_mul(u64::from(self.max_expansion_ratio)) {
            Some(cap) => output_len > cap,
            // Product exceeds u64: the ratio cap is above any representable output.
            None => false,
        }
    }
}
