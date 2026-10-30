"""Single whole-record schema used by admission, readmission, and comparison."""
from __future__ import annotations

import re
from collections import Counter

from .launch import PROTOCOL
from .strata import STRATA


_SHA_PATTERN = re.compile(r"[0-9a-f]{40}")
_DIGEST_PATTERN = re.compile(r"sha256:[0-9a-f]{64}")
ORACLE_ANSWER_STATUSES = frozenset({"ok", "inventory_miss", "sut_error"})
PROBE_STATUS_VOCABULARY = ORACLE_ANSWER_STATUSES | {
    "oracle_error",
    "oracle_timeout",
    "seed_unaddressable",
}


def _sha(value) -> bool:
    return isinstance(value, str) and _SHA_PATTERN.fullmatch(value) is not None


def _digest(value) -> bool:
    return isinstance(value, str) and _DIGEST_PATTERN.fullmatch(value) is not None


def _digest_or_absent(value) -> bool:
    return value == "absent" or _digest(value)


def _nonempty_string(value) -> bool:
    return isinstance(value, str) and bool(value.strip())


def _boolean(value) -> bool:
    return isinstance(value, bool)


def _true(value) -> bool:
    return value is True


def _nonnegative_int(value) -> bool:
    return isinstance(value, int) and not isinstance(value, bool) and value >= 0


def _integer(value) -> bool:
    return isinstance(value, int) and not isinstance(value, bool)


def _sha_list(value) -> bool:
    return isinstance(value, list) and bool(value) and all(_sha(item) for item in value)


def _string_list(value) -> bool:
    return isinstance(value, list) and all(_nonempty_string(item) for item in value)


def _nonempty_string_list(value) -> bool:
    return _string_list(value) and bool(value)


def _string_map(value) -> bool:
    return (
        isinstance(value, dict)
        and bool(value)
        and all(_nonempty_string(key) and isinstance(item, str) for key, item in value.items())
    )


def _mapping(value) -> bool:
    return isinstance(value, dict)


def _list(value) -> bool:
    return isinstance(value, list)


def _site_list(value) -> bool:
    return isinstance(value, list) and all(
        isinstance(site, list)
        and len(site) in {3, 4}
        and _nonempty_string(site[0])
        and _nonnegative_int(site[1])
        and _nonnegative_int(site[2])
        and (
            len(site) == 3
            or (
                isinstance(site[3], dict)
                and (
                    "resolution_kind" not in site[3]
                    or site[3]["resolution_kind"] is None
                    or _nonempty_string(site[3]["resolution_kind"])
                )
                and (
                    "dispatch_kind" not in site[3]
                    or site[3]["dispatch_kind"] is None
                    or _nonempty_string(site[3]["dispatch_kind"])
                )
                and (
                    "score" not in site[3]
                    or site[3]["score"] is None
                    or (
                        isinstance(site[3]["score"], (int, float))
                        and not isinstance(site[3]["score"], bool)
                    )
                )
            )
        )
        for site in value
    )


def _function_list(value) -> bool:
    return isinstance(value, list) and all(
        isinstance(row, list)
        and len(row) == 2
        and all(_nonempty_string(item) for item in row)
        for row in value
    )


def _string_values_map(value) -> bool:
    return isinstance(value, dict) and all(
        _nonempty_string(key) and _nonempty_string(item)
        for key, item in value.items()
    )


_PINNED_OUTCOMES = {
    "ok",
    "flip_candidate",
    "known_fail",
    "regression",
    "missing",
    "unsupported_expected",
    "seed_missing",
    "error",
    "sut_error",
    "sut_timeout",
    "oracle_error",
    "oracle_timeout",
}
_MATRIX_OUTCOMES = {"ok", "regression", "expected_gap", "flip_candidate"}


_COMMON_IDENTITY_SCHEMA = {
    "corpus_sha_full": _sha,
    "sha_aliases": _sha_list,
    "closure_digest": _digest,
    "sut_sha_full": _sha,
    "sut_digest": _digest,
    "oracle": _nonempty_string,
    "oracle_init": {"cargo": {"features": _nonempty_string}},
    "policy_digest": _digest,
    "population_digest": _digest,
    "snapshot_digest": _digest,
    "env_allow": _nonempty_string_list,
    "env_values": _string_map,
    "harness_sha": _sha,
    "seed": _integer,
}

_IDENTITY_SCHEMAS = {
    "legacy": {
        **_COMMON_IDENTITY_SCHEMA,
        "sample_digest": _digest,
        "runtime": {"python": _nonempty_string, "uv_lock": _digest},
        "data": {"eval/adjudications.jsonl": {"digest": _digest}},
    },
    "legacy_lock_data": {
        **_COMMON_IDENTITY_SCHEMA,
        "sample_digest": _digest,
        "runtime": {"python": _nonempty_string, "uv_lock": _digest},
        "data": {
            "adjudications": {"path": _nonempty_string, "digest": _digest}
        },
    },
    "lock_v1": {
        **_COMMON_IDENTITY_SCHEMA,
        "sample_digest": {"full": _digest, "quick": _digest},
        "runtime": {
            "python": _nonempty_string,
            "python_version_file": _digest,
            "uv_lock": _digest,
            "rustc": _nonempty_string,
            "cargo": _nonempty_string,
            "rust_toolchain": _digest_or_absent,
            "cargo_config": _digest_or_absent,
        },
        "data": {
            "adjudications": {
                "path": lambda value: value == "eval/adjudications.jsonl",
                "digest": _digest,
                "records": _nonnegative_int,
            },
            "fixtures": _digest,
            "pinned_probes": _digest,
        },
    },
}


def _identity_schema(meta: dict) -> dict:
    data = meta.get("data")
    if isinstance(meta.get("sample_digest"), dict):
        version = "lock_v1"
    elif isinstance(data, dict) and "adjudications" in data:
        version = "legacy_lock_data"
    else:
        version = "legacy"
    return _IDENTITY_SCHEMAS[version]


def identity_schema(meta: dict) -> dict:
    return _identity_schema(meta)


def required_identity_leaf_paths(meta: dict) -> list[tuple]:
    return _schema_leaf_paths(_identity_schema(meta))


def identity_refusals(meta: dict) -> list[str]:
    return [
        ".".join(map(str, path))
        for path in _invalid_schema_paths(meta, _identity_schema(meta))
    ]


def record_schema(record: dict) -> dict:
    meta = record.get("meta", {}) if isinstance(record, dict) else {}
    return {
        "meta": {
            **_identity_schema(meta),
            "admitted": _true,
            "baseline_invalid": _boolean,
            "invalid_reasons": _string_list,
            "sample_per_stratum": _nonnegative_int,
            "launch": {
                "isolated": _true,
                "prefix_fresh": _true,
                "protocol": lambda value: value == PROTOCOL,
            },
        },
        "probes": _mapping,
        "probe_status": _mapping,
        "site_fingerprints": _string_values_map,
        "pinned": _list,
        "matrix": _list,
        "failures": _mapping,
        "m1": _mapping,
        "m2": _mapping,
        "m3": _mapping,
        "oracle_filtered": _list,
        "pending": _list,
    }


def _schema_leaf_paths(schema: dict, prefix=()) -> list[tuple]:
    paths = []
    for key, expected in schema.items():
        path = (*prefix, key)
        if isinstance(expected, dict):
            paths.extend(_schema_leaf_paths(expected, path))
        else:
            paths.append(path)
    return paths


def _probe_schema(status: str) -> dict:
    schema = {
        "outcome": _nonempty_string,
        "direction": lambda value: value in {"callers", "callees"},
        "stratum": _nonempty_string,
        "seed_def": _nonempty_string,
    }
    if status in ORACLE_ANSWER_STATUSES:
        schema.update(
            prism_sites=_site_list,
            oracle_sites=_site_list,
            prism_functions=_function_list,
            oracle_functions=_function_list,
        )
    return schema


def required_record_leaf_paths(record: dict) -> list[tuple]:
    paths = _schema_leaf_paths(record_schema(record))
    probes = record.get("probes", {})
    statuses = record.get("probe_status", {})
    if isinstance(probes, dict):
        paths.extend(("probes", key) for key in ("_corpus", "_strata"))
        strata = probes.get("_strata", {})
        if isinstance(strata, dict):
            for stratum in sorted(strata):
                paths.extend(
                    ("probes", "_strata", stratum, field)
                    for field in (
                        "population_symbols",
                        "eligible_symbols",
                        "eligible_probes",
                        "target",
                    )
                )
        for pid in sorted(key for key in probes if not key.startswith("_")):
            status = statuses.get(pid) if isinstance(statuses, dict) else None
            paths.extend(
                ("probes", pid, field) for field in _probe_schema(status)
            )
            paths.append(("probe_status", pid))
    pinned = record.get("pinned", [])
    if isinstance(pinned, list):
        for index, row in enumerate(pinned):
            if isinstance(row, dict):
                paths.extend(("pinned", index, field) for field in ("id", "outcome"))
                if row.get("error_side") != "oracle":
                    paths.append(("pinned", index, "oracle_sites"))
    matrix = record.get("matrix", [])
    if isinstance(matrix, list):
        for index, row in enumerate(matrix):
            if isinstance(row, dict):
                paths.extend(
                    ("matrix", index, field)
                    for field in ("id", "language", "capability", "outcome")
                )
    return paths


def _invalid_schema_paths(value, schema, path=()) -> list[tuple]:
    if not isinstance(schema, dict):
        return [] if schema(value) else [path]
    if not isinstance(value, dict):
        return [path]
    invalid = []
    for key, expected in schema.items():
        child = (*path, key)
        if key not in value:
            invalid.extend(
                _schema_leaf_paths(expected, child)
                if isinstance(expected, dict)
                else [child]
            )
        else:
            invalid.extend(_invalid_schema_paths(value[key], expected, child))
    return invalid


def validate_run_record(record: dict, *, purpose: str) -> list[str]:
    if purpose not in {"admission", "compare", "lock", "publication"}:
        raise ValueError(f"unknown record validation purpose: {purpose}")
    invalid = _invalid_schema_paths(record, record_schema(record))
    refusals = [".".join(map(str, path)) for path in invalid]
    if not isinstance(record, dict):
        return refusals
    meta = record.get("meta", {})
    if isinstance(meta, dict):
        invalid_flag = meta.get("baseline_invalid")
        reasons = meta.get("invalid_reasons")
        if isinstance(invalid_flag, bool) and isinstance(reasons, list):
            if invalid_flag != bool(reasons):
                refusals.append("meta.invalid_reasons: inconsistent")
        if purpose in {"compare", "lock", "publication"} and invalid_flag is True:
            refusals.append(
                "meta.baseline_invalid: " + ", ".join(map(str, reasons or ["unspecified"]))
            )
    probes = record.get("probes")
    statuses = record.get("probe_status")
    if not isinstance(probes, dict) or not isinstance(statuses, dict):
        return sorted(set(refusals))
    probe_ids = {pid for pid in probes if not pid.startswith("_")}
    if set(statuses) != probe_ids:
        refusals.append("probe_status: probe_set_mismatch")
    for pid in sorted(probe_ids & set(statuses)):
        status = statuses[pid]
        if status not in PROBE_STATUS_VOCABULARY:
            refusals.append(f"probe_status.{pid}: vocabulary")
            continue
        probe = probes.get(pid)
        for path in _invalid_schema_paths(probe, _probe_schema(status), ("probes", pid)):
            refusals.append(".".join(map(str, path)))
        if isinstance(probe, dict) and probe.get("outcome") != status:
            refusals.append(f"probes.{pid}.outcome_status_mismatch")
    strata = probes.get("_strata")
    if not _nonempty_string(probes.get("_corpus")):
        refusals.append("probes._corpus")
    if not isinstance(strata, dict):
        refusals.append("probes._strata")
    else:
        stratum_schema = {
            "population_symbols": _nonnegative_int,
            "eligible_symbols": _nonnegative_int,
            "eligible_probes": _nonnegative_int,
            "target": _nonnegative_int,
        }
        for stratum, row in strata.items():
            for path in _invalid_schema_paths(
                row, stratum_schema, ("probes", "_strata", stratum)
            ):
                refusals.append(".".join(map(str, path)))
        if set(strata) != set(STRATA):
            refusals.append("probes._strata: stratum_set_mismatch")
        counts = Counter(
            probe.get("stratum")
            for pid, probe in probes.items()
            if not pid.startswith("_") and isinstance(probe, dict)
        )
        sample_per_stratum = meta.get("sample_per_stratum") if isinstance(meta, dict) else None
        for stratum, row in strata.items():
            if not isinstance(row, dict):
                continue
            eligible_symbols = row.get("eligible_symbols")
            eligible_probes = row.get("eligible_probes")
            target = row.get("target")
            if (
                _nonnegative_int(eligible_symbols)
                and _nonnegative_int(eligible_probes)
                and (
                    eligible_probes != eligible_symbols * 2
                    or counts[stratum] != eligible_probes
                )
            ):
                refusals.append(f"probes._strata.{stratum}: count_mismatch")
            if _nonnegative_int(sample_per_stratum) and target != sample_per_stratum:
                refusals.append(f"probes._strata.{stratum}: target_mismatch")

    pinned = record.get("pinned")
    if isinstance(pinned, list):
        pinned_ids = []
        for index, row in enumerate(pinned):
            prefix = f"pinned.{index}"
            if not isinstance(row, dict):
                refusals.append(prefix)
                continue
            pid = row.get("id")
            outcome = row.get("outcome")
            if not _nonempty_string(pid):
                refusals.append(f"{prefix}.id")
            else:
                pinned_ids.append(pid)
            if outcome not in _PINNED_OUTCOMES:
                refusals.append(f"{prefix}.outcome")
            error_side = row.get("error_side")
            if error_side is not None and error_side not in {"oracle", "sut"}:
                refusals.append(f"{prefix}.error_side")
            oracle_unavailable = outcome in {
                "seed_missing",
                "unsupported_expected",
                "oracle_error",
                "oracle_timeout",
            }
            if (
                error_side != "oracle"
                and not oracle_unavailable
                and not _site_list(row.get("oracle_sites"))
            ):
                refusals.append(f"{prefix}.oracle_sites")
        if len(pinned_ids) != len(set(pinned_ids)):
            refusals.append("pinned: duplicate_ids")

    matrix = record.get("matrix")
    if isinstance(matrix, list):
        matrix_ids = []
        for index, row in enumerate(matrix):
            prefix = f"matrix.{index}"
            if not isinstance(row, dict):
                refusals.append(prefix)
                continue
            for field in ("id", "language", "capability"):
                if not _nonempty_string(row.get(field)):
                    refusals.append(f"{prefix}.{field}")
            if row.get("outcome") not in _MATRIX_OUTCOMES:
                refusals.append(f"{prefix}.outcome")
            if _nonempty_string(row.get("id")):
                matrix_ids.append(row["id"])
        if len(matrix_ids) != len(set(matrix_ids)):
            refusals.append("matrix: duplicate_ids")
    return sorted(set(refusals))


def record_refusals(record: dict) -> list[str]:
    """Compatibility wrapper; new consumers call validate_run_record directly."""
    return validate_run_record(record, purpose="admission")


def schema_covers_path(record: dict, path: tuple) -> bool:
    """Return whether an accessed record path is constrained by this schema."""
    if not path:
        return False
    required = set(required_record_leaf_paths(record))
    if any(
        path[: len(leaf)] == leaf or leaf[: len(path)] == path
        for leaf in required
    ):
        return True
    if path[0] in {"failures", "m1", "m2", "m3", "oracle_filtered", "pending"}:
        return len(path) == 1
    if path[0] == "site_fingerprints":
        return len(path) <= 2
    if path[0] == "pinned" and len(path) >= 3:
        return path[2] in {"id", "outcome", "error_side", "oracle_sites"}
    if path[0] == "matrix" and len(path) >= 3:
        return path[2] in {"id", "language", "capability", "name", "outcome"}
    return False
