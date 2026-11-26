"""Load corpus registrations and verify the first available local candidate."""

from __future__ import annotations

from dataclasses import dataclass
import os
from pathlib import Path, PurePosixPath
import re

from .inputs import (PinError, SHA256, TreeBinding, bind_tree, git, git_tree_digest,
                     json_object, manifest_hash)

REPO = Path(__file__).resolve().parents[2]
DEFAULT_MANIFEST = Path(__file__).with_name("corpora.json")
SCHEMA = "prism.measure.corpora/1"
COMMIT = re.compile(r"[0-9a-f]{40}")
KEY = re.compile(r"[A-Za-z0-9_@+~-]+(?:\.[A-Za-z0-9_@+~-]+)*")
REQUIRED = {"name", "kind", "url", "commit", "root", "history", "local",
            "manifest_sha256", "cloud_asset", "tree_sha256"}


def text(value):
    return (isinstance(value, str) and bool(value.strip())
            and all(ord(char) >= 32 and ord(char) != 127 for char in value))


def relative(value):
    if not text(value) or "\\" in value:
        return False
    path = PurePosixPath(value)
    return not path.is_absolute() and not {"..", ".git"}.intersection(path.parts)


def digest(value, pattern=SHA256):
    return isinstance(value, str) and bool(pattern.fullmatch(value))


def load_manifest(path):
    data = json_object(path.read_bytes())
    if set(data) != {"schema", "corpora"} or data["schema"] != SCHEMA:
        raise ValueError("unsupported corpus manifest schema")
    entries = data["corpora"]
    if not isinstance(entries, dict):
        raise ValueError("corpora must be an object")
    for key, entry in entries.items():
        if (not KEY.fullmatch(key) or not isinstance(entry, dict)
                or not REQUIRED <= entry.keys() <= REQUIRED | {"archive_sha256"}):
            raise ValueError(f"invalid corpus entry {key!r}")
        if (entry["kind"] not in ("git", "tree") or not text(entry["name"])
                or not text(entry["root"]) or type(entry["history"]) is not bool):
            raise ValueError(f"invalid kind, name, root or history for {key}")
        if entry["url"] is not None and not text(entry["url"]):
            raise ValueError(f"invalid URL for {key}")
        if entry["commit"] is not None and not digest(entry["commit"], COMMIT):
            raise ValueError(f"invalid commit for {key}")
        if entry["kind"] == "git" and entry["commit"] is None:
            raise ValueError(f"missing Git commit for {key}")
        if (not isinstance(entry["local"], list) or not entry["local"]
                or not all(relative(path) for path in entry["local"])):
            raise ValueError(f"invalid local paths for {key}")
        asset = entry["cloud_asset"]
        if asset is not None and (not relative(asset) or "/" in asset or asset == "."):
            raise ValueError(f"invalid cloud asset for {key}")
        for field in ("manifest_sha256", "tree_sha256", "archive_sha256"):
            value = entry.get(field)
            if value is not None and not digest(value):
                raise ValueError(f"invalid {field} for {key}")
        if key.startswith("SB."):
            category, _, package = key[3:].partition(".")
            package_fields = {"name": key, "kind": "tree", "root": ".", "history": False,
                              "url": None, "commit": None, "cloud_asset": "secbench-recall-v1",
                              "local": [f"{category}/{package}/src/package",
                                        f"pkgs/{category}/{package}"]}
            if not category or not package or any(entry[k] != v for k, v in package_fields.items()):
                raise ValueError(f"invalid per-package corpus registration for {key}")
    return entries


def lookup(entry, search_roots):
    suffix = entry["root"]
    if not relative(suffix):
        raise PinError("query root escapes the selected checkout or enters .git")
    env_roots = os.environ.get("PRISM_MEASURE_CORPORA", "").split(os.pathsep)
    bases = [*search_roots, *(Path(path) for path in env_roots if path), REPO]
    for base in bases:
        for path in entry["local"]:
            checkout = (base / path).resolve()
            if not checkout.is_dir():
                continue
            current = checkout
            for part in PurePosixPath(suffix).parts:
                current /= part
                if current.is_symlink():
                    raise PinError("linked query root is not a bound directory")
            if current.is_dir():
                return checkout, current
    raise PinError("no local query root")


@dataclass(frozen=True)
class Corpus:
    key: str
    entry: dict
    root: Path
    pin_check: str
    binding: TreeBinding


def verify(key, entry, search_roots):
    try:
        checkout, root = lookup(entry, search_roots)
        git_checkout = entry["kind"] == "git" and os.path.lexists(checkout / ".git")
        if entry["kind"] == "git" and not git_checkout and os.path.lexists(root / ".git"):
            raise PinError("Git metadata is not at the registered checkout root")
        tree_pin = entry["tree_sha256"]
        if git_checkout:
            top = os.fsdecode(git(checkout, "rev-parse", "--show-toplevel").rstrip(b"\n"))
            if Path(top).resolve() != checkout or git(checkout, "rev-parse", "HEAD").strip().decode() != entry["commit"]:
                raise PinError("Git checkout or commit pin mismatch")
            expected = git_tree_digest(checkout, entry["commit"], entry["root"])
            if tree_pin is not None and expected != tree_pin:
                raise PinError("registered tree pin disagrees with the Git commit")
        else:
            if tree_pin is None:
                raise PinError("missing registered tree_sha256")
            expected = tree_pin
        binding = bind_tree(root)
        if binding.sha256 != expected:
            raise PinError("query root differs from its pinned bytes or entry types")
        if git_checkout:
            status = git(checkout, "--literal-pathspecs", "status", "--porcelain=v1", "-z",
                         "--untracked-files=all", "--ignored=matching", "--ignore-submodules=none",
                         "--", entry["root"])
            if status:
                raise PinError("dirty Git query root")
        if entry["manifest_sha256"] is not None:
            actual = manifest_hash(root, binding.members)
            if actual != entry["manifest_sha256"]:
                raise PinError("perf-cloud manifest digest mismatch")
        return Corpus(key, entry, root, "git-clean" if git_checkout else "manifest", binding)
    except (OSError, RuntimeError) as error:
        raise PinError(str(error)) from error


def list_corpora(entries):
    print("key\tname\tkind\tpin\thistory\tcloud_asset")
    for key, entry in sorted(entries.items()):
        print(key, entry["name"], entry["kind"], entry["commit"] or entry["tree_sha256"] or "",
              str(entry["history"]).lower(), entry["cloud_asset"] or "", sep="\t")
    return 0


def verify_corpora(entries, keys, search_roots):
    print("key\tstatus\tdetail")
    exit_code = 0
    for key in keys:
        try:
            corpus = verify(key, entries[key], search_roots)
        except PinError as error:
            print(key, "refused", " ".join(str(error).split()), sep="\t")
            exit_code = 3
        else:
            print(key, "ok", corpus.pin_check, sep="\t")
    return exit_code
