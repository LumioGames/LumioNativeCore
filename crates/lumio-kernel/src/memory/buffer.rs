//! Three call-scoped buffer classes. Native payload ownership is out of scope.

use crate::error::{KernelError, KernelResult};
use crate::handle::Handle;

/// Caller-owned input bytes valid only for the current synchronous call.
pub struct BorrowedCallBuffer<'a>(&'a [u8]);

/// Caller-owned output bytes written only for the current synchronous call.
pub struct CallerOutputBuffer<'a> {
    bytes: &'a mut [u8],
    written: usize,
}

/// Type tag distinguishing native-owned buffer handles from other `Handle<T>` values.
pub enum NativeBufferTag {}

/// Native-owned buffer identity. Released only by the creating allocator.
pub struct NativeOwnedBufferHandle(#[allow(dead_code)] Handle<NativeBufferTag>);

impl<'a> BorrowedCallBuffer<'a> {
    pub fn new(bytes: &'a [u8]) -> Self {
        Self(bytes)
    }

    pub fn as_slice(&self) -> &'a [u8] {
        self.0
    }
}

impl<'a> CallerOutputBuffer<'a> {
    pub fn new(bytes: &'a mut [u8]) -> Self {
        Self { bytes, written: 0 }
    }

    pub fn capacity(&self) -> usize {
        self.bytes.len()
    }

    pub fn written(&self) -> usize {
        self.written
    }

    /// All-or-nothing: on overflow, `written` and dest bytes stay unchanged.
    pub fn write_all(&mut self, src: &[u8]) -> KernelResult<()> {
        let required = (self.written as u64).saturating_add(src.len() as u64);
        let provided = self.bytes.len() as u64;
        if required > provided {
            return Err(KernelError::buffer_too_small(required, provided));
        }
        let start = self.written;
        let end = start + src.len();
        self.bytes[start..end].copy_from_slice(src);
        self.written = end;
        Ok(())
    }

    pub fn finish(self) -> &'a mut [u8] {
        let CallerOutputBuffer { bytes, written } = self;
        &mut bytes[..written]
    }
}
