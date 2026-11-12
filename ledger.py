import sys, collections, os, json
def kind(f):
    lo = f.lower()
    if lo.endswith(".c"): return "c_source"
    if lo.endswith(".h"): return "dot_h"
    if lo.endswith((".hpp", ".hh", ".hxx", ".inl", ".ipp", ".tcc")): return "cpp_header"
    return "cpp_source"
out = {}
for c in ["leveldb", "redis", "cpython"]:
    own = collections.defaultdict(collections.Counter)
    esc = collections.defaultdict(collections.Counter)
    use = collections.defaultdict(collections.Counter)
    dr = collections.defaultdict(collections.Counter)
    seen = set()
    for l in open(f"/tmp/out/{c}.head.trace", errors="replace"):
        p = l.rstrip("\n").split("\t")
        if p[0] == "c-prune-owner":
            k = kind(p[-2])
            st = p[1]
            if st.startswith("off:"):
                own[k][st] += 1
            else:
                own[k]["on" if st == "on" else "on-no-d(G6)"] += 1
                f = {x.split("=")[0]: x.split("=", 1)[1] for x in p[2:-2] if "=" in x}
                if f.get("template") == "true": own[k]["template_owner(on)"] += 1
                if int(f.get("scalar_fail", 0)) > 0: own[k]["owners_with_scalar_gate_failures"] += 1
                for ek in filter(None, f.get("esc_kinds", "").split(",")):
                    esc[k][ek.split(":")[0]] += 1
            own[k]["total"] += 1
        elif p[0] == "c-prune-use":
            k = kind(p[1]); use[k][p[4].split("+")[0] if p[4].startswith("escape") else p[4]] += 1
            if p[4].startswith("escape:"):
                for e in p[4][7:].split("+"): use[k]["escape_kind:" + e] += 1
        elif p[0] == "c-prune-drop":
            dr[kind(p[2])][p[1]] += 1
    out[c] = {"owners": {k: dict(v) for k, v in own.items()}, "owners_with_escape_kind": {k: dict(v) for k, v in esc.items()},
              "refused_use_lines": {k: dict(v) for k, v in use.items()}, "drop_events": {k: dict(v) for k, v in dr.items()}}
print(json.dumps(out, indent=1))
