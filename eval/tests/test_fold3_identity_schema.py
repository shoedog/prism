import copy

import pytest

from tests.helpers import DECLARED_ADJ, base_ids, fake_run
from tier_a.compare_report import compare_runs, required_identity_leaf_paths


CANDIDATE_DIGEST = "sha256:" + "b" * 64


def _delete_path(mapping, path):
    current = mapping
    for part in path[:-1]:
        current = current[part]
    del current[path[-1]]


def _null_path(mapping, path):
    current = mapping
    for part in path[:-1]:
        current = current[part]
    current[path[-1]] = None


def _lock_ids():
    identities = base_ids()
    identities["sample_digest"] = {
        "full": "sha256:" + "1" * 64,
        "quick": "sha256:" + "2" * 64,
    }
    identities["runtime"] = {
        "python": "3.12.13",
        "python_version_file": "sha256:" + "3" * 64,
        "uv_lock": "sha256:" + "4" * 64,
        "rustc": "rustc 1.94.0",
        "cargo": "cargo 1.94.0",
        "rust_toolchain": "absent",
        "cargo_config": "absent",
    }
    identities["data"] = {
        "adjudications": {
            "path": "eval/adjudications.jsonl",
            "digest": DECLARED_ADJ[1],
            "records": 2,
        },
        "fixtures": "sha256:" + "5" * 64,
        "pinned_probes": "sha256:" + "6" * 64,
    }
    return identities


@pytest.mark.parametrize("identities", [base_ids(), _lock_ids()], ids=("legacy", "lock-v1"))
def test_every_required_identity_schema_leaf_refuses_when_deleted_or_null(identities):
    candidate_identities = {
        **copy.deepcopy(identities),
        "sut_sha_full": "b" * 40,
        "sut_digest": CANDIDATE_DIGEST,
    }
    leaf_paths = required_identity_leaf_paths(identities)
    assert leaf_paths

    for mutation in (_delete_path, _null_path):
        for path in leaf_paths:
            base_meta = copy.deepcopy(identities)
            mutation(base_meta, path)
            report = compare_runs(
                fake_run(base_meta),
                fake_run(candidate_identities),
                DECLARED_ADJ,
            )
            rendered_path = ".".join(path)
            assert report["verdict"] == (
                f"NOT_COMPARABLE(identity_invalid: base.{rendered_path})"
            )
