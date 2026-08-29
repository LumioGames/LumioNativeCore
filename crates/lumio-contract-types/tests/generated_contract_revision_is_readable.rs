mod common;

use lumio_contract_types::{
    AbiVersion, ArchitectureErrorCode, ArchitectureOperationId, CapabilityBits, LumioBuffer,
    LumioCoreConfigV1, LumioHandle, LumioStatus, StructSize, abi_version, architecture_baseline_id,
    root_abi_binding, verify_generated_contract_revision,
};

#[test]
fn generated_contract_revision_is_readable() {
    assert_eq!(
        architecture_baseline_id(),
        "LGE-V1.4-2026-08-27",
        "architecture baseline id must match the published architecture source id"
    );
    assert_eq!(
        verify_generated_contract_revision(),
        Ok(()),
        "generated adapter revision must match the published baseline"
    );

    for name in [
        core::any::type_name::<AbiVersion>(),
        core::any::type_name::<ArchitectureErrorCode>(),
        core::any::type_name::<ArchitectureOperationId>(),
        core::any::type_name::<CapabilityBits>(),
        core::any::type_name::<StructSize>(),
        core::any::type_name::<LumioStatus>(),
        core::any::type_name::<LumioHandle>(),
        core::any::type_name::<LumioBuffer>(),
        core::any::type_name::<LumioCoreConfigV1>(),
    ] {
        assert!(
            name.starts_with("lumio_contract_types::"),
            "controlled re-export leaked a non-crate type: {name}"
        );
    }
}

/// The bound identity constants must equal the published mirror, field by
/// field — the adapter binds, it never invents (ADR-040 §7).
#[test]
fn bound_identity_matches_published_bundle_mirror() {
    let bundle = common::parse_mirror("root-abi-bundle.json");
    let binding = root_abi_binding();

    assert_eq!(binding.baseline_id, bundle.get("baselineId").as_str());
    assert_eq!(binding.bundle_id, bundle.get("bundleId").as_str());
    let compiler = bundle.get("compiler");
    assert_eq!(binding.compiler_name, compiler.get("name").as_str());
    assert_eq!(binding.compiler_version, compiler.get("version").as_str());
    assert_eq!(binding.compiler_digest, compiler.get("digest").as_str());
    assert_eq!(binding.input_hash, bundle.get("inputHash").as_str());
    assert_eq!(
        binding.layout_profile_id,
        bundle.get("layoutProfile").get("targetProfileId").as_str()
    );

    let abi = bundle.get("abi");
    assert_eq!(
        i64::from(abi_version().raw()),
        abi.get("abiVersion").as_i64()
    );
    assert_eq!(binding.symbol_prefix, abi.get("symbolPrefix").as_str());
}

/// The recorded bundle digest must match what the package inventory
/// publishes for this bundle, and this repository must hold consumer
/// standing in `rootAbi.consumers` (ADR-040 §5/§7).
#[test]
fn bundle_digest_and_consumer_standing_match_packages_index_mirror() {
    let packages = common::parse_mirror("packages-index.json");
    let root_abi = packages.get("rootAbi");
    let binding = root_abi_binding();

    assert_eq!(binding.bundle_digest, root_abi.get("bundleDigest").as_str());
    assert_eq!(
        binding.compiler_digest,
        root_abi.get("compiler").get("digest").as_str()
    );
    assert_eq!(binding.input_hash, root_abi.get("inputHash").as_str());
    assert_eq!(
        binding.layout_profile_id,
        root_abi.get("layoutProfileId").as_str()
    );

    let consumers: Vec<&str> = root_abi
        .get("consumers")
        .as_arr()
        .iter()
        .map(|c| c.as_str())
        .collect();
    assert!(
        consumers.contains(&"LumioNativeCore"),
        "this repository must be a registered rootAbi consumer, got {consumers:?}"
    );

    let header = root_abi
        .get("outputFiles")
        .as_arr()
        .iter()
        .find(|f| f.get("role").as_str() == "CHeader")
        .expect("published CHeader output");
    assert_eq!(binding.header_digest, header.get("digest").as_str());
}

/// The `.baseline.sha256` pin for the mirrored bundle file must equal the
/// published `rootAbi.bundleDigest`: CI's `sha256sum -c` proves file bytes
/// match the pin, this test proves the pin matches the publication.
#[test]
fn baseline_pin_for_bundle_mirror_equals_published_digest() {
    let pin_path = common::mirror_path("../.baseline.sha256");
    let pin_body = std::fs::read_to_string(&pin_path)
        .unwrap_or_else(|e| panic!("read {}: {e}", pin_path.display()));
    let binding = root_abi_binding();

    let mut pinned = None;
    for line in pin_body.lines() {
        let mut parts = line.split_whitespace();
        if let (Some(hex), Some(rel)) = (parts.next(), parts.next())
            && rel == "docs/architecture/abi/root-abi-bundle.json"
        {
            pinned = Some(hex.to_ascii_lowercase());
        }
    }
    assert_eq!(
        pinned.as_deref(),
        Some(binding.bundle_digest),
        ".baseline.sha256 pin for root-abi-bundle.json must equal the published bundleDigest"
    );
}
