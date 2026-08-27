//! `lumio-codec`：纯字节压缩 / 校验 / diff Kernel。
//!
//! BaselineStatus = pending（ADR 0005）：仅限私有原型，不进公共 Header/export list；
//! schema 语义判定归上游生成 Serializer，本 crate 不做。
//! 默认构建只导出有界 `CodecLimits` 与私有 8 字节校验；vendor adapter 不得进入 default 依赖图。

#![forbid(unsafe_code)]

mod bounds;
mod checksum;

pub use bounds::CodecLimits;
pub use checksum::checksum_bytes;
