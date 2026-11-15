#!/usr/bin/env python3
"""Runtime receipts for #829 (TEST-FORM section 4, 'one runtime driver per class'). Python 3.9.

A receipt compiles a fixture with clang, runs `owner(0)` and `owner(1)` and shows the pair
`x = <first def> -> use` is LIVE at run time: owner(0) returns the first def's value (41 in C,
42 through `sink(x + 1)` in C++), so the overwrite `x = 2` does not always run. That is the strongest truth ("a live pair is dropped on head") and the
anchor for the oracle's cheaper static verdicts. Not run in CI (needs clang); sources are
data in `tests/fixtures/dfg_prune/oracle/receipts.jsonl`, never files under `tests/fixtures` with C-like names.

  xarm_receipts.py build EXPORT_DIR   select one row per class from the export, write the data
  xarm_receipts.py run                compile and run every receipt, print a table, exit 1 on a
                                      receipt whose pair is not live
"""
import json
import os
import subprocess
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
ORACLE = os.path.join(HERE, "..", "..", "tests", "fixtures", "dfg_prune", "oracle")
DATA = os.path.join(ORACLE, "receipts.jsonl")
sys.path.insert(0, HERE)
import xarm  # noqa: E402

# class name -> (test-name substring, `what` substring or "", extension of the target)
CLASSES = {
    "enclosing group opened in a sibling owner": ("opus3_03_opener_inside_a_sibling", "", "c"),
    "macro drops the overwriting piece": ("opus3_02_a_macro_drops_the_overwriting_piece_c", "", "c"),
    "opener inside a declaration initializer": ("opus3_04_opener_inside_a_declaration", "", "c"),
    "digraph brace formed by a splice": ("sonnet3_01_digraph_brace", "", "c"),
    "comment opener formed by a splice": ("sonnet3_02_comment_opener", "", "c"),
    "line-comment introducer formed by a splice": ("sonnet3_03_line_comment", "", "c"),
    "comment introducer in a string, then a brace": ("terra3_01_string_comment_introducer", "", "c"),
    "comma between comment parentheses": ("terra3_02_comma_between_comment", "", "c"),
    "alias chain of 1000 to an unbalanced macro": ("sonnet3_07_alias_chain_to_an_unbalanced", "", "c"),
    "Latin-1 byte in the payload": ("sonnet3_05_latin1_payload", "defs.h", "c"),
    "transitive non-C include": ("g_sol_14_transitive_non_c_include", "", "c"),
    "computed include of a net file": ("g2_terra_03_computed_include_of_a_net_file", "", "c"),
    "closer inside a following owner": ("opus3_05_closer_inside_a_following", "", "c"),
    "external macro via a forced include (declared)": ("opus3_10_external_enclosing", "", "c"),
    "rescan split through a header": ("g_sol_03_rescan_split", "", "c"),
}


def includes_resolve(r):
    """Every quoted include of the target names a file of the row (or a forced include does)."""
    text = r["_files"][r["target"]].decode("latin-1")
    for t in xarm.core.INC.findall(text.encode("latin-1")):
        if t.decode() not in r["_files"] and not t.decode().startswith("."):
            return False
    return True


def pick(rows, test, what, ext):
    for r in rows:
        if test in r["test"] and what in r["what"] and r["target"].endswith("." + ext) \
                and r["oracle"] == "CONTRADICTED" and includes_resolve(r):
            return r
    return None


def build(export):
    rows = xarm.load_pair_rows(export)
    labelled = {json.loads(l)["key"]: json.loads(l)
                for l in open(os.path.join(ORACLE, "pair_rows.jsonl"))}
    for r in rows:
        r.update({k: labelled[r["key"]][k] for k in ("oracle", "config")})
    out = []
    for cls, (test, what, ext) in CLASSES.items():
        r = pick(rows, test, what, ext)
        if not r:
            print("no row for", cls, file=sys.stderr)
            continue
        cfg = r.get("config") or {}
        decl = r.get("declared") or {}
        out.append({"class": cls, "key": r["key"], "target": r["target"], "owner": r["owner"],
                    "files": {n: d.decode("latin-1") for n, d in r["_files"].items()},
                    "flags": cfg.get("flags", []), "prelude": dict(decl.get("prelude", [])),
                    "links": r["links"]})
    with open(DATA, "w") as f:
        for o in out:
            f.write(json.dumps(o, sort_keys=True) + "\n")
    print(len(out), "receipts")


def main_for(target, owner):
    stdio = "#include <stdio.h>\n"
    return stdio + "int main(void) { printf(\"%%d %%d\\n\", %s(0), %s(1)); return 0; }\n" % (owner, owner)


def run():
    bad = 0
    for line in open(DATA):
        o = json.loads(line)
        with tempfile.TemporaryDirectory() as d:
            files = {n: c.encode("latin-1") for n, c in o["files"].items()}
            xarm.core.materialize(files, o["links"], d, rewrite_abs=True)
            for n, text in o["prelude"].items():
                p = os.path.join(d, "..", n)
                os.makedirs(os.path.dirname(os.path.abspath(p)), exist_ok=True)
                open(p, "w").write(text)
            tgt = os.path.join(d, o["target"])
            src = open(tgt, "rb").read().decode("latin-1")
            if "main(" not in src:
                open(tgt, "wb").write((src + main_for(o["target"], o["owner"])).encode("latin-1"))
            cxx = o["target"].endswith(".cc")
            flags = [f.replace("{PRELUDE}", os.path.abspath(os.path.join(d, ".."))) for f in o["flags"]]
            exe = os.path.join(d, "prog")
            cmd = ["clang++" if cxx else "clang", "-w", "-fbracket-depth=100000", "-I.",
                   "-std=c++17" if cxx else "-std=gnu11"] + flags + [o["target"], "-o", exe]
            p = subprocess.run(cmd, cwd=d, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
            if p.returncode:
                print("%-52s COMPILE-FAIL %s" % (o["class"], p.stderr.decode()[:100]))
                bad += 1
                continue
            run = subprocess.run([exe], stdout=subprocess.PIPE)
            vals = run.stdout.decode().split()
            if "main(" in src:
                # the fixture's own driver exits 0 when probe(0) == 41 and probe(1) == 2
                live, vals = run.returncode == 0, ["exit", str(run.returncode)]
            else:
                # the first def reaches the use at c == 0: 41 in C, 42 through sink(x + 1) in C++
                live = bool(vals) and vals[0] in ("41", "42")
            bad += not live
            print("%-52s %-8s %s" % (o["class"], "LIVE" if live else "NOT-LIVE", " ".join(vals)))
    sys.exit(1 if bad else 0)


if __name__ == "__main__":
    if sys.argv[1:2] == ["build"]:
        build(sys.argv[2])
    elif sys.argv[1:2] == ["run"]:
        run()
    else:
        print(__doc__)
        sys.exit(2)
