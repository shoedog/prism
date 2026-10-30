"""Tier-A admission, command execution, and run preflights."""
from __future__ import annotations

import argparse
import datetime
import json
import math
import subprocess
import sys
import tomllib
import traceback
from dataclasses import asdict
from pathlib import Path

from .artifacts import ArtifactPaths
from .boundary import (
    BoundaryViolation,
    Observation,
    OpenAudit,
    admit,
    authenticate_sut_inputs,
    load_declared_inputs,
    observe_archive,
    observe_cargo,
    scrubbed_env,
)
from .closure import corpus_control_paths, raw_digest
from .adjudication import load_records
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
from .launch import launch_refusal_reason
from .record_schema import record_refusals
from . import policy as run_policy


EVAL_DIR = Path(__file__).resolve().parents[1]

OBSERVED_RUN_IDENTITIES = (
    "corpus_sha_full",
    "sut_sha_full",
    "sut_digest",
    "oracle",
    "oracle_init",
    "snapshot_digest",
    "env_values",
    "harness_sha",
)


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
        parser = argparse.ArgumentParser(prog="tier-a compare")
        parser.add_argument("--base", required=True, type=Path)
        parser.add_argument("--candidate", required=True, type=Path)
        parser.add_argument("--out", required=True, type=Path)
        args = parser.parse_args(argv[1:])
        result = compare(args.base, args.candidate, args.out)
        return 0 if result["verdict"] == "NO_REGRESSION" else 2

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


def _repo_run(path: Path, repo_root: Path) -> tuple[Path, dict]:
    resolved_root = repo_root.resolve(strict=True)
    resolved_path = path.resolve(strict=True)
    relative_path = resolved_path.relative_to(resolved_root).as_posix()
    return resolved_path, {
        "path_base": "repo_root",
        "path": relative_path,
        "digest": raw_digest(resolved_path),
    }


def _paths_alias(left: Path, right: Path) -> bool:
    try:
        if left.exists() and right.exists() and left.samefile(right):
            return True
    except OSError:
        pass
    return left.resolve(strict=False) == right.resolve(strict=False)


def _refuse_compare_output_aliases(
    outputs: tuple[Path, ...], inputs: tuple[Path, ...]
) -> None:
    for index, output in enumerate(outputs):
        for peer in outputs[index + 1 :]:
            if _paths_alias(output, peer):
                raise ValueError(
                    f"compare_outputs_alias_each_other: {output} -> {peer}"
                )
    for output in outputs:
        for source in inputs:
            if _paths_alias(output, source):
                raise ValueError(
                    f"compare_output_aliases_input: {output} -> {source}"
                )


def compare(
    base: Path,
    candidate: Path,
    out: Path,
    adjudications: Path = EVAL_DIR / "adjudications.jsonl",
    repo_root: Path = EVAL_DIR.parent,
) -> dict:
    """Compare two stored runs and write the JSON/Markdown report pair."""
    from .compare_report import compare_runs, render_compare_md

    base, base_record = _repo_run(base, repo_root)
    candidate, candidate_record = _repo_run(candidate, repo_root)
    adjudications = adjudications.resolve(strict=True)
    outputs = (Path(f"{out}.json"), Path(f"{out}.md"))
    _refuse_compare_output_aliases(outputs, (base, candidate, adjudications))
    run_b = json.loads(base.read_text())
    run_c = json.loads(candidate.read_text())
    records = load_records(adjudications)
    digest = raw_digest(adjudications)
    result = compare_runs(run_b, run_c, (records, digest))
    result["runs"] = {
        "base": base_record,
        "candidate": candidate_record,
    }
    result["meta"] = {
        "harness_sha": _harness_sha(EVAL_DIR),
        "generated_at": datetime.datetime.now(datetime.timezone.utc).isoformat(),
    }
    if "adjudications_digest" not in result["verdict"]:
        out.parent.mkdir(parents=True, exist_ok=True)
        outputs[0].write_text(
            json.dumps(result, indent=1, sort_keys=True, allow_nan=False)
        )
        outputs[1].write_text(render_compare_md(result))
    return result


def _bootstrap(argv: list[str]) -> int:
    from .bootstrap import (
        finalize_policy,
        mark_history,
        prepare_snapshot_only,
        publish_anchor_v1,
        stage_candidate,
        validate_staged,
        verify_re_anchor_previous,
    )
    from .sut import PrismCli, SutStale

    def refuse(exc: Exception) -> int:
        named = (BoundaryViolation, SutStale, ValueError, subprocess.SubprocessError)
        if not isinstance(exc, named):
            lines = "".join(
                traceback.format_exception(type(exc), exc, exc.__traceback__, limit=4)
            ).rstrip().splitlines()
            if len(lines) > 14:
                lines = [lines[0], *lines[-13:]]
            print("\n".join(lines), file=sys.stderr)
        print(f"FATAL bootstrap_refused: {exc}", file=sys.stderr)
        return 3

    parser = argparse.ArgumentParser(prog="tier-a bootstrap")
    parser.add_argument("--sut-bin")
    mode = parser.add_mutually_exclusive_group()
    mode.add_argument("--finalize-policy-only", action="store_true")
    mode.add_argument("--prepare-snapshot-only", action="store_true")
    mode.add_argument("--re-anchor", action="store_true")
    args = parser.parse_args(argv)
    if args.prepare_snapshot_only:
        if args.sut_bin is None:
            parser.error("--sut-bin is required")
        try:
            _snapshot, present, sut_sha, sut_digest = prepare_snapshot_only(
                EVAL_DIR.parent,
                EVAL_DIR,
                Path(args.sut_bin),
                {"cargo": {"features": "all"}},
            )
            if present:
                print("snapshot present")
            else:
                print(f"snapshot prepared: sha={sut_sha} digest={sut_digest}")
        except Exception as exc:
            return refuse(exc)
        return 0
    if args.finalize_policy_only:
        finalize_policy(EVAL_DIR)
        return 0
    if args.sut_bin is None:
        parser.error("--sut-bin is required")
    staged_snapshot = None
    try:
        previous = None
        previous_paths = None
        existing = _load_optional_lock(EVAL_DIR)
        if args.re_anchor:
            if existing is None:
                raise ValueError("re_anchor_previous_mismatch: eval/tier-a.lock.toml")
            previous, previous_paths = verify_re_anchor_previous(
                EVAL_DIR.parent, EVAL_DIR
            )
        else:
            existing_reasons = integrity_check(existing, EVAL_DIR)
            if existing_reasons:
                raise BoundaryViolation(existing_reasons)
            if existing is not None:
                raise ValueError("anchor_already_exists")
        finalize_policy(EVAL_DIR)
        root = tier_a_root()
        binary = Path(args.sut_bin).resolve()
        if args.re_anchor:
            snapshot, present, _sut_sha, _sut_digest = prepare_snapshot_only(
                EVAL_DIR.parent,
                EVAL_DIR,
                binary,
                {"cargo": {"features": "all"}},
                allow_existing_anchor=True,
                stage_for_re_anchor=True,
            )
            if not present:
                staged_snapshot = snapshot
        candidate = stage_candidate(
            EVAL_DIR.parent,
            EVAL_DIR,
            binary,
            {"cargo": {"features": "all"}},
            root,
            str(EVAL_DIR.parent),
            snapshot_source=staged_snapshot,
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
            EVAL_DIR, candidate=candidate, sut=sut, re_anchor=args.re_anchor,
        )
        published = publish_anchor_v1(
            candidate, run, EVAL_DIR, EVAL_DIR.parent / "docs/eval/tier-a",
            root / "sut", superseded=previous, previous_paths=previous_paths,
        )
        mark_history(EVAL_DIR.parent / "docs/eval/tier-a/baseline.md", run_args.date)
        if integrity_check(published, EVAL_DIR):
            raise BoundaryViolation(integrity_check(published, EVAL_DIR))
    except Exception as exc:
        return refuse(exc)
    finally:
        if staged_snapshot is not None:
            staged_snapshot.unlink(missing_ok=True)
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
        raise BoundaryViolation(["no_lock"])

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
    for key in OBSERVED_RUN_IDENTITIES:
        identities.pop(key, None)
    return (
        lock,
        checkout,
        {
            **identities,
            "corpus_sha": identity.sha,
            "corpus_dirty": identity.dirty,
            "untracked_sources": identity.untracked,
            "corpus_mode": "pinned",
        },
        identity,
    )


def _validate_lock_sample(lock: Lock | None, args) -> None:
    if lock is None or getattr(args, "sample", None) is None:
        return
    mode = "quick" if getattr(args, "quick", False) else "full"
    expected = lock.prism["sample"]["per_stratum"][mode]
    actual = args.sample
    if actual != expected:
        raise BoundaryViolation(
            [f"sample_override_mismatch: {mode}: {actual} != {expected}"]
        )


def finalize_metrics(run: dict, declared, defaults: dict, cfg: dict) -> dict:
    """Compute verdict-bearing metrics only after boundary admission."""
    return run_policy.finalize_deferred_metrics(
        run, declared.adjudications, defaults, cfg
    )


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
    paths.update(corpus_control_paths(root, tracked_only=False))
    return paths


def _sut_input_state(repo: Path, rows: list[tuple]) -> tuple[tuple[str, str], ...]:
    state = []
    for row in rows:
        path = row[0]
        disk_path = repo / path
        state.append(
            (path, raw_digest(disk_path) if disk_path.is_file() else "absent")
        )
    return tuple(sorted(state))


def run_quick_admitted(
    name: str,
    cfg: dict,
    defaults: dict,
    args,
    eval_dir: Path,
    *,
    canonical_name: str | None = None,
    candidate=None,
    sut=None,
    re_anchor: bool = False,
) -> tuple[dict, dict]:
    """Run quick observations, admit them, then compute verdict metrics."""
    from . import runner

    if candidate is None:
        lock, corpus_path, overlay, identity = preflight_quick(eval_dir, args.live)
        artifacts = (
            ArtifactPaths.from_lock(eval_dir.parent, lock)
            if lock is not None
            else None
        )
    else:
        existing = _load_optional_lock(eval_dir)
        if not re_anchor:
            reasons = integrity_check(existing, eval_dir)
            if reasons:
                raise BoundaryViolation(reasons)
            if existing is not None:
                raise BoundaryViolation(["anchor_already_exists"])
        if existing is None and re_anchor:
            raise BoundaryViolation(["re_anchor_previous_mismatch"])
        lock, overlay = candidate.lock, {}
        artifacts = candidate.artifacts
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
    _validate_lock_sample(lock, args)
    sut_state_before = None
    if lock is not None and artifacts is not None:
        try:
            sut_rows = authenticate_sut_inputs(lock, artifacts)
            sut_state_before = _sut_input_state(eval_dir.parent, sut_rows)
        except OSError as exc:
            location = exc.filename or "sut_input_state"
            raise BoundaryViolation(
                [f"sut_observation_unavailable: {location}"]
            ) from exc
    corpus_cfg = {**cfg, "path": str(corpus_path)}
    env = scrubbed_env(lock)
    if sut is None and getattr(args, "sut_archive", None):
        from .sut import PrismCli

        sut = PrismCli.from_archive(
            str(eval_dir.parent),
            args.sut_archive,
            lock.prism["sut"]["sha"],
            lock.prism["sut"]["digest"],
            env=env,
        )
    measurement_name = canonical_name or name
    with OpenAudit(eval_dir, artifacts) as audit:
        declared = load_declared_inputs(
            lock,
            eval_dir,
            corpus=measurement_name,
            cfg=corpus_cfg,
            artifacts=artifacts,
        )
        run = runner.run_corpus(
            name,
            corpus_cfg,
            defaults,
            args,
            canonical_name=measurement_name,
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

    if lock is not None and identity is not None:
        try:
            identity_after = resolve_corpus_identity(lock, corpus_path)
        except Exception as exc:
            raise BoundaryViolation(["corpus_changed_during_run"]) from exc
        if identity_after != identity:
            raise BoundaryViolation(["corpus_changed_during_run"])
        assert sut_state_before is not None
        try:
            sut_state_after = _sut_input_state(eval_dir.parent, sut_rows)
        except OSError as exc:
            raise BoundaryViolation(["sut_inputs_changed_during_run"]) from exc
        if sut_state_after != sut_state_before:
            raise BoundaryViolation(["sut_inputs_changed_during_run"])

    launch_reason = launch_refusal_reason(run.get("meta", {}).get("launch"))
    if launch_reason is not None:
        raise BoundaryViolation([launch_reason])

    sut_owner = run.pop("_sut_owner", sut)
    sut_bin = Path(
        run.pop("_sut_bin", args.sut_bin or eval_dir.parent / "target/release/prism")
    )
    try:
        run["meta"]["sut_digest"] = raw_digest(sut_bin)
    except OSError as exc:
        raise BoundaryViolation(
            [f"sut_observation_unavailable: executable_digest: {sut_bin}"]
        ) from exc
    try:
        if getattr(args, "sut_archive", None):
            if lock is None or artifacts is None:
                raise ValueError("sut_observation_unavailable: archive is not lock-bound")
            env_reads, dep_paths = observe_archive(
                lock, eval_dir.parent, env, artifacts
            )
        else:
            source_bin = Path(getattr(sut_owner, "source_bin", sut_bin))
            if getattr(sut_owner, "sut_digest", None) is not None:
                try:
                    source_digest = raw_digest(source_bin)
                except OSError as exc:
                    raise ValueError(
                        f"sut_observation_unavailable: executable_digest: {source_bin}"
                    ) from exc
                if source_digest != sut_owner.sut_digest:
                    raise ValueError(
                        "sut_observation_unavailable: executable_drift_after_snapshot"
                    )
            build_dir = (
                source_bin.parent
                if source_bin.parent != Path(".")
                else eval_dir.parent / "target/release"
            )
            env_reads, dep_paths = observe_cargo(
                build_dir, eval_dir.parent, env, source_bin.stem
            )
    except ValueError as exc:
        reason = str(exc)
        if not reason.startswith(
            ("sut_fingerprint_unavailable:", "sut_observation_unavailable:")
        ):
            reason = f"sut_observation_unavailable: {reason}"
        raise BoundaryViolation([reason]) from exc
    observed = Observation(env_reads, dep_paths, audit.opens)
    closure_paths = (
        {row[0] for row in getattr(lock, "_corpus_rows", [])}
        if lock is not None
        else _lockless_closure_paths(corpus_cfg)
    )
    admission = admit(
        lock,
        declared,
        observed,
        closure_paths,
        eval_dir,
        artifacts,
        archive_provenance=bool(getattr(args, "sut_archive", None)),
    )

    run["meta"]["observation"] = {
        "env_reads": [asdict(read) for read in observed.env_reads],
        "dep_paths": sorted(observed.dep_paths),
        "harness_opens": sorted(observed.harness_opens),
    }
    run["meta"].update(admission)
    finalize_metrics(run, declared, defaults, corpus_cfg)

    run["meta"]["env_values"] = dict(env)
    run["meta"]["data"] = {
        path: {"digest": digest} for path, digest in sorted(declared.digests.items())
    }
    run["meta"]["data"]["eval/fixtures"] = {
        "digest": declared.fixtures_digest
    }
    if lock is None:
        run_policy.apply_lockless_refusal(run)
    else:
        observed = {
            key: run["meta"].get(key) for key in OBSERVED_RUN_IDENTITIES
        }
        declared = lock_identities(lock)
        for key in OBSERVED_RUN_IDENTITIES:
            declared.pop(key, None)
        run["meta"].update(declared)
        run["meta"].update(observed)
        if identity is not None:
            run["meta"]["corpus_sha_full"] = identity.sha_full
        assert artifacts is not None
        run["meta"]["snapshot_digest"] = raw_digest(artifacts.snapshot())
        run["meta"]["admitted"] = True
        if not getattr(args, "live", False):
            schema_reasons = record_refusals(run)
            if schema_reasons:
                raise BoundaryViolation(
                    [f"run_record_schema_invalid: {reason}" for reason in schema_reasons]
                )
    return run, overlay


def dispatch_or_run(argv: list[str], parser: argparse.ArgumentParser) -> int:
    """Route command plumbing or execute one parsed measurement invocation."""
    command_rc = dispatch(argv)
    if command_rc is not None:
        return command_rc
    args = parser.parse_args(argv)
    if args.sample is not None and args.sample <= 0:
        parser.error("sample must be positive")
    timeout_keys = ("query_timeout_s", "startup_timeout_s", "oracle_budget_s")
    if any(
        getattr(args, key) is not None
        and (not math.isfinite(getattr(args, key)) or getattr(args, key) <= 0)
        for key in timeout_keys
    ):
        parser.error("timeouts must be positive")
    if args.date is None:
        args.date = datetime.date.today().isoformat()
    from .runner import run

    return run(args)
