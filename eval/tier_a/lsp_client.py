"""Stdlib JSON-RPC-over-stdio client (spec §2.2): Content-Length framing,
request/response correlation, per-request timeout, notification capture."""
from __future__ import annotations

import json
import subprocess
import threading


class LspError(Exception):
    pass


class LspTimeout(LspError):
    pass


class LspServerError(LspError):
    def __init__(self, err: dict):
        super().__init__(f"server error {err.get('code')}: {err.get('message')}")
        self.err = err


class LspClient:
    def __init__(self, cmd: list[str], cwd: str, default_timeout: float = 30.0,
                 root_uri: str | None = None):
        # root_uri: live LSP servers (rust-analyzer/gopls/pyright) need workspace
        # context for documentSymbol/callHierarchy; the echo-server tests pass None.
        self._cmd, self._cwd, self._timeout = cmd, cwd, default_timeout
        self._root_uri = root_uri
        self._proc: subprocess.Popen | None = None
        self._next_id = 0
        self._lock = threading.Lock()
        self._pending: dict[int, dict] = {}      # id -> {"event", "result"/"error"}
        self._notifications: list[dict] = []

    def start(self) -> None:
        self._proc = subprocess.Popen(
            self._cmd, cwd=self._cwd, stdin=subprocess.PIPE,
            stdout=subprocess.PIPE, stderr=subprocess.DEVNULL)
        threading.Thread(target=self._reader, daemon=True).start()
        params = {"processId": None, "rootUri": self._root_uri,
                  "capabilities": {
                      "window": {"workDoneProgress": True},
                      # hierarchical symbols carry selectionRange (name token);
                      # flat SymbolInformation ranges start at doc comments and
                      # systematically break §2.4 selection-line matching
                      "textDocument": {"documentSymbol": {
                          "hierarchicalDocumentSymbolSupport": True}},
                  }}
        if self._root_uri:
            params["workspaceFolders"] = [{"uri": self._root_uri, "name": "corpus"}]
        self.server_info = self.request("initialize", params).get("serverInfo", {})
        self.notify("initialized", {})

    def stop(self) -> None:
        if self._proc and self._proc.poll() is None:
            try:
                self.notify("exit", {})
            except Exception:
                pass
            self._proc.wait(timeout=5)

    def _write(self, obj: dict) -> None:
        body = json.dumps(obj).encode()
        frame = f"Content-Length: {len(body)}\r\n\r\n".encode() + body
        with self._lock:
            self._proc.stdin.write(frame)
            self._proc.stdin.flush()

    def _reader(self) -> None:
        out = self._proc.stdout
        while True:
            headers = {}
            while True:
                line = out.readline()
                if not line:
                    return
                if line in (b"\r\n", b"\n"):
                    break
                k, v = line.decode().split(":", 1)
                headers[k.strip().lower()] = v.strip()
            msg = json.loads(out.read(int(headers["content-length"])))
            if "id" in msg and ("result" in msg or "error" in msg):
                slot = self._pending.get(msg["id"])
                if slot is not None:
                    slot["msg"] = msg
                    slot["event"].set()
            elif "id" in msg and "method" in msg:
                # server->client REQUEST (e.g. window/workDoneProgress/create,
                # workspace/configuration). It needs a response or the server
                # stalls its progress pipeline — acknowledge with null and let
                # the notification log keep a record.
                self._write({"jsonrpc": "2.0", "id": msg["id"], "result": None})
                self._notifications.append(msg)
            else:
                self._notifications.append(msg)

    def request(self, method: str, params: dict, timeout: float | None = None):
        with self._lock:
            self._next_id += 1
            rid = self._next_id
        slot = {"event": threading.Event(), "msg": None}
        self._pending[rid] = slot
        self._write({"jsonrpc": "2.0", "id": rid, "method": method, "params": params})
        if not slot["event"].wait(timeout or self._timeout):
            self._pending.pop(rid, None)
            raise LspTimeout(f"{method} timed out")
        msg = self._pending.pop(rid)["msg"]
        if "error" in msg:
            raise LspServerError(msg["error"])
        return msg["result"]

    def notify(self, method: str, params: dict) -> None:
        self._write({"jsonrpc": "2.0", "method": method, "params": params})

    def drain_notifications(self) -> list[dict]:
        out, self._notifications = self._notifications, []
        return out
