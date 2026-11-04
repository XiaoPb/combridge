"""本机窗口确认通道验收探针。

用途：模拟一个**没有声明 elicitation 能力**的客户端（WorkBuddy Connector 5.9.3 就是这种），
验证服务端是否改走 ComBridge 窗口弹确认框：

    客户端无 elicitation
        → 服务端 emit mcp-confirm-request
        → ComBridge 窗口弹 McpConfirmModal
        → 用户点确认/取消
        → resolve_mcp_confirm 回传
        → 工具继续执行或返回 CONFIRM_REQUIRED

脚本本身不能点窗口按钮，所以它有三种模式：

    wait      发起调用后一直等（默认 120s 超时）—— 用于配合人手点窗口，观察结果
    cancel    发起调用后，等 N 秒再直接放弃（验证超时路径，不需要人）
    timeout   同 cancel，但用更短的等待表明是"没人应答"

用法：
    python scripts/mcp/local_confirm_probe.py wait
    python scripts/mcp/local_confirm_probe.py cancel --after 5

前置：ComBridge 已启用 MCP Server 且档位为 standard。
探针目标是 `combridge_gh3036_factory_test(action="stop")` —— L3 但无副作用
（未运行时只会返回「产测未在运行」），无论确认与否都不会真正驱动硬件。
"""

import argparse
import json
import os
import sys
import threading
import time
import urllib.error
import urllib.request

EP = os.path.join(os.environ["LOCALAPPDATA"], "combridge", "mcp-endpoint.json")
ep = json.load(open(EP, encoding="utf-8"))
URL, TOKEN = ep["url"], ep["token"]
SID = {"v": None}


def _headers():
    h = {
        "Authorization": f"Bearer {TOKEN}",
        "Content-Type": "application/json",
        "Accept": "application/json, text/event-stream",
    }
    if SID["v"]:
        h["mcp-session-id"] = SID["v"]
    return h


def send(payload, timeout=200):
    req = urllib.request.Request(
        URL, data=json.dumps(payload).encode(), headers=_headers(), method="POST"
    )
    try:
        resp = urllib.request.urlopen(req, timeout=timeout)
    except urllib.error.HTTPError as e:
        return None, e.read().decode("utf-8", "replace")[:300]
    except Exception as e:  # noqa: BLE001
        return None, str(e)[:300]
    sid = resp.headers.get("mcp-session-id")
    if sid:
        SID["v"] = sid
    return resp, None


def read_json_response(resp):
    body = resp.read().decode("utf-8", "replace")
    for line in body.splitlines():
        if line.startswith("data:"):
            raw = line[5:].strip()
            if not raw:
                continue
            try:
                obj = json.loads(raw)
            except json.JSONDecodeError:
                continue
            if "result" in obj or "error" in obj:
                return obj
    return {"raw": body[:400]}


def initialize():
    """刻意不声明 elicitation —— 复现桌面宿主的能力缺口。"""
    resp, err = send(
        {
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-06-18",
                "capabilities": {},
                "clientInfo": {"name": "local-confirm-probe", "version": "0.1"},
            },
        }
    )
    if err:
        sys.exit(f"initialize 失败: {err}")
    out = read_json_response(resp)
    info = out.get("result", {}).get("serverInfo", {})
    print(f"initialize: {info.get('name')} {info.get('version')} / session={SID['v']}")
    # 通知已初始化（无状态 HTTP 下不强制，但有些实现要求）
    try:
        send({"jsonrpc": "2.0", "method": "notifications/initialized"})
    except Exception:  # noqa: BLE001
        pass


def call_factory_stop(owner="probe"):
    """owner 同时是授权 scope：换一个 owner 就会重新要求确认。"""
    return send(
        {
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/call",
            "params": {
                "name": "combridge_gh3036_factory_test",
                "arguments": {"action": "stop", "owner": owner},
            },
        }
    )


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("mode", choices=["wait", "cancel", "timeout"], nargs="?", default="wait")
    ap.add_argument("--after", type=float, default=0, help="cancel 模式下等多久再放弃")
    ap.add_argument("--owner", default="probe", help="产测 owner（= 授权 scope，换个值即绕过已有授权）")
    args = ap.parse_args()

    initialize()
    started = time.time()

    if args.mode == "wait":
        print("已发起 L3 调用，请到 ComBridge 窗口点确认/取消（最多 120s）…")
        resp, err = call_factory_stop(args.owner)
    else:
        # 后台发起，主线程跑一段就放弃，模拟"没人点窗口"
        box = {}

        def worker():
            box["result"] = call_factory_stop(args.owner)

        t = threading.Thread(target=worker, daemon=True)
        t0 = time.time()
        t.start()
        time.sleep(args.after)
        print(f"已发起 L3 调用并等待 {args.after:.0f}s（模拟无人应答），现在退出观察超时行为")
        sys.exit(0)

    elapsed = time.time() - started
    if err:
        sys.exit(f"调用失败: {err}")
    out = read_json_response(resp)
    print(f"耗时 {elapsed:.1f}s")
    print(json.dumps(out, ensure_ascii=False, indent=2))

    payload = json.dumps(out, ensure_ascii=False)
    if "CONFIRM_REQUIRED" in payload:
        if "超时" in payload:
            print("\n判定：✅ 走的是本机窗口通道，但没人应答（超时按拒绝，符合 fail-closed）")
        elif "窗口不可用" in payload:
            print("\n判定：⚠️ 两条通道都不可用 —— 检查 ComBridge 窗口是否开着")
        elif "未声明 elicitation" in payload and "本机" not in payload:
            print("\n判定：❌ 仍是旧行为（未落到本机通道），确认服务端代码是否已重启")
        else:
            print("\n判定：用户取消或拒绝（通道正常，符合预期）")
    elif "result" in out:
        print("\n判定：✅ 确认通过，工具已继续执行（本机窗口通道生效）")


if __name__ == "__main__":
    main()
