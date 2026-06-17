#!/usr/bin/env python3
"""Re-usable gopls interface-satisfaction oracle for prism Go dispatch sites (Phase-IP Slice E).

WHY THIS EXISTS
---------------
prism resolves Go interface-dispatch call sites (e.g. caddy's
``x.(caddy.Module).CaddyModule()``) by *minting an implementer set* — for a recovered
interface receiver it fans the call out to every in-repo type prism believes satisfies
that interface (RTA-pruned to live / constructed types). `prism nav interface-manifest`
emits, per dispatch site, that set as ``implementers`` (and ``fanout`` = its size).

The open question is *soundness*: is prism's minted set a subset of the types that
**actually** satisfy the interface, or does it over-approximate (mint a non-satisfier =
a false call edge / a precision bug)? The ground-truth oracle for "what truly satisfies
interface I" is gopls ``textDocument/implementation``. This tool queries gopls once per
unique ``(interface, method)`` and compares prism's per-site set to gopls's satisfier set.

THE TAXONOMY (per dispatch site)
--------------------------------
Let ``P`` = prism's minted implementer types, ``G`` = gopls's satisfier types.

- ``over_approx``  — ``P \\ G`` is non-empty. prism minted a type gopls says does NOT
                     satisfy the interface => a false edge => a **prism_fp candidate**.
                     ``prism_only_types`` lists the offenders. These are the sites the
                     controller's dual-adjudicator κ pass examines. Highest precedence.
- ``sound``        — ``P ⊆ G`` with ``P`` non-empty (and the trivial ``P = G = ∅``). Every
                     minted edge is a real satisfier. prism may be a STRICT subset of G
                     (RTA prunes to live/constructed types) — that is correct precision,
                     NOT a miss, and is STILL ``sound``. (This is the caddy ``CaddyModule``
                     verdict: prism's 121 ⊆ gopls's satisfier set => sound, the §4 answer.)
- ``recall_gap``   — ``P`` is EMPTY while ``G`` is non-empty: prism minted nothing for a
                     site that has real satisfiers — a genuine recall hole. Informational,
                     not a precision bug. (NB: the manifest only feeds fanout>0 sites, so on
                     real corpora ``P`` is non-empty and this is an edge-case label; the
                     strict-subset "prism under-covers but is sound" case is ``sound`` above,
                     and the ``gopls_only_types`` field still records what prism missed.)
- ``oracle_timeout`` — gopls did not answer for this site's ``(interface, method)`` group
                     within the timeout. Recorded, never scored (excluded from precision
                     and from the sound/over_approx/recall_gap tallies).

Precedence: ``over_approx`` (any false edge) > ``recall_gap`` (prism empty, gopls non-empty)
> ``sound``. A false edge is the precision-relevant verdict even when prism also under-covers.

THE GATE METRIC
---------------
``dispatch_precision = |P ∩ G| / |P|`` — summed over all scored (non-timeout) sites for
the overall figure, and per ``(interface, method)``. It is the §8 dispatch precision/recall
regression gate.

  **Baseline dispatch_precision must stay AT-OR-ABOVE on a re-run. A DECREASE is a
  deliberate, recorded decision (e.g. a refactor that trades precision for recall) — it
  is never silently accepted. Paste the new summary into the PR and explain the delta.**

A future baseline regenerates the manifest + re-runs this oracle on the same corpus SHA
and compares ``overall.dispatch_precision`` (and the over_approx site list) to the recorded
baseline. ``recall_gap`` is reported but does not gate (RTA pruning is by-design precision).

RE-RUN COMMAND (the gate)
-------------------------
From the repo root, regenerate the manifest with the current prism, then run the oracle::

    cargo build --release
    target/release/prism nav interface-manifest --repo ~/code/bench-repos/caddy \\
        > /tmp/caddy-manifest.json
    cd eval && uv run python tools/dispatch_oracle.py \\
        --manifest /tmp/caddy-manifest.json \\
        --repo ~/code/bench-repos/caddy \\
        --corpus caddy \\
        --out /tmp/caddy-dispatch-oracle.json

Any Go corpus in ``eval/corpora.toml`` works (``--corpus`` reads the oracle ``cmd`` from
there; with no ``--corpus`` the gopls on ``PATH`` is used). gopls can be slow on large
corpora — ``--group-timeout`` is generous by default; a group that still times out is
recorded as ``oracle_timeout`` rather than failing the run.

IMPLEMENTATION DETAILS
----------------------
- Reuses the harness LSP infra: ``tier_a.lsp_client.LspClient`` (Content-Length framing,
  request/response correlation) and ``tier_a.oracles`` helpers (``uri_to_rel``,
  ``_split_receiver`` for ``(*T).Method`` -> ``T``). It does NOT reinvent the client.
- gopls query — per site, cached by interface-method DECLARATION (two requests/site, the
  second cached): at each dispatch site's **method-name token**, ``textDocument/definition``
  resolves WHICH interface method the call dispatches on (its declaration location); then
  ``textDocument/implementation`` at that declaration returns the satisfiers, cached by the
  decl location. This is the def->impl path, chosen over a single per-group implementation
  query because prism's grouping (by implementer set) is NOT always gopls's interface: caddy
  ``next.ServeHTTP`` resolves to ``http.Handler.ServeHTTP`` at some sites and
  ``caddyhttp.Handler.ServeHTTP`` at others, so a single representative would query the wrong
  interface for the rest of the group. Implementation results point at concrete method defs;
  each is mapped to its receiver TYPE via ``documentSymbol`` + ``_split_receiver`` (smallest
  containing span). ``summarize`` still groups records by ``(method, implementer-set)`` for
  the per-(interface, method) rollup.
- The query column is computed against the ORIGINAL source line (``method_token_col``), NOT
  ``tier_a.spotcheck.find_call_position`` — that helper indexes a strings-stripped copy and
  would mis-position the gopls request on a line with a preceding string literal.
- Readiness: gopls returns an empty ``implementation`` list until it has type-checked the
  package. The tool warms every dispatch file (open + ``documentSymbol``) and resettles
  before scoring, and on an empty result for a fanout>0 method (which has >=1 minted impl,
  so a true zero is impossible) it resettles and retries once; a still-empty / errored query
  is recorded ``oracle_timeout`` (UNSCORED) — never a precision-0 ``over_approx``.
- The interface display label is the enclosing type at the decl (gopls), falling back to a
  ``x.(pkg.Iface).Method()`` type-assertion source label, then a synthetic ``<iface-of:..>``.
  The label is cosmetic — the rollup identity is the implementer set, never the label.
"""
from __future__ import annotations

import argparse
import json
import os
import re
import sys
import time
import tomllib
import urllib.parse
from pathlib import Path

# eval/ on sys.path so the tool runs both as `python tools/dispatch_oracle.py` and via
# the test's importlib path-load (it never imports tier_a at module import time for the
# pure-logic tests, only inside the live-run functions).
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

EVAL = Path(__file__).resolve().parents[1]
REPO = EVAL.parent


# ===========================================================================
# Pure logic (unit-tested in eval/tests/test_dispatch_oracle.py)
# ===========================================================================

def classify(prism_set: set[str], gopls_set: set[str]) -> str:
    """sound | over_approx | recall_gap (mutually exclusive), given prism's minted set and
    gopls's satisfier set.

    Precedence (see the module docstring): a false edge (prism\\gopls non-empty) is
    ``over_approx`` regardless of coverage; an EMPTY prism set against a non-empty gopls set
    is ``recall_gap`` (prism resolved nothing — a genuine recall hole); otherwise prism is a
    sound subset (strict-subset included — RTA pruning is correct precision) => ``sound``.
    """
    if prism_set - gopls_set:
        return "over_approx"
    if not prism_set and gopls_set:
        return "recall_gap"
    return "sound"


def dispatch_precision(prism_set: set[str], gopls_set: set[str]) -> float:
    """|prism ∩ gopls| / |prism|. Empty prism => 1.0 (no minted edges => no false edges)."""
    if not prism_set:
        return 1.0
    return len(prism_set & gopls_set) / len(prism_set)


def compare_site(
    file: str,
    line: int,
    interface: str,
    method: str,
    prism_set: set[str],
    gopls_set: set[str] | None,
) -> dict:
    """One per-site comparison record. ``gopls_set is None`` => the group timed out."""
    if gopls_set is None:
        return {
            "file": file,
            "line": line,
            "interface": interface,
            "method": method,
            "prism_implementers": sorted(prism_set),
            "gopls_satisfiers": None,
            "classification": "oracle_timeout",
            "prism_only_types": [],
            "gopls_only_types": [],
        }
    return {
        "file": file,
        "line": line,
        "interface": interface,
        "method": method,
        "prism_implementers": sorted(prism_set),
        "gopls_satisfiers": sorted(gopls_set),
        "classification": classify(prism_set, gopls_set),
        "prism_only_types": sorted(prism_set - gopls_set),
        "gopls_only_types": sorted(gopls_set - prism_set),
    }


def _precision_acc() -> dict:
    return {"inter": 0, "prism": 0}


def summarize(sites: list[dict]) -> dict:
    """Per-(interface, method) + overall rollup of compare_site records.

    dispatch_precision aggregates as (sum |P∩G|) / (sum |P|) over scored sites; an empty
    overall denominator is vacuously 1.0. oracle_timeout sites are excluded from precision
    and from the sound/over_approx/recall_gap tallies (only counted as oracle_timeout).
    """
    groups: dict[tuple, dict] = {}
    overall = {
        "sites": 0,
        "sound": 0,
        "over_approx": 0,
        "recall_gap": 0,
        "oracle_timeout": 0,
    }
    overall_acc = _precision_acc()
    over_approx_sites: list[dict] = []
    timeout_groups: dict[tuple, tuple[str, str]] = {}

    for s in sites:
        # Group identity is (method, the minted implementer set) — the same (iface_key,
        # method) minted that set, so this is the true (interface, method) pair. A method
        # declared on two interfaces (distinct sets) stays split; a single interface whose
        # sites carry different display labels stays merged. `interface` is the display
        # label of the first site in the group.
        key = (s["method"], tuple(s["prism_implementers"]))
        g = groups.setdefault(
            key,
            {
                "interface": s["interface"],
                "method": s["method"],
                "sites": 0,
                "sound": 0,
                "over_approx": 0,
                "recall_gap": 0,
                "oracle_timeout": 0,
                "_acc": _precision_acc(),
            },
        )
        cls = s["classification"]
        g["sites"] += 1
        g[cls] += 1
        overall["sites"] += 1
        overall[cls] += 1
        if cls == "oracle_timeout":
            timeout_groups[key] = (s["interface"], s["method"])
            continue
        prism = set(s["prism_implementers"])
        gopls = set(s["gopls_satisfiers"])
        inter = len(prism & gopls)
        g["_acc"]["inter"] += inter
        g["_acc"]["prism"] += len(prism)
        overall_acc["inter"] += inter
        overall_acc["prism"] += len(prism)
        if cls == "over_approx":
            over_approx_sites.append({
                "file": s["file"],
                "line": s["line"],
                "interface": s["interface"],
                "method": s["method"],
                "prism_only_types": s["prism_only_types"],
            })

    group_list = []
    for key in sorted(groups):
        g = groups[key]
        acc = g.pop("_acc")
        g["dispatch_precision"] = (
            acc["inter"] / acc["prism"] if acc["prism"] else 1.0
        )
        group_list.append(g)

    overall["dispatch_precision"] = (
        overall_acc["inter"] / overall_acc["prism"] if overall_acc["prism"] else 1.0
    )
    return {
        "overall": overall,
        "groups": group_list,
        "over_approx_sites": sorted(
            over_approx_sites, key=lambda r: (r["file"], r["line"])
        ),
        "oracle_timeout_groups": [
            {"interface": i, "method": m}
            for (i, m) in sorted(timeout_groups.values())
        ],
    }


# ===========================================================================
# Manifest + source helpers (no LSP)
# ===========================================================================

# Recover the interface name from a type-assertion call source: `x.(pkg.Iface).Method()`.
_ASSERT_RE = re.compile(r"\.\(\s*\*?\s*([A-Za-z_][\w.]*)\s*\)")


def method_token_col(line_text: str, method: str) -> int | None:
    """0-based column of `method` in CALL position (``.method(`` / ``method(``) in the
    ORIGINAL line — the position fed to gopls.

    NB: tier_a.spotcheck.find_call_position is deliberately NOT reused here: it computes
    its match offset against a strings/comments-STRIPPED copy of the line (it only ever
    needs presence, not an absolute column), so its index is wrong for a line with a
    preceding string literal — which silently mis-positioned the gopls query. We need the
    column in the real document, so we match the raw line directly.
    """
    for m in re.finditer(rf"(?:(?<=\.)|(?<=::)|\b){re.escape(method)}\s*\(", line_text):
        return m.start()
    m = re.search(rf"\b{re.escape(method)}\b", line_text)
    return m.start() if m else None


def load_dispatch_sites(manifest_path: str) -> list[dict]:
    """The fanout>0 sites of a `prism nav interface-manifest` document (Slice-E shape)."""
    doc = json.loads(Path(manifest_path).read_text())
    return [s for s in doc.get("sites", []) if s.get("fanout", 0) > 0]


def interface_label(line_text: str | None, method: str, ordinal: int) -> str:
    """Recover the interface name from a type-assertion call line; else a synthetic label.

    The label is display-only; group identity is the implementer set, not this string.
    """
    if line_text:
        m = _ASSERT_RE.search(line_text)
        if m:
            return m.group(1)
    return f"<iface-of:{method}/{ordinal}>"


# ===========================================================================
# Live gopls oracle (integration; smoke-run, not unit-tested)
# ===========================================================================

def _settle(client, cap_s: float, settle_s: float) -> None:
    deadline = time.monotonic() + cap_s
    active: set = set()
    quiet = time.monotonic()
    while time.monotonic() < deadline:
        for n in client.drain_notifications():
            if n.get("method") == "$/progress":
                v = n["params"]["value"]
                tok = n["params"]["token"]
                if v.get("kind") == "begin":
                    active.add(tok)
                elif v.get("kind") == "end":
                    active.discard(tok)
                quiet = time.monotonic()
        if not active and time.monotonic() - quiet >= settle_s:
            return
        time.sleep(0.1)


class GoplsSatisfiers:
    """Thin live-gopls wrapper: open files, query textDocument/implementation, map results
    to receiver TYPE names via documentSymbol + _split_receiver (smallest containing span)."""

    def __init__(self, repo: str, cmd: list[str], group_timeout: float,
                 settle_s: float = 5.0, cap_s: float = 300.0):
        from tier_a.lsp_client import LspClient

        self.root = os.path.abspath(repo)
        self.group_timeout = group_timeout
        self._settle_s, self._cap_s = settle_s, cap_s
        root_uri = "file://" + urllib.parse.quote(self.root)
        self.client = LspClient(cmd, cwd=self.root, root_uri=root_uri,
                                default_timeout=group_timeout)
        self._opened: set[str] = set()
        self._docsym: dict[str, list[tuple]] = {}

    def start(self) -> None:
        self.client.start()
        _settle(self.client, self._cap_s, self._settle_s)

    def resettle(self, settle_s: float | None = None) -> None:
        """Drain progress and wait for quiescence again (used to retry an empty result —
        gopls may not have type-checked the package the first time)."""
        _settle(self.client, self._cap_s, settle_s or self._settle_s)

    def stop(self) -> None:
        stop = getattr(self.client, "stop", None)
        if stop:
            stop()

    def _uri(self, rel: str) -> str:
        return "file://" + urllib.parse.quote(os.path.join(self.root, rel))

    def _did_open(self, rel: str) -> bool:
        if rel in self._opened:
            return True
        p = os.path.join(self.root, rel)
        if not os.path.exists(p):
            return False
        text = Path(p).read_text(encoding="utf-8", errors="replace")
        self.client.notify("textDocument/didOpen", {"textDocument": {
            "uri": self._uri(rel), "languageId": "go", "version": 1, "text": text}})
        self._opened.add(rel)
        return True

    def _methods(self, rel: str) -> list[tuple]:
        """[(name, container, start_line0, end_line0)] for methods/funcs in `rel`."""
        if rel in self._docsym:
            return self._docsym[rel]
        if not self._did_open(rel):
            self._docsym[rel] = []
            return []
        from tier_a.oracles import _split_receiver

        try:
            syms = self.client.request(
                "textDocument/documentSymbol",
                {"textDocument": {"uri": self._uri(rel)}},
                timeout=self.group_timeout,
            )
        except Exception:
            syms = []
        out: list[tuple] = []

        def walk(nodes, container):
            for nd in nodes or []:
                name, cont = _split_receiver(nd.get("name"), container)
                rng = nd.get("range", {})
                s = rng.get("start", {}).get("line")
                e = rng.get("end", {}).get("line")
                if s is not None and e is not None:
                    out.append((name, cont, s, e))
                walk(nd.get("children", []), nd.get("name"))

        walk(syms, None)
        self._docsym[rel] = out
        return out

    def _type_at(self, rel: str, line0: int) -> str | None:
        """Receiver type of the smallest method/func span containing `line0`."""
        best = None
        for _name, cont, s, e in self._methods(rel):
            if s <= line0 <= e:
                span = e - s
                if best is None or span < best[0]:
                    best = (span, cont)
        return best[1] if best else None

    def method_decl(self, rel: str, line0: int, char0: int) -> tuple[str, int, int] | None:
        """textDocument/definition at a call's method token -> the interface method's
        declaration site (rel_file, decl_line0, decl_char0), or None.

        This is the gopls-side disambiguator: two call sites that prism lumps into one
        implementer-set group can resolve to DIFFERENT interface declarations (caddy's
        ``next.ServeHTTP`` is ``http.Handler.ServeHTTP`` at one site and
        ``caddyhttp.Handler.ServeHTTP`` at another). Keying the implementation cache by the
        decl location, not by prism's group, queries the interface gopls actually sees."""
        from tier_a.lsp_client import LspError
        from tier_a.oracles import uri_to_rel

        if not self._did_open(rel):
            return None
        try:
            raw = self.client.request(
                "textDocument/definition",
                {"textDocument": {"uri": self._uri(rel)},
                 "position": {"line": line0, "character": char0}},
                timeout=self.group_timeout,
            )
        except LspError:
            return None
        for d in (raw if isinstance(raw, list) else [raw] if raw else []):
            uri = d.get("uri") or d.get("targetUri")
            rng = d.get("range") or d.get("targetSelectionRange") or d.get("targetRange")
            f = uri_to_rel(uri, self.root) if uri else None
            if f is None or rng is None:
                continue
            return (f, rng["start"]["line"], rng["start"]["character"])
        return None

    def satisfier_types(self, rel: str, line0: int, char0: int) -> tuple[set[str], int] | None:
        """gopls textDocument/implementation at (rel, line0, char0) -> (receiver TYPE name
        set, raw result count). None on timeout/error.

        The raw count lets the caller distinguish "gopls returned implementation locations
        but we could not map them to a container type" (a mapping gap, count>0) from "gopls
        returned an empty list" (count==0, almost always a not-ready artifact for an
        interface method — see run_oracle's retry)."""
        from tier_a.lsp_client import LspError
        from tier_a.oracles import uri_to_rel

        if not self._did_open(rel):
            return None
        try:
            results = self.client.request(
                "textDocument/implementation",
                {"textDocument": {"uri": self._uri(rel)},
                 "position": {"line": line0, "character": char0}},
                timeout=self.group_timeout,
            )
        except LspError:
            return None
        results = results or []
        types: set[str] = set()
        for it in results:
            f = uri_to_rel(it["uri"], self.root)
            if f is None:
                continue
            t = self._type_at(f, it["range"]["start"]["line"])
            if t:
                types.add(t)
        return types, len(results)


def make_cmd(corpus: str | None) -> list[str]:
    """gopls command — from corpora.toml `oracle` for the named Go corpus, else PATH gopls."""
    if corpus:
        cfg = tomllib.loads((EVAL / "corpora.toml").read_text())
        c = cfg["corpus"][corpus]
        oracle = c.get("oracle", "gopls")
        cmd = {"gopls": ["gopls", "serve"]}.get(oracle)
        if cmd is None:
            sys.exit(f"corpus {corpus!r} oracle is {oracle!r}, not gopls; this tool is Go-only")
        return cmd
    return ["gopls", "serve"]


def _read_line(root: str, rel: str, line: int) -> str | None:
    try:
        src = (Path(root) / rel).read_text(
            encoding="utf-8", errors="replace").splitlines()
    except (FileNotFoundError, IsADirectoryError):
        return None
    return src[line - 1] if 1 <= line <= len(src) else None


def run_oracle(manifest_path: str, repo: str, cmd: list[str],
               group_timeout: float, log=sys.stderr) -> tuple[list[dict], dict]:
    """Load the manifest, query gopls once per unique (interface, method) group, and build
    per-site compare records + a summary. Returns (sites, summary)."""
    repo = os.path.abspath(os.path.expanduser(repo))
    dispatch = load_dispatch_sites(manifest_path)

    print(f"dispatch sites (fanout>0): {len(dispatch)}", file=log)

    oracle = GoplsSatisfiers(repo, cmd, group_timeout=group_timeout)
    records: list[dict] = []
    # Cache the satisfier set per INTERFACE-METHOD DECLARATION location (file, decl_line),
    # not per prism group: gopls disambiguates the interface a call site dispatches on
    # (caddy `next.ServeHTTP` is `http.Handler.ServeHTTP` at some sites and
    # `caddyhttp.Handler.ServeHTTP` at others, even though prism groups them together by
    # implementer set). Keying on the decl location queries the interface gopls sees and is
    # immune to a prism grouping that lumps two interfaces. Value = (types, label).
    decl_cache: dict[tuple[str, int], tuple[set[str], str]] = {}
    try:
        t0 = time.monotonic()
        oracle.start()
        print(f"gopls settled in {round(time.monotonic() - t0, 1)}s", file=log)

        # Warmup (race mitigation): gopls answers cross-package textDocument/implementation
        # with an empty list until it has type-checked the relevant packages, and the
        # $/progress-quiet settle heuristic can fire before that. Opening every dispatch
        # site's file + forcing a documentSymbol type-checks those packages; a final
        # resettle waits out the triggered indexing. Without this the first cross-package
        # query (e.g. caddy CaddyModule) can race and return 0 satisfiers, mis-scored as a
        # precision-0 over_approx.
        tw = time.monotonic()
        warm_files = sorted({s["file"] for s in dispatch})
        warmed = 0
        for f in warm_files:
            if oracle._did_open(f):
                oracle._methods(f)  # documentSymbol -> forces package type-check
                warmed += 1
        oracle.resettle()
        print(f"warmed {warmed}/{len(warm_files)} files in {round(time.monotonic() - tw, 1)}s",
              file=log)

        scored = 0
        for si, s in enumerate(dispatch, 1):
            method = s["method"]
            prism_set = set(s.get("implementers", []))
            line_text = _read_line(repo, s["file"], s["line"])
            col = method_token_col(line_text or "", method)

            gopls_set: set[str] | None = None
            iface = None
            if col is not None:
                # 1) resolve which interface this call dispatches on (its method decl).
                decl = oracle.method_decl(s["file"], s["line"] - 1, col)
                if decl is not None:
                    decl_file, decl_line, decl_char = decl
                    ckey = (decl_file, decl_line)
                    if ckey not in decl_cache:
                        # 2) implementation at the decl's method token, with empty-retry
                        #    (empty for a fanout>0 method => not-ready, not a true zero).
                        out = oracle.satisfier_types(decl_file, decl_line, decl_char)
                        if out is not None and out[1] == 0:
                            oracle.resettle(settle_s=oracle._settle_s)
                            out = oracle.satisfier_types(decl_file, decl_line, decl_char)
                        if out is not None and out[1] > 0:
                            # the enclosing type at the decl line is the interface name.
                            label = oracle._type_at(decl_file, decl_line) or \
                                f"{Path(decl_file).stem}:{decl_line + 1}"
                            decl_cache[ckey] = (out[0], label)
                    cached = decl_cache.get(ckey)
                    if cached is not None:
                        gopls_set, iface = cached
            if iface is None:
                # decl/impl unavailable: fall back to a type-assertion source label, then
                # a synthetic one. gopls_set stays None => the site is oracle_timeout.
                m = _ASSERT_RE.search(line_text) if line_text else None
                iface = m.group(1) if m else interface_label(line_text, method, si)

            if gopls_set is not None:
                scored += 1
            records.append(compare_site(
                file=s["file"], line=s["line"], interface=iface,
                method=method, prism_set=prism_set, gopls_set=gopls_set))

        print(f"scored {scored}/{len(dispatch)} sites; "
              f"{len(decl_cache)} unique interface-method declarations; "
              f"{len(dispatch) - scored} oracle_timeout in "
              f"{round(time.monotonic() - t0, 1)}s total", file=log)
    finally:
        oracle.stop()

    return records, summarize(records)


def _print_summary(summary: dict, log=sys.stdout) -> None:
    o = summary["overall"]
    print("\n=== dispatch oracle summary ===", file=log)
    print(f"overall dispatch_precision = {o['dispatch_precision']:.4f}  "
          f"(sites={o['sites']} sound={o['sound']} over_approx={o['over_approx']} "
          f"recall_gap={o['recall_gap']} oracle_timeout={o['oracle_timeout']})", file=log)
    print("per (interface, method):", file=log)
    for g in summary["groups"]:
        print(f"  {g['interface']}.{g['method']}: precision={g['dispatch_precision']:.4f} "
              f"sites={g['sites']} sound={g['sound']} over_approx={g['over_approx']} "
              f"recall_gap={g['recall_gap']} oracle_timeout={g['oracle_timeout']}", file=log)
    if summary["over_approx_sites"]:
        print(f"\nover_approx sites ({len(summary['over_approx_sites'])}) "
              f"— FP candidates for adjudication:", file=log)
        for r in summary["over_approx_sites"]:
            print(f"  {r['file']}:{r['line']} {r['interface']}.{r['method']} "
                  f"minted-but-not-satisfier: {r['prism_only_types']}", file=log)
    else:
        print("\nover_approx sites: NONE (every minted edge is a real satisfier)", file=log)
    if summary["oracle_timeout_groups"]:
        print(f"\noracle_timeout groups ({len(summary['oracle_timeout_groups'])}):", file=log)
        for r in summary["oracle_timeout_groups"]:
            print(f"  {r['interface']}.{r['method']}", file=log)


def main() -> int:
    ap = argparse.ArgumentParser(
        description="gopls interface-satisfaction oracle for prism Go dispatch sites "
                    "(Phase-IP Slice E; the §8 dispatch precision/recall gate).")
    ap.add_argument("--manifest", required=True,
                    help="prism nav interface-manifest JSON (with the `implementers` field)")
    ap.add_argument("--repo", required=True, help="path to the Go corpus checkout")
    ap.add_argument("--corpus", default=None,
                    help="corpus name in corpora.toml (reads the gopls cmd); default PATH gopls")
    ap.add_argument("--out", required=True, help="output comparison.json path")
    ap.add_argument("--group-timeout", type=float, default=300.0,
                    help="per-(interface,method) gopls request timeout seconds "
                         "(generous; a slow group is recorded oracle_timeout, not fatal)")
    args = ap.parse_args()

    cmd = make_cmd(args.corpus)
    sites, summary = run_oracle(args.manifest, args.repo, cmd, args.group_timeout)
    out = {
        "manifest": os.path.abspath(args.manifest),
        "repo": os.path.abspath(os.path.expanduser(args.repo)),
        "corpus": args.corpus,
        "oracle_cmd": cmd,
        "sites": sites,
        "summary": summary,
    }
    Path(args.out).write_text(json.dumps(out, indent=1, sort_keys=True))
    _print_summary(summary)
    print(f"\nwrote {args.out}", file=sys.stdout)
    return 0


if __name__ == "__main__":
    sys.exit(main())
