"""Tier-A command dispatch and run preflights."""
from __future__ import annotations

import argparse
import datetime
import subprocess
import sys
import tomllib
from dataclasses import asdict
from pathlib import Path

from .boundary import (
    BoundaryViolation,
    Observation,
    OpenAudit,
    admit,
    load_declared_inputs,
    observe_cargo,
    scrubbed_env,
)
from .closure import raw_digest
from .corpus import universe
from .lock import (
    CorpusIdentity,
    Lock,
    corpus_check,
    integrity_check,
    load_lock,
    lock_identities,
    resolve_corpus_identity,
)
from .materialize import materialize, tier_a_root


EVAL_DIR = Path(__file__).resolve().parents[1]


def _load_optional_lock(eval_dir: Path) -> Lock | None:
    path = eval_dir / "tier-a.lock.toml"
    if not path.exists():
        return None
    try:
        return load_lock(path)
    except Exception as exc:
        raise BoundaryViolation([f"lock_invalid: {exc}"]) from exc


def _version(*command: str) -> str:
    return subprocess.run(
        command,
        check=True,
        capture_output=True,
        text=True,
    ).stdout.strip()


def _harness_sha(eval_dir: Path) -> str:
    return _version("git", "-C", str(eval_dir.parent), "rev-parse", "HEAD")


def _toolchain() -> dict:
    return {
        "rustc": _version("rustc", "--version"),
        "cargo": _version("cargo", "--version"),
        "python": sys.version.split()[0],
    }


def dispatch(argv: list[str]) -> int | None:
    if not argv or argv[0] not in {"lock-check", "bootstrap", "compare"}:
        return None
    if argv[0] == "bootstrap":
        return _bootstrap(argv[1:])
    if argv[0] == "compare":
        print(f"FATAL command_not_implemented: {argv[0]}", file=sys.stderr)
        return 3

    integrity = "--integrity" in argv[1:]
    corpus = None
    if "--corpus" in argv[1:]:
        index = argv.index("--corpus")
        if index + 1 >= len(argv):
            print("FATAL boundary_violation: missing corpus name", file=sys.stderr)
            return 3
        corpus = argv[index + 1]
    if not integrity and corpus is None:
        print(
            "FATAL boundary_violation: lock-check requires --integrity or --corpus",
            file=sys.stderr,
        )
        return 3
    try:
        if integrity:
            preflight_matrix(EVAL_DIR)
        if corpus is not None:
            if corpus != "prism":
                raise BoundaryViolation([f"unknown_corpus: {corpus}"])
            preflight_quick(EVAL_DIR, live=False)
    except BoundaryViolation as exc:
        print(f"FATAL boundary_violation: {exc}", file=sys.stderr)
        return 3
    return 0


def _bootstrap(argv: list[str]) -> int:
    from .bootstrap import (
        finalize_policy,
        mark_history,
        publish_anchor_v1,
        stage_candidate,
        validate_staged,
    )
    from .sut import PrismCli, SutStale

    parser = argparse.ArgumentParser(prog="tier-a bootstrap")
    parser.add_argument("--sut-bin")
    parser.add_argument("--finalize-policy-only", action="store_true")
    args = parser.parse_args(argv)
    finalize_policy(EVAL_DIR)
    if args.finalize_policy_only:
        return 0
    if args.sut_bin is None:
        parser.error("--sut-bin is required")
    try:
        root = tier_a_root()
        binary = Path(args.sut_bin).resolve()
        candidate = stage_candidate(
            EVAL_DIR.parent,
            EVAL_DIR,
            binary,
            {"cargo": {"features": "all"}},
            root,
            str(EVAL_DIR.parent),
        )
        staged_reasons = validate_staged(candidate)
        if staged_reasons:
            raise BoundaryViolation(staged_reasons)
        config = tomllib.loads((EVAL_DIR / "corpora.toml").read_text())
        run_args = argparse.Namespace(
            corpus="prism", quick=False, matrix_only=False, report_only=None,
            sut_bin=str(binary), allow_stale_sut=False, live=False,
            oracle=None, date=datetime.date.today().isoformat(), run_id=None,
            sut_archive=None,
        )
        sut = PrismCli.from_verified(
            str(EVAL_DIR.parent), str(binary),
            candidate.lock.prism["sut"]["sha"],
            candidate.lock.prism["sut"]["digest"],
            env=scrubbed_env(candidate.lock),
        )
        run, _overlay = run_quick_admitted(
            "prism", config["corpus"]["prism"], config["defaults"], run_args,
            EVAL_DIR, candidate=candidate, sut=sut,
        )
        published = publish_anchor_v1(
            candidate, run, EVAL_DIR, EVAL_DIR.parent / "docs/eval/tier-a",
            root / "sut",
        )
        mark_history(EVAL_DIR.parent / "docs/eval/tier-a/baseline.md", run_args.date)
        if integrity_check(published, EVAL_DIR):
            raise BoundaryViolation(integrity_check(published, EVAL_DIR))
    except (BoundaryViolation, SutStale, ValueError, OSError, subprocess.SubprocessError) as exc:
        print(f"FATAL bootstrap_refused: {exc}", file=sys.stderr)
        return 3
    return 0


def preflight_matrix(eval_dir: Path) -> Lock | None:
    lock = _load_optional_lock(eval_dir)
    try:
        reasons = integrity_check(lock, eval_dir)
    except Exception as exc:
        raise BoundaryViolation([f"lock_invalid: {exc}"]) from exc
    if reasons:
        raise BoundaryViolation(reasons)
    return lock


def preflight_quick(
    eval_dir: Path,
    live: bool,
) -> tuple[Lock | None, Path, dict, CorpusIdentity | None]:
    lock = _load_optional_lock(eval_dir)
    try:
        reasons = integrity_check(lock, eval_dir)
    except Exception as exc:
        raise BoundaryViolation([f"lock_invalid: {exc}"]) from exc
    if reasons:
        raise BoundaryViolation(reasons)
    if lock is not None:
        locked_harness_sha = lock.prism.get("harness_sha")
        computed_harness_sha = _harness_sha(eval_dir)
        if locked_harness_sha != computed_harness_sha:
            raise BoundaryViolation(
                [
                    "harness_sha_drift "
                    f"{locked_harness_sha} != {computed_harness_sha}"
                ]
            )
    if live:
        return (
            lock,
            eval_dir.parent,
            {
                "corpus_mode": "live",
                "baseline_invalid": True,
                "invalid_reasons": ["live_corpus"],
            },
            None,
        )
    if lock is None:
        return None, eval_dir.parent, {}, None

    try:
        checkout = materialize(lock, tier_a_root(), str(eval_dir.parent))
        reasons = corpus_check(
            lock,
            checkout,
            _version("rust-analyzer", "--version"),
            _toolchain(),
        )
        if reasons:
            raise BoundaryViolation(reasons)
        identity = resolve_corpus_identity(lock, checkout)
    except BoundaryViolation:
        raise
    except Exception as exc:
        raise BoundaryViolation([f"corpus_mismatch: {exc}"]) from exc
    identities = lock_identities(lock)
    identities.pop("harness_sha", None)
    return (
        lock,
        checkout,
        {
            **identities,
            "corpus_sha": identity.sha,
            "corpus_dirty": identity.dirty,
            "untracked_sources": identity.untracked,
            "prism_sha": lock.prism["sut"]["sha"][:12],
            "corpus_mode": "pinned",
        },
        identity,
    )


def finalize_metrics(run: dict, declared, defaults: dict, cfg: dict) -> dict:
    """Compute verdict-bearing metrics only after boundary admission."""
    from . import cli

    state = run.pop("_boundary_state", None)
    if state is not None:
        initial = state["initial_invalid_reasons"]
        ok, reasons = cli.evaluate_floors(
            state["strata_counts"],
            run["meta"].get("oracle_error_rate", 0.0),
            run["meta"].get("sut_error_rate", 0.0),
            defaults["oracle_error_floor"][cfg["lang"]],
            defaults["sut_error_floor"],
        )
        run["meta"]["baseline_invalid"] = (not ok) or bool(initial)
        run["meta"]["invalid_reasons"] = initial + reasons
    if "probes" in run:
        run["m2"], run["pending"], stale = cli._compute_m2_and_pending(
            run["probes"], declared.adjudications, run.get("site_fingerprints", {})
        )
        run["meta"]["stale_adjudications"] = stale
    return run


def _lockless_closure_paths(cfg: dict) -> set[str]:
    root = Path(cfg["path"])
    paths = set(
        universe(
            str(root),
            cfg["lang"],
            cfg.get("excludes", []),
            tracked_only=True,
        )
    )
    paths.update(
        name
        for name in (
            "Cargo.lock",
            "Cargo.toml",
            "build.rs",
            "rust-toolchain",
            "rust-toolchain.toml",
        )
        if (root / name).is_file()
    )
    return paths


def run_quick_admitted(
    name: str,
    cfg: dict,
    defaults: dict,
    args,
    eval_dir: Path,
    *,
    candidate=None,
    sut=None,
) -> tuple[dict, dict]:
    """Run quick observations, admit them, then compute verdict metrics."""
    from . import cli

    if candidate is None:
        lock, corpus_path, overlay, identity = preflight_quick(eval_dir, args.live)
    else:
        existing = _load_optional_lock(eval_dir)
        reasons = integrity_check(existing, eval_dir)
        if reasons:
            raise BoundaryViolation(reasons)
        if existing is not None:
            raise BoundaryViolation(["anchor_already_exists"])
        lock, overlay = candidate.lock, {}
        try:
            corpus_path = materialize(
                lock, candidate._root, candidate._origin_url
            )
            reasons = corpus_check(
                lock,
                corpus_path,
                _version("rust-analyzer", "--version"),
                _toolchain(),
            )
            if reasons:
                raise BoundaryViolation(reasons)
            identity = resolve_corpus_identity(lock, corpus_path)
        except BoundaryViolation:
            raise
        except Exception as exc:
            raise BoundaryViolation([f"corpus_mismatch: {exc}"]) from exc
    corpus_cfg = {**cfg, "path": str(corpus_path)}
    env = scrubbed_env(lock)
    with OpenAudit(eval_dir) as audit:
        declared = load_declared_inputs(
            lock,
            eval_dir,
            corpus=name,
            cfg=corpus_cfg,
        )
        run = cli.run_corpus(
            name,
            corpus_cfg,
            defaults,
            args,
            oracle_init=(lock.prism["oracle"]["init"] if lock is not None else None),
            lock_oracle_version=(
                lock.prism["oracle"]["version"] if lock is not None else None
            ),
            env=env,
            data_inputs=declared,
            corpus_identity=identity,
            sut=sut,
            compute_metrics=False,
        )
        run["meta"]["corpus"] = name
        run["meta"]["date"] = args.date

    sut_bin = Path(
        run.pop("_sut_bin", args.sut_bin or eval_dir.parent / "target/release/prism")
    )
    build_dir = sut_bin.parent if sut_bin.parent != Path(".") else eval_dir.parent / "target/release"
    env_reads, dep_paths = observe_cargo(build_dir, eval_dir.parent, env)
    observed = Observation(env_reads, dep_paths, audit.opens)
    closure_paths = (
        {row[0] for row in getattr(lock, "_corpus_rows", [])}
        if lock is not None
        else _lockless_closure_paths(corpus_cfg)
    )
    admit(lock, declared, observed, closure_paths, eval_dir)

    run["meta"]["observation"] = {
        "env_reads": [asdict(read) for read in observed.env_reads],
        "dep_paths": sorted(observed.dep_paths),
        "harness_opens": sorted(observed.harness_opens),
    }
    finalize_metrics(run, declared, defaults, corpus_cfg)

    run["meta"]["env_values"] = dict(env)
    run["meta"]["data"] = {
        path: {"digest": digest} for path, digest in sorted(declared.digests.items())
    }
    run["meta"]["data"]["eval/fixtures"] = {
        "digest": declared.fixtures_digest
    }
    if lock is None:
        run["meta"]["baseline_invalid"] = True
        run["meta"]["invalid_reasons"] = list(
            dict.fromkeys(run["meta"].get("invalid_reasons", []) + ["no_lock"])
        )
        run["meta"]["admitted"] = False
    else:
        observed = {
            key: run["meta"].get(key)
            for key in (
                "corpus_sha_full",
                "sut_sha_full",
                "oracle",
                "oracle_init",
                "harness_sha",
            )
        }
        run["meta"].update(lock_identities(lock))
        run["meta"]["corpus_sha_full"] = (
            identity.sha_full
            if identity is not None
            else run["meta"]["corpus_sha_full"]
        )
        for key, value in observed.items():
            if value is not None:
                run["meta"][key] = value
        run["meta"]["snapshot_digest"] = raw_digest(
            eval_dir.parent / lock.prism["snapshot"]["path"]
        )
        run["meta"]["admitted"] = True
    return run, overlay
