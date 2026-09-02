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

mod handles;
pub use handles::decode_handle_for_context;

mod exports;
pub use exports::{LumioCoreApi, provider_core_api_table, smoke_decode_handle};

mod timer;
pub use timer::{
    LumioEngineRootApiV1, NATIVE_ABI_DEFINITION_SHA256, TimerDrainRecord, TimerHandleAbi,
    map_timer_error, provider_engine_root_api,
};

mod symbol_guard;
pub use symbol_guard::{
    crate_sources_contain_root_symbol, forbidden_root_symbol_name, mirror_entry_symbol,
};
