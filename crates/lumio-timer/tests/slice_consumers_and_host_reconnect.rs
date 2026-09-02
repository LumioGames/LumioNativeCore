//! Slice consumers: Bot cadence and server periodic on tickFrame adapters;
//! five-minute reconnect on the same kernel in wallClock mode.

use lumio_timer::{
    BOT_CHAT_CADENCE_TICKS, ClientTimerManager, RECONNECT_RETENTION_DISPATCH,
    RECONNECT_RETENTION_MS, SERVER_WORLD_HEARTBEAT_TICKS, ScopeKind, ServerTimerManager,
    SliceTraceEvent, TimerManager, TimerMode,
};

#[test]
fn bot_chat_cadence_runs_on_client_timer_manager() {
    assert_eq!(BOT_CHAT_CADENCE_TICKS, 5);
    let mut client = ClientTimerManager::new(1);
    let scope = client
        .register_scope(ScopeKind::Adapter, 7)
        .expect("client scope");
    let handle = client
        .schedule_bot_chat_cadence(scope)
        .expect("bot cadence");
    let _ = handle;
    client.pump(20).expect("pump client");
    assert_eq!(client.trace().bot_utterance_ticks(), [5, 10, 15, 20]);
    assert!(
        client
            .trace()
            .events()
            .iter()
            .any(|e| matches!(e, SliceTraceEvent::BotUtteranceSubmit { due_tick: 5 }))
    );
}

#[test]
fn server_periodic_task_runs_on_server_timer_manager() {
    assert_eq!(SERVER_WORLD_HEARTBEAT_TICKS, 10);
    let mut server = ServerTimerManager::new(2);
    let scope = server
        .register_scope(ScopeKind::World, 1)
        .expect("server scope");
    server
        .schedule_world_heartbeat(scope)
        .expect("world heartbeat");
    server.pump(20).expect("pump server");
    assert_eq!(server.trace().server_checkpoint_ticks(), [10, 20]);
    assert!(server.trace().events().iter().any(|e| matches!(
        e,
        SliceTraceEvent::ServerPeriodicCheckpoint {
            due_tick: 10,
            live_timers: 1
        }
    )));
}

#[test]
fn reconnect_deadline_runs_on_kernel_wall_clock() {
    let mut wall = TimerManager::with_mode(3, TimerMode::WallClock);
    let mut client = ClientTimerManager::new(1);
    let mut server = ServerTimerManager::new(2);

    let client_scope = client
        .register_scope(ScopeKind::Adapter, 1)
        .expect("client scope");
    client
        .schedule_bot_chat_cadence(client_scope)
        .expect("bot cadence");
    let server_scope = server
        .register_scope(ScopeKind::World, 1)
        .expect("server scope");
    server
        .schedule_world_heartbeat(server_scope)
        .expect("heartbeat");

    let wall_scope = wall
        .register_scope(1, ScopeKind::Session)
        .expect("wall scope");
    let wall_slot = wall.create_slot().expect("wall slot");
    wall.bind_slot(wall_slot, RECONNECT_RETENTION_DISPATCH)
        .expect("bind reconnect");
    let handle = wall
        .schedule_one_shot(wall_scope, RECONNECT_RETENTION_MS, wall_slot)
        .expect("reconnect window");

    client.pump(20).expect("native client ticks");
    server.pump(20).expect("native server ticks");
    let early = wall
        .pump(RECONNECT_RETENTION_MS - 1)
        .expect("pump before five minutes");
    assert!(
        early.firings().is_empty(),
        "tick advance must not fire the wallClock reconnect deadline"
    );
    assert!(wall.drain_records().expect("drain early").is_empty());
    assert!(!client.trace().bot_utterance_ticks().is_empty());
    assert!(!server.trace().server_checkpoint_ticks().is_empty());

    let expired = wall.pump(RECONNECT_RETENTION_MS).expect("pump at 300s");
    assert_eq!(expired.firings().len(), 1);
    assert_eq!(expired.firings()[0].handle, handle);
    assert_eq!(
        expired.firings()[0].slot_dispatch_id,
        RECONNECT_RETENTION_DISPATCH
    );
    let records = wall.drain_records().expect("drain reconnect");
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].due_tick, RECONNECT_RETENTION_MS);
    assert_eq!(
        wall.cancel(handle),
        Err(lumio_timer::TimerError::StaleHandle)
    );
    assert!(
        client
            .trace()
            .events()
            .iter()
            .chain(server.trace().events())
            .all(|e| !format!("{e:?}").to_ascii_lowercase().contains("reconnect"))
    );
}
