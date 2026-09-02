//! Timer slots of `engine/abi/native-abi.json` (C-4′). Table fields match
//! the definition field-by-field; NativeCore does not export the Root entry
//! symbol. CallbackSlot never takes a function pointer.

use core::ffi::{c_char, c_void};
use std::collections::HashSet;
use std::sync::Mutex;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU64, Ordering};

use lumio_timer::{
    CallbackSlot, DispatchId, ScopeKind, SlotLifecycle, TimerError, TimerHandle, TimerManager,
    TimerMode, TimerScope,
};

/// SHA-256 of Arch `936046a` `engine/abi/native-abi.json`.
pub const NATIVE_ABI_DEFINITION_SHA256: &str =
    "ee2f6c6dc2e73a58561ba82325bc1c7c12fbfee52e94e9466642bd0a38510a41";

#[cfg(test)]
const NATIVE_ABI_JSON: &[u8] = include_bytes!("../native-abi.json");

pub const STATUS_SUCCESS: i32 = 0;
pub const STATUS_INVALID_ARGUMENT: i32 = 1;
pub const STATUS_BUFFER_TOO_SMALL: i32 = 5;
pub const STATUS_TIMER_STALE_HANDLE: i32 = 6;
pub const STATUS_TIMER_SCOPE_INVALID: i32 = 7;
pub const STATUS_TIMER_SCOPE_GENERATION_MISMATCH: i32 = 8;
pub const STATUS_TIMER_INVALID_DUE_TICK: i32 = 9;
pub const STATUS_TIMER_INVALID_INTERVAL: i32 = 10;
pub const STATUS_TIMER_SCHEDULE_BUDGET_EXCEEDED: i32 = 11;
pub const STATUS_TIMER_SLOT_CLOSED: i32 = 12;
pub const STATUS_TIMER_SLOT_UNBOUND: i32 = 13;
pub const STATUS_TIMER_SLOT_DISPATCH_MISMATCH: i32 = 14;
pub const STATUS_TIMER_SLOT_QUEUE_FULL: i32 = 15;
pub const STATUS_TIMER_LATE_COMPLETION: i32 = 16;
pub const STATUS_TIMER_MANAGER_SHUTDOWN: i32 = 17;

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TimerHandleAbi {
    pub index: u32,
    pub generation: u32,
    pub context: u64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TimerDrainRecord {
    pub handle_index: u32,
    pub handle_generation: u32,
    pub handle_context: u64,
    pub due: u64,
    pub schedule_sequence: u64,
    pub slot_dispatch_id: u32,
    pub pad: u32,
}

#[repr(C)]
pub struct LumioEngineRootApiV1 {
    pub abi_version: u32,
    pub struct_size: u32,
    pub abi_hash: [u8; 32],
    pub build_id: [u8; 16],
    pub ping: Option<unsafe extern "C" fn(*mut c_void) -> i32>,
    pub create_clr_host: Option<
        unsafe extern "C" fn(
            *const c_char,
            *const c_char,
            *const c_char,
            *const c_char,
            *mut *mut c_void,
        ) -> i32,
    >,
    pub clr_host_call:
        Option<unsafe extern "C" fn(*mut c_void, *const u8, u32, *mut u8, u32, *mut u32) -> i32>,
    pub destroy_clr_host: Option<unsafe extern "C" fn(*mut c_void) -> i32>,
    pub timer_create_manager: Option<unsafe extern "C" fn(u32, *mut *mut c_void) -> i32>,
    pub timer_destroy_manager: Option<unsafe extern "C" fn(*mut c_void) -> i32>,
    pub timer_register_dispatch: Option<unsafe extern "C" fn(*mut c_void, u32) -> i32>,
    pub timer_register_scope: Option<unsafe extern "C" fn(*mut c_void, u64, u32, *mut u32) -> i32>,
    pub timer_teardown_scope: Option<unsafe extern "C" fn(*mut c_void, u64) -> i32>,
    pub timer_create_slot: Option<unsafe extern "C" fn(*mut c_void, *mut *mut c_void) -> i32>,
    pub timer_bind_slot: Option<unsafe extern "C" fn(*mut c_void, *mut c_void, u32) -> i32>,
    pub timer_close_slot: Option<unsafe extern "C" fn(*mut c_void, *mut c_void) -> i32>,
    pub timer_schedule_one_shot: Option<
        unsafe extern "C" fn(
            *mut c_void,
            u64,
            u32,
            u32,
            u64,
            *mut c_void,
            *mut TimerHandleAbi,
        ) -> i32,
    >,
    pub timer_schedule_repeating: Option<
        unsafe extern "C" fn(
            *mut c_void,
            u64,
            u32,
            u32,
            u64,
            u64,
            *mut c_void,
            *mut TimerHandleAbi,
        ) -> i32,
    >,
    pub timer_cancel: Option<unsafe extern "C" fn(*mut c_void, *const TimerHandleAbi) -> i32>,
    pub timer_advance: Option<unsafe extern "C" fn(*mut c_void, u64) -> i32>,
    pub timer_pump: Option<unsafe extern "C" fn(*mut c_void, u64) -> i32>,
    pub timer_drain:
        Option<unsafe extern "C" fn(*mut c_void, *mut TimerDrainRecord, u32, *mut u32) -> i32>,
}

struct FfiManager {
    kernel: TimerManager,
    mode: TimerMode,
}

static NEXT_CONTEXT: AtomicU64 = AtomicU64::new(1);
static ISSUED: OnceLock<Mutex<HashSet<usize>>> = OnceLock::new();

fn issued() -> std::sync::MutexGuard<'static, HashSet<usize>> {
    ISSUED
        .get_or_init(|| Mutex::new(HashSet::new()))
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}

pub fn map_timer_error(error: TimerError) -> i32 {
    match error {
        TimerError::StaleHandle => STATUS_TIMER_STALE_HANDLE,
        TimerError::ScopeInvalid => STATUS_TIMER_SCOPE_INVALID,
        TimerError::ScopeGenerationMismatch => STATUS_TIMER_SCOPE_GENERATION_MISMATCH,
        TimerError::InvalidDueTick => STATUS_TIMER_INVALID_DUE_TICK,
        TimerError::InvalidInterval => STATUS_TIMER_INVALID_INTERVAL,
        TimerError::ScheduleBudgetExceeded => STATUS_TIMER_SCHEDULE_BUDGET_EXCEEDED,
        TimerError::SlotClosed => STATUS_TIMER_SLOT_CLOSED,
        TimerError::SlotUnbound => STATUS_TIMER_SLOT_UNBOUND,
        TimerError::SlotDispatchMismatch => STATUS_TIMER_SLOT_DISPATCH_MISMATCH,
        TimerError::SlotQueueFull => STATUS_TIMER_SLOT_QUEUE_FULL,
        TimerError::LateCompletion => STATUS_TIMER_LATE_COMPLETION,
        TimerError::ManagerShutdown => STATUS_TIMER_MANAGER_SHUTDOWN,
    }
}

fn hex_nibble(value: u8) -> u8 {
    match value {
        b'0'..=b'9' => value - b'0',
        b'a'..=b'f' => value - b'a' + 10,
        b'A'..=b'F' => value - b'A' + 10,
        _ => 0,
    }
}

fn decode_hex<const N: usize>(value: &str) -> [u8; N] {
    let bytes = value.as_bytes();
    let mut output = [0; N];
    for (index, slot) in output.iter_mut().enumerate() {
        let offset = index * 2;
        *slot = (hex_nibble(bytes[offset]) << 4) | hex_nibble(bytes[offset + 1]);
    }
    output
}

fn pack_slot(slot: CallbackSlot) -> *mut c_void {
    let packed = (u64::from(slot.index()) << 32) | u64::from(slot.generation());
    packed as *mut c_void
}

fn unpack_slot(ptr: *mut c_void) -> Option<CallbackSlot> {
    if ptr.is_null() {
        return None;
    }
    let packed = ptr as u64;
    Some(CallbackSlot::from_abi((packed >> 32) as u32, packed as u32))
}

fn manager_mut(ptr: *mut c_void) -> Result<&'static mut FfiManager, i32> {
    if ptr.is_null() {
        return Err(STATUS_INVALID_ARGUMENT);
    }
    if !issued().contains(&(ptr as usize)) {
        return Err(STATUS_INVALID_ARGUMENT);
    }
    // SAFETY: pointer was issued by timer_create_manager and is never freed.
    Ok(unsafe { &mut *ptr.cast::<FfiManager>() })
}

fn running_manager(ptr: *mut c_void) -> Result<&'static mut FfiManager, i32> {
    let manager = manager_mut(ptr)?;
    if !manager.kernel.is_running() {
        return Err(STATUS_TIMER_MANAGER_SHUTDOWN);
    }
    Ok(manager)
}

fn scope_kind(raw: u32) -> Result<ScopeKind, i32> {
    match raw {
        0 => Ok(ScopeKind::World),
        1 => Ok(ScopeKind::Session),
        2 => Ok(ScopeKind::Adapter),
        _ => Err(STATUS_TIMER_SCOPE_INVALID),
    }
}

unsafe extern "C" fn ping(marker: *mut c_void) -> i32 {
    if marker.is_null() {
        return STATUS_INVALID_ARGUMENT;
    }
    unsafe { marker.cast::<u32>().write(1) };
    STATUS_SUCCESS
}

pub unsafe extern "C" fn timer_create_manager(mode: u32, out_manager: *mut *mut c_void) -> i32 {
    if out_manager.is_null() {
        return STATUS_INVALID_ARGUMENT;
    }
    let Some(mode) = TimerMode::from_abi(mode) else {
        unsafe { out_manager.write(core::ptr::null_mut()) };
        return STATUS_INVALID_ARGUMENT;
    };
    let context = NEXT_CONTEXT.fetch_add(1, Ordering::Relaxed);
    let boxed = Box::new(FfiManager {
        kernel: TimerManager::with_mode(context, mode),
        mode,
    });
    let ptr = Box::into_raw(boxed);
    issued().insert(ptr as usize);
    unsafe { out_manager.write(ptr.cast()) };
    STATUS_SUCCESS
}

pub unsafe extern "C" fn timer_destroy_manager(manager: *mut c_void) -> i32 {
    let mgr = match manager_mut(manager) {
        Ok(m) => m,
        Err(code) => return code,
    };
    if !mgr.kernel.is_running() {
        return STATUS_TIMER_MANAGER_SHUTDOWN;
    }
    mgr.kernel.shutdown();
    STATUS_SUCCESS
}

pub unsafe extern "C" fn timer_register_dispatch(manager: *mut c_void, dispatch_id: u32) -> i32 {
    let mgr = match running_manager(manager) {
        Ok(m) => m,
        Err(code) => return code,
    };
    if dispatch_id == 0 {
        return STATUS_INVALID_ARGUMENT;
    }
    let id = DispatchId::from_raw(dispatch_id);
    if mgr.kernel.is_dispatch_registered(id) {
        return STATUS_INVALID_ARGUMENT;
    }
    mgr.kernel
        .register_dispatch(id, lumio_timer::DispatchTarget::Registered);
    STATUS_SUCCESS
}

pub unsafe extern "C" fn timer_register_scope(
    manager: *mut c_void,
    scope_id: u64,
    scope_kind_raw: u32,
    out_generation: *mut u32,
) -> i32 {
    if out_generation.is_null() {
        return STATUS_INVALID_ARGUMENT;
    }
    let mgr = match running_manager(manager) {
        Ok(m) => m,
        Err(code) => return code,
    };
    if mgr.kernel.is_scope_alive(scope_id) {
        return STATUS_INVALID_ARGUMENT;
    }
    let kind = match scope_kind(scope_kind_raw) {
        Ok(k) => k,
        Err(code) => return code,
    };
    match mgr.kernel.register_scope(scope_id, kind) {
        Ok(scope) => {
            unsafe { out_generation.write(scope.generation()) };
            STATUS_SUCCESS
        }
        Err(error) => map_timer_error(error),
    }
}

pub unsafe extern "C" fn timer_teardown_scope(manager: *mut c_void, scope_id: u64) -> i32 {
    let mgr = match running_manager(manager) {
        Ok(m) => m,
        Err(code) => return code,
    };
    match mgr.kernel.teardown_scope(scope_id) {
        Ok(_) => STATUS_SUCCESS,
        Err(error) => map_timer_error(error),
    }
}

pub unsafe extern "C" fn timer_create_slot(
    manager: *mut c_void,
    out_slot: *mut *mut c_void,
) -> i32 {
    if out_slot.is_null() {
        return STATUS_INVALID_ARGUMENT;
    }
    let mgr = match running_manager(manager) {
        Ok(m) => m,
        Err(code) => return code,
    };
    match mgr.kernel.create_slot() {
        Ok(slot) => {
            unsafe { out_slot.write(pack_slot(slot)) };
            STATUS_SUCCESS
        }
        Err(error) => {
            unsafe { out_slot.write(core::ptr::null_mut()) };
            map_timer_error(error)
        }
    }
}

pub unsafe extern "C" fn timer_bind_slot(
    manager: *mut c_void,
    slot: *mut c_void,
    dispatch_id: u32,
) -> i32 {
    let mgr = match running_manager(manager) {
        Ok(m) => m,
        Err(code) => return code,
    };
    let Some(slot) = unpack_slot(slot) else {
        return STATUS_INVALID_ARGUMENT;
    };
    if dispatch_id == 0 {
        return STATUS_INVALID_ARGUMENT;
    }
    let id = DispatchId::from_raw(dispatch_id);
    if !mgr.kernel.is_dispatch_registered(id) {
        return STATUS_INVALID_ARGUMENT;
    }
    match mgr.kernel.slot_lifecycle(slot) {
        Ok(SlotLifecycle::Armed) => STATUS_INVALID_ARGUMENT,
        Ok(SlotLifecycle::Closed) => STATUS_TIMER_SLOT_CLOSED,
        Ok(SlotLifecycle::Unbound) => match mgr.kernel.bind_slot(slot, id) {
            Ok(()) => STATUS_SUCCESS,
            Err(error) => map_timer_error(error),
        },
        Err(error) => map_timer_error(error),
    }
}

pub unsafe extern "C" fn timer_close_slot(manager: *mut c_void, slot: *mut c_void) -> i32 {
    let mgr = match running_manager(manager) {
        Ok(m) => m,
        Err(code) => return code,
    };
    let Some(slot) = unpack_slot(slot) else {
        return STATUS_INVALID_ARGUMENT;
    };
    match mgr.kernel.close_slot(slot) {
        Ok(()) => STATUS_SUCCESS,
        Err(error) => map_timer_error(error),
    }
}

fn write_handle(out: *mut TimerHandleAbi, handle: TimerHandle) {
    unsafe {
        out.write(TimerHandleAbi {
            index: handle.index(),
            generation: handle.generation(),
            context: handle.context(),
        });
    }
}

fn read_handle(ptr: *const TimerHandleAbi) -> Option<TimerHandle> {
    if ptr.is_null() {
        return None;
    }
    let abi = unsafe { ptr.read() };
    Some(TimerHandle::from_abi(
        abi.index,
        abi.generation,
        abi.context,
    ))
}

fn schedule_scope(scope_id: u64, kind: u32, generation: u32) -> Result<TimerScope, i32> {
    Ok(TimerScope::new(scope_id, scope_kind(kind)?, generation))
}

pub unsafe extern "C" fn timer_schedule_one_shot(
    manager: *mut c_void,
    scope_id: u64,
    scope_kind_raw: u32,
    scope_generation: u32,
    due: u64,
    slot: *mut c_void,
    out_handle: *mut TimerHandleAbi,
) -> i32 {
    if out_handle.is_null() {
        return STATUS_INVALID_ARGUMENT;
    }
    let mgr = match running_manager(manager) {
        Ok(m) => m,
        Err(code) => return code,
    };
    let Some(slot) = unpack_slot(slot) else {
        return STATUS_INVALID_ARGUMENT;
    };
    let scope = match schedule_scope(scope_id, scope_kind_raw, scope_generation) {
        Ok(s) => s,
        Err(code) => return code,
    };
    match mgr.kernel.schedule_one_shot(scope, due, slot) {
        Ok(handle) => {
            write_handle(out_handle, handle);
            STATUS_SUCCESS
        }
        Err(error) => map_timer_error(error),
    }
}

pub unsafe extern "C" fn timer_schedule_repeating(
    manager: *mut c_void,
    scope_id: u64,
    scope_kind_raw: u32,
    scope_generation: u32,
    first_due: u64,
    interval: u64,
    slot: *mut c_void,
    out_handle: *mut TimerHandleAbi,
) -> i32 {
    if out_handle.is_null() {
        return STATUS_INVALID_ARGUMENT;
    }
    let mgr = match running_manager(manager) {
        Ok(m) => m,
        Err(code) => return code,
    };
    let Some(slot) = unpack_slot(slot) else {
        return STATUS_INVALID_ARGUMENT;
    };
    let scope = match schedule_scope(scope_id, scope_kind_raw, scope_generation) {
        Ok(s) => s,
        Err(code) => return code,
    };
    match mgr
        .kernel
        .schedule_repeating(scope, first_due, interval, slot)
    {
        Ok(handle) => {
            write_handle(out_handle, handle);
            STATUS_SUCCESS
        }
        Err(error) => map_timer_error(error),
    }
}

pub unsafe extern "C" fn timer_cancel(manager: *mut c_void, handle: *const TimerHandleAbi) -> i32 {
    let mgr = match running_manager(manager) {
        Ok(m) => m,
        Err(code) => return code,
    };
    let Some(handle) = read_handle(handle) else {
        return STATUS_INVALID_ARGUMENT;
    };
    match mgr.kernel.cancel(handle) {
        Ok(_) => STATUS_SUCCESS,
        Err(error) => map_timer_error(error),
    }
}

pub unsafe extern "C" fn timer_advance(manager: *mut c_void, to_tick: u64) -> i32 {
    let mgr = match running_manager(manager) {
        Ok(m) => m,
        Err(code) => return code,
    };
    if mgr.mode != TimerMode::TickFrame {
        return STATUS_INVALID_ARGUMENT;
    }
    match mgr.kernel.advance(to_tick) {
        Ok(_) => STATUS_SUCCESS,
        Err(error) => map_timer_error(error),
    }
}

pub unsafe extern "C" fn timer_pump(manager: *mut c_void, now_ms: u64) -> i32 {
    let mgr = match running_manager(manager) {
        Ok(m) => m,
        Err(code) => return code,
    };
    if mgr.mode != TimerMode::WallClock {
        return STATUS_INVALID_ARGUMENT;
    }
    match mgr.kernel.pump(now_ms) {
        Ok(_) => STATUS_SUCCESS,
        Err(error) => map_timer_error(error),
    }
}

pub unsafe extern "C" fn timer_drain(
    manager: *mut c_void,
    out_records: *mut TimerDrainRecord,
    capacity: u32,
    out_count: *mut u32,
) -> i32 {
    if out_count.is_null() {
        return STATUS_INVALID_ARGUMENT;
    }
    if capacity > 0 && out_records.is_null() {
        return STATUS_INVALID_ARGUMENT;
    }
    let mgr = match running_manager(manager) {
        Ok(m) => m,
        Err(code) => return code,
    };
    let needed = mgr.kernel.pending_record_count();
    if needed > capacity {
        unsafe { out_count.write(needed) };
        return STATUS_BUFFER_TOO_SMALL;
    }
    match mgr.kernel.drain_records() {
        Ok(records) => {
            unsafe { out_count.write(records.len() as u32) };
            for (index, record) in records.iter().enumerate() {
                unsafe {
                    out_records.add(index).write(TimerDrainRecord {
                        handle_index: record.handle.index(),
                        handle_generation: record.handle.generation(),
                        handle_context: record.handle.context(),
                        due: record.due_tick,
                        schedule_sequence: record.schedule_sequence,
                        slot_dispatch_id: record.slot_dispatch_id.raw(),
                        pad: 0,
                    });
                }
            }
            STATUS_SUCCESS
        }
        Err(error) => map_timer_error(error),
    }
}

/// Assemble the native-abi.json root table. Timer slots are populated; CLR
/// host slots stay unpopulated (CoreEngine-owned). No Root entry symbol.
pub fn provider_engine_root_api() -> LumioEngineRootApiV1 {
    LumioEngineRootApiV1 {
        abi_version: 1,
        struct_size: core::mem::size_of::<LumioEngineRootApiV1>() as u32,
        abi_hash: decode_hex(NATIVE_ABI_DEFINITION_SHA256),
        build_id: [0; 16],
        ping: Some(ping),
        create_clr_host: None,
        clr_host_call: None,
        destroy_clr_host: None,
        timer_create_manager: Some(timer_create_manager),
        timer_destroy_manager: Some(timer_destroy_manager),
        timer_register_dispatch: Some(timer_register_dispatch),
        timer_register_scope: Some(timer_register_scope),
        timer_teardown_scope: Some(timer_teardown_scope),
        timer_create_slot: Some(timer_create_slot),
        timer_bind_slot: Some(timer_bind_slot),
        timer_close_slot: Some(timer_close_slot),
        timer_schedule_one_shot: Some(timer_schedule_one_shot),
        timer_schedule_repeating: Some(timer_schedule_repeating),
        timer_cancel: Some(timer_cancel),
        timer_advance: Some(timer_advance),
        timer_pump: Some(timer_pump),
        timer_drain: Some(timer_drain),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use std::process::Command;

    const ROOT_FIELDS: &[(&str, &str)] = &[
        ("abi_version", "u32"),
        ("struct_size", "u32"),
        ("abi_hash", "bytes32"),
        ("build_id", "bytes16"),
        ("ping", "fn(pointer) -> status"),
        (
            "create_clr_host",
            "fn(cstring, cstring, cstring, cstring, pointer) -> status",
        ),
        (
            "clr_host_call",
            "fn(pointer, pointer, u32, pointer, u32, pointer) -> status",
        ),
        ("destroy_clr_host", "fn(pointer) -> status"),
        ("timer_create_manager", "fn(u32, pointer) -> status"),
        ("timer_destroy_manager", "fn(pointer) -> status"),
        ("timer_register_dispatch", "fn(pointer, u32) -> status"),
        (
            "timer_register_scope",
            "fn(pointer, u64, u32, pointer) -> status",
        ),
        ("timer_teardown_scope", "fn(pointer, u64) -> status"),
        ("timer_create_slot", "fn(pointer, pointer) -> status"),
        ("timer_bind_slot", "fn(pointer, pointer, u32) -> status"),
        ("timer_close_slot", "fn(pointer, pointer) -> status"),
        (
            "timer_schedule_one_shot",
            "fn(pointer, u64, u32, u32, u64, pointer, pointer) -> status",
        ),
        (
            "timer_schedule_repeating",
            "fn(pointer, u64, u32, u32, u64, u64, pointer, pointer) -> status",
        ),
        ("timer_cancel", "fn(pointer, pointer) -> status"),
        ("timer_advance", "fn(pointer, u64) -> status"),
        ("timer_pump", "fn(pointer, u64) -> status"),
        (
            "timer_drain",
            "fn(pointer, pointer, u32, pointer) -> status",
        ),
    ];

    fn parse_root_fields(json: &str) -> Vec<(String, String)> {
        let root = json.find("\"root\"").expect("native-abi.json has root");
        let after_root = &json[root..];
        let status = after_root
            .find("\n  \"status\"")
            .expect("status follows root");
        let body = &after_root[..status];
        let mut fields = Vec::new();
        let mut rest = body;
        while let Some(name_at) = rest.find("\"name\"") {
            rest = &rest[name_at + 6..];
            let start = rest.find('"').expect("name string") + 1;
            let end = start + rest[start..].find('"').expect("name end");
            let name = rest[start..end].to_string();
            rest = &rest[end + 1..];
            let type_key = rest.find("\"type\"").expect("type");
            rest = &rest[type_key + 6..];
            let start = rest.find('"').expect("type string") + 1;
            let end = start + rest[start..].find('"').expect("type end");
            let ty = rest[start..end].to_string();
            rest = &rest[end + 1..];
            fields.push((name, ty));
        }
        fields
    }

    fn sha256_hex(path: &Path) -> String {
        if let Ok(out) = Command::new("sha256sum").arg(path).output()
            && out.status.success()
        {
            let text = String::from_utf8_lossy(&out.stdout);
            if let Some(hex) = text.split_whitespace().next() {
                return hex.to_ascii_lowercase();
            }
        }
        let out = Command::new("certutil")
            .args(["-hashfile", &path.to_string_lossy(), "SHA256"])
            .output()
            .expect("certutil");
        assert!(out.status.success(), "certutil failed");
        let text = String::from_utf8_lossy(&out.stdout);
        for line in text.lines() {
            let compact: String = line.chars().filter(|c| !c.is_whitespace()).collect();
            if compact.len() == 64 && compact.bytes().all(|b| b.is_ascii_hexdigit()) {
                return compact.to_ascii_lowercase();
            }
        }
        panic!("no sha256 in hasher output");
    }

    fn table() -> LumioEngineRootApiV1 {
        provider_engine_root_api()
    }

    unsafe fn create(mode: u32) -> *mut c_void {
        let mut manager = core::ptr::null_mut();
        let status = unsafe { timer_create_manager(mode, &mut manager) };
        assert_eq!(status, STATUS_SUCCESS);
        assert!(!manager.is_null());
        manager
    }

    unsafe fn arm_slot(manager: *mut c_void, dispatch: u32) -> (*mut c_void, u32) {
        assert_eq!(
            unsafe { timer_register_dispatch(manager, dispatch) },
            STATUS_SUCCESS
        );
        let mut generation = 0u32;
        assert_eq!(
            unsafe { timer_register_scope(manager, 1, 0, &mut generation) },
            STATUS_SUCCESS
        );
        assert_eq!(generation, 1);
        let mut slot = core::ptr::null_mut();
        assert_eq!(
            unsafe { timer_create_slot(manager, &mut slot) },
            STATUS_SUCCESS
        );
        assert_eq!(
            unsafe { timer_bind_slot(manager, slot, dispatch) },
            STATUS_SUCCESS
        );
        (slot, generation)
    }

    #[test]
    fn native_abi_json_hash_matches_definition_sha256() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("native-abi.json");
        let hashed = sha256_hex(&path);
        assert_eq!(hashed, NATIVE_ABI_DEFINITION_SHA256);
        assert_eq!(NATIVE_ABI_JSON.len(), 12117);
        let table = table();
        assert_eq!(table.abi_hash, decode_hex(NATIVE_ABI_DEFINITION_SHA256));
        println!("DEFINITION_SHA256={hashed}");
        println!(
            "abi_hash={}",
            table
                .abi_hash
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>()
        );
    }

    #[test]
    fn root_table_fields_match_native_abi_json() {
        let json = core::str::from_utf8(NATIVE_ABI_JSON).expect("utf8");
        let parsed = parse_root_fields(json);
        assert_eq!(parsed.len(), ROOT_FIELDS.len());
        for (actual, expected) in parsed.iter().zip(ROOT_FIELDS.iter()) {
            assert_eq!(actual.0, expected.0, "field name");
            assert_eq!(actual.1, expected.1, "field type for {}", expected.0);
        }
        let table = table();
        assert_eq!(table.abi_version, 1);
        assert_eq!(
            table.struct_size,
            core::mem::size_of::<LumioEngineRootApiV1>() as u32
        );
        assert!(table.timer_create_manager.is_some());
        assert!(table.timer_drain.is_some());
        assert!(table.create_clr_host.is_none());
        assert_eq!(core::mem::size_of::<TimerHandleAbi>(), 16);
        assert_eq!(core::mem::size_of::<TimerDrainRecord>(), 40);
        assert_eq!(
            core::mem::offset_of!(TimerDrainRecord, slot_dispatch_id),
            32
        );
        assert_eq!(core::mem::offset_of!(TimerDrainRecord, pad), 36);
    }

    #[test]
    fn managed_wall_clock_one_shot_pump_drain_cancel_stale() {
        let table = table();
        let create = table.timer_create_manager.expect("create");
        let register_dispatch = table.timer_register_dispatch.expect("dispatch");
        let register_scope = table.timer_register_scope.expect("scope");
        let create_slot = table.timer_create_slot.expect("slot");
        let bind = table.timer_bind_slot.expect("bind");
        let schedule = table.timer_schedule_one_shot.expect("schedule");
        let pump = table.timer_pump.expect("pump");
        let drain = table.timer_drain.expect("drain");
        let cancel = table.timer_cancel.expect("cancel");
        let destroy = table.timer_destroy_manager.expect("destroy");
        let advance = table.timer_advance.expect("advance");

        let mut manager = core::ptr::null_mut();
        assert_eq!(unsafe { create(0, &mut manager) }, STATUS_SUCCESS);

        assert_eq!(unsafe { register_dispatch(manager, 7) }, STATUS_SUCCESS);
        let mut generation = 0u32;
        assert_eq!(
            unsafe { register_scope(manager, 9, 1, &mut generation) },
            STATUS_SUCCESS
        );
        let mut slot = core::ptr::null_mut();
        assert_eq!(unsafe { create_slot(manager, &mut slot) }, STATUS_SUCCESS);
        assert_eq!(unsafe { bind(manager, slot, 7) }, STATUS_SUCCESS);

        let mut handle = TimerHandleAbi {
            index: 0,
            generation: 0,
            context: 0,
        };
        assert_eq!(
            unsafe { schedule(manager, 9, 1, generation, 1500, slot, &mut handle) },
            STATUS_SUCCESS
        );
        assert_ne!(handle.generation, 0);

        assert_eq!(
            unsafe { advance(manager, 1) },
            STATUS_INVALID_ARGUMENT,
            "advance on wallClock is InvalidArgument"
        );

        assert_eq!(unsafe { pump(manager, 1000) }, STATUS_SUCCESS);
        let mut count = 99u32;
        let mut records = [TimerDrainRecord {
            handle_index: 0,
            handle_generation: 0,
            handle_context: 0,
            due: 0,
            schedule_sequence: 0,
            slot_dispatch_id: 0,
            pad: 0,
        }; 4];
        assert_eq!(
            unsafe { drain(manager, records.as_mut_ptr(), 4, &mut count) },
            STATUS_SUCCESS
        );
        assert_eq!(count, 0);

        assert_eq!(unsafe { pump(manager, 1500) }, STATUS_SUCCESS);
        count = 0;
        assert_eq!(
            unsafe { drain(manager, records.as_mut_ptr(), 4, &mut count) },
            STATUS_SUCCESS
        );
        assert_eq!(count, 1);
        assert_eq!(records[0].due, 1500);
        assert_eq!(records[0].slot_dispatch_id, 7);
        assert_eq!(records[0].handle_index, handle.index);
        assert_eq!(records[0].handle_generation, handle.generation);
        assert_eq!(records[0].pad, 0);

        assert_eq!(
            unsafe { cancel(manager, &handle) },
            STATUS_TIMER_STALE_HANDLE
        );

        assert_eq!(unsafe { destroy(manager) }, STATUS_SUCCESS);
        assert_eq!(
            unsafe { destroy(manager) },
            STATUS_TIMER_MANAGER_SHUTDOWN,
            "tombstone destroy returns status 17"
        );
        assert_eq!(
            unsafe { pump(manager, 2000) },
            STATUS_TIMER_MANAGER_SHUTDOWN
        );
        assert_eq!(
            unsafe { drain(manager, records.as_mut_ptr(), 4, &mut count) },
            STATUS_TIMER_MANAGER_SHUTDOWN
        );
        assert_eq!(
            unsafe { cancel(manager, &handle) },
            STATUS_TIMER_MANAGER_SHUTDOWN
        );
    }

    #[test]
    fn managed_tick_frame_rejects_pump_and_isolates_handles() {
        let wall = unsafe { create(0) };
        let ticks = unsafe { create(1) };
        let (wall_slot, wall_gen) = unsafe { arm_slot(wall, 11) };
        let (tick_slot, tick_gen) = unsafe { arm_slot(ticks, 12) };

        let mut wall_handle = TimerHandleAbi {
            index: 0,
            generation: 0,
            context: 0,
        };
        let mut tick_handle = TimerHandleAbi {
            index: 0,
            generation: 0,
            context: 0,
        };
        assert_eq!(
            unsafe {
                timer_schedule_one_shot(wall, 1, 0, wall_gen, 50, wall_slot, &mut wall_handle)
            },
            STATUS_SUCCESS
        );
        assert_eq!(
            unsafe {
                timer_schedule_one_shot(ticks, 1, 0, tick_gen, 5, tick_slot, &mut tick_handle)
            },
            STATUS_SUCCESS
        );

        assert_eq!(unsafe { timer_pump(ticks, 1) }, STATUS_INVALID_ARGUMENT);
        assert_eq!(unsafe { timer_advance(wall, 1) }, STATUS_INVALID_ARGUMENT);
        assert_eq!(
            unsafe { timer_cancel(wall, &tick_handle) },
            STATUS_TIMER_STALE_HANDLE
        );
        assert_eq!(
            unsafe { timer_cancel(ticks, &wall_handle) },
            STATUS_TIMER_STALE_HANDLE
        );
        assert_eq!(unsafe { timer_cancel(wall, &wall_handle) }, STATUS_SUCCESS);
        assert_eq!(unsafe { timer_cancel(ticks, &tick_handle) }, STATUS_SUCCESS);
        assert_eq!(unsafe { timer_destroy_manager(wall) }, STATUS_SUCCESS);
        assert_eq!(unsafe { timer_destroy_manager(ticks) }, STATUS_SUCCESS);
    }

    #[test]
    fn drain_buffer_too_small_does_not_partial_write() {
        let manager = unsafe { create(1) };
        let (slot, generation) = unsafe { arm_slot(manager, 3) };
        let mut handle = TimerHandleAbi {
            index: 0,
            generation: 0,
            context: 0,
        };
        assert_eq!(
            unsafe { timer_schedule_one_shot(manager, 1, 0, generation, 4, slot, &mut handle) },
            STATUS_SUCCESS
        );
        assert_eq!(unsafe { timer_advance(manager, 4) }, STATUS_SUCCESS);
        let mut count = 0u32;
        let mut records = [TimerDrainRecord {
            handle_index: 99,
            handle_generation: 99,
            handle_context: 99,
            due: 99,
            schedule_sequence: 99,
            slot_dispatch_id: 99,
            pad: 99,
        }; 1];
        assert_eq!(
            unsafe { timer_drain(manager, records.as_mut_ptr(), 0, &mut count) },
            STATUS_BUFFER_TOO_SMALL
        );
        assert_eq!(count, 1);
        assert_eq!(records[0].due, 99);
        assert_eq!(
            unsafe { timer_drain(manager, records.as_mut_ptr(), 1, &mut count) },
            STATUS_SUCCESS
        );
        assert_eq!(count, 1);
        assert_eq!(records[0].due, 4);
        assert_eq!(unsafe { timer_destroy_manager(manager) }, STATUS_SUCCESS);
    }

    #[test]
    fn null_manager_is_invalid_argument_not_shutdown() {
        assert_eq!(
            unsafe { timer_destroy_manager(core::ptr::null_mut()) },
            STATUS_INVALID_ARGUMENT
        );
        assert_eq!(
            unsafe { timer_pump(core::ptr::null_mut(), 1) },
            STATUS_INVALID_ARGUMENT
        );
    }
}
