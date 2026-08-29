//! `lumio-contract-types`：零依赖类型叶子。
//!
//! 只承载架构源生成的固定宽度 POD、Buffer view、版本标量、opaque handle 表示与
//! 错误码/能力位常量；不含任何行为逻辑。边界与依赖图见
//! `docs/specs/native-core-module-map.md`。
//!
//! 架构源已发布 baseline id `LGE-V1.4-2026-08-27` 与 ADR-040 Root ABI bundle；
//! 本仓登记为该 bundle 的 consumer，按 ADR-040 §7 直接绑定其 C Header 与四个
//! 索引（字节级镜像见 `docs/architecture/abi/`），**不**消费 Rust/C# 生成包。
//! ErrorCode 与 Capability **键**的数值权威只有 `ids/index.json`（分别含
//! ADR-046 kernel band 与 D-015 裁决的键空间，ADR-040 §7.1）；
//! Capability bit 语义（D-015 只裁键空间，掩码/计数与 bit 位仍未冻结）、
//! 非 `linux-x86_64-glibc` 布局档（D-016）与 OperationId（不存在，
//! B-ABI-004 不适用）保持不绑定，一律不得手写。

#![forbid(unsafe_code)]

mod generated;
mod generated_data;
pub mod layout;
pub mod registry;

pub use generated::{
    AbiVersion, ArchitectureCapabilityKey, ArchitectureErrorCode, ArchitectureOperationId,
    CapabilityBits, ContractMismatch, LumioBuffer, LumioCoreConfigV1, LumioHandle, LumioStatus,
    RootAbiBinding, StructSize, abi_version, architecture_baseline_id, root_abi_binding,
    verify_generated_contract_revision, verify_generated_contract_revision_against,
    verify_root_abi_bundle_digest_against,
};
