# 0008 · 唯一定时内核经 native-abi.json 导出，wallClock 与 tickFrame 为同一内核的两种模式

- 日期:2026-09-02
- 状态:生效
- 取代:0007

## 背景

架构源 ADR-056 §7 修订 ADR-055：定时内核只有一个，在 NativeCore，经 `engine/abi/native-abi.json` 暴露给托管侧；`wallClock` 与 `tickFrame` 是同一内核的两种模式，不是两套基础设施。0007 把 Timer Manager 做成不进 C ABI 的进程内 rlib，并并列 `HostTimerService` 墙钟门面——这是第二套定时器，与收敛原则冲突。

C-4′（Arch `936046a`，DEFINITION_SHA256 `ee2f6c6dc2e73a58561ba82325bc1c7c12fbfee52e94e9466642bd0a38510a41`）冻结 `abiSurface`：`timer_*` 槽、destroy=shutdown-tombstone、`slotDispatchId` 为 u32、禁止函数指针。

## 决策

`lumio-timer` 的 `TimerManager` 增加 `TimerMode::{WallClock, TickFrame}`：wallClock 用单调毫秒 `pump(now_ms)`，tickFrame 用 `advance(to_tick)`；共用 handle / slot / 错误码；每个 manager 实例自有 handle 空间，两模式互不解析对方句柄。

`lumio-native-ffi` 按 `native-abi.json` 根表逐字段组装 `LumioEngineRootApiV1`，填充全部 `timer_*` 槽（CLR host 槽仍空，归 CoreEngine）。不导出跨仓 Root 符号。`timer_destroy_manager` 首次 Success 后句柄保持 shutdown-tombstone，其后任意 `timer_*` 返回 status 17。CallbackSlot 只接受预注册 u32 dispatch id。

删除 `HostTimerService`。五分钟重连保留窗改由 kernel:wallClock one-shot 承载。

## 后果

- `lumio-native-ffi -> lumio-timer` 成为批准依赖；`lumio-timer` 不再依赖 `lumio-platform`。
- 消费方契约（TimerHandle、投递保证、错误码）以 C-4′ JSON + `native-abi.json` 为真值。
- 各层 Timer Manager 只做适配：注册 scope/dispatch、绑定 slot、drain。不得另建内核。
