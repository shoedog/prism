#!/usr/bin/env python3
"""Candidate designs for the L4 scorer (`xscore.py`): each is an OWNER-REFUSAL PREDICATE

    def predicate(owner, db) -> None | str

returning None to ADMIT the owner (U and D stay on: its pairs are as on the T arm) or a reason
string to REFUSE it (U and D off: its pairs are as on the Tb baseline arm). The reason's text
before the first `:` is the gate category the report groups by.

A predicate may read only what prism has without the compiler's expansion:
  owner.toks / owner.spelled / owner.start_line / owner.end_line   the owner's own tokens
  owner.enclosing_open        the owner lies inside an unclosed `(` at file scope
  owner.invocations(db)       the in-repo macro invocations and their argument groups
  db.defined(n) db.own(n) db.closed(n)   census facts from the definitions of the census inputs
  BLOCK_MACROS                the external name list of DESIGN section 6.3 (OD-3)
It must NOT use the clang expansion (that is the oracle). To add a candidate, define a function
and register it in CANDIDATES (an ordered dict name -> (predicate, one-line description)); a
module of your own can be loaded with `xscore.py --candidates path.py`.
"""
import collections

from xscore_lex import BLOCK_MACROS

OPEN = {")": "(", "]": "[", "}": "{"}


def arg_split(owner, db, depth1=False):
    """DESIGN G9: a comma inside the argument group of an in-repo macro whose innermost enclosing
    bracket is `{` or `[` (`depth1`: only when that bracket is directly in the group's top level,
    the known-wrong variant of G2-opus-02); a mismatched closer or a digraph bracket inside a
    group counts too."""
    for name, groups in owner.invocations(db):
        for g in groups:
            stack = []
            for t in g:
                if t.pos in owner.digs:
                    return "arg_split_digraph:" + name
                if t.kind != "p":
                    continue
                x = t.text
                if x in "([{":
                    stack.append(x)
                elif x in OPEN:
                    if stack and stack[-1] == OPEN[x]:
                        stack.pop()
                    else:
                        return "arg_split_mismatch:" + name
                elif x == "," and stack and stack[-1] in "{[":
                    if not depth1 or len(stack) == 1:
                        return "arg_split:" + name
    return None


def tentative(owner, db, closure=True, listed=True, comma=True, split="any", enclosing=False):
    """DESIGN section 6 as amended: G7 (an in-repo macro, closed, with an unbalanced body; a
    name of BLOCK_MACROS; an in-repo macro whose transitive free names hold a BLOCK_MACROS
    name), G8 (loose comma, closed), G9 (a brace- or bracket-held comma in an argument group).
    Options: `closure` False = facts of the spelled macro's own definitions only (no
    propagation, no wrapper closure); `listed` False = no BLOCK_MACROS check; `split` None /
    "any" / "depth1"; `enclosing` True adds the open-group check (4b)."""
    if listed:
        for n in sorted(owner.spelled & BLOCK_MACROS):
            return "block_list:" + n
    for n in sorted(owner.spelled):
        if not db.defined(n):
            continue
        f = db.closed(n) if closure else db.own(n)
        if f.unbalanced:
            return "unbalanced:" + n
        if closure and listed and f.free & BLOCK_MACROS:
            return "block_wrapper:" + n
    if comma:
        for n in sorted(owner.spelled):
            if db.defined(n) and (db.closed(n) if closure else db.own(n)).loose_comma:
                return "loose_comma:" + n
    if split:
        r = arg_split(owner, db, depth1=(split == "depth1"))
        if r:
            return r
    if enclosing and owner.enclosing_open:
        return "enclosing_group"
    return None


def cand_tentative(owner, db):
    """1. The tentative rule. It has NO enclosing-group check (confirmed: `enclosing=False`)."""
    return tentative(owner, db)


def cand_no_closure(owner, db):
    """2. The tentative rule without closure (own facts only, no wrapper closure); the list stays."""
    return tentative(owner, db, closure=False)


def cand_spelling_only(owner, db):
    """3. Spelling only: own facts of the macros the owner spells, G9, no closure, no list."""
    return tentative(owner, db, closure=False, listed=False)


def cand_tentative_enclosing(owner, db):
    """4b. Candidate 1 plus: the owner lies inside an open macro argument group at file scope."""
    return tentative(owner, db, enclosing=True)


def cand_any_macro(owner, db):
    """5. Any macro token refuses: the owner spells an in-repo macro name or a name of
    BLOCK_MACROS (external macros other than the list are invisible to prism, as to this)."""
    for n in sorted(owner.spelled):
        if db.defined(n) or n in BLOCK_MACROS:
            return "any_macro:" + n
    return None


def certificate(owner, db, split=None):
    """6. Inertness certificate: admit only when the owner is not inside an open group, spells no
    listed name, and every in-repo macro it spells (closed over the macros it names) is
    balanced, comma-tight, `;`-free, brace-free, control-free and declaration-keyword-free
    (expression-shaped); `split` adds the argument-group check of G9."""
    if owner.enclosing_open:
        return "enclosing_group"
    for n in sorted(owner.spelled & BLOCK_MACROS):
        return "block_list:" + n
    for n in sorted(owner.spelled):
        if not db.defined(n):
            continue
        f = db.closed(n)
        if f.free & BLOCK_MACROS:
            return "block_wrapper:" + n
        if f.unbalanced:
            return "cert_unbalanced:" + n
        if f.loose_comma:
            return "cert_loose_comma:" + n
        if f.semi_any:
            return "cert_semicolon:" + n
        if f.ctrl:
            return "cert_control:" + n
        if f.brace:
            return "cert_brace:" + n
        if f.decl:
            return "cert_declaration:" + n
    if split:
        return arg_split(owner, db)
    return None


def cand_certificate(owner, db):
    return certificate(owner, db)


def cand_certificate_split(owner, db):
    """6b. The certificate plus the argument-group check (a swapped or dropped argument)."""
    return certificate(owner, db, split="any")


def cand_depth1(owner, db):
    """7. Brace-held commas only at depth 1 (known wrong, G2-opus-02)."""
    return tentative(owner, db, split="depth1")


def loose_in_group(owner, db):
    """A finer G8 (exploratory, not in the task's list): a macro with a loose comma only matters
    when its expansion can be re-split, that is when it is spelled inside an argument group of an
    in-repo macro at the owner, or inside a parenthesis, brace or bracket of the body of a macro
    the owner (transitively) spells."""
    for name, groups in owner.invocations(db):
        for g in groups:
            for t in g:
                if t.kind == "id" and db.defined(t.text) and db.closed(t.text).loose_comma:
                    return "loose_in_group:" + t.text
    for n in sorted(owner.spelled):
        if not db.defined(n):
            continue
        for r in db.reach(n):
            for gid in db.own(r).grouped:
                if db.defined(gid) and db.closed(gid).loose_comma:
                    return "loose_in_body:" + n
    return None


def cand_fine_g8(owner, db):
    """8. 4b with the loose-comma fact replaced by the finer, site-and-body check."""
    r = tentative(owner, db, comma=False, split=None)
    if r:
        return r
    r = loose_in_group(owner, db)
    if r:
        return r
    r = arg_split(owner, db)
    if r:
        return r
    return "enclosing_group" if owner.enclosing_open else None


def cand_abl_no_comma(owner, db):
    """A1 (ablation of 4b): without G8 (the loose-comma fact)."""
    return tentative(owner, db, comma=False, enclosing=True)


def cand_abl_no_split(owner, db):
    """A2 (ablation of 4b): without G9 (the argument-group site check)."""
    return tentative(owner, db, split=None, enclosing=True)


def cand_abl_no_list(owner, db):
    """A3 (ablation of 4b): without BLOCK_MACROS and the wrapper closure (the OD-3 list)."""
    return tentative(owner, db, listed=False, enclosing=True)


def cand_abl_unbalanced_only(owner, db):
    """A4 (ablation of 4b): only the unbalanced fact (closed) and the enclosing check: no
    list, no comma fact, no site check."""
    return tentative(owner, db, listed=False, comma=False, split=None, enclosing=True)


def cand_admit_all(owner, db):
    """0. Nothing new: admit every owner (the head with stub gates; red on base)."""
    return None


def cand_refuse_all(owner, db):
    """9. The other extreme: refuse every owner (the baseline arm; bounds the recall axis)."""
    return "refuse_all"


# ---------------------------------------------------------------- DESIGN-v2 (candidate 10)
# The structure-preservation certificate of DESIGN-v2.md. Facts per macro (closed over the macros
# a body names, like the tentative facts):
#   unbalanced*  some reachable body does not nest (as 1)
#   loose*       some reachable body holds a comma or `__VA_ARGS__` whose innermost bracket is
#                not a parenthesis: its expansion can carry a comma that is not inside its own
#                parentheses (not "comma-clean")
#   split*       some reachable body holds a CALLEE GROUP (a paren group that follows a macro
#                name, a parameter, or the `)` of a callee group) with a held comma (innermost
#                bracket `{` or `[`, any depth) or with a macro whose expansion is loose*
INF = 1 << 60


def _callee(g, db, mode):
    """Is body group `g` a callee group? `mode` "macro": its predecessor is a parameter, an
    in-repo macro name (any definition) or the `)` of a callee group; "any": any identifier."""
    while True:
        if g is None or g.pred is None:
            return False
        k = g.pred[0]
        if k == "param":
            return True
        if k == "id":
            return mode == "any" or db.defined(g.pred[1])
        g = g.pred[1]   # chain


def _split_own(name, db, mode, held=True, fwd=True):
    memo = db.__dict__.setdefault("_v2_split_own", {})
    key = (name, mode, held, fwd)
    r = memo.get(key)
    if r is None:
        r = held and db.own(name).forced_split
        for g in db.own(name).groups:
            if not _callee(g, db, mode):
                continue
            if held and g.held:
                r = True
                break
            if fwd and any(db.defined(n) and db.closed(n).loose_comma for n in g.ids):
                r = True
                break
        memo[key] = r
    return r


def _split_closed(name, db, mode, held=True, fwd=True):
    memo = db.__dict__.setdefault("_v2_split_closed", {})
    key = (name, mode, held, fwd)
    r = memo.get(key)
    if r is None:
        r = any(_split_own(m, db, mode, held, fwd) for m in db.reach(name))
        memo[key] = r
    return r


def _hull(fx):
    """Per code token: the interval of the file-level parenthesis balance over the branches of
    conditional compilation (unclamped; `#if` without a plain `#else` may take no branch)."""
    h = getattr(fx, "_v2_hull", None)
    if h is not None:
        return h
    n = len(fx.code)
    lob, hib, loa, hia = [0] * n, [0] * n, [0] * n, [0] * n
    lo = hi = 0
    cs = []  # [base lo, base hi, [branch ends], saw a plain else]
    i = 0
    for t in fx.lexed.tokens:
        if t.kind == "cond":
            if t.text == "if":
                cs.append([lo, hi, [], False])
            elif t.text == "else" and cs:
                cs[-1][2].append((lo, hi))
                if t.pos == 1:
                    cs[-1][3] = True
                lo, hi = cs[-1][0], cs[-1][1]
            elif t.text == "endif" and cs:
                b = cs.pop()
                ends = b[2] + [(lo, hi)]
                if not b[3]:
                    ends.append((b[0], b[1]))
                lo, hi = min(e[0] for e in ends), max(e[1] for e in ends)
            continue
        lob[i], hib[i] = lo, hi
        if t.kind == "p" and t.text == "(":
            lo += 1
            hi += 1
        elif t.kind == "p" and t.text == ")":
            lo -= 1
            hi -= 1
        loa[i], hia[i] = lo, hi
        i += 1
    pre = [0] * (n + 1)   # pre[i]: least lower bound before token i (0 at the file start)
    m = 0
    for i in range(n):
        pre[i] = m
        m = min(m, loa[i])
    pre[n] = m
    suf = [INF] * (n + 2)  # suf[i]: least lower bound after any token at or beyond i
    m = INF
    for i in range(n - 1, -1, -1):
        m = min(m, loa[i])
        suf[i] = m
    fx._v2_hull = (hib, pre, suf, hia)
    return fx._v2_hull


def exposed(owner):
    """DESIGN-v2 5.3: the owner may lie inside a macro argument list that starts outside it: in
    some configuration a literal `(` before the owner is still open at its start, or a `)`
    after the owner closes something that no literal `(` after the owner opened."""
    hib, pre, suf, hia = _hull(owner.fx)
    a, b = owner.fn.hdr, owner.fn.close
    if hib[a] > pre[a]:
        return "exposed_open_before"
    if suf[b + 1] < hia[b]:
        return "exposed_close_after"
    return None


def hidden_opener(owner, db):
    """DESIGN-v2 5.3, third half of G10 (owner decision OD-C): the owner's file spells, in code
    before the owner, an in-repo macro whose expansion can open a parenthesis (`paren_open`,
    closed over the macros a body names). The argument list it opens may hold the owner and be
    closed in the file that includes this one, where no test on this file can see the `)`."""
    fx = owner.fx
    memo = fx.__dict__.setdefault("_v2_hidden", {})
    first = memo.get(id(db))
    if first is None:
        first = (INF, "")
        for i, t in enumerate(fx.code):
            if t.kind == "id" and db.defined(t.text) and db.closed(t.text).paren_open:
                first = (i, t.text)
                break
        memo[id(db)] = first
    return "exposed_hidden_opener:" + first[1] if first[0] < owner.fn.hdr else None


def owner_groups(owner, db, held=True, loose=True, shield=True, any_ident=False):
    """Historical candidate 10 and amended DESIGN-v2 5.2, sharing a bracket-stack pass.

    Candidate 12 uses any_ident=True, shield=False: EVERY identifier-following group and
    its chains are potential callees, with no positions exempted. The old shield reasoning
    below describes the REJECTED candidate only, retained for differential scoring.
    A paren is a
    CALLEE paren after an in-repo macro name or after the `)` of a callee paren (a chained
    group); a SHIELDED paren when it is opened INSIDE a callee paren directly after an identifier
    that is no in-repo macro (`shield`: it can never delimit a macro's arguments, so it stays in
    the compiler's token stream as a unit of the enclosing argument, and its contents must nest
    in every program the compiler accepts); a plain paren otherwise. A token inside at least one
    callee paren and inside no shielded paren must not be a held comma (innermost bracket `{` or
    `[`) nor an in-repo macro whose expansion is loose*; a mismatched closer or a digraph
    bracket inside a callee paren refuses. A paren after an identifier OUTSIDE every callee
    paren shields nothing: a callee group inside it is an invocation of the owner's own text."""
    stack = []          # "C" callee paren, "S" shielded paren, "(" plain paren, "{", "["
    prev = None
    closed = None       # (token, kind) of the paren closed last
    ncallee = nshield = 0
    for t in owner.toks:
        exposed = ncallee > 0 and nshield == 0
        if t.kind == "p":
            x = t.text
            if ncallee and t.pos in owner.digs:
                return "arg_split_digraph"
            if x == "(":
                if prev is not None and prev.kind == "id":
                    k = "C" if any_ident or db.defined(prev.text) else ("S" if shield and ncallee else "(")
                elif closed is not None and closed[0] is prev and closed[1] == "C":
                    k = "C"
                else:
                    k = "("
                stack.append(k)
                ncallee += k == "C"
                nshield += k == "S"
            elif x in "{[":
                stack.append(x)
            elif x in OPEN:
                want = OPEN[x]
                top = stack[-1] if stack else None
                if top == want or (want == "(" and top in ("C", "S")):
                    stack.pop()
                    ncallee -= top == "C"
                    nshield -= top == "S"
                    closed = (t, top)
                elif ncallee:
                    return "arg_split_mismatch"
            elif x == "," and held and exposed and stack[-1] in "{[":
                return "arg_split"
        elif t.kind == "id" and loose and exposed and db.defined(t.text) \
                and db.closed(t.text).loose_comma:
            return "loose_in_owner_group:" + t.text
        prev = t
    return None


# DESIGN-v2 section 7 (OWNER-DECISION OD-B): names a scan of the public headers found with a
# replacement list that does not nest (CPython 3.11 to 3.14, NumPy 2.x), plus the PostgreSQL
# error-handling macros of the fixture preludes; added to the 16 names of OD-3.
BLOCK_MACROS_ADDED = frozenset((
    "Py_TRASHCAN_BEGIN_CONDITION Py_BEGIN_CRITICAL_SECTION_SEQUENCE_FAST "
    "Py_END_CRITICAL_SECTION_SEQUENCE_FAST _Py_BEGIN_SUPPRESS_IPH _Py_END_SUPPRESS_IPH "
    "NPY_BEGIN_ALLOW_THREADS NPY_END_ALLOW_THREADS NPY_BEGIN_SUPPRESS_IPH NPY_END_SUPPRESS_IPH "
    "PG_TRY PG_CATCH PG_FINALLY PG_END_TRY PG_ENSURE_ERROR_CLEANUP "
    "PG_END_ENSURE_ERROR_CLEANUP").split())


def sp_certificate(owner, db, mode="macro", body_held=True, body_fwd=True, owner_held=True,
                   owner_loose=True, expose="hull", listed=True, unbalanced=True, shield=True,
                   names=None, hidden=True, any_ident=False):
    """Shared evaluator for the historical and amended structure-preservation candidates."""
    BLOCK_MACROS = names if names is not None else globals()["BLOCK_MACROS"]
    if listed:
        for n in sorted(owner.spelled & BLOCK_MACROS):
            return "block_list:" + n
    for n in sorted(owner.spelled):
        if not db.defined(n):
            continue
        f = db.closed(n)
        if unbalanced and f.unbalanced:
            return "unbalanced:" + n
        if listed and f.free & BLOCK_MACROS:
            return "block_wrapper:" + n
        if (body_held or body_fwd) and _split_closed(n, db, mode, body_held, body_fwd):
            return "body_split:" + n
    r = owner_groups(owner, db, held=owner_held, loose=owner_loose, shield=shield,
                     any_ident=any_ident)
    if r:
        return r
    if expose == "hull":
        r = exposed(owner)
    elif expose == "first":
        r = "enclosing_group" if owner.enclosing_open else None
    else:
        r = None
    if r is None and hidden and expose:
        r = hidden_opener(owner, db)
    return r


def cand_v2(owner, db):
    """10. Rejected historical DESIGN-v2 rule; retained as a regression control."""
    return sp_certificate(owner, db)


def cand_v2_any(owner, db):
    """10i. 10 with a body paren group read as a callee group after ANY identifier (10 itself:
    after a parameter, an in-repo macro name or a chained callee group), so the facts would not
    depend on which names are macros."""
    return sp_certificate(owner, db, mode="any")


def cand_v2_noshield(owner, db):
    """10n. 10 without the shield: every position of an owner callee group is checked."""
    return sp_certificate(owner, db, shield=False)


def cand_v2_first(owner, db):
    """10f. 10 with the scorer's first-branch-wins enclosing test instead of the interval test."""
    return sp_certificate(owner, db, expose="first")


def cand_v2_semi0(owner, db):
    """10s. 10 plus: the owner spells a macro (closed) whose body holds a top-level `;` (the
    #831 shape; measured for the owner's information, not part of the rule)."""
    r = sp_certificate(owner, db)
    if r:
        return r
    for n in sorted(owner.spelled):
        if db.defined(n) and db.closed(n).semi0:
            return "semi0:" + n
    return None


def cand_v2_list(owner, db):
    """10x. 10 with the extended external list (OD-B: the 16 names plus 15)."""
    return sp_certificate(owner, db, names=BLOCK_MACROS | BLOCK_MACROS_ADDED)


def _abl(**kw):
    def f(owner, db):
        return sp_certificate(owner, db, **kw)
    return f


V2 = collections.OrderedDict([
    ("10-v2-certificate", (cand_v2, "historical DESIGN-v2 certificate (rejected)")),
    ("10i-v2-any-ident", (cand_v2_any, "10 with body callee groups after any identifier")),
    ("10n-v2-no-shield", (cand_v2_noshield, "10 without shielded parentheses in owner groups")),
    ("10f-v2-first-branch", (cand_v2_first, "10 with the first-branch-wins enclosing test")),
    ("10x-v2+list31", (cand_v2_list, "10 with the extended external list (16 + 15 names, OD-B)")),
    ("10s-v2+semi0", (cand_v2_semi0, "10 + a top-level `;` in a reachable body (#831 shape), for information")),
    ("B1-10-no-unbalanced", (_abl(unbalanced=False), "ablation of 10: without unbalanced*")),
    ("B2-10-no-list", (_abl(listed=False), "ablation of 10: without the list and wrapper closure")),
    ("B3-10-no-owner-held", (_abl(owner_held=False), "ablation of 10: without the held-comma check of owner groups")),
    ("B4-10-no-owner-loose", (_abl(owner_loose=False), "ablation of 10: without the loose-macro check of owner groups")),
    ("B5-10-no-body-held", (_abl(body_held=False), "ablation of 10: without the held-comma check of body callee groups")),
    ("B6-10-no-body-fwd", (_abl(body_fwd=False), "ablation of 10: without the loose-macro check of body callee groups")),
    ("B7-10-no-exposed", (_abl(expose=None), "ablation of 10: without the exposed-owner test")),
    ("B8-10-no-hidden-opener", (_abl(hidden=False), "ablation of 10: without the hidden-opener half of G10 (OD-C)")),
])


CANDIDATES = collections.OrderedDict([
    ("0-admit-all", (cand_admit_all, "nothing new: admit every owner (red on base)")),
    ("1-tentative", (cand_tentative, "tentative rule (DESIGN 6 as amended), no enclosing check")),
    ("2-no-closure", (cand_no_closure, "tentative without closure (own facts, no wrapper closure)")),
    ("3-spelling-only", (cand_spelling_only, "spelling only: own facts, no closure, no list")),
    ("4b-tentative+enclosing", (cand_tentative_enclosing, "1 + owner inside an open macro argument group")),
    ("5-any-macro", (cand_any_macro, "any macro token refuses (in-repo names + the 16 listed)")),
    ("6-certificate", (cand_certificate, "inertness certificate (closed bodies expression-shaped) + not in an open group")),
    ("6b-certificate+split", (cand_certificate_split, "6 + argument-group check")),
    ("7-depth1", (cand_depth1, "brace-held commas only at depth 1 (known wrong)")),
    ("9-refuse-all", (cand_refuse_all, "refuse every owner (the recall bound)")),
    ("8-fine-G8", (cand_fine_g8, "exploratory: 4b with G8 replaced by loose-comma-macro-in-a-group (site and body)")),
    ("A1-4b-no-G8", (cand_abl_no_comma, "ablation: 4b without the loose-comma fact")),
    ("A2-4b-no-G9", (cand_abl_no_split, "ablation: 4b without the argument-group site check")),
    ("A3-4b-no-list", (cand_abl_no_list, "ablation: 4b without BLOCK_MACROS and the wrapper closure")),
    ("A4-4b-unbalanced-only", (cand_abl_unbalanced_only, "ablation: unbalanced (closed) + enclosing only")),
])
CANDIDATES.update(V2)


def cand_v2_noshield_list(owner, db):
    """11. Review repair baseline: 10n with OD-B's 31 names."""
    return sp_certificate(owner, db, shield=False, names=BLOCK_MACROS | BLOCK_MACROS_ADDED)


def cand_v2_all_roots(owner, db):
    """12. No shields; every identifier can introduce a callee, in bodies and owners.

    Non-membership in the census proves neither survival nor non-aliasing. Historical
    candidates retain their decision rules; shared fact corrections apply to all of them.
    """
    return sp_certificate(owner, db, shield=False, mode="any", any_ident=True,
                          names=BLOCK_MACROS | BLOCK_MACROS_ADDED)


CANDIDATES.update([
    ("11-no-shield-list31", (cand_v2_noshield_list, "10n with 31 names; review repair control")),
    ("12-all-roots", (cand_v2_all_roots, "no shields, all identifier roots, 31 names")),
    ("B9-12-known-owner-roots", (
        _abl(shield=False, mode="any", names=BLOCK_MACROS | BLOCK_MACROS_ADDED),
        "ablation of 12: only known owner roots; external_identity reopens")),
    ("B10-12-known-body-roots", (
        _abl(shield=False, any_ident=True, names=BLOCK_MACROS | BLOCK_MACROS_ADDED),
        "ablation of 12: only known body roots; body_identity reopens")),
])
