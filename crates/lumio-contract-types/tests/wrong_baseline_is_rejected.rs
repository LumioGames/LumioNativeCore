mod common;

use lumio_contract_types::{
    ContractMismatch, architecture_baseline_id, root_abi_binding,
    verify_generated_contract_revision, verify_generated_contract_revision_against,
    verify_root_abi_bundle_digest_against,
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

/// A bundle whose digest differs from the bound `rootAbi.bundleDigest` is a
/// drift, even under the same baseline id: the baseline names the contract
/// revision, the digest names the exact published bytes.
#[test]
fn wrong_bundle_digest_is_rejected() {
    let binding = root_abi_binding();
    // 上一个已发布的 bundle digest（compiler.digest 变更前），真实的历史漂移样本。
    let stale = "88321f1c3374c40ce2513d258df0f8c58661ef816ddc21811a5b0371bf3b309f";
    assert_ne!(stale, binding.bundle_digest);

    assert_eq!(
        verify_root_abi_bundle_digest_against(stale),
        Err(ContractMismatch {
            expected: binding.bundle_digest,
            found: stale,
        })
    );
    assert_eq!(
        verify_root_abi_bundle_digest_against(binding.bundle_digest),
        Ok(())
    );
}

/// Every mirrored index must agree on one baseline id, and it must be the
/// baseline this adapter binds — a mixed-revision mirror set is a drift.
#[test]
fn mirror_set_agrees_on_one_baseline() {
    let bound = architecture_baseline_id();
    for (file, key) in [
        ("root-abi-bundle.json", "baselineId"),
        ("packages-index.json", "baselineId"),
        ("ids-index.json", "baselineId"),
    ] {
        let mirror = common::parse_mirror(file);
        assert_eq!(
            mirror.get(key).as_str(),
            bound,
            "{file} baselineId must match the bound baseline"
        );
    }
}
