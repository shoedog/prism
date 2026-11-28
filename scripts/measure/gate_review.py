"""Bind review execution and replay to the gate's registered population and arms."""

from __future__ import annotations

import os

from . import gate_file, inputs, population, receipt, review_diff, review_receipt
from .review_findings import decode


def configure(decision, step, context):
    shared, arguments = gate_file.review_arguments(step)
    specifications = [(arm["name"], decision.binary_paths[arm["binary"]], args)
                      for arm, args in zip(step["arms"], arguments)]
    pins = {role: decision.binaries[arm["binary"]]["expect_version"]
            for role, arm in zip(review_receipt.ROLES, step["arms"])}
    context.update(review_plan=(specifications, shared), version_pins=pins,
                   environment={"cleared": sorted(key for key in os.environ if key.startswith("PRISM_")),
                                "product": {**{key: os.environ[key] for key in receipt.HOST_ENV
                                              if key in os.environ}, "LC_ALL": "C"}})


def actual_rows(decision, step, directory, context, child_exit):
    ident = step["id"]
    registered = decision.paths[f"{ident}/population"]
    record = review_receipt.admit(directory, population=registered)
    gate_file.require((record["gate_sha256"], record["gate_step_id"]) == (decision.sha256, ident)
                      and record["population_sha256"] == decision.files[f"{ident}/population"],
                      "review gate or population binding differs")
    gate_file.require(context.get("review_guard") == (
        decision.files[f"{ident}/memguard"], decision.files[f"{ident}/quiet_helper"]),
        "review guard or quiet helper binding differs")
    shared, arguments = gate_file.review_arguments(step)
    for role, arm, argv in zip(review_receipt.ROLES, step["arms"], arguments):
        observed = record["arms"][role]
        binary = observed["binary"]
        bound = decision.binaries[arm["binary"]]
        gate_file.require(binary["sha256"] == bound["sha256"] and observed["name"] == arm["name"]
                          and observed["arm_args"] == argv and observed["shared_args"] == shared,
                          f"review {role} arm binding differs")
        if binary["version"] is not None:
            gate_file.require(receipt.matches_version(binary["version"], bound["expect_version"]),
                              f"review {role} version pin mismatch")
    rows = list(inputs.jsonl_objects(directory / "per-diff.jsonl"))
    totals = inputs.json_object((directory / "totals.json").read_bytes())
    pins = population.load(registered).diffs
    selected = set(step.get("only", [pin["ordinal"] for pin in pins]))
    gate_file.require(totals["seed"] == step.get("seed", review_diff.DEFAULT_SEED)
                      and totals["sample_size"] == min(totals["ok"], step.get("sample", review_diff.DEFAULT_SAMPLE)),
                      "review sample does not match the gate")
    clones = decision.path.parent / step["clones"]
    quiet = False
    successful = []
    pinned_trees = {}
    for row, pin in zip(rows, pins):
        ordinal = pin["ordinal"]
        arms = [row["arms"][arm["name"]] for arm in step["arms"]]
        status, reason = row["status"], row["reason"]
        guards = [arm["guard_status"] for arm in arms if arm["guard_status"] is not None]
        events = [guard for guard in guards if guard["kind"] == "host_quiet"]
        if ordinal not in selected:
            gate_file.require(status == "excluded" and reason == "not-selected" and
                              all(arm["status"] == "unlaunched" for arm in arms),
                              "review recorded an unselected ordinal")
            continue
        gate_file.require(status != "excluded", "review omitted a selected ordinal")
        if quiet:
            gate_file.require((status, reason) == ("refused", review_diff.QUIET)
                              and all(arm["status"] == "unlaunched" for arm in arms),
                              "review continued after host quiet")
            continue
        if events or reason == review_diff.QUIET:
            gate_file.require(len(events) == 1 and (status, reason) == ("refused", review_diff.QUIET),
                              "invalid review host-quiet evidence")
            quiet = True
        elif status == "refused":
            if all(arm["status"] == "unlaunched" for arm in arms):
                gate_file.require(context.get("review_prelaunch", {}).get(ordinal) == reason,
                                  "review pre-launch refusal differs")
            else:
                gate_file.require(reason in (review_diff.INPUT_DRIFT, "review-version-pin-mismatch")
                                  or reason.startswith("review-executable-changed: "),
                                  "invalid review arm refusal")
            gate_file.require(not any(guard["kind"] == "guard_error" for guard in guards),
                              "review guard failure reported as refusal")
        if arms[1]["output"] is not None:
            base = arms[0]
            gate_file.require(base["status"] == "ok" and base["output"] is not None
                              and base["input_before_sha256"] == arms[1]["input_before_sha256"],
                              "review head launch has no corresponding base input")
            decode((directory / base["output"]).read_bytes())

        before = [arm["input_before_sha256"] for arm in arms
                  if arm["output"] is not None or arm["status"] == "ok"]
        if before:
            identity = tuple(pin[field] for field in ("repo", "commit", "parent", "tree", "spec_sha256",
                                                       "n_files", "n_lines"))
            if identity not in pinned_trees:
                source = (clones / pin["repo"]).resolve()
                population.regenerate_spec(source, pin)
                pinned_trees[identity] = inputs.git_tree_digest(source, pin["commit"], ".")
            gate_file.require(all(value == pinned_trees[identity] for value in before),
                              "review row tree or spec identity differs from the population")
        if status == "ok":
            successful.append(pin)
    gate_file.require(quiet == (child_exit == 75) or context.get("pin_refused"),
                      "review exit disagrees with its host-quiet evidence")
    totals.update(distinct_diffs=len({pin["spec_sha256"] for pin in successful}),
                  eligible_ordinals=[pin["ordinal"] for pin in successful if pin["n_lines"] > 0],
                  dispositions=[dict(ordinal=row["ordinal"], status=row["status"], reason=row["reason"])
                                for row in rows if row["status"] != "ok"])
    return totals
