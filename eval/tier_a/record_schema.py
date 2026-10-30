"""Single whole-record schema used by admission, readmission, and comparison."""
from __future__ import annotations

import re

from .launch import PROTOCOL
from .policy import ORACLE_ANSWER_STATUSES, PROBE_STATUS_VOCABULARY


_SHA_PATTERN = re.compile(r"[0-9a-f]{40}")
_DIGEST_PATTERN = re.compile(r"sha256:[0-9a-f]{64}")


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
        and len(site) >= 3
        and _nonempty_string(site[0])
        and _nonnegative_int(site[1])
        and _nonnegative_int(site[2])
        for site in value
    )


def _function_list(value) -> bool:
    return isinstance(value, list) and all(isinstance(row, list) for row in value)


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
            "launch": {
                "isolated": _true,
                "prefix_fresh": _true,
                "protocol": lambda value: value == PROTOCOL,
            },
        },
        "probes": _mapping,
        "probe_status": _mapping,
        "site_fingerprints": _mapping,
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
                    for field in ("eligible_symbols", "eligible_probes", "target")
                )
        for pid in sorted(key for key in probes if not key.startswith("_")):
            status = statuses.get(pid) if isinstance(statuses, dict) else None
            paths.extend(
                ("probes", pid, field) for field in _probe_schema(status)
            )
            paths.append(("probe_status", pid))
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


def record_refusals(record: dict) -> list[str]:
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
    strata = probes.get("_strata")
    if not _nonempty_string(probes.get("_corpus")):
        refusals.append("probes._corpus")
    if not isinstance(strata, dict):
        refusals.append("probes._strata")
    else:
        stratum_schema = {
            "eligible_symbols": _nonnegative_int,
            "eligible_probes": _nonnegative_int,
            "target": _nonnegative_int,
        }
        for stratum, row in strata.items():
            for path in _invalid_schema_paths(
                row, stratum_schema, ("probes", "_strata", stratum)
            ):
                refusals.append(".".join(map(str, path)))
    return sorted(set(refusals))
