import copy
import json
from pathlib import Path

import pytest
from tests.helpers import ADJ, ADJ_DIGEST, ADJ_RECORDS, DECLARED_ADJ, FakeOracle, FakeSut, base_ids, edit, fake_run, quick_probes
from tier_a import pinned as pinned_mod
from tier_a import cli
from tier_a.adjudication import Adjudication
from tier_a.model import CallEdge, FunctionDef, Location
from tier_a.oracles import OracleError
from tier_a.sut import SutError
from tier_a.strata import STRATA
from tier_a.metrics import wilson
from tier_a.closure import raw_digest
from tier_a import commands
from tier_a.compare_report import COMPARE_IDENTITIES, COMPARE_POLICY, PINNED_RANK, SUT_ERROR_LEG, classify_probe, compare_runs, join_probes, oracle_answer, partition_by_oracle, recall_nondecreasing, render_compare_md, wilson_regressed


EVAL_DIR = Path(__file__).resolve().parents[1]
CANDIDATE_DIGEST = "sha256:" + "b" * 64


def test_classes_on_the_sut_axis():
    ids = base_ids(); cand = {**ids, "sut_sha_full": "b"*40, "sut_digest": CANDIDATE_DIGEST}
    same = {"outcome": "ok", "prism_sites": [["src/a.rs", 3, 3]], "oracle_sites": [["src/a.rs", 3, 3]]}
    assert classify_probe("callers:src/a.rs:1", "ok", same, "ok", same, ids, cand) == "carried_same"
    assert classify_probe("callers:src/a.rs:1", "ok", same, "ok", {**same, "prism_sites": []}, ids, cand) == "carried_changed/sut_change"
    assert classify_probe("callers:src/a.rs:1", "ok", same, "sut_error", None, ids, cand) == "carried_changed/sut_change"      # R5-W2/Ruling 16: the errored leg has no record — still a prism difference


def test_content_change_and_unexplained_nondeterminism():
    ids = base_ids(); p = {"outcome": "ok", "prism_sites": [], "oracle_sites": [["src/a.rs", 3, 3]]}
    assert classify_probe("x", "ok", p, "ok", {**p, "prism_sites": [["src/a.rs", 3, 3]]}, ids, {**ids, "closure_digest": "sha256:o"}) == "carried_changed/content_change"
    assert classify_probe("x", "ok", p, "ok", {**p, "prism_sites": [["src/a.rs", 3, 3]]}, ids, ids) == "unexplained"     # same SUT digest, same oracle, different prism answer ⇒ nondeterminism


def test_oracle_answer_is_oracle_only_and_canonical():                                              # R5-W2
    p = {"outcome": "ok", "stratum": "U-free", "prism_sites": [["src/a.rs", 3, 3]], "oracle_sites": [["src/a.rs", 3, 3]], "oracle_functions": [["src/a.rs", "a"]]}
    assert oracle_answer("ok", p) == oracle_answer("inventory_miss", {**p, "outcome": "inventory_miss", "prism_sites": []})          # outcome is a SUT fact: an inventory improvement stays comparable
    dup = {**p, "oracle_sites": [["src/a.rs", 3, 3, {"score": 1}], ["src/a.rs", 3, 3]], "oracle_functions": [["src/a.rs", "a"], ["src/a.rs", "a"]]}
    assert oracle_answer("ok", dup) == oracle_answer("ok", p)                                                                          # duplicates and the trailing meta dict (cli.py:399-416) do not count
    assert oracle_answer("ok", {**p, "oracle_sites": [["src/a.rs", 4, 4]]}) != oracle_answer("ok", p)
    assert oracle_answer("oracle_error", None) == ("oracle_error",) and oracle_answer("sut_error", None) == SUT_ERROR_LEG


def test_oracle_changed_is_excluded_never_attributed_to_prism():                                   # owner ruling 2026-09-12
    ids = base_ids(); cand = {**ids, "sut_sha_full": "b"*40, "sut_digest": CANDIDATE_DIGEST}
    p = {"outcome": "ok", "direction": "callers", "stratum": "U-free", "seed_def": "src/a.rs:1", "prism_sites": [], "oracle_sites": [["src/a.rs", 3, 3]], "prism_functions": [], "oracle_functions": []}
    b = fake_run(ids, probes={"callers:x:1": p, "callers:y:1": p}); c = fake_run(cand, probes={"callers:x:1": {**p, "oracle_sites": [["src/a.rs", 4, 4]], "prism_sites": [["src/a.rs", 4, 4]]}, "callers:y:1": p})
    comparable, changed = partition_by_oracle(join_probes(b, c))
    assert set(changed) == {"callers:x:1"} and set(comparable) == {"callers:y:1"}                    # the oracle moved on x ⇒ excluded, whatever prism did there
    cmp = compare_runs(b, c, DECLARED_ADJ)
    assert cmp["oracle_changed"] == {"count": 1, "fraction": 0.5, "ids": ["callers:x:1"]} and "callers:x:1" not in cmp["classes"]
    assert cmp["verdict"] == "NOT_COMPARABLE(oracle_changed: 1/2 > 0.10; stratum U-free: 1 comparable < 4)"      # Ruling 13: every failing reason, fraction first
    probes = {f"callers:p{i}:1": p for i in range(30)}; b30 = fake_run(ids, probes=probes)
    c30 = fake_run(cand, probes={**probes, "callers:p0:1": {**p, "oracle_sites": [["src/a.rs", 9, 9]]}})
    cmp30 = compare_runs(b30, c30, DECLARED_ADJ); assert cmp30["oracle_changed"]["count"] == 1 and cmp30["criteria"]["oracle_changed_within_threshold"] is True and cmp30["verdict"] == "NO_REGRESSION"
    c30b = fake_run(cand, probes={**probes, **{f"callers:p{i}:1": {**p, "oracle_sites": [["src/a.rs", 9, 9]]} for i in range(4)}})
    assert compare_runs(b30, c30b, DECLARED_ADJ)["verdict"] == "NOT_COMPARABLE(oracle_changed: 4/30 > 0.10)"
    err = {k: v for k, v in probes.items() if k != "callers:p0:1"}                                    # R5-W2: an oracle_error leg has NO probe record (cli.py:658-660) but IS in probe_status
    c_err = fake_run(cand, probes=err, status={**{k: "ok" for k in probes}, "callers:p0:1": "oracle_error"})
    cmp_err = compare_runs(b30, c_err, DECLARED_ADJ)
    assert cmp_err["oracle_changed"]["ids"] == ["callers:p0:1"] and cmp_err["criteria"]["probe_sets_equal"] is True and cmp_err["verdict"] == "NO_REGRESSION"   # the oracle failed on one leg ⇒ oracle_changed, never probe_set


def test_every_non_scored_non_sut_status_is_an_oracle_failure():
    ids = base_ids()
    cand = {
        **ids,
        "sut_sha_full": "b" * 40,
        "sut_digest": "sha256:" + "b" * 64,
    }
    pid = "callers:src/U-free.rs:1"
    base_probes = quick_probes()
    base_probes[pid] = {
        **base_probes[pid],
        "oracle_sites": [],
        "prism_sites": [["src/prism-only.rs", 9, 9]],
    }
    base = fake_run(ids, probes=base_probes)
    candidate_probes = quick_probes()
    del candidate_probes[pid]
    candidate = fake_run(
        cand,
        probes=candidate_probes,
        status={**{key: "ok" for key in quick_probes()}, pid: "oracle_timeout"},
    )

    cmp = compare_runs(base, candidate, DECLARED_ADJ)

    assert cmp["oracle_changed"]["ids"] == [pid]
    assert pid not in cmp["classes"]
    assert cmp["counts"]["carried_changed/sut_change"] == 0


def test_comparison_floor_is_per_stratum_over_both_directions_and_passable_under_quick():     # R5-W3, R6-W6
    ids = base_ids(); cand = {**ids, "sut_sha_full": "b"*40, "sut_digest": CANDIDATE_DIGEST}
    b = fake_run(ids)                                                                              # the default quick_probes() sample: 5 strata × 2 directions × 3 = 30 (cli.py:627) plus the "_corpus"/"_strata" entries the scorer reads
    assert {p["stratum"] for k, p in b["probes"].items() if not k.startswith("_")} == set(STRATA) and len(b["probe_status"]) == 30 and set(b["probes"]["_strata"]) == set(STRATA)
    moved = lambda k: {k: {**quick_probes()[k], "oracle_sites": [["src/a.rs", 9, 9]]}}
    u = lambda i: f"callers:src/U-free.rs:{i}"
    c2 = fake_run(cand, probes={**quick_probes(), **moved(u(1)), **moved(u(2))})                  # U-free keeps 1 + 3 = 4 comparable ⇒ passes (a per-(direction, stratum) floor of 4 could never pass --quick)
    cmp2 = compare_runs(b, c2, DECLARED_ADJ); assert cmp2["oracle_changed"]["count"] == 2 and cmp2["verdict"] == "NO_REGRESSION"
    c3 = fake_run(cand, probes={**quick_probes(), **moved(u(1)), **moved(u(2)), **moved(u(3))})   # 0 + 3 = 3 comparable; 3/30 = 0.10 is NOT > 0.10
    cmp3 = compare_runs(b, c3, DECLARED_ADJ); assert cmp3["verdict"] == "NOT_COMPARABLE(stratum U-free: 3 comparable < 4)"
    relaxed = compare_runs(b, c3, DECLARED_ADJ, policy={**COMPARE_POLICY, "min_comparable_per_stratum": 3})               # R5-W4: the thresholds are an explicit argument, echoed in the report
    assert relaxed["verdict"] == "NO_REGRESSION" and relaxed["policy"]["min_comparable_per_stratum"] == 3 and cmp3["policy"] == COMPARE_POLICY


def test_candidate_sut_error_on_a_comparable_probe_is_a_regression():                            # Ruling 16, R6-W5
    ids = base_ids(); cand = {**ids, "sut_sha_full": "b"*40, "sut_digest": CANDIDATE_DIGEST}
    pid = "callees:src/C-name.rs:2"
    b = fake_run(ids); probes = quick_probes(); del probes[pid]
    c = fake_run(cand, probes=probes, status={**{k: "ok" for k in quick_probes()}, pid: "sut_error"})
    assert partition_by_oracle(join_probes(b, c))[1] == {}                                          # a sut_error leg is comparable against any oracle answer
    cmp = compare_runs(b, c, DECLARED_ADJ)
    assert cmp["classes"][pid] == "carried_changed/sut_change" and cmp["criteria"]["no_new_sut_errors"] is False and cmp["verdict"] == "REGRESSION(no_new_sut_errors)" and cmp["status_transitions"] == {pid: ["ok", "sut_error"]}
    recovered = fake_run(ids, probes={**quick_probes(), pid: {**quick_probes()[pid], "prism_sites": []}})   # R6-W5: the base errored, the candidate answers — with an EMPTY prism answer (recall 0 on that probe alone)
    cmp_r = compare_runs(c, recovered, DECLARED_ADJ)
    assert cmp_r["verdict"] == "NO_REGRESSION" and cmp_r["criteria"]["no_recall_loss"] is True and cmp_r["status_transitions"] == {pid: ["sut_error", "ok"]}   # m2 over the common scored intersection: the recovered probe sits outside BOTH denominators


def test_adjudications_must_be_the_runs_declared_copy(tmp_path, monkeypatch):                      # R5-W5, R5-W7
    ids = base_ids(); cand = {**ids, "sut_sha_full": "b"*40, "sut_digest": CANDIDATE_DIGEST}
    b = fake_run(ids); c = fake_run(cand)
    assert compare_runs(b, c, (ADJ, "sha256:" + "0"*64))["verdict"] == "NOT_COMPARABLE(adjudications_digest)"   # records whose digest is not the legs' meta.data digest are refused
    adj = tmp_path / "adjudications.jsonl"; adj.write_text("".join(json.dumps(r) + "\n" for r in ADJ_RECORDS)); assert raw_digest(adj) == ADJ_DIGEST
    bp, cp = tmp_path / "base.json", tmp_path / "nested/cand.json"; cp.parent.mkdir(); bp.write_text(json.dumps(b)); cp.write_text(json.dumps(c))
    monkeypatch.chdir(tmp_path)
    cmp = commands.compare(bp, Path("nested/cand.json"), tmp_path / "compare", adjudications=adj, repo_root=tmp_path)
    assert cmp["verdict"] == "NO_REGRESSION" and cmp["runs"] == {
        "base": {"path_base": "repo_root", "path": "base.json", "digest": raw_digest(bp)},
        "candidate": {"path_base": "repo_root", "path": "nested/cand.json", "digest": raw_digest(cp)},
    } and len(cmp["meta"]["harness_sha"]) == 40
    for leg in cmp["runs"].values():
        resolved = tmp_path / leg["path"]
        assert resolved.is_file() and raw_digest(resolved) == leg["digest"]
    assert (tmp_path / "compare.json").exists() and (tmp_path / "compare.md").exists()
    edit(adj, '"verdict": "oracle_miss"', '"verdict": "prism_fp"')                                  # a LEGAL mutation after the runs (adjudication.py:10-18)
    cmp2 = commands.compare(bp, cp, tmp_path / "compare2", adjudications=adj, repo_root=tmp_path)
    assert cmp2["verdict"] == "NOT_COMPARABLE(adjudications_digest)" and not (tmp_path / "compare2.json").exists() and not (tmp_path / "compare2.md").exists()   # R6-S1: a mutated store never silently rescores a historical pair; neither output is written


def test_committed_compare_paths_resolve_from_repo_root_with_recorded_digests():
    repo_root = EVAL_DIR.parent
    report = json.loads(
        (repo_root / "docs/eval/tier-a/compare-2026-10-30-prism.json").read_text()
    )

    for leg in report["runs"].values():
        assert leg["path_base"] == "repo_root"
        resolved = repo_root / leg["path"]
        assert resolved.is_file()
        assert raw_digest(resolved) == leg["digest"]


def test_compare_refuses_input_resolving_outside_repo_root(tmp_path):
    repo_root = tmp_path / "repo"
    repo_root.mkdir()
    outside = tmp_path / "outside.json"
    outside.write_text("{}")

    with pytest.raises(ValueError):
        commands._repo_run(outside, repo_root)


def test_adjudications_digest_accepts_lock_shape_and_legacy_but_refuses_drift():
    ids = base_ids()
    cand = {**ids, "sut_sha_full": "b" * 40, "sut_digest": CANDIDATE_DIGEST}
    base = fake_run(ids)
    candidate = fake_run(cand)
    lock_data = {
        "adjudications": {
            "path": "eval/adjudications.jsonl",
            "digest": ADJ_DIGEST,
            "records": len(ADJ_RECORDS),
        },
        "fixtures": "sha256:fixtures",
        "pinned_probes": "sha256:pinned",
    }
    base["meta"]["data"] = copy.deepcopy(lock_data)
    candidate["meta"]["data"] = copy.deepcopy(lock_data)

    cmp = compare_runs(base, candidate, DECLARED_ADJ)
    assert cmp["verdict"] == "NO_REGRESSION"
    assert cmp["criteria"]["adjudications_digest_keys"] == {
        "base": "data.adjudications.digest",
        "candidate": "data.adjudications.digest",
    }

    candidate["meta"]["data"]["adjudications"]["digest"] = "sha256:different"
    assert (
        compare_runs(base, candidate, DECLARED_ADJ)["verdict"]
        == "NOT_COMPARABLE(adjudications_digest)"
    )

    legacy = fake_run(cand)
    cmp_legacy = compare_runs(fake_run(ids), legacy, DECLARED_ADJ)
    assert cmp_legacy["verdict"] == "NO_REGRESSION"
    assert cmp_legacy["criteria"]["adjudications_digest_keys"] == {
        "base": "data.eval/adjudications.jsonl.digest",
        "candidate": "data.eval/adjudications.jsonl.digest",
    }


def test_adjudications_digest_refuses_missing_wrong_path_and_ambiguous_shapes():
    ids = base_ids()
    base = fake_run(ids)
    cases = (
        ({}, "NOT_COMPARABLE(identity_invalid: base.data, candidate.data)"),
        (
            {"adjudications": {"path": "other.jsonl", "digest": ADJ_DIGEST}},
            "NOT_COMPARABLE(adjudications_digest)",
        ),
        ({
            "adjudications": {
                "path": "eval/adjudications.jsonl",
                "digest": ADJ_DIGEST,
            },
            "eval/adjudications.jsonl": {"digest": ADJ_DIGEST},
        }, "NOT_COMPARABLE(adjudications_digest)"),
    )
    for data, expected in cases:
        base["meta"]["data"] = copy.deepcopy(data)
        candidate = fake_run(ids)
        candidate["meta"]["data"] = copy.deepcopy(data)
        assert compare_runs(base, candidate, DECLARED_ADJ)["verdict"] == expected


def test_compare_uses_observed_sut_digest_for_changed_answers():
    ids = base_ids()
    pid = "callers:src/U-free.rs:1"
    changed = quick_probes()
    changed[pid] = {**changed[pid], "prism_sites": [["src/changed.rs", 9, 9]]}

    different_sut = {
        **ids,
        "sut_sha_full": "b" * 40,
        "sut_digest": "sha256:" + "d" * 64,
    }
    cmp = compare_runs(fake_run(ids), fake_run(different_sut, probes=changed), DECLARED_ADJ)
    assert cmp["classes"][pid] == "carried_changed/sut_change"

    same_digest = {**different_sut, "sut_digest": ids["sut_digest"]}
    cmp_same = compare_runs(fake_run(ids), fake_run(same_digest, probes=changed), DECLARED_ADJ)
    assert cmp_same["classes"][pid] == "unexplained"


def test_exact_m6_mutation_losing_unadjudicated_matched_sites_is_a_regression():
    records_dir = EVAL_DIR.parent / "docs/eval/tier-a"
    base = json.loads((records_dir / "2026-10-29-prism-base3.json").read_text())
    candidate = json.loads(
        (records_dir / "2026-10-29-prism-candidate2.json").read_text()
    )
    pid = "callees:src/ast.rs:2914"
    candidate["probes"][pid]["prism_sites"] = []
    adjudications = EVAL_DIR / "adjudications.jsonl"

    cmp = compare_runs(
        base,
        candidate,
        (commands.load_records(adjudications), raw_digest(adjudications)),
    )

    assert cmp["counts"]["carried_changed/sut_change"] == 1
    assert cmp["counts"]["carried_same"] == 29
    assert cmp["criteria"]["no_recall_loss"] is True
    assert cmp["criteria"]["no_lost_matched_oracle_sites"] is False
    assert cmp["lost_matched_oracle_sites"] == {
        pid: [["src/ast.rs", 2923, 2923], ["src/ast.rs", 2929, 2929]]
    }
    assert cmp["verdict"] == "REGRESSION(no_lost_matched_oracle_sites)"


def test_lost_match_guard_uses_the_scorers_tolerated_one_to_one_matches():
    ids = base_ids()
    candidate_ids = {**ids, "sut_sha_full": "b" * 40, "sut_digest": CANDIDATE_DIGEST}
    pid = "callers:src/U-method.rs:1"
    base_probes = quick_probes()
    base_probes[pid] = {
        **base_probes[pid],
        "prism_sites": [["src/tolerated.rs", 10, 10]],
        "oracle_sites": [["src/tolerated.rs", 11, 11]],
    }
    candidate_probes = copy.deepcopy(base_probes)
    candidate_probes[pid]["prism_sites"] = []

    cmp = compare_runs(
        fake_run(ids, probes=base_probes),
        fake_run(candidate_ids, probes=candidate_probes),
        DECLARED_ADJ,
    )

    assert cmp["lost_matched_oracle_sites"] == {
        pid: [["src/tolerated.rs", 11, 11]]
    }
    assert cmp["verdict"] == "REGRESSION(no_lost_matched_oracle_sites)"


def test_equivalent_coordinate_movement_does_not_create_a_lost_match():
    ids = base_ids()
    candidate_ids = {**ids, "sut_sha_full": "b" * 40, "sut_digest": CANDIDATE_DIGEST}
    pid = "callers:src/U-method.rs:1"
    base_probes = quick_probes()
    base_probes[pid] = {
        **base_probes[pid],
        "prism_sites": [["src/tolerated.rs", 11, 11]],
        "oracle_sites": [["src/tolerated.rs", 11, 11]],
    }
    candidate_probes = copy.deepcopy(base_probes)
    candidate_probes[pid]["prism_sites"] = [["src/tolerated.rs", 10, 10]]

    cmp = compare_runs(
        fake_run(ids, probes=base_probes),
        fake_run(candidate_ids, probes=candidate_probes),
        DECLARED_ADJ,
    )

    assert cmp["lost_matched_oracle_sites"] == {}
    assert cmp["verdict"] == "NO_REGRESSION"


def test_pending_change_without_lost_match_remains_no_regression():
    ids = base_ids()
    candidate_ids = {**ids, "sut_sha_full": "b" * 40, "sut_digest": CANDIDATE_DIGEST}
    pid = "callers:src/U-method.rs:1"
    base_probes = quick_probes()
    base_probes[pid] = {
        **base_probes[pid],
        "prism_sites": [["src/a.rs", 3, 3], ["src/pending.rs", 50, 50]],
    }
    candidate_probes = copy.deepcopy(base_probes)
    candidate_probes[pid]["prism_sites"] = [["src/a.rs", 3, 3]]

    cmp = compare_runs(
        fake_run(ids, probes=base_probes),
        fake_run(candidate_ids, probes=candidate_probes),
        DECLARED_ADJ,
    )

    assert cmp["m2"]["base"]["callers"]["U-method"]["pending"] == 1
    assert cmp["m2"]["candidate"]["callers"]["U-method"]["pending"] == 0
    assert cmp["criteria"]["no_lost_matched_oracle_sites"] is True
    assert cmp["verdict"] == "NO_REGRESSION"


def test_added_prism_only_site_is_precision_only_not_a_matched_site_loss():
    ids = base_ids()
    candidate_ids = {**ids, "sut_sha_full": "b" * 40, "sut_digest": CANDIDATE_DIGEST}
    pid = "callers:src/U-method.rs:1"
    candidate_probes = quick_probes()
    candidate_probes[pid] = {
        **candidate_probes[pid],
        "prism_sites": [["src/a.rs", 3, 3], ["src/added.rs", 60, 60]],
    }

    cmp = compare_runs(
        fake_run(ids),
        fake_run(candidate_ids, probes=candidate_probes),
        DECLARED_ADJ,
    )

    assert cmp["m2"]["base"]["callers"]["U-method"]["pending"] == 0
    assert cmp["m2"]["candidate"]["callers"]["U-method"]["pending"] == 1
    assert cmp["criteria"]["no_lost_matched_oracle_sites"] is True
    assert cmp["verdict"] == "NO_REGRESSION"


def test_adjudicated_lost_matched_site_keeps_existing_recall_regression():
    ids = base_ids()
    digest = "sha256:adjudicated-matched-site-loss"
    candidate_ids = {**ids, "sut_sha_full": "b" * 40, "sut_digest": CANDIDATE_DIGEST}
    for leg in (ids, candidate_ids):
        leg["data"] = {"eval/adjudications.jsonl": {"digest": digest}}
    pid = "callers:src/U-method.rs:1"
    candidate_probes = quick_probes()
    candidate_probes[pid] = {**candidate_probes[pid], "prism_sites": []}
    adjudication = Adjudication(
        corpus="prism",
        measurement="callers",
        direction="oracle_only",
        seed_def="src/U-method.rs:1",
        site="src/a.rs:3",
        verdict="prism_fn",
        reason="fixture",
        adjudicated_by="test",
        date="2026-10-29",
    )

    cmp = compare_runs(
        fake_run(ids),
        fake_run(candidate_ids, probes=candidate_probes),
        ([adjudication], digest),
    )

    assert cmp["criteria"]["no_recall_loss"] is False
    assert cmp["criteria"]["no_lost_matched_oracle_sites"] is False
    assert cmp["verdict"] == (
        "REGRESSION(no_recall_loss; no_lost_matched_oracle_sites)"
    )


def test_real_m6_lock_bound_pair_is_comparable():
    fixtures = Path(__file__).parent / "fixtures/m6pre2"
    base = json.loads((fixtures / "2026-10-29-prism-base.json").read_text())
    candidate = json.loads(
        (fixtures / "2026-10-29-prism-candidate.json").read_text()
    )
    digest = base["meta"]["data"]["adjudications"]["digest"]
    assert candidate["meta"]["sut_digest"] == base["meta"]["sut_digest"]

    cmp = compare_runs(base, candidate, ([], digest))
    assert cmp["verdict"] == "NO_REGRESSION"
    assert cmp["counts"]["carried_same"] == 30
    assert set(cmp["classes"].values()) == {"carried_same"}


def test_pinned_probes_are_partitioned_by_oracle_answer_and_ranked_by_contract():                 # R5-W6, R6-W3, R6-W4, R7-W1
    # Rows are hand-built but PRODUCER-COHERENT with evaluate_pinned (pinned.py:34-70): an oracle_miss_site pin is "ok" iff the configured miss site is in prism_only (prism reports it, the oracle does not), else "missing"; a known_fail pin is "known_fail" iff P <= 0.2 and R <= 0.2 (prism sites disjoint from the oracle's), else "flip_candidate". compare_report reads only id/outcome/oracle_sites/error_side. (The implementer may build these rows through evaluate_pinned with CallEdge fixtures instead; the transitions below are the ones it can emit.)
    ids = base_ids(); cand = {**ids, "sut_sha_full": "b"*40, "sut_digest": CANDIDATE_DIGEST}
    ok = {"id": "pin-1", "expected": "oracle_miss_site", "oracle_miss_site": "src/mcp/x.rs:9", "outcome": "ok", "prism_sites": ["src/a.rs:3", "src/mcp/x.rs:9"], "oracle_sites": ["src/a.rs:3"], "prism_only": ["src/mcp/x.rs:9"], "oracle_only": [], "oracle_miss_site_found": True}
    missing = {**ok, "outcome": "missing", "prism_sites": ["src/a.rs:3"], "prism_only": [], "oracle_miss_site_found": False}        # same oracle answer; prism no longer reports the miss site
    b = fake_run(ids, pinned=[ok])
    moved = compare_runs(b, fake_run(cand, pinned=[{**missing, "oracle_sites": ["src/a.rs:4"], "oracle_only": ["src/a.rs:4"]}]), DECLARED_ADJ)
    assert moved["pinned_oracle_changed"] == ["pin-1"] and moved["criteria"]["pinned_no_regression"] is True and moved["verdict"] == "NO_REGRESSION"   # rust-analyzer moved ⇒ excluded, listed
    oerr = compare_runs(b, fake_run(cand, pinned=[{"id": "pin-1", "expected": "oracle_miss_site", "outcome": "error", "error_side": "oracle", "error": "timeout"}]), DECLARED_ADJ)
    assert oerr["pinned_oracle_changed"] == ["pin-1"] and oerr["verdict"] == "NO_REGRESSION"           # the oracle failed ⇒ never attributed to prism
    serr = compare_runs(b, fake_run(cand, pinned=[{"id": "pin-1", "expected": "oracle_miss_site", "outcome": "error", "error_side": "sut", "oracle_sites": ["src/a.rs:3"], "error": "SutError"}]), DECLARED_ADJ)
    assert serr["pinned_oracle_changed"] == [] and serr["verdict"] == "REGRESSION(pinned_no_regression)"   # R6-W4: prism raised after a stable oracle answer ⇒ regression
    same = compare_runs(b, fake_run(cand, pinned=[missing]), DECLARED_ADJ)
    assert same["pinned_oracle_changed"] == [] and same["criteria"]["pinned_no_regression"] is False and same["verdict"] == "REGRESSION(pinned_no_regression)"   # ok → missing under the same oracle answer
    kf = {"id": "pin-2", "expected": "known_fail", "outcome": "known_fail", "prism_sites": ["src/b.rs:1"], "oracle_sites": ["src/a.rs:3"], "prism_only": ["src/b.rs:1"], "oracle_only": ["src/a.rs:3"], "known_fail": True}   # P = R = 0
    flip = {**kf, "outcome": "flip_candidate", "prism_sites": ["src/a.rs:3"], "prism_only": [], "oracle_only": [], "known_fail": False}                        # P = R = 1 under the same oracle answer
    assert compare_runs(fake_run(ids, pinned=[kf]), fake_run(cand, pinned=[flip]), DECLARED_ADJ)["criteria"]["pinned_no_regression"] is True    # R6-W3: the known failure now passes — PINNED_RANK 1 → 2
    assert compare_runs(fake_run(ids, pinned=[flip]), fake_run(cand, pinned=[kf]), DECLARED_ADJ)["verdict"] == "REGRESSION(pinned_no_regression)"   # 2 → 1
    assert PINNED_RANK["ok"] == PINNED_RANK["flip_candidate"] > PINNED_RANK["known_fail"] > PINNED_RANK["regression"] == PINNED_RANK["missing"] == PINNED_RANK["error"] == PINNED_RANK["seed_missing"]


def test_run_pinned_records_which_side_failed(monkeypatch):                                        # R6-W4 (pinned.py:110-120 split try)
    probe = {"id": "pin-x", "expected": "oracle_miss_site", "oracle_miss_site": "src/a.rs:3", "file": "src/a.rs", "name": "f"}
    monkeypatch.setattr(pinned_mod, "PINNED", [probe]); monkeypatch.setattr(pinned_mod, "_resolve", lambda snap, p: FunctionDef("f", "function", None, Location("src/a.rs", 1, 1), 1))
    class OracleRaises(FakeOracle):  # noqa: D401
        def callers(self, fd): raise OracleError("no item")
    class SutRaises(FakeSut):
        def callers(self, root, fd): raise SutError("boom")
    rows = pinned_mod.run_pinned(OracleRaises(), FakeSut(), [], "/repo"); assert rows[0]["outcome"] == "error" and rows[0]["error_side"] == "oracle" and "oracle_sites" not in rows[0]
    rows = pinned_mod.run_pinned(FakeOracle(), SutRaises(), [], "/repo"); assert rows[0]["outcome"] == "error" and rows[0]["error_side"] == "sut" and rows[0]["oracle_sites"] == []   # FakeOracle.callers returns [] — the oracle DID answer


def test_missing_probe_and_identity_mismatch_are_not_comparable():                    # W11
    ids = base_ids(); cand = {**ids, "sut_sha_full": "b"*40, "sut_digest": CANDIDATE_DIGEST}
    b = fake_run(ids, probes={"callers:x:1": {"outcome": "ok", "stratum": "U-free", "prism_sites": [], "oracle_sites": []}})
    c = fake_run(cand, probes={})
    assert compare_runs(b, c, DECLARED_ADJ)["verdict"] == "NOT_COMPARABLE(probe_set)"
    different = {
        "sha_aliases": ["c" * 40],
        "harness_sha": "c" * 40,
        "env_values": {"PATH": "/other"},
        "policy_digest": "sha256:" + "c" * 64,
        "snapshot_digest": "sha256:" + "c" * 64,
    }
    for k, value in different.items():        # astra W11; observation_coverage no longer exists (owner ruling)
        assert compare_runs(fake_run(ids), fake_run({**cand, k: value}), DECLARED_ADJ)["verdict"] == f"NOT_COMPARABLE({k})"
    assert compare_runs(fake_run(ids), fake_run({**cand, "admitted": False}), DECLARED_ADJ)["verdict"] == "NOT_COMPARABLE(admitted)"


@pytest.mark.parametrize(
    ("side", "key", "value"),
    [
        ("base", "sut_digest", None),
        ("candidate", "sut_digest", None),
        ("base", "harness_sha", "not-a-sha"),
        ("candidate", "sut_sha_full", "abc"),
        ("candidate", "env_allow", "PATH"),
    ],
)
def test_missing_null_or_malformed_required_identity_refuses(side, key, value):
    ids = base_ids()
    cand = {**ids, "sut_sha_full": "b" * 40, "sut_digest": "sha256:" + "b" * 64}
    base = fake_run(ids)
    candidate = fake_run(cand)
    leg = base if side == "base" else candidate
    if value is None:
        leg["meta"].pop(key)
    else:
        leg["meta"][key] = value

    cmp = compare_runs(base, candidate, DECLARED_ADJ)

    assert cmp["verdict"] == f"NOT_COMPARABLE(identity_invalid: {side}.{key})"


def test_invalid_baseline_leg_refuses_before_regression_classification():
    ids = base_ids()
    cand = {**ids, "sut_sha_full": "b" * 40, "sut_digest": "sha256:" + "b" * 64}
    base = fake_run(ids)
    candidate = fake_run(cand)
    candidate["meta"]["baseline_invalid"] = True
    candidate["meta"]["invalid_reasons"] = ["oracle_timeout", "live_corpus"]

    cmp = compare_runs(base, candidate, DECLARED_ADJ)

    assert cmp["criteria"]["both_baseline_valid"] is False
    assert cmp["verdict"] == (
        "NOT_COMPARABLE(baseline_invalid: candidate: oracle_timeout, live_corpus)"
    )


def test_unexplained_nondeterminism_fails_the_gate():                                      # astra W11, re-scoped by the owner ruling
    ids = base_ids(); p = {"outcome": "ok", "stratum": "U-free", "prism_sites": [], "oracle_sites": [["src/a.rs", 3, 3]]}
    probes = {f"callers:p{i}:1": p for i in range(30)}
    b = fake_run(ids, probes=probes); c = fake_run(ids, probes={**probes, "callers:p0:1": {**p, "prism_sites": [["src/a.rs", 3, 3]]}})   # SAME sut_digest, same oracle, prism differs ⇒ unexplained
    cmp = compare_runs(b, c, DECLARED_ADJ); assert cmp["counts"]["unexplained"] == 1 and cmp["criteria"]["unexplained_zero"] is False and cmp["verdict"] == "REGRESSION(unexplained)"


from tier_a.metrics import wilson
def _m2(p, r):   # report-shaped: m2[direction][stratum] = {raw, corrected, function, pending, shortfall} — exactly cli.py:308-323 (r2 W6)
    pr = lambda v: {"precision": v[0], "recall": v[1], "tp": 9, "fp": 1, "fn": 1}
    return {"callers": {"U-free": {"raw": pr((p, r)), "corrected": pr((p, r)), "function": {"raw": pr((p, r)), "corrected": pr((p, r))}, "pending": 0, "shortfall": 0}}}


def test_wilson_is_descriptive_and_recall_gate_is_nondecreasing():                      # r2 W6 shape; astra W12 gate
    base = _m2([0.9, 0.80, 0.95], [0.9, 0.80, 0.95])
    assert wilson_regressed(base, _m2([0.9, 0.80, 0.95], [0.7, 0.55, 0.79])) == ["callers/U-free/corrected/recall"]   # hi 0.79 < lo 0.80 (flag)
    assert wilson_regressed(base, _m2([0.9, 0.80, 0.95], [0.75, 0.60, 0.85])) == []                                    # overlapping ⇒ no Wilson flag …
    assert recall_nondecreasing(base, _m2([0.9, 0.80, 0.95], [0.75, 0.60, 0.85])) == ["callers/U-free/corrected/recall"]   # … but the GATE fails: 0.75 < 0.9
    w100 = wilson(100, 100); w99 = wilson(99, 100)                                                                       # the astra counterexample, with the repository's own formula
    assert wilson_regressed(_m2(w100, w100), _m2(w100, w99)) == [] and recall_nondecreasing(_m2(w100, w100), _m2(w100, w99)) == ["callers/U-free/corrected/recall"]
    assert recall_nondecreasing(base, _m2([0.9, 0.80, 0.95], [0.9, 0.80, 0.95])) == []                                  # equal ⇒ pass
    assert recall_nondecreasing(base, {"callers": {}}) == ["callers/U-free/corrected/missing"]                          # stratum absent on one side is never silently skipped


def test_report_carries_e5_contract():
    ids = base_ids(); cand = {**ids, "sut_sha_full": "b"*40, "sut_digest": CANDIDATE_DIGEST}
    cmp = compare_runs(fake_run(ids), fake_run(cand), DECLARED_ADJ)
    for k in ("base_sha", "candidate_sha", "base_sut_digest", "candidate_sut_digest", *COMPARE_IDENTITIES): assert k in cmp["identities"]
    assert len(cmp["identities"]["base_sha"]) == 40 and len(cmp["identities"]["candidate_sha"]) == 40                   # full SHAs (astra W5)
    assert cmp["criteria"] == {"both_admitted": True, "both_baseline_valid": True, "identities_equal": True, "probe_sets_equal": True, "adjudications_match": True, "adjudications_digest_keys": {"base": "data.eval/adjudications.jsonl.digest", "candidate": "data.eval/adjudications.jsonl.digest"}, "oracle_answers_joined": True, "oracle_changed_within_threshold": True, "no_new_sut_errors": True, "unexplained_zero": True, "no_recall_loss": True, "no_lost_matched_oracle_sites": True, "matrix_no_regression": True, "pinned_no_regression": True}
    assert cmp["oracle_changed"] == {"count": 0, "fraction": 0.0, "ids": []}
    assert cmp["policy"] == COMPARE_POLICY and cmp["pinned_oracle_changed"] == [] and cmp["runs"] == {"base": None, "candidate": None}   # R5-W4/W6/W7: policy echoed; runs filled by commands.compare
    assert cmp["verdict"] == "NO_REGRESSION" and "wilson_flags" in cmp


def test_compare_subcommand_exit_contract(monkeypatch, tmp_path):
    paths = [tmp_path / name for name in ("base.json", "candidate.json", "out")]
    monkeypatch.setattr(commands, "compare", lambda *args: {"verdict": "NO_REGRESSION"})
    assert commands.dispatch(
        ["compare", "--base", str(paths[0]), "--candidate", str(paths[1]), "--out", str(paths[2])]
    ) == 0
    monkeypatch.setattr(commands, "compare", lambda *args: {"verdict": "REGRESSION(unexplained)"})
    assert commands.dispatch(
        ["compare", "--base", str(paths[0]), "--candidate", str(paths[1]), "--out", str(paths[2])]
    ) == 2


@pytest.mark.parametrize("bad_name,language", [
    ("route_decorator_guard", "python"),
    ("route_handler_entry", "python"),
    ("route_handler_entry", "javascript"),
])
def test_matrix_fixture_identity_preserves_colliding_rows(bad_name, language):
    from tier_a.matrix import _run_callers_case, load_case

    class FixtureSut:
        def __init__(self, bad):
            self.bad = bad

        def callers(self, root, fd):
            case = load_case(Path(root) / "expected.toml")
            sites = set(case.expect_callers)
            if self.bad and Path(root).name == bad_name:
                sites = set() if sites else {("app.py", 99)}
            return [
                CallEdge(
                    "caller", fd, None, None, Location(file, line, line),
                    case.expected_resolution_kind or "framework_entry",
                )
                for file, line in sites
            ]

    names = (
        ["route_decorator_guard", "route_handler_entry", "route_handler_entry_fastapi"]
        if language == "python"
        else ["route_handler_entry", "route_receiver_guard"]
    )
    cases = [
        load_case(EVAL_DIR / "fixtures" / language / name / "expected.toml")
        for name in names
    ]
    ids = base_ids(); cand = {**ids, "sut_sha_full": "b"*40, "sut_digest": CANDIDATE_DIGEST}
    base = fake_run(ids); candidate = fake_run(cand)
    base["matrix"] = [
        cli.matrix_result_to_json(_run_callers_case(case, language, FixtureSut(False)))
        for case in cases
    ]
    candidate["matrix"] = [
        cli.matrix_result_to_json(_run_callers_case(case, language, FixtureSut(True)))
        for case in cases
    ]
    unchanged = compare_runs(base, copy.deepcopy(base), DECLARED_ADJ)
    assert unchanged["verdict"] == "NO_REGRESSION" and unchanged["matrix_diffs"] == {}
    index = names.index(bad_name)
    fixture_id = str(cases[index].path)
    missing = copy.deepcopy(candidate)
    missing["matrix"].pop(index)
    missing_cmp = compare_runs(base, missing, DECLARED_ADJ)
    assert missing_cmp["matrix_diffs"][fixture_id] == ["ok", None]
    cmp = compare_runs(base, candidate, DECLARED_ADJ)
    assert cmp["matrix_diffs"][fixture_id] == ["ok", "regression"]
    assert cmp["verdict"] == "REGRESSION(matrix_no_regression)"


def _pinned_edge(file, line):
    fd = FunctionDef("f", "function", None, Location("src/f.rs", 1, 1), 1)
    return CallEdge("caller", fd, Location(file, line, line), "caller", Location(file, line, line))


def test_pinned_oracle_sites_use_sets_for_improvements_and_real_changes():
    cfg = {"id": "pin-duplicates", "expected": "known_fail"}
    a, b, miss = _pinned_edge("src/a.rs", 3), _pinned_edge("src/b.rs", 8), _pinned_edge("src/miss.rs", 9)
    known = pinned_mod.evaluate_pinned(cfg, [miss], [a, b], False)
    flip = pinned_mod.evaluate_pinned(cfg, [b, a], [b, a, a], False)
    ids = base_ids(); cand = {**ids, "sut_sha_full": "b"*40, "sut_digest": CANDIDATE_DIGEST}
    cmp = compare_runs(fake_run(ids, pinned=[known]), fake_run(cand, pinned=[flip]), DECLARED_ADJ)
    assert cmp["pinned_oracle_changed"] == []
    assert cmp["criteria"]["pinned_no_regression"] is True
    assert cmp["verdict"] == "NO_REGRESSION"
    moved = copy.deepcopy(flip)
    moved["oracle_sites"] = ["src/other.rs:4"]
    changed = compare_runs(fake_run(ids, pinned=[known]), fake_run(cand, pinned=[moved]), DECLARED_ADJ)
    assert changed["pinned_oracle_changed"] == ["pin-duplicates"]
    assert changed["pinned_diffs"] == {}


def test_pinned_oracle_site_duplicates_do_not_hide_regression():
    cfg = {"id": "pin-duplicates", "expected": "known_fail"}
    a, b, miss = _pinned_edge("src/a.rs", 3), _pinned_edge("src/b.rs", 8), _pinned_edge("src/miss.rs", 9)
    flip = pinned_mod.evaluate_pinned(cfg, [a, b], [b, a, a], False)
    known = pinned_mod.evaluate_pinned(cfg, [miss], [a, b], False)
    ids = base_ids(); cand = {**ids, "sut_sha_full": "b"*40, "sut_digest": CANDIDATE_DIGEST}
    cmp = compare_runs(fake_run(ids, pinned=[flip]), fake_run(cand, pinned=[known]), DECLARED_ADJ)
    assert cmp["pinned_oracle_changed"] == []
    assert cmp["pinned_diffs"] == {"pin-duplicates": ["flip_candidate", "known_fail"]}
    assert cmp["verdict"] == "REGRESSION(pinned_no_regression)"


def test_markdown_renders_complete_comparison_contract():
    ids = base_ids(); cand = {**ids, "sut_sha_full": "b"*40, "sut_digest": CANDIDATE_DIGEST}
    cmp = compare_runs(fake_run(ids), fake_run(cand), DECLARED_ADJ)
    cmp["runs"] = {
        "base": {"path_base": "REPO-ROOT-BASE", "path": "BASE-RUN-PATH", "digest": "BASE-RUN-DIGEST"},
        "candidate": {"path_base": "REPO-ROOT-BASE", "path": "CAND-RUN-PATH", "digest": "CAND-RUN-DIGEST"},
    }
    cmp["meta"] = {"harness_sha": "COMPARE-HARNESS-SHA", "generated_at": "GENERATED-AT"}
    cmp["policy"] = {"oracle_changed_max_fraction": 0.123, "min_comparable_per_stratum": 7}
    cmp["oracle_changed"] = {"count": 1, "fraction": 0.03125, "ids": ["ORACLE-CHANGED-ID"]}
    cmp["oracle_changed_answers"] = {"ORACLE-CHANGED-ID": {"base": ["BASE-ANSWER"], "candidate": ["CAND-ANSWER"]}}
    cmp["pinned_oracle_changed"] = ["PINNED-EXCLUSION-ID"]
    cmp["pinned_diffs"] = {"PIN-DIFF-ID": ["PIN-BEFORE", "PIN-AFTER"]}
    cmp["matrix_diffs"] = {"MATRIX-DIFF-ID": ["MATRIX-BEFORE", "MATRIX-AFTER"]}
    cmp["wilson_flags"] = ["WILSON-FLAG-VALUE"]
    cmp["recall_regressions"] = ["RECALL-REGRESSION-VALUE"]
    cmp["lost_matched_oracle_sites"] = {"LOST-PROBE-ID": [["LOST-SITE", 1, 1]]}
    cmp["status_transitions"] = {"STATUS-ID": ["STATUS-BEFORE", "STATUS-AFTER"]}
    cmp["m2"] = {
        "base": {"direction": {"stratum": {"raw": {"marker": "BASE-RAW"}, "corrected": {"marker": "BASE-CORRECTED"}}}},
        "candidate": {"direction": {"stratum": {"raw": {"marker": "CAND-RAW"}, "corrected": {"marker": "CAND-CORRECTED"}}}},
    }
    cmp["full_sample_m2"] = {"base": {"marker": "BASE-FULL-SAMPLE"}, "candidate": {"marker": "CAND-FULL-SAMPLE"}}
    markdown = render_compare_md(cmp)
    for literal in (
        "REPO-ROOT-BASE", "BASE-RUN-PATH", "BASE-RUN-DIGEST", "CAND-RUN-PATH", "CAND-RUN-DIGEST",
        "COMPARE-HARNESS-SHA", "GENERATED-AT", "0.123", "7", "1", "0.03125",
        "ORACLE-CHANGED-ID", "BASE-ANSWER", "CAND-ANSWER", "PINNED-EXCLUSION-ID",
        "PIN-DIFF-ID", "MATRIX-DIFF-ID", "WILSON-FLAG-VALUE", "RECALL-REGRESSION-VALUE", "LOST-PROBE-ID", "LOST-SITE",
        "STATUS-ID", "BASE-RAW", "CAND-RAW", "BASE-CORRECTED", "CAND-CORRECTED",
        "BASE-FULL-SAMPLE", "CAND-FULL-SAMPLE",
    ):
        assert literal in markdown


def test_not_comparable_reports_identity_churn_and_floor_reasons_in_order():
    ids = base_ids(); cand = {**ids, "sut_sha_full": "b"*40, "sut_digest": CANDIDATE_DIGEST}
    base = fake_run(ids); candidate = fake_run(cand)
    candidate["meta"]["harness_sha"] = "d" * 40
    for i in range(1, 4):
        candidate["probes"][f"callers:src/U-free.rs:{i}"]["oracle_sites"] = [["changed.rs", 9, 9]]
    candidate["probes"]["callers:src/U-method.rs:1"]["oracle_sites"] = [["changed.rs", 9, 9]]
    cmp = compare_runs(base, candidate, DECLARED_ADJ)
    assert cmp["verdict"] == "NOT_COMPARABLE(harness_sha; oracle_changed: 4/30 > 0.10; stratum U-free: 3 comparable < 4)"
    assert cmp["oracle_changed"] == {
        "count": 4,
        "fraction": 4 / 30,
        "ids": [
            "callers:src/U-free.rs:1", "callers:src/U-free.rs:2",
            "callers:src/U-free.rs:3", "callers:src/U-method.rs:1",
        ],
    }
