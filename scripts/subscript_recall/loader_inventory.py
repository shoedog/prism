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
import bisect
import json
import sys
from collections import Counter, defaultdict, deque
from hashlib import sha256
from pathlib import Path

import flow
from flow import INDEX, MAX_COUNTER
from python_frontend import CALLABLES, COMPREHENSIONS, PythonFile, walk

SCHEMA = "element_recall_independent/1"
# The six corpora of the I2 measurement; a Go or JavaScript corpus holds no Python owner and gets zero rows.
DEFAULT_CORPORA = ("P1", "P2", "X", "G1", "G2", "R")
LIMITS = (flow.RAW_LIMIT, flow.READ_LIMIT, flow.OWNER_LIMIT)
RD_MAX_DEFS, RD_MAX_LINES = 2048, 4096
ROUTE_BUDGET = 5000
# Flags that only say the endpoint is not the unique node at its key (not a syntax or binding refusal).
JOIN_FLAGS_ONLY = frozenset(("join:missing", "join:truncated", "join:mismatched", "join:ambiguousowner",
                             "same_line_collision"))


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

    def reaches(self, w_node, r_node, owner, value):
        """Whether the value W stores already reaches R by other data-flow edges.

        `True`/`False` when the search finished; `None` (unknown) when it ran out of budget, when W's
        right-hand side is not known, or when a name it reads has no Use node in the graph. The walk is the
        production one (`dfg_forward_reachable`): data-flow edges, plus a Use feeds the Defs on its own line
        of the same owner (`d = {"old": t}; d["new"] = t; return d` carries `t` through the same-line Def of
        `d`). W itself is never entered: it is the route being asked about.
        It is seeded from the Uses inside W's right-hand side (`value`, see `store_values`) and from no other
        Use on W's lines: an index (`d[t] = u`) or the container is not the stored value."""
        if value is None:
            return None
        start, end, first, last, names = value
        uses = [use for line in range(first, last + 1) for use in self.at_line.get((owner, "use", line), ())
                if start <= use[6] and use[7] <= end]
        if any(not any(use[5][0] == name for use in uses) for name in names):
            return None
        seeds = {pred for use in uses for pred in self.pred.get(use, ())}
        seen, todo = set(seeds), deque(seeds)
        while todo:
            if len(seen) > ROUTE_BUDGET:
                return None
            node = todo.popleft()
            nexts = list(self.succ.get(node, ()))
            if node[4] == "use":
                nexts += self.at_line.get((node[:3], "def", node[3]), ())
            for nxt in nexts:
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

    def __init__(self, file, key, node, ident, graph, entry, ambiguous, values=None):
        self.file, self.key, self.node, self.graph, self.entry = file, key, node, graph, entry
        self.values = values or {}
        self.ident, self.span = ident, ident.span
        self.name = node.name
        # An owner whose product start line cannot be read off the source joins nothing.
        self.ambiguous = ambiguous or ident.line is None
        # The graph keys an owner by the line the product starts it on: the first decorator's `@` line.
        self.owner = (file, node.name, ident.line)


def statement_lines(body):
    """Distinct start lines of the statements the product's CFG sees in a function body.

    The product's `statements_in_function` lists the body's statements and those nested in compound
    statements (not the `def` line, not nested definitions' bodies); one line leaves the owner without CFG
    edges, whatever span the `def` header and its decorators cover."""
    lines = set()
    for node in body:
        lines.add(node.lineno)
        if isinstance(node, ast.Try) or type(node).__name__ == "TryStar":
            lines.update(statement_lines(node.body + node.orelse + node.finalbody))
            for handler in node.handlers:
                lines.update(statement_lines(handler.body))
        elif type(node).__name__ == "Match":
            for case in node.cases:
                lines.update(statement_lines(case.body))
        elif isinstance(node, (ast.If, ast.For, ast.AsyncFor, ast.While, ast.With, ast.AsyncWith)):
            lines.update(statement_lines(node.body + getattr(node, "orelse", [])))
    return lines


def value_names(value):
    """Roots of the names a store's value reads in its own scope: not a called bare name (a call, not a
    variable read), nothing inside a lambda, and of a comprehension only the leftmost iterable."""
    names, todo = set(), [value]
    while todo:
        node = todo.pop()
        if isinstance(node, ast.Lambda):
            continue
        if isinstance(node, COMPREHENSIONS):
            todo.append(node.generators[0].iter)
            continue
        if isinstance(node, ast.Call) and isinstance(node.func, ast.Name):
            todo.extend(node.args + [kw.value for kw in node.keywords])
            continue
        if isinstance(node, ast.Name) and isinstance(node.ctx, ast.Load):
            names.add(node.id)
        todo.extend(ast.iter_child_nodes(node))
    return names


def store_values(parsed, owner):
    """`target span -> (value start, end, first line, last line, names read)` for the owner's own plain
    stores (`t[k] = v`, `t[k] += v`, `t[k] = u[j] = v`): the right-hand side whose Uses are the stored value.
    A target inside an unpacking, or of a statement with no value, has none (the route is then unknown)."""
    starts, out = parsed.starts, {}
    for node in (n for stmt in owner.body for n in walk(stmt, CALLABLES + (ast.ClassDef,))):
        if not isinstance(node, (ast.Assign, ast.AnnAssign, ast.AugAssign)):
            continue
        if node.value is None:
            continue
        value = node.value
        span = (starts[value.lineno - 1] + value.col_offset, starts[value.end_lineno - 1] + value.end_col_offset)
        info = (*span, value.lineno, value.end_lineno, frozenset(value_names(value)))
        for target in (node.targets if isinstance(node, ast.Assign) else [node.target]):
            if isinstance(target, ast.Subscript):
                out[(starts[target.lineno - 1] + target.col_offset,
                     starts[target.end_lineno - 1] + target.end_col_offset)] = info
    return out


def rd_exit(parent, cand):
    if len(parent.defs.get(cand.owner, ())) > RD_MAX_DEFS:
        return "def_cap"
    if len(parent.lines.get(cand.owner, ())) > RD_MAX_LINES:
        return "line_cap"
    return "no_cfg" if len(statement_lines(cand.node.body)) < 2 else "available"


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
    # The typed reader shows the graph after first-wins materialization; the product counts raw bucket
    # rows before it, so an endpoint refused only by a join (a collapsed key) still counts toward its P.
    # The count over those endpoints too is the most that P can be; the clean count is the least.
    loose = defaultdict(list)
    for read in reads:
        if not read.flags - JOIN_FLAGS_ONLY:
            loose[read.binding_path].append(read)
    most, _ = flow.work((w.binding_path for w in writes if not w.flags - JOIN_FLAGS_ONLY), loose)
    rd = rd_exit(parent, cand)
    row = dict(rd=rd, rd_source="graph_caps", raw_pairs=None if rd != "available" else min(raw, MAX_COUNTER),
               raw_pairs_max=None if rd != "available" else min(max(raw, most), MAX_COUNTER),
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
            parent_route = parent.reaches(storage_key(owner, "def", w), storage_key(owner, "use", r),
                                          owner, cand.values.get(w.span))
            emit(w, r, "predicted", route=route, occupied=False, parent_route=parent_route)
    row["attrition"] = dict(admitted=len(pairs), routed=len(survivors), after_fanin=len(rest),
                            published=0 if row["over_edge_cap"] else len(rest))
    endpoint_flags = Counter(f for p in writes + reads for f in p.flags)
    row.update(writes=len(writes), reads=len(reads), flags=dict(endpoint_flags),
               overlapping_flags=sum(len(p.flags) > 1 for p in writes + reads),
               joined_writes=sum(not any(f.startswith("join:") for f in w.flags) for w in writes),
               joined_reads=sum(not any(f.startswith("join:") for f in r.flags) for r in reads))
    return row, pairs, writes, reads


def trailing_comments(source, end):
    """End of the comment-only text that directly follows a node ending at `end` (else `end`).

    tree-sitter closes a function after a trailing comment (`return x  # note`), `ast` before it; the join
    accepts a product end between the two and no other difference."""
    at, last = end, end
    while at < len(source):
        byte = source[at:at + 1]
        if byte in (b" ", b"\t", b"\n", b"\r", b"\f"):
            at += 1
        elif byte == b"#":
            line_end = source.find(b"\n", at)
            at = last = len(source) if line_end < 0 else line_end
            if source[last - 1:last] == b"\r":
                last -= 1
        else:
            break
    return last


class Ident:
    """Where one function owner lives for each side: the def (census key) and the product's own start.

    The product starts a decorated function at its first decorator's `@` (tree-sitter's
    `decorated_definition`) and keys its graph nodes and trace records by that line, while the frame and
    census key it by the `def` line. `line`/`span` are the product's; `def_line`/`def_span` the census's.
    `line` is None when the `@` cannot be found in the source, so the identity is not established."""

    def __init__(self, parsed, node):
        starts, source = parsed.starts, parsed.source
        end = starts[node.end_lineno - 1] + node.end_col_offset
        self.def_line = node.lineno
        self.def_span = (starts[node.lineno - 1] + node.col_offset, end)
        self.decorated = bool(node.decorator_list)
        self.line, self.span = self.def_line, self.def_span
        self.end_max = trailing_comments(source, end)
        if self.decorated:
            first = node.decorator_list[0]
            at = starts[first.lineno - 1] + first.col_offset
            while at > 0 and source[at - 1:at] in (b" ", b"\t", b"\n", b"\r", b"\f", b"("):
                at -= 1
            if source[at - 1:at] == b"@":
                self.span = (at - 1, end)
                self.line = bisect.bisect_right(starts, at - 1)
            else:
                self.line = self.span = None


def owners_of(parsed):
    """Function owners (the product scans function definitions), with the product's name@line ambiguity.

    Ambiguity is counted on the product's identity (name and start line), the key its graph uses."""
    count = Counter()
    found = []
    for key, nodes in parsed.owners.items():
        for node in nodes:
            if not isinstance(node, ast.Lambda):
                ident = Ident(parsed, node)
                count[(node.name, ident.line)] += 1
                found.append((key, node, ident))
    return [(key, node, ident, count[(node.name, ident.line)] > 1) for key, node, ident in found]


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
        functions = []
        for key, node, ident, ambiguous in owners_of(parsed):
            # Every function owner is listed, candidate or not: the identity table `admission.py` joins on.
            functions.append([key, node.name, ident.def_line, ident.line, list(ident.span or ()),
                              list(ident.def_span), int(ambiguous), ident.end_max])
            graph, entry = parsed.compile(node)
            if graph.writes:
                found.append((digest, Candidate(name, key, node, ident, graph, entry, ambiguous,
                                                   store_values(parsed, node))))
        records.append(dict(kind="file", corpus=corpus, file=name, sha256=digest, status="parsed",
                            owners=len(functions), functions=functions))
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
        ident = cand.ident
        base = dict(corpus=corpus, file=cand.file, owner=cand.key, name=cand.name,
                    line=ident.def_line, span=list(ident.def_span), owner_line=ident.line,
                    owner_span=list(ident.span or ()), end_max=ident.end_max, decorated=ident.decorated, parser="python-ast")
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
    cli.add_argument("--corpora", nargs="+", default=list(DEFAULT_CORPORA),
                     help="corpora with a base run under --runs (default: all six)")
    args = cli.parse_args(argv)
    corpora = args.corpora
    absent = [c for c in corpora if not (args.runs / c / "base" / "manifest.json").is_file()]
    if absent:
        cli.error(f"no base run under --runs for {', '.join(absent)}")
    rows = []
    for corpus in corpora:
        rows.extend(corpus_records(corpus, args.runs / corpus / "base"))
    args.out.write_text("".join(dump(r) + "\n" for r in rows))
    print(f"loader_inventory: {sum(r['kind'] == 'owner' for r in rows)} owners in {len(corpora)} corpora")
    return 0


if __name__ == "__main__":
    sys.exit(main())
