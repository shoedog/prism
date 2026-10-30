import copy
import itertools

from tests.helpers import DECLARED_ADJ, base_ids, fake_run, quick_probes
from tier_a.compare import match_oracle_sites
from tier_a.compare_report import compare_runs


CANDIDATE_DIGEST = "sha256:" + "b" * 64


def _subsets(values):
    return [set(combo) for size in range(len(values) + 1) for combo in itertools.combinations(values, size)]


def test_lost_match_guard_iff_scorer_matched_identity_is_lost():
    identities = base_ids()
    candidate_identities = {
        **identities,
        "sut_sha_full": "b" * 40,
        "sut_digest": CANDIDATE_DIGEST,
    }
    pid = "callers:src/U-free.rs:1"
    oracle_windows = {("src/a.rs", 10, 10), ("src/a.rs", 10, 12)}
    generated_site_sets = _subsets({("src/a.rs", 10), ("src/a.rs", 11), ("src/a.rs", 12)})

    for base_sites in generated_site_sets:
        for candidate_sites in generated_site_sets:
            probes = quick_probes()
            probes[pid] = {
                **probes[pid],
                "oracle_sites": [list(site) for site in sorted(oracle_windows)],
                "prism_sites": [list(site) + [site[1]] for site in sorted(base_sites)],
            }
            candidate_probes = copy.deepcopy(probes)
            candidate_probes[pid]["prism_sites"] = [
                list(site) + [site[1]] for site in sorted(candidate_sites)
            ]
            base_matched = {
                window[:2] for window in match_oracle_sites(base_sites, oracle_windows).values()
            }
            candidate_matched = {
                window[:2]
                for window in match_oracle_sites(candidate_sites, oracle_windows).values()
            }
            expected_loss = base_matched - candidate_matched

            report = compare_runs(
                fake_run(identities, probes=probes),
                fake_run(candidate_identities, probes=candidate_probes),
                DECLARED_ADJ,
            )

            assert (pid in report["lost_matched_oracle_sites"]) is bool(expected_loss)
            if expected_loss:
                representative = {}
                for window in sorted(oracle_windows):
                    representative.setdefault(window[:2], window)
                assert report["lost_matched_oracle_sites"][pid] == [
                    list(representative[site]) for site in sorted(expected_loss)
                ]


def test_lost_match_guard_uses_collapsed_oracle_site_identity():
    identities = base_ids()
    candidate_identities = {
        **identities,
        "sut_sha_full": "b" * 40,
        "sut_digest": CANDIDATE_DIGEST,
    }
    pid = "callers:src/U-free.rs:1"
    probes = quick_probes()
    probes[pid] = {
        **probes[pid],
        "oracle_sites": [["src/a.rs", 10, 10], ["src/a.rs", 10, 12]],
        "prism_sites": [["src/a.rs", 10, 10]],
    }
    candidate_probes = copy.deepcopy(probes)
    candidate_probes[pid]["prism_sites"] = [["src/a.rs", 11, 11]]

    report = compare_runs(
        fake_run(identities, probes=probes),
        fake_run(candidate_identities, probes=candidate_probes),
        DECLARED_ADJ,
    )

    assert report["lost_matched_oracle_sites"] == {}
    assert report["verdict"] == "NO_REGRESSION"
