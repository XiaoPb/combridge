# gh-uprotocol

GH3220 UProtocol 的主机侧编解码库（Rust）。纯字节处理，不做 I/O。

协议规范见 [`../../docs/protocols/gh3220/uprotocol-reference.md`](../../docs/protocols/gh3220/uprotocol-reference.md)，
移植来源与同步约定见 [`UPSTREAM.md`](UPSTREAM.md)。

本 crate 由 ComBridge 的 `src-tauri` 以 path 依赖引入，业务层实现见
`src-tauri/src/gh3220/`。

## 模块

| 模块 | 内容 |
|---|---|
| `frame` | `AA 11 CMD LEN PAYLOAD CRC8` 编解码 + 流式 `FrameDecoder`（含重同步与统计） |
| `crc8` | 表驱动 CRC8（poly 0x07 / init 0xFF），与 SDK 逐位一致 |
| `cmd` | 20 条命令的请求构造与响应解析 |
| `data` | `0x0B` 数据包：8 字节包头、非压缩帧、4bit 差分（`data::zip`）、结果块、拼接 |
| `event` | `0x16` 事件解析与 ACK 构造 |

## 快速使用

```rust
use gh_uprotocol::{Frame, DataDecoder};
use gh_uprotocol::cmd::Request;

// 组包
let bytes = Frame::new(0xA0, vec![0]).encode().unwrap(); // AA 11 A0 01 00 ..

// 流式拆包
let mut dec = FrameDecoder::new();
for f in dec.push(&bytes) { /* ... */ }

// 数据解码
let mut dd = DataDecoder::new();
dd.chnl_override = Some(2);          // 设备包头不含通道总数，可覆盖
for rec in dd.feed(&frame.payload).unwrap() { /* ... */ }
```

## 与设备行为对齐的几个点

1. **无参命令补 1 字节**：设备外层凑帧器拒绝 `LEN == 0`，`get_max_len()` 等已在库内补 `0x00`。
2. **启停极性**：`START_CTRL` 的 `payload[0]` 0 = 启动、1 = 停止，`Request::start_ctrl(true, ..)` 会编码成 0。
3. **绝对帧**：`package_type.bit1` 表示"本包首帧为绝对帧"，历史跨包保留。
4. **通道数**：包内只有通道掩码，库用"最高置位 + 1"推导，可用 `chnl_override` 覆盖。
5. **容错**：CRC 错/坏长度只计数不 panic；`DecoderStats` 可查。

## 测试

```bash
cargo test
```

覆盖 CRC 已知向量、帧往返、半帧/噪声重同步、命令 payload、zip 解码（含文档示例）。
