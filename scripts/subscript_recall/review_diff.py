#!/usr/bin/env python3
"""Seeded review parity for the lane 10 element-recall diagnostics.

`replay` freezes one-line diff seeds (store lines of the admission's candidate pairs, the upstream source line of
each stored value, matrix controls, a disconnected seed and a Rust control), binds the dispatch from
`prism --list-algorithms`, and runs `prism --no-cache --repo R --diff SEED.json --algorithm A --format F` for every
applicable reader on three arms: parent binary, head default and head with PRISM_ELEMENT_RECALL_TRACE=1. Parent vs
head default and head default vs trace-on must give identical complete output (findings, grades, severities, rules,
slices, diagrams). Binaries and source trees are bound to the `compare.py run` manifests; a mismatch, a refused run,
a reader missing from the listing or an unaccounted dispatch entry fails the replay.
"""

import argparse
import ast
import hashlib
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
from collections import Counter
from pathlib import Path

import compare
from compare import Refused, canon, sha256_file, tree_digest

SCHEMA = "review_diff/1"
ARMS = (("base", False), ("off", False), ("on", True))  # parent, head default, head trace-on
GRAPH = ("leftflow", "fullflow", "relevant", "conditioned", "barrier", "chop", "taint", "delta", "spiral",
         "circular", "vertical", "gradient", "provenance", "membrane", "echo")
CONTROL = ("originaldiff",)  # graph-independent
UNREPLAYED = dict({n: "reads ctx.files only, no CPG or DataFlow" for n in (
    "parentfunction", "thin", "quantum", "horizontal", "angle", "absence", "symmetry", "phantom", "resonance",
    "contract", "peer", "callback", "primitive")}, **{"3d": "needs git history, not a seeded fixture"})
MATCH = tuple(getattr(ast, n) for n in ("MatchAs", "MatchStar") if hasattr(ast, n))  # Python 3.10+
CLEAN = b'"CLEAN"'
NO_EDIT = {"formal": "no admitted RHS edit", "iteration": "bound iteration target", "with_alias": "bound alias",
           "except_alias": "bound alias", "match_capture": "bound capture", "compound": "compound write",
           "destructuring": "destructuring target", "chained_assign": "chained target",
           "no_initializer": "no initializer", "walrus": "walrus edit unproven", "other": "import or other Def"}


def jsonl(path):
    return [json.loads(line) for line in Path(path).read_text().splitlines() if line.strip()]


def sha(data):
    return hashlib.sha256(data if isinstance(data, bytes) else data.encode()).hexdigest()


# ---- Python source analysis (stdlib ast, independent of the product) ----

def line_of(src, off):
    return src.count(b"\n", 0, off) + 1


def abs_off(src, line, col):
    return sum(len(x) + 1 for x in src.split(b"\n")[:line - 1]) + col


def parse(src):
    try:
        return ast.parse(src)
    except (SyntaxError, ValueError):
        return None


def stores(tree):
    """`(statement, root name)` for each value-carrying statement with a subscript target."""
    out = []
    for n in ast.walk(tree) if tree else ():
        if isinstance(n, (ast.Assign, ast.AugAssign)) or (isinstance(n, ast.AnnAssign) and n.value is not None):
            for t in (n.targets if isinstance(n, ast.Assign) else [n.target]):
                root = t
                while isinstance(root, (ast.Subscript, ast.Attribute)):
                    root = root.value
                if isinstance(t, ast.Subscript) and isinstance(root, ast.Name):
                    out.append((n, root.id))
    return out


def defs_of(scope, name, line):
    """Textual Defs of `name` at or before `line` in `scope`: `(lineno, col, form, node)`."""
    parents = {c: p for p in ast.walk(scope) for c in ast.iter_child_nodes(p)}
    out = []
    if isinstance(scope, (ast.FunctionDef, ast.AsyncFunctionDef)):
        a = scope.args
        out += [(x.lineno, x.col_offset, "formal", x) for x in a.posonlyargs + a.args + a.kwonlyargs + [
            y for y in (a.vararg, a.kwarg) if y] if x.arg == name]
    for n in ast.walk(scope):
        if getattr(n, "lineno", line + 1) > line:
            continue
        if isinstance(n, ast.Name) and n.id == name and isinstance(n.ctx, ast.Store):
            p = parents.get(n)
            form = {ast.For: "iteration", ast.comprehension: "iteration", ast.withitem: "with_alias",
                    ast.AugAssign: "compound", ast.NamedExpr: "walrus", ast.Tuple: "destructuring",
                    ast.List: "destructuring", ast.Starred: "destructuring"}.get(type(p), "other")
            if isinstance(p, ast.Assign):
                form = "plain_assign" if p.targets == [n] else "chained_assign"
            elif isinstance(p, ast.AnnAssign):
                form = "plain_assign" if p.value is not None else "no_initializer"
            out.append((n.lineno, n.col_offset, form, p if form == "plain_assign" else n))
        elif isinstance(n, (ast.ExceptHandler,) + MATCH) and n.name == name:
            out.append((n.lineno, n.col_offset, "except_alias" if isinstance(n, ast.ExceptHandler)
                        else "match_capture", n))
        elif isinstance(n, (ast.Import, ast.ImportFrom)) and any((x.asname or x.name.split(".")[0]) == name
                                                                  for x in n.names):
            out.append((n.lineno, n.col_offset, "other", n))
    return sorted(out, key=lambda d: d[:2])


def upstream(src, w_line):
    """The Def feeding the value stored at `w_line`: that of the first contributing name by byte. CHOICE: the
    textually latest earlier Def reaches; more than one contributing name with a Def, or two Defs on the latest
    line, is reported ambiguous and the first is still selected."""
    tree = parse(src)
    found = [s for s, _ in stores(tree) if s.lineno == w_line]
    if not found:
        return dict(status="unparsed" if tree is None else "no_store")
    scope = tree
    for n in ast.walk(tree):
        if isinstance(n, (ast.FunctionDef, ast.AsyncFunctionDef)) and n.lineno <= w_line <= n.end_lineno:
            scope = n if scope is tree or n.lineno >= scope.lineno else scope
    names = sorted((n for n in ast.walk(found[0].value) if isinstance(n, ast.Name) and isinstance(n.ctx, ast.Load)),
                   key=lambda n: (n.lineno, n.col_offset))
    picks = [(n.id, ds[-1], sum(d[0] == ds[-1][0] for d in ds)) for n in names if (ds := defs_of(scope, n.id, w_line))]
    if not picks:
        return dict(status="missing")
    name, (line, _, form, node), tied = picks[0]
    return dict(status="ok", name=name, line=line, form=form, node=node, ambiguous=len(picks) > 1 or tied > 1)


def plain_edit(src, form, node):
    """Clean RHS-only edit of a plain one-line single-target assignment, or `(None, reason)`."""
    if form != "plain_assign":
        return None, NO_EDIT.get(form, form)
    v = node.value
    if node.lineno != node.end_lineno or v.lineno != v.end_lineno:
        return None, "multi-line initializer"
    return src[:abs_off(src, v.lineno, v.col_offset)] + CLEAN + src[abs_off(src, v.lineno, v.end_col_offset):], None


def noop_edit(src, stmt):
    """A store statement owning its line replaced by `pass`, or `(None, reason)`."""
    lines = src.split(b"\n")
    ls = lines[stmt.lineno - 1]
    if stmt.lineno != stmt.end_lineno or ls[:stmt.col_offset].strip() or ls[stmt.end_col_offset:].split(b"#")[0].strip():
        return None, "store shares its line with other code"
    lines[stmt.lineno - 1] = ls[:stmt.col_offset] + b"pass" + ls[stmt.end_col_offset:]
    return b"\n".join(lines), None


def delta_edits(src, s):
    """`(variant, edited bytes | None, reason)` per bound Def of an upstream seed, else one store no-op. Every
    bound Def is a row, so a formal stays in the Delta denominator."""
    out = []
    for (name, form), (node, _) in sorted(s["defs"].items()):
        out.append((f"delta@{name}",) + plain_edit(src, form, node))
    if not s["defs"]:
        stmt = next((x for x, _ in stores(parse(src)) if x.lineno == s["line"]), None)
        out.append(("delta",) + (noop_edit(src, stmt) if stmt else (None, "no store statement")))
    return [(v, d if d is not None and parse(d) else None, why or "invalid edit") for v, d, why in out]


# ---- seeds ----

def diff_spec(file, line):
    return json.dumps(dict(files=[dict(file_path=file, modify_type="Modified", diff_lines=[line])]), sort_keys=True)


class Seeds(dict):
    """Seeds deduplicated by (corpus, file, line), each with its links, roots, R lines and bound Defs."""

    def add(self, cls, corpus, file, line, link=None, root=None, r_line=None):
        s = self.setdefault((corpus, file, line), dict(
            corpus=corpus, file=file, line=line, classes=set(), links=set(), roots=set(), r_lines=set(), defs={},
            digest=sha(f"{corpus}\0{diff_spec(file, line)}")))
        s["classes"].add(cls)
        for key, val in (("links", link), ("roots", root), ("r_lines", r_line)):
            if val is not None:
                s[key].add(val)
        return s

    def link_upstream(self, cls, corpus, file, src, w_line, store, **kw):
        up = upstream(src, w_line)
        if up["status"] == "ok" and up["line"] != w_line:
            u = self.add(cls, corpus, file, up["line"], **kw)
            u["defs"][(up["name"], up["form"])] = (up["node"], store["digest"])
        return up


def pair_seeds(seeds, corpus, repo, pairs, rep):
    for p in pairs:
        src = (repo / p["file"]).read_bytes()
        if not (0 <= p["w"][0] < p["w"][1] <= len(src) and 0 <= p["r"][0] < p["r"][1] <= len(src)):
            raise Refused(f"pair {p['key']}: byte span outside {p['file']}")
        w_line, r_line = line_of(src, p["w"][0]), line_of(src, p["r"][0])
        root = next((r for s, r in stores(parse(src)) if s.lineno == w_line), None)
        store = seeds.add("store", corpus, p["file"], w_line, link=p["key"], root=root, r_line=r_line)
        up = seeds.link_upstream("upstream", corpus, p["file"], src, w_line, store, link=p["key"], root=root,
                                 r_line=r_line)
        rep["pairs"] += 1
        rep["missing"] += up["status"] != "ok"
        rep["ambiguous"] += bool(up.get("ambiguous"))
        rep["same_line"] += up["status"] == "ok" and up["line"] == w_line
        if up["status"] == "ok" and up["line"] != w_line:
            rep["upstream_lines"].add((corpus, p["file"], up["line"]))


def control_seeds(seeds, controls, limit, rep):
    """Matrix controls (P14 always, `limit` other Python cases in hash order) with their upstream seeds, one
    disconnected seed (a Python case without a store, else the first non-Rust case) and one Rust control."""
    files = sorted(p for p in controls.iterdir() if p.is_file())
    ranked = sorted((sha(f"{f.name}:{st.lineno}"), f.name) for f in files if f.suffix == ".py"
                    for st, _ in stores(parse(f.read_bytes())))
    keep = {n for _, n in ranked[:limit]} | {"P14.py"}
    first = lambda src: next((i + 1 for i, l in enumerate(src.decode(errors="replace").splitlines())
                              if l.strip() and not l.strip().startswith(("#", "//"))), None)
    for f in files:
        src = f.read_bytes()
        for st, root in stores(parse(src)) if f.name in keep else ():
            store = seeds.add("matrix", "CTRL", f.name, st.lineno, root=root)
            if seeds.link_upstream("matrix_upstream", "CTRL", f.name, src, st.lineno, store, root=root)["status"] == "ok":
                rep["controls_upstream"].add(f.name)
    rust = next((f for f in files if f.suffix == ".rs"), None)
    quiet = next((f for f in sorted(files, key=lambda g: (g.suffix != ".py", g.name))
                  if f.suffix != ".rs" and not stores(parse(f.read_bytes()))), None)
    for f, cls in ((rust, "rust_control"), (quiet, "disconnected")):
        if f:
            seeds.add(cls, "CTRL", f.name, first(f.read_bytes()) or 1)


# ---- dispatch, manifests, arms ----

def bind_dispatch(binaries, wanted):
    listed = {}
    for arm, b in binaries.items():
        p = subprocess.run([str(b), "--list-algorithms"], capture_output=True, text=True, check=False)
        if p.returncode != 0:
            raise Refused(f"{arm}: --list-algorithms failed")
        listed[arm] = (sorted(m.group(1) for m in re.finditer(r"^ {4}(\S+)\s{2,}", p.stdout, re.M)), sha(p.stdout))
    if listed["base"][0] != listed["off"][0]:
        raise Refused("parent and head list different algorithms")
    have = set(listed["off"][0])
    missing, unaccounted = sorted(set(wanted) - have), sorted(have - set(GRAPH) - set(CONTROL) - set(UNREPLAYED))
    if missing or unaccounted:
        raise Refused(f"dispatch: reader(s) missing from --list-algorithms {missing}; unaccounted entries {unaccounted}")
    return dict(listing_sha256=listed["off"][1], replayed=sorted(wanted),
                unreplayed={n: UNREPLAYED[n] for n in sorted(have & set(UNREPLAYED))})


def load_manifests(runs, corpus):
    ms = {}
    for arm, _ in ARMS:
        d = Path(runs) / corpus / arm
        try:
            m = json.loads((d / "manifest.json").read_text())
        except (OSError, ValueError):
            raise Refused(f"{corpus}/{arm}: no readable manifest")
        if (d / "FAILED").exists() or m.get("complete") is not True or m.get("schema") != compare.SCHEMA \
                or m.get("corpus") != corpus or m.get("trace") != compare.TRACE_BY_ARM[arm]:
            raise Refused(f"{corpus}/{arm}: incomplete or mismatched manifest")
        b = Path(m["binary"]["path"])
        if not b.is_file() or sha256_file(b) != m["binary"]["sha256"]:
            raise Refused(f"{corpus}/{arm}: binary does not match its manifest")
        ms[arm] = m
    if len({m["source"]["digest"] for m in ms.values()}) != 1 or len({m["repo"] for m in ms.values()}) != 1:
        raise Refused(f"{corpus}: arms are bound to different source trees")
    if (ms["off"]["binary"], ms["off"]["version"]) != (ms["on"]["binary"], ms["on"]["version"]):
        raise Refused(f"{corpus}: head default and trace-on arms used different binaries")
    return ms


def old_tree(repo, rel, data, store):
    """A hard-linked copy of `repo` whose file `rel` holds `data`; its path and tree digest."""
    dest = Path(tempfile.mkdtemp(prefix="old-", dir=store))
    for dirpath, dirs, names in os.walk(repo):
        dirs[:] = [d for d in dirs if d != ".git"]
        for n in names:
            src, dst = Path(dirpath, n), dest / Path(dirpath, n).relative_to(repo)
            dst.parent.mkdir(parents=True, exist_ok=True)
            try:
                os.link(src, dst)
            except OSError:
                shutil.copy2(src, dst)
    (dest / rel).unlink()
    (dest / rel).write_bytes(data)
    return dest, tree_digest(dest)[0]


def variants(algos, s):
    """`(name, algorithm, args)`; `args` is the reason when the reader has no input for this seed."""
    loc = f"{s['file']}:{s['line']}"
    for a in algos:
        if a == "chop":
            for r in sorted(s["r_lines"]) or [None]:
                yield ("chop@%s" % r, a, ["--chop-source", loc, "--chop-sink", f"{s['file']}:{r}"]) if r else (
                    "chop", None, "no associated R line")
        elif a == "conditioned":
            yield ("conditioned", a, ["--condition", f"{min(s['roots'])} != null"]) if s["roots"] else (
                "conditioned", None, "no named root")
        elif a == "taint":
            yield "taint", a, []
            yield "taint+return-flow", a, ["--taint-return-flow"]
        else:
            yield a, a, []


def summarize(doc):
    c = Counter()
    def walk(x):
        if isinstance(x, dict):
            if "severity" in x and "description" in x:
                c["findings"] += 1
                c["severity:" + str(x["severity"])] += 1
                c.update("rule:" + str(r.get("id")) for r in x.get("rules", []) if isinstance(r, dict))
            if "ruleId" in x:
                c["sarif_results"] += 1
            c.update({k: len(x[k]) for k in ("slices", "diagrams") if isinstance(x.get(k), list)})
            for v in x.values():
                walk(v)
        elif isinstance(x, list):
            for v in x:
                walk(v)
    walk(doc)
    return dict(sorted(c.items()))


def run_once(binary, repo, seed_file, algo, args, fmt, trace, timeout):
    env = {k: v for k, v in os.environ.items() if not k.startswith("PRISM_")}
    if trace:
        env["PRISM_ELEMENT_RECALL_TRACE"] = "1"
    cmd = [str(binary), "--no-cache", "--repo", str(repo), "--diff", str(seed_file), "--algorithm", algo,
           "--format", fmt, *args]
    p = subprocess.run(cmd, env=env, capture_output=True, cwd=tempfile.gettempdir(), timeout=timeout, check=False)
    return p.returncode, p.stdout, p.stderr[-400:].decode(errors="replace")


def compare_arms(outs):
    """`(status, detail)` over the arms' `(code, stdout, stderr)`; only complete, nonempty runs compare."""
    bad = [arm for (arm, _), (code, out, _) in zip(ARMS, outs) if code != 0 or not out.strip()]
    if bad:
        return "refused", f"arm(s) {bad} failed or produced no output"
    docs = []
    for _, out, _ in outs:
        try:
            docs.append(canon(json.loads(out)))
        except ValueError:
            docs.append(out)
    if docs[0] != docs[1]:
        return "mismatch", "parent vs head default differ"
    if docs[1] != docs[2]:
        return "mismatch", "head default vs trace-on differ"
    try:
        s = summarize(json.loads(outs[0][1]))
    except ValueError:
        return "equal", dict(bytes=len(outs[0][1]))
    return ("equal" if any(s.get(k) for k in ("findings", "sarif_results", "slices", "diagrams")) else "empty"), s


def replay(a):
    out, runs, adm = Path(a.out), Path(a.runs), Path(a.admission)
    if out.exists() and any(out.iterdir()):
        raise Refused(f"{out} is not empty")
    corpora = a.corpora or sorted(p.name for p in runs.iterdir() if (p / "base" / "manifest.json").is_file())
    if not corpora:
        raise Refused("no corpus runs to replay")
    pair_file = adm / "qualifying.jsonl" if (adm / "qualifying.jsonl").exists() else adm / "candidates.jsonl"
    pairs, ms = jsonl(pair_file), {c: load_manifests(runs, c) for c in corpora}
    binaries = {arm: Path(ms[corpora[0]][arm]["binary"]["path"]) for arm, _ in ARMS}
    for c in corpora:
        if any(ms[c][arm]["binary"]["sha256"] != ms[corpora[0]][arm]["binary"]["sha256"] for arm, _ in ARMS):
            raise Refused(f"{c}: binaries differ across corpora")
    algos = a.algorithms.split(",") if a.algorithms else list(GRAPH + CONTROL)
    dispatch = bind_dispatch(binaries, algos)
    rep = dict(pairs=0, missing=0, ambiguous=0, same_line=0, upstream_lines=set(), controls_upstream=set())
    seeds, repos, trees = Seeds(), {}, {}
    for c in corpora:
        repos[c] = Path(ms[c]["off"]["repo"])
        trees[c] = tree_digest(repos[c])[0]
        if trees[c] != ms[c]["off"]["source"]["digest"]:
            raise Refused(f"{c}: repository bytes differ from the manifests' source digest")
        pair_seeds(seeds, c, repos[c], sorted((p for p in pairs if p["corpus"] == c), key=lambda p: sha(p["key"]))[:a.sample], rep)
    if a.controls and Path(a.controls).is_dir():
        repos["CTRL"], trees["CTRL"] = Path(a.controls), tree_digest(a.controls)[0]
        control_seeds(seeds, repos["CTRL"], a.control_sample, rep)
    ordered = sorted(seeds.values(), key=lambda s: (s["corpus"], s["file"], s["line"]))
    (out / "seeds").mkdir(parents=True)
    for s in ordered:
        if not (repos[s["corpus"]] / s["file"]).is_file():
            raise Refused(f"seed file {s['file']} is not in the {s['corpus']} tree")
        (out / "seeds" / f"{s['digest']}.json").write_text(diff_spec(s["file"], s["line"]))
    (out / "seeds.jsonl").write_text("".join(canon(dict(
        {k: s[k] for k in ("digest", "corpus", "file", "line")}, classes=sorted(s["classes"]), links=sorted(s["links"]),
        roots=sorted(s["roots"]), r_lines=sorted(s["r_lines"]))) + "\n" for s in ordered))
    count, results, delta, scratch = Counter(), [], Counter(), Path(tempfile.mkdtemp(prefix="review-diff-old-"))
    try:
        for s in ordered:
            c, repo = s["corpus"], repos[s["corpus"]]
            src = (repo / s["file"]).read_bytes()
            seed_file = out / "seeds" / f"{s['digest']}.json"
            arm_bin = {arm: Path(ms[c][arm]["binary"]["path"]) if c in ms else binaries[arm] for arm, _ in ARMS}
            plan = [(n, al, ar, None) for n, al, ar in variants(algos, s) if al != "delta"]
            if "delta" in algos:
                plan += [("delta", "delta", None, e) for e in delta_edits(src, s)]
            for name, algo, args, edit in plan:
                old, kind = None, ""
                if edit:
                    name, data, why = edit
                    kind = "" if s["defs"] else "store_noop_"  # the Delta denominator is the bound upstream Defs
                    delta[kind + "bound"] += 1
                    if data is None:
                        delta[f"{kind}no_preimage:{why}"] += 1
                        results.append(dict(seed=s["digest"], variant=name, status="no_preimage", reason=why))
                        continue
                    delta[kind + "eligible"] += 1
                    old, old_digest = old_tree(repo, s["file"], data, scratch)
                    args = ["--old-repo", str(old)]
                if algo is None:
                    count["inapplicable"] += 1
                    results.append(dict(seed=s["digest"], variant=name, status="inapplicable", reason=args))
                    continue
                for fmt in a.formats.split(","):
                    outs = [run_once(arm_bin[arm], repo, seed_file, algo, args, fmt, trace, a.timeout)
                            for arm, trace in ARMS]
                    status, detail = compare_arms(outs)
                    count[status] += 1
                    delta[kind + "completed"] += bool(edit) and status in ("equal", "empty")
                    results.append(dict(
                        seed=s["digest"], variant=name, format=fmt, status=status, detail=detail,
                        input_sha256=sha(canon([s["digest"], algo, args if not old else old_digest, fmt, trees[c]])),
                        outputs=[sha(o[1]) for o in outs]))
                    if status in ("mismatch", "refused"):
                        (out / f"{status}-{s['digest'][:12]}-{name}-{fmt}.txt").write_text("\n".join(
                            f"== {arm}\n{o[2]}\n{o[1].decode(errors='replace')[:20000]}" for (arm, _), o in zip(ARMS, outs)))
                if old:
                    shutil.rmtree(old, ignore_errors=True)
        for c, repo in repos.items():
            if tree_digest(repo)[0] != trees[c]:
                raise Refused(f"{c}: the source tree changed during the replay")
    finally:
        shutil.rmtree(scratch, ignore_errors=True)
    controls = Path(a.controls) if a.controls else None
    p14_missing = bool(controls and (controls / "P14.py").is_file() and "P14.py" not in rep["controls_upstream"])
    classes = Counter(k for s in ordered for k in s["classes"])
    prepared = adm / "prepared.json"
    summary = dict(
        schema=SCHEMA, seeds=len(ordered), seed_classes=dict(classes), sample=a.sample, corpora=corpora,
        dispatch=dispatch, binaries={arm: sha256_file(b) for arm, b in binaries.items()}, formats=a.formats.split(","),
        admission_pairs_sha256=sha256_file(pair_file),
        admission_valid=json.loads(prepared.read_text()).get("valid") if prepared.exists() else None,
        upstream=dict(pairs=rep["pairs"], same_line=rep["same_line"], missing_or_unparsed=rep["missing"],
                      ambiguous=rep["ambiguous"], distinct_upstream_lines=len(rep["upstream_lines"])),
        delta=dict(delta), counts=dict(count), p14_upstream_missing=p14_missing,
        disconnected_seed=classes["disconnected"] > 0 or controls is None,
        rust_control=classes["rust_control"] > 0 or controls is None)
    summary["verdict"] = "FAIL" if count["mismatch"] or count["refused"] or p14_missing else "PASS"
    (out / "results.jsonl").write_text("".join(canon(r) + "\n" for r in results))
    (out / "summary.json").write_text(json.dumps(summary, indent=1, sort_keys=True) + "\n")
    print(f"review_diff {summary['verdict']}: {len(ordered)} seeds, {dict(count)}; delta {dict(delta)}")
    return 0 if summary["verdict"] == "PASS" else 1


def main(argv=None):
    cli = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    r = cli.add_subparsers(dest="cmd", required=True).add_parser("replay")
    for flag in ("runs", "admission", "out"):
        r.add_argument("--" + flag, required=True)
    r.add_argument("--corpora", nargs="*")
    r.add_argument("--sample", type=int, default=8, help="candidate pairs per corpus, in hash order")
    r.add_argument("--algorithms", help="comma list (default: every replayed reader)")
    r.add_argument("--formats", default="json", help="comma list of --format values, e.g. json,sarif")
    r.add_argument("--controls", default=str(Path(__file__).resolve().parents[2] / "tests/data/subscript_matrix/cases"))
    r.add_argument("--control-sample", type=int, default=12, help="matrix control files besides P14")
    r.add_argument("--timeout", type=int, default=3600)
    a = cli.parse_args(argv)
    try:
        return replay(a)
    except (Refused, OSError, ValueError, KeyError, subprocess.TimeoutExpired) as e:
        print(f"review_diff replay refused: {e}", file=sys.stderr)
        return 3


if __name__ == "__main__":
    sys.exit(main())
