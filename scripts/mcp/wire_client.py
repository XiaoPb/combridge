#!/usr/bin/env python3
"""把 localhost 上的 ComBridge MCP Server 接到 MCP 客户端（以 WorkBuddy 为例）。

> **通常不需要手动跑这个脚本。** 服务启动时会自己写这两份文件
> （`src-tauri/src/mcp/client_wire.rs`，设置页「集成」里的「自动接入客户端」开关控制），
> 用户侧只需要开个新会话。此脚本保留作兜底：其它机器、或客户端装在非默认目录时。

做的事只有两件，都是写文件：
  1. `<configDir>/mcp.json`           —— 加入（或更新）一条 `combridge` 的 http server
  2. `<configDir>/mcp-approvals.json` —— 预置信任记录，省掉客户端里"点 Trust"这一步

为什么要第 2 步：客户端首次连接第三方 MCP server 会弹确认框，信任结果按
`sha256(origin)::serverName` 存盘。combridge 的 token 已改为持久化（重启不变），
所以这两份配置写一次就长期有效；但改了监听端口（origin 变了）或轮换了 token
（先轮换再重跑本脚本）时需要重新执行。

为什么绕不开"开个新会话"：MCP 连接在会话启动时建立，运行中的会话不会热加载
新 server。这是客户端行为，不是 combridge 能控制的。

用法：
    python scripts/mcp/wire_client.py                # 默认写 WorkBuddy
    python scripts/mcp/wire_client.py --name my-mcp  # 换个 server 名
    python scripts/mcp/wire_client.py --dry-run      # 只打印将写入的内容
    python scripts/mcp/wire_client.py --remove       # 撤销（删配置 + 信任记录）
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import pathlib
import sys
import time
from urllib.parse import urlparse

ENDPOINT = pathlib.Path(os.environ.get("LOCALAPPDATA", "")) / "combridge" / "mcp-endpoint.json"


def config_dir() -> pathlib.Path:
    """与客户端保持一致的配置目录：环境变量优先，其次 ~/.workbuddy。"""
    for key in ("WORKBUDDY_CONFIG_DIR", "CODEBUDDY_CONFIG_DIR"):
        value = os.environ.get(key, "").strip()
        if value:
            return pathlib.Path(value)
    return pathlib.Path.home() / ".workbuddy"


def load_endpoint() -> dict:
    if not ENDPOINT.exists():
        sys.exit(f"找不到 {ENDPOINT}；先在 ComBridge 设置页启用 MCP 服务")
    endpoint = json.loads(ENDPOINT.read_text(encoding="utf-8"))
    if not endpoint.get("url") or not endpoint.get("token"):
        sys.exit(f"{ENDPOINT} 缺 url/token，服务可能没正常启动")
    return endpoint


def approval_key(url: str, name: str) -> str:
    """与客户端 computeTrustConfigHash + approvalKey 一致：sha256(origin)::name。"""
    parsed = urlparse(url)
    origin = f"{parsed.scheme}://{parsed.netloc}"
    return f"{hashlib.sha256(origin.encode()).hexdigest()}::{name}"


def read_json(path: pathlib.Path, default: dict) -> dict:
    if not path.exists():
        return default
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except json.JSONDecodeError:
        sys.exit(f"{path} 不是合法 JSON，先手工修好再跑（避免覆盖掉别的配置）")


def write_json(path: pathlib.Path, data: dict, dry_run: bool) -> None:
    text = json.dumps(data, ensure_ascii=False, indent=2) + "\n"
    if dry_run:
        print(f"--- {path}\n{text}")
        return
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text, encoding="utf-8")
    print(f"已写入 {path}")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--name", default="combridge", help="server 名（默认 combridge）")
    parser.add_argument("--dry-run", action="store_true")
    parser.add_argument("--remove", action="store_true")
    args = parser.parse_args()

    mcp_path = config_dir() / "mcp.json"
    approvals_path = config_dir() / "mcp-approvals.json"
    mcp = read_json(mcp_path, {"mcpServers": {}})
    mcp.setdefault("mcpServers", {})
    approvals = read_json(approvals_path, {})

    if args.remove:
        mcp["mcpServers"].pop(args.name, None)
        for key in [k for k in approvals if k.endswith(f"::{args.name}")]:
            approvals.pop(key)
        write_json(mcp_path, mcp, args.dry_run)
        write_json(approvals_path, approvals, args.dry_run)
        print(f"已移除 {args.name}；客户端下次启动不再加载它")
        return

    endpoint = load_endpoint()
    url, token = endpoint["url"], endpoint["token"]
    mcp["mcpServers"][args.name] = {
        "type": "http",
        "url": url,
        "headers": {"Authorization": f"Bearer {token}"},
    }
    key = approval_key(url, args.name)
    approvals[key] = int(time.time() * 1000)

    print(f"endpoint : {url} (tier={endpoint.get('tier')}, pid={endpoint.get('pid')})")
    print(f"token    : {token[:8]}…（与 combridge 配置文件一致，重启不变）")
    print(f"trust key: {key}")
    write_json(mcp_path, mcp, args.dry_run)
    write_json(approvals_path, approvals, args.dry_run)
    print("\n下一步：开一个新会话（MCP 连接在会话启动时建立），然后让它调用 combridge_about 验证。")


if __name__ == "__main__":
    main()
