#!/usr/bin/env python3
"""Minimal MCP stdio JSON-RPC client for integration tests (newline-delimited)."""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
from typing import Any


def send(proc: subprocess.Popen[str], msg: dict[str, Any]) -> None:
    assert proc.stdin is not None
    proc.stdin.write(json.dumps(msg, separators=(",", ":")) + "\n")
    proc.stdin.flush()


def recv(proc: subprocess.Popen[str]) -> dict[str, Any]:
    assert proc.stdout is not None
    while True:
        line = proc.stdout.readline()
        if line == "":
            raise RuntimeError("MCP server closed stdout before response")
        line = line.strip()
        if not line:
            continue
        return json.loads(line)


def rpc(
    proc: subprocess.Popen[str],
    method: str,
    params: Any | None = None,
    req_id: int = 1,
) -> dict[str, Any]:
    msg: dict[str, Any] = {"jsonrpc": "2.0", "id": req_id, "method": method}
    if params is not None:
        msg["params"] = params
    send(proc, msg)
    return recv(proc)


def notify(proc: subprocess.Popen[str], method: str, params: Any | None = None) -> None:
    msg: dict[str, Any] = {"jsonrpc": "2.0", "method": method}
    if params is not None:
        msg["params"] = params
    send(proc, msg)


def expect_result(resp: dict[str, Any], label: str) -> Any:
    if "error" in resp and resp["error"] is not None:
        raise AssertionError(f"{label}: JSON-RPC error: {resp['error']}")
    if "result" not in resp:
        raise AssertionError(f"{label}: missing result: {resp}")
    return resp["result"]


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", nargs="+", help="MCP server argv")
    parser.add_argument(
        "--expect-tool",
        action="append",
        default=[],
        help="Tool name that must appear in tools/list (repeatable)",
    )
    parser.add_argument(
        "--call",
        action="append",
        default=[],
        metavar="NAME[:JSON_ARGS]",
        help="tools/call NAME with optional JSON args object",
    )
    parser.add_argument("--cwd", default=None, help="Working directory for the server process")
    args = parser.parse_args()

    proc = subprocess.Popen(
        args.command,
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        cwd=args.cwd,
    )
    try:
        init = expect_result(
            rpc(
                proc,
                "initialize",
                {
                    "protocolVersion": "2024-11-05",
                    "capabilities": {},
                    "clientInfo": {"name": "impetus-ext-integration", "version": "0.1.0"},
                },
                req_id=1,
            ),
            "initialize",
        )
        if "serverInfo" not in init and "protocolVersion" not in init:
            raise AssertionError(f"initialize: unexpected payload: {init}")
        notify(proc, "notifications/initialized", {})

        listed = expect_result(rpc(proc, "tools/list", {}, req_id=2), "tools/list")
        tools = listed.get("tools") if isinstance(listed, dict) else None
        if not isinstance(tools, list) or not tools:
            raise AssertionError(f"tools/list: expected non-empty tools, got {listed}")
        names = {t.get("name") for t in tools if isinstance(t, dict)}
        for need in args.expect_tool:
            if need not in names:
                raise AssertionError(
                    f"tools/list missing `{need}`; have {sorted(n for n in names if n)}"
                )

        next_id = 3
        for call_spec in args.call:
            if ":" in call_spec:
                name, raw = call_spec.split(":", 1)
                call_args = json.loads(raw) if raw else {}
            else:
                name, call_args = call_spec, {}
            called = expect_result(
                rpc(
                    proc,
                    "tools/call",
                    {"name": name, "arguments": call_args},
                    req_id=next_id,
                ),
                f"tools/call {name}",
            )
            next_id += 1
            if isinstance(called, dict) and called.get("isError") is True:
                raise AssertionError(f"tools/call {name} returned isError: {called}")

        print("OK: initialize + tools/list" + (" + tools/call" if args.call else ""))
        print("tools:", ", ".join(sorted(n for n in names if n)))
        return 0
    finally:
        try:
            if proc.stdin:
                proc.stdin.close()
        except Exception:
            pass
        proc.terminate()
        try:
            proc.wait(timeout=5)
        except subprocess.TimeoutExpired:
            proc.kill()


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception as exc:  # noqa: BLE001
        print(f"FAIL: {exc}", file=sys.stderr)
        raise SystemExit(1) from exc
