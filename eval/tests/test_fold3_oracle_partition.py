import itertools

from tests.helpers import DECLARED_ADJ, base_ids, fake_run, quick_probes
from tier_a.compare_report import join_probes, partition_by_oracle, compare_runs
from tier_a.policy import ORACLE_ANSWER_STATUSES, PROBE_STATUS_VOCABULARY


CANDIDATE_DIGEST = "sha256:" + "b" * 64


def test_every_runner_status_pair_has_the_required_oracle_partition():
    identities = base_ids()
    candidate_identities = {
        **identities,
        "sut_sha_full": "b" * 40,
        "sut_digest": CANDIDATE_DIGEST,
    }
    pid = "callers:src/U-free.rs:1"
    probe = quick_probes()[pid]

    for base_status, candidate_status in itertools.product(
        PROBE_STATUS_VOCABULARY, repeat=2
    ):
        base = fake_run(identities, probes={pid: probe}, status={pid: base_status})
        candidate = fake_run(
            candidate_identities,
            probes={pid: probe},
            status={pid: candidate_status},
        )
        comparable, changed = partition_by_oracle(join_probes(base, candidate))
        expected_comparable = (
            base_status in ORACLE_ANSWER_STATUSES
            and candidate_status in ORACLE_ANSWER_STATUSES
        )
        assert (pid in comparable) is expected_comparable
        assert (pid in changed) is (not expected_comparable)


def test_paired_oracle_failures_count_against_threshold_and_floor():
    identities = base_ids()
    candidate_identities = {
        **identities,
        "sut_sha_full": "b" * 40,
        "sut_digest": CANDIDATE_DIGEST,
    }
    probes = quick_probes()
    failed = [
        "callers:src/U-free.rs:1",
        "callers:src/U-free.rs:2",
        "callers:src/U-free.rs:3",
        "callees:src/U-free.rs:1",
    ]
    for pid in failed:
        probes[pid] = {**probes[pid], "outcome": "oracle_error"}
    base = fake_run(identities, probes=probes)
    candidate = fake_run(candidate_identities, probes=probes)

    report = compare_runs(base, candidate, DECLARED_ADJ)

    assert report["oracle_changed"]["ids"] == sorted(failed)
    assert report["verdict"] == (
        "NOT_COMPARABLE(oracle_changed: 4/30 > 0.10; "
        "stratum U-free: 2 comparable < 4)"
    )
