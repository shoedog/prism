import re

import pytest

from tier_a import cli
from tier_a.lock import (
    IDENTITY_KEYS,
    Lock,
    load_lock,
    lock_identities,
    report_bindable,
    resolve_prefix,
    save_lock,
)
from tier_a.strata import STRATA
from tests.helpers import ADJ, base_ids, fake_run


def _spec_lock(ids: dict) -> Lock:
    return Lock(
        prism={
            "sha_aliases": ids["sha_aliases"],
            "closure": {"digest": ids["closure_digest"]},
            "sut": {"sha": ids["sut_sha_full"], "digest": ids["sut_digest"]},
            "oracle": {"version": ids["oracle"], "init": ids["oracle_init"]},
            "policy": {"digest": ids["policy_digest"]},
            "population": {"digest": ids["population_digest"]},
            "snapshot": {"digest": ids["snapshot_digest"]},
            "sample": {"digest": ids["sample_digest"], "seed": ids["seed"]},
            "runtime": ids["runtime"],
            "data": ids["data"],
            "env": {"allow": ids["env_allow"], "values": ids["env_values"]},
            "harness_sha": ids["harness_sha"],
        }
    )


def test_lock_roundtrip_is_lossless(tmp_path):
    lock = Lock(
        prism={
            "closure": {
                "digest": "sha256:ab",
                "manifest": "eval/closure/prism-corpus-x.tsv",
            },
            "sha_aliases": ["a" * 40],
            "oracle": {
                "version": "rust-analyzer 1.94.0",
                "init": {"cargo": {"features": "all"}},
            },
            "env": {
                "allow": ["PATH", "HOME"],
                "forbid_read_outside_allow": True,
            },
        }
    )
    save_lock(lock, tmp_path / "l.toml")
    assert load_lock(tmp_path / "l.toml") == lock


def test_report_without_required_identities_is_not_bindable():
    june = {
        "corpus_sha": "20c8490591a3",
        "oracle": "rust-analyzer 1.89.0",
    }
    assert set(report_bindable(june)) == set(IDENTITY_KEYS) - {"oracle"}
    assert report_bindable({k: "x" for k in IDENTITY_KEYS}) == []


def test_lock_identities_omits_missing_harness_sha():
    lock = _spec_lock(base_ids())
    del lock.prism["harness_sha"]

    assert "harness_sha" not in lock_identities(lock)


def test_lock_identities_projects_every_stored_identity():
    ids = base_ids()
    lock = _spec_lock(ids)

    projected = lock_identities(lock)

    assert set(projected) == set(IDENTITY_KEYS)
    assert projected == ids
    assert projected["harness_sha"] == lock.prism["harness_sha"]
    assert projected["corpus_sha_full"] == lock.prism["sha_aliases"][0]


def test_prefix_resolution_is_explicit_and_unique():
    full = "a" * 40
    other = "a" * 11 + "b" + "c" * 28
    assert resolve_prefix("a" * 12, [full, other]) == full
    with pytest.raises(ValueError, match="ambiguous prefix"):
        resolve_prefix("a" * 11, [full, other])
    with pytest.raises(ValueError, match="unknown prefix"):
        resolve_prefix("f" * 12, [full])


def test_fake_run_fixture_is_accepted_by_the_scorer():
    m2, _pending, _stale = cli._compute_m2_and_pending(
        fake_run(base_ids())["probes"], ADJ, {}
    )
    assert set(m2["callers"]) == set(m2["callees"]) == set(STRATA)
    for direction in ("callers", "callees"):
        for stratum in STRATA:
            assert m2[direction][stratum]["corrected"]["recall"][0] == 1.0


def test_fake_run_preserves_explicitly_empty_probes():
    run = fake_run(base_ids(), probes={})

    assert run["probe_status"] == {}
    assert set(run["probes"]) == {"_corpus", "_strata"}


def test_base_ids_uses_canonical_identity_formats():
    ids = base_ids()
    digest_pattern = re.compile(r"^sha256:[0-9a-f]{64}$")
    sha_pattern = re.compile(r"^[0-9a-f]{40}$")

    digest_values = {
        key: value for key, value in ids.items() if key.endswith("_digest")
    }
    digest_values["runtime.uv_lock"] = ids["runtime"]["uv_lock"]
    for key, value in digest_values.items():
        assert digest_pattern.fullmatch(value), key
    for key, value in ids.items():
        if key.endswith("_sha_full") or key == "harness_sha":
            assert sha_pattern.fullmatch(value), key
    assert all(sha_pattern.fullmatch(alias) for alias in ids["sha_aliases"])
