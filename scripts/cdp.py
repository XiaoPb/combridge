# -*- coding: utf-8 -*-
"""CDP 驱动工具: 连接 WebView2 远程调试端口, 操作页面/截图/取值。

用法:
    python scripts/cdp.py list                     # 列出调试目标
    python scripts/cdp.py eval "<js>"              # 在页面执行 JS, 返回结果
    python scripts/cdp.py evalf file.js            # 执行 JS 文件
    python scripts/cdp.py shot out.png             # 截图
"""
import asyncio
import base64
import json
import sys
import urllib.request

import websockets

DEBUG_HOST = "http://127.0.0.1:9222"


def list_targets():
    with urllib.request.urlopen(f"{DEBUG_HOST}/json") as r:
        return json.load(r)


def find_page():
    """选中 ComBridge 应用页，而不是 DevTools 的调试页面。

    WebView2 会把 `devtools://` 页面也列为 type=page；旧实现按遍历顺序取第一个，
    在应用开着 DevTools 时会连到 DevTools 页，导致 eval 操作不到应用 DOM。
    """
    targets = list_targets()
    for t in targets:
        if (
            t.get("type") == "page"
            and t.get("title") == "ComBridge"
            and str(t.get("url", "")).startswith("http")
        ):
            return t
    # 回退：第一个 http(s) 页面（排除 devtools:// / chrome-extension:// 等）
    for t in targets:
        if t.get("type") == "page" and str(t.get("url", "")).startswith("http"):
            return t
    raise RuntimeError("no page target found")


class CDP:
    def __init__(self, ws_url):
        self.ws_url = ws_url
        self._id = 0

    async def __aenter__(self):
        self.ws = await websockets.connect(self.ws_url, max_size=50 * 1024 * 1024)
        return self

    async def __aexit__(self, *exc):
        await self.ws.close()

    async def cmd(self, method, **params):
        self._id += 1
        mid = self._id
        await self.ws.send(json.dumps({"id": mid, "method": method, "params": params}))
        while True:
            msg = json.loads(await self.ws.recv())
            if msg.get("id") == mid:
                if "error" in msg:
                    raise RuntimeError(f"{method}: {msg['error']}")
                return msg.get("result", {})

    async def eval(self, expr):
        res = await self.cmd(
            "Runtime.evaluate",
            expression=expr,
            returnByValue=True,
            awaitPromise=True,
        )
        if res.get("exceptionDetails"):
            detail = res["exceptionDetails"]
            text = detail.get("exception", {}).get("description") or detail.get("text")
            raise RuntimeError(f"eval exception: {text}")
        return res.get("result", {}).get("value")

    async def screenshot(self, path):
        res = await self.cmd("Page.captureScreenshot", format="png")
        with open(path, "wb") as f:
            f.write(base64.b64decode(res["data"]))
        print(f"saved: {path}")


async def main():
    action = sys.argv[1]
    target = find_page()
    async with CDP(target["webSocketDebuggerUrl"]) as cdp:
        await cdp.cmd("Page.enable")
        await cdp.cmd("Runtime.enable")
        if action == "list":
            print(json.dumps(list_targets(), indent=2, ensure_ascii=False))
        elif action == "eval":
            print(await cdp.eval(sys.argv[2]))
        elif action == "evalf":
            with open(sys.argv[2], encoding="utf-8") as f:
                print(await cdp.eval(f.read()))
        elif action == "shot":
            await cdp.screenshot(sys.argv[2])
        else:
            raise SystemExit(f"unknown action: {action}")


if __name__ == "__main__":
    asyncio.run(main())
