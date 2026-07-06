"""Part-D structural set-math scorer (design-of-record §5, §8.3). Pure set
arithmetic against a FROZEN, adjudicated `gold.json` — no LLM, no prism, no
judges. Mirrors investigator.py's small-module shape.

LSP is a candidate source + cross-check for `gold.json`'s AUTHORING (buildgold.py),
never the measurand here: this module only ever reads the frozen, adjudicated
gold set. Provenance/d_member ride along on each gold site for reporting, not
for scoring weight.
"""
from __future__ import annotations

import dataclasses
import json
import re
from dataclasses import dataclass, field
from pathlib import Path

_GENERIC = re.compile(r"<[^<>]*>")


def norm_symbol(raw: str) -> str:
    """Normalize a symbol string to a bare, casefolded name for site-key matching.

    Strips a leading receiver/type qualifier and generic parameters, then
    casefolds:
      '(*FastRegexMatcher).MatchString' -> 'matchstring'
      'TypeChecker::match_annotation'   -> 'match_annotation'
      'Foo<T>::bar'                     -> 'bar'
      'match_annotation'                -> 'match_annotation' (unchanged shape)
    """
    s = _GENERIC.sub("", raw or "").strip()
    if "::" in s:
        s = s.rsplit("::", 1)[-1]
    if "." in s:
        s = s.rsplit(".", 1)[-1]
    return s.strip().casefold()


def _bare_name(raw: str) -> str:
    """Bare identifier for TEXT-PRESENCE checks (verify_site_exists): strips the
    same receiver/scope qualifiers as norm_symbol but preserves case, since
    source text is case-sensitive."""
    s = _GENERIC.sub("", raw or "").strip()
    if "::" in s:
        s = s.rsplit("::", 1)[-1]
    if "." in s:
        s = s.rsplit(".", 1)[-1]
    return s.strip()


@dataclass(frozen=True)
class GoldSite:
    file: str
    symbol: str
    line: int
    provenance: str       # "both" | "lsp" | "prism"
    adjudication: str     # "real" | "excluded"
    d_member: str = "none"  # "D1" | "D2" | "none"
    reason: str = ""


@dataclass(frozen=True)
class ClaimedSite:
    """One arm-claimed impact site (from tier_c.impact.ImpactSite or a raw dict)."""
    file: str
    symbol: str
    reason: str = ""


@dataclass(frozen=True)
class StructuralReport:
    file_precision: float
    file_recall: float
    file_f1: float
    symbol_precision: float
    symbol_recall: float
    symbol_f1: float
    d_recall: float          # THE headline metric (design §5): recall over d_member in {D1,D2}
    precision: float         # alias of symbol_precision (report headline)
    phantom: int             # claimed sites verified NOT to exist in the checkout
    unmatched_extra: int     # claimed sites not in gold but NOT verified-nonexistent
    gold_size: int           # number of adjudication=="real" gold sites
    d_gold_size: int         # number of distinct D-subset gold FILES (d_recall denominator)
    claimed_size: int

    def to_dict(self) -> dict:
        return dataclasses.asdict(self)


def _as_gold_site(d) -> GoldSite:
    if isinstance(d, GoldSite):
        return d
    return GoldSite(
        file=d["file"], symbol=d["symbol"], line=int(d.get("line", 0)),
        provenance=d.get("provenance", "?"), adjudication=d["adjudication"],
        d_member=d.get("d_member", "none"), reason=d.get("reason", ""),
    )


def _as_claimed_site(c) -> ClaimedSite:
    if isinstance(c, ClaimedSite):
        return c
    if isinstance(c, dict):
        return ClaimedSite(file=c["file"], symbol=c["symbol"], reason=c.get("reason", ""))
    # Duck-typed object (e.g. tier_c.impact.ImpactSite)
    return ClaimedSite(file=c.file, symbol=c.symbol, reason=getattr(c, "reason", ""))


def _f1(p: float, r: float) -> float:
    return 0.0 if (p + r) == 0 else 2 * p * r / (p + r)


def _ratio(numer: int, denom: int) -> float:
    return numer / denom if denom else 0.0


def load_gold(path: str | Path) -> dict:
    """Load a frozen gold.json (the controller's adjudicated output). Pure I/O;
    no validation beyond valid JSON — score_structural validates shape via
    _as_gold_site."""
    return json.loads(Path(path).read_text())


def verify_site_exists(co, file: str, symbol: str) -> bool:
    """The phantom oracle (design §5, spec P3): True iff `file` exists in the
    checkout AND the bare symbol token appears somewhere in its text.

    This is a TEXT-PRESENCE check, not a claim of correctness — it exists only
    to distinguish a truly nonexistent claim (phantom) from a claim that is
    real but simply absent from gold (unmatched_extra, which is a precision
    miss routed to the adjudicated-extras channel, never a phantom penalty).
    `co` needs only `.file_exists(rel)` and `.read_text(rel)` (Checkout).
    """
    if not co.file_exists(file):
        return False
    if not symbol:
        return True
    text = co.read_text(file) or ""
    bare = _bare_name(symbol)
    if not bare:
        return True
    return re.search(rf"\b{re.escape(bare)}\b", text) is not None


def score_structural(claimed, gold: dict, *, verify_exists=None) -> StructuralReport:
    """Pure set-math scorer (design §5 metrics).

    claimed: list of ClaimedSite | dict{file,symbol,[reason]} | duck-typed
             ImpactSite objects (an arm's parsed impact set).
    gold:    the frozen gold.json dict (schema: {task_id,repo,sha,symbol,sites:[...]});
             only sites with adjudication=="real" count toward gold.
    verify_exists: optional callable(file, symbol) -> bool used to distinguish
             phantom (verified nonexistent) from unmatched_extra (not in gold
             but not verified-nonexistent). When omitted, no phantom detection
             is performed and every non-gold claim is unmatched_extra (spec:
             "claimed-but-real-and-not-in-gold is NOT a phantom").
    """
    gold_sites = [_as_gold_site(s) for s in gold.get("sites", [])]
    real = [s for s in gold_sites if s.adjudication == "real"]
    # Adjudicated-excluded sites (e.g. same-name/other-receiver collisions in the
    # exclusion table) are PHANTOM BAIT: claiming one is an adjudicated wrong answer,
    # scored as a phantom (stronger than unmatched_extra, which is for real-but-not-gold).
    excluded_syms = {(s.file, norm_symbol(s.symbol)) for s in gold_sites if s.adjudication == "excluded"}
    claims = [_as_claimed_site(c) for c in claimed]

    gold_files = {s.file for s in real}
    gold_syms = {(s.file, norm_symbol(s.symbol)) for s in real}
    d_gold_files = {s.file for s in real if s.d_member in ("D1", "D2")}

    claimed_files = {c.file for c in claims}
    claimed_syms = {(c.file, norm_symbol(c.symbol)) for c in claims}

    phantom = 0
    unmatched_extra = 0
    for c in claims:
        key = (c.file, norm_symbol(c.symbol))
        if key in gold_syms:
            continue
        if key in excluded_syms:
            phantom += 1  # adjudicated non-real (exclusion table) — a wrong answer, not just an extra
        elif verify_exists is not None and not verify_exists(c.file, c.symbol):
            phantom += 1
        else:
            unmatched_extra += 1

    file_tp = len(claimed_files & gold_files)
    file_p = _ratio(file_tp, len(claimed_files))
    file_r = _ratio(file_tp, len(gold_files))
    file_f1 = _f1(file_p, file_r)

    sym_tp = len(claimed_syms & gold_syms)
    sym_p = _ratio(sym_tp, len(claimed_syms))
    sym_r = _ratio(sym_tp, len(gold_syms))
    sym_f1 = _f1(sym_p, sym_r)

    d_tp = len(claimed_files & d_gold_files)
    d_recall = _ratio(d_tp, len(d_gold_files))

    return StructuralReport(
        file_precision=file_p, file_recall=file_r, file_f1=file_f1,
        symbol_precision=sym_p, symbol_recall=sym_r, symbol_f1=sym_f1,
        d_recall=d_recall, precision=sym_p,
        phantom=phantom, unmatched_extra=unmatched_extra,
        gold_size=len(real), d_gold_size=len(d_gold_files),
        claimed_size=len(claims),
    )
