//! T-handle-01: HandleKey orders by context, then slot, then generation.

use lumio_kernel::handle::{ContextKey, Generation, Handle, HandleKey, SlotIndex};

fn key(context: u64, slot: u32, generation: u32) -> HandleKey {
    HandleKey {
        context: ContextKey::new(context),
        slot: SlotIndex::new(slot),
        generation: Generation::new(generation),
    }
}

#[test]
fn handle_key_orders_stably() {
    let mut keys = vec![
        key(2, 0, 1),
        key(1, 2, 0),
        key(1, 0, 9),
        key(1, 0, 1),
        key(2, 0, 0),
        key(1, 1, 0),
    ];
    keys.sort();
    assert_eq!(
        keys,
        vec![
            key(1, 0, 1),
            key(1, 0, 9),
            key(1, 1, 0),
            key(1, 2, 0),
            key(2, 0, 0),
            key(2, 0, 1),
        ]
    );

    let k = key(7, 3, 11);
    assert_eq!(Handle::<u8>::from_key(k).key(), k);
}
