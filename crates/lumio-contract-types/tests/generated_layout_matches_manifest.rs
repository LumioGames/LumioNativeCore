mod common;

use lumio_contract_types::layout;

/// The generated golden rows must equal the published bundle mirror field by
/// field — the data is derived, never invented, and any drift fails here.
#[test]
fn generated_layout_matches_manifest() {
    layout::verify_layout().expect("bound Rust layouts must match the published Golden");

    let bundle = common::parse_mirror("root-abi-bundle.json");
    let profile = bundle.get("layoutProfile");
    assert_eq!(
        i64::from(layout::pointer_bytes()),
        profile.get("pointerBytes").as_i64()
    );
    assert_eq!(
        i64::from(layout::max_alignment()),
        profile.get("maxAlignment").as_i64()
    );
    assert_eq!(
        i64::from(layout::pointer_bytes()) * 8,
        bundle.get("abi").get("pointerWidth").as_i64()
    );

    // typeMapping：镜像中每个 lumio_ 前缀命名 C 类型都必须出现在 golden 中，
    // 且 size/align 一致；golden 不得多出镜像没有的行。
    let mut mirror_named: Vec<(&str, i64, i64)> = Vec::new();
    for row in bundle.get("typeMapping").as_arr() {
        let c_name = row.get("c").as_str();
        if !c_name.starts_with("lumio_") || c_name.contains('*') {
            continue;
        }
        let size = row.get("size").as_i64();
        let align = row.get("align").as_i64();
        if let Some(existing) = mirror_named.iter().find(|(n, _, _)| *n == c_name) {
            assert_eq!(
                (existing.1, existing.2),
                (size, align),
                "mirror typeMapping rows disagree for {c_name}"
            );
        } else {
            mirror_named.push((c_name, size, align));
        }
    }
    let golden_types = layout::type_entries();
    assert_eq!(golden_types.len(), mirror_named.len());
    for (name, size, align) in &mirror_named {
        let g = golden_types
            .iter()
            .find(|t| t.name == *name)
            .unwrap_or_else(|| panic!("golden missing type {name}"));
        assert_eq!(i64::from(g.size), *size, "size mismatch for {name}");
        assert_eq!(i64::from(g.align), *align, "align mismatch for {name}");
    }

    // root + tables：declared/minimum 尺寸与每个成员偏移逐项一致。
    let golden_structs = layout::struct_entries();
    let root = bundle.get("root");
    check_struct(
        golden_structs,
        "lumio_root_api",
        root,
        &["fields", "tables"],
    );
    let tables = bundle.get("tables").as_arr();
    assert_eq!(
        golden_structs.len(),
        tables.len() + 1,
        "golden must carry the root plus every published table"
    );
    for table in tables {
        check_struct(
            golden_structs,
            table.get("name").as_str(),
            table,
            &["fields", "slots"],
        );
    }

    // entries()：POD 行在前、struct 行在后，尺寸取自 golden。
    let entries = layout::entries();
    assert_eq!(entries.len(), golden_types.len() + golden_structs.len());
    for t in golden_types {
        assert!(
            entries
                .iter()
                .any(|(name, size)| *name == t.name && size.bytes() == t.size)
        );
    }
    for s in golden_structs {
        assert!(
            entries
                .iter()
                .any(|(name, size)| *name == s.name && size.bytes() == s.declared_size)
        );
    }
}

fn check_struct(
    golden: &[layout::AbiStructGolden],
    name: &str,
    mirror: &common::Json,
    member_keys: &[&str],
) {
    let g = golden
        .iter()
        .find(|s| s.name == name)
        .unwrap_or_else(|| panic!("golden missing struct {name}"));
    assert_eq!(
        i64::from(g.declared_size),
        mirror.get("declaredStructSize").as_i64(),
        "declared size mismatch for {name}"
    );
    assert_eq!(
        i64::from(g.minimum_size),
        mirror.get("minimumStructSize").as_i64(),
        "minimum size mismatch for {name}"
    );
    let mut expected: Vec<(String, i64)> = Vec::new();
    for key in member_keys {
        for row in mirror.get(key).as_arr() {
            expected.push((
                row.get("name").as_str().to_string(),
                row.get("offset").as_i64(),
            ));
        }
    }
    let actual: Vec<(String, i64)> = g
        .members
        .iter()
        .map(|(n, off)| ((*n).to_string(), i64::from(*off)))
        .collect();
    assert_eq!(actual, expected, "member offsets mismatch for {name}");
}

/// Golden comparisons of the bound Rust types run only on the certified
/// profile; other targets publish no Golden to compare against (ADR-040 §7).
#[cfg(all(target_arch = "x86_64", target_os = "linux", target_env = "gnu"))]
#[test]
fn bound_rust_types_match_golden_on_certified_profile() {
    use lumio_contract_types::{LumioBuffer, LumioHandle, LumioStatus};

    let sizes: &[(&str, usize, usize)] = &[
        (
            "lumio_status_t",
            size_of::<LumioStatus>(),
            align_of::<LumioStatus>(),
        ),
        (
            "lumio_handle_t",
            size_of::<LumioHandle>(),
            align_of::<LumioHandle>(),
        ),
        (
            "lumio_buffer_t",
            size_of::<LumioBuffer>(),
            align_of::<LumioBuffer>(),
        ),
    ];
    for &(name, size, align) in sizes {
        let g = layout::type_entries()
            .iter()
            .find(|t| t.name == name)
            .unwrap_or_else(|| panic!("golden missing {name}"));
        assert_eq!(size as u32, g.size, "size mismatch for {name}");
        assert_eq!(align as u32, g.align, "align mismatch for {name}");
    }
    assert_eq!(size_of::<usize>() as u32, layout::pointer_bytes());
}
