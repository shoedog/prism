"""Operation groups and their field, status and client tables."""

import json

from .report_analysis import FIELD_USE, MISSING_DECLARATION, NO_FIELD_USE, RISK_ACTIONS
from .report_layout import _code, compact_html, request_counts, site_link, source_site as _site, table
from .report_matches import OVERLAPS, WILDCARD, parts
from .report_proof import explain

_SERVICE = ["What changed", "Before", "After", "Service code"]
_CLIENT = ["Client call and function", "Request as written → path checked", "Match before", "Match after", "What to check"]

_TYPES = {
    "string": "text", "integer": "whole number", "number": "number (may have a fractional part)",
    "boolean": "true or false", "array": "list", "object": "group of named values",
    "null": "explicitly no value",
}
_FORMATS = {"date": "calendar date", "date-time": "date and time",
            "int32": "32-bit integer", "int64": "64-bit integer"}
def _type(value):
    if value.startswith("["):
        return " or ".join(_type(item) for item in json.loads(value))
    if not value:
        return "No type was recorded"
    return _TYPES.get(value, f"{_code(value)} (meaning: Not checked)")


def _field_label(record):
    root = record["root"]
    message = ("Request body" if root.startswith("req:") else
               f"Response body ({root.rsplit(':', 1)[1]})")
    path = {"[]": "each list item", "*": "each map value (the value under each key)"}
    return " › ".join([message, *(path.get(segment, _code(segment)) for segment in record["path"].split("/"))])


def risk_label(kind):
    return "No matching declaration among those read" if kind == MISSING_DECLARATION else kind


class OperationTables:
    def operation(self, group, complete=False, anchor=None):
        identifier = group["operation_id"]
        method, path = parts(identifier)
        name = f"{method} {path}"
        chunks = [f'<a id="{anchor}"></a>' if anchor else "", f"#### {_code(name)}"]
        limits = [self.mark(reason, name) for reason in group["limits"]]
        if method == "*":
            limits.insert(0, "Every method is listed · " + self.mark(WILDCARD, name))
        if limits:
            chunks.append(self.list("limits for " + name, list(dict.fromkeys(limits)), complete=complete))

        rows, whole_operation = self.operation_rows(group, complete)
        statuses = []
        for key, change, before, after in (
            ("status_removals", "removed", "Listed", "Absent"), ("status_additions", "added", "Absent", "Listed"),
        ):
            statuses.extend((f"Successful response status {change} · {status['status']}", before, after,
                             self.source_site(status["site"], "Schema response status")) for status in group[key])
        visible_statuses, status_link = self.section(f"Statuses for {name}", statuses, 3, "statuses", complete)
        if status_link:
            self._fill(status_link, table(_SERVICE, statuses))

        distinct = [row for row in group["fields"] if next(
            iter(self.shared_fields.get(self.field_key(row), {})), identifier) == identifier]
        fields, field_link = self.section(f"Fields for {name}", group["fields"], 3, "fields", complete, shown=distinct)
        field_rows = [self.field_row(group, row, complete) for row in fields]
        if field_link:
            self._fill(field_link, table(_SERVICE, [self.field_row(group, row, True) for row in group["fields"]]))

        schema_rows = [*visible_statuses, *field_rows]
        schema_links = " · ".join(filter(None, (status_link, field_link)))
        if whole_operation and (schema_rows or schema_links):
            # The status changes remain independent, including responses without a body.
            detail = compact_html(table(_SERVICE, schema_rows)) if schema_rows else ""
            if schema_links:
                detail += compact_html(schema_links)
            rows.append((f"<details><summary>Fields and response statuses</summary>{detail}</details>", "", "", ""))
        else:
            rows.extend(schema_rows)
        chunks.append(table(_SERVICE, rows))
        if schema_links and not whole_operation:
            chunks.append(schema_links)
        if group["response_root_pairs"]:
            chunks.append(self.list("response status moves", [
                f"Successful response status changed, {pair['before_status']} → {pair['after_status']}"
                for pair in group["response_root_pairs"]
            ], complete=complete))
        if group.get("message_coverage"):
            bodies = []
            for record in group["message_coverage"]:
                root = record["root"]
                message = "request body" if root.startswith("req:") else "response body " + root.rsplit(":", 1)[1]
                bodies.append(f"{record['side'].title()} {message} · application/json from {record['media_source']}.")
            chunks.append(self.list("message coverage", bodies, complete=complete))
        if group["fields"]:
            linked = any(identifier in match["target_ids"] and match["status"] in {"match", "possible"}
                         for side in ("before", "after") for match in self.pairs[side]["matches"].values())
            chunks.append("Client field use · " + self.mark(FIELD_USE if linked else NO_FIELD_USE, name))
        chunks.extend(self.operation_calls(group, complete))
        if any(change["kind"] == "Client call removed" for change in group["call_changes"]):
            chunks.append("The checked after client no longer has this call at its before site. "
                          "[See client changes](#client-changes).")
        actions = [group["review_action"]["text"], *(RISK_ACTIONS[risk["kind"]] for risk in group["risks"])]
        if group["fields"]:
            actions.append("Check what the client sends and how it reads the changed fields.")
        if statuses:
            actions.append("Check clients that branch on the response status.")
        if group["prefix_hints"]:
            actions.append("Check the leading parts of addresses listed as possible matches under another prefix.")
        chunks.append("**What to check** · " + " ".join(dict.fromkeys(actions)))
        return "\n\n".join(filter(None, chunks))

    def operation_rows(self, group, complete):
        rows = []
        kinds = group["change_kinds"]
        whole_operation = next((kind for kind in kinds if kind in {"Operation added", "Operation removed"}), None)
        if whole_operation:
            kinds = [whole_operation]
        for kind in kinds or ["Operation comparison"]:
            if kind in {"Operation fields changed", "Successful response statuses changed"}:
                continue
            file_changes = [change for change in group["file_changes"]
                            if kind == f"Declaration {'removed from' if change['kind'] == 'removal' else 'added to'} "
                            + change["site"]["file"]]
            sites = ([self.route_site(change["site"], "before" if change["kind"] == "removal" else "after")
                      for change in file_changes] if file_changes else
                     [self.route_site(site, side) for side, entries in group["producer_sites"].items() for site in entries])
            source = self.list("service code locations", sites, complete=complete) or self.mark(
                "source location was unavailable", group["operation_id"].removeprefix("op:"))
            if kind == "Route declaration changed":
                source += "<br>" + self.mark(
                    "Validation or handler edits: Not checked — the two source bodies were not compared.",
                    group["operation_id"].removeprefix("op:"))
            before = "Listed at this site" if file_changes and file_changes[0]["kind"] == "removal" else (
                "No declaration at this site" if file_changes else
                self.presence(group["before_presence"], group["proof_failures"]))
            after = "Listed at this site" if file_changes and file_changes[0]["kind"] == "addition" else (
                "No declaration at this site" if file_changes else
                self.presence(group["after_presence"], group["proof_failures"]))
            rows.append((kind, before, after, source))
        return rows, whole_operation

    def presence(self, value, failures):
        if value == "unknown":
            return self.mark(explain(failures[0]) if failures else
                             "the service has no recorded declaration for this schema operation")
        return "Listed" if value == "present" else "Absent from the checked operations"

    def source_site(self, site, kind=None):
        return _site(site, kind) if site else self.mark("source location was unavailable")

    def route_site(self, site, side):
        source = _site(site, f"{side.title()} · Route annotation" if site["kind"] == "annotation"
                       else f"{side.title()} · Route declaration")
        if site["kind"] == "annotation":
            return f"{source} · Handler named in a comment {_code(site['function'])}"
        if site["handler_site"]:
            return f"{source} · {_site(site['handler_site'], 'Handler')}"
        return source + " · Handler location " + self.mark(
            "Handler location: Not checked — only the declaration was found", site_link(site))

    def field_row(self, group, row, complete):
        previous, following = row["before_shape"], row["after_shape"]
        changes = []
        if previous is None:
            changes.append("Field declaration added")
        elif following is None:
            changes.append("Field declaration removed")
        else:
            for key, label in (("type", "Type changed"), ("format", "Format changed"), ("items_type", "List item type changed")):
                if previous[key] != following[key]:
                    changes.append(label)
            if previous["required"] != following["required"]:
                changes.append("Now listed as required in its parent" if following["required"] else
                               "No longer listed as required in its parent")
            for a, b, label in ((following, previous, "Listed values added"), (previous, following, "Listed values removed")):
                values = sorted(set(a["enum"]) - set(b["enum"]))
                if values:
                    changes.append(f"{label} · " + self.list("listed values", [_code(v) for v in values], complete=complete))
        sites = [_site(site, "Schema entry for this operation") for site in group["schema_sites"] if site]
        field = _field_label(row["after"] or row["before"])
        affected = group["operation_id"].removeprefix("op:") + " · " + field
        location_reason = ("Field location: Not checked — only the operation was located" if sites else
                           "Field location: Not checked — the schema operation site was unavailable")
        sites.append("Field location " + self.mark(location_reason, affected))
        same = self.shared_fields.get(self.field_key(row), {})
        if not complete and len(same) > 1:
            changes.append(f"Same change in {len(same) - 1} more operations")
        return (field + " · " +
                self.list("properties changed", changes, complete=complete),
                self.shape(previous, complete, affected + " · before"),
                self.shape(following, complete, affected + " · after"),
                self.list("field source locations", sites, complete=complete))

    def shape(self, shape, complete, affected):
        if shape is None:
            return "Absent"
        parts = [_type(shape["type"]),
                 "Required when its parent is present" if shape["required"] else "Optional in its parent"]
        if shape["format"]:
            value = shape["format"]
            parts.append("Format " + (f"{_code(value)} ({_FORMATS[value]})" if value in _FORMATS else _code(value) + " · " + self.mark(
                "the schema gives no meaning the reader recognizes for this format", affected)))
        if shape["items_type"]:
            parts.append("List item type " + " or ".join(_type(item) for item in shape["item_types"]))
        if shape["enum"]:
            parts.append("Listed values " + self.list("listed values", [_code(v) for v in shape["enum"]], complete=complete))
        else:
            parts.append("Listed values " + self.mark(
                "Listed values: Not checked — the scan could not distinguish an omitted list from an empty or unreadable one",
                affected))
        return self.list("field properties", parts, complete=complete)

    def operation_calls(self, group, complete):
        chunks = []
        name = group["operation_id"].removeprefix("op:")
        for checked in group["declaration_comparisons"]:
            context = [f"**{checked['side'].title()} requests** · " + request_counts(checked["purpose"], checked, self.mark)]
            if not checked["closest_site_ids"]:
                reason = checked["inventory_reason"] or checked["known_unmatched_count"]["reason"]
                if reason:
                    context.append("Closest matches " + self.mark(reason, name))
                else:
                    context.append(f"None of the {checked['compared']} calls compared had this declaration among its closest matches")
            context.append("Other matching paths " + self.mark(OVERLAPS))
            if checked["inventory_reason"]:
                context.append("Other prefixes " + self.mark(checked["inventory_reason"], name))
            chunks.append(" · ".join(context))
        occurrences = [*(risk["site_id"] for risk in group["risks"]), *group["client_site_ids"]]
        keys = list(dict.fromkeys(self.usage_keys.get(key, key) for key in occurrences))
        if keys:
            visible, link = self.section("Calls for " + name, keys, 3, "calls", complete)
            chunks.append(table(_CLIENT, [self.operation_call(group, key, complete) for key in visible]))
            if link:
                self._fill(link, table(_CLIENT, [self.operation_call(group, key, True) for key in keys]))
                chunks.append(link)
        if group["prefix_hints"]:
            hints = group["prefix_hints"]
            chunks.append(self.list("other-prefix hints", [
                f"Possible match under another prefix — both addresses end in {hint['common_suffix']}. "
                f"{_site(self.calls[hint['site_id']]['site'], excerpt=False)}. "
                f"{self.request(self.calls[hint['site_id']])}" for hint in hints], complete=complete))
        return chunks

    def operation_call(self, group, key, complete):
        use = self.usages.get(key)
        call = self.calls[key]
        prior = None
        if use and use["before_site_id"] and use["after_site_id"] and use["before_site_id"] != use["after_site_id"]:
            prior, call = (self.calls[use[f"{side}_site_id"]] for side in ("before", "after"))
        old = use["match_before"] if use else self.pairs["before"]["matches"].get(key)
        new = use["match_after"] if use else self.pairs["after"]["matches"].get(key)
        risks = [risk_label(row["kind"]) for row in group["risks"]
                 if self.usage_keys.get(row["site_id"], row["site_id"]) == key]
        if risks and "service_first" in self.pairs:
            risks = ["If the service ships first · " + risk for risk in risks]
        absence = use and "absent" in (use["before_presence"], use["after_presence"])
        missing = use["presence_reason"] if use and not absence else None
        return self.call_row(call, old, new, " · ".join(risks) or "See below", complete,
                             missing=missing, prior=prior)

