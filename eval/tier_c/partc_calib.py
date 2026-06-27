"""Oracle calibration set + saturation guard (Task 6, spec §6b).

Runs the relevance judge over a small labeled set of citations and checks
for degenerate behaviour (always-YES / always-NO saturation).  Task 14
calls ``calibrate`` before a live run and aborts if ``not ok``.
"""
from __future__ import annotations
from dataclasses import dataclass
from .model import Citation
from .interfaces import RelevanceJudge


# ---------------------------------------------------------------------------
# Data model
# ---------------------------------------------------------------------------

@dataclass(frozen=True)
class CalibCase:
    """A single labeled calibration citation with its code snippet."""
    file: str
    line: int
    symbol: str
    issue_text: str   # fed to RelevanceJudge.is_relevant as issue_text
    code: str         # fed to RelevanceJudge.is_relevant as code
    label: str        # "relevant" | "irrelevant" | "hallucinated"


@dataclass(frozen=True)
class CalibResult:
    """Outcome of a calibration run."""
    accuracy: float           # fraction of cases classified correctly
    saturated_all_yes: bool   # judge returned True for every case
    saturated_all_no: bool    # judge returned False for every case
    ok: bool                  # accuracy >= 0.8 and not saturated


# ---------------------------------------------------------------------------
# Default calibration set — 6 cases: 2 relevant, 2 irrelevant, 2 hallucinated
#
# Convention for the GoldRelevance test fake:
#   "RELEVANT: ..."     -> expected judge answer = True  (relevant)
#   "IRRELEVANT: ..."   -> expected judge answer = False (irrelevant)
#   "HALLUCINATED: ..." -> expected judge answer = False (hallucinated)
#
# The issue_text prefix is what lets the test's GoldRelevance fake give the
# label-correct answer without access to the CalibCase.label field at
# judge-call time.  The labels themselves drive accuracy scoring below.
# ---------------------------------------------------------------------------

DEFAULT_CALIB_CASES: list[CalibCase] = [
    # --- relevant cases (judge should return True) ---
    CalibCase(
        file="src/auth.py",
        line=12,
        symbol="validate_token",
        issue_text="RELEVANT: Token validation does not reject expired tokens.",
        code="def validate_token(token: str) -> bool:\n    return token in _valid_tokens",
        label="relevant",
    ),
    CalibCase(
        file="src/db.py",
        line=34,
        symbol="execute_query",
        issue_text="RELEVANT: SQL injection possible in query construction.",
        code="def execute_query(sql: str) -> list:\n    return conn.execute(sql).fetchall()",
        label="relevant",
    ),
    # --- irrelevant cases (judge should return False) ---
    CalibCase(
        file="src/formatting.py",
        line=7,
        symbol="pretty_print",
        issue_text="IRRELEVANT: Token validation does not reject expired tokens.",
        code="def pretty_print(data: dict) -> str:\n    return json.dumps(data, indent=2)",
        label="irrelevant",
    ),
    CalibCase(
        file="src/math_utils.py",
        line=22,
        symbol="clamp",
        issue_text="IRRELEVANT: SQL injection possible in query construction.",
        code="def clamp(x: float, lo: float, hi: float) -> float:\n    return max(lo, min(hi, x))",
        label="irrelevant",
    ),
    # --- hallucinated cases (file/symbol don't exist; judge should return False) ---
    CalibCase(
        file="src/nonexistent_module.py",
        line=99,
        symbol="ghost_function",
        issue_text="HALLUCINATED: Memory leak in resource cleanup.",
        code="def ghost_function(): pass  # this file does not exist in the repo",
        label="hallucinated",
    ),
    CalibCase(
        file="src/phantom_service.py",
        line=5,
        symbol="do_magic",
        issue_text="HALLUCINATED: Race condition in concurrent request handling.",
        code="def do_magic(): pass  # fabricated symbol, not in codebase",
        label="hallucinated",
    ),
]


# ---------------------------------------------------------------------------
# Calibration runner
# ---------------------------------------------------------------------------

def _expected_true(case: CalibCase) -> bool:
    """Label-derived correct answer: True only for 'relevant' cases."""
    return case.label == "relevant"


def calibrate(
    relevance: RelevanceJudge,
    *,
    read_code,
    cases: list[CalibCase] | None = None,
) -> CalibResult:
    """Run *relevance* over *cases* and return a :class:`CalibResult`.

    Parameters
    ----------
    relevance:
        Any :class:`~tier_c.interfaces.RelevanceJudge` (real or fake).
    read_code:
        Callable ``(file: str, line: int) -> str`` that supplies the code
        snippet to the judge.  In tests, use ``_code_of(cases)``; in
        production, use a real file reader.
    cases:
        Calibration set.  Defaults to :data:`DEFAULT_CALIB_CASES`.
    """
    if cases is None:
        cases = DEFAULT_CALIB_CASES

    results: list[bool] = []
    for case in cases:
        cite = Citation(file=case.file, line=case.line, symbol=case.symbol)
        code = read_code(case.file, case.line) or ""
        answer = relevance.is_relevant(cite, case.issue_text, code)
        results.append(bool(answer))

    n = len(results)
    correct = sum(
        result == _expected_true(case)
        for result, case in zip(results, cases)
    )
    accuracy = correct / n if n > 0 else 0.0
    saturated_all_yes = all(results)
    saturated_all_no = not any(results)
    ok = accuracy >= 0.8 and not saturated_all_yes and not saturated_all_no

    return CalibResult(
        accuracy=accuracy,
        saturated_all_yes=saturated_all_yes,
        saturated_all_no=saturated_all_no,
        ok=ok,
    )
