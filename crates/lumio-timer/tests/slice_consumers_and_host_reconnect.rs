//! Slice consumers: Bot cadence on Client Timer Manager, server periodic on
//! Server Timer Manager, five-minute reconnect on Host timer service.

use std::sync::Arc;
use std::time::Duration;

use lumio_platform::{MonotonicClock, Ticks};
use lumio_test_support::FakeClock;
use lumio_timer::{
    BOT_CHAT_CADENCE_TICKS, ClientTimerManager, HostCommandKind, HostTimerService,
    RECONNECT_RETENTION_SECS, SERVER_WORLD_HEARTBEAT_TICKS, ScopeKind, ServerTimerManager,
    SliceTrace, SliceTraceEvent,
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
fn reconnect_deadline_stays_on_host_timer_service() {
    let clock = Arc::new(FakeClock::new(Ticks::ZERO));
    let mut host = HostTimerService::new(Arc::clone(&clock) as Arc<dyn MonotonicClock>);
    let mut client = ClientTimerManager::new(1);
    let mut server = ServerTimerManager::new(2);
    let mut trace = SliceTrace::new();

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

    let _key = host
        .schedule_reconnect_retention(42)
        .expect("host reconnect window");

    client.pump(20).expect("native client ticks");
    server.pump(20).expect("native server ticks");
    let early = host
        .poll_into(&mut trace)
        .expect("poll before five minutes");
    assert!(
        early.is_empty(),
        "tick advance must not fire the host reconnect deadline"
    );
    assert!(trace.events().is_empty());
    assert!(!client.trace().bot_utterance_ticks().is_empty());
    assert!(!server.trace().server_checkpoint_ticks().is_empty());

    clock.advance(Duration::from_secs(RECONNECT_RETENTION_SECS - 1));
    assert!(host.poll().expect("poll at 299s").is_empty());
    clock.advance(Duration::from_secs(1));
    let expired = host.poll_into(&mut trace).expect("poll at 300s");
    assert_eq!(expired.len(), 1);
    assert_eq!(
        expired[0].kind,
        HostCommandKind::ReconnectRetentionExpired { retention_id: 42 }
    );
    assert!(matches!(
        trace.events()[0],
        SliceTraceEvent::HostReconnectExpired { .. }
    ));
    assert!(
        client
            .trace()
            .events()
            .iter()
            .chain(server.trace().events())
            .all(|e| !format!("{e:?}").to_ascii_lowercase().contains("reconnect"))
    );
}
