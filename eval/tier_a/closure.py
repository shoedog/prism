"""Declared-input closure manifests for Tier-A anchors."""
from __future__ import annotations

import hashlib
import json
import shlex
import subprocess
import tomllib
import unicodedata
from collections import Counter
from pathlib import Path, PurePosixPath

from .corpus import EXTENSIONS
from .strata import STRATA, classify


__all__ = [
    "canonical_path",
    "raw_digest",
    "corpus_manifest",
    "manifest_digest",
    "write_tsv",
    "read_tsv",
    "sut_inputs_manifest",
    "policy_manifest",
    "policy_digest",
    "population_manifest",
]


_POLICY_MODULES = (
    "corpus.py",
    "strata.py",
    "accounting.py",
    "spotcheck.py",
    "pinned.py",
    "cli.py",
)


def canonical_path(root: Path, p: Path) -> str:
    normalized_root = Path(unicodedata.normalize("NFC", root.resolve().as_posix()))
    normalized_path = Path(unicodedata.normalize("NFC", p.resolve().as_posix()))
    return normalized_path.relative_to(normalized_root).as_posix()


def raw_digest(p: Path) -> str:
    return "sha256:" + hashlib.sha256(p.read_bytes()).hexdigest()


def corpus_manifest(
    root: Path, files: list[str]
) -> list[tuple[str, str, str]]:
    rows = []
    for declared in files:
        disk_path = root / declared
        path = canonical_path(root, disk_path)
        result = subprocess.run(
            ["git", "ls-files", "-s", "-z", "--", declared],
            cwd=root,
            check=True,
            capture_output=True,
        )
        entries = [entry for entry in result.stdout.split(b"\0") if entry]
        if len(entries) != 1:
            raise ValueError(f"declared corpus path is not a tracked file: {path}")
        metadata, _separator, _git_path = entries[0].partition(b"\t")
        fields = metadata.split()
        if len(fields) != 3 or fields[2] != b"0":
            raise ValueError(f"unexpected git index entry for {path}")
        rows.append((path, raw_digest(disk_path), fields[1].decode("ascii")))
    return sorted(rows)


def manifest_digest(rows) -> str:
    lines = [f"{row[0]}\t{row[1]}" for row in rows]
    payload = "".join(f"{line}\n" for line in sorted(lines)).encode()
    return "sha256:" + hashlib.sha256(payload).hexdigest()


def write_tsv(rows, path: Path) -> None:
    serialized = []
    for row in sorted(tuple(str(field) for field in row) for row in rows):
        if any("\t" in field or "\n" in field or "\r" in field for field in row):
            raise ValueError("TSV fields cannot contain tabs or newlines")
        serialized.append("\t".join(row))
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes("".join(f"{line}\n" for line in serialized).encode())


def read_tsv(path: Path) -> list[tuple]:
    return [tuple(line.split("\t")) for line in path.read_text().splitlines()]


def _makefile_dependencies(path: Path) -> list[str]:
    contents = path.read_text().replace("\\\n", "")
    _target, separator, dependencies = contents.partition(":")
    if not separator:
        raise ValueError(f"invalid dep-info: {path}")
    return shlex.split(dependencies, comments=False, posix=True)


def _input_files(repo: Path, value: str) -> list[Path]:
    candidate = Path(value)
    path = candidate if candidate.is_absolute() else repo / candidate
    if path.is_dir():
        return sorted(item for item in path.rglob("*") if item.is_file())
    return [path]


def sut_inputs_manifest(repo: Path, binary: Path) -> list[tuple[str, str]]:
    declared = _makefile_dependencies(binary.with_suffix(".d"))
    for output in sorted((repo / "target/release/build").glob("prism-*/output")):
        for line in output.read_text().splitlines():
            for prefix in ("cargo:rerun-if-changed=", "cargo::rerun-if-changed="):
                if line.startswith(prefix):
                    declared.append(line.removeprefix(prefix))
                    break
    declared.extend(
        name for name in ("Cargo.toml", "Cargo.lock") if (repo / name).is_file()
    )

    inputs = {}
    for declared_path in declared:
        for disk_path in _input_files(repo, declared_path):
            path = canonical_path(repo, disk_path)
            inputs[path] = raw_digest(disk_path) if disk_path.is_file() else "absent"
    return sorted(inputs.items())


def policy_manifest(eval_dir: Path) -> dict:
    config = tomllib.loads((eval_dir / "corpora.toml").read_text())
    defaults = config["defaults"]
    return {
        "corpora": config,
        "modules": {
            f"tier_a/{name}": raw_digest(eval_dir / "tier_a" / name)
            for name in _POLICY_MODULES
        },
        "extensions": EXTENSIONS,
        "strata": list(STRATA),
        "sampling": {
            "sampling_schema": 1,
            "quick_per_stratum": 3,
            "full_per_stratum": defaults["per_stratum"],
        },
    }


def policy_digest(policy: dict) -> str:
    payload = json.dumps(policy, sort_keys=True).encode()
    return "sha256:" + hashlib.sha256(payload).hexdigest()


def population_manifest(snapshot, policy, package_dirs, lang) -> dict:
    definitions_per_name = Counter(fd.name for fd in snapshot if fd.name is not None)
    symbols = {}
    for fd in sorted(
        snapshot,
        key=lambda item: (
            item.location.file,
            item.location.start_line,
            item.name or "",
        ),
    ):
        raw_file = unicodedata.normalize("NFC", fd.location.file.replace("\\", "/"))
        relative_file = PurePosixPath(raw_file)
        if relative_file.is_absolute() or ".." in relative_file.parts:
            raise ValueError(f"non-canonical snapshot path: {fd.location.file}")
        file = relative_file.as_posix()
        key = f"{file}:{fd.location.start_line}"
        if key in symbols:
            raise ValueError(f"duplicate population symbol: {key}")
        symbols[key] = {
            "stratum": classify(fd, definitions_per_name, lang, package_dirs),
            "eligible": True,
            "rule": None,
        }
    return {
        "policy_digest": policy_digest(policy),
        "lang": lang,
        "package_dirs": sorted(package_dirs),
        "symbols": symbols,
    }
