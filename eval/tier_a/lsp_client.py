"""Stdlib JSON-RPC-over-stdio client (spec §2.2): Content-Length framing,
request/response correlation, per-request timeout, notification capture."""
from __future__ import annotations

import json
import os
import select
import subprocess
import threading
import time


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
                 root_uri: str | None = None, session_timeout: float = 600.0,
                 initialization_options: dict | None = None):
        # root_uri: live LSP servers (rust-analyzer/gopls/pyright) need workspace
        # context for documentSymbol/callHierarchy; the echo-server tests pass None.
        self._cmd, self._cwd, self._timeout = cmd, cwd, default_timeout
        self._root_uri = root_uri
        self._initialization_options = initialization_options
        self._session_timeout = session_timeout
        self.deadline: float | None = None
        self._dead = threading.Event()
        self._proc: subprocess.Popen | None = None
        self._next_id = 0
        self._id_lock = threading.Lock()
        self._lock = threading.Lock()
        self._pending: dict[int, dict] = {}      # id -> {"event", "result"/"error"}
        self._notifications: list[dict] = []

    def start(self) -> None:
        self._spawn()
        params = {"processId": None, "rootUri": self._root_uri,
                  "capabilities": {
                      "window": {"workDoneProgress": True},
                      "textDocument": {"documentSymbol": {
                          "hierarchicalDocumentSymbolSupport": True}},
                  }}
        if self._root_uri:
            params["workspaceFolders"] = [{"uri": self._root_uri, "name": "corpus"}]
        if self._initialization_options is not None:
            params["initializationOptions"] = self._initialization_options
        self.server_info = self.request("initialize", params).get("serverInfo", {})
        self.notify("initialized", {})

    def _spawn(self) -> None:
        self.deadline = time.monotonic() + self._session_timeout
        self._proc = subprocess.Popen(
            self._cmd, cwd=self._cwd, stdin=subprocess.PIPE,
            stdout=subprocess.PIPE, stderr=subprocess.DEVNULL)
        os.set_blocking(self._proc.stdin.fileno(), False)
        threading.Thread(target=self._reader, daemon=True).start()

    def stop(self) -> None:
        if self._proc and self._proc.poll() is None:
            try:
                self._write(self._request_object("exit", {}, None),
                            time.monotonic() + 0.1)
            except Exception:
                pass
            try:
                self._proc.wait(timeout=0.5)
            except subprocess.TimeoutExpired:
                self._proc.terminate()
                try:
                    self._proc.wait(timeout=0.5)
                except subprocess.TimeoutExpired:
                    self._proc.kill()
                    self._proc.wait(timeout=1)
        if self._proc:
            self._proc.stdin.close()

    def _encode(self, obj: dict) -> bytes:
        body = json.dumps(obj).encode()
        return f"Content-Length: {len(body)}\r\n\r\n".encode() + body

    def _deadline(self, timeout=None):
        now = time.monotonic()
        if self.deadline is not None and now >= self.deadline:
            raise LspTimeout("oracle session budget exhausted")
        return min(now + (self._timeout if timeout is None else timeout),
                   self.deadline if self.deadline is not None else float("inf"))

    def _write(self, obj: dict, deadline=None) -> None:
        deadline = self._deadline() if deadline is None else deadline
        frame = memoryview(self._encode(obj))
        if not self._lock.acquire(timeout=max(0, deadline - time.monotonic())):
            raise LspTimeout("oracle write lock timed out")
        try:
            while frame:
                remaining = deadline - time.monotonic()
                if remaining <= 0:
                    raise LspTimeout("oracle write timed out")
                if self._dead.is_set() or self._proc.poll() is not None:
                    raise LspError("oracle process exited")
                if not select.select([], [self._proc.stdin], [], remaining)[1]:
                    raise LspTimeout("oracle write timed out")
                try:
                    n = os.write(self._proc.stdin.fileno(), frame)
                except BlockingIOError:
                    continue
                frame = frame[n:]
        except (OSError, ValueError) as exc:
            raise LspError(f"oracle write failed: {exc}") from exc
        finally:
            self._lock.release()

    def _reader(self) -> None:
        out = self._proc.stdout
        try:
            while True:
                headers = {}
                while True:
                    line = out.readline()
                    if not line:
                        return
                    if line in (b"\r\n", b"\n"):
                        if headers:
                            break
                        continue  # tsserver puts a newline after each payload
                    k, v = line.decode().split(":", 1)
                    headers[k.strip().lower()] = v.strip()
                self._dispatch(json.loads(out.read(int(headers["content-length"]))))
        except (OSError, ValueError, KeyError, LspError):
            return
        finally:
            self._dead.set()
            for slot in list(self._pending.values()):
                slot["event"].set()

    def _dispatch(self, msg):
        if "id" in msg and ("result" in msg or "error" in msg):
            slot = self._pending.get(msg["id"])
            if slot is not None:
                slot["msg"] = msg
                slot["event"].set()
        elif "id" in msg and "method" in msg:
            self._write({"jsonrpc": "2.0", "id": msg["id"], "result": None})
            self._notifications.append(msg)
        else:
            self._notifications.append(msg)

    def _request_object(self, method, params, rid):
        obj = {"jsonrpc": "2.0", "method": method, "params": params}
        if rid is not None:
            obj["id"] = rid
        return obj

    def _response(self, msg):
        if "error" in msg:
            raise LspServerError(msg["error"])
        return msg["result"]

    def request(self, method: str, params: dict, timeout: float | None = None):
        deadline = self._deadline(timeout)
        with self._id_lock:
            self._next_id += 1
            rid = self._next_id
        slot = {"event": threading.Event(), "msg": None}
        self._pending[rid] = slot
        try:
            self._write(self._request_object(method, params, rid), deadline)
            if self._dead.is_set():
                raise LspError("oracle process exited or sent malformed framing")
            if not slot["event"].wait(max(0, deadline - time.monotonic())):
                raise LspTimeout(f"{method} timed out")
            if slot["msg"] is None:
                raise LspError("oracle process exited or sent malformed framing")
            return self._response(slot["msg"])
        finally:
            self._pending.pop(rid, None)

    def notify(self, method: str, params: dict) -> None:
        self._write(self._request_object(method, params, None))

    def drain_notifications(self) -> list[dict]:
        out, self._notifications = self._notifications, []
        return out
