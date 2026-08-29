//! Architecture-source generated-contract adapter.
//!
//! Binds the published Root ABI bundle (`origin/main:packages/abi/`, recorded
//! by ADR-040 — Draft, but the artifacts themselves are published) for which
//! this repository holds `rootAbi.consumers` standing. The values below are
//! transcriptions of the byte-pinned mirror under `docs/architecture/abi/`
//! (pinned revision in its README, hashes in `.baseline.sha256`); the crate's
//! integration tests re-read the mirror and reject any drift, so nothing here
//! is invented. Per ADR-040 §7 this repository consumes the C Header plus the
//! four indices, never the Rust/C# generated packages.
//!
//! Still deliberately unbound (treated as absent, not inferred):
//! - `capability_bits` semantics and any bit position (D-015 pending);
//! - any layout profile other than `linux-x86_64-glibc` (D-016 pending);
//! - an `OperationId` namespace (does not exist; identity is the published
//!   (`apiTable[].name`, `slots[].slotIndex`) pair).

/// Published architecture baseline this crate binds to.
pub(crate) const ARCHITECTURE_BASELINE_ID: &str = "LGE-V1.4-2026-08-27";

/// Revision recorded by this adapter.
///
/// The bundle carries the baseline id as its revision anchor; the digest
/// chain (`RootAbiBinding`) carries the byte-level identity.
pub(crate) const GENERATED_CONTRACT_REVISION: &str = ARCHITECTURE_BASELINE_ID;

const ROOT_ABI_BUNDLE_ID: &str = "root-abi-v1";
const ROOT_ABI_BUNDLE_DIGEST: &str =
    "03ca75361fed3ca95f8efd55af2e311ea8300b2635b590ae6d46394d58bc6a39";
const ROOT_ABI_HEADER_DIGEST: &str =
    "040451bbde5a4dec3726be5f5a7be4bb934c3f68a1ca87f9c55559cae738efc7";
const ROOT_ABI_COMPILER_NAME: &str = "lumio-abi-compiler";
const ROOT_ABI_COMPILER_VERSION: &str = "1.0.0";
const ROOT_ABI_COMPILER_DIGEST: &str =
    "217437fd4755e1a339e2029838cc4a2d2fb305fa05520c8cfd10ea98cc2ff290";
const ROOT_ABI_INPUT_HASH: &str =
    "696a58d0525b897b549dd1e432166ae1020835902a5984221a8e60d5d8285bb3";
const ROOT_ABI_LAYOUT_PROFILE_ID: &str = "linux-x86_64-glibc";
const ROOT_ABI_SYMBOL_PREFIX: &str = "lumio_";
const ROOT_ABI_ABI_VERSION: u32 = 1;

/// Identity record of the bound Root ABI bundle (ADR-040 §7 verification
/// obligations: bundle digest, compiler identity, input hash, layout profile).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RootAbiBinding {
    pub baseline_id: &'static str,
    pub bundle_id: &'static str,
    pub bundle_digest: &'static str,
    pub header_digest: &'static str,
    pub compiler_name: &'static str,
    pub compiler_version: &'static str,
    pub compiler_digest: &'static str,
    pub input_hash: &'static str,
    pub layout_profile_id: &'static str,
    pub symbol_prefix: &'static str,
}

pub fn root_abi_binding() -> RootAbiBinding {
    RootAbiBinding {
        baseline_id: ARCHITECTURE_BASELINE_ID,
        bundle_id: ROOT_ABI_BUNDLE_ID,
        bundle_digest: ROOT_ABI_BUNDLE_DIGEST,
        header_digest: ROOT_ABI_HEADER_DIGEST,
        compiler_name: ROOT_ABI_COMPILER_NAME,
        compiler_version: ROOT_ABI_COMPILER_VERSION,
        compiler_digest: ROOT_ABI_COMPILER_DIGEST,
        input_hash: ROOT_ABI_INPUT_HASH,
        layout_profile_id: ROOT_ABI_LAYOUT_PROFILE_ID,
        symbol_prefix: ROOT_ABI_SYMBOL_PREFIX,
    }
}

/// ABI package version scalar (`abi.abiVersion` of the bundle).
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct AbiVersion(u32);

impl AbiVersion {
    pub const fn raw(self) -> u32 {
        self.0
    }
}

/// The published ABI version of the bound bundle.
pub fn abi_version() -> AbiVersion {
    AbiVersion(ROOT_ABI_ABI_VERSION)
}

/// One registered `ErrorCode` value. `ids/index.json` is the sole numeric
/// authority (ADR-040 §7); instances exist only in the generated registry
/// tables, so no caller can mint an unregistered numeric.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ArchitectureErrorCode {
    id: &'static str,
    numeric: i32,
}

impl ArchitectureErrorCode {
    /// Only the generated registry tables construct instances.
    pub(crate) const fn new(id: &'static str, numeric: i32) -> Self {
        Self { id, numeric }
    }

    /// Registered id string, e.g. `"InvalidHandle"`.
    pub const fn id(self) -> &'static str {
        self.id
    }

    /// Registered numeric; the value carried by `lumio_status_t` (ADR-040 §3).
    pub const fn numeric(self) -> i32 {
        self.numeric
    }
}

/// Architecture operation-id newtype. Permanently uninhabited: no
/// `OperationId` namespace exists or is reserved — the public identity of a
/// callable operation is (`apiTable[].name`, `slots[].slotIndex`) (ADR-040
/// §7, B-ABI-004 adjudicated not-applicable). Kept only because `lumio-job`'s
/// negative gate consumes the empty iterator.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ArchitectureOperationId {
    _private: (),
}

/// Capability-bit newtype. Uninhabited until D-015 lands: V1 freezes neither
/// mask-vs-count semantics nor any bit position, and the ID Registry
/// `Capability` numerics are CoreEngine package-capability enumeration
/// ordinals, not bit positions — deriving a key from either is forbidden
/// (ADR-040 §7).
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct CapabilityBits {
    _private: (),
}

/// Byte size of a generated type or struct, as published by the bundle
/// Golden. Constructed only from generated data or measured Rust layouts.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct StructSize(u32);

impl StructSize {
    pub(crate) const fn new(bytes: u32) -> Self {
        Self(bytes)
    }

    pub const fn bytes(self) -> u32 {
        self.0
    }
}

/// `lumio_status_t`: `int32_t` carrying a registered `ErrorCode` numeric;
/// `0` is success and no other value is reused (ADR-040 §3, ADR-046).
/// Constructible only as success or from a registered code, so an
/// unregistered non-zero status cannot originate in this workspace.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[repr(transparent)]
pub struct LumioStatus(i32);

impl LumioStatus {
    pub const SUCCESS: LumioStatus = LumioStatus(0);

    pub const fn from_error_code(code: ArchitectureErrorCode) -> Self {
        Self(code.numeric())
    }

    pub const fn raw(self) -> i32 {
        self.0
    }

    pub const fn is_success(self) -> bool {
        self.0 == 0
    }
}

/// `lumio_handle_t`: the Index+Generation+Context encoding of ADR-006
/// (16 bytes, align 8 on the published profile).
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[repr(C)]
pub struct LumioHandle {
    pub index: u32,
    pub generation: u32,
    pub context: u64,
}

/// `lumio_buffer_t`: the Ptr+Len+Capacity layout of ADR-017 (24 bytes,
/// align 8). `len`/`capacity` are fixed-width `u64`, never `usize`.
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct LumioBuffer {
    pub ptr: *mut core::ffi::c_void,
    pub len: u64,
    pub capacity: u64,
}

/// `struct lumio_core_config_v1`: caller-owned opaque payload. The body is
/// not part of the Root ABI at this granularity and stays guarded by its own
/// leading `struct_size` (ADR-040 §3); it crosses the boundary by pointer
/// only, so this type is deliberately not constructible.
#[repr(C)]
pub struct LumioCoreConfigV1 {
    _private: [u8; 0],
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

/// Drift gate for the bundle bytes: an observed bundle digest that differs
/// from the bound `rootAbi.bundleDigest` is a contract drift, not a warning.
/// (CI's `sha256sum -c` proves the mirror file still hashes to the pin; the
/// crate tests prove pin == published digest == this constant.)
pub fn verify_root_abi_bundle_digest_against(found: &'static str) -> Result<(), ContractMismatch> {
    let expected = ROOT_ABI_BUNDLE_DIGEST;
    if found == expected {
        Ok(())
    } else {
        Err(ContractMismatch { expected, found })
    }
}
