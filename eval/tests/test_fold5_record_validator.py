import copy
import json

import pytest

from tests.helpers import DECLARED_ADJ, anchored_repo, base_ids, fake_run
from tier_a.bootstrap import publish_anchor_v1, stage_candidate
from tier_a.closure import raw_digest
from tier_a.compare_report import compare_runs
from tier_a.lock import integrity_check, lock_identities


BASE_PATH = "docs/eval/tier-a/2026-10-30-prism-mf4base.json"
CAND_PATH = "docs/eval/tier-a/2026-10-30-prism-mf4cand.json"


def _access_recording(value, path, accessed):
    if isinstance(value, dict):
        class RecordingDict(dict):
            def __getitem__(self, key):
                accessed.add((*path, key))
                return super().__getitem__(key)

            def get(self, key, default=None):
                return self[key] if key in self else default

            def items(self):
                for key in super().keys():
                    yield key, self[key]

            def values(self):
                for key in super().keys():
                    yield self[key]

        return RecordingDict(
            {
                key: _access_recording(item, (*path, key), accessed)
                for key, item in value.items()
            }
        )
    if isinstance(value, list):
        class RecordingList(list):
            def __getitem__(self, index):
                accessed.add((*path, index))
                return super().__getitem__(index)

            def __iter__(self):
                for index in range(len(self)):
                    yield self[index]

        return RecordingList(
            _access_recording(item, (*path, index), accessed)
            for index, item in enumerate(value)
        )
    return value


def _committed_records():
    root = __import__("pathlib").Path(__file__).resolve().parents[2]
    return (
        json.loads((root / BASE_PATH).read_text()),
        json.loads((root / CAND_PATH).read_text()),
    )


def test_single_validator_rejects_missing_matrix_outcome_before_compare():
    from tier_a.record_schema import validate_run_record

    base, candidate = _committed_records()
    candidate["matrix"][0].pop("outcome")

    assert any(
        reason.startswith("matrix.0.outcome")
        for reason in validate_run_record(candidate, purpose="compare")
    )
    assert compare_runs(base, candidate, DECLARED_ADJ)["verdict"].startswith(
        "NOT_COMPARABLE(record_invalid:"
    )


def test_single_validator_enforces_probe_status_authority_and_sample_counts():
    from tier_a.record_schema import validate_run_record

    base, _candidate = _committed_records()
    pid = next(iter(base["probe_status"]))
    contradictory = copy.deepcopy(base)
    contradictory["probes"][pid]["outcome"] = "oracle_error"
    assert any(
        "outcome_status_mismatch" in reason
        for reason in validate_run_record(contradictory, purpose="compare")
    )

    truncated = copy.deepcopy(base)
    removed = next(iter(truncated["probe_status"]))
    del truncated["probe_status"][removed]
    del truncated["probes"][removed]
    assert any(
        "count_mismatch" in reason
        for reason in validate_run_record(truncated, purpose="compare")
    )


@pytest.mark.parametrize("mutation", ("launch", "payloads", "admitted"))
def test_lock_integrity_validates_stored_anchor_record_bytes(tmp_path, mutation):
    lock, eval_dir, repo = anchored_repo(tmp_path)
    report = repo / lock.prism["baseline_report"]["path"]
    record = json.loads(report.read_text())
    if mutation == "launch":
        record["meta"].pop("launch")
    elif mutation == "payloads":
        record.pop("probes")
    else:
        record["meta"]["admitted"] = False
    report.write_text(json.dumps(record, indent=1, sort_keys=True) + "\n")
    lock.prism["baseline_report"]["digest"] = raw_digest(report)

    reasons = integrity_check(lock, eval_dir)

    assert any("baseline_report_record_invalid" in reason for reason in reasons)


@pytest.mark.parametrize("mutation", ("launch", "payloads", "admitted"))
def test_publication_uses_the_same_validator_and_writes_nothing(tmp_path, mutation):
    _lock, eval_dir, repo = anchored_repo(tmp_path, publish=False)
    candidate = stage_candidate(
        repo,
        eval_dir,
        repo / "prism",
        {"cargo": {"features": "all"}},
        tmp_path / "root",
        str(repo),
    )
    record = fake_run(lock_identities(candidate.lock))
    if mutation == "launch":
        record["meta"].pop("launch")
    elif mutation == "payloads":
        record.pop("probes")
    else:
        record["meta"]["admitted"] = False

    with pytest.raises(ValueError, match="not publishable: record_invalid"):
        publish_anchor_v1(
            candidate,
            record,
            eval_dir,
            repo / "docs/eval/tier-a",
            tmp_path / "root/sut",
        )
    assert not (eval_dir / "tier-a.lock.toml").exists()


def test_validator_requires_pinned_matrix_and_complete_nested_shapes():
    from tier_a.record_schema import validate_run_record

    run = fake_run(base_ids())
    for section in ("pinned", "matrix"):
        changed = copy.deepcopy(run)
        changed.pop(section)
        assert validate_run_record(changed, purpose="admission")

    changed = copy.deepcopy(run)
    changed["site_fingerprints"] = {"src/a.rs:3": 7}
    assert validate_run_record(changed, purpose="admission")


def test_access_derived_schema_covers_compare_scorer_and_admission_reads():
    from tier_a.policy import _compute_m2_and_pending
    from tier_a.record_schema import schema_covers_path, validate_run_record

    base_plain, candidate_plain = _committed_records()
    accessed = set()
    base = _access_recording(base_plain, (), accessed)
    candidate = _access_recording(candidate_plain, (), accessed)

    compare_runs(base, candidate, DECLARED_ADJ)
    _compute_m2_and_pending(base["probes"], [], base["site_fingerprints"])
    validate_run_record(base, purpose="admission")

    uncovered = sorted(
        path for path in accessed if not schema_covers_path(base_plain, path)
    )
    assert uncovered == []
