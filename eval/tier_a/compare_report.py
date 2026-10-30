"""Base-vs-candidate Tier-A comparative report."""
from __future__ import annotations

import json
from collections import Counter

from .adjudication import Adjudication
from .compare import match_oracle_sites
from .launch import launch_refusal_reason
from .policy import ORACLE_ANSWER_STATUSES, SCORED_PROBE_STATUSES
from .record_schema import (
    identity_refusals,
    record_refusals,
    required_identity_leaf_paths,
)
from .strata import STRATA


CLASSES = (
    "carried_same",
    "carried_changed/sut_change",
    "carried_changed/content_change",
    "resampled",
    "unexplained",
    "oracle_changed",
)
COMPARE_IDENTITIES = (
    "corpus_sha_full",
    "sha_aliases",
    "closure_digest",
    "oracle",
    "oracle_init",
    "policy_digest",
    "population_digest",
    "snapshot_digest",
    "sample_digest",
    "runtime",
    "data",
    "env_allow",
    "env_values",
    "harness_sha",
    "seed",
)
COMPARE_POLICY = {
    "oracle_changed_max_fraction": 0.10,
    "min_comparable_per_stratum": 4,
}
REQUIRED_IDENTITIES = ("sut_sha_full", "sut_digest", *COMPARE_IDENTITIES)
SUT_ERROR_LEG = ("sut_error",)
PINNED_RANK = {
    "ok": 2,
    "flip_candidate": 2,
    "known_fail": 1,
    "regression": 0,
    "missing": 0,
    "unsupported_expected": 0,
    "seed_missing": 0,
    "error": 0,
}

_ORACLE_ERROR = ("oracle_error",)
_SCORED = SCORED_PROBE_STATUSES


def _canonical_sites(probe: dict, key: str) -> tuple:
    return tuple(sorted({tuple(site[:3]) for site in probe.get(key, [])}))


def _canonical_functions(probe: dict, key: str) -> tuple:
    return tuple(sorted({tuple(function) for function in probe.get(key, [])}))


def oracle_answer(status: str, probe: dict | None) -> tuple:
    """Return the canonical oracle-only answer recorded for one probe leg."""
    if status == "sut_error":
        return SUT_ERROR_LEG
    if status not in _SCORED:
        return _ORACLE_ERROR
    probe = probe or {}
    return (
        _canonical_functions(probe, "oracle_functions"),
        _canonical_sites(probe, "oracle_sites"),
    )


def _prism_answer(probe: dict | None) -> tuple:
    probe = probe or {}
    return (
        _canonical_functions(probe, "prism_functions"),
        _canonical_sites(probe, "prism_sites"),
    )


def join_probes(
    base_run: dict, cand_run: dict
) -> dict[str, tuple[str, dict | None, str, dict | None]]:
    """Join a fixed sample by the persisted probe-status keys."""
    return {
        pid: (
            base_run["probe_status"][pid],
            base_run["probes"].get(pid),
            cand_run["probe_status"][pid],
            cand_run["probes"].get(pid),
        )
        for pid in sorted(base_run["probe_status"])
    }


def partition_by_oracle(joined: dict) -> tuple[dict, dict]:
    """Split joined probes by equality of their recorded oracle answers."""
    comparable = {}
    changed = {}
    for pid, legs in joined.items():
        base_status, base, cand_status, cand = legs
        if (
            base_status not in ORACLE_ANSWER_STATUSES
            or cand_status not in ORACLE_ANSWER_STATUSES
        ):
            changed[pid] = legs
            continue
        base_answer = oracle_answer(base_status, base)
        cand_answer = oracle_answer(cand_status, cand)
        sut_error_comparable = (
            SUT_ERROR_LEG in (base_answer, cand_answer)
            and _ORACLE_ERROR not in (base_answer, cand_answer)
        )
        if base_answer == cand_answer or sut_error_comparable:
            comparable[pid] = legs
        else:
            changed[pid] = legs
    return comparable, changed


def classify_probe(
    pid: str,
    base_status: str,
    base: dict | None,
    cand_status: str,
    cand: dict | None,
    ids_base: dict,
    ids_cand: dict,
) -> str:
    """Classify one oracle-comparable probe."""
    del pid
    if "sut_error" in (base_status, cand_status) and base_status != cand_status:
        if ids_base.get("closure_digest") != ids_cand.get("closure_digest"):
            return "carried_changed/content_change"
        if ids_base.get("sut_digest") != ids_cand.get("sut_digest"):
            return "carried_changed/sut_change"
        return "unexplained"
    if _prism_answer(base) == _prism_answer(cand):
        return "carried_same"
    if ids_base.get("closure_digest") != ids_cand.get("closure_digest"):
        return "carried_changed/content_change"
    if ids_base.get("sut_digest") != ids_cand.get("sut_digest"):
        return "carried_changed/sut_change"
    return "unexplained"


def _metric_rows(base_m2: dict, cand_m2: dict):
    for direction in sorted(set(base_m2) | set(cand_m2)):
        base_strata = base_m2.get(direction, {})
        cand_strata = cand_m2.get(direction, {})
        for stratum in sorted(set(base_strata) | set(cand_strata)):
            yield direction, stratum, base_strata.get(stratum), cand_strata.get(stratum)


def wilson_regressed(
    base_m2: dict, cand_m2: dict, tier: str = "corrected"
) -> list[str]:
    """Describe non-overlapping Wilson intervals; this is not the recall gate."""
    regressed = []
    for direction, stratum, base, cand in _metric_rows(base_m2, cand_m2):
        if base is None or cand is None:
            continue
        for metric in ("precision", "recall"):
            base_interval = base[tier][metric]
            cand_interval = cand[tier][metric]
            if cand_interval[2] < base_interval[1]:
                regressed.append(f"{direction}/{stratum}/{tier}/{metric}")
    return regressed


def recall_nondecreasing(
    base_m2: dict, cand_m2: dict, tier: str = "corrected"
) -> list[str]:
    """Gate on nondecreasing measured recall for every direction and stratum."""
    regressed = []
    for direction, stratum, base, cand in _metric_rows(base_m2, cand_m2):
        prefix = f"{direction}/{stratum}/{tier}"
        if base is None or cand is None or tier not in base or tier not in cand:
            regressed.append(f"{prefix}/missing")
        elif cand[tier]["recall"][0] < base[tier]["recall"][0]:
            regressed.append(f"{prefix}/recall")
    return regressed


def _empty_report(base_run: dict, cand_run: dict, policy: dict) -> dict:
    base_meta = base_run.get("meta", {})
    cand_meta = cand_run.get("meta", {})
    identities = {
        "base_sha": base_meta.get("sut_sha_full"),
        "candidate_sha": cand_meta.get("sut_sha_full"),
        "base_sut_digest": base_meta.get("sut_digest"),
        "candidate_sut_digest": cand_meta.get("sut_digest"),
        **{key: base_meta.get(key) for key in COMPARE_IDENTITIES},
    }
    return {
        "identities": identities,
        "identity_legs": {
            "base": {key: base_meta.get(key) for key in COMPARE_IDENTITIES},
            "candidate": {key: cand_meta.get(key) for key in COMPARE_IDENTITIES},
        },
        "policy": dict(policy),
        "runs": {"base": None, "candidate": None},
        "criteria": {},
        "classes": {},
        "counts": {name: 0 for name in CLASSES},
        "oracle_changed": {"count": 0, "fraction": 0.0, "ids": []},
        "oracle_changed_answers": {},
        "status_transitions": {},
        "pinned_oracle_changed": [],
        "pinned_diffs": {},
        "matrix_diffs": {},
        "m2": {"base": {}, "candidate": {}},
        "full_sample_m2": {
            "base": base_run.get("m2", {}),
            "candidate": cand_run.get("m2", {}),
        },
        "lost_matched_oracle_sites": {},
        "wilson_flags": [],
        "recall_regressions": [],
    }


def _stratum(legs: tuple) -> str | None:
    for probe in (legs[1], legs[3]):
        if probe is not None and probe.get("stratum") is not None:
            return probe["stratum"]
    return None


def _common_m2(run: dict, ids: set[str], records: list[Adjudication]) -> dict:
    from .policy import _compute_m2_and_pending

    probes = {
        pid: run["probes"][pid]
        for pid in sorted(ids)
        if pid in run["probes"]
    }
    probes["_corpus"] = run["probes"]["_corpus"]
    probes["_strata"] = run["probes"]["_strata"]
    return _compute_m2_and_pending(
        probes, records, run.get("site_fingerprints", {})
    )[0]


def _lost_matched_oracle_sites(comparable: dict) -> dict[str, list[list]]:
    """Return base true-positive sites absent from an oracle-stable candidate."""
    losses = {}
    for pid, legs in comparable.items():
        base_status, base, cand_status, cand = legs
        if base_status not in _SCORED or cand_status not in _SCORED:
            continue
        oracle_sites = set(_canonical_sites(base or {}, "oracle_sites"))
        representative = {}
        for window in sorted(oracle_sites):
            representative.setdefault(window[:2], window)
        base_matches = {
            window[:2]
            for window in match_oracle_sites(
                {site[:2] for site in _canonical_sites(base or {}, "prism_sites")},
                oracle_sites,
            ).values()
        }
        candidate_matches = {
            window[:2]
            for window in match_oracle_sites(
                {site[:2] for site in _canonical_sites(cand or {}, "prism_sites")},
                oracle_sites,
            ).values()
        }
        lost_identities = sorted(base_matches - candidate_matches)
        if lost_identities:
            losses[pid] = [list(representative[site]) for site in lost_identities]
    return losses


def _canonical_pinned_sites(row: dict) -> tuple:
    sites = {
        tuple(site[:3]) if isinstance(site, (list, tuple)) else site
        for site in row.get("oracle_sites", [])
    }
    return tuple(sorted(sites, key=lambda site: json.dumps(site, sort_keys=True)))


def _pinned_result(base_run: dict, cand_run: dict) -> tuple[list[str], dict, bool]:
    base = {row["id"]: row for row in base_run.get("pinned", [])}
    cand = {row["id"]: row for row in cand_run.get("pinned", [])}
    changed = []
    diffs = {}
    no_regression = True
    for pid in sorted(set(base) | set(cand)):
        b = base.get(pid)
        c = cand.get(pid)
        oracle_moved = (
            (b or {}).get("error_side") == "oracle"
            or (c or {}).get("error_side") == "oracle"
            or (
                b is not None
                and c is not None
                and "oracle_sites" in b
                and "oracle_sites" in c
                and _canonical_pinned_sites(b) != _canonical_pinned_sites(c)
            )
        )
        if oracle_moved:
            changed.append(pid)
            continue
        base_outcome = (b or {}).get("outcome", "error")
        cand_outcome = (c or {}).get("outcome", "error")
        if base_outcome != cand_outcome:
            diffs[pid] = [base_outcome, cand_outcome]
        if PINNED_RANK.get(cand_outcome, 0) < PINNED_RANK.get(base_outcome, 0):
            no_regression = False
    return changed, diffs, no_regression


def _matrix_result(base_run: dict, cand_run: dict) -> tuple[dict, bool]:
    def key(row: dict) -> str:
        if "id" in row:
            return str(row["id"])
        return "/".join(
            str(row.get(name, "")) for name in ("language", "capability", "name")
        ).rstrip("/")

    base = {key(row): row for row in base_run.get("matrix", [])}
    cand = {key(row): row for row in cand_run.get("matrix", [])}
    diffs = {}
    no_regression = set(base) == set(cand)
    for mid in sorted(set(base) | set(cand)):
        before = base.get(mid, {}).get("outcome")
        after = cand.get(mid, {}).get("outcome")
        if before != after:
            diffs[mid] = [before, after]
        if after == "regression" and before != "regression":
            no_regression = False
    return diffs, no_regression


def _adjudications_digest(meta: dict) -> tuple[str | None, str | None]:
    data = meta.get("data", {})
    if not isinstance(data, dict):
        return None, None
    lock_present = "adjudications" in data
    legacy_present = "eval/adjudications.jsonl" in data
    if lock_present == legacy_present:
        return None, None
    if lock_present:
        entry = data.get("adjudications")
        if not isinstance(entry, dict) or entry.get("path") != "eval/adjudications.jsonl":
            return None, None
        key = "data.adjudications.digest"
    else:
        entry = data.get("eval/adjudications.jsonl")
        if not isinstance(entry, dict):
            return None, None
        key = "data.eval/adjudications.jsonl.digest"
    digest = entry.get("digest")
    return (digest, key) if isinstance(digest, str) else (None, None)


def _compare_identity(meta: dict, key: str):
    value = meta.get(key)
    if key != "data" or not isinstance(value, dict):
        return value
    return {
        name: entry
        for name, entry in value.items()
        if name not in {"adjudications", "eval/adjudications.jsonl"}
    }


def _invalid_identities(side: str, meta: dict) -> list[str]:
    return [f"{side}.{reason}" for reason in identity_refusals(meta)]


def _baseline_refusals(base_meta: dict, cand_meta: dict) -> list[str]:
    refusals = []
    for side, meta in (("base", base_meta), ("candidate", cand_meta)):
        if not meta.get("baseline_invalid"):
            continue
        reasons = meta.get("invalid_reasons")
        if not isinstance(reasons, list) or not reasons:
            reasons = ["unspecified"]
        refusals.append(f"baseline_invalid: {side}: {', '.join(map(str, reasons))}")
    return refusals


def compare_runs(
    base_run: dict,
    cand_run: dict,
    adjudications: tuple[list[Adjudication], str],
    policy: dict = COMPARE_POLICY,
) -> dict:
    """Compare two run records without consulting ambient harness state."""
    records, adjudications_digest = adjudications
    base_meta = base_run.get("meta", {})
    cand_meta = cand_run.get("meta", {})
    report = _empty_report(base_run, cand_run, policy)

    launch_refusals = []
    for side, meta in (("base", base_meta), ("candidate", cand_meta)):
        reason = launch_refusal_reason(meta.get("launch"))
        if reason is not None:
            launch_refusals.append(f"{side}: {reason}")
    if launch_refusals:
        report["verdict"] = f"NOT_COMPARABLE(launch_invalid: {', '.join(launch_refusals)})"
        return report

    invalid_identities = _invalid_identities("base", base_meta)
    invalid_identities.extend(_invalid_identities("candidate", cand_meta))
    if invalid_identities:
        report["verdict"] = (
            f"NOT_COMPARABLE(identity_invalid: {', '.join(invalid_identities)})"
        )
        return report

    if base_meta.get("admitted") is not True or cand_meta.get("admitted") is not True:
        report["verdict"] = "NOT_COMPARABLE(admitted)"
        return report

    invalid_records = [f"base.{reason}" for reason in record_refusals(base_run)]
    invalid_records.extend(
        f"candidate.{reason}" for reason in record_refusals(cand_run)
    )
    if invalid_records:
        report["verdict"] = (
            f"NOT_COMPARABLE(record_invalid: {', '.join(invalid_records)})"
        )
        return report

    identities_equal = all(
        _compare_identity(base_meta, key) == _compare_identity(cand_meta, key)
        for key in COMPARE_IDENTITIES
    )
    both_admitted = bool(base_meta.get("admitted") and cand_meta.get("admitted"))
    both_baseline_valid = not (
        base_meta.get("baseline_invalid") or cand_meta.get("baseline_invalid")
    )
    base_probe_set = set(base_run.get("probe_status", {}))
    cand_probe_set = set(cand_run.get("probe_status", {}))
    probe_sets_equal = base_probe_set == cand_probe_set
    declared_digest, base_digest_key = _adjudications_digest(base_meta)
    candidate_digest, candidate_digest_key = _adjudications_digest(cand_meta)
    adjudications_match = (
        adjudications_digest == declared_digest == candidate_digest
    )
    criteria = {
        "both_admitted": both_admitted,
        "both_baseline_valid": both_baseline_valid,
        "identities_equal": identities_equal,
        "probe_sets_equal": probe_sets_equal,
        "adjudications_match": adjudications_match,
        "adjudications_digest_keys": {
            "base": base_digest_key,
            "candidate": candidate_digest_key,
        },
        "oracle_answers_joined": probe_sets_equal,
        "oracle_changed_within_threshold": True,
        "no_new_sut_errors": True,
        "unexplained_zero": True,
        "no_recall_loss": True,
        "no_lost_matched_oracle_sites": True,
        "matrix_no_regression": True,
        "pinned_no_regression": True,
    }
    report["criteria"] = criteria

    refusal = [
        key
        for key in COMPARE_IDENTITIES
        if _compare_identity(base_meta, key) != _compare_identity(cand_meta, key)
    ]
    if not both_admitted:
        refusal.append("admitted")
    refusal.extend(_baseline_refusals(base_meta, cand_meta))
    if not probe_sets_equal:
        refusal.append("probe_set")
    if not adjudications_match:
        refusal.append("adjudications_digest")
    if not probe_sets_equal:
        report["verdict"] = f"NOT_COMPARABLE({'; '.join(refusal)})"
        return report

    joined = join_probes(base_run, cand_run)
    comparable, oracle_changed = partition_by_oracle(joined)
    total = len(joined)
    changed_ids = sorted(oracle_changed)
    fraction = len(changed_ids) / total if total else 0.0
    report["oracle_changed"] = {
        "count": len(changed_ids),
        "fraction": fraction,
        "ids": changed_ids,
    }
    report["oracle_changed_answers"] = {
        pid: {
            "base": oracle_answer(legs[0], legs[1]),
            "candidate": oracle_answer(legs[2], legs[3]),
        }
        for pid, legs in sorted(oracle_changed.items())
    }

    threshold = policy["oracle_changed_max_fraction"]
    oracle_refusal = []
    if fraction > threshold:
        oracle_refusal.append(
            f"oracle_changed: {len(changed_ids)}/{total} > {threshold:.2f}"
        )
    present_strata = {_stratum(legs) for legs in joined.values()} - {None}
    floor = policy["min_comparable_per_stratum"]
    for stratum in STRATA:
        if stratum not in present_strata:
            continue
        count = sum(_stratum(legs) == stratum for legs in comparable.values())
        if count < floor:
            oracle_refusal.append(f"stratum {stratum}: {count} comparable < {floor}")
    criteria["oracle_changed_within_threshold"] = not oracle_refusal
    refusal.extend(oracle_refusal)
    if refusal:
        report["verdict"] = f"NOT_COMPARABLE({'; '.join(refusal)})"
        return report

    classes = {
        pid: classify_probe(pid, *legs, base_meta, cand_meta)
        for pid, legs in comparable.items()
    }
    report["classes"] = classes
    counts = Counter(classes.values())
    counts["oracle_changed"] = len(changed_ids)
    report["counts"] = {name: counts[name] for name in CLASSES}
    report["status_transitions"] = {
        pid: [legs[0], legs[2]]
        for pid, legs in comparable.items()
        if legs[0] != legs[2]
    }
    criteria["no_new_sut_errors"] = not any(
        legs[0] in _SCORED and legs[2] == "sut_error"
        for legs in comparable.values()
    )
    criteria["unexplained_zero"] = counts["unexplained"] == 0

    common_scored = {
        pid
        for pid, legs in comparable.items()
        if legs[0] in _SCORED and legs[2] in _SCORED
    }
    base_m2 = _common_m2(base_run, common_scored, records)
    cand_m2 = _common_m2(cand_run, common_scored, records)
    report["m2"] = {"base": base_m2, "candidate": cand_m2}
    report["wilson_flags"] = wilson_regressed(base_m2, cand_m2)
    report["recall_regressions"] = recall_nondecreasing(base_m2, cand_m2)
    criteria["no_recall_loss"] = not report["recall_regressions"]
    report["lost_matched_oracle_sites"] = _lost_matched_oracle_sites(comparable)
    criteria["no_lost_matched_oracle_sites"] = not report[
        "lost_matched_oracle_sites"
    ]

    pinned_changed, pinned_diffs, pinned_ok = _pinned_result(base_run, cand_run)
    report["pinned_oracle_changed"] = pinned_changed
    report["pinned_diffs"] = pinned_diffs
    criteria["pinned_no_regression"] = pinned_ok
    matrix_diffs, matrix_ok = _matrix_result(base_run, cand_run)
    report["matrix_diffs"] = matrix_diffs
    criteria["matrix_no_regression"] = matrix_ok

    regressions = []
    for criterion, label in (
        ("no_new_sut_errors", "no_new_sut_errors"),
        ("unexplained_zero", "unexplained"),
        ("no_recall_loss", "no_recall_loss"),
        ("no_lost_matched_oracle_sites", "no_lost_matched_oracle_sites"),
        ("matrix_no_regression", "matrix_no_regression"),
        ("pinned_no_regression", "pinned_no_regression"),
    ):
        if not criteria[criterion]:
            regressions.append(label)
    report["verdict"] = (
        f"REGRESSION({'; '.join(regressions)})" if regressions else "NO_REGRESSION"
    )
    return report


def render_compare_md(cmp: dict) -> str:
    """Render the JSON comparison as a reviewable Markdown report."""
    from .compare_render import render_compare_md as render

    return render(cmp)
