"""TypeScript's native tsserver oracle, using the same bounded framed transport.

tsserver accepts newline JSON and emits Content-Length responses. Protocol
line/offset are 1-based, end exclusive; selection_char remains 0-based here.
Automatic typing acquisition is disabled: this lane never acquires packages.
"""
from __future__ import annotations

import json
import os
import shutil
import time
from pathlib import Path

from .lsp_client import LspClient, LspError, LspServerError, LspTimeout
from .model import CallEdge, FunctionDef, Location
from .oracles import LspOracle, OracleError, OracleTimeout, enrich_definitions, uri_to_rel


class TsserverClient(LspClient):
    def start(self):
        begun = time.monotonic()
        self._spawn()
        self.request("configure", {"preferences": {"includePackageJsonAutoImports": "off"}},
                     timeout=max(0, getattr(self, "startup_timeout_s", self._timeout) - (time.monotonic() - begun)))

    def _encode(self, obj):
        return (json.dumps(obj) + "\n").encode()

    def _request_object(self, method, params, rid):
        return {"seq": rid or 0, "type": "request", "command": method, "arguments": params}

    def _dispatch(self, msg):
        if msg.get("type") == "response":
            slot = self._pending.get(msg.get("request_seq"))
            if slot is not None:
                slot["msg"] = msg
                slot["event"].set()
        else:
            self._notifications.append(msg)

    def _response(self, msg):
        if not msg.get("success"):
            raise LspServerError({"code": "tsserver", "message": msg.get("message")})
        return msg.get("body")


def _location(file, span):
    start, end = span["start"], span["end"]
    # An exclusive end at column 1 belongs to the preceding line.
    return Location(file, start["line"],
                    max(start["line"], end["line"] - (end["offset"] == 1)))


def _relative(file, root):
    return uri_to_rel(Path(file).as_uri(), root)


def map_ts_calls(seed, calls, root, direction):
    out = []
    for call in calls:
        item = call["from" if direction == "callers" else "to"]
        rel = _relative(item["file"], root)
        if rel is None:
            continue
        for span in call["fromSpans"]:
            site_file = rel if direction == "callers" else seed.location.file
            out.append(CallEdge(direction, seed, _location(rel, item["span"]),
                                item["name"], _location(site_file, span)))
    return out


class TsserverOracle(LspOracle):
    def __init__(self, cmd, root, lang, **timeouts):
        self.root, self.lang = os.path.abspath(root), lang
        self.not_quiescent = False
        self.client = TsserverClient(cmd, self.root,
            default_timeout=timeouts.get("query_timeout_s", 10.0),
            session_timeout=timeouts.get("oracle_budget_s", 600.0))
        self._cmd = cmd
        self._opened = set()
        self.client.startup_timeout_s = timeouts.get("startup_timeout_s", 60.0)
        self.oracle_filtered = []
        self._binding_census = {}

    def start(self):
        try:
            self.client.start()
        except LspTimeout as exc:
            raise OracleTimeout(f"tsserver startup: {exc}") from exc
        except OSError as exc:
            raise OracleError(f"tsserver launch: {exc}") from exc

    def capability_probe(self):
        # The real-symbol fallback avoids altering the pinned input project.
        return False

    def _open(self, rel):
        if rel not in self._opened:
            self._req("open", {"file": os.path.join(self.root, rel),
                               "projectRootPath": self.root}, allow_null=True)
            self._opened.add(rel)

    def _req(self, method, params, timeout=None, allow_null=False):
        try:
            result = self.client.request(method, params, timeout)
        except LspTimeout as exc:
            raise OracleTimeout(f"{method}: {exc}") from exc
        except LspError as exc:
            raise OracleError(f"{method}: {exc}") from exc
        if result is None and not allow_null:
            raise OracleError(f"{method}: null result")
        return result

    def document_symbols(self, rel):
        self._open(rel)
        tree = self._req("navtree", {"file": os.path.join(self.root, rel)})
        out = []
        def walk(node, container=None):
            # navtree reports arrow/function initializers as const/let/var or
            # property. prepareCallHierarchy distinguishes these from scalars.
            if node["kind"] in ("function", "method", "constructor", "const", "let", "var", "property"):
                sel = node.get("nameSpan")
                if sel:
                    args = {"file": os.path.join(self.root, rel),
                            "line": sel["start"]["line"], "offset": sel["start"]["offset"]}
                    try:
                        items = self._req("prepareCallHierarchy", args, allow_null=True)
                    except OracleError as exc:
                        if not isinstance(exc, OracleTimeout) and "No content available." in str(exc):
                            items = []  # explicit tsserver non-callable result
                        else:
                            raise
                    if isinstance(items, dict):
                        items = [items]
                    for item in items or []:
                        if (item["file"] != args["file"] or
                                item["selectionSpan"]["start"] != sel["start"] or
                                item["selectionSpan"]["end"] != sel["end"]):
                            continue  # imports/aliases are not definitions here
                        selection = item["selectionSpan"]["start"]
                        kind = {"method": "method", "constructor": "constructor"}.get(node["kind"], "function")
                        out.append(FunctionDef(item["name"], kind, container,
                            _location(rel, item["span"]), selection["line"], selection["offset"] - 1))
            for child in node.get("childItems", []):
                walk(child, node["text"] if node["kind"] not in ("module", "script") else None)
        walk(tree)
        return list(dict.fromkeys(out))

    def _query(self, fd, command):
        self._open(fd.location.file)
        args = {"file": os.path.join(self.root, fd.location.file),
                "line": fd.selection_line, "offset": fd.selection_char + 1}
        if not self._req("prepareCallHierarchy", args, allow_null=True):
            raise OracleError(f"prepareCallHierarchy: no item for {fd.name}")
        return self._req(command, args)

    def callers(self, fd):
        calls = self._query(fd, "provideCallHierarchyIncomingCalls")
        from .member_sample import syntax_census, definition_matches
        # Parse only seed files that actually have incoming sites. Cache syntax
        # identities for the immutable corpus during this oracle session.
        if calls and fd.location.file not in self._binding_census:
            self._binding_census[fd.location.file] = syntax_census(self, [fd.location.file])['bindings']
        identities = [d for d in self._binding_census.get(fd.location.file, [])
                      if (d['line'], d['character']) == (fd.selection_line, fd.selection_char)
                      and fd.location.start_line <= d['line'] <= fd.location.end_line]
        # Constructors and unsupported syntax retain the existing exact-token rule.
        if not identities:
            identities = [{'file': fd.location.file, 'line': fd.selection_line, 'character': fd.selection_char}]
        kept = []
        for call in calls:
            rel = _relative(call["from"]["file"], self.root)
            if rel is None:
                continue
            self._open(rel)
            for span in call["fromSpans"]:
                # Native spans start at the call-name token, already in UTF-16.
                defs = self._req("definition", {"file": call["from"]["file"],
                                "line": span["start"]["line"], "offset": span["start"]["offset"]})
                matches = [d for d in defs if any(definition_matches(identity,
                           {**d, 'file': _relative(d['file'], self.root)}) for identity in identities)]
                if matches:
                    kept.append({**call, "fromSpans": [span]})
                else:
                    self.oracle_filtered.append({"seed": f"{fd.location.file}:{fd.selection_line}",
                                                 "file": rel, "span": span, "definitions": defs})
        return map_ts_calls(fd, kept, self.root, "callers")

    def callees(self, fd):
        return map_ts_calls(fd, self._query(fd, "provideCallHierarchyOutgoingCalls"), self.root, "callees")

    def raw_definitions_at(self, rel_path, line, character):
        self._open(rel_path)
        return self._req("definition", {"file": os.path.join(self.root, rel_path),
                                        "line": line, "offset": character + 1})

    def definitions_at(self, inventory, rel_path, line, character):
        defs = self.raw_definitions_at(rel_path, line, character)
        def lsp_pos(pos):
            return {"line": pos["line"] - 1, "character": pos["offset"] - 1}
        raw = [{"uri": Path(d["file"]).as_uri(), "range": {
                "start": lsp_pos(d["start"]), "end": lsp_pos(d["end"])}} for d in defs]
        return enrich_definitions(raw, inventory, self.root)

    def version(self):
        executable = shutil.which(self._cmd[0])
        if executable:
            package = Path(executable).resolve().parents[1] / "package.json"
            if package.is_file():
                return "tsserver " + json.loads(package.read_text())["version"]
        return "tsserver unknown"
