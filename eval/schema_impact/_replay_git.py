"""Read historical trees and follow source lines through first-parent history."""

from ast import literal_eval
from collections.abc import Iterator
from contextlib import contextmanager
from dataclasses import dataclass
from functools import cache
from pathlib import Path
import re
import subprocess
import tarfile
from tempfile import TemporaryDirectory


@dataclass(frozen=True)
class SourceLine:
    file: str
    line: int
    text: str


@dataclass(frozen=True)
class Commit:
    commit: str
    date: str
    subject: str
    earliest_release_tag: str | None


@dataclass(frozen=True)
class TracedLine:
    initial: SourceLine
    last_commit: str
    last_line: SourceLine


def _command(repo: Path, *args: str) -> list[str]:
    return ["git", "-C", str(repo), *args]


def git(repo: Path, *args: str) -> bytes:
    return subprocess.check_output(_command(repo, *args))


def revision(repo: Path, name: str) -> str:
    return git(repo, "rev-parse", "--verify", f"{name}^{{commit}}").decode().strip()


def first_parents(repo: Path, pin: str) -> list[str]:
    return git(repo, "rev-list", "--first-parent", "--reverse", revision(repo, pin)).decode().splitlines()


def parent(repo: Path, commit: str) -> str | None:
    _, *parents = git(repo, "rev-list", "--parents", "-n", "1", commit).decode().split()
    return parents[0] if parents else None


def changed_files(repo: Path, before: str, after: str, *paths: str) -> list[str]:
    output = git(repo, "diff", "--no-renames", "--name-only", "-z", before, after, "--", *paths)
    return [name.decode() for name in output.split(b"\0") if name]


def regular_files(repo: Path, commit: str, *paths: str) -> list[str]:
    output = git(repo, "ls-tree", "-r", "-z", commit, "--", *paths)
    result = []
    for row in output.split(b"\0"):
        if row.startswith(b"100"):
            result.append(row.split(b"\t", 1)[1].decode())
    return result


def blob(repo: Path, commit: str, file: str) -> bytes:
    return git(repo, "show", f"{commit}:{file}")


@contextmanager
def materialize(repo: Path, commit: str) -> Iterator[Path]:
    """Use the archive fallback for read-only repositories and Git metadata."""
    with TemporaryDirectory(prefix="schema-impact-parent-") as directory:
        root = Path(directory).resolve()
        args = _command(repo, "archive", "--format=tar", commit)
        with subprocess.Popen(args, stdout=subprocess.PIPE) as process:
            assert process.stdout is not None
            try:
                with tarfile.open(fileobj=process.stdout, mode="r|") as archive:
                    archive.extractall(root, filter="data")
            finally:
                process.stdout.close()
            if process.wait():
                raise subprocess.CalledProcessError(process.returncode, args)
        yield root


class History:
    def __init__(self, repo: Path, pin: str):
        self.repo = repo
        pin = revision(repo, pin)
        log = git(repo, "log", "-z", "--first-parent", "--reverse", "--format=%H%x00%ct", pin)
        rows = log.decode().rstrip("\0").split("\0")
        self.seconds = {sha: int(date) for sha, date in zip(rows[::2], rows[1::2], strict=True)}
        self.commits = list(self.seconds)
        self.position = {sha: i for i, sha in enumerate(self.commits)}
        self.pin = pin
        self.detail = cache(self.detail)

    def detail(self, sha: str) -> Commit:
        date, subject = git(self.repo, "show", "-s", "--format=%cI%x00%s", sha).decode().rstrip("\n").split("\0")
        tags = git(self.repo, "tag", "--contains", sha, "--sort=creatordate").decode().splitlines()
        return Commit(sha, date, subject, next(iter(tags), None))

    def successor(self, sha: str) -> str | None:
        following = self.position[sha] + 1
        return self.commits[following] if following < len(self.commits) else None


def _filename(raw: bytes) -> str:
    # Git's quoted paths use octal bytes and the standard C escapes.
    return literal_eval("b" + raw.decode("ascii")).decode() if raw.startswith(b'"') else raw.decode()


class LineFollower:
    def __init__(self, history: History, p0: str, snapshot: Path | None = None):
        if p0 not in history.position:
            raise ValueError(f"{p0} is not on the pin's first-parent history")
        self.history = history
        self.p0 = p0
        self.snapshot = snapshot
        self.git_dir = (git(history.repo, "rev-parse", "--absolute-git-dir").decode().strip()
                        if snapshot is not None else None)
        self.trace = cache(self.trace)

    def trace(self, file: str, line: int) -> TracedLine:
        if self.p0 == self.history.pin:
            source = blob(self.history.repo, self.p0, file).split(b"\n")[line - 1].decode()
            site = SourceLine(file, line, source)
            return TracedLine(site, self.p0, site)
        repo = self.history.repo
        args = ("blame", "--reverse", "--first-parent", "-M", "-C", "--line-porcelain",
                "-L", f"{line},{line}", f"{self.p0}..{self.history.pin}", "--", file)
        if self.snapshot is None:
            result = git(repo, *args)
        else:
            result = git(self.snapshot, f"--git-dir={self.git_dir}", f"--work-tree={self.snapshot}", *args)
        header, *body = result.split(b"\n")
        sha, last_line, *_ = header.decode().split()
        path = next(row.removeprefix(b"filename ") for row in body if row.startswith(b"filename "))
        source = next(row[1:].decode() for row in body if row.startswith(b"\t"))
        return TracedLine(SourceLine(file, line, source), sha, SourceLine(_filename(path), int(last_line), source))

    def first_touch(self, traces: list[TracedLine]) -> str | None:
        return self.history.successor(min(
            (trace.last_commit for trace in traces), key=self.history.position.__getitem__,
            default=self.history.pin,
        ))


_HUNK = re.compile(r"^@@ -(\d+)(?:,(\d+))? \+.* @@")

def touch_hunk(repo: Path, trace: TracedLine, touching: str) -> str | None:
    site = trace.last_line
    patch = git(repo, "diff", "--no-ext-diff", "--no-textconv", "--no-color", "--find-renames",
                trace.last_commit, touching, "--", f":(literal){site.file}").decode(errors="replace")
    lines = patch.splitlines()
    starts = [i for i, row in enumerate(lines) if _HUNK.match(row)]
    for i, end in zip(starts, starts[1:] + [len(lines)]):
        match = _HUNK.match(lines[i])
        assert match is not None
        first, count = int(match[1]), int(match[2] or 1)
        if first <= site.line < first + count:
            return "\n".join(lines[:starts[0]] + lines[i:end])
    return patch.rstrip() or None
