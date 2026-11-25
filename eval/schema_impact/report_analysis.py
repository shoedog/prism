"""Compare the frozen inventories and assemble the live version-2 document."""

from collections import Counter, defaultdict
from datetime import datetime
from difflib import SequenceMatcher
from hashlib import sha256
import json

from .report_matches import GATEWAY, OVERLAPS, WILDCARD, compare, parts, prefix_hints
from .report_schema import compare_schema


CORRESPONDENCE = "Corresponding call: Not checked — the scan recorded line numbers only."
EARLIER = "Earlier match: Not checked — the earlier scan’s binding record is unavailable."
FIELD_USE = "Client field use: Not checked — the call was matched by method and path."
NO_FIELD_USE = "Client field use: Not checked — no recorded read or send links this call to the field."


def _fingerprint(value):
    return sha256(json.dumps(value, sort_keys=True, ensure_ascii=False).encode()).hexdigest()


def _file_status(snapshot):
    return {row["path"]: row["status"] for row in snapshot["file_status"]}


def _diff(before, after):
    a, b = (before or "").splitlines(), (after or "").splitlines()
    return a, b, list(SequenceMatcher(None, a, b, autojunk=False).get_opcodes())


def _changed_line(opcodes, line, end, side):
    return any(tag != "equal" and all(lo <= pos < hi for pos in range(line - 1, end))
               for tag, *bounds in opcodes for lo, hi in [bounds[side:side + 2]])


def _service_changes(frozen):
    before, after = frozen.before["service"], frozen.after["service"]
    old, new = before["operations"], after["operations"]
    old_files, new_files = _file_status(before), _file_status(after)
    fields, limits = compare_schema(before, after)
    changes = {}
    for identifier in old.keys() | new.keys() | fields.keys():
        old_sites = old.get(identifier, {}).get("sites", [])
        new_sites = new.get(identifier, {}).get("sites", [])
        group = fields.get(identifier, {"fields": [], "status_additions": [], "status_removals": [],
                                        "response_root_pairs": [], "limits": [], "schema_sites": [None, None]})
        kinds, scoped = [], []
        presence = ["present" if sites else "unknown" if snap["unaccounted_scopes"] else "absent"
                    for sites, snap in ((old_sites, before), (new_sites, after))]
        for side, (owned, other, other_files, other_sources) in enumerate((
            (old_sites, new_sites, new_files, frozen.after_sources),
            (new_sites, old_sites, old_files, frozen.before_sources),
        )):
            word, label = ("removal", "after") if side == 0 else ("addition", "before")
            for entry in owned:
                file = entry["file"]
                siblings = [s for s in other if s["file"] == file]
                if siblings:
                    if (len(owned) == len(other) == 1 and file in frozen.changed_paths
                            and old_files.get(file) == new_files.get(file) == "complete"
                            and (entry["text"], entry["handler_ref"]) != (siblings[0]["text"], siblings[0]["handler_ref"])):
                        kinds.append("Route declaration changed")
                    continue
                _, _, opcodes = _diff(frozen.before_sources.get(file), frozen.after_sources.get(file))
                owner_snapshot = before if side == 0 else after
                other_text = other_sources.get(file)
                retained_as_text = bool(other_text and entry["text"] in other_text)
                # The declaration must be accounted for on both sides, including a
                # proved deletion from the tree and entries still present as text.
                complete = (_file_status(owner_snapshot).get(file) == "complete"
                            and (other_files.get(file) == "complete" or file not in other_sources)
                            and file in frozen.changed_paths and not retained_as_text
                            and _changed_line(opcodes, entry["line"], entry["end_line"], 2 * side))
                if complete:
                    scoped.append({"kind": word, "site": entry, "proof": "complete inventories and changed source lines"})
                elif not other:
                    presence[1 - side] = "unknown"
                    group["limits"].append(f"{word.title()}: Not checked — {file} has no complete {label} scan.")
                    if side == 0 and other_files.get(file) == "unreadable":
                        group["limits"].append(f"Observed before at {file}:{entry['line']}. "
                                               f"After: Not checked — {file} could not be read")
        for side, (owned, other) in enumerate(((old_sites, new_sites), (new_sites, old_sites))):
            if owned and not other:
                word = "removal" if side == 0 else "addition"
                if presence[1 - side] == "absent" and any(row["kind"] == word for row in scoped):
                    kinds.append("Operation removed" if side == 0 else "Operation added")
                else:
                    kinds.append("Listed only before among the declarations read" if side == 0 else
                                 "Listed only after among the declarations read")
                    if not group["limits"]:
                        label = "after" if side == 0 else "before"
                        group["limits"].append(f"{word.title()}: Not checked — {owned[0]['file']} has no complete {label} scan.")
        kinds.extend(f"Declaration {'removed from' if row['kind'] == 'removal' else 'added to'} {row['site']['file']}"
                     for row in scoped if "Operation removed" not in kinds and "Operation added" not in kinds)
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
    return changes, limits


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
        left, right, opcodes = _diff(frozen.before_sources.get(file), frozen.after_sources.get(file))
        left_counts, right_counts = Counter(left), Counter(right)
        complete = (af.get(file, "absent") in {"complete", "absent"} and
                    bf.get(file, "absent") in {"complete", "absent"} and
                    not any(gap["file"] in {file, before["scope"]}
                            for snap in (before, after) for gap in snap["unaccounted_scopes"]))
        old_lines = Counter(row["site"]["line"] for row in old[file])
        new_lines = Counter(row["site"]["line"] for row in new[file])
        functions = [{name for name, spans in _function_spans(rows).items() if len(spans) == 1}
                     for rows in (old[file], new[file])]
        equal_lines = {i1 + offset + 1: j1 + offset + 1
                       for tag, i1, i2, j1, _ in opcodes if tag == "equal" for offset in range(i2 - i1)}
        by_line = {row["site"]["line"]: row for row in new[file]}
        handled = set()
        for row in old[file]:
            loc = row["site"]
            line, function = loc["line"], row["enclosing_function"]
            other = by_line.get(equal_lines.get(line))
            if (complete and other and function and function == other["enclosing_function"]
                    and function in functions[0] & functions[1]
                    and loc["line"] == loc["end_line"] and other["site"]["line"] == other["site"]["end_line"]
                    and old_lines[line] == new_lines[other["site"]["line"]] == 1
                    and left_counts[left[line - 1]] == right_counts[left[line - 1]] == 1
                    and loc["text"] == other["site"]["text"]):
                evidence = "one complete call on a uniquely mapped unchanged line in the same named function"
                handled.update((row["site_id"], other["site_id"]))
                paired.append((row, other, evidence))
                if _target(row) != _target(other):
                    changes.append(_change(row, other, "Client target changed", evidence=evidence))
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
                untouched = (diffs == ["equal"] and _target(row) in
                             {_target(r) for r in (new[file] if side == 0 else old[file])
                              if r["site"]["text"] == loc["text"]})
                if untouched:
                    continue
                moved_or_copied = any(
                    source and loc["text"] in source for source in counterpart_sources(frozen, file, side)
                )
                if (complete and diffs == [expected] and not moved_or_copied
                        and only[loc["line"]] == 1 and lines[loc["line"] - 1] not in counterpart):
                    kind = "Client call removed" if side == 0 else "Client call added"
                    reason = None
                else:
                    kind = "Observed before" if side == 0 else "Observed after"
                    reason = CORRESPONDENCE if complete else (
                        f"{'Removal' if side == 0 else 'Addition'}: Not checked — {file} could not be compared."
                    )
                changes.append(_change(row if side == 0 else None, row if side == 1 else None, kind, reason))
    return changes, paired


def counterpart_sources(frozen, file, side):
    sources = frozen.after_sources if side == 0 else frozen.before_sources
    return (sources.get(path) for path in set(frozen.changed_paths) | {file})


def _function_spans(rows):
    result = defaultdict(set)
    for row in rows:
        result[row["enclosing_function"]].add(row["function_span"])
    return result


def _target(row):
    return row["requested_method"], row["mapped_path"], row["target_reason"]


def _change(old, new, kind, reason=None, evidence=None):
    return {"before_site_id": old["site_id"] if old else None, "after_site_id": new["site_id"] if new else None,
            "kind": kind, "reason": reason, "correspondence_evidence": evidence,
            "before_presence": "present" if old else "unknown" if reason else "absent",
            "after_presence": "present" if new else "unknown" if reason else "absent"}


def _count(observed, scope, version, reason=None):
    return {"observed": observed, "total": observed if reason is None else None,
            "scope": scope, "version": version, "reason": reason}


def _requests(pair, client, service):
    results = list(pair["matches"].values())
    reason = next((g["reason"] for g in client["unaccounted_scopes"]), None)
    match_gap = next((r["reason"] for r in results if r["status"] in {"partial", "not_checked"}), None)
    match_gap = match_gap or next((g["reason"] for g in service["unaccounted_scopes"]), None)
    basis = [pair["service_input_id"], pair["client_input_id"]]
    no_match = [key for key, match in pair["matches"].items() if match["status"] == "no_match"]
    unknown = [key for key, match in pair["matches"].items() if match["status"] == "target_unknown"]
    return {"known_unmatched_count": _count(len(no_match), purpose_scope(client), basis, reason or match_gap),
            "unknown_target_count": _count(len(unknown), purpose_scope(client), basis, reason),
            "no_match_site_ids": no_match, "unknown_site_ids": unknown}


def purpose_scope(client):
    return f"observed production client request sites under {client['scope']}"


def _service_risks(group, client_before, client_after, before, after, request_counts):
    identifier = group["operation_id"]
    kinds = group["change_kinds"]
    relevance = set()
    for pair in (before, after):
        relevance.update(key for key, result in pair["matches"].items() if identifier in result["target_ids"])
    group["client_site_ids"] = sorted(relevance)
    group["prefix_hints"], group["declaration_comparisons"] = [], []
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
        group["prefix_hints"].extend(dict(hint, side=side) for hint in hints)
        group["declaration_comparisons"].append({
            "side": side, "purpose": pair["purpose"], "compared": len(client["calls"]),
            "inventory_reason": next((gap["reason"] for gap in client["unaccounted_scopes"]), None),
            "closest_site_ids": [key for key, result in pair["matches"].items() if identifier in result["target_ids"]],
            **observed,
        })
    if "Operation removed" in kinds:
        for key in sorted(relevance & before["matches"].keys() & after["matches"].keys()):
            lost, remains = before["matches"][key], after["matches"][key]
            if identifier in lost["target_ids"] and lost["status"] == "match":
                if remains["status"] == "no_match":
                    group["risks"].append({"kind": "Client calls a missing route", "site_id": key})
                elif remains["status"] == "match" and identifier not in remains["target_ids"]:
                    group["risks"].append({"kind": "Client target needs confirmation", "site_id": key})
    if "Operation added" in kinds:
        for key in sorted(relevance & before["matches"].keys() & after["matches"].keys()):
            old, new = before["matches"][key], after["matches"][key]
            if old["status"] == "no_match" and new["status"] == "match" and identifier in new["target_ids"]:
                group["risks"].append({"kind": "Existing client call gains a matching declaration", "site_id": key})
    if not group["risks"]:
        keys = next((key for key in sorted(relevance) if any(pair["matches"].get(key, {}).get("status") == "possible"
                                                            for pair in (before, after))), None)
        if keys is not None:
            group["risks"].append({"kind": "Client target needs confirmation", "site_id": keys})


def _action(group):
    risks = [risk["kind"] for risk in group["risks"]]
    if "Client calls a missing route" in risks:
        return "Check whether this client still needs the old declaration and which version it will use."
    if "Existing client call gains a matching declaration" in risks:
        return "Check which service version this client needs."
    if "Client target needs confirmation" in risks:
        return "Check the address value and which route takes priority."
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
        change["risk"] = "Client calls a missing route"
    elif new and (new["status"] == "possible" or old and old["target_ids"] != new["target_ids"]):
        change["risk"] = "Client target needs confirmation"
    change["requested_target"] = _target(target)[:2]
    change["remaining_after_site_ids"] = [
        key for key, row in after["matches"].items()
        if a and not b and set(row["target_ids"]) & set(change["operation_ids"])
    ]
    change["review_action"] = ("Check which service version this call needs." if b else
                                "Check which clients still need the old declaration.")
    return change


def _standing(before, after, shared, ops, snapshots):
    result = []
    for a, b in shared:
        old, new = before["matches"].get(a), after["matches"].get(b)
        if not any(match and match["status"] == "no_match" for match in (old, new)):
            continue
        timing = "Already present" if all(match and match["status"] == "no_match" for match in (old, new)) else None
        if timing is None and old and new and {old["status"], new["status"]} == {"match", "no_match"}:
            timing = "Appears with this change" if new["status"] == "no_match" else "Clears with this change"
        seen = [match["service_rev"][:8] + " with client " + match["client_rev"][:8]
                for match in (old, new) if match and match["status"] == "no_match"]
        result.append({"kind": "request has no same-method match among the declarations read",
                       "before_site_id": a, "after_site_id": b, "operation_id": None,
                       "linked_to_change": any(key in group["client_site_ids"] for key in (a, b) for group in ops),
                       "timing": timing or f"Seen at {' and '.join(seen)}. When this difference began: "
                       f"Not checked — {'before' if not old or old['status'] == 'partial' else 'after'} comparison is missing.",
                       "versions": seen})
    for snapshot in snapshots:
        document = snapshot["schema"]
        if document["status"] != "complete":
            continue
        for op in sorted(document["operations"].keys() ^ snapshot["operations"].keys()):
            origin = "The committed schema" if op in document["operations"] else snapshot["operation_authority"]
            result.append({"kind": f"{origin} lists this operation. Registration: Not checked — no gateway mapping was checked.",
                           "before_site_id": None, "after_site_id": None, "operation_id": op,
                           "linked_to_change": any(op == group["operation_id"] for group in ops),
                           "timing": f"Seen at {snapshot['rev'][:8]}. When this difference began: Not checked — "
                           "no complete registration comparison was available.",
                           "versions": [snapshot["rev"]]})
    return result


def _summary(operations, service_limits, service_sides, usage, pairs, client_changes):
    service_gap = next((g["reason"] for snap in service_sides for g in snap["unaccounted_scopes"]), None)
    fields_gap = next(iter(service_limits), None) or next(
        (gap for group in operations for gap in group["limits"]
         if "JSON fields:" in gap or "stops at" in gap or "Body comparison:" in gap or "Status change:" in gap), None)
    counts = {"operation_count": _count(sum(bool(
        group["fields"] or group["status_additions"] or group["status_removals"]
        or any(kind in {"Operation added", "Operation removed", "Route declaration changed"} for kind in group["change_kinds"])
    ) for group in operations), "changed operations among declarations and schema read",
        [snap["rev"] for snap in service_sides], service_gap),
        "changed_field_count": _count(sum(len(group["fields"]) for group in operations),
                                      "measured JSON field declarations", [snap["rev"] for snap in service_sides], fields_gap),
        "listed_usage_count": _count(len(usage), "distinct client request sites shown",
                                     [pair["client_input_id"] for pair in pairs]),
        "possible_only_usage_count": _count(sum(
            "possible" in statuses and statuses <= {"possible", "no_match"}
            for row in usage for statuses in [{m["status"] for m in (row["match_before"], row["match_after"]) if m}]
        ), "shown requests with possible matches only", [pair["client_input_id"] for pair in pairs],
            next((m["reason"] for row in usage for m in (row["match_before"], row["match_after"])
                  if m and m["status"] in {"not_checked", "partial"}), None)),
        "client_change_count": _count(sum(not row["reason"] for row in client_changes), "proved client call edits",
                                      [pair["client_input_id"] for pair in pairs],
                                      next((row["reason"] for row in client_changes if row["reason"]), None)),
    }
    return counts


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
    client_changed |= [row["revision"] for row in bc["config_sources"]] != [
        row["revision"] for row in ac["config_sources"]]
    mixed = frozen.reference is None and service_changed and client_changed
    show_service = mixed or requested == "producer"
    show_client = mixed or requested == "consumer"
    service_after, client_after = (aps if show_service else bs), (ac if show_client else bc)
    pairs = [compare(bs, bc, "before"), compare(service_after, client_after, "after")]
    service_pairs = pairs
    if mixed:
        pairs.extend([compare(aps, bc, "service_first"), compare(bs, ac, "client_first")])
        service_pairs = [pairs[0], pairs[2]]
    snapshots = {snap["input_id"]: snap for snap in (bs, bc, service_after, client_after)}
    request_counts = {pair["purpose"]: _requests(pair, snapshots[pair["client_input_id"]],
                                                snapshots[pair["service_input_id"]]) for pair in pairs}
    service_changes, field_limits = _service_changes(frozen) if show_service else ({}, [])
    if not show_service:
        if bs["snapshot_kind"] == "unavailable":
            field_limits = [bs["schema"]["reason"]]
        else:
            fixed_schema, field_limits = compare_schema(bs, bs)
            field_limits.extend(limit for group in fixed_schema.values() for limit in group["limits"])
    operations = list(service_changes.values())
    calls = {row["site_id"]: row for snap in (bc, client_after) for row in snap["calls"]}
    for group in operations:
        _service_risks(group, bc, bc, *service_pairs, request_counts)
        if mixed:
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
        0 if any(r["kind"] == "Client calls a missing route" for r in group["risks"]) else
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
        usages.append({"site_id": key, "before_site_id": lhs, "after_site_id": rhs,
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
        reference = bc if requested == "producer" else bs
        delta = datetime.fromisoformat(subject["committed_at"]) - datetime.fromisoformat(reference["committed_at"])
        ref = {"reference_input_id": reference["input_id"], "selected_by": "first parent in the same repository",
               "selection_reason": "the counterpart is held at the base; joint changes also check the after tree",
               "availability_status": "selected", "availability_evidence": {"tree": reference["tree"]},
               "duration_seconds": -int(delta.total_seconds()), "branch": None, "observed_tip": None,
               "observed_at": None, "selection_history_tip": None}
    for group in operations:
        group["call_changes"] = [row for row in client_changes if group["operation_id"] in row["operation_ids"]]
    elements = [{"element_id": group["operation_id"], "change_kinds": group["change_kinds"]}
                for group in operations if group["change_kinds"]]
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
    summary = _summary(operations, field_limits, [bs, service_after], usages, pairs, client_changes)
    if reason := ref.get("unavailable_reason"):
        summary["possible_only_usage_count"].update(total=None, reason=reason)
    summary.update({name: {purpose: counts[name] for purpose, counts in request_counts.items()}
                    for name in ("known_unmatched_count", "unknown_target_count")})
    schema_unchanged = (frozen.reference is None or show_service) and service_sources_changed and (
        bool(bs["sha256"]) and bs["sha256"] == aps["sha256"]
    )
    return {
        "schema_version": "2", "report_id": _fingerprint([version, ref, frozen.request.selection_time,
                                                         [(key, snap["config_sources"], snap["tool"]["build_identity"],
                                                           snap["scope"], snap["source_url"], snap["operation_authority"])
                                                          for key, snap in inputs.items()]]),
        "api": bs["api"], "mode": "version_comparison", "report_time": frozen.request.selection_time,
        "cutoff_at": frozen.request.selection_time, "inputs": inputs, "reference": ref, "comparisons": pairs,
        "change": version, "impacts": operations, "call_changes": client_changes, "usages": usages,
        "binding_status": {"status": "not_checked", "reason": EARLIER},
        "coverage": [{"input_id": key, "scope": snap["scope"],
                      "status": ("not_checked" if snap["snapshot_kind"] == "unavailable" else
                                 "partial" if snap["unaccounted_scopes"] else "complete"),
                      "reason": snap["unaccounted_scopes"][0]["reason"] if snap["unaccounted_scopes"] else None}
                     for key, snap in inputs.items()],
        "warnings": warnings, "field_limits": field_limits, "code_changed_with_unchanged_schema": schema_unchanged,
        "standing_mismatches": _standing(*pairs[:2], shared_keys, operations, (bs,) if bs == service_after else (bs, service_after)),
        "summary": summary, "request_counts": request_counts,
        "unobserved": None if unobserved_reason else [], "unobserved_reason": unobserved_reason,
    }
