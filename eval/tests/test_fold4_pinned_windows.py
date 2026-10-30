from tests.helpers import DECLARED_ADJ, base_ids, fake_run
from tier_a.compare_report import compare_runs
from tier_a.model import CallEdge, FunctionDef, Location
from tier_a.pinned import evaluate_pinned


def _edge(start, end):
    fd = FunctionDef("f", "function", None, Location("src/f.rs", 1, 1), 1)
    return CallEdge(
        "caller",
        fd,
        Location("src/call.rs", start, end),
        "caller",
        Location("src/call.rs", start, end),
    )


def test_pinned_producer_serializes_complete_oracle_windows():
    row = evaluate_pinned(
        {"id": "pin", "expected": "known_fail"},
        [],
        [_edge(10, 12)],
        False,
    )
    assert row["oracle_sites"] == [["src/call.rs", 10, 12]]


def test_pinned_end_line_only_change_is_excluded_from_regression():
    ids = base_ids()
    candidate_ids = {
        **ids,
        "sut_sha_full": "b" * 40,
        "sut_digest": "sha256:" + "b" * 64,
    }
    base = {"id": "pin", "outcome": "ok", "oracle_sites": [["src/a.rs", 10, 12]]}
    candidate = {
        "id": "pin",
        "outcome": "regression",
        "oracle_sites": [["src/a.rs", 10, 13]],
    }
    result = compare_runs(
        fake_run(ids, pinned=[base]),
        fake_run(candidate_ids, pinned=[candidate]),
        DECLARED_ADJ,
    )
    assert result["pinned_oracle_changed"] == ["pin"]
    assert result["criteria"]["pinned_no_regression"] is True


def test_record_validator_refuses_legacy_pinned_sites_without_end_line():
    ids = base_ids()
    base = {"id": "pin", "outcome": "ok", "oracle_sites": ["src/a.rs:10"]}
    candidate = {
        "id": "pin",
        "outcome": "regression",
        "oracle_sites": ["src/a.rs:10"],
    }
    base_run = fake_run(ids)
    candidate_run = fake_run(ids)
    base_run["pinned"] = [base]
    candidate_run["pinned"] = [candidate]
    result = compare_runs(base_run, candidate_run, DECLARED_ADJ)
    assert result["verdict"] == (
        "NOT_COMPARABLE(record_invalid: base.pinned.0.oracle_sites, "
        "candidate.pinned.0.oracle_sites)"
    )
