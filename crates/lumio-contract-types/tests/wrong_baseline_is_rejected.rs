use lumio_contract_types::{
    ContractMismatch, architecture_baseline_id, verify_generated_contract_revision,
    verify_generated_contract_revision_against,
};

#[test]
fn wrong_baseline_is_rejected() {
    let current = architecture_baseline_id();
    let stale = "LGE-V1.2-2026-08-27";
    assert_eq!(current, "LGE-V1.4-2026-08-27");
    assert_ne!(stale, current);

    assert_eq!(
        verify_generated_contract_revision_against(stale),
        Err(ContractMismatch {
            expected: current,
            found: stale,
        })
    );
    assert_eq!(verify_generated_contract_revision(), Ok(()));
    assert_eq!(verify_generated_contract_revision_against(current), Ok(()));
}
