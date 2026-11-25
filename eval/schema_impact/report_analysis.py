"""Compare the frozen inventories and assemble the live version-2 document."""

from collections import Counter, defaultdict
from datetime import datetime
from hashlib import sha256
import json

from .report_claims import Record, decide, reasons
from .report_inputs import effective_invocation
from .report_matches import GATEWAY, compare, parts, prefix_hints
from .report_proof import (declaration_changes, diff, equal_lines, explain, failed,
                           measured_operation_sides,
                           paired_functions, route_record, source_gap, source_record)
from .report_schema import compare_schema, schema_record
from .report_summary import request_summary, summarize


MISSING_DECLARATION = "Client calls a missing route"
RISK_ACTIONS = {
    MISSING_DECLARATION: "Check whether this client still needs the old declaration and which version it will use.",
    "Existing client call gains a matching declaration": "Check which service version this client needs.",
    "Client target needs confirmation": "Check the address value and which route takes priority.",
}
EARLIER = "Earlier match: Not checked — the earlier scan’s binding record is unavailable."
FIELD_USE = "Client field use: Not checked — the call was matched by method and path."
NO_FIELD_USE = "Client field use: Not checked — no recorded read or send links this call to the field."


def _fingerprint(value):
    return sha256(json.dumps(value, sort_keys=True, ensure_ascii=False).encode()).hexdigest()


def _file_status(snapshot):
    return {row["path"]: row["status"] for row in snapshot["file_status"]}


def _service_changes(frozen):
    before, after = frozen.before["service"], frozen.after["service"]
    old, new = before["operations"], after["operations"]
    inventories = _file_status(before), _file_status(after)
    files = {site["file"] for snapshot in (old, new) for operation in snapshot.values()
             for site in operation["sites"]}
    diffs = {}
    for file in files:
        _, _, blocks = diff(frozen.before_sources.get(file), frozen.after_sources.get(file))
        diffs[file] = blocks, equal_lines(blocks)
    fields, limits = compare_schema(before, after)
    changes, notices = {}, {}
    for identifier in sorted(old.keys() | new.keys() | fields.keys()):
        old_sites = old.get(identifier, {}).get("sites", [])
        new_sites = new.get(identifier, {}).get("sites", [])
        group = fields.get(identifier, {"fields": [], "status_additions": [], "status_removals": [],
                                        "response_root_pairs": [], "limits": [], "schema_sites": [None, None]})
        kinds, scoped, presence, failures, body_notices, proofs = declaration_changes(
            frozen, old_sites, new_sites, inventories, diffs)
        known = {(row["side"], row["file"], row["reason"]) for row in failures}
        failures.extend(row for proof in proofs.values() for row in proof["failures"]
                        if (row["side"], row["file"], row["reason"]) not in known)
        notices.update((row["file"], row) for row in body_notices)
        group["proof_failures"] = failures
        group["schema_limits"] = list(group["limits"])
        group["limits"].extend(map(explain, failures))
        group["claims"] = proofs
        group["presence_claims"] = {}
        for side, snapshot, value in zip(("before", "after"), (before, after), presence):
            relevant = [row for row in failures if row["side"] == side]
            proof = decide("Operation presence", {
                Record("declaration inventory"): True if value == "present" else
                (relevant or source_record("Operation presence", snapshot, side)),
                Record("route specification"): route_record(snapshot, side),
            })
            group["presence_claims"][side] = proof
        if group["fields"]:
            kinds.append("Operation fields changed")
        if group["status_removals"] or group["status_additions"]:
            kinds.append("Successful response statuses changed")
        if not kinds and not group["limits"]:
            continue
        changes[identifier] = {
            **group, "operation_id": identifier, "change_kinds": list(dict.fromkeys(kinds)),
            "before_presence": presence[0], "after_presence": presence[1],
            "producer_sites": {"before": old_sites, "after": new_sites},
            "file_changes": scoped, "before_shape": None, "after_shape": None,
            "call_changes": [], "risks": [], "review_action": None,
        }
    return changes, limits, list(notices.values())


def _client_changes(frozen):
    before, after = frozen.before["client"], frozen.after["client"]
    a, b = before["calls"], after["calls"]
    af, bf = _file_status(before), _file_status(after)
    old, new = defaultdict(list), defaultdict(list)
    for source, index in ((a, old), (b, new)):
        for row in source:
            index[row["site"]["file"]].append(row)
    paired, changes = [], []
    for file in sorted(old.keys() | new.keys()):
        left, right, opcodes = diff(frozen.before_sources.get(file), frozen.after_sources.get(file))
        left_counts, right_counts = Counter(left), Counter(right)
        failures = []
        for label, snapshot, statuses, sources in (
            ("before", before, af, frozen.before_sources), ("after", after, bf, frozen.after_sources),
        ):
            gap = source_record("Client call comparison", snapshot, label, file=file)
            if gap is True and statuses.get(file) != "complete" and file in sources:
                gap = failed("Client call comparison", file, snapshot["rev"],
                             "no complete client file inventory was recorded", side=label)
            if gap is not True:
                failures.append(gap)
        records = {Record("client inventory", label): [row for row in failures if row["side"] == label] or True
                   for label in ("before", "after")}
        complete = not failures
        old_lines = Counter(row["site"]["line"] for row in old[file])
        new_lines = Counter(row["site"]["line"] for row in new[file])
        mapped = equal_lines(opcodes)
        functions = paired_functions(before.get("functions", {}).get(file, []),
                                     after.get("functions", {}).get(file, []), mapped)
        by_line = {row["site"]["line"]: row for row in new[file]}
        handled = set()
        unpaired = {}
        for row in old[file]:
            loc = row["site"]
            line, function = loc["line"], row["enclosing_function"]
            other = by_line.get(mapped.get(line))
            correspondence = bool(other and function and function == other["enclosing_function"]
                    and row["function_span"] and other["function_span"]
                    and functions.get(tuple(row["function_span"])) == tuple(other["function_span"])
                    and loc["line"] == loc["end_line"] and other["site"]["line"] == other["site"]["end_line"]
                    and old_lines[line] == new_lines[other["site"]["line"]] == 1
                    and left_counts[left[line - 1]] == right_counts[left[line - 1]] == 1
                    and loc["text"] == other["site"]["text"])
            proof = decide("Corresponding call", records | {Record("call span diff"): bool(other),
                           Record("call correspondence"): correspondence})
            if correspondence and not proof["confirmed"]:
                for candidate in (row, other):
                    unpaired[candidate["site_id"]] = proof["failures"]
            if proof["confirmed"]:
                evidence = "one complete call on a uniquely mapped unchanged line inside paired function declarations"
                handled.update((row["site_id"], other["site_id"]))
                paired.append((row, other, evidence))
                if _target(row) != _target(other):
                    target = decide("Client target changed", records | {Record("call span diff"): True,
                                    Record("call correspondence"): correspondence})
                    if target["confirmed"]:
                        changes.append(_change(row, other, target["kind"], evidence=evidence, proof=target))
                    else:
                        changes.extend([
                            _change(row, None, "Observed before", target["failures"], proof=target),
                            _change(None, other, "Observed after", target["failures"], proof=target),
                        ])
        for side, rows in enumerate((old[file], new[file])):
            for row in rows:
                loc = row["site"]
                if row["site_id"] in handled:
                    continue
                if (file not in frozen.changed_paths and not complete and
                        frozen.before_sources.get(file) == frozen.after_sources.get(file)):
                    continue
                lines, counterpart = (left, right) if side == 0 else (right, left)
                only = old_lines if side == 0 else new_lines
                diffs = [tag for tag, *bounds in opcodes
                         for lo, hi in [bounds[side * 2:side * 2 + 2]]
                         if lo <= loc["line"] - 1 and loc["end_line"] - 1 < hi]
                expected = "delete" if side == 0 else "insert"
                others = new[file] if side == 0 else old[file]
                unchanged_file = file not in frozen.changed_paths and not failures
                if unchanged_file and any(
                    r["site"]["byte_span"] == loc["byte_span"] and _target(r) == _target(row) for r in others
                ):
                    continue
                moved_or_copied = any(
                    source and loc["text"] in source for source in counterpart_sources(frozen, file, side)
                )
                kind = "Client call removed" if side == 0 else "Client call added"
                span = (diffs == [expected] and not moved_or_copied
                        and only[loc["line"]] == 1 and lines[loc["line"] - 1] not in counterpart)
                proof = decide(kind, records | {Record("call span diff"): span or failed(
                    "Corresponding call", file, (before, after)[side]["rev"],
                    "the call span is not a unique insertion or deletion and no unique call and function correspondence was proved",
                    side=("before", "after")[side], line=loc["line"])})
                if proof["confirmed"]:
                    selected_failures = []
                else:
                    kind = "Observed before" if side == 0 else "Observed after"
                    selected_failures = failures or unpaired.get(row["site_id"]) or proof["failures"]
                changes.append(_change(row if side == 0 else None, row if side == 1 else None, kind,
                                       selected_failures, proof=proof))
    return changes, paired


def counterpart_sources(frozen, file, side):
    sources = frozen.after_sources if side == 0 else frozen.before_sources
    return (sources.get(path) for path in set(frozen.changed_paths) | {file})


def _target(row):
    return row["requested_method"], row["mapped_path"], row["target_reason"]


def _change(old, new, kind, failures=(), evidence=None, proof=None):
    reason = " ".join(map(explain, failures)) or None
    return {"before_site_id": old["site_id"] if old else None, "after_site_id": new["site_id"] if new else None,
            "kind": kind, "reason": reason, "proof_failure": failures[0] if len(failures) == 1 else None,
            "proof_failures": list(failures), "claim": proof, "correspondence_evidence": evidence,
            "before_presence": "present" if old else "unknown" if reason else "absent",
            "after_presence": "present" if new else "unknown" if reason else "absent"}


def _service_risks(group, client_before, client_after, before, after, request_counts):
    identifier = group["operation_id"]
    kinds = group["change_kinds"]
    relevance = set()
    group["prefix_hints"], group["declaration_comparisons"], group["client_site_ids"] = [], [], []
    if not kinds:
        return
    for pair in (before, after):
        relevance.update(key for key, result in pair["matches"].items() if identifier in result["target_ids"])
    group["client_site_ids"] = sorted(relevance)

    def add_risk(kind, key):
        checked = [pair["matches"].get(key, {}).get("claim", {}).get("confirmed", False)
                   for pair in (before, after)]
        proof = decide(kind, {
            Record("request comparison", "before"): checked[0],
            Record("request comparison", "after"): checked[1],
            Record("request comparison"): any(checked),
            Record("deployment pair"): before["client_input_id"] == after["client_input_id"],
        })
        if proof["confirmed"]:
            group["risks"].append({"kind": kind, "site_id": key, "claim": proof,
                                   "comparison_purposes": [before["purpose"], after["purpose"]]})
        else:
            group["limits"].append(reasons(proof))
    for side, client, pair, observed in (
        ("before", client_before, before, request_counts[before["purpose"]]),
        ("after", client_after, after, request_counts[after["purpose"]]),
    ):
        marker = "removed" if side == "before" else "added"
        listed = any(kind in kinds for kind in (f"Operation {marker}",
                     f"Listed only {side} among the declarations read"))
        if not listed:
            continue
        hints = prefix_hints(identifier, client)
        group["prefix_hints"].extend(dict(hint, side=side) for hint in hints if hint["claim"]["confirmed"])
        group["limits"].extend(reasons(hint["claim"]) for hint in hints if not hint["claim"]["confirmed"])
        group["declaration_comparisons"].append({
            "side": side, "purpose": pair["purpose"], "compared": len(client["calls"]),
            "inventory_reason": source_gap("Client request inventory", client),
            "closest_site_ids": [key for key, result in pair["matches"].items() if identifier in result["target_ids"]],
            **observed,
        })
    if "Operation removed" in kinds:
        for key in sorted(relevance & before["matches"].keys() & after["matches"].keys()):
            lost, remains = before["matches"][key], after["matches"][key]
            if identifier in lost["target_ids"] and lost["status"] == "match":
                if remains["status"] == "no_match":
                    add_risk(MISSING_DECLARATION, key)
                elif remains["status"] == "match" and identifier not in remains["target_ids"]:
                    add_risk("Client target needs confirmation", key)
    if "Operation added" in kinds:
        for key in sorted(relevance & before["matches"].keys() & after["matches"].keys()):
            old, new = before["matches"][key], after["matches"][key]
            if old["status"] == "no_match" and new["status"] == "match" and identifier in new["target_ids"]:
                add_risk("Existing client call gains a matching declaration", key)
    if not group["risks"] and measured_operation_sides(group):
        keys = next((key for key in sorted(relevance) if any(pair["matches"].get(key, {}).get("status") == "possible"
                                                            for pair in (before, after))), None)
        if keys is not None:
            add_risk("Client target needs confirmation", keys)


def _action(group):
    for kind, text in RISK_ACTIONS.items():
        if any(risk["kind"] == kind for risk in group["risks"]):
            return text
    if group["fields"]:
        return "Check what the client sends and how it reads the changed fields."
    if group["status_additions"] or group["status_removals"]:
        return "Check clients that branch on the response status."
    kinds = group["change_kinds"]
    if "Operation added" in kinds:
        return "Check who needs this addition."
    if "Operation removed" in kinds:
        return "Check who still uses this declaration."
    if any(kind.startswith("Listed only") for kind in kinds):
        return ("Confirm the removal, then check whether this client still needs it." if group["before_presence"] == "present"
                else "Confirm the addition, then check the intended client.")
    return "Check whether this edit changes what requests the handler accepts or returns."


def _client_action(change, calls, before, after):
    a, b = change["before_site_id"], change["after_site_id"]
    old, new = before["matches"].get(a), after["matches"].get(b)
    target = calls.get(b) or calls[a]
    change["operation_ids"] = sorted({
        identifier for match in (old, new) if match for identifier in match["target_ids"]
    })
    change["risk"] = None
    if new and new["status"] == "no_match":
        change["risk"] = MISSING_DECLARATION
    elif new and (new["status"] == "possible" or old and old["target_ids"] != new["target_ids"]):
        change["risk"] = "Client target needs confirmation"
    change["requested_target"] = _target(target)[:2]
    change["remaining_after_site_ids"] = [
        key for key, row in after["matches"].items()
        if change["kind"] == "Client call removed" and set(row["target_ids"]) & set(change["operation_ids"])
    ]
    if change["kind"] == "Client call removed":
        action = "Check which clients still need the old declaration."
    elif change["kind"] == "Observed before":
        action = "Check whether this call has a corresponding occurrence after the edit."
    else:
        action = "Check which service version this call needs."
    change["review_action"] = action
    return change


def _standing(before, after, shared, ops, snapshots, client_changes):
    changed_sites = {row[key] for row in client_changes for key in ("before_site_id", "after_site_id") if row[key]}
    changed_sites.update(key for group in ops for key in group["client_site_ids"])
    result, limits = [], []
    proved_absent = {row["before_site_id"] or row["after_site_id"] for row in client_changes
                     if "absent" in (row["before_presence"], row["after_presence"])}
    for a, b in shared:
        old, new = before["matches"].get(a), after["matches"].get(b)
        if not any(match and match["status"] == "no_match" for match in (old, new)):
            continue
        timing = "Already present" if all(match and match["status"] == "no_match" for match in (old, new)) else None
        if timing is None and old and new and {old["status"], new["status"]} == {"match", "no_match"}:
            timing = "Appears with this change" if new["status"] == "no_match" else "Clears with this change"
        seen = [match["service_rev"][:8] + " with client " + match["client_rev"][:8]
                for match in (old, new) if match and match["status"] == "no_match"]
        gaps = []
        if timing is None:
            for label, match in (("before", old), ("after", new)):
                if match is None:
                    absent = (a or b) in proved_absent
                    gaps.append(failed("Timing", None, None, "there is no corresponding request at this version"
                                       if absent else "request correspondence is unproved at this version", side=label))
                elif match["status"] not in {"match", "no_match"}:
                    gap = match.get("proof_failure") or failed(
                        "Timing", None, match["service_rev"],
                        f"the comparison recorded {match['status'].replace('_', ' ')}",
                    )
                    gaps.append(dict(gap, side=label))
        proof = decide("Standing difference timing", {
            Record("request comparison", label): (
                True if match and match["status"] in {"match", "no_match"} else
                [gap for gap in gaps if gap["side"] == label])
            for label, match in (("before", old), ("after", new))
        } | {Record("call correspondence"): bool(a and b)})
        result.append({"kind": "request has no same-method match among the declarations read",
                       "before_site_id": a, "after_site_id": b, "operation_id": None,
                       "linked_to_change": a in changed_sites or b in changed_sites,
                       "timing": timing if timing and proof["confirmed"] else f"Seen at {' and '.join(seen)}.",
                       "timing_failures": gaps or proof["failures"], "versions": seen, "claim": proof})
    for snapshot in snapshots:
        document = snapshot["schema"]
        evidence = schema_record(document, None)
        if evidence is True and document["path_gaps"]:
            evidence = failed("Standing schema difference", document["source"]["path"], snapshot["rev"],
                              "the schema path item inventory is incomplete")
        proof = decide("Standing schema difference", {
            Record("schema inventory"): evidence,
            Record("declaration inventory"): source_record("Standing schema difference", snapshot),
            Record("route specification"): route_record(snapshot),
        })
        if not proof["confirmed"]:
            if document["status"] == "complete":
                limits.append(proof)
            continue
        for op in sorted(document["operations"].keys() ^ snapshot["operations"].keys()):
            origin = "The committed schema" if op in document["operations"] else snapshot["operation_authority"]
            result.append({"kind": f"{origin} lists this operation.", "registration_reason": GATEWAY,
                           "before_site_id": None, "after_site_id": None, "operation_id": op,
                           "linked_to_change": any(op == group["operation_id"] for group in ops),
                           "timing": f"Seen at {snapshot['rev'][:8]}.",
                           "timing_failures": [failed("Timing", snapshot["schema"]["source"]["path"], snapshot["rev"],
                                                      "no complete registration comparison was available")],
                           "versions": [snapshot["rev"]], "claim": proof})
    return result, limits


def build_document(frozen):
    b, a = frozen.before, frozen.after
    bs, bc, aps, ac = b["service"], b["client"], a["service"], a["client"]
    subject = a["commit"]
    requested = frozen.request.role
    schema_path = bs["schema"]["source"]["path"]
    service_sources_changed = any(
        row["path"] in frozen.changed_paths for snap in (bs, aps) for row in snap["file_status"]
    )
    service_changed = service_sources_changed or schema_path in frozen.changed_paths
    client_changed = any(row["path"] in frozen.changed_paths and row["status"] != "skipped"
                         for snap in (bc, ac) for row in snap["file_status"])
    client_changed |= any(dep["path"] in frozen.changed_paths for snap in (bc, ac)
                          for dep in snap["dependencies"])
    client_changed |= [row["revision"] for row in bc["config_sources"]] != [
        row["revision"] for row in ac["config_sources"]]
    mixed = frozen.reference is None and service_changed and client_changed
    infer = frozen.reference is None and (service_changed or client_changed)
    show_service = service_changed if infer else requested == "producer"
    show_client = client_changed if infer else requested == "consumer"
    service_after, client_after = (aps if show_service else bs), (ac if show_client else bc)
    pairs = [compare(bs, bc, "before"), compare(service_after, client_after, "after")]
    service_pairs = pairs
    if mixed:
        pairs.extend([compare(aps, bc, "service_first"), compare(bs, ac, "client_first")])
        service_pairs = [pairs[0], pairs[2]]
    snapshots = {snap["input_id"]: snap for snap in (bs, bc, service_after, client_after)}
    request_counts = {pair["purpose"]: request_summary(pair, snapshots[pair["client_input_id"]],
                                                snapshots[pair["service_input_id"]]) for pair in pairs}
    service_changes, field_limits, notices = _service_changes(frozen) if show_service else ({}, [], [])
    if not show_service:
        if bs["snapshot_kind"] == "unavailable":
            field_limits = [bs["schema"]["reason"]]
        else:
            fixed_schema, field_limits = compare_schema(bs, bs, fixed=True)
            field_limits.extend(limit for group in fixed_schema.values() for limit in group["limits"])
    operations = list(service_changes.values())
    calls = {row["site_id"]: row for snap in (bc, client_after) for row in snap["calls"]}
    for group in operations:
        _service_risks(group, bc, bc, *service_pairs, request_counts)
        if mixed and group["change_kinds"]:
            _, combined = pairs[:2]
            relevant = {key for key, match in combined["matches"].items()
                        if group["operation_id"] in match["target_ids"]}
            group["client_site_ids"] = sorted(set(group["client_site_ids"]) | relevant)
        group["review_action"] = {"text": _action(group), "supporting_row_ids": [
            group["operation_id"], *(risk["site_id"] for risk in group["risks"])
        ]}
    client_changes, paired = _client_changes(frozen) if show_client else ([], [])
    client_changes = [_client_action(row, calls, *pairs[:2]) for row in client_changes]
    operations.sort(key=lambda group: (
        4 if not group["change_kinds"] else
        0 if any(r["kind"] == MISSING_DECLARATION for r in group["risks"]) else
        1 if group["fields"] or group["status_additions"] or group["status_removals"] else
        2 if group["risks"] else 3,
        parts(group["operation_id"])[1], parts(group["operation_id"])[0]))
    wanted = {key for group in operations for key in group["client_site_ids"]} | {
        row[key] for row in client_changes for key in ("before_site_id", "after_site_id") if row[key]
    } | {hint["site_id"] for group in operations for hint in group["prefix_hints"]}
    shared = [(x["site_id"], y["site_id"], evidence) for x, y, evidence in paired]
    if not show_client:
        shared = [(key, key, "same frozen client occurrence") for key in calls]
    join = {key: pair for pair in shared for key in pair[:2]}
    client_proofs = {edit[side]: edit for edit in client_changes for side in ("before_site_id", "after_site_id")
                     if edit[side]}
    usages = []
    for key in sorted(wanted):
        lhs, rhs, evidence = join.get(key, (
            key if key in pairs[0]["matches"] else None, key if key in pairs[1]["matches"] else None, None
        ))
        if rhs == key and lhs and lhs != key and lhs in wanted:
            continue
        x, y = calls.get(lhs), calls.get(rhs)
        before_match, after_match = pairs[0]["matches"].get(lhs), pairs[1]["matches"].get(rhs)
        matched = any(row and row["status"] in {"match", "possible"} for row in (before_match, after_match))
        edit = client_proofs.get(key)
        presence_failure = (
            failed("Corresponding call", (x or y)["site"]["file"], (x or y)["site"]["rev"],
                   "no corresponding call was proved at the other version") if not edit and not (lhs and rhs) else None
        )
        usages.append({"site_id": key, "before_site_id": lhs, "after_site_id": rhs,
                       "before_presence": edit["before_presence"] if edit else "present" if lhs else "unknown",
                       "after_presence": edit["after_presence"] if edit else "present" if rhs else "unknown",
                       "presence_reason": edit["reason"] if edit else explain(presence_failure) if presence_failure else None,
                       "proof_failures": edit["proof_failures"] if edit else [presence_failure] if presence_failure else [],
                       "before_site": x["site"] if x else None,
                       "after_site": y["site"] if y else None,
                       "usage_id": f"{lhs}→{rhs}" if evidence else None, "correspondence_evidence": evidence,
                       "requested_target": {"before": _target(x) if x else None, "after": _target(y) if y else None},
                       "match_before": before_match, "match_after": after_match,
                       "field_use": {"status": "not_checked", "reason": FIELD_USE if matched else NO_FIELD_USE},
                       "caller_evidence":
                       (y or x)["callers"], "prefix_hints": [
                           hint for group in operations for hint in group["prefix_hints"] if hint["site_id"] in (lhs, rhs)
                       ]})
    inputs = snapshots
    warnings = [{"input_id": snap["input_id"], "rev": snap["rev"], **gap}
                for snap in inputs.values() for gap in snap["unaccounted_scopes"]]
    shared_keys = [(x, y) for x, y, _ in shared]
    shared_keys.extend((row["before_site_id"], row["after_site_id"]) for row in client_changes
                       if not row["correspondence_evidence"])
    version = {"before": b["commit"]["rev"], "after": subject["rev"], "requested_rev": subject["rev"],
               "subject": subject, "base": b["commit"],
               "subject_role": "both" if mixed else "service" if show_service else "client",
               "requested_role": requested, "before_input_id": {"service": bs["input_id"], "client": bc["input_id"]},
               "after_input_id": {"service": service_after["input_id"], "client": client_after["input_id"]},
               "changed_paths": frozen.changed_paths, "target": None, "merge_result": None, "merge_receipt": None}
    ref = frozen.reference
    if ref is None:
        reference = bc if show_service else bs
        delta = datetime.fromisoformat(subject["committed_at"]) - datetime.fromisoformat(reference["committed_at"])
        ref = {"reference_input_id": reference["input_id"], "selected_by": "first parent in the same repository",
               "selection_reason": "the counterpart is held at the base; joint changes also check the after tree",
               "availability_status": "selected", "availability_evidence": {"tree": reference["tree"]},
               "duration_seconds": -int(delta.total_seconds()), "branch": None, "observed_tip": None,
               "observed_at": None, "selection_history_tip": None}
    for group in operations:
        group["call_changes"] = [row for row in client_changes if group["operation_id"] in row["operation_ids"]]
    elements = [{"element_id": group["operation_id"], "change_kinds": group["change_kinds"]}
                for group in operations if measured_operation_sides(group)]
    elements.extend({"element_id": (field["after"] or field["before"])["field_id"],
                     "before_element_id": field["before"]["field_id"] if field["before"] else None,
                     "after_element_id": field["after"]["field_id"] if field["after"] else None}
                    for group in operations for field in group["fields"])
    version["elements"] = elements
    unobserved_reason = None if not bs["unaccounted_scopes"] and not service_after["unaccounted_scopes"] and all(
        row["element_id"].startswith("op:") and (row["element_id"] in bs["operations"]
                                                or row["element_id"] in service_after["operations"])
        for row in elements
    ) else "Not checked — the binding inventory is incomplete"
    summary = summarize(operations, field_limits, [bs, service_after], [bc, client_after], usages, pairs, client_changes)
    summary.update({name: {purpose: counts[name] for purpose, counts in request_counts.items()}
                    for name in ("known_unmatched_count", "unknown_target_count")})
    schema_unchanged = (frozen.reference is None or show_service) and service_sources_changed and (
        bool(bs["sha256"]) and bs["sha256"] == aps["sha256"]
    )
    manifest = [{key: snap[key] for key in ("input_id", "repository_id", "tree", "source_url", "scope",
                                            "operation_authority", "config_sources")} |
                {"schema": {key: snap["schema"][key] for key in ("source", "status", "sha256")}
                 if snap["schema"] else None, "driver": snap["tool"]}
                for snap in inputs.values()]
    invocation = effective_invocation(frozen.request)
    standing, standing_limits = _standing(*pairs[:2], shared_keys, operations,
                                          (bs,) if bs == service_after else (bs, service_after), client_changes)
    return {
        "schema_version": "2", "report_id": _fingerprint([
            version, ref, frozen.request.mode, frozen.request.selection_time, invocation, manifest]),
        "api": bs["api"], "mode": frozen.request.mode, "report_time": frozen.request.selection_time,
        "input_manifest": manifest, "study_invocation": invocation,
        "cutoff_at": frozen.request.selection_time, "inputs": inputs, "reference": ref, "comparisons": pairs,
        "change": version, "impacts": operations, "call_changes": client_changes, "usages": usages,
        "binding_status": {"status": "not_checked", "reason": EARLIER},
        "coverage": [{"input_id": key, "scope": snap["scope"],
                      "status": ("not_checked" if snap["snapshot_kind"] == "unavailable" else
                                 "partial" if snap["unaccounted_scopes"] else "complete"),
                      "reason": snap["unaccounted_scopes"][0]["reason"] if snap["unaccounted_scopes"] else None}
                     for key, snap in inputs.items()],
        "warnings": warnings, "field_limits": field_limits, "body_notices": notices,
        "code_changed_with_unchanged_schema": schema_unchanged,
        "standing_mismatches": standing, "standing_limits": standing_limits,
        "summary": summary, "request_counts": request_counts,
        "unobserved": None if unobserved_reason else [], "unobserved_reason": unobserved_reason,
    }
