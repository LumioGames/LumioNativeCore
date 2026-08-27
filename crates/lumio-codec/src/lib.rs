//! `lumio-codec`：纯字节压缩 / 校验 / diff Kernel。
//!
//! BaselineStatus = pending（ADR 0005）：仅限私有原型，不进公共 Header/export list；
//! schema 语义判定归上游生成 Serializer，本 crate 不做。
//! 当前为脚手架，公共 API 面为空。

#![forbid(unsafe_code)]
