#!/usr/bin/env python3
"""Deterministic Appendix A extraction for the bounded entry/call proof.

Usage: python3 extract.py <run-dir> <source-root> <targets.json> <out-dir>
Reads the raw `prism nav` outputs of one Appendix A run and writes four sorted,
canonical JSON extracts plus a decision summary. Read-only over its inputs.
"""
import json, os, re, sys

run, root, targets_path, out = sys.argv[1:5]
targets = json.load(open(targets_path))["targets"]
# Default-object member aliases, from source `Stack.tsx:59` (export default {Row: RowStack, Col: ColStack}).
ALIASES = {"RowStack": ["Row"], "ColStack": ["Col"]}


def jl(name):
    with open(os.path.join(run, name)) as f:
        return [json.loads(line) for line in f if line.strip()]


def dump(name, value):
    with open(os.path.join(out, name), "w") as f:
        f.write(json.dumps(value, indent=1, sort_keys=True) + "\n")


def start_line(t):
    src = open(os.path.join(root, t["path"]), "rb").read()
    return src[: t["callable"]["start_byte"]].count(b"\n") + 1


def split_args(text):
    """Top-level comma split of the parenthesized argument list of a call expression."""
    i = text.index("(")
    depth, cur, args, quote = 0, "", [], None
    for ch in text[i + 1:]:
        if quote:
            cur += ch
            quote = None if ch == quote else quote
            continue
        if ch in "'\"`":
            quote = ch
        elif ch in "([{":
            depth += 1
        elif ch in ")]}":
            if depth == 0:
                break
            depth -= 1
        elif ch == "," and depth == 0:
            args.append(cur.strip())
            cur = ""
            continue
        cur += ch
    if cur.strip():
        args.append(cur.strip())
    return args


sites = jl("call-stats-dump-sites.jsonl")
edges = jl("dfg-stats-edges.jsonl")
e1, e2, e3, e4, rows = [], [], [], [], []
for i, t in enumerate(targets):
    name, path, ordinal = t["callable"]["name"], t["path"], t["ordinal"]
    line, tag = start_line(t), f"{i:02d}-{name}"
    callers = json.load(open(os.path.join(run, f"callers-{tag}.json")))["items"]
    for c in callers:
        called = [w["CalledBy"] for w in c["why"] if "CalledBy" in w]
        kind = [w["Resolution"]["kind"] for w in c["why"] if "Resolution" in w]
        e1.append({"target": name, "caller_file": c["location"]["file"], "caller": called[0]["caller"],
                   "call_line": called[0]["call_site_line"], "kind": kind[0] if kind else None,
                   "score": c["score"]})
    names = [name] + ALIASES.get(name, [])
    for s in sites:
        if s.get("record_kind") != "call_site" or s["callee_text"] not in names:
            continue
        hit = [r for r in s["resolved_targets"]
               if r["function_id"]["file"] == path and r["function_id"]["name"] == name]
        lane = ("resolved_to_target" if hit else "resolved_elsewhere" if s["resolved_targets"]
                else f"drop:{s['drop']}" if s["drop"] else "unowned_or_unrecorded")
        if s["callee_text"] != name:
            lane += ":alias"
        e2.append({"target": name, "callee_text": s["callee_text"], "file": s["source_span"]["file"],
                   "line": s["source_span"]["line"], "start_byte": s["source_span"]["start_byte"],
                   "lane": lane})
    nodes = json.load(open(os.path.join(run, f"nodes-at-{tag}.json")))["items"]
    b = t["binding"]
    entry = any(n["symbol"].get("Variable", {}).get("access") == "Def"
                and n["location"]["start_byte"] == b["start_byte"]
                and n["location"]["end_byte"] == b["end_byte"] for n in nodes)
    e3.append({"target": name, "binding": b["name"], "ordinal": ordinal, "entry_def": entry})
    inbound = [x for x in edges if x["to"]["file"] == path and x["to"]["line"] == line
               and x["to"]["access"] == "def" and x["to"]["path"]["base"] == b["name"]]
    e4.append({"target": name, "binding": b["name"], "entry_line": line, "inbound_edges": len(inbound)})
    # Per resolved caller: the argument text at the selected ordinal, from source bytes.
    for s in sites:
        if s.get("record_kind") != "call_site":
            continue
        if not any(r["function_id"]["file"] == path and r["function_id"]["name"] == name
                   for r in s["resolved_targets"]):
            continue
        sp = s["source_span"]
        text = open(os.path.join(root, sp["file"]), "rb").read()[sp["start_byte"]:sp["end_byte"]]
        args = split_args(text.decode("utf-8"))
        arg = args[ordinal] if ordinal < len(args) else None
        # A bare JS identifier: [A-Za-z_$][A-Za-z0-9_$]*, excluding literals and keywords that cannot carry flow.
        is_ident = (bool(arg) and re.fullmatch(r"[A-Za-z_$][A-Za-z0-9_$]*", arg) is not None
                    and arg not in {"true", "false", "null", "undefined", "this", "NaN", "Infinity"})
        rows.append({"target": name, "file": sp["file"], "line": sp["line"], "arg_at_ordinal": arg,
                     "identifier_arg": is_ident, "entry_def": entry,
                     "blocked_only_by_prefix": bool(entry and is_ident)})

key = lambda r: json.dumps(r, sort_keys=True)
for name, rows_ in [("E1-callers.json", e1), ("E2-callsite-ledger.json", e2), ("E3-entry-defs.json", e3),
                    ("E4-inbound-edges.json", e4), ("E5-caller-arguments.json", rows)]:
    dump(name, sorted(rows_, key=key))
blocked = [r for r in rows if r["blocked_only_by_prefix"]]
Y, S = len(blocked), len({r["target"] for r in blocked})
control = [x for x in edges if x["to"]["file"] == "packages/excalidraw/data/index.ts"
           and x["to"]["access"] == "def" and x["to"]["path"]["base"] == "elements"
           and x["to"]["line"] == 47]
dump("DECISION.json", {"Y": Y, "S": S, "threshold": "Y>=3 and S>=2",
                       "warranted": Y >= 3 and S >= 2,
                       "decision": ("repair_warranted" if Y >= 3 and S >= 2 else
                                    "defer/no_exact_caller_blocked_only_by_positional_prefix" if Y == 0
                                    else "defer/prefix_only_yield_below_threshold"),
                       "ordinal0_control_edges_into_elements": len(control),
                       "blocked_rows": blocked})
print(json.dumps({"Y": Y, "S": S, "entry_defs": sum(r["entry_def"] for r in e3),
                  "resolved_callers": len(e1), "inbound_edges": sum(r["inbound_edges"] for r in e4),
                  "control": len(control)}))
