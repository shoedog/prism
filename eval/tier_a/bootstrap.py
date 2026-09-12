"""Staged creation and atomic publication of the first Tier-A anchor."""
from __future__ import annotations

import copy
import datetime
import hashlib
import json
import os
import re
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path

from .boundary import DEFAULT_ENV_ALLOW, scrubbed_env
from .closure import canonical_path, corpus_manifest, manifest_digest, policy_digest
from .closure import policy_manifest, population_manifest, raw_digest
from .closure import sut_inputs_manifest, write_tsv
from .corpus import load_snapshot, universe
from .lock import IDENTITY_KEYS, Lock, integrity_check, lock_identities
from .lock import report_bindable, save_lock
from .report import render_markdown
from .sut import verified_binary_identity


@dataclass
class CandidateIdentity:
    lock: Lock
    staged: dict[Path, Path]


def _run(*command: str, env: dict[str, str] | None = None) -> str:
    return subprocess.run(command, check=True, capture_output=True, text=True,
                          env=env).stdout.strip()

def _git(repo: Path, *args: str) -> str: return _run("git", "-C", str(repo), *args)

def finalize_policy(eval_dir: Path) -> None:
    path = eval_dir / "corpora.toml"
    lines = path.read_text().splitlines(keepends=True)
    in_prism = False
    changed = False
    output = []
    for line in lines:
        stripped = line.strip()
        if stripped.startswith("["):
            in_prism = stripped == "[corpus.prism]"
        if in_prism and re.match(r"^\s*pinned_sha\s*=", line):
            match = re.search(r'"([0-9a-f]{12,40})"', line)
            suffix = (
                f"; bootstrap snapshot: prism-{match.group(1)[:12]}.json"
                if match
                else ""
            )
            output.append(
                "# Corpus identity is stored in eval/tier-a.lock.toml"
                f"{suffix}.\n"
            )
            changed = True
        else:
            output.append(line)
    if changed:
        path.write_text("".join(output))


def _snapshot_for(eval_dir: Path, sut_sha: str) -> Path:
    path = eval_dir / "snapshots" / f"prism-{sut_sha[:12]}.json"
    if not path.is_file():
        raise ValueError(f"snapshot_unavailable: {sut_sha}")
    return path

def _tool_file(repo: Path, names: tuple[str, ...]) -> str:
    paths = [repo / name for name in names if (repo / name).is_file()]
    return raw_digest(paths[0]) if len(paths) == 1 else "absent"

def _json_bytes(value: object) -> bytes:
    return (json.dumps(value, indent=2, sort_keys=True) + "\n").encode()


def stage_candidate(
    repo: Path,
    eval_dir: Path,
    binary: Path,
    init_options: dict,
    root: Path,
    origin_url: str,
) -> CandidateIdentity:
    repo, eval_dir, binary = repo.resolve(), eval_dir.resolve(), binary.resolve()
    policy_file = (eval_dir / "corpora.toml").relative_to(repo).as_posix()
    if _git(repo, "status", "--porcelain", "--", policy_file):
        raise ValueError("policy_uncommitted")

    env = scrubbed_env(None)
    _display_sha, sut_sha = verified_binary_identity(str(repo), str(binary), env)
    harness_sha = _git(repo, "rev-parse", "HEAD")
    policy = policy_manifest(eval_dir)
    cfg = policy["corpora"]["corpus"]["prism"]
    closure_paths = set(
        universe(str(repo), cfg["lang"], cfg.get("excludes", []), tracked_only=True)
    )
    closure_paths.update(
        name
        for name in ("Cargo.lock", "Cargo.toml", "build.rs", "rust-toolchain", "rust-toolchain.toml")
        if (repo / name).is_file()
    )
    closure_rows = corpus_manifest(repo, sorted(closure_paths))
    sut_rows = sut_inputs_manifest(repo, binary)
    snapshot = _snapshot_for(eval_dir, sut_sha)
    package_dirs = set()
    if cfg["lang"] == "python":
        from .cli import package_dirs as find_package_dirs

        package_dirs = find_package_dirs(str(repo))
    population = population_manifest(
        load_snapshot(snapshot), policy, package_dirs, cfg["lang"]
    )
    ids = {
        "closure": manifest_digest(closure_rows),
        "sut_inputs": manifest_digest(sut_rows),
        "policy": policy_digest(policy),
        "population": policy_digest(population),
    }
    staging = eval_dir / "closure" / f".staging-{os.getpid()}"
    staging.mkdir(parents=True, exist_ok=False)
    finals = [
        (eval_dir / "closure" / f"prism-corpus-{ids['closure'][7:19]}.tsv", closure_rows),
        (eval_dir / "closure" / f"sut-inputs-{ids['sut_inputs'][7:19]}.tsv", sut_rows),
        (eval_dir / "closure" / f"policy-{ids['policy'][7:19]}.json", policy),
        (eval_dir / "closure" / f"prism-population-{ids['population'][7:19]}.json", population),
    ]
    staged, manifest_bytes = {}, {}
    for final, value in finals:
        temporary = staging / final.name
        write_tsv(value, temporary) if final.suffix == ".tsv" else temporary.write_bytes(_json_bytes(value))
        staged[final], manifest_bytes[final] = temporary, temporary.read_bytes()

    defaults = policy["corpora"]["defaults"]
    sample = {
        "schema": 1,
        "seed": defaults["seed"],
        "per_stratum": {"quick": 3, "full": defaults["per_stratum"]},
    }
    sample["digest"] = {
        mode: policy_digest(
            {"schema": sample["schema"], "seed": sample["seed"], "per_stratum": count}
        )
        for mode, count in sample["per_stratum"].items()
    }
    fixture_rows = [
        (path.relative_to(repo).as_posix(), raw_digest(path))
        for path in sorted((eval_dir / "fixtures").rglob("*"))
        if path.is_file()
    ]
    sut_digest = raw_digest(binary)
    lock = Lock(prism={
        "closure": {"digest": ids["closure"], "manifest": canonical_path(repo, finals[0][0])},
        "sha_aliases": [sut_sha],
        "policy": {"digest": ids["policy"], "manifest": canonical_path(repo, finals[2][0])},
        "population": {"digest": ids["population"], "manifest": canonical_path(repo, finals[3][0])},
        "snapshot": {"path": canonical_path(repo, snapshot), "digest": raw_digest(snapshot)},
        "sample": sample,
        "oracle": {"version": _run("rust-analyzer", "--version"), "init": copy.deepcopy(init_options)},
        "sut": {"sha": sut_sha, "digest": sut_digest, "inputs": canonical_path(repo, finals[1][0]), "archive": str(root.resolve() / "sut" / sut_digest[7:] / "prism")},
        "runtime": {
            "python": sys.version.split()[0], "python_version_file": raw_digest(eval_dir / ".python-version"),
            "uv_lock": raw_digest(eval_dir / "uv.lock"), "rustc": _run("rustc", "--version"),
            "cargo": _run("cargo", "--version"), "rust_toolchain": _tool_file(repo, ("rust-toolchain.toml", "rust-toolchain")),
            "cargo_config": _tool_file(repo, (".cargo/config.toml", ".cargo/config")),
        },
        "data": {
            "adjudications": {"path": "eval/adjudications.jsonl", "digest": raw_digest(eval_dir / "adjudications.jsonl"), "records": len((eval_dir / "adjudications.jsonl").read_text().splitlines())},
            "fixtures": manifest_digest(fixture_rows), "pinned_probes": raw_digest(eval_dir / "tier_a/pinned.py"),
        },
        "env": {"allow": list(DEFAULT_ENV_ALLOW), "values": env, "forbid_read_outside_allow": True},
        "harness_sha": harness_sha,
    })
    lock._corpus_rows, lock._policy_data, lock._population_data = closure_rows, policy, population
    candidate = CandidateIdentity(lock, staged)
    candidate._repo, candidate._eval_dir, candidate._binary = repo, eval_dir, binary
    candidate._staging, candidate._manifest_bytes = staging, manifest_bytes
    candidate._sut_rows = sut_rows
    candidate._origin_url, candidate._root = origin_url, root
    return candidate


def validate_staged(cand: CandidateIdentity) -> list[str]:
    trial = copy.deepcopy(cand.lock)
    trial.prism.pop("baseline_report", None); trial.prism.pop("blessed", None)
    actual = {
        final: staged if staged.is_file() else final
        for final, staged in cand.staged.items()
    }
    for key in ("closure", "policy", "population"):
        final = cand._repo / trial.prism[key]["manifest"]
        trial.prism[key]["manifest"] = canonical_path(cand._repo, actual[final])
    reasons = [reason for reason in integrity_check(trial, cand._eval_dir) if reason != "baseline_heading_mismatch"]
    for final, path in actual.items():
        if not path.is_file() or path.read_bytes() != cand._manifest_bytes[final]:
            reasons.append(f"staged_manifest_moved: {final.name}")
    try:
        if corpus_manifest(cand._repo, [row[0] for row in cand.lock._corpus_rows]) != cand.lock._corpus_rows:
            reasons.append("closure_digest_moved")
        if sut_inputs_manifest(cand._repo, cand._binary) != cand._sut_rows:
            reasons.append("sut_inputs_moved")
        if raw_digest(cand._binary) != cand.lock.prism["sut"]["digest"]:
            reasons.append("sut_digest_moved")
        if _git(cand._repo, "rev-parse", "HEAD") != cand.lock.prism["harness_sha"]:
            reasons.append("harness_sha_moved")
        if scrubbed_env(None) != cand.lock.prism["env"]["values"]:
            reasons.append("env_values_moved")
    except (OSError, ValueError, subprocess.CalledProcessError) as exc:
        reasons.append(f"staged_candidate_invalid: {exc}")
    return list(dict.fromkeys(reasons))

def _atomic_bytes(path: Path, contents: bytes, mode: int | None = None) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.parent / f".{path.name}.{os.getpid()}.tmp"
    temporary.write_bytes(contents)
    if mode is not None:
        temporary.chmod(mode)
    os.replace(temporary, path)


def publish_anchor_v1(
    cand: CandidateIdentity,
    run: dict,
    eval_dir: Path,
    out_dir: Path,
    archive_dir: Path,
) -> Lock:
    meta = run.get("meta", {})
    if meta.get("admitted") is not True:
        raise ValueError("not publishable: not admitted")
    if meta.get("baseline_invalid") is not False:
        raise ValueError("not publishable: baseline_invalid")
    missing = report_bindable(meta)
    if missing:
        raise ValueError(f"not bindable: {','.join(missing)}")
    expected = lock_identities(cand.lock)
    mismatches = [key for key in IDENTITY_KEYS if meta[key] != expected[key]]
    if mismatches:
        raise ValueError(f"identity mismatch: {','.join(mismatches)}")
    invalid = validate_staged(cand)
    if invalid:
        raise ValueError(f"staged invalid: {','.join(invalid)}")
    archive = archive_dir.resolve() / expected["sut_digest"][7:] / "prism"
    if Path(cand.lock.prism["sut"]["archive"]) != archive:
        raise ValueError("archive path mismatch")
    binary_bytes = cand._binary.read_bytes()
    binary_mode = cand._binary.stat().st_mode & 0o777
    captured_digest = "sha256:" + hashlib.sha256(binary_bytes).hexdigest()
    if captured_digest != expected["sut_digest"]:
        raise ValueError("staged invalid: sut_digest_moved")

    report = copy.deepcopy(run)
    report_meta = report["meta"]
    date = report_meta.setdefault("date", datetime.date.today().isoformat())
    report_meta.setdefault("corpus", "prism")
    for key, value in {
        "corpus_sha": expected["corpus_sha_full"][:12],
        "prism_sha": expected["sut_sha_full"][:12],
        "oracle_error_rate": 0.0,
        "sut_error_rate": 0.0,
        "oracle_not_quiescent": False,
        "wall_s": {},
    }.items():
        report_meta.setdefault(key, value)
    report_base = out_dir / f"{date}-{report_meta['corpus']}"
    report_json, report_md = report_base.with_suffix(".json"), report_base.with_suffix(".md")
    report_bytes = json.dumps(report, indent=1, sort_keys=True, default=str, allow_nan=False).encode()
    cand.lock.prism["baseline_report"] = {
        "path": canonical_path(eval_dir.parent, report_json),
        "digest": "sha256:" + hashlib.sha256(report_bytes).hexdigest(),
    }
    cand.lock.prism["blessed"] = {"by": "bootstrap", "at": date, "sut_sha": expected["sut_sha_full"]}
    for final, staged in cand.staged.items():
        if staged.is_file():
            os.replace(staged, final)
        elif not final.is_file() or final.read_bytes() != cand._manifest_bytes[final]:
            raise ValueError(f"published manifest mismatch: {final.name}")
    _atomic_bytes(report_json, report_bytes)
    _atomic_bytes(report_md, render_markdown(report).encode())
    _atomic_bytes(archive, binary_bytes, binary_mode)
    lock_temp = eval_dir / f".tier-a.lock.toml.{os.getpid()}.tmp"
    save_lock(cand.lock, lock_temp)
    os.replace(lock_temp, eval_dir / "tier-a.lock.toml")
    cand._staging.rmdir()
    return cand.lock


def mark_history(baseline_md: Path, bootstrap_date: str) -> None:
    contents = baseline_md.read_text()
    if contents.startswith("# Tier-A Baseline — anchor v1 ("):
        return
    baseline_md.write_text(
        f"# Tier-A Baseline — anchor v1 ({bootstrap_date}) — prior sections "
        "superseded (pre-boundary, not comparable)\n\n" + contents
    )
