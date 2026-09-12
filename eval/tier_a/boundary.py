"""Declared/observed input boundary for admitted Tier-A quick runs."""
from __future__ import annotations

import os
import shlex
import sys
import tomllib
from copy import deepcopy
from dataclasses import dataclass
from pathlib import Path

from .adjudication import Adjudication, load_records
from .closure import manifest_digest, raw_digest, read_tsv
from .corpus import load_snapshot, snapshot_path
from .lock import Lock, load_lock
from .model import FunctionDef


class BoundaryViolation(Exception):
    def __init__(self, reasons: list[str]):
        self.reasons = reasons
        super().__init__(", ".join(reasons))


@dataclass
class EnvRead:
    name: str
    present: bool
    value: str | None


@dataclass
class Observation:
    env_reads: list[EnvRead]
    dep_paths: set[str]
    harness_opens: set[str]


@dataclass
class DeclaredInputs:
    adjudications: list[Adjudication]
    snapshot: list[FunctionDef]
    fixtures_digest: str
    pinned_digest: str
    digests: dict[str, str]


DEFAULT_ENV_ALLOW = (
    "PATH",
    "HOME",
    "CARGO_HOME",
    "RUSTUP_HOME",
    "TMPDIR",
    "LANG",
    "TIER_A_ROOT",
)
_ACTIVE_OPEN_AUDITS: list[OpenAudit] = []
_AUDIT_DISPATCHER_INSTALLED = False


def _audit_dispatch(event: str, args: tuple) -> None:
    for collector in tuple(_ACTIVE_OPEN_AUDITS):
        collector._record(event, args)


def _install_audit_dispatcher() -> None:
    global _AUDIT_DISPATCHER_INSTALLED
    if not _AUDIT_DISPATCHER_INSTALLED:
        sys.addaudithook(_audit_dispatch)
        _AUDIT_DISPATCHER_INSTALLED = True


class OpenAudit:
    def __init__(self, eval_dir: Path):
        self.eval_dir = eval_dir.resolve()
        self.repo = self.eval_dir.parent
        self.opens: set[str] = set()

    def _record(self, event: str, args: tuple) -> None:
        if event != "open" or not args:
            return
        raw = args[0]
        if not isinstance(raw, (str, bytes, os.PathLike)):
            return
        try:
            path = Path(os.fsdecode(raw)).resolve()
            path.relative_to(self.eval_dir)
            self.opens.add(path.relative_to(self.repo).as_posix())
        except (OSError, ValueError):
            return

    def __enter__(self) -> OpenAudit:
        _install_audit_dispatcher()
        _ACTIVE_OPEN_AUDITS.append(self)
        return self

    def __exit__(self, *_args) -> None:
        _ACTIVE_OPEN_AUDITS.remove(self)


def scrubbed_env(lock: Lock | None) -> dict[str, str]:
    allowed = (
        lock.prism.get("env", {}).get("allow", [])
        if lock is not None
        else DEFAULT_ENV_ALLOW
    )
    return {name: os.environ[name] for name in allowed if name in os.environ}


def _relative_to_repo(path: Path, repo: Path) -> str | None:
    try:
        return path.resolve().relative_to(repo.resolve()).as_posix()
    except ValueError:
        return None


def _dependency_paths(dep_info: Path) -> list[str]:
    contents = dep_info.read_text().replace("\\\n", "")
    _target, separator, dependencies = contents.partition(":")
    if not separator:
        return []
    return shlex.split(dependencies, comments=False, posix=True)


def _package_build_outputs(build_dir: Path, repo: Path) -> list[Path]:
    try:
        package = tomllib.loads((repo / "Cargo.toml").read_text())["package"]["name"]
    except (OSError, KeyError, tomllib.TOMLDecodeError):
        return []
    prefixes = {package, package.replace("-", "_")}
    return [
        output
        for output in sorted(build_dir.rglob("output"))
        if output.parent.parent.name == "build"
        and any(output.parent.name.startswith(f"{prefix}-") for prefix in prefixes)
    ]


def _add_repo_input(paths: set[str], raw: str, repo: Path) -> None:
    candidate = Path(raw)
    disk_path = candidate if candidate.is_absolute() else repo / candidate
    relative = _relative_to_repo(disk_path, repo)
    if relative is None:
        return
    if disk_path.is_dir():
        paths.update(
            child.relative_to(repo.resolve()).as_posix()
            for child in sorted(disk_path.resolve().rglob("*"))
            if child.is_file()
        )
    else:
        paths.add(relative)


def observe_cargo(
    build_dir: Path, repo: Path, env: dict[str, str]
) -> tuple[list[EnvRead], set[str]]:
    env_reads: dict[str, EnvRead] = {}
    dep_paths: set[str] = set()
    for output in _package_build_outputs(build_dir, repo):
        for line in output.read_text().splitlines():
            for prefix in (
                "cargo:rerun-if-env-changed=",
                "cargo::rerun-if-env-changed=",
            ):
                if line.startswith(prefix):
                    name = line.removeprefix(prefix)
                    env_reads[name] = EnvRead(name, name in env, env.get(name))
                    break
            for prefix in ("cargo:rerun-if-changed=", "cargo::rerun-if-changed="):
                if line.startswith(prefix):
                    _add_repo_input(dep_paths, line.removeprefix(prefix), repo)
                    break
    for dep_info in sorted(build_dir.rglob("*.d")):
        for raw in _dependency_paths(dep_info):
            _add_repo_input(dep_paths, raw, repo)
    return [env_reads[name] for name in sorted(env_reads)], dep_paths


def _fixture_rows(eval_dir: Path) -> list[tuple[str, str]]:
    repo = eval_dir.parent
    return [
        (path.relative_to(repo).as_posix(), raw_digest(path))
        for path in sorted((eval_dir / "fixtures").rglob("*"))
        if path.is_file()
    ]


def _unlocked_snapshot_path(
    eval_dir: Path, corpus: str, cfg: dict | None
) -> Path:
    if cfg is None:
        config = tomllib.loads((eval_dir / "corpora.toml").read_text())
        cfg = config["corpus"][corpus]
    return snapshot_path(
        eval_dir / "snapshots", corpus, str(cfg["pinned_sha"])[:12]
    )


def _declared_paths(
    lock: Lock | None,
    eval_dir: Path,
    corpus: str = "prism",
    cfg: dict | None = None,
) -> tuple[str, str]:
    if lock is not None:
        return (
            lock.prism["data"]["adjudications"]["path"],
            lock.prism["snapshot"]["path"],
        )
    snapshot = _unlocked_snapshot_path(eval_dir, corpus, cfg)
    return "eval/adjudications.jsonl", snapshot.relative_to(eval_dir.parent).as_posix()


def load_declared_inputs(
    lock: Lock | None,
    eval_dir: Path,
    *,
    corpus: str = "prism",
    cfg: dict | None = None,
) -> DeclaredInputs:
    repo = eval_dir.parent
    adjudications_entry, snapshot_entry = _declared_paths(lock, eval_dir, corpus, cfg)
    adjudications_path = repo / adjudications_entry
    snapshot_disk_path = repo / snapshot_entry
    pinned_path = eval_dir / "tier_a/pinned.py"
    fixture_rows = _fixture_rows(eval_dir)
    digests = {
        adjudications_entry: raw_digest(adjudications_path),
        snapshot_entry: raw_digest(snapshot_disk_path),
        "eval/tier_a/pinned.py": raw_digest(pinned_path),
        **dict(fixture_rows),
    }
    return DeclaredInputs(
        adjudications=load_records(adjudications_path),
        snapshot=load_snapshot(snapshot_disk_path),
        fixtures_digest=manifest_digest(fixture_rows),
        pinned_digest=digests["eval/tier_a/pinned.py"],
        digests=digests,
    )


def _stored_data_digests(data: dict) -> dict[str, str]:
    inputs = {}
    for key, value in data.items():
        if key.startswith("eval/") and isinstance(value, dict) and "digest" in value:
            inputs[key] = value["digest"]
        elif key == "adjudications" and isinstance(value, dict):
            inputs[value["path"]] = value["digest"]
        elif key == "fixtures" and isinstance(value, str):
            inputs["eval/fixtures"] = value
        elif key == "pinned_probes" and isinstance(value, str):
            inputs["eval/tier_a/pinned.py"] = value
    return inputs


def _current_data_digest(eval_dir: Path, path: str) -> str:
    if path == "eval/fixtures":
        return manifest_digest(_fixture_rows(eval_dir))
    disk_path = eval_dir.parent / path
    return raw_digest(disk_path) if disk_path.is_file() else "absent"


def _known_declared_paths(stored: dict, eval_dir: Path) -> list[str]:
    lock_path = eval_dir / "tier-a.lock.toml"
    lock = load_lock(lock_path) if lock_path.is_file() else None
    corpus = stored.get("meta", {}).get("corpus", "prism")
    adjudications, snapshot = _declared_paths(lock, eval_dir, corpus)
    return list(
        dict.fromkeys(
            [
                adjudications,
                snapshot,
                "eval/tier_a/pinned.py",
                *(path for path, _digest in _fixture_rows(eval_dir)),
                "eval/fixtures",
            ]
        )
    )


def readmit_stored(stored: dict, eval_dir: Path) -> dict:
    replay = deepcopy(stored)
    meta = replay.get("meta", {})
    if meta.get("admitted") is not True:
        return replay
    stored_digests = _stored_data_digests(meta.get("data", {}))
    mismatches = []
    declared_paths = _known_declared_paths(replay, eval_dir)
    for path in declared_paths:
        if path not in stored_digests:
            mismatches.append(f"missing_declared_digest: {path}")
        elif _current_data_digest(eval_dir, path) != stored_digests[path]:
            mismatches.append(f"derived_from_changed_inputs: {path}")
    for path, expected in stored_digests.items():
        if path in declared_paths:
            continue
        if _current_data_digest(eval_dir, path) != expected:
            mismatches.append(f"derived_from_changed_inputs: {path}")
    if mismatches:
        meta = replay.setdefault("meta", {})
        meta["derived"] = True
        meta["admitted"] = False
        meta["invalid_reasons"] = list(
            dict.fromkeys(meta.get("invalid_reasons", []) + mismatches)
        )
    return replay


def _allowed_harness_open(path: str, declared: set[str]) -> bool:
    return (
        path in declared
        or path in {"eval/tier-a.lock.toml", "eval/corpora.toml"}
        or path.startswith("eval/closure/")
        or path.startswith("eval/runs/")
    )


def _sut_input_reasons(lock: Lock, eval_dir: Path) -> tuple[list[str], set[str]]:
    repo = eval_dir.parent
    manifest_path = repo / lock.prism["sut"]["inputs"]
    reasons = []
    paths = set()
    for path, expected, *_rest in read_tsv(manifest_path):
        paths.add(path)
        disk_path = repo / path
        actual = (
            raw_digest(disk_path)
            if disk_path.is_file()
            else "present" if disk_path.exists() else "absent"
        )
        if (expected == "absent") != (actual == "absent"):
            reasons.append(f"sut_input_absent_present_transition: {path}")
        elif actual != expected:
            reasons.append(f"data_digest_moved: {path}")
    return reasons, paths


def admit(
    lock: Lock | None,
    declared: DeclaredInputs,
    observed: Observation,
    closure_paths: set[str],
    eval_dir: Path,
) -> None:
    reasons = [
        f"env_read_outside_allow: {read.name}"
        for read in observed.env_reads
        if read.present
        and read.name
        not in set(
            lock.prism["env"]["allow"] if lock is not None else DEFAULT_ENV_ALLOW
        )
    ]

    sut_paths = set()
    if lock is not None:
        data = lock.prism["data"]
        expected_digests = {
            data["adjudications"]["path"]: data["adjudications"]["digest"],
            lock.prism["snapshot"]["path"]: lock.prism["snapshot"]["digest"],
            "eval/tier_a/pinned.py": data["pinned_probes"],
        }
        reasons.extend(
            f"data_digest_moved: {path}"
            for path, expected in expected_digests.items()
            if declared.digests.get(path) != expected
        )
        if declared.fixtures_digest != data["fixtures"]:
            reasons.append("data_digest_moved: eval/fixtures")

        sut_reasons, sut_paths = _sut_input_reasons(lock, eval_dir)
        reasons.extend(sut_reasons)

    declared_paths = set(declared.digests)
    reasons.extend(
        f"undeclared_read: {path}"
        for path in sorted(observed.harness_opens)
        if not _allowed_harness_open(path, declared_paths)
    )
    allowed_deps = sut_paths | closure_paths
    reasons.extend(
        f"undeclared_read: {path}"
        for path in sorted(observed.dep_paths - allowed_deps)
    )
    if reasons:
        raise BoundaryViolation(reasons)
