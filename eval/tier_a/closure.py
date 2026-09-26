"""Declared-input closure manifests for Tier-A anchors."""
from __future__ import annotations

import hashlib
import json
import re
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
    "checkout_corpus_manifest",
    "manifest_digest",
    "write_tsv",
    "read_tsv",
    "sut_input_paths",
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

_CORPUS_CONTROL_FILES = {
    "Cargo.lock",
    "Cargo.toml",
    "build.rs",
    "go.mod",
    "go.sum",
    "go.work",
    "pyproject.toml",
    "rust-project.json",
    "rust-toolchain",
    "rust-toolchain.toml",
}
_CORPUS_SOURCE_EXTENSIONS = tuple(
    sorted({extension for extensions in EXTENSIONS.values() for extension in extensions})
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


def checkout_corpus_manifest(
    root: Path, expected_paths: set[str]
) -> list[tuple[str, str]]:
    """Enumerate closure-applicable checkout paths and their raw-byte digests."""
    applicable = set()
    git_entry = root / ".git"
    if git_entry.exists():
        applicable.add(".git")
    for disk_path in root.rglob("*"):
        if not disk_path.is_file():
            continue
        path = canonical_path(root, disk_path)
        if (
            path in expected_paths
            or disk_path.name in _CORPUS_CONTROL_FILES
            or path.endswith(_CORPUS_SOURCE_EXTENSIONS)
        ):
            applicable.add(path)
    rows = [
        (path, raw_digest(root / path))
        for path in applicable
        if path != ".git"
    ]
    if ".git" in applicable:
        rows.append((".git", "present"))
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


def makefile_dependencies(path: Path) -> list[str]:
    contents = path.read_text().replace("\\\n", "")
    rule = next((line for line in contents.splitlines() if line.strip()), "")
    _target, separator, dependencies = rule.partition(":")
    if not separator:
        raise ValueError(f"invalid dep-info: {path}")
    return shlex.split(dependencies, comments=False, posix=True)


def cargo_package_name(repo: Path) -> str:
    try:
        return tomllib.loads((repo / "Cargo.toml").read_text())["package"]["name"]
    except (OSError, KeyError, tomllib.TOMLDecodeError):
        return "prism"


def cargo_build_outputs(repo: Path) -> list[Path]:
    package = cargo_package_name(repo)
    prefixes = {package, package.replace("-", "_")}
    return [
        output
        for output in sorted((repo / "target/release/build").glob("*/output"))
        if any(output.parent.name.startswith(f"{prefix}-") for prefix in prefixes)
    ]


def _cargo_crate_name(name: str) -> str:
    return name.replace("-", "_")


def _cargo_dependency_dep_infos(repo: Path, binary: Path) -> list[Path]:
    fingerprint_root = binary.parent / ".fingerprint"
    if not fingerprint_root.is_dir():
        return []
    package = _cargo_crate_name(cargo_package_name(repo))
    target = _cargo_crate_name(binary.stem)
    roots = sorted(fingerprint_root.glob(f"*/bin-{target}.json"))
    if not roots and target != package:
        roots = sorted(fingerprint_root.glob(f"*/bin-{package}.json"))
    if not roots:
        raise ValueError(f"dependency_graph_root_unavailable: {target}")
    if len(roots) != 1:
        raise ValueError(f"dependency_graph_root_ambiguous: {target}")

    queue = list(roots)
    visited = set()
    dep_infos = set()
    while queue:
        fingerprint = queue.pop()
        if fingerprint in visited:
            continue
        visited.add(fingerprint)
        if fingerprint.name.startswith("lib-"):
            target_name = fingerprint.name.removeprefix("lib-").removesuffix(".json")
            discriminator = re.search(r"-([0-9a-f]{16})$", fingerprint.parent.name)
            if discriminator is None:
                raise ValueError(
                    f"dependency_unit_invalid: {fingerprint.parent.name}"
                )
            unit = f"{target_name}-{discriminator.group(1)}"
            dep_info = binary.parent / "deps" / f"{unit}.d"
            if not dep_info.is_file():
                raise ValueError(f"dependency_dep_info_unavailable: {unit}")
            dep_infos.add(dep_info)
        data = json.loads(fingerprint.read_text())
        for dependency in data.get("deps", []):
            if not isinstance(dependency, list) or len(dependency) < 2:
                raise ValueError(
                    f"dependency_unit_invalid: {fingerprint.parent.name}"
                )
            name = dependency[1]
            if not isinstance(name, str) or name == "build_script_build":
                continue
            if len(dependency) < 4 or not isinstance(dependency[3], int):
                raise ValueError(f"dependency_unit_invalid: {name}")
            unit = _cargo_crate_name(name)
            marker = dependency[3].to_bytes(8, "little").hex()
            matches = [
                path.with_name(f"{path.name}.json")
                for path in fingerprint_root.glob(f"*/lib-{unit}")
                if path.read_text().strip() == marker
            ]
            if not matches:
                raise ValueError(f"dependency_unit_unavailable: {unit}:{marker}")
            if len(matches) != 1:
                raise ValueError(f"dependency_unit_ambiguous: {unit}:{marker}")
            queue.append(matches[0])
    return sorted(dep_infos)


def _input_files(repo: Path, value: str) -> list[Path]:
    if value.startswith("env-dep:"):
        return []
    candidate = Path(value)
    path = candidate if candidate.is_absolute() else repo / candidate
    try:
        path.resolve().relative_to(repo.resolve())
    except ValueError:
        return []
    if path.is_dir():
        return sorted(item for item in path.rglob("*") if item.is_file())
    return [path]


def sut_input_paths(
    repo: Path, binary: Path, *, require_dep_info: bool = True
) -> list[str]:
    dep_info = binary.with_suffix(".d")
    declared = (
        makefile_dependencies(dep_info)
        if require_dep_info or dep_info.is_file()
        else []
    )
    for dep_info in _cargo_dependency_dep_infos(repo, binary):
        declared.extend(makefile_dependencies(dep_info))
    for output in cargo_build_outputs(repo):
        for line in output.read_text().splitlines():
            for prefix in ("cargo:rerun-if-changed=", "cargo::rerun-if-changed="):
                if line.startswith(prefix):
                    declared.append(line.removeprefix(prefix))
                    break
    declared.extend(
        name for name in ("Cargo.toml", "Cargo.lock") if (repo / name).is_file()
    )

    inputs = set()
    for declared_path in declared:
        for disk_path in _input_files(repo, declared_path):
            inputs.add(canonical_path(repo, disk_path))
    return sorted(inputs)


def sut_inputs_manifest(repo: Path, binary: Path) -> list[tuple[str, str]]:
    return [
        (path, raw_digest(repo / path) if (repo / path).is_file() else "absent")
        for path in sut_input_paths(repo, binary)
    ]


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
