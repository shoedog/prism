"""Strict wire readers and §2 corpus input bindings."""

from __future__ import annotations

from collections import defaultdict, deque
from contextlib import contextmanager, nullcontext
from dataclasses import dataclass
import gzip
import hashlib
import json
import math
import os
import re
import shutil
import stat
import subprocess
import zlib

SHA256 = re.compile(r"[0-9a-f]{64}")
CHUNK = 1024 * 1024
DIRECTORY = os.O_DIRECTORY | os.O_NOFOLLOW | os.O_RDONLY
# Hosts refuse paths that expand more than a few dozen links (ELOOP). A tree
# whose links expand past this many hops cannot be resolved boundedly.
MAX_LINK_HOPS = 256


class PinError(ValueError):
    """A corpus cannot be bound to the requested pin."""


class UnboundLink(PinError):
    """A link can traverse outside the bound tree or participate in a cycle."""


class AliasedInput(PinError):
    """A hard-linked file may be read or deduplicated through another path."""


def unique_object(pairs):
    obj = {}
    for key, value in pairs:
        if key in obj:
            raise ValueError(f"duplicate JSON field {key!r}")
        obj[key] = value
    return obj


def finite_float(text):
    value = float(text)
    if not math.isfinite(value):
        raise ValueError(f"non-finite JSON number {text!r}")
    return value


def json_object(data):
    obj = json.loads(data.decode("utf-8"), object_pairs_hook=unique_object,
                     parse_constant=finite_float, parse_float=finite_float)
    if not isinstance(obj, dict):
        raise ValueError("expected a JSON object")
    return obj


def transport_lines(path):
    """Yield byte-LF records, verifying concatenated gzip members through EOF."""
    try:
        with path.open("rb") as raw:
            transport = nullcontext(raw)
            if path.suffix == ".gz":
                if raw.read(2) != b"\x1f\x8b":
                    raise ValueError(f"missing gzip member in {path.name}")
                raw.seek(0)
                transport = gzip.GzipFile(fileobj=raw, mode="rb")
            with transport as stream:
                yield from stream
    except (OSError, EOFError, zlib.error) as error:
        raise ValueError(f"cannot decode {path.name}: {error}") from error


def jsonl_objects(path):
    for line in transport_lines(path):
        if line.strip(b" \t\r\n"):
            yield json_object(line)


def sha256_file(path):
    with path.open("rb") as stream:
        return stream_hash(stream)


def stream_hash(stream):
    digest = hashlib.sha256()
    for block in iter(lambda: stream.read(CHUNK), b""):
        digest.update(block)
    return digest.hexdigest()


def frame(digest, value):
    digest.update(len(value).to_bytes(8, "big"))
    digest.update(value)


def record_header(digest, kind, name, size):
    digest.update(kind)
    frame(digest, name)
    digest.update(size.to_bytes(8, "big"))


def add_bytes(digest, stream, size):
    remaining = size
    while remaining:
        block = stream.read(min(CHUNK, remaining))
        if not block:
            raise OSError("input became shorter while hashing")
        digest.update(block)
        remaining -= len(block)


@contextmanager
def directory(root, parts=()):
    fd = os.open(root, DIRECTORY)
    try:
        for part in parts:
            next_fd = os.open(part, DIRECTORY, dir_fd=fd)
            os.close(fd)
            fd = next_fd
        yield fd
    finally:
        os.close(fd)


@contextmanager
def input_file(root, parts):
    with directory(root, parts[:-1]) as parent:
        fd = os.open(parts[-1], os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK, dir_fd=parent)
    with os.fdopen(fd, "rb") as stream:
        info = os.fstat(stream.fileno())
        if not stat.S_ISREG(info.st_mode) or not info.st_mode & 0o444:
            raise OSError(f"unreadable or special member {os.fsdecode(parts[-1])!r}")
        if info.st_nlink != 1:
            raise AliasedInput(f"hard-linked member {os.fsdecode(parts[-1])!r}")
        yield stream, info


@dataclass(frozen=True)
class Member:
    kind: bytes
    info: os.stat_result
    target: bytes = b""


def tree_members(root):
    members = {}

    def walk(fd, parts):
        for name in os.listdir(fd):
            name = os.fsencode(name)
            if name == b".git":
                continue
            location = (*parts, name)
            info = os.stat(name, dir_fd=fd, follow_symlinks=False)
            if stat.S_ISDIR(info.st_mode):
                members[location] = Member(b"D", info)
                child = os.open(name, DIRECTORY, dir_fd=fd)
                try:
                    walk(child, location)
                finally:
                    os.close(child)
            elif stat.S_ISLNK(info.st_mode):
                members[location] = Member(b"L", info, os.readlink(name, dir_fd=fd))
            elif stat.S_ISREG(info.st_mode):
                members[location] = Member(b"F", info)
            else:
                raise OSError(f"special member {os.fsdecode(b'/'.join(location))!r}")

    with directory(root) as fd:
        members[()] = Member(b"D", os.fstat(fd))
        walk(fd, ())
    return members


def inode(info):
    return info.st_dev, info.st_ino


def link_states(root, members):
    paths = {inode(member.info): parts for parts, member in members.items()}
    states = {}
    edges = defaultdict(list)

    def refuse(parts):
        raise UnboundLink(f"unbound-link: {os.fsdecode(b'/'.join(parts))!r}")

    def resolve(link):
        # Resolve relative to the containing directory, expanding a hop before
        # applying any subsequent '..'. Continue checking hops after ENOENT.
        position = link[:-1]
        pending = deque()
        active = set()
        dangling = False
        hops = 0

        def expand(parts, target):
            nonlocal hops
            hops += 1
            if parts in active or target.startswith(b"/") or not target or hops > MAX_LINK_HOPS:
                refuse(link)
            active.add(parts)
            pending.extendleft(reversed([*target.split(b"/"), parts]))

        expand(link, members[link].target)
        while pending:
            part = pending.popleft()
            if isinstance(part, tuple):
                active.remove(part)
                continue
            if position in members and members[position].kind != b"D":
                dangling = True
            if part in (b"", b"."):
                continue
            if part == b"..":
                if not position:
                    refuse(link)
                position = position[:-1]
                continue
            if part == b".git":
                refuse(link)
            candidate = (*position, part)
            if candidate not in members:
                # lstat checks the actual filesystem's case/normalization
                # behavior. Resolve aliases only to inventoried members.
                try:
                    with directory(root, position) as fd:
                        actual = os.stat(part, dir_fd=fd, follow_symlinks=False)
                except (FileNotFoundError, NotADirectoryError):
                    dangling = True
                else:
                    candidate = paths.get(inode(actual))
                    if candidate is None:
                        refuse(link)
            member = members.get(candidate)
            if member and member.kind == b"L":
                position = candidate[:-1]
                expand(candidate, member.target)
            else:
                position = candidate
        return (b"missing", ()) if dangling else (members[position].kind, position)

    for parts, member in members.items():
        if parts and member.kind == b"D":
            edges[parts[:-1]].append(parts)
        if member.kind == b"L":
            state = states[parts] = resolve(parts)
            if state[0] == b"D":
                edges[parts[:-1]].append(state[1])

    # A directory may be revisited through links without any single link
    # pointing directly at itself. Check the whole directory traversal graph.
    directories = {parts: 0 for parts, member in members.items() if member.kind == b"D"}
    for destinations in edges.values():
        for target in destinations:
            directories[target] += 1
    ready = deque(path for path, degree in directories.items() if degree == 0)
    while ready:
        current = ready.popleft()
        for target in edges[current]:
            directories[target] -= 1
            if directories[target] == 0:
                ready.append(target)
        del directories[current]
    if directories:
        raise UnboundLink("unbound-link: cyclic or ancestor directory link")
    return states


def has_git_member(root):
    """True when any file, link or directory of the tree is named `.git`."""
    return any(".git" in (*directories, *names)
               for _, directories, names in os.walk(root, followlinks=False))


def remove_tree(path):
    """Delete a private snapshot whose directories may be read-only."""
    def unlock(directory):
        os.chmod(directory, 0o700)
        with os.scandir(directory) as entries:
            children = [entry.path for entry in entries if entry.is_dir(follow_symlinks=False)]
        for child in children:
            unlock(child)

    unlock(path)
    shutil.rmtree(path)


def manifest_hash(root, members=None):
    """The perf-cloud digest over bound regular files, with GNU escapes."""
    members = tree_members(root) if members is None else members
    digest = hashlib.sha256()
    for parts in sorted(members, key=b"/".join):
        if members[parts].kind != b"F":
            continue
        name = b"/".join(parts)
        escaped = name.replace(b"\\", b"\\\\").replace(b"\n", b"\\n")
        prefix = b"\\" if escaped != name else b""
        with input_file(root, parts) as (stream, _):
            digest.update(prefix + stream_hash(stream).encode() + b"  " + escaped + b"\n")
    return digest.hexdigest()


@dataclass(frozen=True)
class TreeBinding:
    sha256: str
    state_sha256: str
    members: dict[tuple[bytes, ...], Member]


def bind_tree(root):
    members = tree_members(root)
    states = link_states(root, members)
    content = hashlib.sha256()
    structure = hashlib.sha256()
    for parts in sorted(members, key=b"/".join):
        member = members[parts]
        name = b"/".join(parts)
        structure.update(member.kind)
        frame(structure, name)
        if member.kind == b"L":
            record_header(content, b"L", name, len(member.target))
            content.update(member.target)
            kind, target = states[parts]
            frame(structure, kind)
            frame(structure, b"/".join(target))
        else:
            structure.update(stat.S_IMODE(member.info.st_mode).to_bytes(2, "big"))
            if member.kind == b"F":
                with input_file(root, parts) as (stream, info):
                    if inode(info) != inode(member.info) or info.st_mode != member.info.st_mode:
                        raise OSError(f"input changed while hashing {os.fsdecode(name)!r}")
                    record_header(content, b"F", name, info.st_size)
                    add_bytes(content, stream, info.st_size)
                    if stream.read(1):
                        raise OSError(f"input grew while hashing {os.fsdecode(name)!r}")
    return TreeBinding(content.hexdigest(), structure.hexdigest(), members)


def copy_snapshot(source, destination, binding):
    # Create directories before their contents and copy only the inventoried
    # regular files and literal links. No working-tree checkout filters run.
    for parts, member in sorted(binding.members.items()):
        path = destination.joinpath(*(os.fsdecode(part) for part in parts))
        if member.kind == b"D":
            path.mkdir(mode=0o700)
        elif member.kind == b"L":
            os.symlink(member.target, path)
        else:
            with input_file(source, parts) as (stream, _), path.open("xb") as output:
                shutil.copyfileobj(stream, output, CHUNK)
            path.chmod(stat.S_IMODE(member.info.st_mode))
    for parts, member in reversed(sorted(binding.members.items())):
        if member.kind == b"D":
            destination.joinpath(*(os.fsdecode(part) for part in parts)).chmod(
                stat.S_IMODE(member.info.st_mode))


def git_env():
    env = {name: value for name, value in os.environ.items() if not name.startswith("GIT_")}
    env.update(GIT_OPTIONAL_LOCKS="0", GIT_NO_REPLACE_OBJECTS="1", GIT_NO_LAZY_FETCH="1")
    return env


def git_args(checkout, *args):
    return ["git", "-C", str(checkout), *args]


def git(checkout, *args):
    result = subprocess.run(git_args(checkout, *args), env=git_env(), capture_output=True)
    if result.returncode:
        raise PinError(result.stderr.decode("utf-8", "replace").strip() or "git command failed")
    return result.stdout


def git_tree_digest(checkout, commit, root):
    tree = f"{commit}^{{tree}}" if root == "." else f"{commit}:{root}"
    if git(checkout, "cat-file", "-t", tree) != b"tree\n":
        raise PinError("the pinned query root is not a Git tree")
    entries = {}
    for row in git(checkout, "ls-tree", "-rz", tree).split(b"\0"):
        if not row:
            continue
        metadata, name = row.split(b"\t", 1)
        mode, kind, oid = metadata.split()
        if mode == b"160000":
            raise PinError(f"gitlink in pinned query root: {os.fsdecode(name)!r}")
        if kind != b"blob" or mode not in (b"100644", b"100755", b"120000"):
            raise PinError(f"unsupported Git entry: {os.fsdecode(name)!r}")
        if b".git" not in name.split(b"/"):
            entries[name] = (mode, oid)

    digest = hashlib.sha256()
    with subprocess.Popen(git_args(checkout, "cat-file", "--batch"),
                          stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                          stderr=subprocess.DEVNULL, env=git_env()) as blobs:
        try:
            for name, (mode, oid) in sorted(entries.items()):
                blobs.stdin.write(oid + b"\n")
                blobs.stdin.flush()
                fields = blobs.stdout.readline().split()
                if len(fields) != 3 or fields[:2] != [oid, b"blob"]:
                    raise PinError(f"cannot read pinned blob for {os.fsdecode(name)!r}")
                size = int(fields[2])
                record_header(digest, b"L" if mode == b"120000" else b"F", name, size)
                add_bytes(digest, blobs.stdout, size)
                if blobs.stdout.read(1) != b"\n":
                    raise PinError("incomplete Git blob")
        finally:
            blobs.stdin.close()
            # On a read/validation error, do not wait on unread pipe data.
            blobs.stdout.close()
        if blobs.wait():
            raise PinError("cannot read pinned Git tree")
    return digest.hexdigest()
