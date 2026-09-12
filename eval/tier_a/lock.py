"""Tier-A lockfile persistence and report identity binding."""
from __future__ import annotations

import tomllib
from dataclasses import dataclass
from pathlib import Path

import tomli_w


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
