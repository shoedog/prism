"""Report populations and the evidence required to count them."""

from .report_claims import Record, decide, reasons
from .report_proof import declaration_records, failed, route_record, source_record
from .report_schema import schema_record


def _count(observed, scope, version, reason=None):
    proof = reason if isinstance(reason, dict) and "confirmed" in reason else None
    if proof is not None:
        reason = reasons(proof)
    return {"observed": observed, "total": observed if reason is None else None,
            "scope": scope, "version": version, "reason": explain(reason) if isinstance(reason, dict) else reason,
            "proof_failure": reason if isinstance(reason, dict) else None, "claim": proof}


def request_summary(pair, client, service):
    results = list(pair["matches"].values())
    records = {
        Record("client inventory"): source_record("Client request inventory", client),
        Record("declaration inventory"): source_record("Operation inventory", service),
        Record("route specification"): route_record(service),
        Record("match population"): [failure for r in results if r["status"] in {"partial", "not_checked"}
                                     for failure in r["claim"]["failures"]] or True,
    }
    basis = [pair["service_input_id"], pair["client_input_id"]]
    no_match = [key for key, match in pair["matches"].items() if match["status"] == "no_match"]
    unknown = [key for key, match in pair["matches"].items() if match["status"] == "target_unknown"]
    return {"known_unmatched_count": _count(len(no_match), purpose_scope(client), basis, decide("Request count", records)),
            "unknown_target_count": _count(len(unknown), purpose_scope(client), basis,
                                           decide("Unknown target count", records)),
            "closest_match_claim": decide("Closest-match population", records),
            "no_match_site_ids": no_match, "unknown_site_ids": unknown}


def purpose_scope(client):
    return f"observed production client request sites under {client['scope']}"


def summarize(operations, service_limits, service_sides, client_sides, usage, pairs, client_changes):
    sides = ("before", "after")
    service_records = declaration_records(service_sides, "Operation inventory")
    client_records = {Record("client inventory", side): source_record("Client call inventory", snap, side)
                      for side, snap in zip(sides, client_sides)}
    schemas = [snap["schema"] for snap in service_sides]
    state = [schema["status"] for schema in schemas]
    fixed = service_sides[0]["input_id"] == service_sides[1]["input_id"]
    schema_records = {Record("schema inventory", side): schema_record(schema, "fixed service" if fixed else side)
                      for side, schema in zip(sides, schemas)}
    route_only = state in (["path_absent", "path_absent"], ["omitted", "omitted"], ["unselected", "unselected"])
    scope_records = {Record("operation schema scope", side): True if route_only else schema_records[Record(
        "schema inventory", side)] for side in sides}
    schema_gaps = [*service_limits, *(limit for group in operations for limit in group["schema_limits"])]
    body_gaps = [failed("Schema comparison", None, None, limit) for limit in schema_gaps] if state == [
        "complete", "complete"] else []
    proof_gaps = [failure for group in operations for failure in group["proof_failures"]]
    operation_records = service_records | scope_records | {
        Record("operation comparison"): [*body_gaps, *proof_gaps] or True}
    field_records = schema_records | {Record("body comparisons"): body_gaps or True}
    client_records[Record("client comparison")] = [failure for row in client_changes
                                                   for failure in row["proof_failures"]] or True
    reviewed = {row["operation_id"] for row in operations if row["change_kinds"]}
    reviewed.update(identifier for change in client_changes for identifier in change["operation_ids"])
    possible = 0
    usage_gap = None
    for row in usage:
        applicable = [(side, row[f"match_{side}"]) for side in ("before", "after")
                      if row[f"{side}_presence"] != "absent"]
        if any(match is None or match["status"] not in {"match", "possible", "no_match"}
               for _, match in applicable):
            for side, match in applicable:
                if match is None or match["status"] not in {"match", "possible", "no_match"}:
                    usage_gap = usage_gap or (row["presence_reason"] if match is None else
                                             match.get("proof_failure") or match["reason"] or
                                             f"{side} match could not be assessed")
            continue
        matches = {match["status"] for _, match in applicable if reviewed.intersection(match["target_ids"])}
        possible += "possible" in matches and "match" not in matches
    return {
        "operation_count": _count(sum(bool(
            group["fields"] or group["status_additions"] or group["status_removals"]
            or group["file_changes"] or "Route declaration changed" in group["change_kinds"]
        ) for group in operations), "changed operations among declarations read" if route_only else
            "changed operations among declarations and schema read",
            [snap["rev"] for snap in service_sides], decide("Operation total", operation_records)),
        "changed_field_count": _count(sum(len(group["fields"]) for group in operations),
                                      "measured JSON field declarations", [snap["rev"] for snap in service_sides],
                                      decide("Field total", field_records)),
        "schema_coverage": decide("Schema coverage", field_records),
        "empty": {
            "service": decide("Empty service report", operation_records),
            "client": decide("Empty client report", client_records),
            "both": decide("Empty mixed report", operation_records | client_records),
        },
        "listed_usage_count": _count(len(usage), "distinct recorded client calls shown",
                                     [pair["client_input_id"] for pair in pairs]),
        "listed_usage_by_version": {side: sum(row[f"{side}_site_id"] is not None for row in usage)
                                    for side in ("before", "after")},
        "possible_only_usage_count": _count(possible, "shown requests with possible matches only",
                                            [pair["client_input_id"] for pair in pairs], decide(
                                                "Possible-only count", service_records | client_records | {
                                                    Record("applicable match comparisons"): (
                                                        usage_gap if isinstance(usage_gap, dict) else
                                                        failed("Match comparison", None, None, usage_gap)
                                                        if usage_gap else True)})),
        "client_change_count": _count(sum(not row["reason"] for row in client_changes), "proved client call edits",
                                      [pair["client_input_id"] for pair in pairs],
                                      decide("Client change total", client_records)),
    }
