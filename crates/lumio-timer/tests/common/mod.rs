//! Shared Timer Manager fixtures for ABI cases.

#![allow(dead_code)]

use lumio_timer::{
    CallbackSlot, FiringRecord, ScopeKind, SlotDispatchId, TimerManager, TimerScope,
};

pub const TEST_DISPATCH: SlotDispatchId = SlotDispatchId::from_static("test.slot");
pub const TEST_DISPATCH_B: SlotDispatchId = SlotDispatchId::from_static("test.slot.b");

pub fn manager_at_tick(tick: u64) -> (TimerManager, TimerScope, CallbackSlot) {
    let mut manager = TimerManager::new(1);
    let scope = manager
        .register_scope(1, ScopeKind::World)
        .expect("register scope");
    let slot = manager.create_slot().expect("create slot");
    manager.bind_slot(slot, TEST_DISPATCH).expect("bind slot");
    if tick > 0 {
        let report = manager.advance(tick).expect("prime committed tick");
        assert!(report.firings().is_empty(), "prime advance must be empty");
    }
    (manager, scope, slot)
}

pub fn dues(records: &[FiringRecord]) -> Vec<u64> {
    records.iter().map(|r| r.due_tick).collect()
}

pub fn abi_text() -> String {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/architecture/wire/native-timer-abi-v1.json");
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}
