"""Load public population pins and reconstruct the exact #602 review spec."""

from __future__ import annotations

from dataclasses import dataclass
import hashlib
import json
from pathlib import Path
import re
import subprocess

from .corpora import COMMIT, KEY, digest, text
from .edges import integer
from .inputs import PinError, git_args, git_env, git_tree_digest, json_object

SCHEMA = "prism.measure.population/1"
SPEC_RULE = {
    "git_diff": ["-U0", "-M"],
    "extensions": [".cjs", ".cts", ".go", ".js", ".jsx", ".mjs", ".mts", ".py", ".ts", ".tsx"],
    "exclude_test": {
        "basename": r"(\.test\.|\.spec\.|\.e2e-spec\.|_test\.go$|^test_.*\.py$|_test\.py$|^conftest\.py$)",
        "path": r"(^|/)(tests?|__tests__|__mocks__|__snapshots__|testing)(/|$)",
    },
    "modify_type": "Modified",
}
TEST_BASENAME = re.compile(SPEC_RULE["exclude_test"]["basename"])
TEST_PATH = re.compile(SPEC_RULE["exclude_test"]["path"])
ROW_FIELDS = {"ordinal", "repo", "commit", "parent", "tree", "spec_sha256", "n_files", "n_lines"}
DIFF_ARGS = (
    "-c", "core.quotePath=true", "diff", "--no-color", "--no-ext-diff", "--no-textconv",
    "--no-relative", "--src-prefix=a/", "--dst-prefix=b/", "--diff-algorithm=myers",
    "--indent-heuristic", "--inter-hunk-context=0", "-U0", "-M",
)
HUNK = re.compile(rb"@@ -(\d+)(?:,(\d+))? \+(\d+)(?:,(\d+))? @@(?: .*)?")
RECORD = re.compile(rb'diff --git "?a/.* "?b/.*', re.S)
ESCAPES = dict(zip(b'"\\abtnvfr', b'"\\\a\b\t\n\v\f\r'))
ESCAPE = re.compile(rb'\\(?:["\\abtnvfr]|[0-3][0-7]{2})')
QUOTED_PATH = re.compile(rb'"((?:[^"\\]|\\(?:["\\abtnvfr]|[0-3][0-7]{2}))*)"')
NO_NEWLINE = rb"\ No newline at end of file"


@dataclass(frozen=True)
class Population:
    sha256: str
    repos: dict
    diffs: list


def load(path):
    raw = path.read_bytes()
    manifest = json_object(raw)
    if (set(manifest) != {"schema", "name", "repos", "spec_rule", "diffs"}
            or manifest["schema"] != SCHEMA or not text(manifest["name"])
            or manifest["spec_rule"] != SPEC_RULE):
        raise ValueError("invalid review population or spec rule")
    repos, diffs = manifest["repos"], manifest["diffs"]
    if not isinstance(repos, dict) or not isinstance(diffs, list):
        raise ValueError("review population requires a repos object and diffs array")
    for name, repo in repos.items():
        if (not KEY.fullmatch(name) or not isinstance(repo, dict)
                or set(repo) != {"url", "selection_head"} or not text(repo["url"])
                or not digest(repo["selection_head"], COMMIT)):
            raise ValueError(f"invalid population repository: {name!r}")
    seen = set()
    for row in diffs:
        if (not isinstance(row, dict) or row.keys() != ROW_FIELDS
                or not integer(row["ordinal"], 1) or row["ordinal"] in seen
                or not isinstance(row["repo"], str) or row["repo"] not in repos
                or not all(digest(row[field], COMMIT) for field in ("commit", "parent", "tree"))
                or not digest(row["spec_sha256"])
                or not all(integer(row[field], 0) for field in ("n_files", "n_lines"))):
            raise ValueError("invalid population diff or duplicate ordinal")
        seen.add(row["ordinal"])
    return Population(hashlib.sha256(raw).hexdigest(), repos, sorted(diffs, key=lambda r: r["ordinal"]))


def _git(checkout, *args):
    env = git_env()
    env.update(LC_ALL="C", GIT_CONFIG_NOSYSTEM="1", GIT_CONFIG_GLOBAL="/dev/null",
               GIT_ALLOW_PROTOCOL="file", GIT_ATTR_NOSYSTEM="1")
    # A user attributes file can select clean/smudge filters for the scratch checkout.
    proc = subprocess.run(git_args(checkout, "-c", "core.attributesFile=/dev/null", *args),
                          env=env, capture_output=True)
    if proc.returncode:
        raise PinError(proc.stderr.decode("utf-8", "replace").strip() or "Git command failed")
    return proc.stdout


def _escape(match):
    seq = match[0][1:]
    return bytes([ESCAPES[seq[0]] if len(seq) == 1 else int(seq, 8)])


def _path(token, prefix):
    if token.startswith(b'"'):
        match = QUOTED_PATH.match(token)
        if not match or token[match.end():] and token[match.end():match.end()+1] != b"\t":
            raise PinError("malformed Git patch pathname")
        name = ESCAPE.sub(_escape, match[1])
    else:
        name = token.split(b"\t", 1)[0]
    if name == b"/dev/null":
        return None
    if not name.startswith(prefix) or b"\0" in name:
        raise PinError("invalid Git patch pathname prefix or bytes")
    try:
        path = name[len(prefix):].decode("utf-8")
    except UnicodeDecodeError as error:
        raise PinError("Git patch pathname is not UTF-8") from error
    if any(part in ("", ".", "..") for part in path.split("/")):
        raise PinError("invalid Git patch pathname components")
    return path


def _hunk(lines, index, header, added, sides, floor):
    """Consume one hunk; `sides` says which of the preimage and postimage exist and
    `floor` is the last postimage line an earlier hunk of this file reached."""
    old, new = (int(header[group] or b"1") for group in (2, 4))
    line_number = int(header[3])
    if (old and int(header[1]) == 0) or (new and line_number == 0) or not (old or new):
        raise PinError("invalid Git patch hunk range")
    if (old and not sides[0]) or (new and not sides[1]):
        raise PinError("Git patch hunk has lines on an absent file side")
    # -U0 hunks of one file are disjoint and ascending in the postimage; a pure
    # deletion sits after the line its header names.
    if line_number < floor + bool(new):
        raise PinError("Git patch hunks overlap or go backwards")
    floor = line_number + new - 1 if new else line_number
    while old or new:
        line = lines[index] if index < len(lines) else b""
        marker = line[:1]
        if marker not in (b"-", b"+", b" "):
            raise PinError("malformed or incomplete Git patch hunk")
        old -= marker != b"+"
        new -= marker != b"-"
        if old < 0 or new < 0:
            raise PinError("Git patch hunk exceeds declared length")
        if marker == b"+" and added is not None:
            added.add(line_number)
        line_number += marker != b"-"
        index += 1
        if index < len(lines) and lines[index] == NO_NEWLINE:
            index += 1
    return index, floor


def patch_spec(patch):
    """A header can select a file only outside a length-checked hunk."""
    lines = patch.split(b"\n")
    if lines.pop() != b"":
        raise PinError("Git patch lacks its terminating LF")
    files = {}
    index = 0
    in_file = has_postimage = False
    added = sides = None
    floor = 0
    while index < len(lines):
        line = lines[index]
        index += 1
        if line.startswith(b"diff --git "):
            if not RECORD.fullmatch(line):
                raise PinError("Git patch file record names no files")
            in_file, has_postimage = True, False
        elif line.startswith(b"--- ") and in_file and not has_postimage:
            old_path = _path(line[4:], b"a/")
            if index == len(lines) or not lines[index].startswith(b"+++ "):
                raise PinError("missing Git patch postimage header")
            path = _path(lines[index][4:], b"b/")
            index += 1
            has_postimage = True
            if path is not None and path in files:
                raise PinError("duplicate Git patch postimage")
            added = files.setdefault(path, set()) if path is not None else None
            sides, floor = (old_path is not None, path is not None), 0
        elif line.startswith(b"@@") and has_postimage:
            header = HUNK.fullmatch(line)
            if not header:
                raise PinError("malformed Git patch hunk header")
            index, floor = _hunk(lines, index, header, added, sides, floor)
        elif not in_file or has_postimage or line[:1] in (b"@", b"+", b"-", b" ", b"\\"):
            raise PinError("unexpected Git patch line")
    return {"files": [
        {"file_path": path, "modify_type": "Modified", "diff_lines": sorted(files[path])}
        for path in sorted(files)
        if Path(path).suffix in SPEC_RULE["extensions"]
        and not (TEST_BASENAME.search(path.rsplit("/", 1)[-1]) or TEST_PATH.search(path))
    ]}


def regenerate_spec(checkout, row):
    commit = row["commit"]
    if (_git(checkout, "rev-parse", "--verify", f"{commit}^").strip().decode("ascii") != row["parent"]
            or _git(checkout, "rev-parse", "--verify", f"{commit}^{{tree}}").strip().decode("ascii")
            != row["tree"]):
        raise PinError("review commit parent or tree does not match the population")
    spec = patch_spec(_git(checkout, *DIFF_ARGS, row["parent"], commit, "--"))
    raw = json.dumps(spec).encode("utf-8")
    files = spec["files"]
    if hashlib.sha256(raw).hexdigest() != row["spec_sha256"]:
        raise PinError("spec_sha256 mismatch")
    if len(files) != row["n_files"] or sum(len(f["diff_lines"]) for f in files) != row["n_lines"]:
        raise PinError("spec counts do not match the population")
    return raw


def materialize(source, scratch, row):
    # Keep local source worktrees and their refs untouched. Do not copy hooks or
    # inherit checkout filters from the machine's Git configuration.
    _git(source, "clone", "--shared", "--local", "--no-checkout", "--template=", "--", ".",
         str(scratch))
    spec = regenerate_spec(scratch, row)
    pinned = git_tree_digest(scratch, row["commit"], ".")
    _git(scratch, "-c", "core.hooksPath=/dev/null", "checkout", "--detach", row["commit"])
    return spec, pinned
