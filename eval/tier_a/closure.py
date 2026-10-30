"""Declared-input closure manifests for Tier-A anchors."""
from __future__ import annotations

import fnmatch
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
    "corpus_control_paths",
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
    "accounting.py",
    "adjudication.py",
    "admission.py",
    "artifacts.py",
    "bootstrap.py",
    "boundary.py",
    "closure.py",
    "compare.py",
    "compare_render.py",
    "compare_report.py",
    "corpus.py",
    "lock.py",
    "lsp_client.py",
    "launch.py",
    "materialize.py",
    "matrix.py",
    "member_sample.py",
    "metrics.py",
    "model.py",
    "oracles.py",
    "pinned.py",
    "policy.py",
    "report.py",
    "record_schema.py",
    "runner.py",
    "spotcheck.py",
    "strata.py",
    "sut.py",
    "tsserver.py",
)

_POLICY_EXCLUSIONS = {
    "__init__.py": "package marker with no measurement behavior",
    "cli.py": "argument parsing, dispatch, printing, and paths only",
    "commands.py": "compatibility alias for unhashed command plumbing",
    "interfaces.py": "unused protocol declarations; measurement imports concrete adapters",
    "manifest.py": "separate interface-dispatch study not imported by Tier-A measurement",
}

_CORPUS_CONTROL_FILES = {
    "Cargo.lock",
    "build.rs",
    "go.sum",
    "pyproject.toml",
    "rust-project.json",
    "rust-toolchain",
    "rust-toolchain.toml",
}


def _native_loader_manifest_basenames(source_path: Path | None = None) -> frozenset[str]:
    """Derive the recursive manifest topology from the native loader predicate."""
    if source_path is None:
        source_path = Path(__file__).resolve().parents[2] / "src/repo_loader.rs"
    source = source_path.read_text()
    predicate = re.search(
        r"fn is_manifest_name\(name: &str\) -> bool \{(.*?)\n\}", source, re.S
    )
    if predicate is None:
        raise ValueError("native_manifest_predicate_unreadable")
    names = frozenset(re.findall(r'"([^"]+)"', predicate.group(1)))
    if not names:
        raise ValueError("native_manifest_predicate_empty")
    return names


_NATIVE_MANIFEST_BASENAMES = _native_loader_manifest_basenames()


def _is_corpus_control_path(path: str) -> bool:
    return (
        path in _CORPUS_CONTROL_FILES
        or PurePosixPath(path).name in _NATIVE_MANIFEST_BASENAMES
    )


def corpus_control_paths(root: Path, *, tracked_only: bool) -> list[str]:
    """Enumerate control files consumed by corpus/native loading."""
    if tracked_only:
        result = subprocess.run(
            ["git", "ls-files", "-z"],
            cwd=root,
            check=True,
            capture_output=True,
        )
        candidates = [
            entry.decode("utf-8")
            for entry in result.stdout.split(b"\0")
            if entry
        ]
    else:
        candidates = [
            path.relative_to(root).as_posix()
            for path in root.rglob("*")
            if path.is_file()
        ]
    return sorted(path for path in candidates if _is_corpus_control_path(path))


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
    root: Path,
    expected_paths: set[str],
    lang: str,
    excludes: list[str],
) -> list[tuple[str, str]]:
    """Enumerate closure-applicable checkout paths and their raw-byte digests."""
    applicable = set()
    extensions = tuple(EXTENSIONS[lang])
    git_entry = root / ".git"
    if git_entry.exists():
        applicable.add(".git")
    for disk_path in root.rglob("*"):
        if not disk_path.is_file():
            continue
        lexical_path = disk_path.relative_to(root).as_posix()
        is_control = _is_corpus_control_path(lexical_path)
        path = lexical_path if is_control else canonical_path(root, disk_path)
        if (
            lexical_path in expected_paths
            or is_control
            or (
                lexical_path.endswith(extensions)
                and not any(
                    fnmatch.fnmatch(lexical_path, pattern) for pattern in excludes
                )
            )
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


def _fingerprint_discriminator(path: Path) -> str:
    match = re.search(r"-([0-9a-f]{16})$", path.name)
    if match is None:
        raise ValueError(f"dependency_unit_invalid: {path.name}")
    return match.group(1)


def _sut_root_fingerprint(binary: Path, target: str) -> Path:
    fingerprint_root = binary.parent / ".fingerprint"
    artifact_root = binary.parent / "deps"
    artifacts = [
        path
        for path in sorted(artifact_root.glob(f"{target}-*"))
        if path.is_file()
        and re.fullmatch(rf"{re.escape(target)}-[0-9a-f]{{16}}", path.name)
    ]
    if not artifacts or not fingerprint_root.is_dir() or not binary.is_file():
        raise ValueError(f"sut_fingerprint_unavailable: {target}")

    binary_digest = raw_digest(binary)
    selected = [path for path in artifacts if raw_digest(path) == binary_digest]
    if len(selected) > 1:
        raise ValueError(f"sut_fingerprint_ambiguous: {target}")
    if not selected:
        raise ValueError(f"sut_fingerprint_unavailable: {target}")

    discriminator = _fingerprint_discriminator(selected[0])
    dep_info = artifact_root / f"{target}-{discriminator}.d"
    roots = sorted(
        fingerprint_root.glob(f"*-{discriminator}/bin-{target}.json")
    )
    if not dep_info.is_file() or not roots:
        raise ValueError(f"sut_fingerprint_unavailable: {target}")
    if len(roots) > 1:
        raise ValueError(f"sut_fingerprint_ambiguous: {target}")
    return roots[0]


def _unit_dep_info(binary: Path, fingerprint: Path) -> Path | None:
    discriminator = _fingerprint_discriminator(fingerprint.parent)
    if fingerprint.name.startswith("lib-"):
        target_name = fingerprint.name.removeprefix("lib-").removesuffix(".json")
        unit = f"{target_name}-{discriminator}"
        dep_info = binary.parent / "deps" / f"{unit}.d"
    elif fingerprint.name == "build-script-build-script-build.json":
        unit = f"build_script_build-{discriminator}"
        dep_info = (
            binary.parent
            / "build"
            / fingerprint.parent.name
            / f"{unit}.d"
        )
    else:
        return None
    if not dep_info.is_file():
        raise ValueError(f"dependency_dep_info_unavailable: {unit}")
    return dep_info


def _dependency_unit_matches(
    fingerprint_root: Path, name: str, marker: str
) -> list[Path]:
    if name == "build_script_build":
        markers = [
            *fingerprint_root.glob("*/run-build-script-build-script-build"),
            *fingerprint_root.glob("*/build-script-build-script-build"),
        ]
    else:
        unit = _cargo_crate_name(name)
        markers = list(fingerprint_root.glob(f"*/lib-{unit}"))
    matches = []
    for path in markers:
        fingerprint = path.with_name(f"{path.name}.json")
        if path.read_text().strip() == marker and fingerprint.is_file():
            matches.append(fingerprint)
    if (
        len(matches) > 1
        and all(path.name == "run-build-script-build-script-build.json" for path in matches)
        and len({path.read_bytes() for path in matches}) == 1
    ):
        return [sorted(matches)[0]]
    return sorted(matches)


def _cargo_dependency_dep_infos(_repo: Path, binary: Path) -> list[Path]:
    fingerprint_root = binary.parent / ".fingerprint"
    target = _cargo_crate_name(binary.stem)
    root = _sut_root_fingerprint(binary, target)

    queue = [root]
    visited = set()
    dep_infos = set()
    while queue:
        fingerprint = queue.pop()
        if fingerprint in visited:
            continue
        visited.add(fingerprint)
        dep_info = _unit_dep_info(binary, fingerprint)
        if dep_info is not None:
            dep_infos.add(dep_info)
        data = json.loads(fingerprint.read_text())
        for dependency in data.get("deps", []):
            if not isinstance(dependency, list) or len(dependency) < 2:
                raise ValueError(
                    f"dependency_unit_invalid: {fingerprint.parent.name}"
                )
            name = dependency[1]
            if not isinstance(name, str):
                raise ValueError(
                    f"dependency_unit_invalid: {fingerprint.parent.name}"
                )
            # Only the selected binary's run/compile chain crosses host profiles.
            if name == "build_script_build" and fingerprint.name.startswith("lib-"):
                continue
            if len(dependency) < 4 or not isinstance(dependency[3], int):
                raise ValueError(f"dependency_unit_invalid: {name}")
            try:
                marker = dependency[3].to_bytes(8, "little").hex()
            except OverflowError as exc:
                raise ValueError(f"dependency_unit_invalid: {name}") from exc
            matches = _dependency_unit_matches(fingerprint_root, name, marker)
            if name == "build_script_build":
                if fingerprint.name.startswith("bin-"):
                    matches = [
                        path
                        for path in matches
                        if path.name == "run-build-script-build-script-build.json"
                    ]
                elif fingerprint.name == "run-build-script-build-script-build.json":
                    compile_matches = [
                        path
                        for path in matches
                        if path.name == "build-script-build-script-build.json"
                    ]
                    if compile_matches:
                        matches = compile_matches
                    else:
                        ordering_matches = [
                            path
                            for path in matches
                            if path.name
                            == "run-build-script-build-script-build.json"
                        ]
                        if len(ordering_matches) > 1:
                            raise ValueError(
                                "dependency_unit_ambiguous: "
                                f"build_script_build:{marker}"
                            )
                        if not ordering_matches:
                            raise ValueError(
                                "build_script_compile_unit_unavailable: "
                                f"build_script_build:{marker}"
                            )
                        # A resolved run unit is an ordering edge, not this run's
                        # compiler unit.
                        continue
            unit = _cargo_crate_name(name)
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
