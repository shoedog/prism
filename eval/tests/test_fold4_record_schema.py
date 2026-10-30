import copy

from tests.helpers import base_ids, fake_run


def _valid_run():
    run = fake_run(base_ids())
    run["meta"]["invalid_reasons"] = []
    return run


def _parent(record, path):
    value = record
    for part in path[:-1]:
        value = value[part]
    return value, path[-1]


def _wrong_type(value):
    if isinstance(value, bool):
        return "true"
    if isinstance(value, str):
        return 7
    if isinstance(value, int):
        return "7"
    if isinstance(value, float):
        return "0.0"
    if isinstance(value, list):
        return {}
    if isinstance(value, dict):
        return []
    return object()


def test_every_whole_record_schema_leaf_rejects_delete_null_and_wrong_type():
    from tier_a.record_schema import record_refusals, required_record_leaf_paths

    valid = _valid_run()
    assert record_refusals(valid) == []
    for path in required_record_leaf_paths(valid):
        parent, key = _parent(valid, path)
        original = parent[key]
        for mutation in ("delete", "null", "type"):
            changed = copy.deepcopy(valid)
            changed_parent, changed_key = _parent(changed, path)
            if mutation == "delete":
                del changed_parent[changed_key]
            elif mutation == "null":
                changed_parent[changed_key] = None
            else:
                changed_parent[changed_key] = _wrong_type(original)
            assert record_refusals(changed), (path, mutation)


def test_removing_all_per_probe_payloads_refuses_before_scoring():
    from tier_a.record_schema import record_refusals

    run = _valid_run()
    for pid, probe in run["probes"].items():
        if not pid.startswith("_"):
            for key in (
                "prism_sites",
                "oracle_sites",
                "prism_functions",
                "oracle_functions",
            ):
                probe.pop(key)
    assert any("probes." in reason for reason in record_refusals(run))


def test_invalid_reason_consistency_and_probe_status_counts_are_schema_rules():
    from tier_a.record_schema import record_refusals

    run = _valid_run()
    run["meta"]["baseline_invalid"] = True
    assert "meta.invalid_reasons: inconsistent" in record_refusals(run)
    run = _valid_run()
    run["probe_status"].pop(next(iter(run["probe_status"])))
    assert "probe_status: probe_set_mismatch" in record_refusals(run)
