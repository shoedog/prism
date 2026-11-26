#!/usr/bin/env python3
"""Independent parent/off/on arm comparator for the lane 10 element-recall diagnostics.

`run` captures one arm of one corpus: a cold `PRISM_EDGES_BYTES=1 prism nav --no-cache dfg-stats --repo R
--edges`, cold `dfg-stats` and `call-stats` JSON, and the typed graph reader, all on the same unchanged
source tree, and writes a manifest binding binary, reader, source tree, version and options. A partial run
(any nonzero exit, truncated stream, version mismatch, a trace record with the trace off) is refused even when
every exit status was zero. `zero` compares the `base`, `off` and `on` runs of each corpus: byte-mode DataFlow
bags, typed input/node/edge bags, DFG stats and call-stats must be equal (an empty allowance for stats
deltas). The `on` arm's stderr trace records are stored apart and never enter a comparison.
"""

import argparse
import collections
import hashlib
import json
import os
import re
import subprocess
import sys
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


def read_jsonl(path, what):
    rows = []
    for n, line in enumerate(Path(path).read_text(encoding="utf-8").splitlines(), 1):
        try:
            row = json.loads(line)
        except ValueError:
            raise Refused(f"{what} line {n} is not JSON")
        if not isinstance(row, dict):
            raise Refused(f"{what} line {n} is not an object")
        rows.append(row)
    return rows


def load_edges(path):
    """Byte-mode DataFlow bag: canonical record -> multiplicity."""
    rows = read_jsonl(path, "edges")
    if not rows:
        raise Refused("empty byte-mode edge dump")
    for row in rows:
        if not isinstance(row.get("from"), dict) or not isinstance(row.get("to"), dict):
            raise Refused("byte-mode edge without endpoints")
    return collections.Counter(canon(r) for r in rows), len(rows)


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


def load_graph(path):
    """Typed reader output as bags (inputs, nodes, edges) after structural completeness checks."""
    rows = read_jsonl(path, "graph")
    if len(rows) < 2 or rows[0].get("kind") != "meta" or rows[-1].get("kind") != "summary":
        raise Refused("graph stream lacks its meta head or summary tail")
    meta, summary = rows[0], rows[-1]
    if meta.get("schema") != READER_SCHEMA:
        raise Refused(f"unexpected reader schema {meta.get('schema')!r}")
    inputs, nodes, edges = collections.Counter(), {}, collections.Counter()
    degree = collections.Counter()
    by_variant = collections.Counter()
    for row in rows[1:-1]:
        kind = row.get("kind")
        if kind in ("file", "manifest", "skip"):
            inputs[canon(row)] += 1
        elif kind == "node":
            key = canon({k: v for k, v in row.items() if k not in ("kind", "isolated")})
            if key in nodes:
                raise Refused("two graph nodes share one full identity")
            nodes[key] = bool(row.get("isolated"))
        elif kind == "edge":
            ends = [canon(row.get(side)) for side in ("from", "to")]
            edges[canon({k: v for k, v in row.items() if k != "kind"})] += 1
            degree.update(ends)
            by_variant[(row.get("edge") or {}).get("variant")] += 1
        else:
            raise Refused(f"unknown graph record kind {kind!r}")
    for row in rows[1:-1]:
        if row["kind"] == "edge" and any(canon(row[s]) not in nodes for s in ("from", "to")):
            raise Refused("graph edge endpoint is not a node of the stream")
    isolated = sum(nodes.values())
    if any(alone != (degree[key] == 0) for key, alone in nodes.items()):
        raise Refused("isolated flag disagrees with the edges")
    if (summary.get("nodes"), summary.get("edges"), summary.get("isolated_nodes")) != (
            len(nodes), sum(edges.values()), isolated) or summary.get("edges_by_variant") != dict(by_variant):
        raise Refused("graph summary disagrees with its records")
    counts = dict(nodes=len(nodes), edges=sum(edges.values()), isolated=isolated)
    files = {json.loads(k)["key"]: json.loads(k)["sha256"] for k in inputs if json.loads(k)["kind"] == "file"}
    return dict(meta=meta, inputs=inputs, nodes=collections.Counter(nodes), edges=edges,
                counts=counts, files=files)


def split_trace(err_path, trace_path, tracing):
    """Move trace records out of stderr (kept in `trace_path` when given, else dropped). A record with the
    trace off, or a malformed one, is refused."""
    keep, trace = [], []
    for line in Path(err_path).read_text(encoding="utf-8", errors="replace").splitlines():
        if not line.startswith(TRACE_PREFIX):
            keep.append(line)
            continue
        try:
            record = json.loads(line[len(TRACE_PREFIX):])
        except ValueError:
            record = None
        if not isinstance(record, dict) or "kind" not in record:
            raise Refused("malformed trace record")
        trace.append(line)
    if trace and not tracing:
        raise Refused("trace records with the trace off")
    Path(err_path).write_text("".join(k + "\n" for k in keep), encoding="utf-8")
    if trace_path:
        Path(trace_path).write_text("".join(t + "\n" for t in trace), encoding="utf-8")


def spawn(argv, env, out_path, err_path):
    with open(out_path, "wb") as out, open(err_path, "wb") as err:
        code = subprocess.run(argv, env=env, stdout=out, stderr=err, stdin=subprocess.DEVNULL).returncode
    if code != 0:
        raise Refused(f"{Path(argv[0]).name} exited {code}: {' '.join(argv[1:])}")


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
    _, n_edges = load_edges(out / "edges.jsonl")
    dfg = load_stats(out / "dfg-stats.json", "dfg-stats")
    load_stats(out / "call-stats.json", "call-stats")
    if sum(dfg.get(k, 0) for k in LABEL_COUNTERS) != n_edges:
        raise Refused("byte-mode edge count disagrees with the dfg-stats label counters")
    graph = load_graph(out / "graph.jsonl")
    if graph["meta"].get("prism_version") != semver:
        raise Refused("reader and binary report different versions")
    for key, digest in graph["files"].items():
        if files.get(key) != digest:
            raise Refused(f"reader file hash for {key} disagrees with the source tree")
    after, _ = tree_digest(repo)
    if after != before:
        raise Refused("source tree changed during the run")
    return dict(
        schema=SCHEMA, run_id=uuid.uuid4().hex, corpus=a.corpus, repo=str(repo), trace=a.trace,
        expect_version=want, version=dict(semver=semver, identity=identity, dirty=dirty),
        binary=dict(path=str(binary), sha256=sha256_file(binary)),
        reader=dict(path=str(reader), sha256=sha256_file(reader), schema=graph["meta"]["schema"]),
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


def load_run(directory, corpus, arm):
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
    graph = load_graph(d / "graph.jsonl")
    bytes_bag, n = load_edges(d / "edges.jsonl")
    if dict(graph["counts"], edge_lines=n) != manifest["counts"]:
        raise Refused(f"{d}: re-read counts disagree with the manifest")
    return dict(manifest=manifest, graph=graph, bytes=bytes_bag,
                dfg=load_stats(d / "dfg-stats.json", "dfg-stats"),
                calls=load_stats(d / "call-stats.json", "call-stats"))


def bag_diff(name, left, right):
    only_l, only_r = left - right, right - left
    if not only_l and not only_r:
        return []
    sample = [f"-{c}x {k[:160]}" for k, c in list(only_l.items())[:2]] + [
        f"+{c}x {k[:160]}" for k, c in list(only_r.items())[:2]]
    return [f"{name}: {sum(only_l.values())} only in base arm, {sum(only_r.values())} only in other; " + "; ".join(sample)]


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


def zero_corpus(runs_dir, corpus):
    runs = {arm: load_run(Path(runs_dir) / corpus / arm, corpus, arm) for arm in TRACE_BY_ARM}
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
    for arm in ("off", "on"):
        problems += [f"base vs {arm}: {p}" for p in compare_runs(runs["base"], runs[arm])]
    return problems


def cmd_zero(a):
    failed = 0
    for corpus in a.corpora:
        try:
            problems = zero_corpus(a.runs, corpus)
        except (Refused, OSError, KeyError, ValueError) as e:
            problems = [f"refused: {e}"]
        print(f"{corpus}: {'FAIL' if problems else 'PASS'}")
        for p in problems:
            print(f"  {p}", file=sys.stderr)
        failed += bool(problems)
    print(f"zero: {'FAIL' if failed else 'PASS'} ({len(a.corpora)} corpora, {failed} failed)")
    return 1 if failed else 0


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
    z.set_defaults(fn=cmd_zero)
    args = p.parse_args(argv)
    return args.fn(args)


if __name__ == "__main__":
    sys.exit(main())
