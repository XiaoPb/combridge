# GH3220 协议模块

## 概述

GH3220 模块提供 UProtocol 的主机侧实现：BLE 连接、命令下发、数据流解码、事件 ACK、CSV 落盘与统计。
编解码与业务逻辑分离：

- `libs/gh-uprotocol/`：纯字节编解码，无 I/O，可独立测试与复用。
- `src-tauri/src/gh3220/`：传输、会话、管理器、CSV。

协议规范见 [`docs/protocols/gh3220/uprotocol-reference.md`](../../protocols/gh3220/uprotocol-reference.md)，
移植来源见 `libs/gh-uprotocol/UPSTREAM.md`。

## 模块结构

```
src-tauri/src/gh3220/
├── mod.rs          模块导出、Gh3220ManagerRef
├── types.rs        DTO、配置、错误、十六进制解析
├── link.rs         BLE / TCP 调试链路，BLE 分片写
├── session.rs      命令串行队列、oneshot、超时
├── manager.rs      生命周期、解码任务、ACK、采集、统计
└── csv_writer.rs   按 function_id 分文件、变宽列
```

## 数据流

```
BLE 通知 → BleManager 发布 ble:data
  → Gh3220Manager（过滤 address + notify UUID）
  → FrameDecoder
      ├─ 0x0B：DataDecoder → 批量 → gh3220:data → 前端 / CSV
      ├─ 0x16：事件解析 → 自动 ACK → gh3220:event
      └─ 其他：匹配 pending 请求

前端 invoke → commands/gh3220.rs → Manager → Session → Link
```

## BLE 约束

真机 GATT：

| 项 | UUID |
|---|---|
| Service | `0000190e-0000-1000-8000-00805f9b34fb` |
| 写特征 | `00000004-0000-1000-8000-00805f9b34fb` |
| 通知特征 | `00000003-0000-1000-8000-00805f9b34fb` |

实现时必须遵守：

1. 连接后先 `discover_services` / `discover_characteristics`，再订阅；`connect` 不会填充特征缓存。
2. 写特征按 `mtu - 3` 分片，整帧持有写锁；默认回退 20 字节。
3. 订阅前检查 `get_subscriptions`，避免重复订阅导致同一数据多次投递。
4. **订阅返回不代表 CCCD 已生效**：必须等首条通知（最长 1.5s）再下发命令，否则设备的第一条响应会在订阅生效前到达而丢失。
5. 仅支持 Native 模式；AT 模式的 notify/写语义不同。
6. 命令默认超时 2s：真机持续上报时，BLE 通知会挤占命令响应，800ms 在 Rust/BlueZ 链路上会偶发超时。

## 会话与解码

- 协议响应无序列号，命令必须串行：`Session` 用异步锁 + 单 pending + 800ms 默认超时。
- 解码任务独占 `FrameDecoder` / `DataDecoder`；EventBus 回调只负责把字节推进 mpsc。
- `0x16` 事件必须 ACK，失败只告警，不阻塞数据。
- 每功能首帧可能出现畸形 `0x2C` 包，重同步丢弃属预期，不报错。
- 通道数按掩码 popcount；可在采集配置中覆盖。

## 事件主题

| 主题 | 载荷 |
|---|---|
| `gh3220:status` | `{ status }` |
| `gh3220:data` | `{ seq, records, dropped }`，60ms / 100 条批量 |
| `gh3220:event` | `{ eventId, reportId, acked, tsMs }` |
| `gh3220:stats` | 解码统计，约 1s 一次 |

## CSV

按 `function_id` 分文件：`gh3220_func<N>_<yyyyMMdd_HHmmss>[_partN].csv`。

- 首条记录确定 `raw/agc` 列宽；变宽时新建 `_partN`，不回改表头。
- 可选字段（cap/temp/flags/slot）固定列，缺失填 0 或空。
- 空闲超过 `idle_close_secs` 关闭句柄，下次写入以追加方式重新打开。

## 配置

`Preferences.gh3220`：

```yaml
gh3220:
  csv:
    enabled: true
    outputDir: ...
    idleCloseSecs: 10
  capture:
    channelsOverride: null
    autoStop: false
  debugTcp:
    enabled: false
    address: 127.0.0.1:9000
```

调试 TCP 只允许回环地址，且仅在开发构建的调试页签可见。

## 已知设备行为

- `WORK_MODE` 是 `START` 的前置条件。
- 真机 STOP 后通常需要复位才能再次 START。
- `wakeup` 会清空 WORK_MODE。
- `GET_OPENED_FUNC` 不能用于判断当前采样状态。
- 设备 `MAX_CONN=1`，重连前需断开旧连接。

## 测试

- `cargo test --manifest-path libs/gh-uprotocol/Cargo.toml`：协议编解码 17 项。
- `src-tauri` 单测：分片边界、TCP 回环校验、CSV 变宽、寄存器表解析。
- 调试联调：`uprotocol-sim --listen 127.0.0.1:9000` + 开发构建的调试页签。
