//! Consume origin/main `2b7e321` native-timer-abi-v1.json as the semantic source.

mod common;

const TEST_CASES: [&str; 4] = [
    "one_shot_fires_exactly_once",
    "repeating_fires_each_interval",
    "cancel_prevents_delivery",
    "delivery_order_stable_and_replayable",
];

const INVALID_CASES: [&str; 18] = [
    "stale_handle_never_fuzzy_matches",
    "scope_reset_invalidates_all_handles",
    "scope_generation_mismatch_at_schedule",
    "late_completion_terminal_no_state_write",
    "slot_failure_explicit_not_silent",
    "slot_unbound_rejected_at_schedule",
    "slot_closed_rejected_at_schedule",
    "slot_queue_full_timer_terminal_process_lives",
    "slot_dispatch_mismatch_at_drain",
    "scope_invalid_unregistered_scope",
    "manager_shutdown_rejects_all_operations",
    "double_cancel_stable_error",
    "invalid_interval_rejected",
    "invalid_due_tick_at_schedule",
    "advance_rollback_rejected",
    "schedule_budget_exceeded_no_partial_schedule",
    "max_schedules_per_tick_exceeded",
    "generation_overflow_is_fatal",
];

#[test]
fn contract_mirror_is_frozen_c4_revision() {
    let text = common::abi_text();
    assert_eq!(text.len(), 36021);
    assert!(text.starts_with("{\n  \"contractId\": \"lumio.native-timer-abi.v1\""));
    assert!(!text.contains('\r'), "mirrored ABI must be LF-only");
    assert_eq!(lumio_timer::CONTRACT_ID, "lumio.native-timer-abi.v1");
    assert_eq!(lumio_timer::CONTRACT_REVISION, "2b7e321");
    assert!(text.contains("(committedTick, toTick]"));
    assert!(text.contains("dueTick == toTick 必须开火"));
    assert!(text.contains("恰在 12、17 与 22 三刻各触发一次"));
    assert!(text.contains("\"kind\": \"in-process-api-contract\""));
    assert!(text.contains("Client Timer Manager"));
    assert!(text.contains("Server Timer Manager"));
    assert!(text.contains("hostTimerService"));
    for name in TEST_CASES {
        assert!(
            text.contains(&format!("\"name\": \"{name}\"")),
            "missing testCase {name}"
        );
    }
    for name in INVALID_CASES {
        assert!(
            text.contains(&format!("\"name\": \"{name}\"")),
            "missing invalidCase {name}"
        );
    }
    assert_eq!(TEST_CASES.len(), 4);
    assert_eq!(INVALID_CASES.len(), 18);
}
