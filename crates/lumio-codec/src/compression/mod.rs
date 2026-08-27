//! Compression adapter seams. Vendor crates stay behind these files.

mod lz4_adapter;
mod zstd_adapter;

pub use lz4_adapter::Lz4Adapter;
pub use zstd_adapter::ZstdAdapter;
