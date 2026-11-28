"""Admit oracle output under the gate's run, checker and certificate bindings."""

from __future__ import annotations

from collections import defaultdict

from . import gate_file, inputs, receipt
from .oracle_inputs import RESULTS, name, shape
from .oracle_selection import TARGET_CONFIDENCES


def check_counts(tuples, denominators, calls):
    scopes = denominators["scopes"]
    gate_file.require(isinstance(scopes, list), "invalid oracle scopes")
    ids = set()
    grouped = defaultdict(lambda: dict.fromkeys(RESULTS, 0))
    for row in scopes:
        gate_file.require(shape(row, ("relation", "property", "confidence", "kind", "graded", *RESULTS))
                          and name(row["relation"]) and name(row["property"])
                          and row["confidence"] in (TARGET_CONFIDENCES if calls else ("exact", "nameonly"))
                          and (row["kind"] is None or calls and name(row["kind"]))
                          and all(type(row[k]) is int and row[k] >= 0 for k in ("graded", *RESULTS))
                          and row["graded"] == sum(row[k] for k in RESULTS), "invalid oracle scope counts")
        identity = tuple(row[k] for k in ("relation", "property", "confidence", "kind"))
        gate_file.require(identity not in ids, "duplicate oracle scope")
        ids.add(identity)
        for label in RESULTS:
            grouped[identity[:2]][label] += row[label]
    for relation, properties in tuples.items():
        gate_file.require(name(relation) and isinstance(properties, dict), "invalid oracle tuple relation")
        for prop, counts in properties.items():
            gate_file.require(name(prop) and shape(counts, RESULTS)
                              and all(type(count) is int for count in counts.values())
                              and counts == grouped.pop((relation, prop), dict.fromkeys(RESULTS, 0)),
                              "oracle tuple counts differ from scopes")
    gate_file.require(not grouped, "oracle tuples omit graded scopes")
    totals = {key.lower(): sum(row[key] for row in scopes) for key in RESULTS}
    counts = dict(correct=totals["correct"], wrong=totals["wrong"], inadmissible=totals["inadmissible"],
                  decided=totals["correct"] + totals["wrong"], graded=sum(totals.values()))
    gate_file.require(all(type(denominators[key]) is int and denominators[key] == value
                          for key, value in counts.items()), "oracle denominators differ from scopes")


def read_oracle(decision, step, directory, runs, out):
    ident = step["id"]
    record = inputs.json_object(gate_file.regular(directory / "oracle-receipt.json"))
    gate_file.require(shape(record, ("schema", "inputs", "selfcheck_key", "mode", "relation", "selection",
                                     "qualification_sha256", "gate_sha256", "gate_step_id", "artifacts"))
                      and record["schema"] == "prism.measure.oracle/1"
                      and (record["gate_sha256"], record["gate_step_id"]) == (decision.sha256, ident),
                      "invalid oracle gate receipt")
    prepared = decision.paths[f"{ident}/oracle_inputs"]
    gate_file.require(record["selfcheck_key"] == prepared["key"] and record["relation"] == step.get("relation")
                      and record["qualification_sha256"] == prepared["qualification_sha256"],
                      "oracle input binding differs")
    calls = step["do"] == "oracle ts-calls"
    selected = "census" if "census" in step or step.get("all") else "head"
    changes = "changes" in step
    binding = record["inputs"]
    gate_file.require(shape(binding, ("runs", "qualification", "runtime", "kit_commit"),
                            ("changes_sha256", "certificate_sha256", "certificate_result")),
                      "invalid oracle input receipt")
    if changes:
        comparison = decision.by_id[step["changes"]]
        selected = step.get("side") or next(arm for arm in ("base", "head") if comparison[arm] == step["run"])
        predecessor = out / step["changes"]
        gate_file.require(binding.get("certificate_sha256") == inputs.sha256_file(predecessor / "certificate.json")
                          and binding.get("changes_sha256") == inputs.sha256_file(predecessor / "changes.jsonl")
                          and binding.get("certificate_result") == "pass", "oracle certificate binding differs")
        refs = {arm: comparison[arm] for arm in ("base", "head")}
        mode = "edge-changes"
    elif calls and "census" not in step:
        refs = {arm: step[arm] for arm in ("base", "head")}
        mode = "call-diff"
    else:
        refs = {"census": step["census" if calls else "run"]}
        mode = "call-census" if calls else "edge-census"
    gate_file.require(binding["runs"] == [dict(arm=arm, receipt_sha256=runs[ref][2])
                                          for arm, ref in refs.items()], "oracle run binding differs")
    runtime = binding["runtime"]
    gate_file.require(shape(runtime, ("node", "python")) and all(map(name, runtime.values())),
                      "invalid oracle runtime")
    selection = {"arm": selected}
    if calls:
        selection.update(target_confidence=list(dict.fromkeys(step.get("target_confidence", ["exact"]))),
                         target_kind=list(dict.fromkeys(step.get("target_kind", []))),
                         aggregate_only=step.get("aggregate_only", False))
    else:
        selection.update(kind=list(dict.fromkeys(step.get("kind", []))),
                         **{"class": list(dict.fromkeys(step.get("class", [])))})
    gate_file.require(record["selection"] == selection and record["mode"] == mode,
                      "oracle measurement selection differs")
    required = {"tuples.json", "denominators.json"}
    if not step.get("aggregate_only"):
        required.add("verdicts.jsonl")
        if mode == "call-diff":
            required.add("deltas.jsonl")
    artifacts = record["artifacts"]
    gate_file.require(isinstance(artifacts, dict) and artifacts.keys() == required,
                      "oracle receipt lacks required outputs")
    for filename, digest in artifacts.items():
        path = directory / filename
        gate_file.require(receipt.is_digest(digest) and not path.is_symlink() and path.is_file()
                          and inputs.sha256_file(path) == digest, f"oracle artifact differs: {filename}")
    tuples, denominators = (inputs.json_object(gate_file.regular(directory / filename))
                            for filename in ("tuples.json", "denominators.json"))
    gate_file.require(denominators["mode"] == mode, "oracle denominators mode differs")
    check_counts(tuples, denominators, calls)
    return dict(tuples=tuples, denominators=denominators), runtime
