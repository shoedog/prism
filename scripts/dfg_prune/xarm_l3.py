#!/usr/bin/env python3
"""L3 corpus differentials for #829 (TEST-FORM section 4, L3). Python 3.9, stdlib only.

  xarm_l3.py sweep CORPUS --out DIR --tag TAG [--prism BIN] [--jobs N]
      D2: every pair the rules drop is judged against the expansion (T, Tb, X, Xb prism runs of
      BIN; the expansion text is made once per DIR and reused by every tag). Writes
      DIR/TAG/{rows.tsv,triage.json,summary.json}, the per-run edges, owner states and traces.
  xarm_l3.py compare DIR --base TAG --head TAG
      D1 (monotonicity: the head's edge multiset contains the base's) and D3 (the recall
      ledger: owners newly off and pairs restored, per gate reason, split CONFIRMED /
      CONTRADICTED / UNJUDGED by the base's X verdicts). Run `sweep` once per binary.
  xarm_l3.py parity-langs REPO... --base-bin B --head-bin H --out DIR
      D4a: the edge set of a repository of another language is identical under both binaries.
  xarm_l3.py cache-modes --repo R --prism BIN --out DIR
      D4b: cold, full-hit and partial-hit builds of a C repository give identical edges.
  xarm_l3.py corpora                                 list the corpus table

Corpora are read-only and are never built or run: the files the census reads are copied to
DIR/T (and DIR/Tb with the census-poison file that turns U and D off), clang only lexes and
preprocesses (`-fsyntax-only -Xclang -dump-tokens`), outside headers that do not exist are
stubbed empty, and every in-repo header directory goes on `-idirafter` (a bundled `stdint.h`
must not shadow the system one). See XARM.md for what the verdicts mean.
"""
import argparse
import bisect
import collections
import concurrent.futures as cf
import gzip
import hashlib
import json
import os
import re
import shutil
import subprocess
import sys
import time

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import xarm_core as core  # noqa: E402

TABLE = os.path.join(HERE, "l3_corpora.json")
MAIN_EXT = (".c", ".cc", ".cpp", ".cxx")


def corpora():
    return {k: v for k, v in json.load(open(TABLE)).items() if not k.startswith("_")}


def expand_path(p):
    return os.path.realpath(os.path.expanduser(p))


# ---------------------------------------------------------------- corpus files
def collect(root):
    """{rel: bytes} of the files the census reads (C-like extensions), no symlinks, no .git."""
    files = {}
    for dp, dn, fs in os.walk(root):
        dn[:] = [d for d in dn if d != ".git"]
        for f in fs:
            p = os.path.join(dp, f)
            rel = os.path.relpath(p, root)
            if os.path.islink(p) or "eval/secbench" in rel:
                continue
            if f.lower().endswith(tuple(core.CLIKE)):
                with open(p, "rb") as fh:
                    files[rel] = fh.read()
    return files


def in_scope(rel, scope):
    return any(s == "." or rel == s or rel.startswith(s.rstrip("/") + "/") for s in scope)


def mains_of(files, scope):
    return sorted(r for r in files if r.endswith(MAIN_EXT) and in_scope(r, scope))


def write_tree(files, dest):
    shutil.rmtree(dest, ignore_errors=True)
    for rel, data in files.items():
        p = os.path.join(dest, rel)
        os.makedirs(os.path.dirname(p), exist_ok=True)
        with open(p, "wb") as f:
            f.write(data)


def header_dirs(root_t, files):
    dirs = sorted({os.path.join(root_t, os.path.dirname(r)) for r in files
                   if r.lower().endswith((".h", ".hpp", ".hh", ".hxx", ".inc"))}, key=len)
    return [x for d in dirs for x in ("-idirafter", d)]


def prepare_config(cfg, outdir):
    """Writes the corpus's config files under `outdir` (never into the corpus) -> {name: sha}."""
    cdir = os.path.join(outdir, "config")
    shutil.rmtree(cdir, ignore_errors=True)
    os.makedirs(cdir)
    shas = {}
    for name, spec in (cfg.get("config") or {}).items():
        src = expand_path(spec["from"])
        shutil.copy(src, os.path.join(cdir, name))
        with open(src, "rb") as f:
            shas[name] = {"from": spec["from"], "sha256": hashlib.sha256(f.read()).hexdigest()}
    return cdir, shas


# ---------------------------------------------------------------- tokenization
def prestub(src, flags, cwd, stubdir):
    """One `clang -M -MG` pass lists every include that does not exist (assumed generated, no
    failure); each becomes an empty stub. The stubs hold no includes, so one pass is enough."""
    cmd = core.clang_cmd(src, flags + ["-idirafter", stubdir], ["-M", "-MG"])
    cmd = [c for c in cmd if c != "-fsyntax-only"]
    p = subprocess.run(cmd, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, cwd=cwd,
                       timeout=600)
    made = []
    for dep in p.stdout.decode("utf-8", "replace").replace("\\\n", " ").split()[1:]:
        if dep.startswith("/") or ".." in dep.split("/"):
            continue
        if os.path.exists(os.path.join(cwd, dep)) or os.path.exists(os.path.join(stubdir, dep)):
            continue
        sp = os.path.join(stubdir, dep)
        os.makedirs(os.path.dirname(sp), exist_ok=True)
        open(sp, "w").close()
        made.append(dep)
    return made


def tokenize_one(src, flags, cwd, stubdir):
    made = prestub(src, flags, cwd, stubdir)
    rc, body, err, stubbed = core.tokenize_with_stubs(src, flags, cwd, stubdir)
    return rc, body, err, made + stubbed


def error_class(err):
    err = re.sub(r"^\S+?:\d+:\d+: ", "", err)
    err = re.sub(r"'[^']*'", "'..'", err)
    return err[:90] or "?"


def tokenize_all(T, mains, flags, stubdir, jobs):
    os.makedirs(stubdir, exist_ok=True)
    out, failed, stubbed = {}, {}, set()

    def one(rel):
        return rel, tokenize_one(os.path.join(T, rel), flags, T, stubdir)

    with cf.ThreadPoolExecutor(jobs) as ex:
        for rel, (rc, body, err, stubs) in ex.map(one, mains):
            stubbed.update(stubs)
            if rc != 0:
                failed[rel] = error_class(err)
            else:
                out[rel] = body
    return out, failed, stubbed


# ---------------------------------------------------------------- prism runs
def run_trace(prism, repo, tag, outdir):
    """prism dfg-stats --edges on `repo` -> {edges, owners, uses, drops, seconds}."""
    errp = os.path.join(outdir, tag + ".err")
    t0 = time.time()
    with open(errp, "w") as err:
        p = subprocess.run([prism, "nav", "--no-cache", "dfg-stats", "--repo", repo, "--edges"],
                           stdout=subprocess.PIPE, stderr=err, text=True, timeout=7200,
                           env=dict(os.environ, PRISM_C_PRUNE_TRACE="1"))
    edges = collections.Counter()
    for line in p.stdout.splitlines():
        if not line.startswith('{"from"'):
            continue
        e = json.loads(line)
        f, t = e["from"], e["to"]
        path = ".".join([f["path"]["base"]] + f["path"].get("fields", []))
        edges[(f["file"], path, f["line"], t["line"])] += 1
    owners, uses, drops = {}, {}, {}
    for line in open(errp, errors="replace"):
        if line.startswith("c-prune-owner"):
            parts = line.rstrip("\n").split("\t")
            try:
                owners[(parts[-2], int(parts[-1]))] = parts[1]
            except ValueError:
                pass
        elif line.startswith("c-prune-use\t"):
            parts = line.rstrip("\n").split("\t")
            if len(parts) >= 5:
                uses.setdefault((parts[1], int(parts[2]), parts[3]), parts[4])
        elif line.startswith("c-prune-drop\t"):
            parts = line.rstrip("\n").split("\t")
            if len(parts) >= 5:
                drops.setdefault((parts[2], int(parts[3]), parts[4]), parts[1])
    return {"edges": edges, "owners": owners, "uses": uses, "drops": drops,
            "seconds": round(time.time() - t0, 1), "stdout_bytes": len(p.stdout)}


def save_run(run, path):
    with gzip.open(path, "wt") as f:
        json.dump({"edges": [list(k) + [v] for k, v in run["edges"].items()],
                   "owners": [list(k) + [v] for k, v in run["owners"].items()],
                   "uses": [list(k) + [v] for k, v in run["uses"].items()],
                   "drops": [list(k) + [v] for k, v in run["drops"].items()],
                   "seconds": run["seconds"]}, f)


def load_run(path):
    with gzip.open(path, "rt") as f:
        d = json.load(f)
    return {"edges": collections.Counter({tuple(r[:4]): r[4] for r in d["edges"]}),
            "owners": {(r[0], r[1]): r[2] for r in d["owners"]},
            "uses": {(r[0], r[1], r[2]): r[3] for r in d["uses"]},
            "drops": {(r[0], r[1], r[2]): r[3] for r in d["drops"]},
            "seconds": d["seconds"]}


def digest(edges):
    h = hashlib.sha256()
    for k in sorted(edges):
        h.update(repr((k, edges[k])).encode())
    return h.hexdigest()[:16]


class OwnerIndex:
    """core.owner_state over a run's owners, by bisection (the linear scan is quadratic on a
    corpus with tens of thousands of pairs)."""

    def __init__(self, owners):
        by = collections.defaultdict(list)
        for (f, start), st in owners.items():
            by[f].append((start, st))
        self.by = {f: sorted(v) for f, v in by.items()}
        self.starts = {f: [s for s, _ in v] for f, v in self.by.items()}

    def state(self, file, line):
        starts = self.starts.get(file)
        if not starts:
            return "none"
        i = bisect.bisect_right(starts, line) - 1
        return self.by[file][i][1] if i >= 0 else "none"


# ---------------------------------------------------------------- verdicts and triage
def dropped_pairs(T, Tb):
    return sorted(k for k in Tb["edges"] if T["edges"].get(k, 0) < Tb["edges"][k])


def verdict(d, T, Tb, X, Xb, failed, mains, xidx=None):
    """-> (verdict, detail) of a pair dropped on T (core.verdict, over saved runs)."""
    file, path, a, b = d
    if file not in mains:
        return "OUT_OF_SCOPE", ""
    if file in failed:
        return "UNJUDGED", "tokenize-failed: " + failed[file]
    bx = Xb["edges"].get(d, 0)
    if bx == 0:
        return "UNJUDGED", "no-pair-in-expansion"
    if X["edges"].get(d, 0) < bx:
        return "CONFIRMED", ""
    st = (xidx or OwnerIndex(X["owners"])).state(file, min(a, b))
    return ("CONTRADICTED", "") if st.startswith("on") else ("XOFF", st)


def x_reason(d, X, xidx=None):
    """The X arm's first refusal reason at the pair's use line (then def line)."""
    file, path, a, b = d
    base = path.split(".")[0]
    for line in (b, a):
        r = X["uses"].get((file, line, base))
        if r:
            return r
    return "kept-without-trace(%s)" % (xidx or OwnerIndex(X["owners"])).state(file, min(a, b))


IDENT = re.compile(r"[A-Za-z_]\w*")


class Sources:
    """Source and expansion lines of the files of a sweep, for naming the macros a pair spans."""

    def __init__(self, tdir, xdir):
        self.tdir, self.xdir, self.cache = tdir, xdir, {}

    def lines(self, base, rel):
        k = (base, rel)
        if k not in self.cache:
            try:
                with open(os.path.join(base, rel), errors="replace") as f:
                    self.cache[k] = f.read().split("\n")
            except OSError:
                self.cache[k] = []
        return self.cache[k]

    def macros_between(self, rel, a, b):
        """Identifiers spelled on the source lines of the pair that are absent from the
        expansion's lines: the macro names (and keywords the headers rewrite) it spans."""
        lo, hi = min(a, b), max(a, b)
        src = self.lines(self.tdir, rel)[lo - 1:hi]
        exp = self.lines(self.xdir, rel)[lo - 1:hi]
        have = set(IDENT.findall("\n".join(exp)))
        text = "\n".join(src)
        text = re.sub(r"/\*.*?\*/", " ", text, flags=re.S)
        text = re.sub(r"(?m)^\s*#.*$", " ", text)
        text = re.sub(r'//[^\n]*|"(?:[^"\\\n]|\\.)*"|\'(?:[^\'\\\n]|\\.)*\'', " ", text)
        names = []
        for n in IDENT.findall(text):
            if n not in have and n not in names:
                names.append(n)
        return names


def rule_of(d, T):
    file, path, a, b = d
    base = path.split(".")[0]
    return T["drops"].get((file, b, base)) or T["drops"].get((file, a, base)) or "?"


def sweep(name, out, tag, prism, jobs):
    cfgs = corpora()
    cfg = cfgs[name]
    root = expand_path(cfg["root"])
    out = os.path.abspath(out)
    os.makedirs(out, exist_ok=True)
    tdir = os.path.join(out, tag)
    os.makedirs(tdir, exist_ok=True)
    t0 = time.time()
    files = collect(root)
    mains = mains_of(files, cfg["scope"])
    T, Tb = os.path.join(out, "T"), os.path.join(out, "Tb")
    if not os.path.isdir(T):
        write_tree(files, T)
        write_tree(files, Tb)
        with open(os.path.join(Tb, "zz_poison_829.c"), "w") as f:
            f.write(core.POISON)
    cdir, shas = prepare_config(cfg, out)
    flags = ([f.replace("{CONFIG}", cdir) for f in cfg["flags"]] + header_dirs(T, files)
             + cfg.get("flags_late", []))
    xdir, xbdir = os.path.join(out, "X"), os.path.join(out, "Xb")
    meta_p = os.path.join(out, "expansion.json")
    if not os.path.isdir(xdir):
        bodies, failed, stubbed = tokenize_all(T, mains, flags, os.path.join(out, "stubs"), jobs)
        write_tree({r: b.encode("utf-8", "replace") for r, b in bodies.items()}, xdir)
        shutil.copytree(xdir, xbdir)
        with open(os.path.join(xbdir, "zz_poison_829.c"), "w") as f:
            f.write(core.POISON)
        json.dump({"failed": failed, "stubbed": sorted(stubbed), "mains": len(mains),
                   "tokenize_seconds": round(time.time() - t0)}, open(meta_p, "w"), indent=1)
    meta = json.load(open(meta_p))
    failed = meta["failed"]
    runs = {}
    for arm, d in (("T", T), ("Tb", Tb), ("X", xdir), ("Xb", xbdir)):
        runs[arm] = run_trace(prism, d, arm + "." + tag, tdir)
        save_run(runs[arm], os.path.join(tdir, arm + ".json.gz"))
    pairs = dropped_pairs(runs["T"], runs["Tb"])
    mainset = set(mains)
    xidx = OwnerIndex(runs["X"]["owners"])
    srcs = Sources(T, xdir)
    counts, rows = collections.Counter(), []
    triage = collections.defaultdict(lambda: collections.Counter())
    samples = collections.defaultdict(list)
    for d in pairs:
        v, why = verdict(d, runs["T"], runs["Tb"], runs["X"], runs["Xb"], failed, mainset, xidx)
        rule = rule_of(d, runs["T"])
        counts[v] += 1
        counts["rule:" + rule] += 1
        macros = ""
        if v in ("CONTRADICTED", "CONFIRMED"):
            names = srcs.macros_between(d[0], d[2], d[3])
            macros = ",".join(names[:6])
            if names:
                counts["touched:" + v] += 1
        if v == "CONTRADICTED":
            reason = x_reason(d, runs["X"], xidx) + "; macros: " + macros
        elif v == "XOFF":
            reason = why
        elif v == "UNJUDGED":
            reason = why.split(":")[0] if not why.startswith("tokenize") else "tokenize-failed"
        else:
            reason = ""
        if reason:
            triage[v][reason] += 1
            if len(samples[(v, reason)]) < 4:
                samples[(v, reason)].append(list(d))
        rows.append((v, reason, rule) + d + (macros,))
    with open(os.path.join(tdir, "rows.tsv"), "w") as f:
        for r in sorted(rows, key=str):
            f.write("\t".join(map(str, r)) + "\n")
    states = collections.Counter(s.split(":")[0] + (":" + s.split(":")[1] if ":" in s else "")
                                 for s in runs["T"]["owners"].values())
    fail_classes = collections.Counter(failed.values())
    # pairs kept on T and dropped on X: an upper bound on the recall headroom of a design
    pairset = set(pairs)
    headroom = sum(1 for k, n in runs["Tb"]["edges"].items()
                   if k[0] in mainset and k not in pairset
                   and runs["X"]["edges"].get(k, 0) < runs["Xb"]["edges"].get(k, 0)
                   and runs["T"]["edges"].get(k, 0) >= n)
    summary = {
        "corpus": name, "tag": tag, "prism": prism, "scope": cfg["scope"],
        "config": shas, "mains": len(mains), "tokenized": len(mains) - len(failed),
        "failed": len(failed), "failed_classes": fail_classes.most_common(8),
        "stubbed_headers": len(meta["stubbed"]), "edges_T": sum(runs["T"]["edges"].values()),
        "edges_Tb": sum(runs["Tb"]["edges"].values()), "dropped_pairs": len(pairs),
        "dropped_pairs_in_scope": sum(1 for d in pairs if d[0] in mainset),
        "verdicts": {k: v for k, v in counts.items() if not k.startswith(("rule:", "touched:"))},
        "pairs_spanning_a_macro": {k[8:]: v for k, v in counts.items() if k.startswith("touched:")},
        "rules": {k[5:]: v for k, v in counts.items() if k.startswith("rule:")},
        "owner_states_T": states.most_common(), "headroom_kept_on_T_dropped_on_X": headroom,
        "seconds": {"run": {a: r["seconds"] for a, r in runs.items()},
                    "tokenize": meta.get("tokenize_seconds"), "total": round(time.time() - t0)},
        "edge_digest_T": digest(runs["T"]["edges"]), "edge_digest_Tb": digest(runs["Tb"]["edges"]),
    }
    json.dump(summary, open(os.path.join(tdir, "summary.json"), "w"), indent=1)
    json.dump({"triage": {v: dict(c) for v, c in triage.items()},
               "samples": {"%s|%s" % k: v for k, v in samples.items()}},
              open(os.path.join(tdir, "triage.json"), "w"), indent=1)
    print(json.dumps(summary, indent=1))
    return summary


# ---------------------------------------------------------------- D1 and D3
def compare(out, base_tag, head_tag):
    out = os.path.abspath(out)
    def ld(tag, arm):
        return load_run(os.path.join(out, tag, arm + ".json.gz"))
    bT, bTb, bX, bXb = (ld(base_tag, a) for a in ("T", "Tb", "X", "Xb"))
    hT, hTb = ld(head_tag, "T"), ld(head_tag, "Tb")
    meta = json.load(open(os.path.join(out, "expansion.json")))
    failed = meta["failed"]
    judged = set(failed)
    for dp, _, fs in os.walk(os.path.join(out, "X")):
        judged.update(os.path.relpath(os.path.join(dp, f), os.path.join(out, "X")) for f in fs)
    # D1: head's edges contain main's
    missing = [(k, n, hT["edges"].get(k, 0)) for k, n in bT["edges"].items()
               if hT["edges"].get(k, 0) < n]
    added = sum(1 for k, n in hT["edges"].items() if bT["edges"].get(k, 0) < n)
    d1 = {"base_edges": sum(bT["edges"].values()), "head_edges": sum(hT["edges"].values()),
          "violations": len(missing), "examples": [list(m[0]) + [m[1], m[2]] for m in missing[:5]],
          "head_adds_edges": added}
    # D3: ledger
    pb = set(dropped_pairs(bT, bTb))
    ph = set(dropped_pairs(hT, hTb))
    restored = sorted(pb - ph)
    newly_dropped = sorted(ph - pb)
    owners_new_off = collections.Counter()
    off_files = set()
    for k, st in hT["owners"].items():
        if st.startswith("off") and not bT["owners"].get(k, "off").startswith("off"):
            owners_new_off[st] += 1
            off_files.add(k[0])
    ledger = collections.defaultdict(lambda: collections.Counter())
    hidx, xidx = OwnerIndex(hT["owners"]), OwnerIndex(bX["owners"])
    for d in restored:
        st = hidx.state(d[0], min(d[2], d[3]))
        reason = st if st.startswith("off") else "on(" + x_reason_head(d, hT) + ")"
        v = verdict_loose(d, bT, bTb, bX, bXb, failed, xidx, judged)
        ledger[reason][v] += 1
    d3 = {"base_dropped": len(pb), "head_dropped": len(ph), "restored": len(restored),
          "newly_dropped_on_head": len(newly_dropped),
          "owners_newly_off": dict(owners_new_off),
          "restored_by_reason": {r: dict(c) for r, c in ledger.items()},
          "restored_total_by_verdict": dict(collections.Counter(
              v for c in ledger.values() for v, n in c.items() for _ in range(n)))}
    res = {"d1": d1, "d3": d3}
    json.dump(res, open(os.path.join(out, "compare-%s-%s.json" % (base_tag, head_tag)), "w"),
              indent=1)
    print(json.dumps(res, indent=1))
    return res


def x_reason_head(d, hT):
    base = d[1].split(".")[0]
    return hT["uses"].get((d[0], d[3], base)) or hT["uses"].get((d[0], d[2], base)) or "kept"


def verdict_loose(d, T, Tb, X, Xb, failed, xidx, judged):
    file = d[0]
    if file not in judged:
        return "OUT_OF_SCOPE"
    if file in failed:
        return "UNJUDGED"
    bx = Xb["edges"].get(d, 0)
    if bx == 0:
        return "UNJUDGED"
    if X["edges"].get(d, 0) < bx:
        return "CONFIRMED"
    st = xidx.state(file, min(d[2], d[3]))
    return "CONTRADICTED" if st.startswith("on") else "XOFF"


# ---------------------------------------------------------------- D4
def parity_langs(repos, base_bin, head_bin, out):
    out = os.path.abspath(out)
    os.makedirs(out, exist_ok=True)
    res = []
    for repo in repos:
        repo = expand_path(repo)
        rows = {}
        for tag, b in (("base", base_bin), ("head", head_bin), ("base2", base_bin)):
            r = run_trace(b, repo, "par-" + os.path.basename(repo) + "." + tag, out)
            rows[tag] = {"edges": sum(r["edges"].values()), "digest": digest(r["edges"]),
                         "seconds": r["seconds"]}
        rows["repo"] = os.path.basename(repo)
        rows["identical"] = rows["base"]["digest"] == rows["head"]["digest"]
        rows["deterministic"] = rows["base"]["digest"] == rows["base2"]["digest"]
        res.append(rows)
        print(json.dumps(rows))
    json.dump(res, open(os.path.join(out, "parity-langs.json"), "w"), indent=1)
    return res


def cache_run(prism, repo, cache, tag, out):
    t0 = time.time()
    with open(os.path.join(out, tag + ".err"), "w") as err:
        p = subprocess.run([prism, "nav"] + (["--cache-dir", cache] if cache else ["--no-cache"])
                           + ["dfg-stats", "--repo", repo, "--edges"], stdout=subprocess.PIPE,
                           stderr=err, text=True, timeout=7200)
    edges = collections.Counter()
    for line in p.stdout.splitlines():
        if line.startswith('{"from"'):
            e = json.loads(line)
            f, t = e["from"], e["to"]
            rest = json.dumps({k: v for k, v in e.items() if k not in ("from", "to")},
                              sort_keys=True)
            edges[(f["file"], ".".join([f["path"]["base"]] + f["path"].get("fields", [])),
                   f["line"], t["line"], rest)] += 1
    stats = [l for l in p.stdout.splitlines() if not l.startswith('{"from"')] + \
        [l for l in open(os.path.join(out, tag + ".err"), errors="replace").read().splitlines()
         if l.startswith(("dfg_", "{", "}", '"'))]
    note = [re.sub(r"/\S+", "<dir>", l) for l in open(os.path.join(out, tag + ".err"),
                                                      errors="replace").read().splitlines()
            if l.startswith("nav cache")]
    return {"cache": note[:2], "edges": sum(edges.values()), "digest": digest(edges), "seconds": round(
        time.time() - t0, 1), "stats_digest": hashlib.sha256("\n".join(stats).encode())
        .hexdigest()[:16]}


def cache_modes(repo, prism, out):
    """Cold, full-hit and partial-hit builds against the no-cache build of the same tree."""
    out = os.path.abspath(out)
    shutil.rmtree(out, ignore_errors=True)
    os.makedirs(out)
    src = expand_path(repo)
    tree = os.path.join(out, "tree")
    files = collect(src)
    write_tree(files, tree)
    cache = os.path.join(out, "cache")
    res = {"repo": os.path.basename(src), "files": len(files)}
    res["nocache"] = cache_run(prism, tree, None, "nocache", out)
    res["cold"] = cache_run(prism, tree, cache, "cold", out)
    res["hit"] = cache_run(prism, tree, cache, "hit", out)
    # partial hit 1: a source file changes (a new function: new defs and uses)
    target = sorted(r for r in files if r.endswith(".c") and r.count("/") <= 1)[0]
    with open(os.path.join(tree, target), "a") as f:
        f.write("\nstatic int xarm_cache_probe(int c) {\n    int x = 41;\n    if (c) {\n"
                "        x = 2;\n    }\n    return x;\n}\n")
    res["partial_source_file"] = target
    res["partial_source"] = cache_run(prism, tree, cache, "partial-src", out)
    res["partial_source_nocache"] = cache_run(prism, tree, None, "partial-src-nocache", out)
    # partial hit 2: a header gains a block macro (the census digest moves)
    hdr = sorted(r for r in files if r.endswith(".h") and r.count("/") <= 1)[0]
    with open(os.path.join(tree, hdr), "a") as f:
        f.write("\n#define XARM_OPEN_829 {\n#define XARM_CLOSE_829 }\n")
    res["partial_header_file"] = hdr
    res["partial_header"] = cache_run(prism, tree, cache, "partial-hdr", out)
    res["partial_header_nocache"] = cache_run(prism, tree, None, "partial-hdr-nocache", out)
    res["parity"] = {
        "cold==nocache": res["cold"]["digest"] == res["nocache"]["digest"],
        "hit==cold": res["hit"]["digest"] == res["cold"]["digest"],
        "partial_source==nocache": res["partial_source"]["digest"] ==
        res["partial_source_nocache"]["digest"],
        "partial_header==nocache": res["partial_header"]["digest"] ==
        res["partial_header_nocache"]["digest"],
        "stats cold==hit": res["cold"]["stats_digest"] == res["hit"]["stats_digest"],
    }
    json.dump(res, open(os.path.join(out, "cache-modes.json"), "w"), indent=1)
    print(json.dumps(res, indent=1))
    return res


def md_table(head, rows):
    out = ["| " + " | ".join(head) + " |", "|" + "---|" * len(head)]
    return out + ["| " + " | ".join(str(c) for c in r) + " |" for r in rows]


def report(root, names, tag):
    """Markdown tables of the sweeps under `root` (for L3-BASE.md)."""
    sums = {}
    for n in names:
        p = os.path.join(root, n, tag, "summary.json")
        if os.path.exists(p):
            sums[n] = json.load(open(p))
    L = ["### D2 coverage and verdicts (base binary)", ""]
    rows = []
    for n, s in sums.items():
        v = s["verdicts"]
        rows.append([n, s["mains"], s["tokenized"], s["stubbed_headers"], s["edges_T"],
                     s["dropped_pairs"],
                     s["rules"].get("U", 0), s["rules"].get("D", 0),
                     v.get("CONFIRMED", 0), v.get("CONTRADICTED", 0), v.get("XOFF", 0),
                     v.get("UNJUDGED", 0), v.get("OUT_OF_SCOPE", 0),
                     s.get("pairs_spanning_a_macro", {}).get("CONFIRMED", 0),
                     s["headroom_kept_on_T_dropped_on_X"]])
    L += md_table(["corpus", "mains", "tokenized", "stubbed hdrs", "edges (T)", "dropped pairs",
                   "by U", "by D", "CONFIRMED", "CONTRADICTED", "XOFF", "UNJUDGED",
                   "out of scope", "CONFIRMED spanning a macro", "kept on T, dropped on X"],
                  rows)
    L += ["", "### Owner states under the rules (T)", ""]
    rows = [[n, ", ".join("%s %d" % (k, c) for k, c in s["owner_states_T"])] for n, s in sums.items()]
    L += md_table(["corpus", "owners by state"], rows)
    L += ["", "### Triage (CONTRADICTED by the X arm's refusal reason; XOFF by off reason; "
          "UNJUDGED by cause)", ""]
    rows = []
    for n in sums:
        t = json.load(open(os.path.join(root, n, tag, "triage.json")))["triage"]
        for v in ("CONTRADICTED", "XOFF", "UNJUDGED"):
            for reason, c in sorted(t.get(v, {}).items(), key=lambda kv: -kv[1]):
                rows.append([n, v, reason, c])
    L += md_table(["corpus", "verdict", "reason", "pairs"], rows)
    L += ["", "### Tokenize failures", ""]
    rows = [[n, s["failed"], "; ".join("%s (%d)" % (k, c) for k, c in s["failed_classes"])]
            for n, s in sums.items()]
    L += md_table(["corpus", "files failed", "classes"], rows)
    L += ["", "### Runtimes (seconds)", ""]
    rows = [[n, s["seconds"]["tokenize"], s["seconds"]["run"]["T"], s["seconds"]["run"]["Tb"],
             s["seconds"]["run"]["X"], s["seconds"]["run"]["Xb"], s["seconds"]["total"]]
            for n, s in sums.items()]
    L += md_table(["corpus", "tokenize", "T run", "Tb run", "X run", "Xb run", "total"], rows)
    L += ["", "### Edge-set digests (base binary; a head must match for other languages)", ""]
    rows = [[n, s["edge_digest_T"], s["edge_digest_Tb"]] for n, s in sums.items()]
    L += md_table(["corpus", "T", "Tb"], rows)
    print("\n".join(L))


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    sub = ap.add_subparsers(dest="cmd", required=True)
    ps = sub.add_parser("sweep")
    ps.add_argument("corpus")
    ps.add_argument("--out", required=True)
    ps.add_argument("--tag", default="base")
    ps.add_argument("--prism", default=core.PRISM)
    ps.add_argument("--jobs", type=int, default=8)
    pc = sub.add_parser("compare")
    pc.add_argument("out")
    pc.add_argument("--base", default="base")
    pc.add_argument("--head", required=True)
    pl = sub.add_parser("parity-langs")
    pl.add_argument("repos", nargs="+")
    pl.add_argument("--base-bin", default=core.PRISM)
    pl.add_argument("--head-bin", default=core.PRISM)
    pl.add_argument("--out", required=True)
    pm = sub.add_parser("cache-modes")
    pm.add_argument("--repo", required=True)
    pm.add_argument("--prism", default=core.PRISM)
    pm.add_argument("--out", required=True)
    pr = sub.add_parser("report")
    pr.add_argument("root")
    pr.add_argument("names", nargs="+")
    pr.add_argument("--tag", default="base")
    sub.add_parser("corpora")
    a = ap.parse_args(argv)
    if a.cmd == "sweep":
        sweep(a.corpus, a.out, a.tag, os.path.abspath(a.prism), a.jobs)
    elif a.cmd == "compare":
        compare(a.out, a.base, a.head)
    elif a.cmd == "parity-langs":
        parity_langs(a.repos, os.path.abspath(a.base_bin), os.path.abspath(a.head_bin), a.out)
    elif a.cmd == "cache-modes":
        cache_modes(a.repo, os.path.abspath(a.prism), a.out)
    elif a.cmd == "report":
        report(a.root, a.names, a.tag)
    else:
        print(json.dumps(corpora(), indent=1))


if __name__ == "__main__":
    main()
