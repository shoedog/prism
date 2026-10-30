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

from . import commands, pinned as pinned_mod
from .accounting import CorpusAccounting, evaluate_floors
from .adjudication import load_records
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
from .matrix import MATRIX_LANGUAGES, run_matrix
from .oracles import OracleError, OracleTimeout
from .policy import (
    OracleInventoryResult,
    _compute_m2_and_pending,
    _dispatch_kind_for_site,
    _edges,
    _pair_metrics,
    _pending_for_probe,
    _pinned_accounting_outcome,
    _site_fingerprints_for_probes,
    _site_key,
    _site_parts,
    _stored_functions,
    _stored_sites,
    _tier_for_site,
    baseline_error_reasons,
    check_pinned,
    compute_m2_from_probes,
    initial_run_invalid_reasons,
    invalid_exception_metrics,
    is_live_corpus,
    match_snapshot_to_prism,
    matrix_result_to_json,
    oracle_failure_outcome,
    oracle_inventory,
    package_dirs,
    probe_outcome,
    recompute_metrics_from_stored,
    resolve_capability,
    run_m3_spotcheck,
    sample_per_stratum,
    seed_id,
    seed_is_unaddressable,
    select_corpora,
    spotcheck_cap,
    sut_failure_outcome,
    summarize_m2,
)
from .report import write_reports
from .strata import classify, filter_to_universe, inventory_diff, sample_strata
from .sut import PrismCli, SutAmbiguous, SutError, SutStale, SutTimeout, configure_seed_addresses
from .tsserver import TsserverOracle  # imported with the harness, not mid-run: a lazy import is an undeclared read under the boundary

EVAL_DIR = Path(__file__).resolve().parents[1]


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


def make_oracle(cfg: dict, init_options=None):
    from .oracles import LspOracle

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
        init_options=init_options,
        query_timeout_s=cfg.get("query_timeout_s", 10.0),
        startup_timeout_s=cfg.get("startup_timeout_s", 60.0),
        oracle_budget_s=cfg.get("oracle_budget_s", 600.0),
        cargo_features=cfg.get("cargo_features"),
    )


def run_corpus(
    name: str,
    cfg: dict,
    defaults: dict,
    args,
    *,
    corpus_identity=None,
    oracle_init=None,
    lock_oracle_version=None,
    env=None,
    data_inputs=None,
    sut=None,
    compute_metrics=True,
) -> dict:
    started = time.monotonic()

    def progress(message):
        if not compute_metrics:
            return
        print(
            f"[{name} +{time.monotonic() - started:.1f}s] {message}",
            file=sys.stderr,
            flush=True,
        )

    progress("checking SUT and corpus identity")
    sut = sut or PrismCli(
        str(EVAL_DIR.parent),
        sut_bin=args.sut_bin,
        allow_stale=args.allow_stale_sut,
        env=env,
    )
    manifest_files, source_drift = (
        verify_source_manifest(cfg)
        if corpus_identity is None and cfg.get("source_manifest")
        else (None, [])
    )
    sha = (
        corpus_identity.sha
        if corpus_identity is not None
        else cfg["pinned_sha"]
        if manifest_files is not None
        else corpus_sha(cfg["path"])
    )
    if corpus_identity is not None:
        pinned_ok, pinned_reason = True, ""
    else:
        pinned_ok, pinned_reason = check_pinned(
            cfg, sha, getattr(args, "allow_drift", False)
        )
    live_tree = is_live_corpus(
        name,
        bool(getattr(args, "live", False)),
        corpus_identity is not None,
        manifest_files is not None,
    )
    initial_invalid_reasons = initial_run_invalid_reasons(
        pinned_ok, pinned_reason, source_drift, live_tree
    )
    untracked = (
        corpus_identity.untracked
        if corpus_identity is not None
        else []
        if manifest_files is not None
        else untracked_sources(cfg["path"], cfg["lang"])
    )
    dirty_reasons = []
    if untracked:
        dirty_reasons.append(f"untracked_sources: {len(untracked)}")
    run: dict = {
        "meta": {
            "corpus": name,
            "corpus_sha": sha,
            "corpus_sha_full": (
                corpus_identity.sha_full if corpus_identity is not None else sha
            ),
            "corpus_dirty": (
                corpus_identity.dirty
                if corpus_identity is not None
                else bool(source_drift)
                if manifest_files is not None
                else corpus_dirty(cfg["path"]) or bool(untracked)
            ),
            "untracked_sources": untracked,
            "corpus_identity": {
                "component": (
                    "tier_a_lock"
                    if corpus_identity is not None
                    else "source_manifest"
                    if manifest_files is not None
                    else "git_head"
                ),
                "expected": (
                    corpus_identity.sha_full
                    if corpus_identity is not None
                    else cfg.get("source_manifest_sha256", cfg.get("pinned_sha"))
                ),
                "observed": (
                    corpus_identity.sha_full
                    if corpus_identity is not None
                    else hashlib.sha256(Path(cfg["source_manifest"]).read_bytes()).hexdigest()
                    if manifest_files is not None
                    else sha
                ),
                "manifest_sha256": cfg.get("source_manifest_sha256"),
                "drift": source_drift,
            },
            "language": cfg["lang"],
            "oracle_configuration": {"cargo_features": cfg.get("cargo_features")},
            "corpus_dirty_reasons": dirty_reasons,
            "prism_sha": sut.sha,
            "sut_sha_full": getattr(sut, "sha_full", sut.sha),
            "prism_dirty": sut.dirty,
            "seed": defaults["seed"],
            "date": args.date,
            "harness_sha": commands._harness_sha(EVAL_DIR),
            "oracle_not_quiescent": False,
            "oracle": cfg["oracle"],
            "oracle_error_rate": 0.0,
            "sut_error_rate": 0.0,
            "baseline_invalid": bool(initial_invalid_reasons),
            "invalid_reasons": initial_invalid_reasons,
            "stale_adjudications": 0,
            "wall_s": {},
        },
        "probes": {"_corpus": name},
        "failures": {},
    }
    if getattr(sut, "identity_note", None):
        run["meta"]["sut_identity_note"] = sut.identity_note
    if live_tree:
        run["meta"]["corpus_mode"] = "live"
    if env is not None:
        run["meta"]["env_values"] = dict(env)
    if not compute_metrics:
        run["_sut_bin"] = getattr(
            sut, "bin", args.sut_bin or EVAL_DIR.parent / "target/release/prism"
        )
        # Keep the private executable snapshot alive through admission's
        # post-run digest and provenance observation.
        run["_sut_owner"] = sut
    if oracle_init is not None:
        run["meta"]["oracle_init"] = oracle_init
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
    oracle = (
        make_oracle(oracle_cfg, init_options=oracle_init)
        if oracle_init is not None
        else make_oracle(oracle_cfg)
    )
    acc = CorpusAccounting()

    def failure(pid, exc):
        outcome = oracle_failure_outcome(exc)
        acc.record(pid, outcome)
        run["failures"][pid] = {"outcome": outcome, "error": str(exc)}
        progress(f"{outcome} {pid}: {exc}")
    try:
        t0 = time.monotonic()
        progress(f"starting {cfg['oracle']}")
        oracle.start()
        run["meta"]["wall_s"]["oracle_start"] = round(time.monotonic() - t0, 3)
        oracle_version = oracle.version()
        run["meta"]["oracle"] = oracle_version
        if (
            lock_oracle_version is not None
            and oracle_version != lock_oracle_version
        ):
            initial_invalid_reasons.append(
                f"oracle_version_mismatch: {oracle_version} != "
                f"{lock_oracle_version}"
            )
        if cfg["oracle"] == "tsserver":
            environment = {
                "oracle": oracle_version,
                "node_modules": node_modules_environment(cfg["path"]),
            }
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

        files = (
            corpus_identity.universe
            if corpus_identity is not None
            else universe(
                cfg["path"],
                cfg["lang"],
                cfg.get("excludes", []),
                tracked_only=manifest_files is None,
            )
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
        if data_inputs is not None:
            snap = data_inputs.snapshot
        elif cfg["lang"] in ("ts", "js"):
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
        per = sample_per_stratum(
            getattr(args, "sample", None), args.quick, defaults["per_stratum"]
        )
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
                    if seed_is_unaddressable(
                        pfd,
                        sut.unaddressable_seeds,
                        oracle_named_lines[
                            (fd.location.file, fd.selection_line, fd.name)
                        ]
                        > 1,
                    ):
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
                    outcome = probe_outcome(pfd)
                    if outcome == "inventory_miss":
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
                            run["failures"][pid] = {
                                "outcome": sut_failure_outcome(exc),
                                "error": str(exc),
                            }
                            run["probes"][pid] = {**run["failures"][pid], "direction": direction, "stratum": stratum, "seed_def": sd}
                            progress(f"{run['failures'][pid]['outcome']} {pid}: {exc}")
                            continue
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
                accounting_outcome = _pinned_accounting_outcome(p)
                if accounting_outcome is not None:
                    acc.record("pinned:" + p["id"], accounting_outcome)
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
        m3_cap = spotcheck_cap(args.quick)
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
        run["probe_status"] = {
            pid: outcome
            for pid, outcome in acc.probes.items()
            if not pid.startswith("docsym:")
        }
        run["site_fingerprints"] = _site_fingerprints_for_probes(run["probes"], cfg["path"])
        error_reasons, pinned_failures = baseline_error_reasons(
            run["failures"], run["m3"], run.get("pinned", []), run["probes"]
        )
        run["failures"].update(pinned_failures)
        metric_initial_reasons = list(dict.fromkeys(initial_invalid_reasons + error_reasons))
        if compute_metrics:
            ok, reasons = evaluate_floors(
                strata_counts,
                acc.oracle_error_rate(),
                acc.sut_error_rate(),
                defaults["oracle_error_floor"][cfg["lang"]],
                defaults["sut_error_floor"],
            )
            run["meta"]["baseline_invalid"] = (not ok) or bool(metric_initial_reasons)
            run["meta"]["invalid_reasons"] = metric_initial_reasons + reasons
            adj = (
                data_inputs.adjudications
                if data_inputs is not None
                else load_records(EVAL_DIR / "adjudications.jsonl")
            )
            run["m2"], run["pending"], stale = _compute_m2_and_pending(
                run["probes"], adj, run["site_fingerprints"]
            )
            run["meta"]["stale_adjudications"] = stale
            run["summary"] = summarize_m2(run["m2"])
        else:
            run["_boundary_state"] = {
                "strata_counts": strata_counts,
                "initial_invalid_reasons": metric_initial_reasons,
            }
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


def invalid_corpus_run(
    name: str,
    cfg: dict,
    defaults: dict,
    args,
    exc: Exception,
    corpus_identity=None,
) -> dict:
    if corpus_identity is not None:
        csha = corpus_identity.sha
        csha_full = corpus_identity.sha_full
        cdirty = corpus_identity.dirty
    else:
        try:
            csha = corpus_sha(cfg["path"])
            csha_full = csha
            cdirty = corpus_dirty(cfg["path"])
        except Exception:
            csha = "unknown"
            csha_full = "unknown"
            cdirty = False
    try:
        hsha = commands._harness_sha(EVAL_DIR)
    except Exception:
        hsha = "unknown"
    oracle_error_rate, sut_error_rate, invalid_reason = invalid_exception_metrics(exc)
    return {
        "meta": {
            "corpus": name,
            "corpus_sha": csha,
            "corpus_sha_full": csha_full,
            "corpus_dirty": cdirty,
            "prism_sha": "unknown",
            "seed": defaults.get("seed"),
            "date": args.date,
            "harness_sha": hsha,
            "oracle": cfg.get("oracle", "unknown"),
            "oracle_not_quiescent": False,
            "oracle_error_rate": oracle_error_rate,
            "sut_error_rate": sut_error_rate,
            "baseline_invalid": True,
            "invalid_reasons": [invalid_reason],
            "error": str(exc),
            "wall_s": {},
        },
        "probes": {"_corpus": name},
        "m2": {},
        "pending": [],
    }


def main() -> int:
    command_rc = commands.dispatch(sys.argv[1:])
    if command_rc is not None:
        return command_rc
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
    ap.add_argument(
        "--live", action="store_true", help="measure the working tree, never anchoring"
    )
    ap.add_argument("--run-id")
    ap.add_argument("--sut-archive")
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
        try:
            lock = commands.preflight_matrix(EVAL_DIR)
        except commands.BoundaryViolation as exc:
            print(f"FATAL boundary_violation: {exc}", file=sys.stderr)
            return 3
        sut = PrismCli(
            str(EVAL_DIR.parent),
            sut_bin=args.sut_bin,
            allow_stale=args.allow_stale_sut,
            env=commands.scrubbed_env(lock),
        )
        results = run_matrix(EVAL_DIR / "fixtures", sut, MATRIX_LANGUAGES)
        for r in results:
            print(f"{r.language}/{r.capability}: {r.outcome}")
        return 1 if any(r.outcome == "regression" for r in results) else 0
    cfg = load_corpora()
    names = select_corpora(cfg, args)
    rc = 0
    for name in names:
        corpus_cfg = cfg["corpus"][name]
        identity = None
        overlay = {}
        run_name = name
        if args.oracle:
            corpus_cfg = {**corpus_cfg, "oracle": args.oracle}
            run_name = f"{name}-{args.oracle}"
        try:
            if args.quick and name == "prism":
                run, overlay = commands.run_quick_admitted(
                    run_name,
                    corpus_cfg,
                    cfg["defaults"],
                    args,
                    EVAL_DIR,
                )
            else:
                run = run_corpus(run_name, corpus_cfg, cfg["defaults"], args)
        except commands.BoundaryViolation as exc:
            print(f"FATAL boundary_violation: {exc}", file=sys.stderr)
            return 3
        except SutStale as exc:
            # a stale SUT is the operator's problem, not the oracle's — abort the
            # whole invocation loudly instead of mislabeling it per-corpus
            print(f"FATAL sut_stale: {exc}", file=sys.stderr)
            return 3
        except Exception as exc:
            run = invalid_corpus_run(
                run_name,
                corpus_cfg,
                cfg["defaults"],
                args,
                exc,
                corpus_identity=identity,
            )
        run["meta"].setdefault("corpus", run_name)
        run["meta"].setdefault("date", args.date)
        if args.run_id:
            run["meta"]["corpus"] = f"{run_name}-{args.run_id}"
        if overlay.get("baseline_invalid"):
            run["meta"]["baseline_invalid"] = True
            run["meta"]["invalid_reasons"] = list(
                dict.fromkeys(
                    run["meta"].get("invalid_reasons", [])
                    + overlay.get("invalid_reasons", [])
                )
            )
        run["meta"].update(
            {
                key: value
                for key, value in overlay.items()
                if key not in {"baseline_invalid", "invalid_reasons"}
            }
        )
        for key, value in (
            ("oracle_error_rate", 0.0),
            ("sut_error_rate", 0.0),
            ("oracle_not_quiescent", False),
            ("wall_s", {}),
        ):
            run["meta"].setdefault(key, value)
        (EVAL_DIR / "runs").mkdir(exist_ok=True)
        artifact_name = run["meta"]["corpus"]
        (EVAL_DIR / "runs" / f"{args.date}-{artifact_name}.json").write_text(
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
