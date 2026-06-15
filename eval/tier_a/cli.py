"""Runner CLI (spec §2.11). Glue over the tested layers.

G3 replay property: raw metric inputs are stored in the run JSON's ``probes`` key;
corrected metrics additionally apply the current adjudication store.
"""
from __future__ import annotations

import argparse
import json
import os
import sys
import time
import tomllib
from pathlib import Path

from . import pinned as pinned_mod
from .accounting import CorpusAccounting, evaluate_floors
from .adjudication import apply_verdicts, load_records
from .compare import caller_fn_sets, site_compare
from .corpus import (
    corpus_dirty,
    corpus_sha,
    load_snapshot,
    save_snapshot,
    snapshot_path,
    untracked_sources,
    universe,
)
from .lsp_client import LspError
from .matrix import run_matrix
from .metrics import precision_recall
from .model import CallEdge, FunctionDef, Location, match_by_selection
from .oracles import OracleError
from .report import write_reports
from .spotcheck import classify_site, find_call_position
from .strata import filter_to_universe, inventory_diff, sample_strata
from .sut import PrismCli, SutAmbiguous, SutError, SutStale

EVAL_DIR = Path(__file__).resolve().parents[1]


def _edges(sites: list, direction: str) -> list[CallEdge]:
    """Rebuild minimal CallEdges from stored [file, start, end] site triples."""
    dummy = FunctionDef("_", "function", None, Location("_", 1, 1), 1)
    return [
        CallEdge(direction, dummy, None, None, Location(f, s, e))
        for f, s, e in sites
    ]


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
        for triple in list(p.get("prism_sites", [])) + list(p.get("oracle_sites", [])):
            by_file.setdefault(triple[0], set()).add(triple[1])
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
    diff = site_compare(
        _edges(probe["prism_sites"], direction),
        _edges(probe["oracle_sites"], direction),
    )
    adjudicated = {
        (r.direction, r.site)
        for r in adjudications
        if r.corpus == corpus
        and r.measurement == direction
        and r.seed_def == probe["seed_def"]
    }
    pending = []
    for file, line in sorted(diff.fp):
        key = ("prism_only", _site_key(file, line))
        if key not in adjudicated:
            pending.append({
                "corpus": corpus,
                "measurement": direction,
                "direction": "prism_only",
                "seed_def": probe["seed_def"],
                "site": key[1],
                "site_fingerprint": site_fps.get(key[1]),
            })
    for file, line in sorted(diff.fn):
        key = ("oracle_only", _site_key(file, line))
        if key not in adjudicated:
            pending.append({
                "corpus": corpus,
                "measurement": direction,
                "direction": "oracle_only",
                "seed_def": probe["seed_def"],
                "site": key[1],
                "site_fingerprint": site_fps.get(key[1]),
            })
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
            r = site_compare(
                _edges(p["prism_sites"], direction),
                _edges(p["oracle_sites"], direction),
            )
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
                },
            )
            s["tp"] += len(r.tp)
            s["fp"] += len(r.fp)
            s["fn"] += len(r.fn)
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
            except (OracleError, LspError):
                return False
    return False


def make_oracle(cfg: dict):
    from .oracles import LspOracle

    cmd = {
        "rust-analyzer": ["rust-analyzer"],
        "gopls": ["gopls", "serve"],
        "pyright": ["pyright-langserver", "--stdio"],
    }[cfg["oracle"]]
    return LspOracle(
        cmd,
        cfg["path"],
        cfg["lang"],
        settle_s=cfg.get("settle_s", 2.0),
        quiescence_cap_s=cfg.get("quiescence_cap_s", 300.0),
    )


def _stored_sites(edges: list[CallEdge], direction: str) -> list[list]:
    return [
        [e.call_site.file, e.call_site.start_line, e.call_site.end_line]
        for e in edges
        if direction == "callers" or e.other_def is not None
    ]


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
    used: set[FunctionDef] = set()
    matched = {}
    for fd in snapshot:
        match = match_by_selection(fd, [p for p in prism_inventory if p not in used])
        if match is not None:
            used.add(match)
            matched[fd] = match
    return matched


def matrix_result_to_json(result) -> dict:
    return {
        "capability": result.capability,
        "language": result.language,
        "outcome": result.outcome,
        "got": [list(site) for site in sorted(result.got)],
        "expected": [list(site) for site in sorted(result.expected)],
    }


def run_m3_spotcheck(
    probes: dict,
    oracle,
    snapshot: list[FunctionDef],
    corpus_root: str,
    cap: int,
) -> dict:
    """Spot-check prism-only caller sites against oracle definitions."""
    by_seed = {f"{fd.location.file}:{fd.selection_line}": fd for fd in snapshot}
    counts = {"confirmed_tp": 0, "confirmed_fp": 0, "ambiguous": 0, "alias_site": 0}
    checked = []
    for pid, probe in sorted(probes.items()):
        if len(checked) >= cap:
            break
        if pid == "_corpus" or probe.get("direction") != "callers":
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
                    defs = oracle.definitions_at(snapshot, file, line, char)
                except OracleError:
                    verdict = "ambiguous"
                else:
                    verdict = classify_site(text, seed.name, defs, seed)
            else:
                verdict = classify_site(text, seed.name, defs, seed)
            counts[verdict] = counts.get(verdict, 0) + 1
            checked.append({"probe": pid, "site": f"{file}:{line}", "verdict": verdict})
    return {"cap": cap, "checked": checked, "counts": counts}


def run_corpus(name: str, cfg: dict, defaults: dict, args) -> dict:
    sut = PrismCli(str(EVAL_DIR.parent), sut_bin=args.sut_bin,
                   allow_stale=args.allow_stale_sut)
    sha = corpus_sha(cfg["path"])
    pinned_ok, pinned_reason = check_pinned(
        cfg,
        sha,
        getattr(args, "allow_drift", False),
    )
    initial_invalid_reasons = [] if pinned_ok else [pinned_reason]
    untracked = untracked_sources(cfg["path"], cfg["lang"])
    dirty_reasons = []
    if untracked:
        dirty_reasons.append(f"untracked_sources: {len(untracked)}")
    run: dict = {
        "meta": {
            "corpus": name,
            "corpus_sha": sha,
            "corpus_dirty": corpus_dirty(cfg["path"]) or bool(untracked),
            "corpus_dirty_reasons": dirty_reasons,
            "prism_sha": sut.sha,
            "prism_dirty": sut.dirty,
            "seed": defaults["seed"],
            "date": args.date,
            "harness_sha": corpus_sha(str(EVAL_DIR.parent)),
            "oracle_not_quiescent": False,
            "baseline_invalid": not pinned_ok,
            "invalid_reasons": initial_invalid_reasons,
            "stale_adjudications": 0,
            "wall_s": {},
        },
        "probes": {"_corpus": name},
    }
    oracle_cfg = {
        **cfg,
        "settle_s": defaults.get("settle_s", 2.0),
        "quiescence_cap_s": defaults.get("quiescence_cap_s", 300.0),
    }
    oracle = make_oracle(oracle_cfg)
    try:
        t0 = time.monotonic()
        oracle.start()
        run["meta"]["wall_s"]["oracle_start"] = round(time.monotonic() - t0, 3)
        run["meta"]["oracle"] = oracle.version()
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
            tracked_only=True,
        )
        acc = CorpusAccounting()
        oracle_inv = []
        t0 = time.monotonic()
        for f in files:
            try:
                oracle_inv.extend(oracle.document_symbols(f))
            except OracleError:
                acc.record(f"docsym:{f}", "oracle_error")
        run["meta"]["wall_s"]["m1_oracle_inventory"] = round(
            time.monotonic() - t0,
            3,
        )

        if not resolve_capability(oracle, overlay_probe_ok, oracle_inv):
            run["meta"].update(
                baseline_invalid=True,
                invalid_reasons=initial_invalid_reasons + ["oracle_unsupported"],
                oracle_error_rate=1.0,
                sut_error_rate=0.0,
            )
            return run

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
        if sp.exists():
            snap = load_snapshot(sp)
        else:
            snap = oracle_inv
            save_snapshot(sp, snap)

        snapshot_matches = match_snapshot_to_prism(snap, prism_inv)
        run["m1"]["snapshot_prism_missing"] = len(snap) - len(snapshot_matches)

        n_defs: dict = {}
        for fd in snap:
            if fd.name:
                n_defs[fd.name] = n_defs.get(fd.name, 0) + 1
        per = 3 if args.quick else defaults["per_stratum"]
        pkg_dirs = package_dirs(cfg["path"]) if cfg["lang"] == "python" else None
        sample = sample_strata(
            snap,
            n_defs,
            cfg["lang"],
            defaults["seed"],
            per,
            package_dirs=pkg_dirs,
        )
        strata_counts: dict = {}
        run["probes"]["_strata"] = {}
        t0 = time.monotonic()
        for stratum, fds in sample.items():
            strata_counts[stratum] = {"eligible": len(fds) * 2, "successful": 0}
            run["probes"]["_strata"][stratum] = {
                "eligible_symbols": len(fds),
                "eligible_probes": len(fds) * 2,
                "target": per,
            }
            for fd in fds:
                sd = f"{fd.location.file}:{fd.selection_line}"
                pfd = snapshot_matches.get(fd)
                for direction in ("callers", "callees"):
                    pid = f"{direction}:{sd}"
                    try:
                        osites = (
                            oracle.callers(fd)
                            if direction == "callers"
                            else oracle.callees(fd)
                        )
                    except OracleError:
                        acc.record(pid, "oracle_error")
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
                        except (SutError, SutAmbiguous):
                            acc.record(pid, "sut_error")
                            continue
                        outcome = "ok"
                        acc.record(pid, outcome)
                    strata_counts[stratum]["successful"] += 1
                    run["probes"][pid] = {
                        "outcome": outcome,
                        "direction": direction,
                        "stratum": stratum,
                        "seed_def": sd,
                        "prism_sites": _stored_sites(psites, direction),
                        "oracle_sites": _stored_sites(osites, direction),
                        "prism_functions": _stored_functions(psites, snap),
                        "oracle_functions": _stored_functions(osites, snap),
                    }
        run["meta"]["wall_s"]["m2"] = round(time.monotonic() - t0, 3)

        if name == "prism":
            t0 = time.monotonic()
            run["pinned"] = pinned_mod.run_pinned(oracle, sut, snap, cfg["path"],
                                                  prism_by_oracle=snapshot_matches)
            run["meta"]["wall_s"]["pinned"] = round(time.monotonic() - t0, 3)
            t0 = time.monotonic()
            run["matrix"] = [
                matrix_result_to_json(r)
                for r in run_matrix(
                    EVAL_DIR / "fixtures",
                    sut,
                    ["rust", "go", "python"],
                )
            ]
            run["meta"]["wall_s"]["matrix"] = round(time.monotonic() - t0, 3)

        t0 = time.monotonic()
        m3_cap = 10 if args.quick else 25
        run["m3"] = run_m3_spotcheck(run["probes"], oracle, snap, cfg["path"], m3_cap)
        run["meta"]["wall_s"]["m3"] = round(time.monotonic() - t0, 3)

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
        adj = load_records(EVAL_DIR / "adjudications.jsonl")
        run["site_fingerprints"] = _site_fingerprints_for_probes(run["probes"], cfg["path"])
        run["m2"], run["pending"], stale = _compute_m2_and_pending(
            run["probes"], adj, run["site_fingerprints"]
        )
        run["meta"]["stale_adjudications"] = stale
        return run
    finally:
        stop = getattr(oracle, "stop", None)
        if stop is not None:
            stop()


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
            "oracle_error_rate": 1.0,
            "sut_error_rate": 0.0,
            "baseline_invalid": True,
            "invalid_reasons": ["oracle_unavailable"],
            "error": str(exc),
            "wall_s": {},
        },
        "probes": {"_corpus": name},
        "m2": {},
        "pending": [],
    }


def main() -> int:
    ap = argparse.ArgumentParser(prog="tier-a")
    ap.add_argument("--corpus", default="prism")
    ap.add_argument("--quick", action="store_true")
    ap.add_argument("--matrix-only", action="store_true")
    ap.add_argument("--report-only")
    ap.add_argument("--sut-bin")
    ap.add_argument("--allow-stale-sut", action="store_true")
    ap.add_argument("--allow-drift", action="store_true")
    ap.add_argument("--date", default=None)
    args = ap.parse_args()
    if args.date is None:
        import datetime

        args.date = datetime.date.today().isoformat()
    out_dir = EVAL_DIR.parent / "docs" / "eval" / "tier-a"
    if args.report_only:
        write_reports(
            recompute_metrics_from_stored(json.loads(Path(args.report_only).read_text())),
            out_dir,
        )
        return 0
    if args.matrix_only:
        sut = PrismCli(str(EVAL_DIR.parent), sut_bin=args.sut_bin,
                       allow_stale=args.allow_stale_sut)
        results = run_matrix(EVAL_DIR / "fixtures", sut, ["rust", "go", "python"])
        for r in results:
            print(f"{r.language}/{r.capability}: {r.outcome}")
        return 1 if any(r.outcome == "regression" for r in results) else 0
    cfg = load_corpora()
    names = ["prism"] if args.quick else (
        list(cfg["corpus"]) if args.corpus == "all" else [args.corpus]
    )
    rc = 0
    for name in names:
        corpus_cfg = cfg["corpus"][name]
        try:
            run = run_corpus(name, corpus_cfg, cfg["defaults"], args)
        except SutStale as exc:
            # a stale SUT is the operator's problem, not the oracle's — abort the
            # whole invocation loudly instead of mislabeling it per-corpus
            print(f"FATAL sut_stale: {exc}", file=sys.stderr)
            return 3
        except Exception as exc:
            run = invalid_corpus_run(name, corpus_cfg, cfg["defaults"], args, exc)
        (EVAL_DIR / "runs").mkdir(exist_ok=True)
        (EVAL_DIR / "runs" / f"{args.date}-{name}.json").write_text(
            json.dumps(run, indent=1, sort_keys=True, default=str, allow_nan=False)
        )
        write_reports(run, out_dir)
        if run["meta"].get("baseline_invalid"):
            rc = 2
        elif any(r.get("outcome") == "regression" for r in run.get("matrix", [])):
            rc = max(rc, 1)
    return rc


if __name__ == "__main__":
    sys.exit(main())
