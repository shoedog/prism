#!/usr/bin/env python3
"""Compare two `prism nav dfg-stats --edges` JSONL files (base vs head).

usage: compare_edges.py BASE.edges.jsonl HEAD.edges.jsonl [--sample N --seed S]
Edge identity = (from, to) with confidence/doubt/kill_line ignored; multiplicity counts.
Exit 0 iff the edge multisets are identical (labels may differ); exit 1 otherwise.
Prints the set difference, the label-transition table, and optionally N moved edges.
"""
import collections, json, random, sys

def load(path):
    m = collections.defaultdict(list)
    for line in open(path):
        j = json.loads(line)
        key = json.dumps({k: j[k] for k in ("from", "to")}, sort_keys=True)
        m[key].append((j["confidence"], j.get("doubt") or "-", j.get("kill_line") is not None))
    return m

def main():
    args = sys.argv[1:]
    base, head = load(args[0]), load(args[1])
    n = int(args[args.index("--sample") + 1]) if "--sample" in args else 0
    seed = int(args[args.index("--seed") + 1]) if "--seed" in args else 599
    cb = collections.Counter({k: len(v) for k, v in base.items()})
    ch = collections.Counter({k: len(v) for k, v in head.items()})
    only_base, only_head = cb - ch, ch - cb
    print(f"edges base={sum(cb.values())} head={sum(ch.values())}")
    print(f"only in base: {sum(only_base.values())}  only in head: {sum(only_head.values())}")
    trans, moved = collections.Counter(), []
    for k in base:
        if k in head and sorted(base[k]) != sorted(head[k]):
            moved.append(k)
            old = collections.Counter(base[k]); new = collections.Counter(head[k])
            for o in (old - new).elements():
                trans[("-", o)] += 1
            for w in (new - old).elements():
                trans[("+", w)] += 1
    print(f"edge keys whose label multiset changed: {len(moved)}")
    print("label rows leaving (-) / arriving (+)  [confidence, doubt, has_kill_line]:")
    for (sign, lab), c in sorted(trans.items(), key=lambda x: (-x[1], x[0])):
        print(f"  {sign} {lab}: {c}")
    if n and moved:
        random.seed(seed)
        for k in random.sample(moved, min(n, len(moved))):
            j = json.loads(k)
            print(f"  {j['from']['file']} L{j['from']['line']} {j['from']['path']['base']} -> L{j['to']['line']} "
                  f"{j['to']['path']['base']}{''.join('.'+x for x in j['to']['path']['fields'])}: "
                  f"{sorted(base[k])} -> {sorted(head[k])}")
    ok = not only_base and not only_head
    print("PASS: identical edge sets, only labels moved" if ok else "FAIL: edge sets differ")
    return 0 if ok else 1

sys.exit(main())
