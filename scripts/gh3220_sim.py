# -*- coding: utf-8 -*-
"""GH3220 UProtocol TCP 模拟器（替代 uprotocol-sim，用于无硬件 UI 联调）。

用法:
    python scripts/gh3220_sim.py [--listen 127.0.0.1:9000] [--rate 25]

行为:
- 监听 TCP，按 UProtocol 帧格式(AA 11 CMD LEN PAYLOAD CRC8)解析主机命令并应答;
- 收到 START_CTRL(start=0) 后按 rate Hz 持续发送 0x0B 非压缩数据帧
  (func1: 4 通道 + gs + agc; func6: 2 通道, 若 func mask 含 spo2);
- STOP_CTRL 后停止发送; 0x16 事件 ACK 不应答。
"""
import argparse
import math
import socket
import socketserver
import struct
import threading
import time

# ---------------- CRC8 (poly 0x07, init 0xFF) ----------------
_TBL = []
for _i in range(256):
    _c = _i
    for _ in range(8):
        _c = ((_c << 1) ^ 0x07) & 0xFF if _c & 0x80 else (_c << 1) & 0xFF
    _TBL.append(_c)


def crc8(data: bytes) -> int:
    c = 0xFF
    for b in data:
        c = _TBL[c ^ b]
    return c


def frame(cmd: int, payload: bytes) -> bytes:
    head = bytes([0xAA, 0x11, cmd, len(payload)]) + payload
    return head + bytes([crc8(head)])


# ---------------- 命令号 ----------------
CMD_REG_RW = 0x03
CMD_PACKAGE_TEST = 0x05
CMD_START_CTRL = 0x0C
CMD_WORK_MODE = 0x10
CMD_EVENT = 0x16
CMD_CHIP_CTRL = 0x17
CMD_GET_VER = 0x19
CMD_CONN_STA = 0x1A
CMD_GET_OPENED_FUNC = 0x25
CMD_GET_MAX_LEN = 0xA0
CMD_DEBUG_STATUS = 0xA2

VERSIONS = {
    0x01: b"GH(M)3X2X_DEMO_v1.6_SIM",
    0x0A: b"DEMO_SIM_1.0",
    0x0E: b"Health_Uprotocol_v0.4",
    0x10: b"v4.3.0.0",
    0x11: b"v12_ev04",
}


class DeviceState:
    def __init__(self, rate: float):
        self.lock = threading.Lock()
        self.all_func = 0
        self.opened_func = 0
        self.capturing = False
        self.rate = rate
        self.frame_id = 0
        self.t0 = time.time()
        self.report_id = 0

    def handle(self, cmd: int, payload: bytes) -> bytes | None:
        with self.lock:
            if cmd == CMD_GET_VER:
                ty = payload[0] if payload else 0x01
                s = VERSIONS.get(ty, b"unknown")
                return bytes([ty, len(s)]) + s
            if cmd == CMD_GET_MAX_LEN:
                return bytes([238])
            if cmd == CMD_GET_OPENED_FUNC:
                return bytes([238]) + self.opened_func.to_bytes(4, "big")
            if cmd == CMD_WORK_MODE:
                mode = payload[0] if payload else 0
                mask = int.from_bytes(payload[1:5], "little") if len(payload) >= 5 else 0
                self.all_func = mask
                self.opened_func = 0
                print(f"[sim] WORK_MODE mode={mode} mask=0x{mask:x}")
                return b"\x00"
            if cmd == CMD_START_CTRL:
                start = payload[0] == 0 if payload else False
                mask = int.from_bytes(payload[3:7], "little") if len(payload) >= 7 else 0
                if mask & ~self.all_func:
                    print(f"[sim] START_CTRL rejected mask=0x{mask:x} all=0x{self.all_func:x}")
                    return b"\x01"
                self.opened_func = mask if start else 0
                self.capturing = start
                if start:
                    self.frame_id = 0
                    self.t0 = time.time()
                print(f"[sim] START_CTRL start={start} mask=0x{mask:x}")
                return b"\x00"
            if cmd == CMD_CHIP_CTRL:
                ctrl = payload[0] if payload else 0
                if ctrl in (0x5A, 0xC2):  # 复位: 停采集
                    self.capturing = False
                    self.opened_func = 0
                return b"\x00"
            if cmd == CMD_CONN_STA:
                return b"\x01"
            if cmd == CMD_PACKAGE_TEST:
                return bytes(payload)
            if cmd == CMD_REG_RW:
                if payload and payload[0] == 0:  # 读
                    cnt = payload[1]
                    vals = b"".join(struct.pack(">H", 0x1234 + i) for i in range(cnt))
                    return bytes(payload[:4]) + vals
                return bytes(payload)  # 写: 回显
            if cmd == CMD_DEBUG_STATUS:
                return bytes([payload[0] if payload else 0, 1, 1, 0, 0, 0])
            if cmd == CMD_EVENT:
                return None  # ACK, 无响应
            return b""


def build_packet(state: DeviceState, func: int, ch_mask: int) -> bytes:
    """构造一个 0x0B 非压缩包, 含 1 条帧记录。"""
    t = time.time() - state.t0
    chn = bin(ch_mask).count("1")
    # gs: 轻微晃动
    gs = b"".join(
        struct.pack(">h", int(v * 100)) for v in (math.sin(t) * 0.1, math.cos(t) * 0.1, 0.98)
    )
    # raw: 正弦波, 不同通道相位差, 24bit 值
    raw = b""
    for i in range(chn):
        amp = 0x180000
        base = 0x200000
        val = int(base + amp * math.sin(2 * math.pi * (t * 1.5 + i * 0.25)))
        val = max(0, min(0xFFFFFF, val))
        raw += val.to_bytes(4, "big")
    # agc: 慢变, LE
    agc = b""
    for i in range(chn):
        v = int(0x40 + 0x10 * math.sin(0.5 * t + i)) & 0xFFFF
        agc += struct.pack("<I", v)
    body = bytes([state.frame_id & 0xFF]) + gs + raw + agc + b"\x00"  # result_len=0
    state.frame_id += 1
    header = bytes([func, 0x07]) + ch_mask.to_bytes(4, "big") + bytes([0x00, len(body)])
    payload = header + body
    return frame(0x0B, payload)


def stream_loop(conn: socket.socket, state: DeviceState):
    interval = 1.0 / state.rate
    packet_count = 0
    while True:
        if not state.capturing:
            time.sleep(0.05)
            continue
        with state.lock:
            mask = state.opened_func
        try:
            # func1 (hr): 4 通道, 掩码 0x33 (bit0/1/4/5, 与真机一致)
            if mask & (1 << 1):
                conn.sendall(build_packet(state, 1, 0x00000033))
                packet_count += 1
            # func6 (spo2): 2 通道
            if mask & (1 << 6):
                conn.sendall(build_packet(state, 6, 0x00000003))
                packet_count += 1
            # 每 50 包上报一次 0x16 事件（IRQ 状态大端 + reportId），等待主机 ACK
            if packet_count and packet_count % 50 == 0:
                with state.lock:
                    state.report_id = (state.report_id + 1) & 0xFF
                    rid = state.report_id
                payload = bytes([0x00, 0x08, rid])
                conn.sendall(frame(0x16, payload))
                print(f"[sim] -> event reportId={rid}")
        except OSError:
            return
        time.sleep(interval)


class Handler(socketserver.BaseRequestHandler):
    def setup(self):
        self.state: DeviceState = self.server.state
        self.buf = b""
        threading.Thread(target=stream_loop, args=(self.request, self.state), daemon=True).start()

    def handle(self):
        print(f"[sim] client connected: {self.client_address}")
        while True:
            try:
                chunk = self.request.recv(4096)
            except OSError:
                break
            if not chunk:
                break
            self.buf += chunk
            while True:
                f, self.buf = try_parse(self.buf)
                if f is None:
                    break
                cmd, payload = f
                print(f"[sim] <- cmd=0x{cmd:02X} len={len(payload)} {payload.hex()[:48]}")
                resp = self.state.handle(cmd, payload)
                if resp is not None:
                    try:
                        self.request.sendall(frame(cmd, resp))
                    except OSError:
                        return
        print("[sim] client disconnected")


def try_parse(buf: bytes):
    """返回 ((cmd, payload), rest) 或 (None, buf)。"""
    while True:
        idx = buf.find(b"\xaa\x11")
        if idx < 0:
            return None, b"" if len(buf) < 2 else buf[-1:]
        buf = buf[idx:]
        if len(buf) < 4:
            return None, buf
        ln = buf[3]
        total = 5 + ln
        if len(buf) < total:
            return None, buf
        frame_bytes, rest = buf[:total], buf[total:]
        if crc8(frame_bytes[:-1]) == frame_bytes[-1]:
            return (frame_bytes[2], frame_bytes[4:-1]), rest
        buf = buf[2:]  # CRC 错: 跳过帧头重试


class Server(socketserver.ThreadingTCPServer):
    allow_reuse_address = True
    daemon_threads = True


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--listen", default="127.0.0.1:9000")
    ap.add_argument("--rate", type=float, default=25.0)
    args = ap.parse_args()
    host, port = args.listen.rsplit(":", 1)
    srv = Server((host, int(port)), Handler)
    srv.state = DeviceState(args.rate)
    print(f"[sim] GH3220 uprotocol simulator listening on {args.listen}, rate={args.rate}Hz")
    try:
        srv.serve_forever()
    except KeyboardInterrupt:
        pass


if __name__ == "__main__":
    main()
