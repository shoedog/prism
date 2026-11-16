#!/usr/bin/env python3
"""Bounded generator for the #829 expansion-arm oracle (TEST-FORM section 4, L2). Python 3.9.

Offline authoring tool, not CI. Programs come from productions over the enumerated classes;
only programs clang accepts count; the oracle (`xarm_core`: T, Tb, X, Xb prism runs) judges all
of them; CONTRADICTED programs (head drops the pair, the compiler's tokens keep it) are grouped
by key = (effect classes, site family, owner kind) and the minimal member of each key is frozen
as a row of `tests/fixtures/dfg_prune/oracle/generated_rows.jsonl` (`source: generator`).

  xgen.py run   --out DIR --bound K [--sample N --seed S] [--families ...] [--jobs J]
  xgen.py keys  DIR [DIR ...]                    group the results, print keys and counts
  xgen.py freeze DIR [DIR ...] [--known-from F]  freeze the minimal member of every key

Space (see XARM.md, 'The generator'): a program is one of the FAMILIES (use-site templates) over
an owner kind, a macro kind, a lexical variant and a definition channel; macro bodies are
sequences of fragments, and the BOUND K is the total number of body fragments over the (at most
two) macros of the program. Bound K is enumerated exhaustively over bodies with the plain
lexical variant and the same-file channel; the other lexical variants and channels are applied
to every contradicted program (and to a seeded sample of the others). A larger bound is drawn
at random (`--sample`), seeded.
"""
import argparse
import collections
import concurrent.futures as cf
import hashlib
import itertools
import json
import os
import random
import re
import shutil
import subprocess
import sys
import time

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import xarm_core as core  # noqa: E402

FIX = os.path.join(HERE, "..", "..", "tests", "fixtures", "dfg_prune", "oracle")

# ---------------------------------------------------------------- fragments
OBJ = ["{", "}", "(", ")", "[", "]", "<%", "%>", "<:", ":>", ",", ";", "if (c)", "else", "do",
       "while (0)", "for (;;)", "int x", "int *x", "x = 3", "c"]
PARAM_FR = ["p", "q", "r"]
# enclosing-group macros: the parameters, with and without a `;`, and small statement wrappers
ENC_FR = PARAM_FR + ["p;", "q;", "if (c) q;", "{ q; }", "if (c) { q; }"] + OBJ
DIGRAPH = {"<%": "{", "%>": "}", "<:": "[", ":>": "]"}
OPEN, CLOSE = "{([", "})]"
CTRL = {"if (c)", "else", "do", "while (0)", "for (;;)"}
CTRL_PREFIX = {"if (c)", "else", "do", "for (;;)"}
WRITE = {"x = 3", "int x", "int *x"}

FAMILIES = ["stmt", "guard", "init", "decl", "arg", "arg1", "enc1", "enc2", "enc3", "enc4"]
# #829 PLAN-v2 task 2: the macro-in-macro family (offline, `--families nest`; not in the default
# set, so the frozen L2 runs stay as they were). Two use sites `A;` / `B;` around `x = 2;`
# (statement or guard), inner macros `K(p, q)` / `K2(p, q)` with bodies from NEST_K, and outer
# object-like macros A, B of five forms over the inner macro and a third macro N / N2 (a body
# of NEST_A fragments):
#   f1 K(<frags>)   f2 ID(K <N>)   f3 K(<N>)   f4 one(K(<frags>))   f5 ID(one(<N>))
NEST_FORMS = (1, 2, 3, 4, 5)
NEST_SITES = ("stmt", "guard")
NEST_OWNERS = ["c", "cxx"]
# Fragments are composite: the first run over atomic fragments (the object-macro alphabet, bound 3,
# 550,920 programs) compiled 228 programs and found no contradicted one, because the hazards of
# a nested group (`K({ int a, b; }, c)`) need five or more atomic fragments. The composite
# fragments below put a held-comma block, a tight group and a statement expression each in one
# fragment, and the whole bodies `q if (c) p` and `q p` of the known contradicted shapes are
# one fragment each, so the canonical program (two inner macros, two groups) has four fragments:
# bound 4 over four slots.
NEST_K = ["p", "q", "p;", "q if (c) p", "q p", "if (c) p"]
NEST_A = ["{ int a, b; }", "{ a, b }", "(a, b)", "({ int a, b; })", "c"]
NEST_N_FORMS = (2, 3, 5)     # forms whose argument slots are bodies of N / N2
NEST_SHIELD_FORMS = (4, 5)   # the group sits inside `one(..)`
# families that take the macro kind variants (fn0: `A()`; alias: `#define A A0`)
KINDED = ["stmt", "guard"]
OWNERS_OBJ = ["c", "cxx", "tpl"]
OWNERS_KINDED = ["c", "tpl"]
OWNERS_EXTRA = ["method", "local"]  # stmt and guard only, plain
OWNERS_ENC = ["c", "cxx", "tpl"]
LEXICAL = ["plain", "splice_gap", "splice_tok", "comment_gap", "crlf"]
CHANNELS = ["same", "header", "payload"]
SITES = {
    "stmt": ("{A};", "{B};"), "guard": ("if (c) {A};", "{B};"), "init": ("int y = {A};", "{B};"),
    "decl": ("int {A};", "{B};"), "arg": ("H({A});", "H({B});"), "arg1": ("G({A});", "G({B});"),
}
ENC_OPEN = {  # an enclosing group opened at: file scope, a sibling owner, a declaration
    "enc1": "{M}(static int dummy;\n",
    "enc2": "void first(void) {{ {M}(0;\n}}\n",
    "enc3": "static int dummy = {M}(0;\n",
    "enc4": "{M}(static int dummy;\n",
}
FAMILY_NOTE = {
    "stmt": "statement", "guard": "guard", "init": "initializer", "decl": "declarator",
    "arg": "variadic argument", "arg1": "argument", "enc1": "enclosing group at file scope",
    "enc2": "enclosing group opened in a sibling", "enc3": "enclosing group in a declaration",
    "enc4": "enclosing group closed in a following owner",
}


def norm(tok):
    return DIGRAPH.get(tok, tok)


def body_tokens(frags):
    toks = []
    for f in frags:
        toks += re.findall(r"<%|%>|<:|:>|[A-Za-z_]\w*|\d+|\S", f)
    return [norm(t) for t in toks]


# ---------------------------------------------------------------- effect classes
def classify(frags, params=(), enc=False):
    """Effect classes of one macro body (the coarse key). In an enclosing-group macro `M(p, q, r)`
    the `;` fragments sequence the parameters (the owner is cut into them): a `;` there is not a
    statement-boundary body, so SEMI0 (#831) is not tagged for `enc`."""
    out = set()
    stack, ok, depth_p, semi0, comma0 = [], True, 0, False, False
    toks = body_tokens(frags)
    has_br = False
    for t in toks:
        if t in OPEN:
            stack.append(t)
            has_br = True
            if t == "(":
                depth_p += 1
        elif t in CLOSE:
            has_br = True
            if t == ")":
                depth_p -= 1
            if stack and OPEN.index(stack[-1]) == CLOSE.index(t):
                stack.pop()
            else:
                ok = False
        elif t == ";" and not stack:
            semi0 = True
        elif t == "," and depth_p <= 0:
            comma0 = True
    if has_br:
        out.add("BR_BAL" if ok and not stack else "BR_UNBAL")
    if comma0:
        out.add("ARGSPLIT")
    if semi0 and not enc:
        out.add("SEMI0")
    if CTRL & set(frags):
        out.add("CTRL")
    if WRITE & set(frags):
        out.add("WRITE")
    if params:
        used = [f for f in frags if f in params]
        if set(used) != set(params):
            out.add("PARAMS")
        elif len(used) != len(set(used)) or used != sorted(used, key=params.index):
            out.add("PARAMS")
    return out


# ---------------------------------------------------------------- rendering
def render_body(frags, lex):
    """Body text of a macro under a lexical variant, or None when the variant does not apply."""
    if lex in ("plain", "crlf"):
        return " ".join(frags)
    if lex == "splice_gap":
        if len(frags) < 2:
            return None
        return frags[0] + " \\\n" + " ".join(frags[1:])
    if lex == "splice_tok":
        for i, f in enumerate(frags):
            if len(f) >= 2 and f[0] != " ":
                cut = 1
                a, b = f[:cut], f[cut:]
                parts = frags[:i] + [a + "\\\n" + b] + frags[i + 1:]
                return " ".join(parts)
        return None
    if lex == "comment_gap":
        if not frags:
            return None
        return " /* } ) ] */ ".join(frags) + " /* { ( [ */"
    raise ValueError(lex)


def define_line(name, params, body):
    head = "#define %s%s" % (name, "(%s)" % ", ".join(params) if params is not None else "")
    return head + (" " + body if body else "") + "\n"


def owner_body(s1, s2, cxx):
    ret = "return sink(x + 1);" if cxx else "return x;"
    return "    int x = 41;\n    %s\n    x = 2;\n    %s\n    %s\n" % (s1, s2, ret)


def owner_text(kind, s1, s2):
    cxx = kind != "c"
    body = owner_body(s1, s2, cxx)
    pre = "int sink(int);\n" if cxx else ""
    if kind in ("c", "cxx"):
        return pre + "int probe(int c) {\n" + body + "}\n"
    if kind == "tpl":
        return pre + "template <typename T>\nint probe(T c) {\n" + body + "}\n"
    if kind == "method":
        return pre + "struct K {\n  int probe(int c) {\n" + body + "  }\n};\n"
    if kind == "local":
        return (pre + "int outer(int c) {\n  struct L {\n    static int probe(int c) {\n" + body
                + "    }\n  };\n  return L::probe(c);\n}\n")
    raise ValueError(kind)


ENC_OWNER = ("int probe(int c) {\n    int x = 41;\n    int a, b;\n    x = 2;\n    int d, e;\n"
             "    return x;\n}\n")


def enc_owner(kind):
    if kind == "c":
        return ENC_OWNER
    cxx = ENC_OWNER.replace("return x;", "return sink(x + 1);")
    if kind == "cxx":
        return cxx
    if kind == "tpl":
        return "template <typename T>\n" + cxx.replace("probe(int c)", "probe(T c)")
    raise ValueError(kind)


def innermost_commas(frags):
    """(a comma whose innermost bracket is `{` or `[`, a comma that is not directly inside a
    parenthesis) of a fragment sequence: the held and the loose commas of a body."""
    held = loose = False
    stack = []
    for t in body_tokens(frags):
        if t in OPEN:
            stack.append(t)
        elif t in CLOSE:
            if stack:
                stack.pop()
        elif t == ",":
            if not stack or stack[-1] != "(":
                loose = True
            if stack and stack[-1] in "{[":
                held = True
    return held, loose


def nest_effects(s):
    """Effect classes of a `nest` program: its four bodies and the three nest classes."""
    form = int(s["kind"].rsplit("_f", 1)[1])
    kb, k2b, ab, bb = s["bodies"]
    eff = classify(kb, ("p", "q")) | classify(k2b, ("p", "q")) | classify(ab) | classify(bb)
    held = innermost_commas(ab)[0] or innermost_commas(bb)[0]
    loose = innermost_commas(ab)[1] or innermost_commas(bb)[1]
    if held:
        eff.add("NEST_HELD")
    if form in NEST_N_FORMS and loose:
        eff.add("NEST_FWD")
    if form in NEST_SHIELD_FORMS:
        eff.add("NEST_SHIELD")
    return eff


def nest_owner_text(kind, s1, s2):
    """The owner of a `nest` program, the shape of the review probes (`bodysplit`): `b` is a
    variable the inner macros can name, and the use sites sit in an inner block that an
    emitted `}` can close."""
    cxx = kind != "c"
    ret = "return sink(x + 1);" if cxx else "return x;"
    first = "int x = 41; int y829 = 0;" if cxx else "int x = 41;"
    pre = "int sink(int);\n" if cxx else ""
    return (pre + "int probe(int c) {\n    %s\n    int b = 0;\n    {\n        %s\n        x = 2;\n"
            "        %s\n        %s\n    }\n}\n" % (first, s1, s2, ret))


def render_nest(s, n, kind_ok, suf, lex):
    """(defs text, owner text, effect classes) of a `nest` program, or None."""
    site, form = s["kind"].rsplit("_f", 1)[0], int(s["kind"].rsplit("_f", 1)[1])
    kb, k2b, ab, bb = s["bodies"]
    rk, rk2 = render_body(kb, lex), render_body(k2b, lex)
    if lex not in ("plain", "crlf"):
        if not kb and not k2b and form not in NEST_N_FORMS:
            return None
        rk = rk if rk is not None else " ".join(kb)
        rk2 = rk2 if rk2 is not None else " ".join(k2b)
    nm = {x: x + suf for x in ("ID", "K", "K2", "N", "N2", "A", "B", "one")}
    na = render_body(ab, lex) if form in NEST_N_FORMS else " ".join(ab)
    nb = render_body(bb, lex) if form in NEST_N_FORMS else " ".join(bb)
    if lex not in ("plain", "crlf") and form in NEST_N_FORMS:
        na = na if na is not None else " ".join(ab)
        nb = nb if nb is not None else " ".join(bb)
    outer = {
        1: "{K}({a})", 2: "{ID}({K} {N})", 3: "{K}({N})", 4: "{one}({K}({a}))",
        5: "{ID}(one({N}))",
    }[form]
    def body(k, nn, a):
        return outer.format(K=nm[k], ID=nm["ID"], one=nm["one"], N=nm[nn], a=a)
    defs = "int %s(int);\n" % nm["one"]
    if form in (2, 5):
        defs += define_line(nm["ID"], ["p"], "p")
    if form != 5:
        defs += define_line(nm["K"], ["p", "q"], rk) + define_line(nm["K2"], ["p", "q"], rk2)
    if form in NEST_N_FORMS:
        defs += define_line(nm["N"], None, na) + define_line(nm["N2"], None, nb)
    defs += define_line(nm["A"], None, body("K", "N", " ".join(ab)))
    defs += define_line(nm["B"], None, body("K2", "N2", " ".join(bb)))
    t1, t2 = SITES[site]
    s1, s2 = t1.replace("{A}", nm["A"]), t2.replace("{B}", nm["B"])
    return defs, nest_owner_text(kind_ok, s1, s2), nest_effects(s)


class Prog:
    """One program of the space. `spec` is JSON-able and re-renders the program."""

    def __init__(self, spec):
        self.spec = spec

    def effects(self):
        """Effect classes of the program's macros (from the spec; no rendering)."""
        s = self.spec
        if s["family"] == "nest":
            return sorted(nest_effects(s))
        if s["family"].startswith("enc"):
            return sorted(classify(s["bodies"][0], ("p", "q", "r"), enc=True))
        eff = set()
        for fr in s["bodies"]:
            eff |= classify(fr)
        return sorted(eff)

    def render(self, n):
        """-> dict(files, texts, target, owner, from, to, effects) with macro-name suffix `n`
        (None: no suffix), or None when the lexical variant does not apply to the program."""
        s = self.spec
        fam, ok, kind, lex, ch = s["family"], s["owner"], s["kind"], s["lex"], s["channel"]
        cxx = ok != "c"
        suf = "" if n is None else "_%s" % n
        defs, eff = "", set()
        if fam == "nest":
            r = render_nest(s, n, ok, suf, lex)
            if r is None:
                return None
            defs, owner_part, eff = r
        elif fam.startswith("enc"):
            body = render_body(s["bodies"][0], lex)
            if body is None:
                return None
            name = "M" + suf
            defs += "int b, e;\n" + define_line(name, ["p", "q", "r"], body)
            if cxx:
                defs += "int sink(int);\n"
            eff |= classify(s["bodies"][0], ("p", "q", "r"), enc=True)
            closer = "void last(void) { )\n}\n" if fam == "enc4" else ")\n"
            owner_part = ENC_OPEN[fam].format(M=name) + enc_owner(ok) + closer
        else:
            a_fr, b_fr = s["bodies"]
            ra, rb = render_body(a_fr, lex), render_body(b_fr, lex)
            if lex not in ("plain", "crlf"):
                if ra is None and rb is None:
                    return None
                ra = ra if ra is not None else " ".join(a_fr)
                rb = rb if rb is not None else " ".join(b_fr)
            an, bn = "A" + suf, "B" + suf
            wrap = None
            if fam in ("arg", "arg1"):
                wrap = ("H" if fam == "arg" else "G") + suf
                defs += define_line(wrap, ["..."] if fam == "arg" else ["p"],
                                    "__VA_ARGS__" if fam == "arg" else "p")
            for nm, fr, rtxt in ((an, a_fr, ra), (bn, b_fr, rb)):
                eff |= classify(fr)
                if kind == "fn0" and nm == an:
                    defs += define_line(nm, [], rtxt)
                elif kind == "alias" and nm == an:
                    defs += define_line(nm + "_a", None, rtxt) + define_line(nm, None, nm + "_a")
                else:
                    defs += define_line(nm, None, rtxt)
            t1, t2 = SITES[fam]
            s1 = t1.replace("{A}", an + ("()" if kind == "fn0" else ""))
            s2 = t2.replace("{B}", bn)
            if wrap:
                s1, s2 = (x.replace("H(", wrap + "(").replace("G(", wrap + "(") for x in (s1, s2))
            owner_part = owner_text(ok, s1, s2)
        ext = ".hpp" if cxx else ".h"
        tgt = "p%s%s" % (suf, ".cc" if cxx else ".c")
        files, texts = {}, []
        if ch == "same":
            main = defs + owner_part
        elif ch == "header":
            hn = "h%s%s" % (suf, ext)
            files[hn] = defs
            main = '#include "%s"\n' % hn + owner_part
        else:
            pn = "d%s.payload" % suf
            files[pn] = defs
            texts.append(pn)
            main = '#include "%s"\n' % pn + owner_part
        lines = main.split("\n")
        frm = next(i for i, l in enumerate(lines, 1) if "int x = 41;" in l)
        to = next(i for i, l in enumerate(lines, 1) if i > frm and "return " in l)
        files[tgt] = main
        if lex == "crlf":
            files = {k: v.replace("\n", "\r\n") for k, v in files.items()}
        return {"files": files, "texts": texts, "target": tgt, "owner": "probe", "from": frm,
                "to": to, "effects": sorted(eff)}


# ---------------------------------------------------------------- prefilter (necessary condition)
TOKRE = re.compile(r"<%|%>|<:|:>|[A-Za-z_]\w*|\d+|\S")


def tokenize(text):
    return [norm(t) for t in TOKRE.findall(text)]


def balanced(toks):
    """Nesting of `{` and `(` only: clang accepts some unterminated `[[` attribute forms."""
    st = []
    for t in toks:
        if t in "[]":
            continue
        if t in OPEN:
            st.append(t)
        elif t in CLOSE:
            if not st or OPEN.index(st[-1]) != CLOSE.index(t):
                return False
            st.pop()
    return not st


def prefilter(rendered):
    """False only when the program cannot compile (unbalanced after expansion, a wrong argument
    count of an identity wrapper). Programs of other shapes pass."""
    text = rendered["files"][rendered["target"]].replace("\r\n", "\n")
    macros, stream = {}, []
    srcs = [text] + [rendered["files"][k].replace("\r\n", "\n") for k in rendered["files"]
                     if k != rendered["target"]]
    for src in srcs:
        i, lines = 0, src.split("\n")
        while i < len(lines):
            line = lines[i]
            while line.endswith("\\") and i + 1 < len(lines):
                i += 1
                line = line[:-1] + lines[i]
            m = re.match(r"\s*#\s*define\s+(\w+)(\(([^)]*)\))?\s*(.*)$", line)
            if m:
                name, _, ps, body = m.groups()
                body = re.sub(r"/\*.*?\*/", " ", body)
                macros[name] = (None if ps is None else [x.strip() for x in ps.split(",") if x.strip()],
                                tokenize(body))
            elif not line.lstrip().startswith("#") and src is text:
                stream += tokenize(line)
            i += 1

    def expand(toks, hide, depth=0):
        if depth > 12:
            return None
        out, i = [], 0
        while i < len(toks):
            t = toks[i]
            mac = macros.get(t)
            if mac is None or t in hide:
                out.append(t)
                i += 1
                continue
            params, body = mac
            if params is None:
                sub = expand(body, hide | {t}, depth + 1)
                if sub is None:
                    return None
                out += sub
                i += 1
                continue
            if i + 1 >= len(toks) or toks[i + 1] != "(":
                out.append(t)
                i += 1
                continue
            j, d, args, cur = i + 2, 1, [], []
            while j < len(toks):
                u = toks[j]
                if u == "(":
                    d += 1
                elif u == ")":
                    d -= 1
                    if d == 0:
                        break
                if u == "," and d == 1:
                    args.append(cur)
                    cur = []
                else:
                    cur.append(u)
                j += 1
            else:
                return None  # unterminated argument list
            args.append(cur)
            if params == ["..."]:
                res = []
                for k, a in enumerate(args):
                    ea = expand(a, hide, depth + 1)
                    if ea is None:
                        return None
                    res += ([","] if k else []) + ea
            elif params == ["p"] and body == ["p"]:
                if len(args) != 1:
                    return None
                res = expand(args[0], hide, depth + 1)
                if res is None:
                    return None
            else:
                return out + toks[i:]  # a shape this filter does not model: accept
            sub = expand(res, hide | {t}, depth + 1)
            if sub is None:
                return None
            out += sub
            i = j + 1
        return out

    ex = expand(stream, frozenset())
    return ex is not None and balanced(ex)


# ---------------------------------------------------------------- enumeration
def bodies_upto(alpha, k):
    for n in range(k + 1):
        yield from itertools.product(alpha, repeat=n)


def enumerate_nest(bound):
    """Exhaustive `nest` specs of one bound: the four bodies [K, K2, A slot, B slot] hold at
    most `bound` fragments in total (form 5 has no inner macro: its K bodies stay empty)."""
    for site in NEST_SITES:
        for form in NEST_FORMS:
            for ok in NEST_OWNERS:
                kmax = 0 if form == 5 else bound
                for n1 in range(kmax + 1):
                    for n2 in range(kmax + 1 - n1):
                        for n3 in range(bound + 1 - n1 - n2):
                            for n4 in range(bound + 1 - n1 - n2 - n3):
                                for b1 in itertools.product(NEST_K, repeat=n1):
                                    for b2 in itertools.product(NEST_K, repeat=n2):
                                        for b3 in itertools.product(NEST_A, repeat=n3):
                                            for b4 in itertools.product(NEST_A, repeat=n4):
                                                yield {"family": "nest", "owner": ok,
                                                       "kind": "%s_f%d" % (site, form),
                                                       "lex": "plain", "channel": "same",
                                                       "bodies": [list(b1), list(b2),
                                                                  list(b3), list(b4)]}


def enumerate_specs(bound, families, owners_override=None):
    """Exhaustive specs of one bound (plain, same file): total body fragments <= bound."""
    for fam in families:
        if fam == "nest":
            yield from enumerate_nest(bound)
            continue
        if fam.startswith("enc"):
            for ok in OWNERS_ENC:
                for body in bodies_upto(ENC_FR, bound):
                    yield {"family": fam, "owner": ok, "kind": "obj", "lex": "plain",
                           "channel": "same", "bodies": [list(body)]}
            continue
        kinds = [("obj", OWNERS_OBJ)]
        if fam in KINDED:
            kinds += [("fn0", OWNERS_KINDED), ("alias", OWNERS_KINDED)]
            kinds += [("obj", OWNERS_EXTRA)]
        for kind, owners in kinds:
            for ok in owners:
                for a in range(bound + 1):
                    for b in range(bound + 1 - a):
                        for ba in itertools.product(OBJ, repeat=a):
                            for bb in itertools.product(OBJ, repeat=b):
                                yield {"family": fam, "owner": ok, "kind": kind, "lex": "plain",
                                       "channel": "same", "bodies": [list(ba), list(bb)]}


def sample_specs(bound, families, count, seed):
    """`count` specs drawn uniformly over (family, kind/owner, total fragments == bound)."""
    rng = random.Random(seed)
    cells = []
    for fam in families:
        if fam == "nest":
            raise SystemExit("the nest family is enumerated exhaustively (no --sample)")
        if fam.startswith("enc"):
            cells += [(fam, "obj", ok) for ok in OWNERS_ENC]
            continue
        cells += [(fam, "obj", ok) for ok in OWNERS_OBJ]
        if fam in KINDED:
            cells += [(fam, k, ok) for k in ("fn0", "alias") for ok in OWNERS_KINDED]
            cells += [(fam, "obj", ok) for ok in OWNERS_EXTRA]
    for _ in range(count):
        fam, kind, ok = rng.choice(cells)
        if fam.startswith("enc"):
            bodies = [[rng.choice(ENC_FR) for _ in range(bound)]]
        else:
            a = rng.randint(0, min(bound, 3))
            b = bound - a
            if b > 3:
                a, b = bound - 3, 3
            bodies = [[rng.choice(OBJ) for _ in range(a)], [rng.choice(OBJ) for _ in range(b)]]
        yield {"family": fam, "owner": ok, "kind": kind, "lex": "plain", "channel": "same",
               "bodies": bodies}


def extension_specs(prev_dirs, count, seed):
    """Every single-fragment insertion into `count` seeded compiled programs (plain, same file)
    of earlier runs: the bound-plus-one neighbourhood of programs clang accepts."""
    rng = random.Random(seed)
    pool = []
    for d in prev_dirs:
        for l in open(os.path.join(d, "results.jsonl")):
            r = json.loads(l)
            sp = r["spec"]
            if sp["lex"] == "plain" and sp["channel"] == "same":
                pool.append(sp)
    rng.shuffle(pool)
    seen = set()
    for sp in pool[:count]:
        for bi, body in enumerate(sp["bodies"]):
            alpha = ENC_FR if sp["family"].startswith("enc") else OBJ
            if not sp["family"].startswith("enc") and len(body) >= 3:
                continue  # a body stays within 3 fragments (the per-body cap)
            for pos in range(len(body) + 1):
                for fr in alpha:
                    nb = [list(b) for b in sp["bodies"]]
                    nb[bi] = body[:pos] + [fr] + body[pos:]
                    spec = dict(sp)
                    spec["bodies"] = nb
                    j = json.dumps(spec, sort_keys=True)
                    if j not in seen:
                        seen.add(j)
                        yield spec


def variants(spec):
    """The other lexical variants and channels of a spec."""
    out = []
    for lex in LEXICAL:
        for ch in CHANNELS:
            if lex == "plain" and ch == "same":
                continue
            s = dict(spec)
            s["lex"], s["channel"] = lex, ch
            out.append(s)
    return out


# ---------------------------------------------------------------- compile filter
def compiles(rendered, scratch):
    """clang accepts the program (syntax only), with the files of the row."""
    cxx = rendered["target"].endswith(".cc")
    files = rendered["files"]
    only = len(files) == 1
    cmd = ["clang++" if cxx else "clang", "-fsyntax-only", "-w", "-ferror-limit=1",
           "-fno-color-diagnostics", "-std=c++17" if cxx else "-std=gnu11"]
    if only:
        p = subprocess.run(cmd + ["-x", "c++" if cxx else "c", "-"], input=list(files.values())[0]
                           .encode("latin-1", "replace"), stdout=subprocess.DEVNULL,
                           stderr=subprocess.DEVNULL, timeout=60)
        return p.returncode == 0
    d = os.path.join(scratch, hashlib.md5(json.dumps(files, sort_keys=True).encode()).hexdigest())
    os.makedirs(d, exist_ok=True)
    for k, v in files.items():
        with open(os.path.join(d, k), "wb") as f:
            f.write(v.encode("latin-1", "replace"))
    p = subprocess.run(cmd + [rendered["target"]], cwd=d, stdout=subprocess.DEVNULL,
                       stderr=subprocess.DEVNULL, timeout=60)
    shutil.rmtree(d, ignore_errors=True)
    return p.returncode == 0


# ---------------------------------------------------------------- judging
def run_batch(items, work, jobs):
    """items: [(pid, rendered)] -> {pid: (verdict, detail)}. One repository per call."""
    work = os.path.realpath(work)
    shutil.rmtree(work, ignore_errors=True)
    files = {}
    for pid, r in items:
        files.update({k: v.encode("latin-1", "replace") for k, v in r["files"].items()})
    T, Tb = os.path.join(work, "T"), os.path.join(work, "Tb")
    core.materialize(files, {}, T, rewrite_abs=False)
    core.materialize(files, {}, Tb, rewrite_abs=False)
    with open(os.path.join(Tb, "zz_poison_829.c"), "w") as f:
        f.write(core.POISON)
    PT, owT = core.run_prism(T, "T", work)
    BT, _ = core.run_prism(Tb, "Tb", work)
    if any(st.startswith("off:Incomplete") for st in owT.values()):
        return None  # a program made the census incomplete: the caller bisects
    X, Xb = os.path.join(work, "X"), os.path.join(work, "Xb")
    os.makedirs(X)
    failed = {}

    def tok(it):
        pid, r = it
        src = os.path.join(T, r["target"])
        rc, body, err = core.token_text(src, ["-I."], T)
        return pid, r, rc, body, err

    with cf.ThreadPoolExecutor(jobs) as ex:
        for pid, r, rc, body, err in ex.map(tok, items):
            if rc != 0:
                failed[pid] = err[:100]
                continue
            with open(os.path.join(X, r["target"]), "w") as f:
                f.write(body)
    shutil.copytree(X, Xb)
    with open(os.path.join(Xb, "zz_poison_829.c"), "w") as f:
        f.write(core.POISON)
    PX, owX = core.run_prism(X, "X", work)
    BX, _ = core.run_prism(Xb, "Xb", work)
    out = {}
    for pid, r in items:
        k = (r["target"], "x", r["from"], r["to"])
        bt, pt = BT.get(k, 0), PT.get(k, 0)
        if bt == 0:
            out[pid] = ("UNJUDGED", "no-baseline-pair")
        elif pt >= bt:
            out[pid] = ("NODROP", "")
        elif pid in failed:
            out[pid] = ("UNJUDGED", "tokenize-failed: " + failed[pid])
        elif BX.get(k, 0) == 0:
            out[pid] = ("UNJUDGED", "no-pair-in-expansion")
        elif PX.get(k, 0) < BX.get(k, 0):
            out[pid] = ("CONFIRMED", "")
        else:
            st = core.owner_state(owX, r["target"], min(r["from"], r["to"]))
            out[pid] = ("CONTRADICTED", "") if st.startswith("on") else ("XOFF", st)
    shutil.rmtree(work, ignore_errors=True)
    return out


def judge_all(items, work, jobs, batch=400):
    """Batches with bisection on a census-incomplete batch."""
    res = {}
    todo = [items[i:i + batch] for i in range(0, len(items), batch)]
    while todo:
        chunk = todo.pop()
        out = run_batch(chunk, work, jobs)
        if out is not None:
            res.update(out)
        elif len(chunk) == 1:
            res[chunk[0][0]] = ("INCOMPLETE", "census incomplete")
        else:
            h = len(chunk) // 2
            todo += [chunk[:h], chunk[h:]]
    return res


# ---------------------------------------------------------------- run
def key_of(spec, effects):
    return "|".join([",".join(sorted(effects)) or "-", spec["family"] + ("+" + spec["kind"] if spec["kind"] != "obj" else ""),
                     spec["owner"]])


def vkey(spec):
    return spec["lex"] + "/" + spec["channel"]


def cmd_run(a):
    out = os.path.abspath(a.out)
    os.makedirs(out, exist_ok=True)
    fams = a.families.split(",") if a.families else FAMILIES
    if a.nest_owners:
        NEST_OWNERS[:] = a.nest_owners.split(",")
    t0 = time.time()
    if a.extend_from:
        specs = extension_specs(a.extend_from, a.extend_count, a.seed)
    elif a.sample:
        specs = sample_specs(a.bound, fams, a.sample, a.seed)
    else:
        specs = enumerate_specs(a.bound, fams)
    scratch = os.path.join(out, "scratch")
    os.makedirs(scratch, exist_ok=True)
    stats = collections.Counter()
    results_path = os.path.join(out, "results.jsonl")
    done = set()
    if os.path.exists(results_path):
        for l in open(results_path):
            done.add(json.dumps(json.loads(l)["spec"], sort_keys=True))
    resf = open(results_path, "a")

    def accept(spec):
        r = Prog(spec).render(None)
        if r is None:
            return None
        # the prefilter models object macros and identity wrappers only: it cannot see through
        # a two-parameter inner macro, so a nest program (whose braces K may rebalance) skips it
        if not spec["family"].startswith("enc") and spec["family"] != "nest" \
                and not prefilter(r):
            return ("pre", spec)
        return ("ok", spec, compiles(r, scratch))

    pending = []

    def flush(force=False):
        nonlocal pending
        while len(pending) >= a.chunk or (force and pending):
            part, pending = pending[:a.chunk], pending[a.chunk:]
            items = []
            for i, spec in enumerate(part):
                r = Prog(spec).render(i)
                items.append(("n%d" % i, r))
            verdicts = judge_all(items, os.path.join(out, "work"), a.jobs, a.chunk)
            for (pid, r), spec in zip(items, part):
                v, d = verdicts[pid]
                stats[v] += 1
                resf.write(json.dumps({"spec": spec, "verdict": v, "detail": d,
                                       "effects": r["effects"]}, sort_keys=True) + "\n")
            resf.flush()
            print("%6.0fs judged %d %s" % (time.time() - t0, sum(
                n for k, n in stats.items() if k.isupper()), {k: n for k, n in stats.items()
                                                              if k.isupper()}), file=sys.stderr)

    def batched(it, n):
        it = iter(it)
        while True:
            chunk = list(itertools.islice(it, n))
            if not chunk:
                return
            yield chunk

    interesting = []
    with cf.ThreadPoolExecutor(a.jobs) as ex:
        for chunk in batched(specs, 4000):
            chunk = [s for s in chunk if json.dumps(s, sort_keys=True) not in done]
            for res in ex.map(accept, chunk):
                stats["candidates"] += 1
                if res is None:
                    stats["variant-na"] += 1
                elif res[0] == "pre":
                    stats["prefiltered"] += 1
                elif not res[2]:
                    stats["rejected-by-clang"] += 1
                else:
                    stats["compiled"] += 1
                    pending.append(res[1])
            flush()
            print("%6.0fs candidates %d prefiltered %d compiled %d" % (
                time.time() - t0, stats["candidates"], stats["prefiltered"], stats["compiled"]),
                file=sys.stderr)
    flush(force=True)
    resf.close()
    # variants: up to --var-per-key minimal contradicted programs of every core key, and a
    # seeded sample of the other programs
    rng = random.Random(a.seed + 1)
    var_specs, seen_var = [], set()
    plain_rows = [json.loads(l) for l in open(results_path)]
    plain_rows = [r for r in plain_rows if r["spec"]["lex"] == "plain"
                  and r["spec"]["channel"] == "same"]
    per_key = collections.defaultdict(list)
    for r in plain_rows:
        if r["verdict"] == "CONTRADICTED":
            per_key[key_of(r["spec"], r["effects"])].append(r)
        elif rng.random() < a.var_sample:
            per_key["~sample"].append(r)
    chosen = []
    for k, rs in per_key.items():
        rs.sort(key=lambda r: (sum(len(b) for b in r["spec"]["bodies"]),
                               json.dumps(r["spec"], sort_keys=True)))
        chosen += rs if k == "~sample" else rs[:a.var_per_key]
    for r in chosen:
        for v in variants(r["spec"]):
            j = json.dumps(v, sort_keys=True)
            if j not in seen_var:
                seen_var.add(j)
                var_specs.append(v)
    var_specs = [s for s in var_specs if json.dumps(s, sort_keys=True) not in done]
    print("variants to judge:", len(var_specs), file=sys.stderr)
    resf = open(results_path, "a")
    with cf.ThreadPoolExecutor(a.jobs) as ex:
        for chunk in batched(var_specs, 4000):
            for res in ex.map(accept, chunk):
                stats["v-candidates"] += 1
                if res is None:
                    stats["v-na"] += 1
                elif res[0] == "pre":
                    stats["v-prefiltered"] += 1
                elif not res[2]:
                    stats["v-rejected"] += 1
                else:
                    stats["v-compiled"] += 1
                    pending.append(res[1])
            flush()
    flush(force=True)
    resf.close()
    summary = dict(stats)
    summary["seconds"] = round(time.time() - t0)
    summary["bound"] = a.bound
    summary["sample"] = a.sample
    json.dump(summary, open(os.path.join(out, "summary.json"), "w"), indent=1)
    print(json.dumps(summary, indent=1))


def load_results(dirs):
    rows = []
    for d in dirs:
        for l in open(os.path.join(d, "results.jsonl")):
            r = json.loads(l)
            r["run"] = d
            rows.append(r)
    return rows


def group(rows):
    """key -> CONTRADICTED rows, minimal first. A key is (effect classes, family, owner kind); a
    variant (lexical, channel) of a program whose core key is already contradicted in the plain
    same-file form counts toward that key (`variant_of`); a variant-only contradiction is a key
    of its own, suffixed with the variant."""
    for r in rows:
        r["effects"] = Prog(r["spec"]).effects()  # recomputed: the classes are a function of the spec
    plain = set()
    for r in rows:
        s = r["spec"]
        if r["verdict"] == "CONTRADICTED" and s["lex"] == "plain" and s["channel"] == "same":
            plain.add(key_of(s, r["effects"]))
    by = collections.defaultdict(list)
    for r in rows:
        if r["verdict"] != "CONTRADICTED":
            continue
        s = r["spec"]
        k = key_of(s, r["effects"])
        if (s["lex"] != "plain" or s["channel"] != "same") and k not in plain:
            k += "|" + vkey(s)
        by[k].append(r)
    for k in by:
        by[k].sort(key=lambda r: (r["spec"]["lex"] != "plain" or r["spec"]["channel"] != "same",
                                  sum(len(b) for b in r["spec"]["bodies"]),
                                  json.dumps(r["spec"], sort_keys=True)))
    return by


def cmd_keys(a):
    rows = load_results(a.dirs)
    by = group(rows)
    cnt = collections.Counter(r["verdict"] for r in rows)
    print("judged", len(rows), dict(cnt))
    n831 = [k for k in by if "SEMI0" in k.split("|")[0]]
    print("keys", len(by), "of which #831 (SEMI0)", len(n831))
    for k in sorted(by):
        print("%5d  %s" % (len(by[k]), k))


def cmd_freeze(a):
    rows = load_results(a.dirs)
    by = group(rows)
    known = set()
    if a.known_from:
        known = {l.strip() for l in open(a.known_from) if l.strip()}
    out, skipped = [], 0
    for k in sorted(by):
        if "SEMI0" in k.split("|")[0]:
            skipped += 1
            continue
        if k in known:
            continue
        spec = by[k][0]["spec"]
        r = Prog(spec).render(None)
        out.append({"key": k, "source": "generator", "spec": spec, "files": r["files"],
                    "texts": r["texts"], "target": r["target"], "owner": r["owner"], "path": "x",
                    "from": r["from"], "to": r["to"], "effects": r["effects"],
                    "members": len(by[k])})
    json.dump(out, open(a.out, "w"), indent=1)
    print("frozen candidates", len(out), "skipped #831 keys", skipped)


def verify_one(cand, work):
    """Re-judges a frozen candidate ALONE with the full oracle (core.Arms: T, Tb, X, Xb, and
    the clang -H inputs check) -> the row, or None when it is not CONTRADICTED with head
    dropping the pair (a batch artifact)."""
    files = {k: v.encode("latin-1", "replace") for k, v in cand["files"].items()}
    arms = core.Arms(work, files, {})
    px, bx, notes = arms.expansion("0", [], {})
    v, d = core.verdict(arms, px, bx, notes, cand["target"], "x", cand["from"], cand["to"])
    drops = core.head_drops(arms, cand["target"], "x", cand["from"], cand["to"])
    gap = core.inputs_gap(files, notes["opened"])
    shutil.rmtree(work, ignore_errors=True)
    if v != "CONTRADICTED" or not drops or gap:
        return None, (v, d, drops, gap)
    row = dict(cand)
    row.update({"oracle": "CONTRADICTED", "assert": "kept", "head_drops": True,
                "assumption": None, "boundary": False,
                "basis": "generator: minimal member of its key, re-judged alone"})
    return row, None


def cmd_verify(a):
    cands = json.load(open(a.cands))
    out, bad = [], []
    with cf.ThreadPoolExecutor(a.jobs) as ex:
        futs = [(c, ex.submit(verify_one, c, os.path.join(a.work, "v%d" % i)))
                for i, c in enumerate(cands)]
        for c, f in futs:
            row, why = f.result()
            (out if row else bad).append(row or (c["key"], why))
    with open(a.out, "w") as f:
        for r in sorted(out, key=lambda r: r["key"]):
            f.write(json.dumps(r, sort_keys=True) + "\n")
    print("verified", len(out), "dropped", len(bad))
    for b in bad:
        print("  not verified:", b)


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    sub = ap.add_subparsers(dest="cmd", required=True)
    pr = sub.add_parser("run")
    pr.add_argument("--out", required=True)
    pr.add_argument("--bound", type=int, required=True)
    pr.add_argument("--sample", type=int, default=0)
    pr.add_argument("--seed", type=int, default=1)
    pr.add_argument("--families", default="")
    pr.add_argument("--nest-owners", default="", help="comma list, family nest only (default c,cxx)")
    pr.add_argument("--extend-from", nargs="+", default=None,
                    help="result dirs: draw the single-fragment insertions of --extend-count of their programs")
    pr.add_argument("--extend-count", type=int, default=3000)
    pr.add_argument("--jobs", type=int, default=8)
    pr.add_argument("--chunk", type=int, default=400)
    pr.add_argument("--var-sample", type=float, default=0.003)
    pr.add_argument("--var-per-key", type=int, default=6)
    pr.set_defaults(fn=cmd_run)
    pk = sub.add_parser("keys")
    pk.add_argument("dirs", nargs="+")
    pk.set_defaults(fn=cmd_keys)
    pf = sub.add_parser("freeze")
    pf.add_argument("dirs", nargs="+")
    pf.add_argument("--out", required=True)
    pf.add_argument("--known-from")
    pf.set_defaults(fn=cmd_freeze)
    pv = sub.add_parser("verify")
    pv.add_argument("cands")
    pv.add_argument("--out", required=True)
    pv.add_argument("--work", default="/tmp/xgen-verify")
    pv.add_argument("--jobs", type=int, default=6)
    pv.set_defaults(fn=cmd_verify)
    a = ap.parse_args()
    a.fn(a)


if __name__ == "__main__":
    main()
