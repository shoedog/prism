"""Tier-A lockfile persistence and report identity binding."""
from __future__ import annotations

import fnmatch
import json
import re
import tomllib
from dataclasses import dataclass
from pathlib import Path

import tomli_w

from .closure import (
    checkout_corpus_manifest,
    manifest_digest,
    policy_digest,
    policy_manifest,
    population_manifest,
    raw_digest,
    read_tsv,
)
from .corpus import EXTENSIONS, load_snapshot


IDENTITY_KEYS = (
    "corpus_sha_full",
    "sha_aliases",
    "closure_digest",
    "sut_sha_full",
    "sut_digest",
    "oracle",
    "oracle_init",
    "policy_digest",
    "population_digest",
    "snapshot_digest",
    "sample_digest",
    "runtime",
    "data",
    "env_allow",
    "env_values",
    "harness_sha",
    "seed",
)
REQUIRED_REPORT_IDENTITIES = IDENTITY_KEYS


@dataclass
class Lock:
    prism: dict


@dataclass
class CorpusIdentity:
    sha_full: str
    sha: str
    aliases: list[str]
    dirty: bool
    untracked: list[str]
    universe: list[str]


def corpus_check(
    lock: Lock,
    checkout: Path,
    oracle_version: str,
    toolchain: dict,
) -> list[str]:
    reasons = []
    prism = lock.prism
    expected_oracle = prism["oracle"]["version"]
    if oracle_version != expected_oracle:
        reasons.append(
            f"oracle_version_mismatch: {oracle_version} != {expected_oracle}"
        )
    runtime = prism["runtime"]
    for name in ("rustc", "cargo", "python"):
        if toolchain.get(name) != runtime.get(name):
            reasons.append(
                f"toolchain_mismatch: {name}: {toolchain.get(name)} != {runtime.get(name)}"
            )
    try:
        population = _bound_population(lock)
        if population["lang"] == "python":
            actual_package_dirs = {
                path.parent.relative_to(checkout).as_posix()
                for path in checkout.rglob("__init__.py")
                if path.parent != checkout
            }
            if actual_package_dirs != set(population.get("package_dirs", [])):
                reasons.append("package_dirs_mismatch")
    except (KeyError, ValueError):
        reasons.append("package_dirs_mismatch")
    if not _checkout_closure_matches(lock, checkout):
        reasons.append("checkout_closure_mismatch")
    return reasons


def integrity_check(lock: Lock | None, eval_dir: Path) -> list[str]:
    closure_dir = eval_dir / "closure"
    published = (
        [
            path
            for path in closure_dir.iterdir()
            if path.name != ".gitkeep" and not path.name.startswith(".staging-")
        ]
        if closure_dir.is_dir()
        else []
    )
    if lock is None:
        return ["half_published_anchor"] if published else []

    reasons = []
    prism = lock.prism
    repo = eval_dir.parent
    try:
        _bind_manifests(lock, repo)
    except Exception as exc:
        reasons.append(f"manifest_invalid: {exc}")
        return reasons

    closure_rows = _bound_rows(lock)
    if manifest_digest(closure_rows) != prism["closure"]["digest"]:
        reasons.append("closure_digest_moved")

    try:
        stored_policy = _bound_policy(lock)
        current_policy = policy_manifest(eval_dir)
        if (
            policy_digest(stored_policy) != prism["policy"]["digest"]
            or policy_digest(current_policy) != prism["policy"]["digest"]
        ):
            reasons.append("policy_digest_moved")
    except Exception as exc:
        reasons.append(f"policy_manifest_invalid: {exc}")
        current_policy = None

    snapshot = _repo_path(repo, prism["snapshot"]["path"])
    if not snapshot.is_file() or raw_digest(snapshot) != prism["snapshot"]["digest"]:
        reasons.append("snapshot_digest_moved")

    population_path = _repo_path(repo, prism["population"]["manifest"])
    try:
        stored_population = json.loads(population_path.read_text())
        population_matches = (
            policy_digest(stored_population) == prism["population"]["digest"]
        )
        if current_policy is not None and snapshot.is_file():
            rebuilt = population_manifest(
                load_snapshot(snapshot),
                current_policy,
                set(stored_population.get("package_dirs", [])),
                stored_population["lang"],
            )
            population_matches &= (
                policy_digest(rebuilt) == prism["population"]["digest"]
            )
        if not population_matches:
            reasons.append("population_digest_moved")
    except Exception as exc:
        reasons.append(f"population_manifest_invalid: {exc}")

    sample = prism["sample"]
    sample_digests = sample.get("digest", {})
    for mode, count in sample.get("per_stratum", {}).items():
        expected = policy_digest(
            {
                "schema": sample["schema"],
                "seed": sample["seed"],
                "per_stratum": count,
            }
        )
        if isinstance(sample_digests, dict) and sample_digests.get(mode) != expected:
            reasons.append(f"sample_digest_moved: {mode}")

    adjudications = prism.get("data", {}).get("adjudications")
    if isinstance(adjudications, dict):
        path = _repo_path(repo, adjudications["path"])
        if not path.is_file() or raw_digest(path) != adjudications["digest"]:
            reasons.append(f"data_digest_moved: {adjudications['path']}")
        elif len(path.read_text().splitlines()) != adjudications.get("records"):
            reasons.append(f"data_records_moved: {adjudications['path']}")
    data = prism.get("data", {})
    fixtures = [path for path in (eval_dir / "fixtures").rglob("*") if path.is_file()]
    fixture_rows = [
        (_repo_path(repo, path.relative_to(repo).as_posix()).relative_to(repo).as_posix(), raw_digest(path))
        for path in fixtures
    ]
    if data.get("fixtures") is not None and manifest_digest(fixture_rows) != data["fixtures"]:
        reasons.append("data_digest_moved: eval/fixtures")
    pinned_path = eval_dir / "tier_a/pinned.py"
    if data.get("pinned_probes") is not None and (
        not pinned_path.is_file() or raw_digest(pinned_path) != data["pinned_probes"]
    ):
        reasons.append("data_digest_moved: eval/tier_a/pinned.py")

    runtime = prism.get("runtime", {})
    for key, name in (
        ("python_version_file", ".python-version"),
        ("uv_lock", "uv.lock"),
    ):
        expected = runtime.get(key)
        path = eval_dir / name
        if expected is not None and (
            not path.is_file() or raw_digest(path) != expected
        ):
            reasons.append(f"runtime_digest_moved: eval/{name}")
    for key, candidates in (
        ("rust_toolchain", ("rust-toolchain.toml", "rust-toolchain")),
        ("cargo_config", (".cargo/config.toml", ".cargo/config")),
    ):
        expected = runtime.get(key)
        paths = [repo / name for name in candidates if (repo / name).is_file()]
        actual = raw_digest(paths[0]) if len(paths) == 1 else "absent"
        if expected is not None and actual != expected:
            reasons.append(f"runtime_digest_moved: {key}")

    aliases = prism.get("sha_aliases", [])
    environment = prism.get("env", {})
    if (
        not aliases
        or any(not re.fullmatch(r"[0-9a-f]{40}", alias) for alias in aliases)
        or not re.fullmatch(r"[0-9a-f]{40}", prism.get("harness_sha", ""))
        or not set(environment.get("values", {})) <= set(environment.get("allow", []))
    ):
        reasons.append("lock_identity_invalid")

    baseline = prism.get("baseline_report")
    if isinstance(baseline, dict):
        path = _repo_path(repo, baseline["path"])
        if not path.is_file() or raw_digest(path) != baseline["digest"]:
            reasons.append("baseline_report_digest_moved")
        else:
            try:
                meta = json.loads(path.read_text())["meta"]
                missing = report_bindable(meta)
                if missing:
                    reasons.append(
                        f"baseline_report_missing_identities: {','.join(missing)}"
                    )
                else:
                    identities = lock_identities(lock)
                    if any(meta[key] != value for key, value in identities.items()):
                        reasons.append("baseline_report_identity_mismatch")
                    corpus_sha = meta.get("corpus_sha_full", meta.get("corpus_sha"))
                    if corpus_sha not in aliases and not any(
                        alias.startswith(str(corpus_sha)) for alias in aliases
                    ):
                        reasons.append("baseline_report_corpus_alias_mismatch")
            except Exception as exc:
                reasons.append(f"baseline_report_invalid: {exc}")
    baseline_md = repo / "docs/eval/tier-a/baseline.md"
    if baseline_md.is_file():
        first_heading = next(
            (line for line in baseline_md.read_text().splitlines() if line.startswith("#")),
            "",
        )
        if aliases[0][:12] not in first_heading and "superseded" not in first_heading.lower():
            reasons.append("baseline_heading_mismatch")
    return reasons


def resolve_corpus_identity(lock: Lock, checkout: Path) -> CorpusIdentity:
    aliases = lock.prism["sha_aliases"]
    sha_full = aliases[0]
    if not re.fullmatch(r"[0-9a-f]{40}", sha_full):
        from .materialize import CorpusPinViolation

        raise CorpusPinViolation("canonical corpus SHA is not 40 lowercase hex")
    matches, untracked = _checkout_closure_state(lock, checkout)
    policy = _bound_policy(lock)
    cfg = policy["corpora"]["corpus"]["prism"]
    extensions = EXTENSIONS[cfg["lang"]]
    excludes = cfg.get("excludes", [])
    files = [
        row[0]
        for row in _bound_rows(lock)
        if any(row[0].endswith(extension) for extension in extensions)
        and not any(fnmatch.fnmatch(row[0], pattern) for pattern in excludes)
    ]
    return CorpusIdentity(
        sha_full,
        sha_full[:12],
        list(aliases),
        not matches,
        untracked,
        sorted(set(files)),
    )


def _repo_path(repo: Path, stored: str) -> Path:
    path = Path(stored)
    if path.is_absolute() or ".." in path.parts:
        raise ValueError(f"non-canonical lock path: {stored}")
    return repo / path


def _bind_manifests(lock: Lock, repo: Path) -> None:
    closure_path = _repo_path(repo, lock.prism["closure"]["manifest"])
    policy_path = _repo_path(repo, lock.prism["policy"]["manifest"])
    population_path = _repo_path(repo, lock.prism["population"]["manifest"])
    lock._corpus_rows = read_tsv(closure_path)
    lock._policy_data = json.loads(policy_path.read_text())
    lock._population_data = json.loads(population_path.read_text())


def _bound_rows(lock: Lock) -> list[tuple]:
    rows = getattr(lock, "_corpus_rows", None)
    if rows is None:
        raise ValueError("corpus manifest is not bound")
    return rows


def _bound_policy(lock: Lock) -> dict:
    policy = getattr(lock, "_policy_data", None)
    if policy is None:
        raise ValueError("policy manifest is not bound")
    return policy


def _bound_population(lock: Lock) -> dict:
    population = getattr(lock, "_population_data", None)
    if population is None:
        raise ValueError("population manifest is not bound")
    return population


def _checkout_closure_matches(lock: Lock, checkout: Path) -> bool:
    return _checkout_closure_state(lock, checkout)[0]


def _checkout_closure_state(lock: Lock, checkout: Path) -> tuple[bool, list[str]]:
    try:
        expected = {}
        rows = _bound_rows(lock)
        if manifest_digest(rows) != lock.prism["closure"]["digest"]:
            return False, []
        for row in rows:
            relative = Path(row[0])
            if relative.is_absolute() or ".." in relative.parts:
                return False, []
            if row[0] in expected:
                return False, []
            expected[row[0]] = row[1]
        recomputed = dict(checkout_corpus_manifest(checkout, set(expected)))
        untracked = sorted(set(recomputed) - set(expected))
        matches = set(recomputed) == set(expected) and all(
            recomputed[path] == digest for path, digest in expected.items()
        )
        return matches, untracked
    except (OSError, ValueError):
        return False, []


def load_lock(path: Path) -> Lock:
    return Lock(prism=tomllib.loads(path.read_text())["prism"])


def save_lock(lock: Lock, path: Path) -> None:
    path.write_text(tomli_w.dumps({"prism": lock.prism}))


def report_bindable(meta: dict) -> list[str]:
    return [key for key in IDENTITY_KEYS if key not in meta]


def lock_identities(lock: Lock) -> dict:
    prism = lock.prism
    aliases = prism["sha_aliases"]
    sample = prism["sample"]
    environment = prism["env"]
    identities = {
        "corpus_sha_full": aliases[0],
        "sha_aliases": aliases,
        "closure_digest": prism["closure"]["digest"],
        "sut_sha_full": prism["sut"]["sha"],
        "sut_digest": prism["sut"]["digest"],
        "oracle": prism["oracle"]["version"],
        "oracle_init": prism["oracle"]["init"],
        "policy_digest": prism["policy"]["digest"],
        "population_digest": prism["population"]["digest"],
        "snapshot_digest": prism["snapshot"]["digest"],
        "sample_digest": sample["digest"],
        "runtime": prism["runtime"],
        "data": prism["data"],
        "env_allow": environment["allow"],
        "env_values": environment.get("values", {}),
    }
    if "harness_sha" in prism:
        identities["harness_sha"] = prism["harness_sha"]
    identities["seed"] = sample["seed"]
    return identities


def resolve_prefix(prefix: str, candidates: list[str]) -> str:
    matches = [candidate for candidate in candidates if candidate.startswith(prefix)]
    if not matches:
        raise ValueError("unknown prefix")
    if len(matches) != 1:
        raise ValueError("ambiguous prefix")
    return matches[0]
