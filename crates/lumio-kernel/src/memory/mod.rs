//! Call-scoped buffers and native-owned buffer handles.

mod buffer;

pub use buffer::{
    BorrowedCallBuffer, CallerOutputBuffer, NativeBufferTag, NativeOwnedBufferHandle,
};
