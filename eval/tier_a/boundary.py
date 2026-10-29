"""Declared/observed input boundary for admitted Tier-A quick runs."""
from __future__ import annotations

import hashlib
import json
import os
import re
import subprocess
import sys
import tomllib
from copy import deepcopy
from dataclasses import dataclass
from pathlib import Path

from .artifacts import ArtifactPaths
from .adjudication import Adjudication, load_records
from .closure import (
    cargo_build_outputs,
    cargo_package_name,
    manifest_digest,
    raw_digest,
    read_tsv,
    sut_input_paths,
)
from .corpus import load_snapshot, snapshot_path
from .lock import Lock, _bound_sut_rows, load_lock
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
    def __init__(self, eval_dir: Path, artifacts: ArtifactPaths | None = None):
        self.eval_dir = eval_dir.resolve()
        self.repo = self.eval_dir.parent
        self.artifacts = artifacts
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
            self.opens.add(
                self.artifacts.declared_open(path)
                if self.artifacts is not None
                else path.relative_to(self.repo).as_posix()
            )
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


def observe_cargo(
    build_dir: Path,
    repo: Path,
    env: dict[str, str],
    binary_name: str | None = None,
) -> tuple[list[EnvRead], set[str]]:
    env_reads: dict[str, EnvRead] = {}
    package = cargo_package_name(repo)
    for output in cargo_build_outputs(repo):
        for line in output.read_text().splitlines():
            for prefix in (
                "cargo:rerun-if-env-changed=",
                "cargo::rerun-if-env-changed=",
            ):
                if line.startswith(prefix):
                    name = line.removeprefix(prefix)
                    env_reads[name] = EnvRead(name, name in env, env.get(name))
                    break
    dep_name = binary_name or package
    binary = build_dir / dep_name
    dep_paths = set(sut_input_paths(repo, binary, require_dep_info=False))
    return [env_reads[name] for name in sorted(env_reads)], dep_paths


def authenticate_sut_inputs(lock: Lock, artifacts: ArtifactPaths) -> list[tuple]:
    """Bind the complete SUT-input manifest to existing anchor commitments."""
    try:
        rows = read_tsv(artifacts.sut_inputs())
        digest = manifest_digest(rows)
        name = Path(lock.prism["sut"]["inputs"]).name
        match = re.fullmatch(r"sut-inputs-([0-9a-f]{12})\.tsv", name)

        provenance = None
        baseline = lock.prism.get("baseline_report")
        if isinstance(baseline, dict):
            report_path = artifacts.resolve(baseline["path"])
            if raw_digest(report_path) != baseline["digest"]:
                raise ValueError("baseline report digest mismatch")
            provenance = json.loads(report_path.read_text()).get("meta", {}).get(
                "sut_input_provenance"
            )
        kinds = (
            "tracked_source",
            "repository_metadata",
            "archived_generated",
        )
        provenance_bound = isinstance(provenance, dict) and any(
            kind in provenance for kind in kinds
        )
        if provenance_bound:
            if not all(
                isinstance(provenance.get(kind), list)
                and all(isinstance(path, str) for path in provenance[kind])
                for kind in kinds
            ):
                raise ValueError("baseline provenance invalid")
            expected_paths = {path for kind in kinds for path in provenance[kind]}
            if {row[0] for row in rows} != expected_paths:
                raise ValueError("baseline provenance path mismatch")

        content_addressed = match is not None and digest[7:19] == match.group(1)
        if not content_addressed:
            raise ValueError("content-addressed name mismatch")
    except (KeyError, IndexError, OSError, TypeError, ValueError) as exc:
        raise BoundaryViolation(["sut_input_manifest_mismatch"]) from exc

    lock._sut_rows = rows
    return rows


def observe_archive(
    lock: Lock,
    repo: Path,
    env: dict[str, str],
    artifacts: ArtifactPaths,
) -> tuple[list[EnvRead], set[str]]:
    """Observe a digest-bound archive through its bound input provenance."""
    sut_rows = authenticate_sut_inputs(lock, artifacts)

    package = cargo_package_name(repo)
    prefixes = {package, package.replace("-", "_")}
    build_outputs = []
    for path, expected, *_rest in sut_rows:
        parts = Path(path).parts
        if (
            expected != "absent"
            and len(parts) >= 5
            and parts[0] == "target"
            and "build" in parts
            and parts[-1] == "output"
            and any(parts[-2].startswith(f"{prefix}-") for prefix in prefixes)
        ):
            archived = artifacts.sut_archive_input(path)
            if archived.is_file():
                build_outputs.append(archived)

    env_reads: dict[str, EnvRead] = {}
    if build_outputs:
        for output in sorted(build_outputs):
            try:
                lines = output.read_text().splitlines()
            except OSError as exc:
                raise BoundaryViolation(
                    [f"sut_observation_unavailable: {output}"]
                ) from exc
            for line in lines:
                for prefix in (
                    "cargo:rerun-if-env-changed=",
                    "cargo::rerun-if-env-changed=",
                ):
                    if line.startswith(prefix):
                        name = line.removeprefix(prefix)
                        env_reads[name] = EnvRead(name, name in env, env.get(name))
                        break
    else:
        env_config = lock.prism.get("env", {})
        allowed = env_config.get("allow")
        if not isinstance(allowed, list) or not all(
            isinstance(name, str) for name in allowed
        ):
            raise ValueError(
                "sut_observation_unavailable: archive has no build outputs and "
                "lock has no recorded env allow-list"
            )
        env_reads = {
            name: EnvRead(name, name in env, env.get(name)) for name in allowed
        }

    return [env_reads[name] for name in sorted(env_reads)], {
        row[0] for row in sut_rows
    }


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
    artifacts: ArtifactPaths | None = None,
) -> DeclaredInputs:
    repo = eval_dir.parent
    adjudications_entry, snapshot_entry = _declared_paths(lock, eval_dir, corpus, cfg)
    artifacts = artifacts or (
        ArtifactPaths.from_lock(repo, lock) if lock is not None else None
    )
    adjudications_path = (
        artifacts.resolve(adjudications_entry)
        if artifacts is not None
        else repo / adjudications_entry
    )
    snapshot_disk_path = (
        artifacts.resolve(snapshot_entry)
        if artifacts is not None
        else repo / snapshot_entry
    )
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


def _current_data_digest(
    eval_dir: Path, path: str, artifacts: ArtifactPaths | None
) -> str:
    if path == "eval/fixtures":
        return manifest_digest(_fixture_rows(eval_dir))
    disk_path = (
        artifacts.resolve(path)
        if artifacts is not None
        else eval_dir.parent / path
    )
    return raw_digest(disk_path) if disk_path.is_file() else "absent"


def _known_declared_paths(
    stored: dict, eval_dir: Path
) -> tuple[list[str], ArtifactPaths | None]:
    lock_path = eval_dir / "tier-a.lock.toml"
    lock = load_lock(lock_path) if lock_path.is_file() else None
    corpus = stored.get("meta", {}).get("corpus", "prism")
    adjudications, snapshot = _declared_paths(lock, eval_dir, corpus)
    return (
        list(
            dict.fromkeys(
                [
                    adjudications,
                    snapshot,
                    "eval/tier_a/pinned.py",
                    *(path for path, _digest in _fixture_rows(eval_dir)),
                    "eval/fixtures",
                ]
            )
        ),
        ArtifactPaths.from_lock(eval_dir.parent, lock) if lock is not None else None,
    )


def readmit_stored(stored: dict, eval_dir: Path) -> dict:
    replay = deepcopy(stored)
    meta = replay.get("meta", {})
    if meta.get("admitted") is not True:
        return replay
    stored_digests = _stored_data_digests(meta.get("data", {}))
    mismatches = []
    declared_paths, artifacts = _known_declared_paths(replay, eval_dir)
    for path in declared_paths:
        if path not in stored_digests:
            mismatches.append(f"missing_declared_digest: {path}")
        elif _current_data_digest(eval_dir, path, artifacts) != stored_digests[path]:
            mismatches.append(f"derived_from_changed_inputs: {path}")
    for path, expected in stored_digests.items():
        if path in declared_paths:
            continue
        if _current_data_digest(eval_dir, path, artifacts) != expected:
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


def _sut_input_verification(
    lock: Lock,
    eval_dir: Path,
    artifacts: ArtifactPaths | None = None,
    *,
    archive_provenance: bool = False,
) -> tuple[list[str], set[str], dict[str, list[str]]]:
    repo = eval_dir.parent
    artifacts = artifacts or ArtifactPaths.from_lock(repo, lock)
    try:
        sut_rows = _bound_sut_rows(lock)
    except ValueError:
        sut_rows = read_tsv(artifacts.sut_inputs())
    reasons = []
    paths = set()
    provenance = {
        "tracked_source": [],
        "repository_metadata": [],
        "archived_generated": [],
    }
    staged_archive = getattr(lock, "_sut_archive_inputs", {})
    for path, expected, *_rest in sut_rows:
        paths.add(path)
        parts = path.split("/")
        if (
            not path
            or path.startswith("/")
            or "\\" in path
            or any(part in {"", ".", ".."} for part in parts)
        ):
            reasons.append(f"sut_input_path_noncanonical: {path}")
            continue
        if path.startswith(".git/"):
            provenance["repository_metadata"].append(path)
            continue
        if path.startswith("target/"):
            provenance["archived_generated"].append(path)
            archived = artifacts.sut_archive_input(path)
            if expected == "absent":
                if (
                    path in staged_archive
                    or archived.is_file()
                    or (repo / path).is_file()
                ):
                    reasons.append(f"sut_input_absent_present_transition: {path}")
                continue
            if archived.is_file():
                try:
                    actual = raw_digest(archived)
                except OSError as exc:
                    if archive_provenance:
                        raise BoundaryViolation(
                            [f"sut_observation_unavailable: {archived}"]
                        ) from exc
                    raise
            elif path in staged_archive:
                actual = "sha256:" + hashlib.sha256(staged_archive[path]).hexdigest()
            else:
                reasons.append(f"sut_input_provenance_missing: {path}")
                continue
        else:
            provenance["tracked_source"].append(path)
            result = subprocess.run(
                [
                    "git",
                    "-C",
                    str(repo),
                    "cat-file",
                    "blob",
                    f"{lock.prism['sut']['sha']}:{path}",
                ],
                capture_output=True,
            )
            if expected == "absent":
                if result.returncode == 0:
                    reasons.append(f"sut_input_absent_present_transition: {path}")
                continue
            if result.returncode:
                reasons.append(f"sut_input_provenance_missing: {path}")
                continue
            actual = "sha256:" + hashlib.sha256(result.stdout).hexdigest()
        if actual != expected:
            reasons.append(f"data_digest_moved: {path}")
    return reasons, paths, {
        kind: sorted(class_paths) for kind, class_paths in provenance.items()
    }


def _sut_input_reasons(
    lock: Lock, eval_dir: Path, artifacts: ArtifactPaths | None = None
) -> tuple[list[str], set[str]]:
    reasons, paths, _provenance = _sut_input_verification(
        lock, eval_dir, artifacts
    )
    return reasons, paths


def admit(
    lock: Lock | None,
    declared: DeclaredInputs,
    observed: Observation,
    closure_paths: set[str],
    eval_dir: Path,
    artifacts: ArtifactPaths | None = None,
    *,
    archive_provenance: bool = False,
) -> dict:
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
    sut_provenance = {
        "tracked_source": [],
        "repository_metadata": [],
        "archived_generated": [],
    }
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

        sut_reasons, sut_paths, sut_provenance = _sut_input_verification(
            lock,
            eval_dir,
            artifacts,
            archive_provenance=archive_provenance,
        )
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
    return {"sut_input_provenance": sut_provenance}
