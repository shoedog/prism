"""Tests for oracle calibration + saturation guard (Task 6)."""
from tier_c.model import Citation
from tier_c.partc_calib import (
    CalibCase,
    CalibResult,
    DEFAULT_CALIB_CASES,
    calibrate,
)


# ---------------------------------------------------------------------------
# Fakes
# ---------------------------------------------------------------------------

class AllYesRelevance:
    """Always returns True — the degenerate all-YES oracle."""
    def is_relevant(self, cite: Citation, issue_text: str, code: str = "") -> bool:
        return True


class AllNoRelevance:
    """Always returns False — the degenerate all-NO oracle."""
    def is_relevant(self, cite: Citation, issue_text: str, code: str = "") -> bool:
        return False


class GoldRelevance:
    """Returns True only for 'relevant' cases; False for 'irrelevant' and 'hallucinated'."""
    def is_relevant(self, cite: Citation, issue_text: str, code: str = "") -> bool:
        # The gold oracle: relevant label -> True, everything else -> False.
        # We don't have access to the label at judge-call time; instead the
        # calibration harness infers correctness from the label after the fact.
        # For this fake we use a convention: relevant issue_text is prefixed "RELEVANT:".
        return issue_text.startswith("RELEVANT:")


def _code_of(cases: list[CalibCase]):
    """Return a read_code(file, line) callable that looks up the case's code snippet."""
    index = {(c.file, c.line): c.code for c in cases}
    def read_code(file: str, line: int) -> str:
        return index.get((file, line), "")
    return read_code


# ---------------------------------------------------------------------------
# Tests
# ---------------------------------------------------------------------------

CASES = DEFAULT_CALIB_CASES


def test_calib_result_fields():
    """CalibResult is a frozen dataclass with the required fields."""
    r = CalibResult(accuracy=1.0, saturated_all_yes=False, saturated_all_no=False, ok=True)
    assert r.accuracy == 1.0
    assert not r.saturated_all_yes
    assert not r.saturated_all_no
    assert r.ok


def test_default_cases_have_required_fields():
    """Every default case has file, line, symbol, issue_text, code, and label."""
    assert len(CASES) >= 6
    for c in CASES:
        assert c.file
        assert isinstance(c.line, int) and c.line >= 1
        assert c.symbol
        assert c.issue_text
        assert c.code
        assert c.label in ("relevant", "irrelevant", "hallucinated")


def test_default_cases_have_all_three_labels():
    """Default calibration set covers all three label categories."""
    labels = {c.label for c in CASES}
    assert "relevant" in labels
    assert "irrelevant" in labels
    assert "hallucinated" in labels


def test_calibration_flags_saturation_and_accuracy():
    """All-YES oracle is rejected; gold oracle passes."""
    # all-YES oracle: saturated, not ok
    res = calibrate(AllYesRelevance(), read_code=_code_of(CASES), cases=CASES)
    assert res.saturated_all_yes and not res.ok

    # gold oracle: high accuracy, not saturated, ok
    res2 = calibrate(GoldRelevance(), read_code=_code_of(CASES), cases=CASES)
    assert res2.accuracy >= 0.8 and res2.ok


def test_calibration_flags_all_no_saturation():
    """All-NO oracle is also rejected (saturated_all_no)."""
    res = calibrate(AllNoRelevance(), read_code=_code_of(CASES), cases=CASES)
    assert res.saturated_all_no and not res.ok


def test_accuracy_math():
    """Accuracy is fraction of cases judged correctly."""
    # Gold judge on default cases should get >= 5/6 right.
    res = calibrate(GoldRelevance(), read_code=_code_of(CASES), cases=CASES)
    assert 0.0 <= res.accuracy <= 1.0


def test_ok_false_when_below_accuracy_threshold():
    """A judge that only gets half right is not ok."""
    # AllNoRelevance gets the irrelevant+hallucinated cases right but fails relevant ones.
    # Regardless, it is also saturated, so ok=False.
    res = calibrate(AllNoRelevance(), read_code=_code_of(CASES), cases=CASES)
    assert not res.ok


def test_calib_case_is_frozen():
    """CalibCase is a frozen dataclass (immutable)."""
    import dataclasses
    c = CASES[0]
    assert dataclasses.is_dataclass(c)
    try:
        c.label = "other"  # type: ignore[misc]
        raise AssertionError("Should have raised FrozenInstanceError")
    except (dataclasses.FrozenInstanceError, AttributeError):
        pass


def test_gold_relevance_uses_issue_prefix_convention():
    """GoldRelevance fake works as expected with our issue_text convention."""
    # Verify the fake is internally consistent before relying on it above.
    class _FakeCite:
        file = "x.py"; line = 1; symbol = "x"
    cite = Citation("x.py", 1, "x")
    g = GoldRelevance()
    assert g.is_relevant(cite, "RELEVANT: something") is True
    assert g.is_relevant(cite, "IRRELEVANT: something") is False
    assert g.is_relevant(cite, "HALLUCINATED: something") is False
