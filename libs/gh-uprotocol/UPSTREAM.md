# 上游来源

本 crate 从以下仓库整体迁入，保持字节级行为一致：

- 仓库：`l-evk-t2-gh3020`
- 路径：`client/protocol/`
- 基线 commit：`2eda9f4`（`fix(data): derive channel count from bitmap popcount; variadic CSV width`）
- 迁入日期：2026-11-05

## 同步约定

1. 上游改动先在本 crate 落地，再同步回上游，避免双份实现漂移。
2. 不修改 `Frame` / `FrameDecoder` / `DataDecoder` / `cmd::*` 的公开签名；确需修改时在本文档登记原因。
3. 只做编解码，不引入 I/O；传输与业务逻辑放在 `src-tauri/src/gh3220/`。
4. 上游 `uprotocol-host/`（CLI、C 模拟器、BLE 桥）不迁入本工程。

## 已知行为（勿“优化”）

- 无参命令补 1 字节，设备拒绝 `LEN == 0`
- `START_CTRL`：0 = 启动，1 = 停止
- 通道数按掩码 popcount，不是最高位 + 1
- `package_type.bit1` 表示本包首帧为绝对帧，跨包保留
- 每功能首帧可能多一个畸形 `0x2C` 包，重同步丢弃属预期
- `0x16` 事件必须 ACK，否则设备每 500ms 重传
- CRC 错/坏长度只计数，不 panic

## 许可证

沿用上游 crate 声明：`MIT OR Apache-2.0`。合入产品前需由负责人确认版权归属与署名方式。
