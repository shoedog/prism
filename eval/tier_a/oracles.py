"""Oracle adapters (spec §2.2).

Pure LSP-JSON->schema mapping (unit-tested) plus server lifecycle (live only).
SymbolKind: Method=6, Constructor=9, Function=12.
"""
from __future__ import annotations

import os
import subprocess
import time
import urllib.parse
from pathlib import PurePosixPath

from .model import CallEdge, DefTarget, FunctionDef, Location, from_lsp_line

SYMBOL_KIND = {6: "method", 9: "constructor", 12: "function"}

PROBE_SOURCES = {
    "rust": (
        "probe_prism_eval.rs",
        "fn probe_callee() {}\nfn probe_caller() { probe_callee(); }\n",
    ),
    "go": (
        "probe_prism_eval.go",
        "package main\n\nfunc probeCallee() {}\nfunc probeCaller() { probeCallee() }\n",
    ),
    "python": (
        "probe_prism_eval.py",
        "def probe_callee():\n    pass\n\ndef probe_caller():\n    probe_callee()\n",
    ),
}


def uri_to_rel(uri: str, root: str) -> str | None:
    """file:// URI -> repo-relative POSIX path; None if outside root (§2.1)."""
    path = urllib.parse.unquote(urllib.parse.urlparse(uri).path)
    try:
        rel = os.path.relpath(path, root)
    except ValueError:
        return None
    if rel == ".." or rel.startswith("../"):
        return None
    return str(PurePosixPath(rel))


def _loc(file: str, rng: dict) -> Location:
    return Location(file, from_lsp_line(rng["start"]["line"]),
                    from_lsp_line(rng["end"]["line"]))


def _split_receiver(name: str | None, container: str | None) -> tuple:
    """gopls encodes Go method receivers in the symbol name —
    `(*AdminConfig).newAdminHandler` — with no container. Normalize to the bare
    method name + receiver-as-container so §2.4 name matching works."""
    if name and name.startswith("(") and ")." in name:
        recv, _, meth = name.partition(").")
        return meth, recv.lstrip("(").lstrip("*")
    return name, container


def map_document_symbols(file: str, syms: list[dict]) -> list[FunctionDef]:
    out: list[FunctionDef] = []

    def walk(nodes: list[dict], container: str | None) -> None:
        for n in nodes:
            kind = SYMBOL_KIND.get(n.get("kind"))
            if kind and "selectionRange" in n:
                name, cont = _split_receiver(n.get("name"), container)
                out.append(FunctionDef(
                    name=name, kind=kind, container=cont,
                    location=_loc(file, n["range"]),
                    selection_line=from_lsp_line(
                        n["selectionRange"]["start"]["line"]),
                    selection_char=n["selectionRange"]["start"]["character"],
                ))
            walk(n.get("children", []), n.get("name"))

    walk(syms, None)
    # Flat SymbolInformation fallback: location.range plus containerName.
    for n in syms:
        if "location" in n and SYMBOL_KIND.get(n.get("kind")):
            loc = _loc(file, n["location"]["range"])
            out.append(FunctionDef(
                name=n.get("name"), kind=SYMBOL_KIND[n["kind"]],
                container=n.get("containerName"), location=loc,
                selection_line=loc.start_line,
            ))
    return out


def map_incoming(seed: FunctionDef, items: list[dict], root: str) -> list[CallEdge]:
    edges = []
    for it in items:
        frm = it["from"]
        f = uri_to_rel(frm["uri"], root)
        if f is None:
            continue
        other = _loc(f, frm["range"])
        for r in it.get("fromRanges", []):
            edges.append(CallEdge("caller", seed, other, frm.get("name"),
                                  _loc(f, r)))
    return edges


def map_outgoing(seed: FunctionDef, items: list[dict], root: str) -> list[CallEdge]:
    edges = []
    for it in items:
        to = it["to"]
        f = uri_to_rel(to["uri"], root)
        if f is None:
            continue
        other = _loc(f, to["range"])
        for r in it.get("fromRanges", []):
            # Outgoing fromRanges are positions in the seed body (LSP spec).
            edges.append(CallEdge("callee", seed, other, to.get("name"),
                                  _loc(seed.location.file, r)))
    return edges


def enrich_definitions(raw: list[dict], inventory: list[FunctionDef],
                       root: str) -> list[DefTarget]:
    """Location/LocationLink -> DefTarget, enriched by smallest containing span."""
    out = []
    for d in raw:
        uri = d.get("uri") or d.get("targetUri")
        rng = d.get("range") or d.get("targetSelectionRange") or d.get("targetRange")
        f = uri_to_rel(uri, root)
        if f is None:
            continue
        loc = _loc(f, rng)
        within = [
            fd for fd in inventory
            if fd.location.file == f
            and fd.location.start_line <= loc.start_line <= fd.location.end_line
        ]
        best = min(within,
                   key=lambda fd: fd.location.end_line - fd.location.start_line,
                   default=None)
        out.append(DefTarget(loc, best.name if best else None,
                             best.kind if best else None))
    return out


class OracleError(Exception):
    """Per-probe oracle failure consumed by the accounting layer."""


class LspOracle:
    """Shared live LSP lifecycle and wrappers that normalize oracle failures."""

    def __init__(self, cmd, root, lang, settle_s=2.0, quiescence_cap_s=300.0):
        from .lsp_client import LspClient

        self.root, self.lang = os.path.abspath(root), lang
        self.not_quiescent = False
        self._cmd = cmd
        root_uri = "file://" + urllib.parse.quote(self.root)
        self.client = LspClient(cmd, cwd=self.root, root_uri=root_uri)
        self._settle, self._cap = settle_s, quiescence_cap_s

    def start(self):
        self.client.start()
        deadline = time.monotonic() + self._cap
        active: set = set()
        quiet_since = time.monotonic()
        while time.monotonic() < deadline:
            for n in self.client.drain_notifications():
                if n.get("method") == "$/progress":
                    tok = n["params"]["token"]
                    kind = n["params"]["value"].get("kind")
                    if kind == "begin":
                        active.add(tok)
                    elif kind == "end":
                        active.discard(tok)
                    quiet_since = time.monotonic()
            if not active and time.monotonic() - quiet_since >= self._settle:
                return
            time.sleep(0.1)
        self.not_quiescent = True

    def stop(self):
        self.client.stop()

    def did_open(self, rel_path, text=None):
        p = os.path.join(self.root, rel_path)
        content = text
        if content is None:
            with open(p, encoding="utf-8", errors="replace") as f:
                content = f.read()
        lang_id = {"rust": "rust", "go": "go", "python": "python"}[self.lang]
        self.client.notify("textDocument/didOpen", {"textDocument": {
            "uri": "file://" + urllib.parse.quote(p),
            "languageId": lang_id,
            "version": 1,
            "text": content,
        }})

    def capability_probe(self) -> bool:
        """Overlay a probe file and require call hierarchy incomingCalls support."""
        name, text = PROBE_SOURCES[self.lang]
        self.did_open(name, text)
        uri = "file://" + urllib.parse.quote(os.path.join(self.root, name))
        col = {"rust": 3, "go": 5, "python": 4}[self.lang]
        line = {"rust": 0, "go": 2, "python": 0}[self.lang]
        try:
            items = self.client.request(
                "textDocument/prepareCallHierarchy",
                {"textDocument": {"uri": uri},
                 "position": {"line": line, "character": col}},
            )
            if not items:
                return False
            calls = self.client.request("callHierarchy/incomingCalls",
                                        {"item": items[0]})
            return calls is not None and len(calls) >= 1
        except Exception:
            return False

    def _req(self, method, params, timeout=None):
        from .lsp_client import LspError

        try:
            r = self.client.request(method, params, timeout=timeout)
        except LspError as e:
            raise OracleError(f"{method}: {e}") from e
        if r is None:
            raise OracleError(f"{method}: null result")
        return r

    def _uri(self, rel_path):
        return "file://" + urllib.parse.quote(os.path.abspath(
            os.path.join(self.root, rel_path)))

    def document_symbols(self, rel_path):
        self.did_open(rel_path)
        syms = self._req("textDocument/documentSymbol",
                         {"textDocument": {"uri": self._uri(rel_path)}})
        return map_document_symbols(rel_path, syms)

    def _hierarchy_item(self, fd):
        items = self._req(
            "textDocument/prepareCallHierarchy",
            {"textDocument": {"uri": self._uri(fd.location.file)},
             "position": {"line": fd.selection_line - 1,
                          "character": fd.selection_char}},
        )
        if not items:
            raise OracleError(f"prepareCallHierarchy: no item for {fd.name}")
        return items[0]

    def callers(self, fd):
        items = self._req("callHierarchy/incomingCalls",
                          {"item": self._hierarchy_item(fd)})
        return map_incoming(fd, items, self.root)

    def callees(self, fd):
        items = self._req("callHierarchy/outgoingCalls",
                          {"item": self._hierarchy_item(fd)})
        return map_outgoing(fd, items, self.root)

    def definitions_at(self, inventory, rel_path, line, character):
        raw = self._req(
            "textDocument/definition",
            {"textDocument": {"uri": self._uri(rel_path)},
             "position": {"line": line - 1, "character": character}},
        )
        raw = raw if isinstance(raw, list) else [raw]
        return enrich_definitions(raw, inventory, self.root)

    def version(self):
        info = getattr(self.client, "server_info", {}) or {}
        if info.get("name"):
            return f"{info['name']} {info.get('version', '?')}"
        try:
            out = subprocess.run(
                [self._cmd[0], "--version"],
                capture_output=True, text=True, timeout=10,
            ).stdout.strip()
            return out.splitlines()[0] if out else self._cmd[0]
        except Exception:
            return self._cmd[0]
