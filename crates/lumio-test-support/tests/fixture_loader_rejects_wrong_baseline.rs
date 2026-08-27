//! T-test-support-04 / R-00185: FixtureLoader rejects a stale architecture baseline.

use lumio_contract_types::ContractMismatch;
use lumio_test_support::FixtureLoader;

#[test]
fn fixture_loader_rejects_wrong_baseline() {
    let loader = FixtureLoader::new();
    let current = loader.current_baseline();
    let stale = "LGE-V1.2-2026-08-27";

    assert_eq!(current, "LGE-V1.4-2026-08-27");
    assert_ne!(stale, current);

    assert_eq!(
        loader.load_baseline(stale),
        Err(ContractMismatch {
            expected: current,
            found: stale,
        })
    );
    assert_eq!(loader.load_baseline(current), Ok(()));
}
