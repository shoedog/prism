"""Runner CLI (spec §2.11). Glue over the tested layers.

G3 replay property: raw metric inputs are stored in the run JSON's ``probes`` key;
corrected metrics additionally apply the current adjudication store.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import math
import os
import sys
import time
import tomllib
from pathlib import Path

from . import pinned as pinned_mod
from .accounting import CorpusAccounting, evaluate_floors
from .adjudication import apply_verdicts, load_records, reanchor_map
from .compare import caller_fn_sets, site_compare
from .corpus import (
    corpus_dirty,
    corpus_sha,
    load_snapshot,
    save_snapshot,
    snapshot_path,
    untracked_sources,
    universe,
    verify_source_manifest,
    node_modules_environment,
)
from .lsp_client import LspError
from .matrix import MATRIX_LANGUAGES, run_matrix
from .metrics import precision_recall
from .model import CallEdge, FunctionDef, Location, SelectionMatcher, edge_tier
from .oracles import OracleError, OracleTimeout
from .report import write_reports
from .spotcheck import classify_site, find_call_position, utf16_column
from .strata import classify, filter_to_universe, inventory_diff, sample_strata
from .sut import PrismCli, SutAmbiguous, SutError, SutStale, SutTimeout, configure_seed_addresses

EVAL_DIR = Path(__file__).resolve().parents[1]


def seed_id(fd, line_counts):
    key = f"{fd.location.file}:{fd.selection_line}"
    if line_counts[(fd.location.file, fd.selection_line)] > 1:
        key += ":" + json.dumps([fd.selection_char, fd.name, fd.kind, fd.container], separators=(",", ":"))
    return key


def _site_parts(site) -> tuple[str, int, int, dict]:
    """Read legacy site triples or metadata-bearing site records."""
    if isinstance(site, dict):
        return (
            site["file"],
            site["start_line"],
            site.get("end_line", site["start_line"]),
            site,
        )
    file, start, end, *rest = site
    meta = {}
    if rest:
        raw = rest[0]
        if isinstance(raw, dict):
            meta = raw
        elif raw is not None:
            meta = {"resolution_kind": raw}
    return file, start, end, meta


def _edges(sites: list, direction: str) -> list[CallEdge]:
    """Rebuild minimal CallEdges from stored site triples plus optional metadata."""
    dummy = FunctionDef("_", "function", None, Location("_", 1, 1), 1)
    return [
        CallEdge(
            direction,
            dummy,
            None,
            None,
            Location(f, s, e),
            meta.get("resolution_kind"),
            meta.get("score"),
        )
        for f, s, e, meta in (_site_parts(site) for site in sites)
    ]


def _dispatch_kind_for_site(sites: list, file: str, line: int) -> str | None:
    for f, s, _e, meta in (_site_parts(site) for site in sites):
        if f == file and s == line:
            dispatch = meta.get("dispatch_kind") or meta.get("resolution_kind")
            if dispatch:
                return dispatch
    return None


def _tier_for_site(edges: list[CallEdge], file: str, line: int) -> str | None:
    """P6a: the edge_tier of the prism edge at (file, line), or None if absent.

    Reuses the CallEdges _edges() already rebuilt (with score restored per item 2b)
    so the exact/candidate cutoff is decided in exactly one place (model.edge_tier).
    """
    for e in edges:
        if e.call_site.file == file and e.call_site.start_line == line:
            return edge_tier(e)
    return None


def _site_key(file: str, line: int) -> str:
    return f"{file}:{line}"


def _pair_metrics(prism: set[tuple], oracle: set[tuple]) -> dict:
    tp = len(prism & oracle)
    return precision_recall(tp, len(prism - oracle), len(oracle - prism))


def _site_fingerprints_for_probes(probes: dict, corpus_root: str) -> dict[str, str]:
    """Fingerprint every probe diff-site's call line (+/- 1) from corpus source, so
    verdicts re-anchor across line drift. Keyed "file:line"; each file read once."""
    from .adjudication import fingerprint

    by_file: dict[str, set] = {}
    for pid, p in probes.items():
        if pid.startswith("_"):
            continue
        for site in list(p.get("prism_sites", [])) + list(p.get("oracle_sites", [])):
            file, line, _end, _meta = _site_parts(site)
            by_file.setdefault(file, set()).add(line)
    out: dict[str, str] = {}
    for f, lines in by_file.items():
        try:
            src = (
                (Path(corpus_root) / f)
                .read_text(encoding="utf-8", errors="replace")
                .splitlines()
            )
        except (FileNotFoundError, IsADirectoryError):
            continue
        for line in lines:
            lo = max(1, line - 1)
            hi = min(len(src), line + 1)
            out[f"{f}:{line}"] = fingerprint(src[lo - 1 : hi])
    return out


def check_pinned(cfg: dict, sha: str, allow: bool) -> tuple[bool, str | None]:
    pinned = cfg.get("pinned_sha")
    if allow or not pinned or pinned == sha:
        return True, None
    return False, f"corpus_sha_drift: {sha} != pinned {pinned}"


def _pending_for_probe(
    probe: dict, corpus: str, adjudications: list, site_fps: dict | None = None
) -> list[dict]:
    site_fps = site_fps or {}
    if probe.get("outcome") == "inventory_miss":
        # the seed has no prism inventory match (declarations, etc.) — its oracle
        # edges are systematic recall already counted by M1; they are not
        # adjudicable call-resolution diffs (adjudication-validated finding:
        # interface-method declaration seeds)
        return []
    direction = probe["direction"]
    prism_edges = _edges(probe["prism_sites"], direction)
    diff = site_compare(
        prism_edges,
        _edges(probe["oracle_sites"], direction),
    )
    scoped = [
        r
        for r in adjudications
        if r.corpus == corpus
        and r.measurement == direction
        and r.seed_def == probe["seed_def"]
    ]
    adjudicated = {(r.direction, r.site) for r in scoped}
    # Sites that re-anchor to a moved verdict are resolved (not pending) — same
    # resolver apply_verdicts uses, so counts and the pending list can't disagree.
    ra = reanchor_map(diff.fp, diff.fn, scoped, site_fps)
    pending = []
    for file, line in sorted(diff.fp):
        key = ("prism_only", _site_key(file, line))
        if key not in adjudicated and key not in ra:
            record = {
                "corpus": corpus,
                "measurement": direction,
                "direction": "prism_only",
                "seed_def": probe["seed_def"],
                "site": key[1],
                "site_fingerprint": site_fps.get(key[1]),
            }
            dispatch_kind = _dispatch_kind_for_site(probe.get("prism_sites", []), file, line)
            if dispatch_kind:
                record["dispatch_kind"] = dispatch_kind
            # P6a: tag candidate-tier prism_only pendings so adjudication can filter
            # them; exact-tier ones keep their existing shape (no "tier" key at all).
            if _tier_for_site(prism_edges, file, line) == "candidate":
                record["tier"] = "candidate"
            pending.append(record)
    for file, line in sorted(diff.fn):
        key = ("oracle_only", _site_key(file, line))
        if key not in adjudicated and key not in ra:
            record = {
                "corpus": corpus,
                "measurement": direction,
                "direction": "oracle_only",
                "seed_def": probe["seed_def"],
                "site": key[1],
                "site_fingerprint": site_fps.get(key[1]),
            }
            dispatch_kind = _dispatch_kind_for_site(probe.get("oracle_sites", []), file, line)
            if dispatch_kind:
                record["dispatch_kind"] = dispatch_kind
            pending.append(record)
    return pending


def _compute_m2_and_pending(
    probes: dict, adjudications: list, site_fps: dict | None = None
) -> tuple[dict, list[dict], int]:
    """Pure replay over stored probes plus the current adjudication store.

    ``site_fps`` ({"file:line": fingerprint}) lets stale verdicts re-anchor across line
    drift (see ``adjudication.apply_verdicts``); empty for legacy runs without it.
    """
    site_fps = site_fps or {}
    corpus = probes.get("_corpus", "?")
    shortfalls = {
        name: max(0, meta.get("target", 0) - meta.get("eligible_symbols", 0))
        for name, meta in probes.get("_strata", {}).items()
    }
    pending_records: list[dict] = []
    out: dict = {}
    stale_adjudications = 0
    scored = {"ok", "inventory_miss"}
    for direction in ("callers", "callees"):
        strata: dict = {}
        for pid, p in sorted(probes.items()):
            if (
                pid.startswith("_")
                or p.get("outcome") not in scored
                or p.get("direction") != direction
            ):
                continue
            prism_edges = _edges(p["prism_sites"], direction)
            oracle_edges = _edges(p["oracle_sites"], direction)
            r = site_compare(prism_edges, oracle_edges)
            # P6a stratification: the legacy raw/corrected fields above stay computed
            # over ALL prism edges (candidates included) for baseline comparability;
            # exact_tier/candidate_tier below are informational additions computed
            # over a tier-filtered prism edge set against the SAME (unfiltered)
            # oracle set. score is None (legacy/old-run replay) -> "exact" (model.edge_tier).
            exact_edges = [e for e in prism_edges if edge_tier(e) == "exact"]
            candidate_edges = [e for e in prism_edges if edge_tier(e) == "candidate"]
            r_exact = site_compare(exact_edges, oracle_edges)
            r_candidate = site_compare(candidate_edges, oracle_edges)
            prism_functions = {tuple(v) for v in p.get("prism_functions", [])}
            oracle_functions = {tuple(v) for v in p.get("oracle_functions", [])}
            function_raw = _pair_metrics(prism_functions, oracle_functions)
            s = strata.setdefault(
                p["stratum"],
                {
                    "tp": 0,
                    "fp": 0,
                    "fn": 0,
                    "corr_tp": 0,
                    "corr_fp": 0,
                    "corr_fn": 0,
                    "pending": 0,
                    "function_tp": 0,
                    "function_fp": 0,
                    "function_fn": 0,
                    "exact_tp": 0,
                    "exact_fp": 0,
                    "exact_fn": 0,
                    "candidate_count": 0,
                    "candidate_confirmed": 0,
                    "candidate_unconfirmed": 0,
                },
            )
            s["tp"] += len(r.tp)
            s["fp"] += len(r.fp)
            s["fn"] += len(r.fn)
            s["exact_tp"] += len(r_exact.tp)
            s["exact_fp"] += len(r_exact.fp)
            s["exact_fn"] += len(r_exact.fn)
            s["candidate_count"] += len(r_candidate.tp) + len(r_candidate.fp)
            s["candidate_confirmed"] += len(r_candidate.tp)
            s["candidate_unconfirmed"] += len(r_candidate.fp)
            c = apply_verdicts(
                tp=len(r.tp),
                fp_sites=r.fp,
                fn_sites=r.fn,
                records=adjudications,
                corpus=corpus,
                measurement=direction,
                seed_def=p["seed_def"],
                site_fps=site_fps,
            )
            s["corr_tp"] += c.tp
            s["corr_fp"] += c.fp
            s["corr_fn"] += c.fn
            s["pending"] += c.pending
            stale_adjudications += c.stale
            s["function_tp"] += function_raw["tp"]
            s["function_fp"] += function_raw["fp"]
            s["function_fn"] += function_raw["fn"]
            pending_records.extend(_pending_for_probe(p, corpus, adjudications, site_fps))
        out[direction] = {}
        for name, s in sorted(strata.items()):
            function_metrics = precision_recall(
                s["function_tp"],
                s["function_fp"],
                s["function_fn"],
            )
            out[direction][name] = {
                "raw": precision_recall(s["tp"], s["fp"], s["fn"]),
                "corrected": precision_recall(
                    s["corr_tp"],
                    s["corr_fp"],
                    s["corr_fn"],
                ),
                "function": {
                    "raw": function_metrics,
                    "corrected": function_metrics,
                },
                "pending": s["pending"],
                "shortfall": shortfalls.get(name, 0),
                # P6a additive fields (never read for baseline comparison; the P3
                # gate reads exact_tier only, candidate_tier is informational +
                # adjudication-fed — see eval/README.md).
                "exact_tier": {
                    "raw": precision_recall(s["exact_tp"], s["exact_fp"], s["exact_fn"]),
                },
                "candidate_tier": {
                    "count": s["candidate_count"],
                    "oracle_confirmed": s["candidate_confirmed"],
                    "oracle_unconfirmed": s["candidate_unconfirmed"],
                },
            }
    return out, pending_records, stale_adjudications


def compute_m2_from_probes(
    probes: dict, adjudications: list, site_fps: dict | None = None
) -> dict:
    """Pure: stored raw sites -> per-direction per-stratum raw+corrected metrics."""
    return _compute_m2_and_pending(probes, adjudications, site_fps)[0]


def recompute_metrics_from_stored(stored: dict) -> dict:
    if "probes" not in stored:
        return stored
    adj = load_records(EVAL_DIR / "adjudications.jsonl")
    site_fps = stored.get("site_fingerprints", {})
    m2, pending, stale = _compute_m2_and_pending(stored["probes"], adj, site_fps)
    meta = {
        **stored.get("meta", {}),
        "stale_adjudications": stale,
    }
    return {**stored, "meta": meta, "m2": m2, "pending": pending}


def load_corpora() -> dict:
    cfg = tomllib.loads((EVAL_DIR / "corpora.toml").read_text())
    for c in cfg["corpus"].values():
        path = os.path.expanduser(c["path"])
        if not os.path.isabs(path):
            path = os.path.join(EVAL_DIR.parent, path)
        c["path"] = os.path.abspath(path)
        if c.get("source_manifest"):
            manifest = os.path.expanduser(c["source_manifest"])
            c["source_manifest"] = str(Path(manifest) if os.path.isabs(manifest) else EVAL_DIR.parent / manifest)
    return cfg


def resolve_capability(oracle, overlay_probe_ok: bool, inventory: list) -> bool:
    """§2.2 two-stage capability check: overlay probe, else one retry of the call
    hierarchy against a real inventory symbol (workspace-membership-safe)."""
    if overlay_probe_ok:
        return True
    for fd in inventory:
        if fd.name is not None:
            try:
                oracle.callers(fd)
                return True
            except OracleTimeout:
                raise
            except (OracleError, LspError):
                return False
    return False


def make_oracle(cfg: dict):
    from .oracles import LspOracle
    from .tsserver import TsserverOracle

    cmd = {
        "rust-analyzer": ["rust-analyzer"],
        "gopls": ["gopls", "serve"],
        "pyright": ["pyright-langserver", "--stdio"],
        "basedpyright": ["basedpyright-langserver", "--stdio"],
        "tsserver": ["tsserver", "--disableAutomaticTypingAcquisition"],
    }[cfg["oracle"]]
    adapter = TsserverOracle if cfg["oracle"] == "tsserver" else LspOracle
    return adapter(
        cmd,
        cfg["path"],
        cfg["lang"],
        settle_s=cfg.get("settle_s", 2.0),
        quiescence_cap_s=cfg.get("quiescence_cap_s", 300.0),
        query_timeout_s=cfg.get("query_timeout_s", 10.0),
        startup_timeout_s=cfg.get("startup_timeout_s", 60.0),
        oracle_budget_s=cfg.get("oracle_budget_s", 600.0),
        cargo_features=cfg.get("cargo_features"),
    )


def _stored_sites(edges: list[CallEdge], direction: str) -> list[list]:
    out = []
    for e in edges:
        if direction != "callers" and e.other_def is None:
            continue
        site = [e.call_site.file, e.call_site.start_line, e.call_site.end_line]
        meta = {}
        if e.resolution_kind is not None:
            meta["resolution_kind"] = e.resolution_kind
        if e.score is not None:
            meta["score"] = e.score
        dispatch_kind = getattr(e, "dispatch_kind", None)
        if dispatch_kind is not None:
            meta["dispatch_kind"] = dispatch_kind
        if meta:
            site.append(meta)
        out.append(site)
    return out


def _stored_functions(edges: list[CallEdge], inventory: list[FunctionDef]) -> list[list]:
    return [list(pair) for pair in sorted(caller_fn_sets(edges, inventory))]


def package_dirs(root: str) -> set[str]:
    dirs = set()
    for dirpath, _dirnames, filenames in os.walk(root):
        if "__init__.py" not in filenames:
            continue
        rel = os.path.relpath(dirpath, root).replace(os.sep, "/")
        if rel != ".":
            dirs.add(rel)
    return dirs


def match_snapshot_to_prism(
    snapshot: list[FunctionDef],
    prism_inventory: list[FunctionDef],
) -> dict[FunctionDef, FunctionDef]:
    matcher = SelectionMatcher(prism_inventory)
    matched = {}
    for fd in snapshot:
        match = matcher.match(fd)
        if match is not None:
            matched[fd] = match
    return matched


def matrix_result_to_json(result) -> dict:
    # "callers" probes carry got/expected as sets of (file, line) call-site
    # tuples -- sort them into deterministic JSON arrays. "taint"/"module_deps"
    # probes already carry deterministic, triage-useful strings (P6bc) built
    # by tier_a.matrix's _format_taint_summary / module-edge join -- pass
    # those through unchanged rather than treating them as tuple sets.
    probe = getattr(result, "probe", "callers")
    if probe == "callers":
        got = [list(site) for site in sorted(result.got)]
        expected = [list(site) for site in sorted(result.expected)]
        got_kinds = {
            f"{file}:{line}": kind
            for (file, line), kind in sorted(result.got_kinds.items())
        }
    else:
        got = result.got
        expected = result.expected
        got_kinds = {}
    return {
        "capability": result.capability,
        "language": result.language,
        "outcome": result.outcome,
        "probe": probe,
        "got": got,
        "expected": expected,
        "got_kinds": got_kinds,
        "expected_resolution_kind": result.expected_resolution_kind,
        "forbid_resolution_kind": result.forbid_resolution_kind,
    }


def run_m3_spotcheck(
    probes: dict,
    oracle,
    snapshot: list[FunctionDef],
    corpus_root: str,
    cap: int,
) -> dict:
    """Spot-check prism-only caller sites against oracle definitions."""
    from collections import Counter
    counts = Counter((fd.location.file, fd.selection_line) for fd in snapshot)
    by_seed = {seed_id(fd, counts): fd for fd in snapshot}
    counts = {"confirmed_tp": 0, "confirmed_fp": 0, "ambiguous": 0, "alias_site": 0}
    checked = []
    for pid, probe in sorted(probes.items()):
        if len(checked) >= cap:
            break
        if pid.startswith("_") or probe.get("direction") != "callers":
            continue
        if probe.get("outcome") not in {"ok", "inventory_miss"}:
            continue
        diff = site_compare(_edges(probe["prism_sites"], "callers"),
                            _edges(probe["oracle_sites"], "callers"))
        seed = by_seed.get(probe["seed_def"])
        if seed is None or seed.name is None:
            continue
        for file, line in sorted(diff.fp):
            if len(checked) >= cap:
                break
            path = Path(corpus_root) / file
            source = path.read_text(encoding="utf-8", errors="replace").splitlines()
            text = source[line - 1] if 1 <= line <= len(source) else ""
            char = find_call_position(text, seed.name)
            defs = []
            if char is not None:
                try:
                    defs = oracle.definitions_at(snapshot, file, line, utf16_column(text, char))
                except OracleError as exc:
                    if isinstance(exc, OracleTimeout):
                        verdict = "oracle_timeout"
                    else:
                        verdict = "oracle_error"
                else:
                    verdict = classify_site(text, seed.name, defs, seed)
            else:
                verdict = classify_site(text, seed.name, defs, seed)
            counts[verdict] = counts.get(verdict, 0) + 1
            checked.append({"probe": pid, "site": f"{file}:{line}", "verdict": verdict})
    return {"cap": cap, "checked": checked, "counts": counts}


def run_corpus(name: str, cfg: dict, defaults: dict, args) -> dict:
    started = time.monotonic()
    def progress(message):
        print(f"[{name} +{time.monotonic() - started:.1f}s] {message}", file=sys.stderr, flush=True)
    progress("checking SUT and corpus identity")
    sut = PrismCli(str(EVAL_DIR.parent), sut_bin=args.sut_bin,
                   allow_stale=args.allow_stale_sut)
    manifest_files, source_drift = (verify_source_manifest(cfg)
                                  if cfg.get("source_manifest") else (None, []))
    sha = cfg["pinned_sha"] if manifest_files is not None else corpus_sha(cfg["path"])
    pinned_ok, pinned_reason = check_pinned(
        cfg,
        sha,
        getattr(args, "allow_drift", False),
    )
    initial_invalid_reasons = ([] if pinned_ok else [pinned_reason]) + source_drift
    untracked = [] if manifest_files is not None else untracked_sources(cfg["path"], cfg["lang"])
    dirty_reasons = []
    if untracked:
        dirty_reasons.append(f"untracked_sources: {len(untracked)}")
    run: dict = {
        "meta": {
            "corpus": name,
            "corpus_sha": sha,
            "corpus_dirty": (bool(source_drift) if manifest_files is not None else
                             corpus_dirty(cfg["path"]) or bool(untracked)),
            "corpus_identity": {"component": "source_manifest" if manifest_files is not None else "git_head",
                                "expected": cfg.get("source_manifest_sha256", cfg.get("pinned_sha")),
                                "observed": (hashlib.sha256(Path(cfg["source_manifest"]).read_bytes()).hexdigest()
                                             if manifest_files is not None else sha),
                                "manifest_sha256": cfg.get("source_manifest_sha256"),
                                "drift": source_drift},
            "language": cfg["lang"],
            "oracle_configuration": {"cargo_features": cfg.get("cargo_features")},
            "corpus_dirty_reasons": dirty_reasons,
            "prism_sha": sut.sha,
            "prism_dirty": sut.dirty,
            "seed": defaults["seed"],
            "date": args.date,
            "harness_sha": corpus_sha(str(EVAL_DIR.parent)),
            "oracle_not_quiescent": False,
            "oracle": cfg["oracle"],
            "oracle_error_rate": 0.0,
            "sut_error_rate": 0.0,
            "baseline_invalid": not pinned_ok,
            "invalid_reasons": initial_invalid_reasons,
            "stale_adjudications": 0,
            "wall_s": {},
        },
        "probes": {"_corpus": name},
        "failures": {},
    }
    if initial_invalid_reasons and manifest_files is not None:
        run["meta"]["baseline_invalid"] = True
        run["meta"]["invalid_reasons"] = initial_invalid_reasons
        progress(f"INVALID source drift: {initial_invalid_reasons}")
        return run
    oracle_cfg = {
        **cfg,
        "settle_s": cfg.get("settle_s", defaults.get("settle_s", 2.0)),
        "quiescence_cap_s": cfg.get(
            "quiescence_cap_s", defaults.get("quiescence_cap_s", 300.0)),
    }
    for key in ("query_timeout_s", "startup_timeout_s", "oracle_budget_s"):
        value = getattr(args, key, None)
        oracle_cfg[key] = value if value is not None else cfg.get(key, defaults.get(key, {"query_timeout_s": 10.0, "startup_timeout_s": 60.0, "oracle_budget_s": 600.0}[key]))
    run["meta"]["timeouts"] = {key: oracle_cfg[key] for key in ("query_timeout_s", "startup_timeout_s", "oracle_budget_s")}
    sut.query_timeout_s = cfg.get("sut_timeout_s", 120.0)
    sut.deadline = started + oracle_cfg["oracle_budget_s"]
    oracle = make_oracle(oracle_cfg)
    acc = CorpusAccounting()
    def failure(pid, exc):
        outcome = "oracle_timeout" if isinstance(exc, OracleTimeout) else "oracle_error"
        acc.record(pid, outcome)
        run["failures"][pid] = {"outcome": outcome, "error": str(exc)}
        progress(f"{outcome} {pid}: {exc}")
    try:
        t0 = time.monotonic()
        progress(f"starting {cfg['oracle']}")
        oracle.start()
        run["meta"]["wall_s"]["oracle_start"] = round(time.monotonic() - t0, 3)
        run["meta"]["oracle"] = oracle.version()
        if cfg["oracle"] == "tsserver":
            environment = {"oracle": oracle.version(), "node_modules": node_modules_environment(cfg['path'])}
            run["meta"]["oracle_environment"] = environment
            run["meta"]["corpus_identity"]["oracle_environment_sha256"] = hashlib.sha256(
                json.dumps(environment, sort_keys=True).encode()).hexdigest()
            if environment["oracle"] != cfg.get("oracle_version"):
                initial_invalid_reasons.append("oracle_version_drift")
        run["meta"]["oracle_not_quiescent"] = oracle.not_quiescent
        # §2.2 capability probe, two-stage: the overlay probe is the fast path, but
        # servers that only analyze workspace-member files (rust-analyzer: an overlay
        # .rs belongs to no crate) legitimately fail it — the spec's named fallback is
        # one retry against a REAL inventory symbol, which requires M1 first.
        overlay_probe_ok = oracle.capability_probe()

        files = universe(
            cfg["path"],
            cfg["lang"],
            cfg.get("excludes", []),
            tracked_only=manifest_files is None,
        )
        if manifest_files is not None:
            files = [f for f in files if f in set(manifest_files)]
        oracle_inv = []
        t0 = time.monotonic()
        progress(f"M1 oracle inventory: {len(files)} files")
        for i, f in enumerate(files):
            if i % 25 == 0:
                progress(f"M1 {i}/{len(files)} {f}")
            try:
                oracle_inv.extend(oracle.document_symbols(f))
            except OracleError as exc:
                failure(f"docsym:{f}", exc)
                if isinstance(exc, OracleTimeout) and "budget" in str(exc):
                    raise
        run["meta"]["wall_s"]["m1_oracle_inventory"] = round(
            time.monotonic() - t0,
            3,
        )

        if cfg["lang"] == "rust" and hasattr(oracle, "wait_ready"):
            progress("waiting for post-inventory rust-analyzer quiescence")
            oracle.wait_ready()

        if not resolve_capability(oracle, overlay_probe_ok, oracle_inv):
            run["meta"].update(
                baseline_invalid=True,
                invalid_reasons=initial_invalid_reasons + ["oracle_unsupported"],
                oracle_error_rate=1.0,
                sut_error_rate=0.0,
            )
            return run

        progress(f"M1 SUT inventory and indexed matching: {len(oracle_inv)} oracle definitions")
        prism_inv = filter_to_universe(sut.inventory(cfg["path"]), set(files))
        diff = inventory_diff(oracle_inv, prism_inv)
        run["m1"] = {
            "matched": len(diff.matched),
            "prism_missing": len(diff.prism_missing),
            "prism_extra": len(diff.prism_extra),
            "anon_oracle": diff.anon_oracle,
            "anon_prism": diff.anon_prism,
        }
        sp = snapshot_path(str(EVAL_DIR / "snapshots"), name, sha)
        if cfg["lang"] in ("ts", "js"):
            # A fresh native inventory preserves newly supported quoted methods.
            # Historical snapshots and anchors remain unchanged.
            snap = oracle_inv
            run["meta"]["sampling_inventory"] = "live"
        elif sp.exists():
            snap = load_snapshot(sp)
        else:
            snap = oracle_inv
            if not run["failures"]:
                save_snapshot(sp, snap)

        snapshot_matches = match_snapshot_to_prism(snap, prism_inv)
        run["m1"]["snapshot_prism_missing"] = len(snap) - len(snapshot_matches)

        n_defs: dict = {}
        for fd in snap:
            if fd.name:
                n_defs[fd.name] = n_defs.get(fd.name, 0) + 1
        per = getattr(args, "sample", None) or (3 if args.quick else defaults["per_stratum"])
        run["meta"]["sample_per_stratum"] = per
        pkg_dirs = package_dirs(cfg["path"]) if cfg["lang"] == "python" else None
        sample = sample_strata(
            snap,
            n_defs,
            cfg["lang"],
            defaults["seed"],
            per,
            package_dirs=pkg_dirs,
        )
        from collections import Counter
        oracle_lines = Counter((fd.location.file, fd.selection_line) for fd in snap)
        oracle_named_lines = Counter((fd.location.file, fd.selection_line, fd.name) for fd in snap)
        sut_lines = Counter((fd.location.file, fd.location.start_line) for fd in prism_inv)
        configure_seed_addresses(sut, prism_inv)
        ambiguous_lines = {line for line, count in oracle_lines.items() if count > 1}
        run["m1"]["shared_oracle_selection_lines"] = len(ambiguous_lines)
        run["m1"]["shared_sut_start_lines"] = sum(count > 1 for count in sut_lines.values())
        primed = {}
        if cfg["lang"] == "rust" and hasattr(oracle, "prime_hierarchy"):
            progress("validating complete Rust hierarchy sample; one bounded session recovery allowed")
            primed = oracle.prime_hierarchy([fd for fds in sample.values() for fd in fds], files, oracle_inv)
        strata_counts: dict = {}
        run["probes"]["_strata"] = {}
        t0 = time.monotonic()
        for stratum, fds in sample.items():
            progress(f"M2 {stratum}: {len(fds)} symbols")
            strata_counts[stratum] = {"eligible": len(fds) * 2, "successful": 0}
            run["probes"]["_strata"][stratum] = {
                "population_symbols": sum(classify(f, n_defs, cfg["lang"], pkg_dirs) == stratum for f in snap if f.name),
                "eligible_symbols": len(fds),
                "eligible_probes": len(fds) * 2,
                "target": per,
            }
            for fd in fds:
                sd = seed_id(fd, oracle_lines)
                pfd = snapshot_matches.get(fd)
                for direction in ("callers", "callees"):
                    pid = f"{direction}:{sd}"
                    progress(f"M2 {pid}")
                    if pfd in sut.unaddressable_seeds or pfd is not None and oracle_named_lines[
                            (fd.location.file, fd.selection_line, fd.name)] > 1:
                        acc.record(pid, "seed_unaddressable")
                        run["failures"][pid] = {"outcome": "seed_unaddressable",
                            "error": "SUT file:line seed cannot select one callable among same-line declarations"}
                        run["probes"][pid] = {**run["failures"][pid], "direction": direction,
                                              "stratum": stratum, "seed_def": sd}
                        continue
                    try:
                        osites = primed[(fd, direction)] if (fd, direction) in primed else (
                            oracle.callers(fd)
                            if direction == "callers"
                            else oracle.callees(fd)
                        )
                        if isinstance(osites, OracleError):
                            raise osites
                    except OracleError as exc:
                        failure(pid, exc)
                        run["probes"][pid] = {**run["failures"][pid], "direction": direction,
                                              "stratum": stratum, "seed_def": sd}
                        continue
                    if pfd is None:
                        outcome = "inventory_miss"
                        acc.record(pid, outcome)
                        psites = []
                    else:
                        try:
                            psites = (
                                sut.callers(cfg["path"], pfd)
                                if direction == "callers"
                                else sut.callees(cfg["path"], pfd)
                            )
                        except (SutError, SutAmbiguous) as exc:
                            acc.record(pid, "sut_error")
                            run["failures"][pid] = {"outcome": "sut_timeout" if isinstance(exc, SutTimeout) else "sut_error", "error": str(exc)}
                            run["probes"][pid] = {**run["failures"][pid], "direction": direction, "stratum": stratum, "seed_def": sd}
                            progress(f"{run['failures'][pid]['outcome']} {pid}: {exc}")
                            continue
                        outcome = "ok"
                        acc.record(pid, outcome)
                    strata_counts[stratum]["successful"] += 1
                    run["probes"][pid] = {
                        "outcome": outcome,
                        "direction": direction,
                        "stratum": stratum,
                        "seed_def": sd,
                        "seed_address_mode": "symbol" if pfd in sut.symbol_seeds else
                                             "interior_location" if pfd in sut.location_seeds else "location",
                        "seed_location_line": sut.location_seeds.get(pfd, pfd.location.start_line) if pfd else None,
                        "prism_sites": _stored_sites(psites, direction),
                        "oracle_sites": _stored_sites(osites, direction),
                        "prism_functions": _stored_functions(psites, snap),
                        "oracle_functions": _stored_functions(osites, snap),
                    }
        run["meta"]["wall_s"]["m2"] = round(time.monotonic() - t0, 3)

        if cfg.get("pinned_probes", name == "prism"):
            progress("pinned probes and capability matrix")
            t0 = time.monotonic()
            run["pinned"] = pinned_mod.run_pinned(oracle, sut, snap, cfg["path"],
                                                  prism_by_oracle=snapshot_matches)
            for p in run["pinned"]:
                if p.get("outcome") in ("oracle_error", "oracle_timeout", "sut_error", "sut_timeout"):
                    acc.record("pinned:" + p["id"], "sut_error" if p["outcome"].startswith("sut") else p["outcome"])
            run["meta"]["wall_s"]["pinned"] = round(time.monotonic() - t0, 3)
            t0 = time.monotonic()
            run["matrix"] = [
                matrix_result_to_json(r)
                for r in run_matrix(
                    EVAL_DIR / "fixtures",
                    sut,
                    MATRIX_LANGUAGES,
                )
            ]
            run["meta"]["wall_s"]["matrix"] = round(time.monotonic() - t0, 3)

        t0 = time.monotonic()
        progress("M3 definition spot checks")
        m3_cap = 10 if args.quick else 25
        run["m3"] = run_m3_spotcheck(run["probes"], oracle, snap, cfg["path"], m3_cap)
        run["meta"]["wall_s"]["m3"] = round(time.monotonic() - t0, 3)

        if cfg["oracle"] == "tsserver":
            from collections import Counter
            from .member_sample import syntax_census, sample_members
            progress("independent syntax census and property/getter definition sample")
            census = syntax_census(oracle, files)
            run["frame"] = {
                "hierarchy_named": len([f for f in oracle_inv if f.name]),
                "prism_named": len([f for f in prism_inv if f.name]),
                "prism_outside_hierarchy": len(diff.prism_extra),
                "outside_by_kind": dict(Counter(f.kind for f in diff.prism_extra)),
                "syntax_callable_by_shape": dict(Counter(d["shape"] for d in census["declarations"])),
                "policy": "native hierarchy inventory plus independent property/getter site sampling",
            }
            run["member_sites"] = sample_members(oracle, sut, prism_inv, census, per, defaults["seed"])
            for shape, measurement in run["member_sites"]["shapes"].items():
                for i, row in enumerate(measurement["rows"]):
                    if row["outcome"] in ("oracle_error", "oracle_timeout", "sut_error", "sut_timeout"):
                        run["failures"][f"member:{shape}:{i}"] = row

        run["meta"]["oracle_error_rate"] = acc.oracle_error_rate()
        run["meta"]["sut_error_rate"] = acc.sut_error_rate()
        ok, reasons = evaluate_floors(
            strata_counts,
            acc.oracle_error_rate(),
            acc.sut_error_rate(),
            defaults["oracle_error_floor"][cfg["lang"]],
            defaults["sut_error_floor"],
        )
        run["meta"]["baseline_invalid"] = (not ok) or bool(initial_invalid_reasons)
        run["meta"]["invalid_reasons"] = initial_invalid_reasons + reasons
        if any(pid.startswith("docsym:") for pid in run["failures"]):
            run["meta"]["invalid_reasons"].append("oracle_inventory_incomplete")
        if any(f["outcome"] == "oracle_timeout" for f in run["failures"].values()):
            run["meta"]["invalid_reasons"].append("oracle_timeout")
        if any(f["outcome"] == "sut_timeout" for f in run["failures"].values()):
            run["meta"]["invalid_reasons"].append("sut_timeout")
        if any(f["outcome"] == "seed_unaddressable" for f in run["failures"].values()):
            run["meta"]["invalid_reasons"].append("seed_unaddressable")
        if any(pid.startswith("member:") for pid in run["failures"]):
            run["meta"]["invalid_reasons"].append("member_sample_error")
        if any(c.get("verdict") in ("oracle_timeout", "oracle_error") for c in run["m3"]["checked"]):
            run["meta"]["invalid_reasons"].append("m3_oracle_error")
        for p in run.get("pinned", []):
            if p.get("outcome") in ("error", "oracle_error", "oracle_timeout", "sut_error", "sut_timeout"):
                run["meta"]["invalid_reasons"].append("pinned_" + p["outcome"])
                run["failures"]["pinned:" + p["id"]] = p
        if not any(p.get("outcome") in ("ok", "inventory_miss") for pid, p in run["probes"].items() if not pid.startswith("_")):
            run["meta"]["invalid_reasons"].append("no_scored_probes")
        run["meta"]["baseline_invalid"] = bool(run["meta"]["invalid_reasons"])
        adj = load_records(EVAL_DIR / "adjudications.jsonl")
        run["site_fingerprints"] = _site_fingerprints_for_probes(run["probes"], cfg["path"])
        run["m2"], run["pending"], stale = _compute_m2_and_pending(
            run["probes"], adj, run["site_fingerprints"]
        )
        run["meta"]["stale_adjudications"] = stale
        run["summary"] = summarize_m2(run["m2"])
        return run
    except OracleTimeout as exc:
        failure("lifecycle", exc)
        run["meta"].update(baseline_invalid=True, oracle_error_rate=1.0,
                             oracle_not_quiescent=oracle.not_quiescent,
                             invalid_reasons=initial_invalid_reasons + ["oracle_timeout"])
        return run
    except SutTimeout as exc:
        run["failures"]["lifecycle"] = {"outcome": "sut_timeout", "error": str(exc)}
        run["meta"].update(baseline_invalid=True, sut_error_rate=1.0,
                             invalid_reasons=initial_invalid_reasons + ["sut_timeout"])
        return run
    finally:
        run["meta"]["oracle_retries"] = getattr(oracle, "retries", [])
        run["meta"]["oracle_readiness"] = getattr(oracle, "readiness", [])
        run["meta"]["oracle_restarts"] = getattr(oracle, "restarts", [])
        run["oracle_filtered"] = getattr(oracle, "oracle_filtered", [])
        stop = getattr(oracle, "stop", None)
        if stop is not None:
            stop()
        if manifest_files is not None:
            try:
                _, final_drift = verify_source_manifest(cfg)
            except (OSError, ValueError) as exc:
                final_drift = [f"source_manifest_unavailable:{exc}"]
            if final_drift:
                run["meta"]["baseline_invalid"] = True
                run["meta"]["invalid_reasons"].extend(f"final:{d}" for d in final_drift)
        run["meta"]["wall_s"]["total"] = round(time.monotonic() - started, 3)


def invalid_corpus_run(name: str, cfg: dict, defaults: dict, args, exc: Exception) -> dict:
    try:
        csha = corpus_sha(cfg["path"])
        cdirty = corpus_dirty(cfg["path"])
    except Exception:
        csha = "unknown"
        cdirty = False
    try:
        hsha = corpus_sha(str(EVAL_DIR.parent))
    except Exception:
        hsha = "unknown"
    return {
        "meta": {
            "corpus": name,
            "corpus_sha": csha,
            "corpus_dirty": cdirty,
            "prism_sha": "unknown",
            "seed": defaults.get("seed"),
            "date": args.date,
            "harness_sha": hsha,
            "oracle": cfg.get("oracle", "unknown"),
            "oracle_not_quiescent": False,
            "oracle_error_rate": float(not isinstance(exc, (SutError, OSError, ValueError))),
            "sut_error_rate": float(isinstance(exc, SutError)),
            "baseline_invalid": True,
            "invalid_reasons": ["oracle_timeout" if isinstance(exc, OracleTimeout) else
                                "sut_error" if isinstance(exc, (SutError, SutAmbiguous)) else
                                "input_unavailable" if isinstance(exc, (OSError, ValueError)) else "oracle_unavailable"],
            "error": str(exc),
            "wall_s": {},
        },
        "probes": {"_corpus": name},
        "m2": {},
        "pending": [],
    }


def summarize_m2(m2: dict) -> dict:
    result = {}
    for direction, strata in m2.items():
        result[direction] = {}
        for tier in ("raw", "exact_tier"):
            rows = [s["raw"] if tier == "raw" else s[tier]["raw"] for s in strata.values()]
            result[direction][tier] = precision_recall(*(sum(r[k] for r in rows) for k in ("tp", "fp", "fn")))
    return result


def select_corpora(cfg, args):
    names = (list(cfg["corpus"]) if args.corpus == "all" else [args.corpus]) if args.corpus else (
        cfg.get("quick", {}).get("corpora", ["prism"]) if args.quick or args.lang else ["prism"])
    if args.lang:
        langs = set(args.lang.split(","))
        names = [n for n in names if cfg["corpus"][n]["lang"] in langs]
    if not names:
        raise ValueError("no corpora match the requested languages")
    return names


def main() -> int:
    ap = argparse.ArgumentParser(prog="tier-a")
    ap.add_argument("--corpus", default=None)
    ap.add_argument("--lang", help="filter selected corpora, e.g. ts,js")
    ap.add_argument("--sample", type=int, help="symbols per stratum; overrides quick's smoke sample of 3")
    ap.add_argument("--out-dir", type=Path, help="report directory (keeps exploratory runs separate from baseline)")
    for key in ("query_timeout_s", "startup_timeout_s", "oracle_budget_s"):
        ap.add_argument("--" + key.replace("_", "-"), type=float)
    ap.add_argument("--quick", action="store_true")
    ap.add_argument("--matrix-only", action="store_true")
    ap.add_argument("--report-only")
    ap.add_argument("--sut-bin")
    ap.add_argument("--allow-stale-sut", action="store_true")
    ap.add_argument("--allow-drift", action="store_true")
    ap.add_argument("--oracle", default=None,
                    help="override per-corpus oracle (e.g. basedpyright); "
                         "outputs are written under <corpus>-<oracle> so they "
                         "do not clobber the committed baseline anchors")
    ap.add_argument("--date", default=None)
    args = ap.parse_args()
    if args.sample is not None and args.sample <= 0:
        ap.error("sample must be positive")
    if any(getattr(args, k) is not None and (not math.isfinite(getattr(args, k)) or getattr(args, k) <= 0) for k in ("query_timeout_s", "startup_timeout_s", "oracle_budget_s")):
        ap.error("timeouts must be positive")
    if args.date is None:
        import datetime

        args.date = datetime.date.today().isoformat()
    out_dir = args.out_dir or EVAL_DIR.parent / "docs" / "eval" / "tier-a"
    if args.report_only:
        write_reports(
            recompute_metrics_from_stored(json.loads(Path(args.report_only).read_text())),
            out_dir,
        )
        return 0
    if args.matrix_only:
        sut = PrismCli(str(EVAL_DIR.parent), sut_bin=args.sut_bin,
                       allow_stale=args.allow_stale_sut)
        results = run_matrix(EVAL_DIR / "fixtures", sut, MATRIX_LANGUAGES)
        for r in results:
            print(f"{r.language}/{r.capability}: {r.outcome}")
        return 1 if any(r.outcome == "regression" for r in results) else 0
    cfg = load_corpora()
    names = select_corpora(cfg, args)
    rc = 0
    for name in names:
        corpus_cfg = cfg["corpus"][name]
        run_name = name
        if args.oracle:
            corpus_cfg = {**corpus_cfg, "oracle": args.oracle}
            run_name = f"{name}-{args.oracle}"
        try:
            run = run_corpus(run_name, corpus_cfg, cfg["defaults"], args)
        except SutStale as exc:
            # a stale SUT is the operator's problem, not the oracle's — abort the
            # whole invocation loudly instead of mislabeling it per-corpus
            print(f"FATAL sut_stale: {exc}", file=sys.stderr)
            return 3
        except Exception as exc:
            run = invalid_corpus_run(run_name, corpus_cfg, cfg["defaults"], args, exc)
        (EVAL_DIR / "runs").mkdir(exist_ok=True)
        (EVAL_DIR / "runs" / f"{args.date}-{run_name}.json").write_text(
            json.dumps(run, indent=1, sort_keys=True, default=str, allow_nan=False)
        )
        write_reports(run, out_dir)
        print(f"{run_name}: {'INVALID' if run['meta'].get('baseline_invalid') else 'VALID'} "
              f"wall_s={run['meta'].get('wall_s')} reasons={run['meta'].get('invalid_reasons', [])}", flush=True)
        if run.get("summary"):
            print(json.dumps(run["summary"], sort_keys=True, allow_nan=False), flush=True)
        if run["meta"].get("baseline_invalid"):
            rc = 2
        elif any(r.get("outcome") == "regression" for r in run.get("matrix", [])):
            rc = max(rc, 1)
    return rc


if __name__ == "__main__":
    sys.exit(main())
