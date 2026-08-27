//! `lumio-native-ffi`：NativeCore 唯一符号导出面（原 abi 模块的门面半边，ADR 0001）。
//!
//! 只提供 provider API Table 组装与 panic 边界；跨仓 Root 符号
//! （`lumio_core_get_api_v1`）归 CoreEngine root-abi 拥有，本 crate 永不导出——
//! `cargo xtask dump-symbols` 强制断言。pending 模块（codec/diagnostics）
//! 只能经 experimental feature 进入本地实验构建，默认发布面不含。
//! 当前为脚手架，C 导出面为空。

mod boundary;
pub use boundary::ffi_boundary;

mod validation;
pub use validation::check_buffer_ptr_len;
