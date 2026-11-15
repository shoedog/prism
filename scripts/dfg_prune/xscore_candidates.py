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
