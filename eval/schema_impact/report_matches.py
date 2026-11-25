"""Fresh method/path matches with explicit limits on declarations and mapping."""

from collections import defaultdict
from dataclasses import dataclass
from functools import cache

from .consumer_ts import ClientCall
from .match import _segments, bind
from .report_proof import failed, source_gap


WILDCARD = ("Method coverage: Not checked — the matcher compares exact methods only and "
            "this declaration lists every method.")
OVERLAPS = "Not checked — the scan kept only the closest matches"
GATEWAY = "Registration: Not checked — no gateway mapping was checked"
MAPPING = "Mapping: Not checked — the scan did not record which rule produced this path."
TARGET_REASONS = {
    "dynamic_url": "The address is built while the program runs",
    "conditional_url": "The address depends on a condition",
    "non_string_key": "The first argument does not give a text address",
    "dynamic_method": "The method is chosen while the program runs",
    "unknown_client_method": "This client function has no configured HTTP method",
    "unconfigured_client_form": "This form of request could not be read",
}


@dataclass
class _Index:
    operations: dict


def parts(identifier):
    return identifier.removeprefix("op:").split(" ", 1)


def _compatible(client, route, optional=False):
    a, b = _segments(client), _segments(route)
    if len(a) == len(b):
        return all(x == y or "{}" in (x, y) for x, y in zip(a, b))
    if optional and len(a) < len(b):
        return any(_compatible(client, "/" + "/".join(b[:i] + b[i + 1:]), True)
                   for i, segment in enumerate(b) if segment == "{}")
    return False


def compare(service, client, purpose):
    per_method = defaultdict(dict)
    for identifier, op in service["operations"].items():
        per_method[parts(identifier)[0]][identifier] = op
    coverage = service["unaccounted_scopes"]
    source = service["operation_authority"]

    @cache
    def outcome(method, path, reason, mapped):
        result = {"authority": source, "target_ids": [], "primary_grade": None, "reason": reason,
                  "alternates": [], "overlaps": None, "overlaps_reason": OVERLAPS,
                  "registration": "not_checked", "registration_reason": GATEWAY}
        def unchecked(status, text, gap=None):
            proof = gap or failed("Request match", service["scope"], service["rev"], text)
            return dict(result, status=status, reason=text, proof_failure=proof)

        if reason or not method or not path:
            return dict(result, status="target_unknown", primary_grade="unbound")
        if service["snapshot_kind"] == "unavailable":
            return unchecked("not_checked", coverage[0]["reason"], source_gap("Request match", service))
        matches = per_method.get(method, {})
        for identifier, op in (matches | per_method.get("*", {})).items():
            optional = any(s["optional_param"] for s in op["sites"])
            if not _compatible(path, parts(identifier)[1], optional):
                continue
            if identifier in per_method.get("*", {}):
                return unchecked("not_checked", WILDCARD)
            if optional:
                return unchecked("not_checked", "Not checked — this path segment may be omitted")
        call = ClientCall("", 0, method, "", path, "", None, None, None)
        grade, ids, why = bind(call, _Index(matches))
        result.update(target_ids=ids, reason=why)
        if not mapped:
            return unchecked("not_checked", MAPPING)
        if any(len(matches[identifier]["sites"]) != 1 for identifier in ids):
            return unchecked("not_checked", "Not checked — two declarations have the same method and path")
        if coverage:
            return unchecked("partial", f"{coverage[0]['file']}: {coverage[0]['reason']}",
                             source_gap("Request match", service))
        if not ids:
            for other_method, operations in sorted(per_method.items()):
                if other_method in {method, "*"}:
                    continue
                for identifier, op in operations.items():
                    optional = any(s["optional_param"] for s in op["sites"])
                    if _compatible(path, parts(identifier)[1], optional):
                        alt_grade, _, alt_reason = bind(
                            ClientCall("", 0, other_method, "", path, "", None, None, None),
                            _Index({identifier: op}),
                        )
                        alternative = {"operation": identifier, "method": other_method, "grade": alt_grade,
                                       "reason": alt_reason}
                        if optional or len(op["sites"]) != 1:
                            alternative.update(grade=None, reason="the alternate declaration is ambiguous")
                        result["alternates"].append(alternative)
        return dict(result, status={"bound": "match", "candidate": "possible", "unbound": "no_match"}[grade],
                    primary_grade=grade)

    matches = {}
    for row in client["calls"]:
        result = outcome(row["requested_method"], row["mapped_path"], row["target_reason"],
                         row["mapping_source"] is not None)
        matches[row["site_id"]] = {
            **result, "service_input_id": service["input_id"], "client_input_id": client["input_id"],
            "mapping": row["mapping_source"], "service_rev": service["rev"], "client_rev": client["rev"],
            "requested_method": row["requested_method"], "mapped_path": row["mapped_path"],
        }
    return {"purpose": purpose, "service_input_id": service["input_id"], "client_input_id": client["input_id"],
            "matches": matches}


def prefix_hints(identifier, client):
    method, path = parts(identifier)
    hints = []
    for call in client["calls"]:
        if call["requested_method"] != method or not call["mapped_path"] or not call["mapping_source"]:
            continue
        compared = call["mapped_path"]
        if _compatible(compared, path):
            continue
        suffix = []
        for left, right in zip(reversed(_segments(compared)), reversed(_segments(path))):
            if left != right or left == "{}":
                break
            suffix.append(left)
        if suffix:
            hints.append({"site_id": call["site_id"], "declaration_path": path,
                          "request_path": compared, "common_suffix": "/" + "/".join(reversed(suffix))})
    return hints


def match_text(match):
    if match is None:
        return "No call at this site"
    status, reason = match["status"], match["reason"]
    ids = [identifier.removeprefix("op:") for identifier in match["target_ids"]]
    if status == "target_unknown":
        return "Target unknown — " + TARGET_REASONS.get(reason, "This form of request could not be read")
    if status == "not_checked":
        return reason
    source, revision = match["authority"], match["service_rev"][:8]
    if status == "partial":
        observation = "Closest match among the declarations read: " + ", ".join(ids) + ". " if ids else ""
        return f"{observation}Not checked — {reason}"
    if status == "match":
        return f"Matches {ids[0]} in {source} at {revision}."
    if status == "possible":
        if len(ids) > 1:
            return "Possible match — several declarations fit: " + ", ".join(ids)
        _, path = parts(ids[0])
        values = [declared for requested, declared in zip(_segments(match["mapped_path"]), _segments(path))
                  if requested == "{}" and declared != "{}"]
        return "Possible match — the value in this address would have to be " + " and ".join(values) + "."
    return f"No {match['requested_method']} match among declarations read from {source} at {revision}. {GATEWAY}."
