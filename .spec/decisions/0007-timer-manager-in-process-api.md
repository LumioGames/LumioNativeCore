# 0007 · Timer Manager 以独立 crate 提供进程内 Tick/Frame API，不进入 C ABI

- 日期:2026-09-01
- 状态:被 0008 取代

## 背景

架构源 ADR-055 冻结 `lumio.native-timer-abi.v1`：Native Tick/Frame Timer Manager 是进程内 API 契约，明确不进入 `engine/abi/native-abi.json`。R-00352 要在 NativeCore 交付 core、Server/Client adapter，以及 Bot 节奏 / 服务器周期任务消费者。墙钟五分钟重连窗口归宿主 Timer 服务，本卡不得做成第二套 gameplay 定时真值。

## 决策

新增 `lumio-timer` rlib：core `TimerManager` + `ClientTimerManager` / `ServerTimerManager` adapter。CallbackSlot 只接受预注册 `DispatchId`，ABI 面不出现函数指针。`lumio-native-ffi` 不依赖本 crate（C 导出面零变更）。并列的 `HostTimerService` 是单调墙钟 typed-command 门面，供切片证明重连窗口不在 Tick Manager 上；它不是 Tick/Frame 调度器。测试剖面冻结 Bot cadence `N = 5` Tick；服务器周期任务为 world occupancy/heartbeat，间隔 10 Tick。`advance` 窗口为 `(committedTick, toTick]`。

## 后果

- 依赖方向：`lumio-timer -> lumio-platform`（仅宿主门面使用单调时钟类型）；不依赖 diagnostics 实现。
- 消费方契约（TimerHandle、投递保证、错误码）以架构源 JSON 为唯一真值；本仓镜像在 `docs/architecture/wire/`。
