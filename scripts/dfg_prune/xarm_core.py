#!/usr/bin/env python3
"""Expansion-arm oracle, core (#829 TEST-FORM section 1; usage in XARM.md). Python 3.9, stdlib only.

Four prism runs per fixture (`prism nav dfg-stats --edges`, no cache):
  T   the fixture as it is                       pairs after U/D rules   P(T)
  Tb  the fixture plus a census-poison file       the baseline, rules off B(T)
  X   every main file replaced by the compiler's post-expansion token stream, macro free
  Xb  X plus the poison file
Per pair d (verdicts describe the pair on the compiler's token stream, whatever head does):
  CONTRADICTED  the rules KEEP d on X with its owner on: the lexical proof fails once the
                macros are expanded, so a design must keep d (must-keep)
  CONFIRMED     the rules DROP d on X: the proof also holds on the compiler's tokens (may-drop)
  XOFF          d is kept on X only because its owner is off there (invalid program, ...)
  UNJUDGED      d does not exist on X (file did not tokenize, owner or lines differ, no baseline pair)
The clang side is a lexer-only action (`-fsyntax-only -Xclang -dump-tokens`), so a file that does
not parse still yields a stream. Tokens spelled in the main file keep their spelling line; tokens
of a macro body stay on the current output line.
"""
import collections
import json
import os
import re
import shutil
import subprocess

PRISM = os.environ.get("PRISM", os.path.join(
    os.path.dirname(os.path.abspath(__file__)), "../../target/release/prism"))
# A trigraph on a directive line makes the C macro census `incomplete`: U and D are off
# repo-wide, which gives the baseline without any code change. Spelled in two pieces so this
# file itself never holds a directive-looking line.
POISON = "?" + "?=define ZZ_POISON_829 1\n"
CEXT = (".c",)
CXXEXT = (".cc", ".cpp", ".cxx")
CLIKE = (".c .h .cc .cpp .cxx .hh .hpp .hxx .h++ .c++ .inl .inc .ipp .tcc .tpp .txx .def "
         ".cu .cuh .x").split()
CANON = {"l_brace": "{", "r_brace": "}", "l_square": "[", "r_square": "]", "hash": "#",
         "hashhash": "##"}
TOK = re.compile(r"^(\S+) '(.*)'\t.*\tLoc=<(.*)>\s*$", re.S)
LOC = re.compile(r"^(.*?):(\d+):(\d+)(?: <Spelling=(.*):(\d+):(\d+)>)?$")
INC = re.compile(rb'^[ \t]*#[ \t]*(?:include|include_next|import)[ \t]*["<]([^">\n]+)[">]', re.M)
ABS_INC = re.compile(rb'(#[ \t]*include[ \t]*")(/[^"\n]+)(")')


def is_cxx(name):
    return name.endswith(CXXEXT)


def materialize(files, links, dest, rewrite_abs=True):
    """Writes `files` ({rel: bytes}) and `links` ({rel: target}) under `dest`. An absolute
    include of a path that no longer exists (a tempdir of the Rust test) is pointed at the file
    of the fixture with the longest common path suffix."""
    shutil.rmtree(dest, ignore_errors=True)
    for rel, data in files.items():
        if rewrite_abs and ABS_INC.search(data):
            def fix(m):
                parts = m.group(2).decode("utf-8", "replace").split("/")
                best = None
                for cand in files:
                    cp = cand.split("/")
                    n = 0
                    while n < min(len(cp), len(parts)) and cp[-1 - n] == parts[-1 - n]:
                        n += 1
                    if n and (best is None or n > best[0]):
                        best = (n, cand)
                if best is None:
                    return m.group(0)
                return m.group(1) + os.path.join(dest, best[1]).encode() + m.group(3)
            data = ABS_INC.sub(fix, data)
        p = os.path.join(dest, rel)
        os.makedirs(os.path.dirname(p), exist_ok=True)
        with open(p, "wb") as f:
            f.write(data)
    for rel, target in links.items():
        p = os.path.join(dest, rel)
        os.makedirs(os.path.dirname(p), exist_ok=True)
        try:
            os.symlink(target, p)
        except OSError:
            pass


def run_prism(repo, tag, out, prism=None):
    """-> (Counter of (file, path, from line, to line), {(file, owner start line): state})."""
    errp = os.path.join(out, tag + ".err")
    with open(errp, "w") as err:
        p = subprocess.run([prism or PRISM, "nav", "--no-cache", "dfg-stats", "--repo", repo, "--edges"],
                           stdout=subprocess.PIPE, stderr=err, text=True, timeout=300,
                           env=dict(os.environ, PRISM_C_PRUNE_TRACE="1"))
    edges = collections.Counter()
    for line in p.stdout.splitlines():
        if not line.startswith('{"from"'):
            continue
        e = json.loads(line)
        f, t = e["from"], e["to"]
        path = ".".join([f["path"]["base"]] + f["path"].get("fields", []))
        edges[(f["file"], path, f["line"], t["line"])] += 1
    owners = {}
    for line in open(errp, errors="replace"):
        if line.startswith("c-prune-owner"):
            parts = line.rstrip("\n").split("\t")
            try:
                owners[(parts[-2], int(parts[-1]))] = parts[1]
            except ValueError:
                pass
    return edges, owners


def owner_state(owners, file, line):
    best = None
    for (f, start), state in owners.items():
        if f == file and start <= line and (best is None or start > best[0]):
            best = (start, state)
    return best[1] if best else "none"


def clang_cmd(src, flags, extra):
    cxx = is_cxx(src)
    return (["clang++" if cxx else "clang", "-fsyntax-only", "-w", "-fbracket-depth=100000"]
            + extra + (["-std=c++17"] if cxx else ["-std=gnu11"]) + flags + [src])


# Builtins the compiler's headers substitute for a source spelling the rules read by name: the
# expansion arm restores the spelling, so the X arm does not differ from the source in a way
# no macro of the program caused (`offsetof(T, f)` is a keyword form for the rules, whose type
# operand is no object name; `__builtin_offsetof(T, f)` is an ordinary call that makes `T` an
# identifier of the file and trips the phantom-declaration gate). redis `dict.c` and
# `ebuckets.c` gave 182 such contradictions in the consult.
BUILTIN_SPELLING = {"__builtin_offsetof": "offsetof", "__builtin_va_start": "va_start",
                    "__builtin_va_arg": "va_arg", "__builtin_va_end": "va_end",
                    "__builtin_va_copy": "va_copy"}
FALLTHROUGH = re.compile(r"__attribute__ \( \( (?:__)?fallthrough(?:__)? \) \) ;")
RECORD_END = re.compile(rb"Loc=<.*>\s*$", re.S)


def main_records(raw, src):
    """The token records of `raw` (clang `-dump-tokens` stderr, bytes) located in `src`: a
    record is the lines up to the one ending in `Loc=<...>` (a token may hold a newline).
    Records of other files are dropped before any parsing, which is what keeps a translation
    unit that includes a large standard library cheap."""
    mark = b"Loc=<" + src.encode("utf-8", "replace") + b":"
    cur = []
    for line in raw.split(b"\n"):
        cur.append(line)
        if RECORD_END.search(line):
            rec = b"\n".join(cur)
            cur = []
            if mark in rec or line.startswith(b"eof"):
                yield rec.decode("utf-8", "replace")
        elif len(cur) > 64:
            cur = []


def token_text(src, flags, cwd):
    """Macro-free text of `src` from clang's post-expansion token stream -> (rc, text, error)."""
    cmd = clang_cmd(src, flags, ["-Xclang", "-dump-tokens"])
    p = subprocess.run(cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE, cwd=cwd, timeout=900)
    fatal = [l for l in p.stderr.decode("utf-8", "replace").splitlines()
             if re.match(r"^[^\t']+:\d+:\d+: (fatal )?error:", l)]
    out, cur = {}, 1
    for rec in main_records(p.stderr, src):
        m = TOK.match(rec)
        if not m:
            continue
        kind, text, loc = m.groups()
        lm = LOC.match(loc) if kind != "eof" else None
        if not lm:
            continue
        f, line, col, sf, sl, sc = lm.groups()
        if os.path.abspath(os.path.join(cwd, f)) != os.path.abspath(src):
            continue
        line, col = int(line), int(col)
        if sf is None:
            home = line
        elif os.path.abspath(os.path.join(cwd, sf)) == os.path.abspath(src) \
                and (int(sl), int(sc)) > (line, col):
            home = int(sl)
        else:
            home = None
        if home is not None and home > cur:
            cur = home
        elif home is None and line > cur:
            cur = line
        if text in ("<%", "%>", "<:", ":>", "%:", "%:%:"):
            text = CANON.get(kind, text)
        text = BUILTIN_SPELLING.get(text, text)
        out.setdefault(cur, []).append(text.replace("\\\n", "").replace("\n", " "))
    n = max(out) if out else 0
    body = "\n".join(" ".join(out.get(i, [])) for i in range(1, n + 1)) + "\n"
    # `Py_FALLTHROUGH` and the like expand to an attribute statement, which tree-sitter-c does
    # not parse inside a `switch` (an owner-wide G1Error in the X arm only). It has no data-flow
    # meaning: a null statement stands in for it.
    body = FALLTHROUGH.sub(";", body)
    return (1 if fatal else 0), body, (fatal[0] if fatal else "")


def tokenize_with_stubs(src, flags, cwd, stubdir):
    """Missing includes become empty files (the repository alone is what the compiler has)."""
    flags = list(flags) + ["-idirafter", stubdir]
    stubbed = []
    for _ in range(60):
        rc, body, err = token_text(src, flags, cwd)
        m = re.search(r"fatal error: '([^']+)' file not found", err)
        if rc == 0 or not m:
            return rc, body, err, stubbed
        sp = os.path.join(stubdir, m.group(1))
        os.makedirs(os.path.dirname(sp), exist_ok=True)
        open(sp, "w").close()
        stubbed.append(m.group(1))
    return rc, body, err, stubbed


def census_inputs(files):
    """Approximation of the C census inputs of the repository (CLAUDE.md, 'census inputs'):
    C-like extensions (not under eval/secbench) plus the transitive include targets of those,
    each resolved by path suffix against every in-repo file."""
    def ok(rel):
        return "eval/secbench" not in rel
    seen = {r for r in files if ok(r) and r.lower().endswith(tuple(CLIKE))}
    todo = list(seen)
    while todo:
        rel = todo.pop()
        for m in INC.finditer(files.get(rel, b"")):
            t = m.group(1).decode("utf-8", "replace").replace("\\", "/")
            parts = []
            for seg in t.split("/"):
                if seg in ("", "."):
                    continue
                if seg == "..":
                    if parts:
                        parts.pop()
                    continue
                parts.append(seg)
            t = "/".join(parts)
            for cand in files:
                if ok(cand) and cand not in seen and (cand == t or cand.endswith("/" + t)):
                    seen.add(cand)
                    todo.append(cand)
    return seen


def opened_in_repo(src, flags, cwd, root):
    """In-repo files clang opens for `src` (`-H`), repo-relative."""
    cmd = clang_cmd(src, flags, ["-H"])
    p = subprocess.run(cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE, cwd=cwd, timeout=300)
    opened = set()
    for line in p.stderr.decode("utf-8", "replace").splitlines():
        m = re.match(r"^\.+ (.+)$", line)
        if m:
            path = os.path.abspath(os.path.join(cwd, m.group(1)))
            if path.startswith(root + os.sep):
                opened.add(os.path.relpath(path, root))
    return opened


class Arms:
    """Prism runs of one fixture in one directory `work`: T and Tb once, X and Xb per config."""

    def __init__(self, work, files, links):
        work = os.path.realpath(work)
        shutil.rmtree(work, ignore_errors=True)
        os.makedirs(work)
        self.work, self.files, self.links = work, files, links
        self.T, self.Tb = os.path.join(work, "T"), os.path.join(work, "Tb")
        materialize(files, links, self.T)
        materialize(files, links, self.Tb)
        with open(os.path.join(self.Tb, "zz_poison_829.c"), "w") as f:
            f.write(POISON)
        self.PT = run_prism(self.T, "T", work)
        self.BT = run_prism(self.Tb, "Tb", work)

    def expansion(self, tag, flags, prelude):
        """X and Xb for a configuration -> (PX, BX, notes). `prelude` is {name: text} placed
        beside the repository directory, outside the census; `{PRELUDE}` in a flag is that
        directory."""
        for name, text in prelude.items():
            p = os.path.join(self.work, name)
            os.makedirs(os.path.dirname(p), exist_ok=True)
            with open(p, "w") as f:
                f.write(text)
        # the build has the repository root on its include path (-I.), as the census's suffix
        # rule assumes for every in-repo directory
        flags = ["-I."] + [f.replace("{PRELUDE}", self.work) for f in flags]
        X, Xb = os.path.join(self.work, "X" + tag), os.path.join(self.work, "Xb" + tag)
        shutil.rmtree(X, ignore_errors=True)
        os.makedirs(X)
        stubdir = os.path.join(self.work, "stubs" + tag)
        shutil.rmtree(stubdir, ignore_errors=True)
        os.makedirs(stubdir)
        notes = {"failed": {}, "stubbed": [], "opened": set()}
        for rel in sorted(self.files):
            if not rel.endswith(CEXT + CXXEXT):
                continue
            src = os.path.join(self.T, rel)
            rc, body, err, stubbed = tokenize_with_stubs(src, flags, self.T, stubdir)
            notes["stubbed"] += stubbed
            if rc != 0:
                notes["failed"][rel] = re.sub(r"/\S*/", "", err)
                continue
            notes["opened"] |= opened_in_repo(src, flags + ["-idirafter", stubdir], self.T,
                                              os.path.realpath(self.T))
            dst = os.path.join(X, rel)
            os.makedirs(os.path.dirname(dst), exist_ok=True)
            with open(dst, "w") as f:
                f.write(body)
        shutil.copytree(X, Xb)
        with open(os.path.join(Xb, "zz_poison_829.c"), "w") as f:
            f.write(POISON)
        px = run_prism(X, "X" + tag, self.work)
        bx = run_prism(Xb, "Xb" + tag, self.work)
        notes["opened"] = sorted(notes["opened"])
        return px, bx, notes


def agg(counter, file, path, a, b):
    return counter.get((file, path, a, b), 0)


def verdict(arms, px, bx, notes, file, path, a, b):
    """-> (verdict string, detail) of the pair `file path@a -> path@b` on one expansion."""
    bt = agg(arms.BT[0], file, path, a, b)
    if bt == 0:
        return "UNJUDGED", "no-baseline-pair"
    if file in notes["failed"]:
        return "UNJUDGED", "tokenize-failed: " + notes["failed"][file][:120]
    bxn, pxn = agg(bx[0], file, path, a, b), agg(px[0], file, path, a, b)
    if bxn == 0:
        return "UNJUDGED", "no-pair-in-expansion"
    if pxn < bxn:
        return "CONFIRMED", ""
    state = owner_state(px[1], file, min(a, b))
    if state.startswith("on"):
        return "CONTRADICTED", ""
    return "XOFF", state


def head_drops(arms, file, path, a, b):
    return agg(arms.PT[0], file, path, a, b) < agg(arms.BT[0], file, path, a, b)


def inputs_gap(files, opened):
    """In-repo files the compiler opened that the census does not read (approximation)."""
    return sorted(set(opened) - census_inputs(files))
