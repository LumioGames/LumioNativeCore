use lumio_contract_types::layout;

#[test]
fn generated_layout_matches_manifest() {
    layout::verify_layout()
        .expect("no generated Header means no structs to check, not invented ABI sizes");
    assert_eq!(
        layout::entries().len(),
        0,
        "must not invent ABI struct sizes while the generated Header is unpublished"
    );
}
