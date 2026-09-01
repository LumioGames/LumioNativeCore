---
name: timer-manager
description: Native Tick/Frame Timer Manager 与 Server/Client adapter——查调度、CallbackSlot 或切片消费者时读
metadata:
  type: doc
  status: 已交付
---

# Native Tick/Frame Timer Manager

确定性 gameplay 调度：固定 Tick/Frame 的 one-shot / repeating / cancel，经 CallbackSlot 投递。墙钟 deadline 不在本模块。

## 背景 / 目标

切片必须给 Timer 补真实消费者（Bot 节奏走 Client、服务器周期任务走 Server）；r1 只出接口无人使用。契约真值是架构源 `lumio.native-timer-abi.v1`。

## 设计

- **设计面**：两层一等公民。Native Manager 拥有 Tick/Frame；宿主 Timer 服务拥有单调墙钟（五分钟重连）。开火窗口 `(committedTick, toTick]`。
- **交互面**：四个公开操作 `scheduleOneShot` / `scheduleRepeating` / `cancel` / `advance`；`drain` 是 adapter 分发点。CallbackSlot 生命周期 `unbound → armed → delivering → closed`。
- **实现面**：crate `lumio-timer`。Bot 节奏 N=5 ticks（`BOT_CHAT_CADENCE_TICKS`）。Server 周期任务是 world authority heartbeat，每 10 ticks。`slot_queue_full` 稳定拒绝并使该定时器终态，进程继续。

## 待解决

- 向 native core scheduler 统一的 P1/P2（单调时间域）不在本切片。

## 相关

- [ADR 0007](../../decisions/0007-timer-manager-in-process-api.md)
- [`modules/timer/README.md`](../../../modules/timer/README.md)
- 架构源 ADR-055
