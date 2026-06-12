"""Minimal JSON-RPC-over-stdio server for framing tests. Not an LSP."""
import json
import sys
import time


def read_msg(stdin):
    headers = {}
    while True:
        line = stdin.readline().decode()
        if line in ("\r\n", "\n", ""):
            break
        k, v = line.split(":", 1)
        headers[k.strip().lower()] = v.strip()
    n = int(headers["content-length"])
    return json.loads(stdin.read(n))


def write_msg(obj):
    body = json.dumps(obj).encode()
    sys.stdout.buffer.write(f"Content-Length: {len(body)}\r\n\r\n".encode() + body)
    sys.stdout.buffer.flush()


while True:
    msg = read_msg(sys.stdin.buffer)
    m, i = msg.get("method"), msg.get("id")
    if m == "initialize":
        write_msg({"jsonrpc": "2.0", "id": i, "result": {"capabilities": {}}})
    elif m == "test/echo":
        write_msg({"jsonrpc": "2.0", "id": i, "result": msg["params"]})
    elif m == "test/slow":
        time.sleep(2)
        write_msg({"jsonrpc": "2.0", "id": i, "result": "late"})
    elif m == "test/notifyme":
        write_msg({"jsonrpc": "2.0", "method": "test/notification", "params": {"k": 1}})
        write_msg({"jsonrpc": "2.0", "id": i, "result": "ok"})
    elif m == "test/error":
        write_msg({"jsonrpc": "2.0", "id": i, "error": {"code": -1, "message": "boom"}})
    elif m == "exit":
        break
