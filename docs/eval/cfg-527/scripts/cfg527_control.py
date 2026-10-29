"""Declared-type control (design §3.2 probe 2 negative control) for WRONG candidates.

Each listed site's call is replaced by `{ let _p527c: <TYPE> = CALL; unreachable!() }`, where
TYPE is the return type of prism's Exact target, spelled as a path valid at the site. If rustc
bound the target, the site compiles (no E0308 there); if it bound another callable, E0308 names
the found type in full. One build per (package, lane) round, `cargo check --keep-going`.
Usage: cfg527_control.py --sample sample.json --corpus C --root DIR --work W --out FILE
         --controls controls.json   ([{site_id, type, config: {name,args,rustflags}}])
"""

import argparse
import collections
import json
import os
import re
import shutil
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import cfg527_adjudicate as adj  # noqa: E402


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--sample", required=True)
    ap.add_argument("--corpus", required=True)
    ap.add_argument("--root", required=True)
    ap.add_argument("--work", required=True)
    ap.add_argument("--controls", required=True)
    ap.add_argument("--out", required=True)
    a = ap.parse_args()
    with open(a.sample) as f:
        sites = {s["site_id"]: s for s in json.load(f)["sites"] if s["corpus"] == a.corpus}
    with open(a.controls) as f:
        controls = json.load(f)
    src = os.path.join(a.work, a.corpus + "-ctl-src")
    if os.path.exists(src):
        shutil.rmtree(src)
    subprocess.run(["cp", "-a", os.path.abspath(a.root), src], check=True)
    logs = os.path.join(a.work, "logs-%s-control" % a.corpus)
    os.makedirs(logs, exist_ok=True)
    groups = collections.defaultdict(list)
    for c in controls:
        s = sites[c["site_id"]]
        lane = "tests" if re.search(r"(^|/)(tests|benches|examples)/", s["caller"]["file"]) else "src"
        groups[(json.dumps(c["config"], sort_keys=True), s.get("caller_package") or "", lane)].append((c, s))
    results = []
    for (cfgj, pkg, lane), items in sorted(groups.items()):
        cfg = json.loads(cfgj)
        ed = adj.Edits()
        for c, s in items:
            sp = s["span"]
            ed.add(sp["file"], sp["start_byte"], "{ let _p527c: %s = " % c["type"])
            ed.add(sp["file"], sp["end_byte"], "; unreachable!() }")
        ed.apply(src, os.path.abspath(a.root))
        env = dict(os.environ)
        env["CARGO_TARGET_DIR"] = os.path.join(a.work, "target-" + a.corpus)
        env["RUSTFLAGS"] = " ".join([adj.CAP] + cfg.get("rustflags", []))
        cmd = ["cargo", "check", "--keep-going", "--message-format=json"] + cfg["args"] + ["-p", pkg]
        tag = "%s-%s-%s" % (cfg["name"], pkg, lane)
        out, code, secs = adj.sh(cmd, src, env, os.path.join(logs, tag + ".stderr"), 1500)
        msgs, ok = adj.messages(out)
        with open(os.path.join(logs, tag + ".jsonl"), "w") as f:
            for m in msgs:
                f.write(json.dumps({k: m.get(k) for k in ("code", "level", "message", "spans")}) + "\n")
        ed.restore(src, os.path.abspath(a.root))
        errs = []
        for m in msgs:
            if m.get("level") != "error":
                continue
            for p in m["spans"]:
                if p["is_primary"]:
                    fr = adj.norm_file(p["file_name"], src)
                    errs.append((fr, ed.to_orig(fr, p["byte_start"]), (m.get("code") or {}).get("code"),
                                 p.get("label"), m.get("message")))
        for c, s in items:
            sp = s["span"]
            here = [e for e in errs if e[0] == sp["file"] and sp["start_byte"] - 64 <= e[1] <= sp["end_byte"]]
            e308 = [e for e in here if e[2] == "E0308"]
            results.append({"site_id": s["site_id"], "callee_text": s["callee_text"], "span": sp,
                            "config": cfg["name"], "declared_type": c["type"], "round_exit": code,
                            "outcome": "type_mismatch" if e308 else ("other_error" if here else "compiles"),
                            "diagnostics": [{"code": e[2], "label": e[3], "message": e[4]} for e in here][:4]})
    with open(a.out, "w") as f:
        for r in results:
            f.write(json.dumps(r, sort_keys=True) + "\n")
    for r in results:
        print(r["callee_text"], r["span"]["file"], r["span"]["line"], r["outcome"],
              [(d["code"], d["label"]) for d in r["diagnostics"]][:2])


if __name__ == "__main__":
    main()
