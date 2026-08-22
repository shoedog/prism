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


def _identity_key(identity: dict) -> tuple[str, str, str] | None:
    """The satisfier-type identity shared by prism and gopls.

    A Go package directory alone is insufficient because `p` and `p_test` can
    coexist there. Method target evidence is deliberately excluded from this
    key and checked separately by `_identity_targets`.
    """
    package_dir = identity.get("package_dir")
    package_clause = identity.get("package_clause")
    name = identity.get("name")
    if not all(isinstance(value, str) and value for value in (package_clause, name)):
        return None
    if not isinstance(package_dir, str):
        return None
    return package_dir, package_clause, name


def _identity_target(identity: dict) -> tuple[str, int, int] | None:
    file = identity.get("file")
    span = identity.get("span")
    if (
        not isinstance(file, str)
        or not isinstance(span, list)
        or len(span) != 2
        or not all(isinstance(line, int) for line in span)
    ):
        return None
    return file, span[0], span[1]


def _identity_targets(identities: list[dict]) -> dict | None:
    """Full target evidence grouped by satisfier-type identity, or None if unscorable."""
    targets: dict[tuple[str, str, str], set[tuple[str, int, int]]] = {}
    for identity in identities:
        key = _identity_key(identity)
        target = _identity_target(identity)
        if key is None or target is None:
            return None
        targets.setdefault(key, set()).add(target)
    return targets


def _identity_key_records(keys) -> list[dict]:
    return [
        {"package_dir": package_dir, "package_clause": package_clause, "name": name}
        for package_dir, package_clause, name in sorted(keys)
    ]


def _identity_records(targets: dict) -> list[dict]:
    records = []
    for (package_dir, package_clause, name), values in sorted(targets.items()):
        for file, start_line, end_line in sorted(values):
            records.append({
                "package_dir": package_dir,
                "package_clause": package_clause,
                "name": name,
                "file": file,
                "span": [start_line, end_line],
            })
    return records


def compare_site(
    file: str,
    line: int,
    interface: str,
    method: str,
    prism_set: set[str] | None = None,
    gopls_set: set[str] | None = None,
    *,
    prism_identities: list[dict] | None = None,
    gopls_identities: list[dict] | None = None,
    oracle_unresolved: bool = False,
) -> dict:
    """One per-site comparison record with qualified or legacy name-only identity.

    New manifests provide full identities and must never silently fall back to
    names. The name-only path is exclusively a compatibility path for manifests
    emitted by older prism binaries.
    """
    if prism_identities is None:
        identity_mode = "name_only"
        prism = set(prism_set or set())
        gopls = None if gopls_set is None else set(gopls_set)
        prism_identity_records: list[dict] = []
        gopls_identity_records: list[dict] | None = None
        prism_only_identities: list[dict] = []
        gopls_only_identities: list[dict] = []
        target_mismatches: list[dict] = []
        if gopls is None:
            classification = "oracle_timeout"
            prism_only = set()
            gopls_only = set()
        else:
            classification = classify(prism, gopls)
            prism_only = prism - gopls
            gopls_only = gopls - prism
    else:
        identity_mode = "qualified"
        prism_targets = _identity_targets(prism_identities)
        gopls_targets = (
            _identity_targets(gopls_identities) if gopls_identities is not None else None
        )
        prism_identity_records = _identity_records(prism_targets) if prism_targets is not None else []
        gopls_identity_records = (
            _identity_records(gopls_targets) if gopls_targets is not None else None
        )
        prism = set(prism_targets or {})
        gopls = None if gopls_targets is None else set(gopls_targets)
        prism_only = set()
        gopls_only = set()
        target_mismatches = []
        if oracle_unresolved or prism_targets is None or (
            gopls_identities is not None and gopls_targets is None
        ):
            classification = "oracle_unresolved"
        elif gopls_targets is None:
            classification = "oracle_timeout"
        else:
            prism_only = prism - gopls
            gopls_only = gopls - prism
            classification = classify(prism, gopls)
            if classification != "over_approx":
                for key in sorted(prism & gopls):
                    prism_targets_for_key = prism_targets[key]
                    gopls_targets_for_key = gopls_targets[key]
                    if not prism_targets_for_key <= gopls_targets_for_key:
                        target_mismatches.append({
                            "identity": _identity_key_records([key])[0],
                            "prism_targets": [
                                {"file": target_file, "span": [start, end]}
                                for target_file, start, end in sorted(prism_targets_for_key)
                            ],
                            "gopls_targets": [
                                {"file": target_file, "span": [start, end]}
                                for target_file, start, end in sorted(gopls_targets_for_key)
                            ],
                        })
                if target_mismatches:
                    classification = "target_mismatch"
        prism_only_identities = _identity_key_records(prism_only)
        gopls_only_identities = _identity_key_records(gopls_only)

    prism_implementers = (
        sorted({identity["name"] for identity in prism_identity_records})
        if identity_mode == "qualified"
        else sorted(prism_set or set())
    )
    return {
        "file": file,
        "line": line,
        "interface": interface,
        "method": method,
        "identity_mode": identity_mode,
        "prism_implementers": prism_implementers,
        "gopls_satisfiers": None if gopls is None else sorted(item[-1] if isinstance(item, tuple) else item for item in gopls),
        "prism_identities": prism_identity_records,
        "gopls_satisfier_identities": gopls_identity_records,
        "classification": classification,
        "prism_only_types": sorted(item[-1] if isinstance(item, tuple) else item for item in prism_only),
        "gopls_only_types": sorted(item[-1] if isinstance(item, tuple) else item for item in gopls_only),
        "prism_only_identities": prism_only_identities,
        "gopls_only_identities": gopls_only_identities,
        "target_mismatches": target_mismatches,
    }


def _precision_acc() -> dict:
    return {"inter": 0, "prism": 0}


def summarize(sites: list[dict]) -> dict:
    """Per-(interface, method) + overall rollup of compare_site records.

    dispatch_precision aggregates as (sum |P∩G|) / (sum |P|) over scored sites. Timeout and
    unresolved sites are excluded from the type-precision ratio; target mismatches remain
    type-scored but are separately blocking Exact-target failures in delta mode.
    """
    groups: dict[tuple, dict] = {}
    overall = {
        "sites": 0,
        "sound": 0,
        "over_approx": 0,
        "recall_gap": 0,
        "oracle_timeout": 0,
        "oracle_unresolved": 0,
        "target_mismatch": 0,
        "scored_sites": 0,
    }
    overall_acc = _precision_acc()
    over_approx_sites: list[dict] = []
    timeout_groups: dict[tuple, tuple[str, str]] = {}
    unresolved_sites: list[dict] = []
    target_mismatch_sites: list[dict] = []
    identity_modes: set[str] = set()

    for s in sites:
        # Group identity is (method, the minted implementer set) — the same (iface_key,
        # method) minted that set, so this is the true (interface, method) pair. A method
        # declared on two interfaces (distinct sets) stays split; a single interface whose
        # sites carry different display labels stays merged. `interface` is the display
        # label of the first site in the group.
        identity_mode = s.get("identity_mode", "name_only")
        identity_modes.add(identity_mode)
        if identity_mode == "qualified":
            group_implementers = tuple(
                sorted(
                    (
                        identity["package_dir"],
                        identity["package_clause"],
                        identity["name"],
                        identity["file"],
                        tuple(identity["span"]),
                    )
                    for identity in s["prism_identities"]
                )
            )
        else:
            group_implementers = tuple(s["prism_implementers"])
        key = (s["method"], identity_mode, group_implementers)
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
                "oracle_unresolved": 0,
                "target_mismatch": 0,
                "scored_sites": 0,
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
        if cls == "oracle_unresolved":
            unresolved_sites.append({
                "file": s["file"], "line": s["line"], "interface": s["interface"],
                "method": s["method"],
            })
            continue
        if identity_mode == "qualified":
            prism = {
                (identity["package_dir"], identity["package_clause"], identity["name"])
                for identity in s["prism_identities"]
            }
            gopls = {
                (identity["package_dir"], identity["package_clause"], identity["name"])
                for identity in s["gopls_satisfier_identities"]
            }
        else:
            prism = set(s["prism_implementers"])
            gopls = set(s["gopls_satisfiers"])
        inter = len(prism & gopls)
        g["scored_sites"] += 1
        overall["scored_sites"] += 1
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
        elif cls == "target_mismatch":
            target_mismatch_sites.append({
                "file": s["file"], "line": s["line"], "interface": s["interface"],
                "method": s["method"], "target_mismatches": s["target_mismatches"],
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
        "identity_mode": (
            "name_only" if identity_modes == {"name_only"} else
            "qualified" if identity_modes == {"qualified"} else "mixed"
        ),
        "groups": group_list,
        "over_approx_sites": sorted(
            over_approx_sites, key=lambda r: (r["file"], r["line"])
        ),
        "oracle_timeout_groups": [
            {"interface": i, "method": m}
            for (i, m) in sorted(timeout_groups.values())
        ],
        "oracle_unresolved_sites": sorted(
            unresolved_sites, key=lambda r: (r["file"], r["line"])
        ),
        "target_mismatch_sites": sorted(
            target_mismatch_sites, key=lambda r: (r["file"], r["line"])
        ),
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

    def _package_clause(self, rel: str) -> str | None:
        """Package declaration for one repo-relative Go source file."""
        try:
            source = (Path(self.root) / rel).read_text(
                encoding="utf-8", errors="replace"
            )
        except (FileNotFoundError, IsADirectoryError):
            return None
        match = re.search(r"(?m)^\s*package\s+([A-Za-z_][A-Za-z0-9_]*)\b", source)
        return match.group(1) if match else None

    def _identity_at(self, rel: str, line0: int) -> dict | None:
        """Qualified satisfier identity and method-definition target at a gopls location."""
        best = None
        for _name, container, start_line, end_line in self._methods(rel):
            if start_line <= line0 <= end_line:
                width = end_line - start_line
                if best is None or width < best[0]:
                    best = (width, container, start_line, end_line)
        if best is None or not best[1]:
            return None
        package_clause = self._package_clause(rel)
        if package_clause is None:
            return None
        package_dir = Path(rel).parent.as_posix()
        return {
            "name": best[1],
            "file": rel,
            "span": [best[2] + 1, best[3] + 1],
            "package_dir": "" if package_dir == "." else package_dir,
            "package_clause": package_clause,
        }

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

    def satisfier_identities(
        self, rel: str, line0: int, char0: int
    ) -> tuple[list[dict], int, bool] | None:
        """gopls implementation locations as qualified identities and method targets.

        The boolean marks a location that gopls returned but the adapter could not map to
        a repository package clause, receiver type, or method declaration target. It is
        deliberately distinct from an LSP timeout: delta mode must block on either, but
        the latter has no gopls evidence while the former exposes an adapter gap.
        """
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
        identities: list[dict] = []
        unresolved = False
        for result in results:
            uri = result.get("uri") or result.get("targetUri")
            location = result.get("range") or result.get("targetSelectionRange") or result.get("targetRange")
            target_file = uri_to_rel(uri, self.root) if uri else None
            if target_file is None or location is None:
                unresolved = True
                continue
            identity = self._identity_at(target_file, location["start"]["line"])
            if identity is None:
                unresolved = True
            else:
                identities.append(identity)
        return identities, len(results), unresolved


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
               group_timeout: float, log=sys.stderr,
               oracle_factory=GoplsSatisfiers) -> tuple[list[dict], dict]:
    """Load the manifest, query gopls once per unique (interface, method) group, and build
    per-site compare records + a summary. Returns (sites, summary)."""
    repo = os.path.abspath(os.path.expanduser(repo))
    dispatch = load_dispatch_sites(manifest_path)

    print(f"dispatch sites (fanout>0): {len(dispatch)}", file=log)

    oracle = oracle_factory(repo, cmd, group_timeout=group_timeout)
    records: list[dict] = []
    # Cache the satisfier set per INTERFACE-METHOD DECLARATION location (file, decl_line),
    # not per prism group: gopls disambiguates the interface a call site dispatches on
    # (caddy `next.ServeHTTP` is `http.Handler.ServeHTTP` at some sites and
    # `caddyhttp.Handler.ServeHTTP` at others, even though prism groups them together by
    # implementer set). Keying on the decl location queries the interface gopls sees and is
    # immune to a prism grouping that lumps two interfaces. Value = (qualified
    # satisfier identities, display label, adapter-unresolved flag).
    decl_cache: dict[tuple[str, int], tuple[list[dict], str, bool]] = {}
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

            gopls_identities: list[dict] | None = None
            oracle_unresolved = False
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
                        out = oracle.satisfier_identities(decl_file, decl_line, decl_char)
                        if out is not None and out[1] == 0:
                            oracle.resettle(settle_s=oracle._settle_s)
                            out = oracle.satisfier_identities(decl_file, decl_line, decl_char)
                        if out is not None and out[1] > 0:
                            # the enclosing type at the decl line is the interface name.
                            label = oracle._type_at(decl_file, decl_line) or \
                                f"{Path(decl_file).stem}:{decl_line + 1}"
                            decl_cache[ckey] = (out[0], label, out[2])
                    cached = decl_cache.get(ckey)
                    if cached is not None:
                        gopls_identities, iface, oracle_unresolved = cached
            if iface is None:
                # decl/impl unavailable: fall back to a type-assertion source label, then
                # a synthetic one. gopls identities stay None => oracle_timeout.
                m = _ASSERT_RE.search(line_text) if line_text else None
                iface = m.group(1) if m else interface_label(line_text, method, si)

            if "implementer_identities" in s:
                record = compare_site(
                    file=s["file"], line=s["line"], interface=iface, method=method,
                    prism_set=prism_set,
                    prism_identities=s["implementer_identities"],
                    gopls_identities=gopls_identities,
                    oracle_unresolved=oracle_unresolved,
                )
            else:
                # Compatibility only: old manifests lack target/package evidence, so the
                # comparison is explicitly marked name_only rather than pretending it is
                # qualified by the new gopls adapter.
                gopls_set = (
                    None if gopls_identities is None
                    else {identity["name"] for identity in gopls_identities}
                )
                record = compare_site(
                    file=s["file"], line=s["line"], interface=iface, method=method,
                    prism_set=prism_set, gopls_set=gopls_set,
                )
            if record["classification"] not in {"oracle_timeout", "oracle_unresolved"}:
                scored += 1
            records.append(record)

        print(f"scored {scored}/{len(dispatch)} sites; "
              f"{len(decl_cache)} unique interface-method declarations; "
              f"{len(dispatch) - scored} timeout_or_unresolved in "
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
