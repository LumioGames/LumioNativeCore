//! Architecture-source generated-contract adapter.
//!
//! The architecture source has published baseline id `LGE-V1.4-2026-08-27` and,
//! under ADR-040, the Root ABI bundle at `origin/main:packages/abi/` — this
//! repository is a registered consumer of that bundle (`rootAbi.consumers`) and
//! binds its C Header directly. It is deliberately NOT a consumer of the Rust /
//! C# generated packages.
//!
//! Binding is not done yet, so this module is still the internal seam only:
//! opaque newtypes, no public numeric registries, no copied schemas. What the
//! bundle publishes (handle / buffer / status layout, ABI version) is bindable;
//! ErrorCode, Capability bits and Operation ids are still unpublished for this
//! repository's needs. See `layout.rs` for the layout-profile caveat.

/// Published architecture baseline this crate binds to.
pub(crate) const ARCHITECTURE_BASELINE_ID: &str = "LGE-V1.4-2026-08-27";

/// Revision recorded by this adapter.
///
/// Until a generated package exists, the seam records the published baseline id
/// rather than inventing a second schema.
pub(crate) const GENERATED_CONTRACT_REVISION: &str = ARCHITECTURE_BASELINE_ID;

/// ABI package version scalar. Width and layout remain blocked.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct AbiVersion {
    _private: (),
}

/// Architecture error-code newtype. No public numeric constants.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ArchitectureErrorCode {
    _private: (),
}

/// Architecture operation-id newtype. No public numeric constants.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ArchitectureOperationId {
    _private: (),
}

/// Capability-bit newtype. No public numeric constants.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct CapabilityBits {
    _private: (),
}

/// Generated struct size token. No public ABI sizes while the Header is blocked.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct StructSize {
    _private: (),
}

/// Generated revision does not match the expected architecture baseline.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContractMismatch {
    pub expected: &'static str,
    pub found: &'static str,
}

pub fn architecture_baseline_id() -> &'static str {
    ARCHITECTURE_BASELINE_ID
}

pub fn verify_generated_contract_revision() -> Result<(), ContractMismatch> {
    verify_generated_contract_revision_against(GENERATED_CONTRACT_REVISION)
}

/// `found` is `'static` so `ContractMismatch::found` can name the observed id.
pub fn verify_generated_contract_revision_against(
    found: &'static str,
) -> Result<(), ContractMismatch> {
    let expected = ARCHITECTURE_BASELINE_ID;
    if found == expected {
        Ok(())
    } else {
        Err(ContractMismatch { expected, found })
    }
}
