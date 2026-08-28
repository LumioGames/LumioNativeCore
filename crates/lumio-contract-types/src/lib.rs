//! `lumio-contract-types`：零依赖类型叶子。
//!
//! 只承载架构源生成的固定宽度 POD、Buffer view、版本标量、opaque handle 表示与
//! 错误码/能力位常量；不含任何行为逻辑。边界与依赖图见
//! `docs/specs/native-core-module-map.md`。
//!
//! Gate-0 只提供内部 seam 与负向 Gate。架构源已发布 baseline id
//! `LGE-V1.4-2026-08-27`，并按 ADR-040 发布了 Root ABI bundle；本仓已登记为该
//! bundle 的 consumer，直接绑定其 C Header，**不**消费 Rust/C# 生成包。绑定本身
//! 尚未落地：ErrorCode / Capability / Operation 数值对本仓的需求仍未发布，
//! 一律不得手写，也不得声称公共 ABI 已完成。

#![forbid(unsafe_code)]

mod generated;
pub mod layout;
pub mod registry;

pub use generated::{
    AbiVersion, ArchitectureErrorCode, ArchitectureOperationId, CapabilityBits, ContractMismatch,
    StructSize, architecture_baseline_id, verify_generated_contract_revision,
    verify_generated_contract_revision_against,
};
