#!/usr/bin/env python3
"""Independent full-loader inventory for the lane 10 Python admission check.

Reuses the stage M source inventory (`python_frontend`, `flow`) but enumerates every Python file the
product loader listed in a run's typed reader output (the `base` arm of `compare.py run`), not the frozen
sample or frame. Each candidate store is joined to the parent graph's Def/Use nodes by owner, access, path,
line and exact UTF-8 byte span; routes, occupancy and the three limits are then recomputed in DESIGN
sections 4 and 5 order. The product's survivor decision is never read. RD availability is estimated from the
graph (def cap, line cap, one-line owner); a product-internal early fallback cap cannot be seen and surfaces
as a reconciliation residual in `admission.py`.
"""

import argparse
import ast
import json
import sys
from collections import Counter, defaultdict, deque
from hashlib import sha256
from pathlib import Path

import flow
from flow import INDEX, MAX_COUNTER
from python_frontend import PythonFile

SCHEMA = "element_recall_independent/1"
LIMITS = (flow.RAW_LIMIT, flow.READ_LIMIT, flow.OWNER_LIMIT)
RD_MAX_DEFS, RD_MAX_LINES = 2048, 4096
ROUTE_BUDGET = 5000


def dump(row):
    return json.dumps(row, sort_keys=True, separators=(",", ":"))


def head(graph_path):
    """File hashes and skips: the reader writes them before the first node."""
    files, skips = {}, []
    with open(graph_path, encoding="utf-8") as stream:
        for line in stream:
            row = json.loads(line)
            if row["kind"] == "file":
                files[row["key"]] = row["sha256"]
            elif row["kind"] == "skip":
                skips.append(row)
            elif row["kind"] in ("node", "edge"):
                break
    return files, skips


def node_key(row):
    path = (row["path"]["base"], *row["path"]["fields"])
    return (row["file"], row["owner"], row["owner_start_line"], row["line"], row["access"], path,
            row["start_byte"], row["end_byte"])


class Parent:
    """Def/Use nodes and data-flow edges of the parent graph, for the owners that hold a candidate."""

    def __init__(self, graph_path, wanted):
        self.nodes, self.succ, self.pred = set(), defaultdict(list), defaultdict(list)
        self.where, self.at_line = defaultdict(list), defaultdict(list)
        self.defs, self.lines = defaultdict(set), defaultdict(set)
        with open(graph_path, encoding="utf-8") as stream:
            for text in stream:
                if '"type":"variable"' not in text:
                    continue
                row = json.loads(text)
                if row["kind"] == "node" and row["type"] == "variable":
                    self.add(row, wanted)
                elif row["kind"] == "edge" and row["edge"]["variant"] == "data_flow":
                    a, b = row["from"], row["to"]
                    if a.get("type") == b.get("type") == "variable" and (
                            (a["file"], a["owner"], a["owner_start_line"])
                            == (b["file"], b["owner"], b["owner_start_line"])
                            and (a["file"], a["owner"], a["owner_start_line"]) in wanted):
                        self.succ[node_key(a)].append(node_key(b))
                        self.pred[node_key(b)].append(node_key(a))

    def add(self, row, wanted):
        owner = (row["file"], row["owner"], row["owner_start_line"])
        if owner not in wanted:
            return
        key = node_key(row)
        self.nodes.add(key)
        _, _, _, line, access, path, start, end = key
        self.where[(owner, access, line, path)].append((start, end))
        self.at_line[(owner, access, line)].append(key)
        self.lines[owner].add(line)
        if access == "def":
            self.defs[owner].add((line, path))

    def reaches(self, w_node, r_node, owner, lines, root):
        """Whether a value feeding W's statement already reaches R by other data-flow edges.
        Uses of the container's own root are not the stored value and seed nothing."""
        seeds = {pred for line in lines for use in self.at_line.get((owner, "use", line), ())
                 if use[5][0] != root for pred in self.pred.get(use, ())}
        seen, todo = set(seeds), deque(seeds)
        while todo and len(seen) <= ROUTE_BUDGET:
            for nxt in self.succ.get(todo.popleft(), ()):
                if nxt == r_node:
                    return True
                if nxt != w_node and nxt not in seen:
                    seen.add(nxt)
                    todo.append(nxt)
        return False


def join_verdict(parent, owner, access, point):
    """`None` for a unique full node at the exact bytes, else the refusal."""
    path = point.location_key(access)[2]
    line = point.line
    key = (owner[0], owner[1], owner[2], line, access, path, *point.span)
    if key in parent.nodes:
        return None
    others = parent.where.get((owner, access, line, path), ())
    if not others:
        return "join:missing"
    inside = any(point.span[0] <= s and e <= point.span[1] for s, e in others)
    return "join:truncated" if inside else "join:mismatched"


def storage_key(owner, access, point):
    return (*owner, point.line, access, point.location_key(access)[2], *point.span)


class Candidate:
    """One owner with at least one indexed-store endpoint in the M inventory."""

    def __init__(self, file, key, node, span, graph, entry, ambiguous):
        self.file, self.key, self.node, self.span, self.graph, self.entry = (
            file, key, node, span, graph, entry)
        self.name = node.name
        self.ambiguous = ambiguous
        self.owner = (file, node.name, node.lineno)


def rd_exit(parent, cand):
    if len(parent.defs.get(cand.owner, ())) > RD_MAX_DEFS:
        return "def_cap"
    if len(parent.lines.get(cand.owner, ())) > RD_MAX_LINES:
        return "line_cap"
    return "no_cfg" if cand.node.end_lineno == cand.node.lineno else "available"


def predict(cand, parent, limits=LIMITS):
    """Owner row and prediction rows: RD, raw cap, join, route, occupancy, dedupe, fan-in, owner cap."""
    raw_limit, read_limit, owner_limit = limits
    graph, owner = cand.graph, cand.owner
    for access, points in (("def", graph.writes), ("use", graph.reads)):
        for point in points.values():
            verdict = "join:ambiguousowner" if cand.ambiguous else join_verdict(
                parent, owner, access, point)
            if verdict:
                point.flags.add(verdict)
    graph.collisions()
    writes = sorted(graph.writes.values(), key=lambda p: p.span)
    reads = sorted(graph.reads.values(), key=lambda p: p.span)
    clean = [w for w in writes if not w.flags]
    buckets, spelling = defaultdict(list), defaultdict(list)
    for read in reads:
        spelling[read.static_path].append(read)
        if not read.flags:
            buckets[read.binding_path].append(read)
    raw, _ = flow.work((w.binding_path for w in clean), buckets)
    rd = rd_exit(parent, cand)
    row = dict(rd=rd, rd_source="graph_caps", raw_pairs=None if rd != "available" else min(raw, MAX_COUNTER),
               raw_overflow=int(raw > MAX_COUNTER), raw_capped=int(rd == "available" and raw > raw_limit),
               reads_over_fanin=0, omitted_fanin_pairs=0, over_edge_cap=0, omitted_cap_pairs=0)
    pairs = []

    def emit(w, r, outcome, reason="", **more):
        pairs.append(dict(w=list(w.span), r=list(r.span), role=r.role, binding=w.binding,
                          w_path=list(w.path), r_path=list(r.path), outcome=outcome, reason=reason, **more))

    for w in writes:
        for r in flow.matching(w.static_path, spelling):
            flags = sorted(w.flags | r.flags)
            if rd != "available":
                emit(w, r, "refused:rd:" + rd, "rd")
            elif flags:
                joins = [f for f in flags if f.startswith("join:")]
                emit(w, r, "refused:" + (joins or flags)[0], "endpoint", flags=flags)
    survivors = []
    if rd == "available" and 0 < raw <= raw_limit:
        reach = graph.reachable(cand.entry)
        for w, r, reason, _, proof in flow.pair_rows(graph, clean, buckets, reach, True):
            if reason != "surviving_route":
                emit(w, r, "refused:route:" + reason, reason)
                continue
            wk, rk = storage_key(owner, "def", w), storage_key(owner, "use", r)
            if rk in parent.succ.get(wk, ()):
                emit(w, r, "refused:occupied", "occupied")
            else:
                survivors.append((w, r, [p["phase"] for p in graph.phase_trace(proof[0], w, r, proof[1])]))
    by_read = defaultdict(set)
    for w, r, _ in survivors:
        by_read[r.span].add(w.span)
    over = {span for span, ws in by_read.items() if len(ws) > read_limit}
    row["reads_over_fanin"] = len(over)
    row["omitted_fanin_pairs"] = sum(len(by_read[s]) for s in over)
    rest = [s for s in survivors if s[1].span not in over]
    row["over_edge_cap"] = int(len(rest) > owner_limit)
    row["omitted_cap_pairs"] = len(rest) if row["over_edge_cap"] else 0
    for w, r, route in survivors:
        if r.span in over:
            emit(w, r, "refused:fanin", "fanin")
        elif row["over_edge_cap"]:
            emit(w, r, "refused:ownercap", "ownercap")
        else:
            lines = {w.line, w.statement_line}
            parent_route = parent.reaches(storage_key(owner, "def", w), storage_key(owner, "use", r),
                                          owner, lines, w.path[0])
            emit(w, r, "predicted", route=route, occupied=False, parent_route=parent_route)
    row["attrition"] = dict(admitted=len(pairs), routed=len(survivors), after_fanin=len(rest),
                            published=0 if row["over_edge_cap"] else len(rest))
    endpoint_flags = Counter(f for p in writes + reads for f in p.flags)
    row.update(writes=len(writes), reads=len(reads), flags=dict(endpoint_flags),
               overlapping_flags=sum(len(p.flags) > 1 for p in writes + reads),
               joined_writes=sum(not any(f.startswith("join:") for f in w.flags) for w in writes),
               joined_reads=sum(not any(f.startswith("join:") for f in r.flags) for r in reads))
    return row, pairs, writes, reads


def owners_of(parsed):
    """Function owners (the product scans function definitions), with the product's name@line ambiguity."""
    count = Counter()
    found = []
    for key, nodes in parsed.owners.items():
        for node in nodes:
            if not isinstance(node, ast.Lambda):
                count[(node.name, node.lineno)] += 1
                found.append((key, node))
    return [(key, node, count[(node.name, node.lineno)] > 1) for key, node in found]


def scan_corpus(corpus, run_dir):
    """Stage one: parse each listed Python file and keep the owners that hold an indexed store."""
    manifest = json.loads((run_dir / "manifest.json").read_text())
    if manifest.get("complete") is not True or (run_dir / "FAILED").exists():
        raise ValueError(f"{run_dir}: incomplete run")
    files, skips = head(run_dir / "graph.jsonl")
    repo = Path(manifest["repo"])
    records, found = [], []
    for skip in sorted(skips, key=lambda s: s["path"]):
        if skip["path"].endswith(".py"):
            records.append(dict(kind="file", corpus=corpus, file=skip["path"], status="skipped",
                                reason=skip["reason"]))
    for name in sorted(k for k in files if k.endswith(".py")):
        data = (repo / name).read_bytes()
        digest = sha256(data).hexdigest()
        if digest != files[name]:
            raise ValueError(f"{corpus}/{name}: source hash disagrees with the loader manifest")
        try:
            parsed = PythonFile(data)
        except (SyntaxError, ValueError, UnicodeError, RecursionError):
            records.append(dict(kind="file", corpus=corpus, file=name, sha256=digest, status="parse_error"))
            continue
        total = 0
        for key, node, ambiguous in owners_of(parsed):
            total += 1
            graph, entry = parsed.compile(node)
            if graph.writes:
                span = (parsed.starts[node.lineno - 1] + node.col_offset,
                        parsed.starts[node.end_lineno - 1] + node.end_col_offset)
                found.append((digest, Candidate(name, key, node, span, graph, entry, ambiguous)))
        records.append(dict(kind="file", corpus=corpus, file=name, sha256=digest, status="parsed",
                            owners=total))
    return manifest, records, found


def corpus_records(corpus, run_dir, limits=LIMITS):
    manifest, records, found = scan_corpus(corpus, run_dir)
    parent = Parent(run_dir / "graph.jsonl", {c.owner for _, c in found})
    owners = pairs = 0
    out = [dict(kind="run", schema=SCHEMA, corpus=corpus, repo=manifest["repo"],
                run_id=manifest["run_id"], source_digest=manifest["source"]["digest"],
                version=manifest["version"], parser="python-ast", limits=list(limits))]
    out.extend(records)
    for digest, cand in found:
        row, preds, writes, reads = predict(cand, parent, limits)
        base = dict(corpus=corpus, file=cand.file, owner=cand.key, name=cand.name,
                    line=cand.node.lineno, span=list(cand.span), parser="python-ast")
        out.append(dict(kind="owner", source_sha256=digest, **base, **row))
        for w in writes:
            out.append(dict(kind="write", **{k: base[k] for k in ("corpus", "file", "owner")},
                            span=list(w.span), line=w.line, path=list(w.path), binding=w.binding,
                            flags=sorted(w.flags)))
        for r in reads:
            out.append(dict(kind="node", **{k: base[k] for k in ("corpus", "file", "owner")},
                            span=list(r.span), line=r.line, path=list(r.path), binding=r.binding,
                            role=r.role, flags=sorted(r.flags)))
        for p in preds:
            out.append(dict(kind="prediction", **{k: base[k] for k in ("corpus", "file", "owner")}, **p))
        owners += 1
        pairs += len(preds)
    out.append(dict(kind="end", corpus=corpus, owners=owners, predictions=pairs,
                    files=sum(r["kind"] == "file" for r in records)))
    return out


def main(argv=None):
    cli = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    cli.add_argument("--runs", type=Path, required=True)
    cli.add_argument("--out", type=Path, required=True)
    cli.add_argument("--corpora", nargs="+")
    args = cli.parse_args(argv)
    corpora = args.corpora or sorted(p.name for p in args.runs.iterdir()
                                     if (p / "base" / "manifest.json").is_file())
    if not corpora:
        cli.error("no corpus with a base run under --runs")
    rows = []
    for corpus in corpora:
        rows.extend(corpus_records(corpus, args.runs / corpus / "base"))
    args.out.write_text("".join(dump(r) + "\n" for r in rows))
    print(f"loader_inventory: {sum(r['kind'] == 'owner' for r in rows)} owners in {len(corpora)} corpora")
    return 0


if __name__ == "__main__":
    sys.exit(main())
