use lumio_contract_types::layout;

#[test]
fn generated_layout_matches_manifest() {
    layout::verify_layout().expect("an empty layout table has no structs to check");
    assert_eq!(
        layout::entries().len(),
        0,
        "must not assert ABI struct sizes beyond the one layout profile the \
         architecture bundle certifies (linux-x86_64-glibc)"
    );
}
