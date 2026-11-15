#!/usr/bin/env python3
"""Expansion-arm oracle for the U/D prune rows (#829 TEST-FORM). See XARM.md. Python 3.9.

  xarm.py label  EXPORT_DIR [--jobs N] [--work DIR]   label every exported row, write the data
  xarm.py derive                                      re-derive `assert` from stored verdicts
  xarm.py corpus ...                                  L3 corpus differentials (xarm_l3.py)

`label` reads the rows `PRISM_829_EXPORT_DIR` collected (pair rows built from Rust sources, plus
the inline cases) and writes `tests/fixtures/dfg_prune/oracle/pair_rows.jsonl` and `inline_rows.jsonl`, which
`gen_obligations.py` merges into the obligations and the Rust tests read (no clang there).
"""
import argparse
import collections
import concurrent.futures
import glob
import json
import os
import re
import shutil
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
FIX = os.path.join(HERE, "..", "..", "tests", "fixtures", "dfg_prune")
sys.path.insert(0, HERE)
import xarm_core as core  # noqa: E402

# Headers outside the repository that fixtures name. Missing headers make the row UNJUDGED.
NUMPY = "#define NPY_BEGIN_ALLOW_THREADS Py_BEGIN_ALLOW_THREADS\n" \
        "#define NPY_END_ALLOW_THREADS Py_END_ALLOW_THREADS\n"
PYIPH = ("#define _Py_BEGIN_SUPPRESS_IPH { _invalid_parameter_handler _Py_old_handler = "
         "_set_thread_local_invalid_parameter_handler(_Py_silent_invalid_parameter_handler);\n"
         "#define _Py_END_SUPPRESS_IPH _set_thread_local_invalid_parameter_handler("
         "_Py_old_handler); }\n")
PGTRY = ("#define PG_TRY(...) do { sigjmp_buf *_save_exception_stack = PG_exception_stack; "
         "ErrorContextCallback *_save_context_stack = error_context_stack; sigjmp_buf "
         "_local_sigjmp_buf; bool _do_rethrow = false; if (sigsetjmp(_local_sigjmp_buf, 0) == 0) "
         "{ PG_exception_stack = &_local_sigjmp_buf\n"
         "#define PG_CATCH(...) } else { PG_exception_stack = _save_exception_stack; "
         "error_context_stack = _save_context_stack\n"
         "#define PG_END_TRY(...) } PG_exception_stack = _save_exception_stack; "
         "error_context_stack = _save_context_stack; } while (0)\n")
PYALLOW = ("#define Py_BEGIN_ALLOW_THREADS { PyThreadState *_save; _save = PyEval_SaveThread();\n"
           "#define Py_END_ALLOW_THREADS PyEval_RestoreThread(_save); }\n")
PYTRASH = ("#define Py_TRASHCAN_BEGIN_CONDITION(op, cond) do { PyThreadState *_tstate = NULL; "
           "if (cond) { _tstate = _PyThreadState_UncheckedGet(); "
           "if (_PyTrash_begin(_tstate, _PyObject_CAST(op))) { break; } }\n"
           "#define Py_TRASHCAN_END if (_tstate) { _PyTrash_end(_tstate); } } while (0);\n"
           "#define Py_TRASHCAN_BEGIN(op, dealloc) Py_TRASHCAN_BEGIN_CONDITION((op), "
           "_PyTrash_cond(_PyObject_CAST(op), (destructor)(dealloc)))\n"
           "#define Py_TRASHCAN_SAFE_BEGIN(op) do { PyThreadState *_tstate = PyThreadState_GET(); "
           "if (_PyTrash_begin(_tstate, _PyObject_CAST(op))) break;\n"
           "#define Py_TRASHCAN_SAFE_END(op) _PyTrash_end(_tstate); } while (0);\n")
PYCS = ("#define Py_BEGIN_CRITICAL_SECTION(op) { PyCriticalSection _py_cs; "
        "PyCriticalSection_Begin(&_py_cs, _PyObject_CAST(op))\n"
        "#define Py_END_CRITICAL_SECTION() PyCriticalSection_End(&_py_cs); }\n"
        "#define Py_BEGIN_CRITICAL_SECTION_MUT(mutex) { PyCriticalSection _py_cs; "
        "PyCriticalSection_BeginMutex(&_py_cs, mutex)\n"
        "#define Py_BEGIN_CRITICAL_SECTION_MUTEX(mutex) Py_BEGIN_CRITICAL_SECTION_MUT(mutex)\n"
        "#define Py_BEGIN_CRITICAL_SECTION2(a, b) { PyCriticalSection2 _py_cs2; "
        "PyCriticalSection2_Begin(&_py_cs2, _PyObject_CAST(a), _PyObject_CAST(b))\n"
        "#define Py_BEGIN_CRITICAL_SECTION2_MUT(m1, m2) { PyCriticalSection2 _py_cs2; "
        "PyCriticalSection2_BeginMutex(&_py_cs2, m1, m2)\n"
        "#define Py_BEGIN_CRITICAL_SECTION2_MUTEX(m1, m2) Py_BEGIN_CRITICAL_SECTION2_MUT(m1, m2)\n"
        "#define Py_END_CRITICAL_SECTION2() PyCriticalSection2_End(&_py_cs2); }\n"
        "#define Py_BEGIN_CRITICAL_SECTION_SEQUENCE_FAST(original) { PyCriticalSection _py_cs_sf; "
        "_PyCriticalSection_BeginSequenceFast(&_py_cs_sf, original)\n"
        "#define Py_END_CRITICAL_SECTION_SEQUENCE_FAST() PyCriticalSection_End(&_py_cs_sf); }\n")
# (regex of the names, flags, prelude {name: text}, prefix of the names the repository may
# define itself: then that definition is the one in effect). The Python and PostgreSQL texts
# are reproduced from the headers of the versions the fixtures name (approximate whitespace).
GROUPS = [
    (r"\bpthread_cleanup_(?:push|pop)", ["-include", "pthread.h"], {}, "pthread_cleanup_"),
    (r"\bPy_(?:BEGIN|END)_CRITICAL_SECTION", ["-include", "{PRELUDE}/pycs.h"],
     {"pycs.h": PYCS}, "Py_"),
    (r"\bPy_TRASHCAN", ["-include", "{PRELUDE}/pytrash.h"], {"pytrash.h": PYTRASH}, "Py_"),
    (r"\bPy_(?:BEGIN|END)_ALLOW_THREADS", ["-include", "{PRELUDE}/pyallow.h"],
     {"pyallow.h": PYALLOW}, "Py_"),
    (r"\bNPY_(?:BEGIN|END)_ALLOW_THREADS", ["-include", "{PRELUDE}/pyallow.h", "-include", "{PRELUDE}/numpy.h"],
     {"numpy.h": NUMPY, "pyallow.h": PYALLOW}, "NPY_"),
    (r"\bPG_(?:TRY|CATCH|END_TRY)", ["-include", "{PRELUDE}/pgtry.h"], {"pgtry.h": PGTRY}, "PG_"),
    (r"\b_Py_(?:BEGIN|END)_SUPPRESS_IPH", ["-include", "{PRELUDE}/iph.h"], {"iph.h": PYIPH},
     "_Py_"),
]
# Inline rows whose outside names the fixture cannot show by pattern: (case, id) -> config.
EXT25 = "#define A F\n#define B G\n#define EXT_APPLY(m, ...) m(__VA_ARGS__)\n"
PREFIX = {"flags": ["-include", "{PRELUDE}/m.h"], "prelude": {"m.h": "#define M if (c)\n"},
          "assumption": "A-ext"}
PREFIX_FN = {"flags": ["-include", "{PRELUDE}/m.h"], "prelude": {"m.h": "#define M(x) if (x)\n"},
             "assumption": "A-ext"}
EXT_ALIAS = {"flags": ["-include", "{PRELUDE}/ext25.h"], "prelude": {"ext25.h": EXT25},
             "assumption": "A-ext"}
ROW_CONFIG = {
    ("25_r2_external_alias", "g2t01"): EXT_ALIAS, ("25_r2_external_alias", "g2o06"): EXT_ALIAS,
    ("25_r2_external_alias.cc", "g2t01"): EXT_ALIAS, ("25_r2_external_alias.cc", "g2o06"): EXT_ALIAS,
    ("33_r2_digit_separator", "g2o03"): {"flags": ["-std=c2x"], "assumption": "A-dialect"},
}
for _id in ("g2s03a", "g2s03b", "g2s03d", "g2s03e1", "g2s03e2", "g2s03k1", "g2s03k2", "g2s03k3"):
    ROW_CONFIG[("26_r2_external_names", _id)] = PREFIX
    ROW_CONFIG[("26_r2_external_names.cc", _id)] = PREFIX
ROW_CONFIG[("26_r2_external_names", "g2s03c")] = PREFIX_FN


def owner_scope(files, owner):
    """The text a row of `owner` can depend on: the owner's own lines and every define line."""
    out = []
    for d in files.values():
        lines = d.decode("latin-1").split("\n")
        out += [l for l in lines if re.match(r"\s*#\s*define", l)]
        for i, l in enumerate(lines):
            if not l.startswith("#") and re.search(r"\b" + re.escape(owner) + r"\s*\(", l) \
                    and l[:1].isalpha():
                j = i
                while j < len(lines) and lines[j] != "}":
                    j += 1
                out += lines[i:j + 1]
                break
    return "\n".join(out)


def auto_config(files, owner=None):
    """Config the text calls for: outside names with no in-repo definition (within the owner
    and the defines when an owner is given)."""
    text = owner_scope(files, owner) if owner else \
        "\n".join(d.decode("latin-1") for d in files.values())
    flags, prelude, missing = [], {}, []
    for rx, fl, pre, own in GROUPS:
        if not re.search(rx, text):
            continue
        if own and re.search(r"#\s*define\s+" + own, text):
            continue
        flags += fl
        prelude.update(pre)
    return {"flags": flags, "prelude": prelude, "missing": missing,
            "assumption": "A-ext" if (flags or missing) else None}


def merge(*cfgs):
    out = {"flags": [], "prelude": {}, "assumption": None, "texts": [], "missing": [],
           "declared": False, "in_model": False}
    for c in cfgs:
        if not c:
            continue
        out["flags"] += c.get("flags", [])
        out["prelude"].update(c.get("prelude", {}))
        out["texts"] += c.get("texts", [])
        out["missing"] += c.get("missing", [])
        out["assumption"] = out["assumption"] or c.get("assumption")
        out["declared"] = out["declared"] or c.get("declared", False)
        out["in_model"] = out["in_model"] or c.get("in_model", False)
    return out


def has_config(cfg):
    return bool(cfg["flags"] or cfg["prelude"] or cfg["texts"])


def judge_pair(arms, repo_x, decl_x, cfg, file, path, a, b, files):
    """-> the oracle fields of one pair."""
    v0, d0 = core.verdict(arms, *repo_x, file, path, a, b)
    if decl_x:
        v1, d1 = core.verdict(arms, *decl_x, file, path, a, b)
    elif cfg["missing"]:
        v1, d1 = "UNJUDGED", "outside headers missing: " + ",".join(cfg["missing"])
    else:
        v1, d1 = v0, d0
    opened = (decl_x or repo_x)[2]["opened"]
    gap = core.inputs_gap(files, opened)
    # an in-model row (PLAN-v2 task 1a) declares its outside header as the ORACLE prelude only:
    # the declared run is the row's truth and never a BOUNDARY flip (no assumption waives it)
    flip = bool(decl_x) and v0 != v1 and v1 == "CONTRADICTED" and not cfg.get("in_model")
    assumption = cfg["assumption"]
    if gap:
        assumption = "A-inputs"
    elif not assumption and flip:
        assumption = "A-inputs" if cfg["texts"] else "A-config"
    seen_input = bool(assumption == "A-inputs" and not gap and not cfg["declared"])
    return {"seen_input": seen_input, "oracle": v1, "oracle_repo": v0, "oracle_declared": v1 if decl_x else None,
            "basis": "repo: %s%s; declared: %s%s" % (v0, " " + d0 if d0 else "", v1,
                                                      " " + d1 if d1 else ""),
            "boundary": bool(flip or gap), "inputs_gap": gap, "assumption": assumption}


def config_of(cfg):
    return {"flags": cfg["flags"], "prelude": sorted(cfg["prelude"]), "texts": cfg["texts"]} \
        if has_config(cfg) else None


def run_fixture(work, files, links, wants):
    """`wants`: list of (cfg, [(file, path, a, b)]) -> list of lists of oracle dicts and CLI
    head-drop flags. One Arms per fixture; repo-only expansion plus one per distinct config."""
    arms = core.Arms(work, files, links)
    repo_x = arms.expansion("0", [], {})
    out = []
    for n, (cfg, pairs) in enumerate(wants, 1):
        decl_x = arms.expansion(str(n), cfg["flags"], cfg["prelude"]) \
            if has_config(cfg) and not cfg["missing"] else None
        res = []
        for file, path, a, b in pairs:
            r = judge_pair(arms, repo_x, decl_x, cfg, file, path, a, b, files)
            r["head_drops_cli"] = core.head_drops(arms, file, path, a, b)
            r["config"] = config_of(cfg)
            res.append(r)
        out.append(res)
    return out


HEADER_EXT = (".h", ".hh", ".hpp", ".hxx", ".inc")


def dynamic_reach(files, links, scratch):
    """In-repo files clang opens (`-H`, missing outside headers stubbed) for the main files:
    the include forms a regex cannot read (a computed `#include HEADER`) resolve here."""
    root = os.path.realpath(os.path.join(scratch, "reach"))
    core.materialize(files, links, root)
    stubs = os.path.join(scratch, "reach_stubs")
    shutil.rmtree(stubs, ignore_errors=True)
    os.makedirs(stubs)
    opened = set()
    for rel in files:
        if not rel.endswith(core.CEXT + core.CXXEXT):
            continue
        src = os.path.join(root, rel)
        for _ in range(20):
            cmd = core.clang_cmd(src, ["-I.", "-idirafter", stubs], ["-H"])
            p = core.subprocess.run(cmd, stdout=core.subprocess.PIPE, stderr=core.subprocess.PIPE,
                                    cwd=root, timeout=300)
            err = p.stderr.decode("utf-8", "replace")
            for line in err.splitlines():
                m = re.match(r"^\.+ (.+)$", line)
                if m:
                    path = os.path.abspath(os.path.join(root, m.group(1)))
                    if path.startswith(root + os.sep):
                        opened.add(os.path.relpath(path, root))
            m = re.search(r"fatal error: '([^']+)' file not found", err)
            if not m:
                break
            sp = os.path.join(stubs, m.group(1))
            os.makedirs(os.path.dirname(sp), exist_ok=True)
            open(sp, "w").close()
    return opened


def orphan_headers(files, links=None, scratch=None):
    """Header files no main file reaches through an include (by regex, or by what clang opens
    when `scratch` is given): the fixture author meant them to be part of the translation unit
    only when they model a forced include (the census reads every in-repo header, and a net
    file of a C-like extension is collected without an include); the compiler then gets them
    as forced includes. A header the fixture does not include is otherwise a defect of the
    fixture (the owner file should include it)."""
    reach = set()
    if scratch:
        reach |= dynamic_reach(files, links or {}, scratch)
    todo = [r for r in files if r.endswith(core.CEXT + core.CXXEXT)]
    while todo:
        rel = todo.pop()
        for m in core.INC.finditer(files.get(rel, b"")):
            t = m.group(1).decode("utf-8", "replace").replace("\\", "/").lstrip("./")
            for cand in files:
                if cand not in reach and (cand == t or cand.endswith("/" + t)):
                    reach.add(cand)
                    todo.append(cand)
    return sorted(r for r in files if r.endswith(HEADER_EXT) and r not in reach)


def load_pair_rows(export):
    rows = []
    for rj in sorted(glob.glob(os.path.join(export, "rows", "*", "row.json"))):
        row = json.load(open(rj))
        root = os.path.join(os.path.dirname(rj), "f")
        files = {}
        for dp, _, fs in os.walk(root):
            for f in fs:
                p = os.path.join(dp, f)
                files[os.path.relpath(p, root)] = open(p, "rb").read()
        row["_files"] = files
        rows.append(row)
    return rows


def label_pair_rows(export, jobs, work):
    rows = load_pair_rows(export)
    out = []
    with concurrent.futures.ThreadPoolExecutor(jobs) as ex:
        futs = []
        for i, row in enumerate(rows):
            decl = row.get("declared") or {}
            tcfg = {"flags": [x for t in row["texts"] for x in ("-include", t)],
                    "texts": list(row["texts"]), "assumption": "A-inputs" if row["texts"] else None}
            tu = decl.get("tu")
            if tu and row["target"] != tu:
                # The file is never compiled alone (only the translation unit `tu` includes it):
                # judging it alone has no basis. A runtime receipt decides (overrides.json).
                rec = {k: row[k] for k in ("key", "test", "what", "want", "target", "owner",
                                           "path", "from", "to", "files")}
                rec.update({"seen_input": False, "oracle": "UNJUDGED", "oracle_repo": "UNJUDGED",
                            "oracle_declared": None, "boundary": False, "inputs_gap": [],
                            "assumption": None, "config": None, "tu": tu,
                            "basis": "repo: UNJUDGED target-not-a-translation-unit (tu %s); "
                                     "declared: UNJUDGED" % tu,
                            "head_drops": bool(row["head_before"] and not row["head_after"])})
                out.append(rec)
                continue
            dcfg = {"flags": decl.get("flags", []),
                    "prelude": {n: c for n, c in decl.get("prelude", [])},
                    "assumption": decl.get("assumption"), "declared": bool(decl),
                    "in_model": bool(decl.get("in_model"))}
            orph = [o for o in orphan_headers(row["_files"], row["links"], os.path.join(work, "_o" + row["key"]))
                    if o not in row["texts"]]
            ocfg = {"flags": [x for o in orph for x in ("-include", o)], "texts": orph,
                    "assumption": "A-inputs" if orph else None}
            cfg = merge(tcfg, ocfg, dcfg, auto_config(row["_files"]))
            pair = [(row["target"], row["path"], row["from"], row["to"])]
            w = os.path.join(work, row["key"])
            futs.append((row, cfg, ex.submit(run_fixture, w, row["_files"], row["links"],
                                             [(cfg, pair)])))
        for row, cfg, fut in futs:
            res = fut.result()[0][0]
            rec = {k: row[k] for k in ("key", "test", "what", "want", "target", "owner", "path",
                                       "from", "to", "files")}
            rec.update(res)
            rec["head_drops"] = bool(row["head_before"] and not row["head_after"])
            out.append(rec)
    return out


def obligation_rows():
    rows = []
    for lang, sub in (("c", "c"), ("cpp", "cpp")):
        for line in open(os.path.join(FIX, sub, "obligations.jsonl")):
            r = json.loads(line)
            if re.match(r"^[2-4]\d_", r["case"]):
                r["_lang"] = lang
                rows.append(r)
    return rows


def label_inline(export, jobs, work):
    byfile = collections.defaultdict(list)
    for r in obligation_rows():
        byfile[(r["_lang"], r["case"])].append(r)
    out = []
    with concurrent.futures.ThreadPoolExecutor(jobs) as ex:
        futs = []
        for (lang, case), rows in sorted(byfile.items()):
            name = case + ("" if lang == "cpp" else ".c")
            path = os.path.join(export, "inline", lang, name)
            if not os.path.exists(path):
                print("missing inline export", path, file=sys.stderr)
                continue
            files = {name: open(path, "rb").read()}
            groups = collections.OrderedDict()
            for r in rows:
                cfg = merge(auto_config(files, r.get("owner")), ROW_CONFIG.get((case, r["id"])))
                k = json.dumps([cfg["flags"], cfg["prelude"], cfg["missing"]], sort_keys=True)
                groups.setdefault(k, (cfg, []))[1].append(r)
            wants = [(cfg, [(name, r["path"], r["def_line"], r["use_line"]) for r in rs])
                     for cfg, rs in groups.values()]
            w = os.path.join(work, "inline_" + lang + "_" + case)
            futs.append((list(groups.values()), ex.submit(run_fixture, w, files, {}, wants)))
        for groups, fut in futs:
            for (cfg, rs), res in zip(groups, fut.result()):
                for r, o in zip(rs, res):
                    rec = {"lang": r["_lang"], "case": r["case"], "id": r["id"],
                           "def_line": r["def_line"], "use_line": r["use_line"],
                           "keep": r["keep"], "owner": r.get("owner")}
                    rec.update(o)
                    rec["head_drops"] = o["head_drops_cli"]
                    rec["want"] = "kept" if r["keep"] else "pruned"
                    out.append(rec)
    return out


# Rows with no truth basis by decision (DECISIONS.md OD-9, TEST-FORM section 5), by test name.
NO_BASIS = {
    "mf3_one_of_two_definitions_in_two_files":
        "two definitions of one name in two translation units: no single program",
    "boundary_g2_sol3_03_cxx20_header_unit_import":
        "a header-unit import needs a precompiled unit; the lexer-only oracle cannot express it",
}


def load_overrides():
    """`oracle/overrides.json`: {"<case>:<id>" (inline row) or "<key>" (pair row): {"assert",
    "receipt"}}. A runtime receipt (`xarm_receipts.py`) outranks an oracle verdict of UNJUDGED
    or XOFF that an expansion mis-parse caused (tree-sitter reads the post-expansion text
    wrongly, so the oracle's own parse fails); it never outranks CONFIRMED or CONTRADICTED."""
    p = os.path.join(FIX, "oracle", "overrides.json")
    return json.load(open(p)) if os.path.exists(p) else {}


def derive(pair_rows, inline_rows):
    """Adds `assert` to every record (rules: XARM.md, 'Derived assertions')."""
    def pair_id(r):
        return (r["test"].rsplit("::", 1)[0], r["target"], r["owner"], r["path"], r["from"],
                r["to"])
    overrides = load_overrides()
    witness = {}
    for r in sorted(pair_rows + inline_rows, key=lambda r: r.get("key") or r["case"] + r["id"]):
        if r["boundary"] and r["head_drops"] and r["assumption"] not in (None, "A-inputs") \
                and r["oracle"] == "CONTRADICTED":
            witness.setdefault(r["assumption"], id(r))
    for r in pair_rows + inline_rows:
        v = r["oracle"]
        if r["boundary"]:
            if r["assumption"] == "A-inputs":
                a = "kept" if (r.get("seen_input") or not r["head_drops"]) else "pinned_hole"
            elif r["head_drops"] and witness.get(r["assumption"]) == id(r):
                a = "pinned_hole"
            else:
                a = "guide"
        elif v == "CONTRADICTED":
            a = "kept"
        elif v == "CONFIRMED":
            a = "guide"
        else:
            a = "no_basis"
        why = NO_BASIS.get(r.get("test", "").split("::")[-1])
        if why:
            a = "no_basis"
            r["basis"] = "no truth basis by decision: " + why
        ov = overrides.get(r["key"]) if "key" in r else overrides.get(r["case"] + ":" + r["id"])
        if ov and v in ("UNJUDGED", "XOFF"):
            a = ov["assert"]
            r["basis"] = "receipt: %s (oracle %s: %s)" % (ov["receipt"], v, r["basis"])
            if ov.get("assumption"):
                r["assumption"] = ov["assumption"]
        r["assert"] = a
        r["validity_control"] = False
    # a control guards test validity when a must-keep row of the same module and pair holds at
    # least its files: the control is that row minus (or with other) macro-defining files, and
    # the twin is red for the right reason only if the control prunes
    kept_rows = collections.defaultdict(list)
    for r in pair_rows:
        if r["assert"] == "kept":
            kept_rows[pair_id(r)].append(set(r["files"]))
    for r in pair_rows:
        # A control is asserted only while head drops its pair: once an owner is refused (a
        # recall cost the design accepts) the twin's red-for-the-right-reason argument is moot
        # and the control stays a guide (#829 task 9.2).
        if r["want"] == "pruned" and r["assert"] == "guide" and r["oracle"] == "CONFIRMED" \
                and r.get("head_drops") \
                and any(set(r["files"]) <= k for k in kept_rows.get(pair_id(r), [])):
            r["assert"], r["validity_control"] = "dropped", True


def write_jsonl(path, rows):
    with open(path, "w") as f:
        for r in rows:
            f.write(json.dumps(r, sort_keys=True) + "\n")


def cmd_label(a):
    work = a.work or os.path.join(a.export, "work")
    pr = label_pair_rows(a.export, a.jobs, work)
    ir = label_inline(a.export, a.jobs, work)
    derive(pr, ir)
    odir = os.path.join(FIX, "oracle")
    os.makedirs(odir, exist_ok=True)
    write_jsonl(os.path.join(odir, "pair_rows.jsonl"), sorted(pr, key=lambda r: r["key"]))
    write_jsonl(os.path.join(odir, "inline_rows.jsonl"),
                sorted(ir, key=lambda r: (r["lang"], r["case"], r["id"], r["def_line"],
                                          r["use_line"])))
    for name, rows in (("pair rows", pr), ("inline rows", ir)):
        print(name, len(rows), dict(collections.Counter(r["oracle"] for r in rows)),
              dict(collections.Counter(r["assert"] for r in rows)))


def cmd_derive(a):
    """Re-derives `assert` from the stored verdicts (no clang): after a policy change."""
    odir = os.path.join(FIX, "oracle")
    pr = [json.loads(l) for l in open(os.path.join(odir, "pair_rows.jsonl"))]
    ir = [json.loads(l) for l in open(os.path.join(odir, "inline_rows.jsonl"))]
    derive(pr, ir)
    write_jsonl(os.path.join(odir, "pair_rows.jsonl"), pr)
    write_jsonl(os.path.join(odir, "inline_rows.jsonl"), ir)
    print(dict(collections.Counter(r["assert"] for r in pr + ir)))


def main():
    if sys.argv[1:2] == ["corpus"]:
        import xarm_l3  # L3 corpus differentials (XARM.md)
        return xarm_l3.main(sys.argv[2:])
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    sub = ap.add_subparsers(dest="cmd", required=True)
    pl = sub.add_parser("label")
    pl.add_argument("export")
    pl.add_argument("--jobs", type=int, default=4)
    pl.add_argument("--work")
    pl.set_defaults(fn=cmd_label)
    pd = sub.add_parser("derive")
    pd.set_defaults(fn=cmd_derive)
    a = ap.parse_args()
    a.fn(a)


if __name__ == "__main__":
    main()
