#!/usr/bin/env python3
"""Lexer, macro database and owner locator for the L4 design-scoring harness (`xscore.py`,
#829 TEST-FORM section 4, L4). Python 3.9, stdlib only.

Everything here emulates what prism's C/C++ macro census and owner scan can see WITHOUT the
compiler's expansion: the census reads `#define` lines of every census input (the files given),
the owner scan reads the owner's own tokens. See XARM.md ("L4 scoring") for where the emulation
differs from the real census.
"""
import bisect
import collections
import re

Tok = collections.namedtuple("Tok", "kind text line pos")

TOKEN_RE = re.compile(r"""
 (?P<ws>[ \t\r\f\v]+)
|(?P<nl>\n)
|(?P<bc>/\*.*?(?:\*/|\Z))
|(?P<lc>//[^\n]*)
|(?P<raw>(?:u8|u|U|L)?R"(?P<rd>[^()\\\s"]{0,16})\(.*?\)(?P=rd)")
|(?P<str>(?:u8|u|U|L)?"(?:[^"\\\n]|\\.)*")
|(?P<chr>(?:u8|u|U|L)?'(?:[^'\\\n]|\\.)*')
|(?P<num>\.?\d(?:[eEpP][+-]|'\w|[\w.])*)
|(?P<id>(?:[^\W\d]|\$)[\w$]*)
|(?P<dig><%|%>|<:|:>|%:%:|%:)
|(?P<ell>\.\.\.)
|(?P<p>.)
""", re.X | re.S)
SPLICE_RE = re.compile(r"\\[ \t]*\r?\n")
DIGRAPH_BRACKETS = {"<%": "{", "%>": "}", "<:": "[", ":>": "]"}
CTRL_KW = frozenset("if else do while for switch case default goto return break continue".split())
DECL_KW = frozenset(("int char short long signed unsigned float double void bool _Bool struct union "
                     "enum static extern const volatile register typedef auto inline class template "
                     "typename constexpr").split())
BLOCK_MACROS = frozenset((
    "pthread_cleanup_push pthread_cleanup_pop pthread_cleanup_push_defer_np "
    "pthread_cleanup_pop_restore_np Py_BEGIN_ALLOW_THREADS Py_END_ALLOW_THREADS "
    "Py_BEGIN_CRITICAL_SECTION Py_END_CRITICAL_SECTION Py_BEGIN_CRITICAL_SECTION2 "
    "Py_END_CRITICAL_SECTION2 Py_BEGIN_CRITICAL_SECTION_MUTEX Py_BEGIN_CRITICAL_SECTION2_MUTEX "
    "Py_TRASHCAN_BEGIN Py_TRASHCAN_END Py_TRASHCAN_SAFE_BEGIN Py_TRASHCAN_SAFE_END").split())
PARSED_EXT = (".c", ".h", ".cc", ".cpp", ".cxx", ".hpp", ".hh", ".hxx")


def decode(data):
    """Bytes of a census input as text: UTF-16 with a BOM is decoded (the census does), anything
    else lossily (UTF-16 without a BOM is not readable, as in prism)."""
    if data[:2] in (b"\xff\xfe", b"\xfe\xff"):
        try:
            return data.decode("utf-16")
        except UnicodeDecodeError:
            pass
    if data[:3] == b"\xef\xbb\xbf":
        data = data[3:]
    return data.decode("utf-8", "replace")


def splice(text):
    """Line splices removed -> (text, sorted output offsets of the removed splices)."""
    offs, out, last, removed = [], [], 0, 0
    for m in SPLICE_RE.finditer(text):
        out.append(text[last:m.start()])
        removed += m.start() - last
        offs.append(removed)
        last = m.end()
    if not offs:
        return text, offs
    out.append(text[last:])
    return "".join(out), offs


def tokenize(s):
    """Tokens of a string without whitespace and comments (for a define line)."""
    out = []
    for m in TOKEN_RE.finditer(s):
        k = m.lastgroup
        if k in ("ws", "nl", "bc", "lc"):
            continue
        if k == "rd":
            k = "raw"
        out.append(Tok(_kind(m), m.group(), 0, m.start()))
    return out


def _kind(m):
    k = m.lastgroup
    if k == "rd":
        return "str"
    return {"raw": "str", "chr": "str", "p": "p", "ell": "p"}.get(k, k)


class Lexed:
    """tokens: code tokens (no whitespace, comments or directives) with `cond` pseudo tokens
    (`if`, `else`, `endif`) in place; directives: [(line, [Tok])]."""

    def __init__(self, tokens, directives, dig_pos):
        self.tokens, self.directives, self.dig_pos = tokens, directives, dig_pos


def lex(text):
    text, spl = splice(text)
    toks, dirs, dig_pos = [], [], set()
    n, pos, line, si, ns = len(text), 0, 1, 0, len(spl)
    bol = True
    cur = None  # directive being collected: [line, tokens]
    for m in TOKEN_RE.finditer(text):
        k = m.lastgroup
        pos = m.start()
        while si < ns and spl[si] <= pos:
            line += 1
            si += 1
        s = m.group()
        if k == "ws" or k == "lc":
            continue
        if k == "bc":
            line += s.count("\n")
            continue
        if k == "nl":
            if cur is not None:
                dirs.append((cur[0], cur[1]))
                _cond(cur, toks, line)
                cur = None
            bol = True
            line += 1
            continue
        kind = _kind(m)
        if kind == "str" and k in ("raw",):
            tk = Tok("str", s, line, pos)
            line += s.count("\n")
        else:
            tk = Tok(kind, s, line, pos)
        if cur is not None:
            cur[1].append(tk)
            continue
        if bol and kind in ("p", "dig") and s in ("#", "%:"):
            cur = [line, []]
            bol = False
            continue
        bol = False
        if kind == "dig":  # outside a directive a digraph is the bracket it spells
            c = DIGRAPH_BRACKETS.get(s)
            if c:
                dig_pos.add(pos)
                tk = Tok("p", c, line, pos)
            else:
                tk = Tok("p", "#" if s == "%:" else "##", line, pos)
        toks.append(tk)
    if cur is not None:
        dirs.append((cur[0], cur[1]))
        _cond(cur, toks, line)
    return Lexed(toks, dirs, dig_pos)


def _cond(cur, toks, line):
    ts = cur[1]
    if not ts or ts[0].kind != "id":
        return
    d = ts[0].text
    if d in ("if", "ifdef", "ifndef"):
        toks.append(Tok("cond", "if", cur[0], 0))
    elif d in ("elif", "else", "elifdef", "elifndef"):
        # pos 1 marks a plain `#else` (the branches are exhaustive); FileIndex never reads it
        toks.append(Tok("cond", "else", cur[0], 1 if d == "else" else 0))
    elif d == "endif":
        toks.append(Tok("cond", "endif", cur[0], 0))


# ---------------------------------------------------------------- macro database
DEF_RE = re.compile(r"(?m)^[ \t]*(?:#|%:)[ \t]*define[ \t]+")


class Facts:
    """Facts of one body (own) or of a macro closed over what it names."""
    __slots__ = ("unbalanced", "loose_comma", "semi0", "semi_any", "ctrl", "brace", "decl",
                 "free", "refs", "linear", "grouped", "groups")

    def __init__(self):
        self.unbalanced = self.loose_comma = self.semi0 = self.semi_any = False
        self.ctrl = self.brace = self.decl = False
        self.free = frozenset()
        self.refs = frozenset()
        self.grouped = frozenset()
        # paren groups of the body that follow an identifier, a parameter or the `)` of such a
        # group (DESIGN-v2 5.1): [Group]; read by the v2 candidates only
        self.groups = ()
        self.linear = True

    @property
    def statement_like(self):
        return self.semi_any or self.ctrl or self.brace

    def merge(self, o):
        for a in ("unbalanced", "loose_comma", "semi0", "semi_any", "ctrl", "brace", "decl"):
            setattr(self, a, getattr(self, a) or getattr(o, a))
        self.linear = self.linear and o.linear
        self.free = self.free | o.free
        self.grouped = self.grouped | o.grouped
        self.groups = self.groups + o.groups


class Group:
    """One paren group of a macro body. `pred`: ("param",) when the token before its `(` is a
    parameter or `__VA_ARGS__`, ("id", name) for any other identifier, ("chain", Group) for
    the `)` of another recorded group, None otherwise. `held`: a comma or `__VA_ARGS__` inside
    it (any depth) whose innermost enclosing bracket is `{` or `[`. `ids`: the identifiers
    inside it that are not parameters (any depth)."""
    __slots__ = ("pred", "held", "ids")

    def __init__(self, pred):
        self.pred, self.held, self.ids = pred, False, set()


def scan_body(toks, params, named_variadic, parsed):
    """The automaton of DESIGN section 6.1 plus the facts the other candidates use."""
    f = Facts()
    stack = []
    gstack = []      # the Group (or None) of every `(` / `T` entry of `stack`, in order
    groups = []
    last_closed = None  # (token index of the `)`, Group) of the group closed last
    pending_t = False
    free = set()
    grouped = set()
    uses = collections.Counter()
    n = len(toks)

    def tight():
        s = [x for x in stack if x != "T"]
        return bool(s) and s[-1] == "("

    for i, t in enumerate(toks):
        x = t.text
        if t.kind == "id":
            if x in CTRL_KW:
                f.ctrl = True
            if x in DECL_KW:
                f.decl = True
            if x == "__VA_OPT__" and i + 1 < n and toks[i + 1].text == "(":
                pending_t = True
            elif x == "__VA_ARGS__":
                uses["..."] += 1
                if not tight():
                    f.loose_comma = True
                    for g in gstack:
                        if g is not None:
                            g.held = True
            if x in params:
                uses[x] += 1
                if parsed:
                    continue
            free.add(x)
            if stack:
                grouped.add(x)
            if x != "__VA_ARGS__" and x != "__VA_OPT__":
                for g in gstack:
                    if g is not None:
                        g.ids.add(x)
        elif t.kind == "dig" and x in DIGRAPH_BRACKETS:
            f.unbalanced = True
            f.brace = True
        elif x == "(":
            g = None
            if not pending_t and i > 0:
                pt = toks[i - 1]
                if pt.kind == "id":
                    g = Group(("param",) if (pt.text in params or pt.text == "__VA_ARGS__")
                              else ("id", pt.text))
                elif last_closed is not None and last_closed[0] == i - 1:
                    g = Group(("chain", last_closed[1]))
                if g is not None:
                    groups.append(g)
            stack.append("T" if pending_t else "(")
            gstack.append(g)
            pending_t = False
        elif x == ")":
            if stack and stack[-1] in ("(", "T"):
                stack.pop()
                g = gstack.pop()
                if g is not None:
                    last_closed = (i, g)
            else:
                f.unbalanced = True
        elif x in "{[":
            stack.append(x)
            if x == "{":
                f.brace = True
        elif x in "}]":
            if stack and stack[-1] == ("{" if x == "}" else "["):
                stack.pop()
            else:
                f.unbalanced = True
            if x == "}":
                f.brace = True
        elif x == ",":
            if not tight():
                f.loose_comma = True
                if [e for e in stack if e != "T"]:   # held: the innermost bracket is `{` or `[`
                    for g in gstack:
                        if g is not None:
                            g.held = True
        elif x == ";":
            f.semi_any = True
            if not stack:
                f.semi0 = True
    if stack:
        f.unbalanced = True
    if named_variadic:
        f.loose_comma = True
    f.free = frozenset(free)
    f.grouped = frozenset(grouped)
    f.groups = tuple(groups)
    f.linear = all(uses[p] == 1 for p in params) and (uses["..."] <= 1)
    return f


class MacroDb:
    """Definitions of every census input. `own(name)` ORs the facts of every definition of
    the name (an `#undef` clears nothing); `closed(name)` also ORs over every macro reachable
    through free identifiers that are macros of the repository."""

    def __init__(self, texts):
        """texts: iterable of (path, text)."""
        self.defs = collections.defaultdict(list)
        for path, text in texts:
            parsed = path.lower().endswith(PARSED_EXT)
            self._scan(text, parsed)
        self._own, self._closed = {}, {}

    def _scan(self, text, parsed):
        if "define" not in text:
            return
        text, _ = splice(text)
        for m in DEF_RE.finditer(text):
            end = text.find("\n", m.end())
            line = text[m.end(): end if end >= 0 else len(text)]
            toks = tokenize(line)  # strings and comments are read by the lexer
            if not toks or toks[0].kind != "id":
                continue
            name = toks[0].text
            params, named_var, i = None, False, 1
            if len(toks) > 1 and toks[1].text == "(" and toks[1].pos == toks[0].pos + len(name):
                params, j = [], 2
                while j < len(toks) and toks[j].text != ")":
                    t = toks[j]
                    if t.kind == "id":
                        if j + 1 < len(toks) and toks[j + 1].text == "...":
                            named_var = True
                        params.append(t.text)
                    elif t.text == "...":
                        params.append("...")
                    j += 1
                i = j + 1
            body = toks[i:]
            f = scan_body(body, set(p for p in (params or []) if p != "..."), named_var, parsed)
            f.refs = frozenset()
            self.defs[name].append((params, f))

    def defined(self, name):
        return name in self.defs

    def own(self, name):
        r = self._own.get(name)
        if r is None:
            r = Facts()
            for _, f in self.defs.get(name, ()):
                r.merge(f)
            self._own[name] = r
        return r

    def reach(self, name):
        """Names reachable from `name` through free identifiers that are macros (itself included)."""
        seen, todo = set(), [name]
        while todo:
            x = todo.pop()
            if x in seen:
                continue
            seen.add(x)
            for y in self.own(x).free:
                if y in self.defs and y not in seen:
                    todo.append(y)
        return seen

    def closed(self, name):
        r = self._closed.get(name)
        if r is not None:
            return r
        seen, todo = set(), [name]
        out = Facts()
        while todo:
            x = todo.pop()
            if x in seen:
                continue
            seen.add(x)
            o = self.own(x)
            out.merge(o)
            for y in o.free:
                if y in self.defs and y not in seen:
                    todo.append(y)
        self._closed[name] = out
        return out


# ---------------------------------------------------------------- owners
CONTROL_HEAD = frozenset("if for while switch catch __attribute__ __declspec alignas".split())
ATTR_HEAD = frozenset("__attribute__ __declspec alignas noexcept throw __asm__ asm".split())
QUAL = frozenset("const noexcept override final volatile mutable".split())
TYPE_KW = frozenset("struct class union namespace enum extern".split())


class Fn:
    __slots__ = ("open", "close", "hdr", "kind", "name_idx")

    def __init__(self, open_, close, hdr, name_idx):
        self.open, self.close, self.hdr, self.name_idx = open_, close, hdr, name_idx


class FileIndex:
    """Tokens of one file, the file-level open-parenthesis depth before each token, and the
    function bodies found by a token-level locator (no parser: a `{` after a `)` whose
    matching `(` follows a non-control identifier, outside another function body, or directly in
    a class-like block)."""

    def __init__(self, text):
        lx = lex(text)
        self.lexed = lx
        self.dig_pos = lx.dig_pos
        code, depth, post, conds = [], [], [], []
        d, r, cs = 0, 0, []  # clamped depth, unclamped balance, conditional stack
        for t in lx.tokens:
            if t.kind == "cond":
                conds.append((len(code), t.text))
                if t.text == "if":
                    cs.append([d, None, r, None])
                elif t.text == "else" and cs:
                    if cs[-1][1] is None:
                        cs[-1][1], cs[-1][3] = d, r
                    d, r = cs[-1][0], cs[-1][2]
                elif t.text == "endif" and cs:
                    top = cs.pop()
                    if top[1] is not None:
                        d, r = top[1], top[3]
                continue
            code.append(t)
            depth.append(d)
            if t.kind == "p" and t.text == "(":
                d += 1
                r += 1
            elif t.kind == "p" and t.text == ")":
                r -= 1
                if d > 0:
                    d -= 1
            post.append(r)
        # sufmin[i]: the least balance after any token at or beyond i (an unmatched `)` later in
        # the file makes it fall below the balance at the owner's end)
        sufmin = [0] * (len(code) + 1)
        m = 1 << 60
        for i in range(len(code) - 1, -1, -1):
            m = min(m, post[i])
            sufmin[i] = m
        sufmin[len(code)] = 1 << 60
        self.code, self.depth, self.post, self.sufmin, self.conds = code, depth, post, sufmin, conds
        self.fns = self._functions()

    def _functions(self):
        code = self.code
        pm, pst = {}, []
        for i, t in enumerate(code):
            if t.text == "(":
                pst.append(i)
            elif t.text == ")" and pst:
                pm[i] = pst.pop()
        found, st = {}, []  # st: [open idx, kind, name_idx, hdr]
        # conditional compilation: both branches of an `#if` are read from the stack as it was
        # at the `#if`, and the stack after the `#endif` is the first branch's end (the same
        # heuristic as the parenthesis depth)
        cs, ci, nc = [], 0, len(self.conds)
        for i, t in enumerate(code):
            while ci < nc and self.conds[ci][0] <= i:
                kind = self.conds[ci][1]
                ci += 1
                if kind == "if":
                    cs.append([list(st), None])
                elif kind == "else" and cs:
                    if cs[-1][1] is None:
                        cs[-1][1] = list(st)
                    st = list(cs[-1][0])
                elif kind == "endif" and cs:
                    top = cs.pop()
                    if top[1] is not None:
                        st = top[1]
            if t.text == "{" and t.kind == "p":
                kind, nidx, hdr = self._classify(i, pm)
                allowed = kind == "fn" and (not any(e[1] == "fn" for e in st)
                                            or (st and st[-1][1] in ("type", "ns")))
                st.append([i, kind if kind != "fn" or allowed else "blk", nidx, hdr])
            elif t.text == "}" and t.kind == "p" and st:
                o = st.pop()
                if o[1] == "fn":
                    prev = found.get(o[0])
                    if prev is None or prev.close < i:
                        found[o[0]] = Fn(o[0], i, o[3], o[2])
        return sorted(found.values(), key=lambda f: f.open)

    def _classify(self, i, pm):
        code = self.code
        j = i - 1
        if j < 0:
            return "blk", None, None
        if code[j].text in ("=", ",", "(", "return", "?", ":") and code[j].kind in ("p", "id"):
            return "blk", None, None
        for _ in range(8):
            while j >= 0 and ((code[j].kind == "id" and code[j].text in QUAL)
                              or code[j].text in ("&", "*")):
                j -= 1
            if j >= 0 and code[j].text == ")" and j in pm:
                o = pm[j]
                k = o - 1
                if k >= 0 and code[k].kind == "id" and code[k].text in ATTR_HEAD:
                    j = k - 1
                    continue
                if k >= 0 and code[k].kind == "id" and code[k].text not in CONTROL_HEAD \
                        and code[k].text not in CTRL_KW:
                    return "fn", k, self._hdr(o)
                if k >= 0 and code[k].text in (")", ">", "]") and code[k].text != "]":
                    return "fn", k, self._hdr(o)
                return "blk", None, None
            break
        # a class-like block: a header holding struct/class/union/enum/namespace/extern
        h, steps = i - 1, 0
        while h >= 0 and steps < 64:
            x = code[h]
            if x.text in (";", "{", "}") and x.kind == "p":
                break
            if x.kind == "id" and x.text in TYPE_KW:
                return ("ns" if x.text in ("namespace", "extern") else "type"), None, None
            h -= 1
            steps += 1
        return "blk", None, None

    def _hdr(self, o):
        """First token of the declaration whose parameter list opens at `o`."""
        code = self.code
        h, d = o - 1, 0
        while h >= 0:
            t = code[h]
            if t.kind == "p":
                if t.text == ")":
                    d += 1
                elif t.text == "(":
                    d -= 1
                elif d <= 0 and t.text in (";", "{", "}"):
                    break
            h -= 1
        return h + 1

    def owner_at(self, line):
        """The innermost located function whose lines hold `line`."""
        best = None
        for fn in self.fns:
            lo, hi = self.code[fn.hdr].line, self.code[fn.close].line
            if lo <= line <= hi and (best is None or hi - lo <= best[0]):
                best = (hi - lo, fn)
        return best[1] if best else None

    def owner_for_start(self, start_line):
        """The innermost function whose declaration starts at or before `start_line` and whose
        body opens at or after it (prism's owner start line)."""
        best = None
        for fn in self.fns:
            lo, op, hi = self.code[fn.hdr].line, self.code[fn.open].line, self.code[fn.close].line
            if lo <= start_line <= op and (best is None or hi - lo <= best[0]):
                best = (hi - lo, fn)
        return best[1] if best else None


class Owner:
    """What an owner-refusal predicate may look at: the owner's tokens and spans, and the
    file-scope context around it (never the compiler's expansion)."""

    def __init__(self, fx, fn, file=""):
        self.file = file
        self.fx, self.fn = fx, fn
        self.toks = fx.code[fn.hdr: fn.close + 1]
        self.start_line = fx.code[fn.hdr].line
        self.end_line = fx.code[fn.close].line
        # inside an open group: an unclosed `(` before the owner, or a `)` after it that nothing
        # in the file opens (the opener is hidden in a macro: `BEGIN_FNS static int d; f() {..} )`)
        self.enclosing_open = fx.depth[fn.hdr] > 0 or fx.sufmin[fn.close + 1] < fx.post[fn.close]
        self.name = fx.code[fn.name_idx].text if fn.name_idx is not None else ""
        self.spelled = frozenset(t.text for t in self.toks if t.kind == "id")
        self.digs = frozenset(t.pos for t in self.toks if t.pos in fx.dig_pos)
        self._inv = None

    def invocations(self, db):
        """[(macro name, [groups])] for every identifier of the owner that names an in-repo
        macro and is followed by `(`; a group is the token list between the parentheses (a
        parenthesis match by parentheses only, as the preprocessor splits), chained groups
        (`M(a)(b)`) included."""
        if self._inv is None:
            out, toks, n = [], self.toks, len(self.toks)
            for i, t in enumerate(toks):
                if t.kind != "id" or not db.defined(t.text):
                    continue
                groups, j = [], i + 1
                while j < n and toks[j].text == "(" and toks[j].kind == "p":
                    d, k = 0, j
                    while k < n:
                        if toks[k].kind == "p" and toks[k].text == "(":
                            d += 1
                        elif toks[k].kind == "p" and toks[k].text == ")":
                            d -= 1
                            if d == 0:
                                break
                        k += 1
                    groups.append(toks[j + 1: k])
                    j = k + 1
                if groups:
                    out.append((t.text, groups))
            self._inv = out
        return self._inv
