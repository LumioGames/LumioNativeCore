mod common;

use lumio_contract_types::registry;
use lumio_contract_types::{ArchitectureCapabilityKey, CapabilityBits};

/// SCREAMING_SNAKE projection of a registered id, matching the published
/// header macro spelling (`HybridCLR` -> `HYBRID_CLR`).
fn screaming(id: &str) -> String {
    let chars: Vec<char> = id.chars().collect();
    let mut out = String::new();
    for (i, c) in chars.iter().enumerate() {
        if i > 0
            && c.is_ascii_uppercase()
            && (!chars[i - 1].is_ascii_uppercase()
                || chars.get(i + 1).is_some_and(char::is_ascii_lowercase))
        {
            out.push('_');
        }
        out.push(c.to_ascii_uppercase());
    }
    out
}

fn header_defines() -> Vec<(String, u32)> {
    let path = common::mirror_path("lumio_core.h");
    let body = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("read mirror {}: {e}", path.display()));
    let mut out = Vec::new();
    for line in body.lines() {
        let Some(rest) = line.strip_prefix("#define LUMIO_CAPABILITY_") else {
            continue;
        };
        let mut parts = rest.split_whitespace();
        let (Some(name), Some(value)) = (parts.next(), parts.next()) else {
            continue;
        };
        let numeric: u32 = value
            .trim_end_matches('u')
            .parse()
            .unwrap_or_else(|e| panic!("bad capability define `{line}`: {e}"));
        out.push((name.to_string(), numeric));
    }
    out
}

/// D-015 (ADR-040 §7.1): `ids/index.json` is the sole authority for the
/// capability key space and the architecture generator is its sole emitter.
/// The bound table must agree with the registry mirror AND with the header
/// projection, value for value — a repository-private key table is a
/// violation, so nothing here may be hand-written.
#[test]
fn capability_keys_bind_registry() {
    let ids = common::parse_mirror("ids-index.json");
    let capability_ns = ids
        .get("namespaces")
        .as_arr()
        .iter()
        .find(|ns| ns.get("namespace").as_str() == "Capability")
        .expect("mirror Capability namespace");
    assert_eq!(capability_ns.get("owner").as_str(), "Architecture");

    let mirror: Vec<(&str, i64, &str)> = capability_ns
        .get("values")
        .as_arr()
        .iter()
        .map(|v| {
            let status = v.get("status").as_str();
            assert!(
                status == "Active" || status == "Reserved",
                "unexpected Capability status {status}"
            );
            (v.get("id").as_str(), v.get("numeric").as_i64(), status)
        })
        .collect();

    let bound: Vec<ArchitectureCapabilityKey> = registry::capability_keys().collect();
    assert_eq!(
        bound.len(),
        mirror.len(),
        "bound table must carry every published Capability value"
    );
    for (key, (mirror_id, mirror_numeric, mirror_status)) in bound.iter().zip(&mirror) {
        assert_eq!(key.id(), *mirror_id);
        assert_eq!(i64::from(key.numeric()), *mirror_numeric);
        assert_eq!(key.status(), *mirror_status);
        assert!(
            key.numeric() > 0,
            "capability numerics are 1-based ordinals"
        );
    }

    for (i, a) in bound.iter().enumerate() {
        for b in &bound[i + 1..] {
            assert_ne!(a.id(), b.id(), "duplicate Capability id");
            assert_ne!(a.numeric(), b.numeric(), "duplicate Capability numeric");
        }
    }

    for key in &bound {
        assert_eq!(registry::capability_key(key.id()), Some(*key));
    }
    assert_eq!(registry::capability_key("NotARegisteredCapability"), None);
}

/// The published C header is a projection of the same authority; the bound
/// table must agree with it name for name, count included. Regenerating one
/// side without the other is drift, not a warning.
#[test]
fn capability_keys_match_header_projection() {
    let defines = header_defines();
    let bound: Vec<ArchitectureCapabilityKey> = registry::capability_keys().collect();

    let count = defines
        .iter()
        .find(|(name, _)| name == "COUNT")
        .map(|(_, v)| *v)
        .expect("header publishes LUMIO_CAPABILITY_COUNT");
    assert_eq!(
        count as usize,
        bound.len(),
        "LUMIO_CAPABILITY_COUNT must equal the bound key count"
    );

    for key in &bound {
        let expected = screaming(key.id());
        let (_, numeric) = defines
            .iter()
            .find(|(name, _)| *name == expected)
            .unwrap_or_else(|| panic!("header missing LUMIO_CAPABILITY_{expected}"));
        assert_eq!(
            *numeric,
            key.numeric(),
            "header LUMIO_CAPABILITY_{expected} disagrees with the registry"
        );
    }

    // Every define that is a key must be backed by a registered value: the
    // header may not carry a capability the registry does not publish.
    // `COUNT` is the cardinality and `BITS` is the `capability_bits` scalar —
    // neither is a key, and `BITS` in particular stays outside the key space
    // because D-015 left mask-vs-count and bit assignment unfrozen.
    for (name, _) in defines.iter().filter(|(n, _)| n != "COUNT" && n != "BITS") {
        assert!(
            bound.iter().any(|k| screaming(k.id()) == *name),
            "header publishes LUMIO_CAPABILITY_{name} with no registered key"
        );
    }
    assert!(
        defines.iter().any(|(name, _)| name == "BITS"),
        "LUMIO_CAPABILITY_BITS must stay published as a non-key scalar"
    );
    assert!(
        !bound.iter().any(|k| screaming(k.id()) == "BITS"),
        "LUMIO_CAPABILITY_BITS must never be bound as a capability key"
    );
}

/// D-015 adjudicated the key space only. Mask-vs-count semantics and any bit
/// assignment stay unfrozen, so the bit surface must remain empty: reading a
/// key is allowed, deriving a bit is not.
#[test]
fn capability_bits_stay_unbound() {
    let bits: Vec<CapabilityBits> = registry::capability_bits().collect();
    assert_eq!(bits.len(), 0);
}
