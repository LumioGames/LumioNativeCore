use lumio_contract_types::{
    AbiVersion, ArchitectureErrorCode, ArchitectureOperationId, CapabilityBits, StructSize,
    architecture_baseline_id, verify_generated_contract_revision,
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
    ] {
        assert!(
            name.starts_with("lumio_contract_types::"),
            "controlled re-export leaked a non-crate type: {name}"
        );
    }
}
