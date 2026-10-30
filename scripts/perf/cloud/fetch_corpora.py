#!/usr/bin/env python3
"""Fetch pinned public source archives using GitHub REST; never execute their code."""
import argparse
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import re
import shutil
import subprocess
import tarfile
import tempfile

REGISTRY = Path(__file__).with_name("corpora.json")
SHA256 = re.compile(r"[0-9a-f]{64}\Z")
KEY = re.compile(r"[A-Za-z0-9_-]+\Z")


def relative_path(value):
    if not isinstance(value, str) or not value or "\\" in value:
        raise ValueError(f"invalid relative path: {value!r}")
    path = PurePosixPath(value)
    if path.is_absolute() or ".." in path.parts or path == PurePosixPath("."):
        raise ValueError(f"invalid relative path: {value!r}")
    return path


def load_registry(path=REGISTRY):
    registry = json.loads(Path(path).read_text())
    if not isinstance(registry, dict) or not registry:
        raise ValueError("registry must be a nonempty object")
    for key, spec in registry.items():
        if not KEY.fullmatch(key) or not isinstance(spec, dict):
            raise ValueError(f"invalid corpus entry: {key!r}")
        asset = str(relative_path(spec.get("asset")))
        relative_path(spec.get("root"))
        if "/" in asset or asset in {"SHA256SUMS", "CORPORA.md"}:
            raise ValueError("asset must be an archive basename")
        digest = spec.get("manifest_sha256")
        if digest is not None and (not isinstance(digest, str) or not SHA256.fullmatch(digest)):
            raise ValueError("invalid manifest sha256")
    return registry


def sha256_file(path):
    digest = hashlib.sha256()
    with Path(path).open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def parse_checksums(text):
    sums = {}
    for line in text.splitlines():
        if not line.strip():
            continue
        match = re.fullmatch(r"([0-9a-f]{64}) [ *](.+)", line)
        if not match:
            raise ValueError("invalid SHA256SUMS line")
        digest, name = match.groups()
        relative_path(name)
        if "/" in name or name in sums:
            raise ValueError("invalid or duplicate checksum name")
        sums[name] = digest
    if not sums:
        raise ValueError("empty SHA256SUMS")
    return sums


def verify_checksum(path, sums):
    if path.name not in sums:
        raise ValueError(f"missing checksum for {path.name}")
    if sha256_file(path) != sums[path.name]:
        raise ValueError(f"checksum mismatch: {path.name}")


def manifest_hash(root):
    """GNU find -type f | bytewise sort | xargs sha256sum | sha256sum."""
    files = []
    for directory, _, names in os.walk(root, followlinks=False):
        for name in names:
            path = Path(directory) / name
            if path.is_file() and not path.is_symlink():
                files.append(path.relative_to(root).as_posix())
    digest = hashlib.sha256()
    for name in sorted(files, key=os.fsencode):
        escaped = name.replace("\\", "\\\\").replace("\n", "\\n")
        prefix = "\\" if escaped != name else ""
        line = f"{prefix}{sha256_file(Path(root) / name)}  {escaped}\n"
        digest.update(os.fsencode(line))
    return digest.hexdigest()


def verify_manifest(root, expected):
    if expected is not None and manifest_hash(root) != expected:
        raise ValueError(f"manifest mismatch: {root}")


def extract_archive(archive, destination):
    """Extract regular files/directories first, then internal symlinks; no tar code execution."""
    destination = Path(destination)
    destination.mkdir(parents=True, exist_ok=False)
    with tarfile.open(archive, "r:gz") as tar:
        members = tar.getmembers()
        seen = set()
        symlinks = []
        for member in members:
            path = relative_path(member.name.rstrip("/"))
            name = str(path)
            if name in seen:
                raise ValueError(f"duplicate archive member: {name}")
            seen.add(name)
            if member.issym():
                target = destination / path / ".." / member.linkname
                try:
                    target.resolve().relative_to(destination.resolve())
                except ValueError:
                    raise ValueError(f"symlink escapes archive: {name}") from None
                symlinks.append((path, member.linkname))
            elif not (member.isfile() or member.isdir()):
                raise ValueError(f"unsupported archive member: {name}")
        link_paths = {path for path, _ in symlinks}
        for member in members:
            path = relative_path(member.name.rstrip("/"))
            if any(parent in link_paths for parent in path.parents):
                raise ValueError("archive writes through a symlink")
            if not member.issym():
                output = destination / path
                if member.isdir():
                    output.mkdir(parents=True, exist_ok=True)
                else:
                    output.parent.mkdir(parents=True, exist_ok=True)
                    with tar.extractfile(member) as source, output.open("xb") as target:
                        shutil.copyfileobj(source, target)
        for path, target in symlinks:
            output = destination / path
            if output.exists():
                raise ValueError("archive symlink replaces directory")
            output.parent.mkdir(parents=True, exist_ok=True)
            output.symlink_to(target)
        # A chain of individually internal links may still resolve outside the tree.
        for path, _ in symlinks:
            try:
                (destination / path).resolve().relative_to(destination.resolve())
            except (ValueError, RuntimeError):
                raise ValueError("archive symlink chain escapes or cycles") from None


def gh_api(endpoint, output=None):
    if output is not None:
        with output.open("wb") as stream:
            subprocess.run(["gh", "api", "-H", "Accept: application/octet-stream", endpoint],
                           stdout=stream, check=True)
        return None
    result = subprocess.run(["gh", "api", "--paginate", "--slurp", endpoint],
                            capture_output=True, text=True, check=True)
    return [item for page in json.loads(result.stdout) for item in page]


def fetch_corpora(registry, keys, work_dir, repo="shoedog/prism", release="corpora-v1"):
    releases = [r for r in gh_api(f"repos/{repo}/releases") if r["name"] == release]
    if len(releases) != 1:
        raise ValueError(f"expected exactly one release named {release}; found {len(releases)}")
    assets = gh_api(f"repos/{repo}/releases/{releases[0]['id']}/assets")
    by_name = {}
    for asset in assets:
        if asset["name"] in by_name:
            raise ValueError(f"duplicate release asset: {asset['name']}")
        by_name[asset["name"]] = asset["id"]
    names = list(dict.fromkeys(["SHA256SUMS", "CORPORA.md"] + [registry[k]["asset"] for k in keys]))
    if any(name not in by_name for name in names):
        raise ValueError("release missing required assets")
    work_dir = Path(work_dir)
    work_dir.mkdir(parents=True, exist_ok=True)
    roots = {}
    # Fresh staging prevents an old successful fetch from masking a failed verification.
    with tempfile.TemporaryDirectory(prefix=".fetch-", dir=work_dir) as temp:
        staging = Path(temp)
        for name in names:
            gh_api(f"repos/{repo}/releases/assets/{by_name[name]}", output=staging / name)
        sums = parse_checksums((staging / "SHA256SUMS").read_text())
        for key in keys:
            verify_checksum(staging / registry[key]["asset"], sums)
        if "CORPORA.md" in sums:
            verify_checksum(staging / "CORPORA.md", sums)
        # Verify every selected corpus before publishing any of them.
        for key in keys:
            spec = registry[key]
            extract_archive(staging / spec["asset"], staging / key)
            root = staging / key / spec["root"]
            if not root.is_dir():
                raise ValueError(f"archive missing corpus root: {key}")
            verify_manifest(root, spec.get("manifest_sha256"))
            if (work_dir / "corpora" / key).exists():
                raise ValueError(f"corpus already exists; use a fresh work dir: {key}")
        (work_dir / "corpora").mkdir(exist_ok=True)
        for key in keys:
            (staging / key).rename(work_dir / "corpora" / key)
            roots[key] = work_dir / "corpora" / key / registry[key]["root"]
        for name in ["SHA256SUMS", "CORPORA.md"]:
            shutil.copyfile(staging / name, work_dir / name)
    return roots


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--registry", type=Path, default=REGISTRY)
    parser.add_argument("--corpora", default="T,X")
    parser.add_argument("--work-dir", type=Path, default=Path("/work"))
    parser.add_argument("--repo", default="shoedog/prism")
    parser.add_argument("--release", default="corpora-v1")
    args = parser.parse_args(argv)
    try:
        registry = load_registry(args.registry)
        keys = args.corpora.split(",")
        if len(set(keys)) != len(keys) or any(k not in registry for k in keys):
            raise ValueError("unknown or duplicate corpus keys")
        print(json.dumps({key: str(root) for key, root in fetch_corpora(
            registry, keys, args.work_dir, args.repo, args.release).items()}, indent=2))
    except (ValueError, OSError, subprocess.SubprocessError, tarfile.TarError) as error:
        parser.exit(1, f"fetch failed: {error}\n")


if __name__ == "__main__":
    main()
