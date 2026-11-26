#!/usr/bin/env python3
"""Independent parent/off/on arm comparator for the lane 10 element-recall diagnostics.

`run` captures one arm of one corpus: a cold `PRISM_EDGES_BYTES=1 prism nav --no-cache dfg-stats --repo R
--edges`, cold `dfg-stats` and `call-stats` JSON, and the typed graph reader, all on the same unchanged
source tree, and writes a manifest binding binary, reader, source tree, version and options. A partial run
(any nonzero exit, truncated stream, version mismatch, a trace record with the trace off) is refused even when
every exit status was zero. `zero` compares the `base`, `off` and `on` runs of each corpus, after binding the base arm to an expected base revision
and a binary and reader of its own (`--expect-base`, `--expect-head`; `--allow-same-binary` is for smoke runs only): byte-mode DataFlow
bags, typed input/node/edge bags, DFG stats and call-stats must be equal (an empty allowance for stats
deltas). The `on` arm's stderr trace records are stored apart and never enter a comparison.
"""

import argparse
import collections
import hashlib
import heapq
import json
import os
import re
import subprocess
import sys
import tempfile
import uuid
from pathlib import Path

SCHEMA = "element_recall_compare/1"
READER_SCHEMA = "element_recall_graph/1"
TRACE_PREFIX = "element-recall\tv1\t"
STATS_ALLOWANCE = frozenset()  # diagnostic stats deltas allowed between arms: none
TRACE_BY_ARM = {"base": 0, "off": 0, "on": 1}
VERSION_RE = re.compile(r"^slicing (\S+) \(([0-9a-f]{12,40}|unknown)(-dirty)?\)$")
ARTIFACTS = ("edges.jsonl", "dfg-stats.json", "call-stats.json", "graph.jsonl",
             "edges.err", "dfg-stats.err", "call-stats.err", "graph.err", "trace.txt")
LABEL_COUNTERS = ("dfg_label_exact", "dfg_label_nameonly_killed", "dfg_label_nameonly_sameline",
                  "dfg_label_nameonly_cfg_incomplete", "dfg_label_nameonly_alias_unstable",
                  "dfg_label_nameonly_call")


class Refused(Exception):
    pass


def sha256_file(path):
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def canon(value):
    return json.dumps(value, sort_keys=True, separators=(",", ":"))


def tree_digest(repo):
    """Digest of every file under `repo` (the `.git` directory excluded), and the per-file hashes."""
    files = {}
    for dirpath, dirs, names in os.walk(repo):
        dirs[:] = sorted(d for d in dirs if d != ".git")
        for name in sorted(names):
            p = Path(dirpath, name)
            rel = p.relative_to(repo).as_posix()
            files[rel] = "link:" + os.readlink(p) if p.is_symlink() else sha256_file(p)
    h = hashlib.sha256()
    for rel in sorted(files):
        h.update(rel.encode() + b"\0" + files[rel].encode() + b"\n")
    return h.hexdigest(), files


DIGEST_SIZE = 16
SPILL = 1 << 18  # digests held in memory before a sorted run is written out


def digest(text):
    return hashlib.blake2b(text.encode(), digest_size=DIGEST_SIZE).digest()


def _read_run(path):
    with open(path, "rb") as f:
        while True:
            block = f.read(DIGEST_SIZE * 4096)
            if not block:
                return
            for i in range(0, len(block), DIGEST_SIZE):
                yield block[i:i + DIGEST_SIZE]


class Bag:
    """A multiset of canonical records held as 16-byte digests in sorted runs on disk, so a bag of millions of
    records costs one spill buffer of memory. `strings` re-reads the source artifact to recover the text of a
    digest for a failure message."""

    def __init__(self, scratch, strings):
        self.scratch, self.strings, self.total = scratch, strings, 0
        self.paths, self.buf = [], []

    def add(self, text):
        self.buf.append(digest(text))
        self.total += 1
        if len(self.buf) >= SPILL:
            self.spill()

    def spill(self):
        if self.buf:
            self.buf.sort()
            path = self.scratch.path()
            Path(path).write_bytes(b"".join(self.buf))
            self.paths.append(path)
            self.buf = []

    def seal(self):
        self.spill()
        return self

    def runs(self):
        """(digest, multiplicity) in digest order."""
        prev, count = None, 0
        for d in heapq.merge(*(_read_run(p) for p in self.paths)):
            if d == prev:
                count += 1
                continue
            if prev is not None:
                yield prev, count
            prev, count = d, 1
        if prev is not None:
            yield prev, count

    def texts(self, wanted):
        found = {}
        for text in self.strings():
            d = digest(text)
            if d in wanted and d not in found:
                found[d] = text
                if len(found) == len(wanted):
                    break
        return found


class Scratch:
    def __enter__(self):
        self.dir = tempfile.TemporaryDirectory(prefix="compare-bags-")
        self.n = 0
        return self

    def __exit__(self, *exc):
        self.dir.cleanup()

    def path(self):
        self.n += 1
        return os.path.join(self.dir.name, f"run{self.n}")

    def bag(self, strings):
        return Bag(self, strings)


def diff_bags(left, right, keep=2):
    """Counter-subtraction semantics: (excess records only in left, only in right, first `keep` digests of each)."""
    only_l = only_r = 0
    sample_l, sample_r = [], []
    li, ri = left.runs(), right.runs()
    a, b = next(li, None), next(ri, None)
    while a is not None or b is not None:
        if b is None or (a is not None and a[0] < b[0]):
            only_l += a[1]
            sample_l += [a] if len(sample_l) < keep else []
            a = next(li, None)
        elif a is None or b[0] < a[0]:
            only_r += b[1]
            sample_r += [b] if len(sample_r) < keep else []
            b = next(ri, None)
        else:
            if a[1] > b[1]:
                only_l += a[1] - b[1]
                sample_l += [(a[0], a[1] - b[1])] if len(sample_l) < keep else []
            elif b[1] > a[1]:
                only_r += b[1] - a[1]
                sample_r += [(b[0], b[1] - a[1])] if len(sample_r) < keep else []
            a, b = next(li, None), next(ri, None)
    return only_l, only_r, sample_l, sample_r


def iter_rows(path, what):
    """JSON objects of a jsonl file, one line at a time."""
    with open(path, "rb") as f:
        for n, line in enumerate(f, 1):
            try:
                row = json.loads(line)
            except ValueError:
                raise Refused(f"{what} line {n} is not JSON")
            if not isinstance(row, dict):
                raise Refused(f"{what} line {n} is not an object")
            yield row


def _edge_strings(path):
    return (canon(r) for r in iter_rows(path, "edges"))


def load_edges(path, scratch):
    """Byte-mode DataFlow bag: canonical record digests -> multiplicity."""
    bag = scratch.bag(lambda: _edge_strings(path))
    for row in iter_rows(path, "edges"):
        if not isinstance(row.get("from"), dict) or not isinstance(row.get("to"), dict):
            raise Refused("byte-mode edge without endpoints")
        bag.add(canon(row))
    if not bag.total:
        raise Refused("empty byte-mode edge dump")
    return bag.seal(), bag.total


def load_stats(path, what):
    try:
        stats = json.loads(Path(path).read_text(encoding="utf-8"))
    except ValueError:
        raise Refused(f"{what} is not JSON")
    if not isinstance(stats, dict) or not stats:
        raise Refused(f"{what} is empty")
    return stats


def flatten(value, prefix=""):
    if isinstance(value, dict):
        out = {}
        for k, v in value.items():
            out.update(flatten(v, f"{prefix}{k}."))
        return out
    return {prefix[:-1]: value}


NODE_HIDDEN = ("kind", "isolated")
ISOLATED_MARK = "\tisolated"


def _node_key(row):
    return canon({k: v for k, v in row.items() if k not in NODE_HIDDEN})


def graph_text(row):
    """(bag name, canonical text) of a body record, or None for a head/tail record."""
    kind = row.get("kind")
    if kind in ("file", "manifest", "skip"):
        return "inputs", canon(row)
    if kind == "node":
        return "nodes", _node_key(row) + (ISOLATED_MARK if row.get("isolated") else "")
    if kind == "edge":
        return "edges", canon({k: v for k, v in row.items() if k != "kind"})
    return None


def load_graph(path, scratch):
    """Typed reader output as bags (inputs, nodes, edges) after structural completeness checks, in one pass.
    Node identity needs one digest per node (and per touched endpoint); the records themselves are never held."""
    def strings(name):
        return lambda: (t[1] for t in map(graph_text, iter_rows(path, "graph")) if t and t[0] == name)
    bags = {name: scratch.bag(strings(name)) for name in ("inputs", "nodes", "edges")}
    node_ids, isolated_ids, touched, files = set(), set(), set(), {}
    by_variant = collections.Counter()
    rows = iter_rows(path, "graph")
    meta = next(rows, None)
    if meta is None or meta.get("kind") != "meta":
        raise Refused("graph stream lacks its meta head or summary tail")
    if meta.get("schema") != READER_SCHEMA:
        raise Refused(f"unexpected reader schema {meta.get('schema')!r}")

    def body(row):
        kind = row.get("kind")
        if kind in ("file", "manifest", "skip"):
            bags["inputs"].add(canon(row))
            if kind == "file":
                if "key" not in row or "sha256" not in row:
                    raise Refused("graph file record without key or sha256")
                files[row["key"]] = row["sha256"]
        elif kind == "node":
            key = _node_key(row)
            d = digest(key)
            if d in node_ids:
                raise Refused("two graph nodes share one full identity")
            node_ids.add(d)
            alone = bool(row.get("isolated"))
            if alone:
                isolated_ids.add(d)
            bags["nodes"].add(key + (ISOLATED_MARK if alone else ""))
        elif kind == "edge":
            touched.update(digest(canon(row.get(side))) for side in ("from", "to"))
            bags["edges"].add(canon({k: v for k, v in row.items() if k != "kind"}))
            by_variant[(row.get("edge") or {}).get("variant")] += 1
        else:
            raise Refused(f"unknown graph record kind {kind!r}")

    summary = None
    for row in rows:
        if summary is not None:
            body(summary)
        summary = row
    if summary is None or summary.get("kind") != "summary":
        raise Refused("graph stream lacks its meta head or summary tail")
    for bag in bags.values():
        bag.seal()
    if touched - node_ids:
        raise Refused("graph edge endpoint is not a node of the stream")
    if isolated_ids != node_ids - touched:
        raise Refused("isolated flag disagrees with the edges")
    n_edges = bags["edges"].total
    if (summary.get("nodes"), summary.get("edges"), summary.get("isolated_nodes")) != (
            len(node_ids), n_edges, len(isolated_ids)) or summary.get("edges_by_variant") != dict(by_variant):
        raise Refused("graph summary disagrees with its records")
    counts = dict(nodes=len(node_ids), edges=n_edges, isolated=len(isolated_ids))
    return dict(meta=meta, inputs=bags["inputs"], nodes=bags["nodes"], edges=bags["edges"],
                counts=counts, files=files)


def split_trace(err_path, trace_path, tracing):
    """Move trace records out of stderr (kept in `trace_path` when given, else dropped). A record with the
    trace off, or a malformed one, is refused."""
    err_path = Path(err_path)
    kept = err_path.with_name(err_path.name + ".tmp")
    try:
        if _split_trace(err_path, kept, trace_path) and not tracing:
            raise Refused("trace records with the trace off")
    except BaseException:
        kept.unlink(missing_ok=True)
        raise
    kept.replace(err_path)


def _split_trace(err_path, kept, trace_path):
    traces = 0
    with open(err_path, encoding="utf-8", errors="replace") as src, open(kept, "w", encoding="utf-8") as keep, (
            open(trace_path, "w", encoding="utf-8") if trace_path else open(os.devnull, "w")) as trace:
        for line in src:
            line = line.rstrip("\r\n")
            if not line.startswith(TRACE_PREFIX):
                keep.write(line + "\n")
                continue
            try:
                record = json.loads(line[len(TRACE_PREFIX):])
            except ValueError:
                record = None
            if not isinstance(record, dict) or "kind" not in record:
                raise Refused("malformed trace record")
            traces += 1
            trace.write(line + "\n")
    return traces


def spawn(argv, env, out_path, err_path):
    with open(out_path, "wb") as out, open(err_path, "wb") as err:
        code = subprocess.run(argv, env=env, stdout=out, stderr=err, stdin=subprocess.DEVNULL).returncode
    if code != 0:
        raise Refused(f"{Path(argv[0]).name} exited {code}: {' '.join(argv[1:])}")


def worktree_of(path):
    """`(top level, HEAD)` of the git worktree that holds `path`; `(None, None)` outside one.

    The reader prints no revision of its own, so its identity is where it was built: the worktree it lives in."""
    where = str(Path(path).resolve().parent)

    def git(*args):
        done = subprocess.run(["git", "-C", where, *args], capture_output=True, text=True, check=False)
        return done.stdout.strip() if done.returncode == 0 else None
    top, head = git("rev-parse", "--show-toplevel"), git("rev-parse", "HEAD")
    return (str(Path(top).resolve()), head) if top and head else (None, None)


def run_arm(a, out):
    binary, reader, repo = (Path(p).resolve() for p in (a.binary, a.reader, a.repo))
    if not (binary.is_file() and reader.is_file() and repo.is_dir()):
        raise Refused("binary, reader or repo is missing")
    line = subprocess.run([str(binary), "--version"], capture_output=True, text=True, check=False)
    m = VERSION_RE.match(line.stdout.strip()) if line.returncode == 0 else None
    if not m:
        raise Refused(f"unreadable prism --version: {line.stdout.strip()!r}")
    semver, identity, dirty = m.group(1), m.group(2), bool(m.group(3))
    want = a.expect_version.lower()
    if not re.fullmatch(r"[0-9a-f]{7,40}", want) or identity == "unknown" or not (
            identity.startswith(want) or want.startswith(identity)):
        raise Refused(f"prism --version identity {identity} does not match --expect-version {want}")
    if dirty and not a.allow_dirty:
        raise Refused("prism binary was built from a dirty tree (pass --allow-dirty to bind it anyway)")
    before, files = tree_digest(repo)
    env = {k: v for k, v in os.environ.items() if not k.startswith("PRISM_")}
    env["PRISM_ELEMENT_RECALL_TRACE"] = str(a.trace)
    edge_env = dict(env, PRISM_EDGES_BYTES="1")
    nav = [str(binary), "nav", "--no-cache"]
    cmds = dict(
        edges=(nav + ["dfg-stats", "--repo", str(repo), "--edges"], edge_env, "edges.jsonl", "edges.err"),
        dfg=(nav + ["dfg-stats", "--repo", str(repo)], env, "dfg-stats.json", "dfg-stats.err"),
        calls=(nav + ["call-stats", "--repo", str(repo)], env, "call-stats.json", "call-stats.err"),
        graph=([str(reader), str(repo)], env, "graph.jsonl", "graph.err"),
    )
    for argv, cenv, name, err in cmds.values():
        spawn(argv, cenv, out / name, out / err)
    # The edges capture is the one designated cold build whose trace records are kept.
    for err in ("edges.err", "dfg-stats.err", "call-stats.err", "graph.err"):
        split_trace(out / err, out / "trace.txt" if err == "edges.err" else None, a.trace == 1)
    with Scratch() as scratch:
        _, n_edges = load_edges(out / "edges.jsonl", scratch)
        dfg = load_stats(out / "dfg-stats.json", "dfg-stats")
        load_stats(out / "call-stats.json", "call-stats")
        if sum(dfg.get(k, 0) for k in LABEL_COUNTERS) != n_edges:
            raise Refused("byte-mode edge count disagrees with the dfg-stats label counters")
        graph = load_graph(out / "graph.jsonl", scratch)
    if graph["meta"].get("prism_version") != semver:
        raise Refused("reader and binary report different versions")
    printed = identity + ("-dirty" if dirty else "")
    if graph["meta"].get("build_identity") != printed:
        raise Refused(f"reader build identity {graph['meta'].get('build_identity')!r} is not its binary's {printed!r} "
                      "(a reader built from other code than the binary cannot stand for it)")
    for key, sha in graph["files"].items():
        if files.get(key) != sha:
            raise Refused(f"reader file hash for {key} disagrees with the source tree")
    after, _ = tree_digest(repo)
    if after != before:
        raise Refused("source tree changed during the run")
    binary_tree, binary_head = worktree_of(binary)
    reader_tree, reader_head = worktree_of(reader)
    return dict(
        binary_worktree=binary_tree, binary_worktree_head=binary_head,
        reader_worktree=reader_tree, reader_worktree_head=reader_head,
        schema=SCHEMA, run_id=uuid.uuid4().hex, corpus=a.corpus, repo=str(repo), trace=a.trace,
        expect_version=want, version=dict(semver=semver, identity=identity, dirty=dirty),
        binary=dict(path=str(binary), sha256=sha256_file(binary)),
        reader=dict(path=str(reader), sha256=sha256_file(reader), schema=graph["meta"]["schema"],
                    build_identity=graph["meta"]["build_identity"]),
        source=dict(digest=before, files=len(files)),
        options=dict(commands={k: v[0] for k, v in cmds.items()},
                     env=dict(PRISM_ELEMENT_RECALL_TRACE=str(a.trace), edges_PRISM_EDGES_BYTES="1")),
        counts=dict(graph["counts"], edge_lines=n_edges),
        artifacts={n: sha256_file(out / n) for n in ARTIFACTS}, complete=True)


def cmd_run(a):
    out = Path(a.out)
    if out.exists() and any(out.iterdir()):
        print(f"compare run: {out} is not empty", file=sys.stderr)
        return 2
    out.mkdir(parents=True, exist_ok=True)
    try:
        manifest = run_arm(a, out)
    except (Refused, OSError, UnicodeDecodeError) as e:
        (out / "FAILED").write_text(f"{e}\n")
        print(f"compare run refused: {e}", file=sys.stderr)
        return 1
    tmp = out / "manifest.json.tmp"
    tmp.write_text(json.dumps(manifest, indent=1, sort_keys=True) + "\n")
    tmp.replace(out / "manifest.json")
    print(f"compare run ok: {out} ({manifest['counts']})")
    return 0


def load_run(directory, corpus, arm, scratch):
    """A complete run whose artifacts match its manifest, re-parsed from the bytes on disk."""
    d = Path(directory)
    try:
        manifest = json.loads((d / "manifest.json").read_text())
    except (OSError, ValueError):
        raise Refused(f"{d}: no readable manifest (partial or unbound run)")
    if (d / "FAILED").exists() or manifest.get("complete") is not True or manifest.get("schema") != SCHEMA:
        raise Refused(f"{d}: incomplete run")
    if manifest.get("corpus") != corpus or manifest.get("trace") != TRACE_BY_ARM[arm]:
        raise Refused(f"{d}: manifest is not the {corpus}/{arm} run")
    for name in ARTIFACTS:
        if not (d / name).is_file() or sha256_file(d / name) != manifest["artifacts"].get(name):
            raise Refused(f"{d}: artifact {name} does not match its manifest")
    if TRACE_BY_ARM[arm] == 0 and (d / "trace.txt").read_text():
        raise Refused(f"{d}: trace records in an arm run with the trace off")
    graph = load_graph(d / "graph.jsonl", scratch)
    bytes_bag, n = load_edges(d / "edges.jsonl", scratch)
    if dict(graph["counts"], edge_lines=n) != manifest["counts"]:
        raise Refused(f"{d}: re-read counts disagree with the manifest")
    return dict(manifest=manifest, graph=graph, bytes=bytes_bag,
                dfg=load_stats(d / "dfg-stats.json", "dfg-stats"),
                calls=load_stats(d / "call-stats.json", "call-stats"))


def bag_diff(name, left, right):
    only_l, only_r, sample_l, sample_r = diff_bags(left, right)
    if not only_l and not only_r:
        return []
    texts_l, texts_r = left.texts({d for d, _ in sample_l}), right.texts({d for d, _ in sample_r})
    sample = [f"-{c}x {texts_l.get(d, '?')[:160]}" for d, c in sample_l] + [
        f"+{c}x {texts_r.get(d, '?')[:160]}" for d, c in sample_r]
    return [f"{name}: {only_l} only in base arm, {only_r} only in other; " + "; ".join(sample)]


def compare_runs(base, other):
    problems = []
    for name, left, right in (("byte DataFlow bag", base["bytes"], other["bytes"]),
                              ("typed input bag", base["graph"]["inputs"], other["graph"]["inputs"]),
                              ("typed node bag", base["graph"]["nodes"], other["graph"]["nodes"]),
                              ("typed edge bag", base["graph"]["edges"], other["graph"]["edges"])):
        problems += bag_diff(name, left, right)
    for name in ("dfg", "calls"):
        a = {k: v for k, v in flatten(base[name]).items() if k not in STATS_ALLOWANCE}
        b = {k: v for k, v in flatten(other[name]).items() if k not in STATS_ALLOWANCE}
        diff = sorted(k for k in a.keys() | b.keys() if a.get(k) != b.get(k))
        if diff:
            problems.append(f"{name}-stats differ: " + ", ".join(f"{k} {a.get(k)}->{b.get(k)}" for k in diff[:6]))
    return problems


REV_RE = re.compile(r"[0-9a-f]{7,40}")


def same_revision(a, b):
    a, b = a.lower(), b.lower()
    return a.startswith(b) or b.startswith(a)


def binding_problems(ms, bind, streams):
    """Whether base is a different executable (and reader) of a different revision than the head arms.

    `bind` holds `expect_base`, `expect_head` (revisions, at least 7 hex digits) and `allow_same_binary`
    (a smoke run whose three arms share one executable). Without the flag, both revisions must be named,
    must differ, must be the ones the arms' `--version` reported, and the base arm's binary and reader must
    be different files (by sha256) from the head arms'. `streams` is each arm's reader meta as re-read from its
    captured stream: the reader's own `build_identity` must be the one its manifest recorded, equal its arm's
    binary identity, and be the expected base (base arm) or expected head (off and on arms), so the two
    typed graphs cannot both come from head code."""
    if bind["allow_same_binary"]:
        return []
    problems = []
    want = {"base": bind.get("expect_base"), "head": bind.get("expect_head")}
    for side, rev in want.items():
        if not rev or not REV_RE.fullmatch(rev.lower()):
            problems.append(f"expected {side} revision missing or too short (--expect-{side}, 7+ hex digits)")
    if not problems and same_revision(want["base"], want["head"]):
        problems.append("expected base and head revisions are the same")
    identity = {arm: ms[arm]["version"]["identity"] for arm in TRACE_BY_ARM}
    if same_revision(identity["base"], identity["off"]) or same_revision(identity["base"], identity["on"]):
        problems.append(f"base arm has the head's version identity {identity['base']}")
    if not problems:
        if not same_revision(identity["base"], want["base"]):
            problems.append(f"base arm identity {identity['base']} is not the expected base {want['base']}")
        for arm in ("off", "on"):
            if not same_revision(identity[arm], want["head"]):
                problems.append(f"{arm} arm identity {identity[arm]} is not the expected head {want['head']}")
    problems += reader_problems(ms, want)
    problems += identity_problems(ms, want, streams)
    for what, pick in (("executable", lambda m: m["binary"]["sha256"]),
                       ("reader executable", lambda m: m["reader"]["sha256"])):
        if pick(ms["base"]) == pick(ms["off"]):
            problems.append(f"base arm used the same {what} as the head arms (sha256 {pick(ms['base'])[:12]})")
    return problems


def identity_problems(ms, want, streams):
    """Each reader's embedded build identity against its arm (see `binding_problems`)."""
    problems = []
    for arm, side in (("base", "base"), ("off", "head"), ("on", "head")):
        built = streams[arm].get("build_identity")
        binary = ms[arm]["version"]["identity"] + ("-dirty" if ms[arm]["version"].get("dirty") else "")
        if not isinstance(built, str) or not VERSION_RE.match(f"slicing 0 ({built})") or built == "unknown":
            problems.append(f"{arm} arm: reader build identity {built!r} not recorded in its stream")
            continue
        if ms[arm]["reader"].get("build_identity") != built:
            problems.append(f"{arm} arm: reader build identity {built} is not the manifest's "
                            f"{ms[arm]['reader'].get('build_identity')!r}")
        if built != binary:
            problems.append(f"{arm} arm reader build identity {built} is not its binary's {binary}")
        if want[side] and REV_RE.fullmatch(want[side].lower()) and not same_revision(
                built.removesuffix("-dirty"), want[side]):
            problems.append(f"{arm} arm reader build identity {built} is not the expected {side} {want[side]}")
    return problems


def reader_problems(ms, want):
    """Each arm's typed reader against the worktree it was built in.

    The reader (`examples/element_recall_graph.rs`) carries the package version, not a revision, so two builds
    of different code can both report it. `run` records the git worktree holding the reader and that
    worktree's HEAD; the reader's path must lie under it, the base arm's HEAD must be the expected base
    revision, the off and on arms' the expected head revision, and the base reader may not come from the
    head arms' worktree HEAD."""
    problems, heads = [], {}
    for arm in TRACE_BY_ARM:
        m = ms[arm]
        tree, head = m.get("reader_worktree"), m.get("reader_worktree_head")
        if not tree or not isinstance(head, str) or not REV_RE.fullmatch(head.lower()):
            problems.append(f"{arm} arm: reader worktree HEAD not recorded (run inside a git worktree)")
        elif not Path(m["reader"]["path"]).is_relative_to(tree):
            problems.append(f"{arm} arm: reader {m['reader']['path']} is not under its worktree {tree}")
        else:
            heads[arm] = head
    if "base" in heads:
        for arm in ("off", "on"):
            if arm in heads and same_revision(heads["base"], heads[arm]):
                problems.append(f"base arm reader has the same worktree HEAD ({heads['base']}) as the {arm} arm's")
    for arm, rev, side in (("base", want["base"], "base"), ("off", want["head"], "head"), ("on", want["head"], "head")):
        if arm in heads and rev and REV_RE.fullmatch(rev.lower()) and not same_revision(heads[arm], rev):
            problems.append(f"{arm} arm reader worktree HEAD {heads[arm]} is not the expected {side} {rev}")
    return problems


def zero_corpus(runs_dir, corpus, bind):
    with Scratch() as scratch:
        return zero_runs(runs_dir, corpus, scratch, bind)


def zero_runs(runs_dir, corpus, scratch, bind):
    runs = {arm: load_run(Path(runs_dir) / corpus / arm, corpus, arm, scratch) for arm in TRACE_BY_ARM}
    ms = {arm: r["manifest"] for arm, r in runs.items()}
    problems = []
    if len({m["run_id"] for m in ms.values()}) != 3:
        problems.append("duplicate run: two arms share one run id")
    if len({m["source"]["digest"] for m in ms.values()}) != 1:
        problems.append("arms ran on different source trees")
    for what, pick in (("binary", lambda m: (m["binary"]["path"], m["binary"]["sha256"])),
                       ("reader", lambda m: (m["reader"]["path"], m["reader"]["sha256"])),
                       ("version", lambda m: m["version"])):
        if pick(ms["off"]) != pick(ms["on"]):
            problems.append(f"off and on arms used different {what}")
    if ms["base"]["reader"]["schema"] != ms["on"]["reader"]["schema"]:
        problems.append("arms used different reader schemas")
    problems += binding_problems(ms, bind, {arm: r["graph"]["meta"] for arm, r in runs.items()})
    for arm in ("off", "on"):
        problems += [f"base vs {arm}: {p}" for p in compare_runs(runs["base"], runs[arm])]
    return problems


def cmd_zero(a):
    bind = dict(expect_base=a.expect_base, expect_head=a.expect_head, allow_same_binary=a.allow_same_binary)
    failed = 0
    for corpus in a.corpora:
        try:
            problems = zero_corpus(a.runs, corpus, bind)
        except (Refused, OSError, KeyError, ValueError) as e:
            problems = [f"refused: {e}"]
        print(f"{corpus}: {'FAIL' if problems else 'PASS'}")
        for p in problems:
            print(f"  {p}", file=sys.stderr)
        failed += bool(problems)
    note = " (smoke: base, off and on may share one executable)" if a.allow_same_binary else ""
    print(f"zero: {'FAIL' if failed else 'PASS'} ({len(a.corpora)} corpora, {failed} failed){note}")
    return 1 if failed else 0


def cmd_verify(a):
    """Re-derive one run's bags from its artifact bytes (what `zero` does per arm) and print its counts."""
    try:
        with Scratch() as scratch:
            run = load_run(a.run, a.corpus, a.arm, scratch)
    except (Refused, OSError, KeyError, ValueError) as e:
        print(f"verify refused: {e}", file=sys.stderr)
        return 1
    print(f"verify ok: {a.run} {run['manifest']['counts']}")
    return 0


def main(argv=None):
    p = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    sub = p.add_subparsers(dest="cmd", required=True)
    r = sub.add_parser("run", help="capture one arm of one corpus")
    for flag in ("binary", "reader", "expect-version", "corpus", "repo", "out"):
        r.add_argument("--" + flag, required=True)
    r.add_argument("--trace", type=int, choices=(0, 1), required=True)
    r.add_argument("--allow-dirty", action="store_true")
    r.set_defaults(fn=cmd_run)
    z = sub.add_parser("zero", help="require base, off and on arms equal per corpus")
    z.add_argument("--runs", required=True)
    z.add_argument("--corpora", nargs="+", required=True)
    z.add_argument("--expect-base", help="revision (7+ hex digits) the base arm's binary must report")
    z.add_argument("--expect-head", help="revision (7+ hex digits) the off and on arms' binary must report")
    z.add_argument("--allow-same-binary", action="store_true",
                   help="smoke runs only: let base, off and on share one executable")
    z.set_defaults(fn=cmd_zero)
    v = sub.add_parser("verify", help="re-verify one run's artifacts and bags against its manifest")
    v.add_argument("--run", required=True)
    v.add_argument("--corpus", required=True)
    v.add_argument("--arm", choices=sorted(TRACE_BY_ARM), required=True)
    v.set_defaults(fn=cmd_verify)
    args = p.parse_args(argv)
    return args.fn(args)


if __name__ == "__main__":
    sys.exit(main())
