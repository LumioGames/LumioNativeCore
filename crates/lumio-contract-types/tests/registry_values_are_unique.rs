use lumio_contract_types::registry;
use lumio_contract_types::{ArchitectureErrorCode, ArchitectureOperationId, CapabilityBits};
use std::collections::HashSet;
use std::fs;
use std::path::Path;

fn assert_unique<T: Eq + std::hash::Hash + std::fmt::Debug>(items: &[T]) {
    let mut seen = HashSet::new();
    for item in items {
        assert!(seen.insert(item), "registry value is not unique: {item:?}");
    }
}

fn is_public_numeric_const(line: &str) -> bool {
    let trimmed = line.trim();
    if !trimmed.starts_with("pub const ") {
        return false;
    }
    trimmed.contains(": u")
        || trimmed.contains(": i")
        || trimmed.contains(": usize")
        || trimmed.contains(": isize")
}

#[test]
fn registry_values_are_unique() {
    let error_codes: Vec<ArchitectureErrorCode> = registry::error_codes().collect();
    let operation_ids: Vec<ArchitectureOperationId> = registry::operation_ids().collect();
    let capability_bits: Vec<CapabilityBits> = registry::capability_bits().collect();

    // Generated Error/Capability/Operation package is unpublished: empty is unique.
    assert_eq!(error_codes.len(), 0);
    assert_eq!(operation_ids.len(), 0);
    assert_eq!(capability_bits.len(), 0);

    assert_unique(&error_codes);
    assert_unique(&operation_ids);
    assert_unique(&capability_bits);

    let src_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    for entry in fs::read_dir(&src_dir).expect("crate src") {
        let path = entry.expect("src entry").path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("rs") {
            continue;
        }
        let source = fs::read_to_string(&path).expect("read crate source");
        for line in source.lines() {
            assert!(
                !is_public_numeric_const(line),
                "{} leaks a public numeric constant while the generated package is blocked: {line}",
                path.display()
            );
        }
    }
}
