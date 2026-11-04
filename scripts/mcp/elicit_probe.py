"""MCP elicitation 验收探针。

用途：在服务端处于 standard 档时，验证「客户端声明 elicitation 能力 →
服务端发起 elicitation/create → 客户端回应 → 工具按回应继续或中止」这条往返链路。

与普通冒烟脚本的区别：必须**边读 SSE 边回应**服务端发起的请求，
不能用 `urlopen().read()` 一次性读完，否则会在等待响应时死锁。

用法：
    python scripts/mcp/elicit_probe.py accept     # 勾选确认 -> 工具继续执行
    python scripts/mcp/elicit_probe.py decline    # 点拒绝   -> CONFIRM_REQUIRED
    python scripts/mcp/elicit_probe.py cancel     # 关弹窗   -> CONFIRM_REQUIRED
    python scripts/mcp/elicit_probe.py unchecked  # 提交但未勾选 -> CONFIRM_REQUIRED

前置：
    - ComBridge 已启用 MCP Server，且档位为 standard（dev 档按设计跳过确认）
    - 端点信息从 %LOCALAPPDATA%/combridge/mcp-endpoint.json 读取

说明：脚本用 `combridge_gh3036_factory_test(action="stop")` 作为探针目标 ——
它是 L3 操作但无副作用（未运行时只会返回「产测未在运行」），
因此无论确认与否都不会真正驱动硬件。
"""

import json
import os
import sys
import urllib.error
import urllib.request

EP = os.path.join(os.environ["LOCALAPPDATA"], "combridge", "mcp-endpoint.json")
ep = json.load(open(EP, encoding="utf-8"))
URL, TOKEN = ep["url"], ep["token"]
SID = {"v": None}
NID = [100]


def _headers():
    h = {
        "Authorization": f"Bearer {TOKEN}",
        "Content-Type": "application/json",
        "Accept": "application/json, text/event-stream",
    }
    if SID["v"]:
        h["mcp-session-id"] = SID["v"]
    return h


def send(payload):
    """POST 一条 JSON-RPC 消息，返回 (response|None, error_text)。"""
    req = urllib.request.Request(
        URL, data=json.dumps(payload).encode(), headers=_headers(), method="POST"
    )
    try:
        resp = urllib.request.urlopen(req, timeout=180)
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
    return {"raw": body[:300]}


def call(method, params=None):
    NID[0] += 1
    resp, err = send(
        {"jsonrpc": "2.0", "id": NID[0], "method": method, "params": params or {}}
    )
    if resp is None:
        return {"error": {"message": err}}
    out = read_json_response(resp)
    resp.close()
    return out


def call_tool_with_elicit(name, args, decide):
    """调用工具；服务端发来 elicitation/create 时用 decide() 的返回值回应。"""
    NID[0] += 1
    cid = NID[0]
    resp, err = send(
        {
            "jsonrpc": "2.0",
            "id": cid,
            "method": "tools/call",
            "params": {"name": name, "arguments": args},
        }
    )
    if resp is None:
        return {"error": {"message": err}}, []

    seen = []
    result = None
    for raw in resp:
        line = raw.decode("utf-8", "replace").strip()
        if not line.startswith("data:"):
            continue
        body = line[5:].strip()
        if not body:
            continue
        try:
            obj = json.loads(body)
        except json.JSONDecodeError:
            continue

        if obj.get("method") == "elicitation/create":
            params = obj.get("params", {})
            seen.append(params)
            print("  <- elicitation/create")
            print("     message:", (params.get("message") or "").replace("\n", " | "))
            print(
                "     schema :",
                json.dumps(params.get("requestedSchema"), ensure_ascii=False),
            )
            decision = decide()
            print("  -> client reply:", json.dumps(decision, ensure_ascii=False))
            r2, e2 = send({"jsonrpc": "2.0", "id": obj["id"], "result": decision})
            if r2 is not None:
                r2.read()
                r2.close()
            elif e2:
                print("     reply failed:", e2)
            continue

        if obj.get("id") == cid:
            result = obj
            break

    resp.close()
    return result or {"raw": "no response"}, seen


def payload_text(node):
    res = (node or {}).get("result") or {}
    text = ""
    content = res.get("content") or []
    if content:
        text = content[0].get("text", "")
    try:
        return res.get("isError", False), json.loads(text) if text else text
    except json.JSONDecodeError:
        return res.get("isError", False), text


def main():
    mode = sys.argv[1] if len(sys.argv) > 1 else "accept"

    init = call(
        "initialize",
        {
            "protocolVersion": "2025-06-18",
            "capabilities": {"elicitation": {}},
            "clientInfo": {"name": "elicit-probe", "version": "1"},
        },
    )
    print("initialize:", ((init.get("result") or {}).get("serverInfo") or {}).get("name"))

    s = send({"jsonrpc": "2.0", "method": "notifications/initialized", "params": {}})
    if s[0] is not None:
        s[0].read()
        s[0].close()

    _, about = payload_text(
        call("tools/call", {"name": "combridge_about", "arguments": {}})
    )
    print("tier =", about.get("tier") if isinstance(about, dict) else about)

    def decide():
        if mode == "accept":
            return {"action": "accept", "content": {"confirm": True}}
        if mode == "decline":
            return {"action": "decline"}
        if mode == "cancel":
            return {"action": "cancel"}
        return {"action": "accept", "content": {"confirm": False}}

    node, seen = call_tool_with_elicit(
        "combridge_gh3036_factory_test", {"action": "stop"}, decide
    )
    is_err, data = payload_text(node)
    print("result isError =", is_err)
    print("result payload =", json.dumps(data, ensure_ascii=False)[:400])
    print("elicitations received =", len(seen))
    return 0


if __name__ == "__main__":
    sys.exit(main())
