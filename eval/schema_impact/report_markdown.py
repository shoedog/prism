"""Render the live report and its complete tables from the version-2 document."""

from collections import defaultdict
import json

from .report_analysis import EARLIER, FIELD_USE, NO_FIELD_USE
from .report_layout import (
    MODE, Limits, _code, appendix_section, compact_html, coverage, provenance, review_first, site_link, table, versions_line,
)
from .report_matches import GATEWAY, MAPPING, OVERLAPS, WILDCARD, match_text, parts


_TYPES = {
    "string": "text", "integer": "whole number", "number": "number (may have a fractional part)",
    "boolean": "true or false", "array": "list", "object": "group of named values",
    "null": "explicitly no value",
}
_FORMATS = {"date": "calendar date", "date-time": "date and time",
            "int32": "32-bit integer", "int64": "64-bit integer"}
_PAIR_LABELS = {"before": "Before both edits", "after": "Both edits included",
                "service_first": "If the service ships first", "client_first": "If the client ships first"}
_SERVICE = ["What changed", "Before", "After", "Service code"]
_CLIENT = ["Client call and function", "Request as written → path checked", "Match before", "Match after", "What to check"]
_UNKNOWN = "Changed client calls with an unknown target"
_RISK_ACTIONS = {
    "Client calls a missing route": "Check whether this client still needs the old declaration and which version it will use.",
    "Existing client call gains a matching declaration": "Check which service version this client needs.",
    "Client target needs confirmation": "Check the address value and which route takes priority.",
}


def _site(site, kind=None, *, excerpt=True):
    if not site:
        return "Not checked — source location was unavailable"
    location = _code(f"{site['file']}:{site['line']}")
    if site.get("url"):
        location = f"[{location}]({site['url']})"
    version = _code(site["rev"][:8])
    text = _code(site.get("text")) if site.get("text") and (excerpt or not site.get("url")) else ""
    return " · ".join(part for part in (kind, location, version, text) if part)


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


class Markdown:
    def __init__(self, doc):
        self.doc = doc
        self.calls = {call["site_id"]: call for snapshot in doc["inputs"].values() if snapshot["role"] == "consumer"
                      for call in snapshot["calls"]}
        self.pairs = {pair["purpose"]: pair for pair in doc["comparisons"]}
        self.usages = {row["site_id"]: row for row in doc["usages"]}
        self.usage_keys = {key: row["site_id"] for row in doc["usages"]
                           for key in (row["before_site_id"], row["after_site_id"]) if key}
        self.shared_fields = defaultdict(dict)
        self.service_slots = 4 if doc["change"]["subject_role"] == "both" and doc["call_changes"] else 5
        for group in doc["impacts"][:self.service_slots]:
            for row in group["fields"][:3]:
                if key := self.field_key(row):
                    self.shared_fields[key].setdefault(group["operation_id"], row)
        for group in doc["impacts"]:
            for row in group["fields"]:
                if (key := self.field_key(row)) in self.shared_fields:
                    self.shared_fields[key].setdefault(group["operation_id"], row)
        self.appendix = []
        self.limits = Limits()
        self.mark = self.limits.add
        self.receipts = {}
        self.unknown_anchor = "unknown-changed-calls"
        for snapshot in doc["inputs"].values():
            for source in snapshot["config_sources"]:
                key = snapshot["role"], source["file"], source["revision"]
                self.receipts.setdefault(key, f"configuration-{len(self.receipts) + 1}")
        for warning in doc["warnings"]:
            self.limits.warning(warning["reason"], f"{_code(warning['file'])} at {_code(warning['rev'][:8])}")

    @staticmethod
    def field_key(row):
        definitions = tuple(col["definition"] if col else None for col in (row["before"], row["after"]))
        if any(definitions):
            return definitions, row["path"].rsplit("/", 1)[-1], json.dumps([
                row["before_shape"], row["after_shape"],
            ], sort_keys=True)
        return None

    def section(self, title, rows, limit, kind, complete=False, shown=None):
        visible = rows[:limit] if shown is None else shown[:limit]
        if complete:
            return rows, ""
        if len(visible) == len(rows):
            return visible, ""
        anchor = f"report-appendix-{len(self.appendix) + 1}"
        self.appendix.append([anchor, title, None])
        return visible, f"Showing {len(visible)} of {len(rows)} · [See all {kind}](#{anchor})"

    def _fill(self, link, content):
        if link:
            anchor = link.rsplit("#", 1)[1][:-1]
            self.appendix[int(anchor.rsplit("-", 1)[1]) - 1][2] = content

    def list(self, title, values, limit=3, complete=False):
        visible, link = self.section(title, values, limit, title, complete)
        if link:
            self._fill(link, "\n".join(f"- {value}" for value in values))
        return "<br>".join([*visible, *([link] if link else [])])

    def render(self):
        doc = self.doc
        role = doc["change"]["subject_role"]
        direction = {"service": "Service change checked against client code",
                     "client": "Client change checked against service code",
                     "both": "Service and client changed together"}[role]
        operations, client_changes = doc["impacts"], doc["call_changes"]
        start = [f"# {direction} · {MODE}",
                 "An operation is a method and path, such as GET /star-proofs. The service declares it. "
                 "A client call is code that sends a request. {…} stands for one variable part of the address."]
        review = review_first(doc, self.calls)
        summary = self.counts()
        for reason in self.limits.warning_locations:
            self.mark(reason)
        for reason in (EARLIER, GATEWAY, OVERLAPS, doc["unobserved_reason"], *doc["field_limits"]):
            if reason:
                self.mark(reason)
        lines = ["## Coverage", *coverage(doc), "## Changes by operation"]
        if role == "both":
            lines.append("### Service changes")
        if role in {"service", "both"}:
            visible, link = self.section("All operations", operations, self.service_slots, "operations")
            for number, group in enumerate(visible, 1):
                lines.append(self.operation(group, anchor=f"operation-{number}" if link else None))
            if link:
                lines.append(link)
                index, details = [], []
                for number, group in enumerate(operations, 1):
                    anchor = f"operation-{number}"
                    name = group["operation_id"].removeprefix("op:")
                    index.append((f"[{_code(name)}](#{anchor})",))
                    if number > len(visible):
                        content = compact_html(self.operation(group, complete=True))
                        details.append(" ".join(appendix_section(anchor, name, [content]).splitlines()))
                self._fill(link, [compact_html(table(["Operation"], index)), *details])
            if not operations:
                gaps = [row for row in doc["warnings"] if row["input_id"].startswith("service:")]
                lines.append(
                    "No difference among the checked operations. Additional changes " + self.mark(gaps[0]["reason"])
                    if gaps else "No method or path was added, removed or changed in the checked declarations."
                )
                if doc["summary"]["changed_field_count"]["total"] == 0:
                    lines.append("No difference in the measured field declarations.")
        if role in {"client", "both"}:
            if role == "client" and not client_changes:
                self.mark(NO_FIELD_USE)
            lines.extend(['<a id="client-changes"></a>', "### Client changes",
                          *self.client_changes(client_changes, 5 - min(self.service_slots, len(operations)))])
        if role == "both":
            lines.extend(["### Both sides in one change", self.rollout()])
        lines.extend(["## Who calls this code", self.callers(), "## Other differences at these versions", self.standing()])
        self.requests_appendix()
        versions = provenance(doc, self.receipts, self.mark)
        limit_rows = self.limits.render(self.list)
        start.extend(["## Review first", f"{review} {len(limit_rows)} things could not be checked ([see Limits](#limits)).",
                      versions_line(doc), "## Summary", summary, '<a id="limits"></a>', "## Limits"])
        extra = limit_rows[5:]
        start.append(table(["Limit", "What could not be checked"], limit_rows[:5]))
        if extra:
            start.append(f"Showing 5 of {len(limit_rows)} · [See all limits](#all-limits)")
            links = [(f"[L{number}](#L{number})", "See the Limits preview above") for number in range(1, 6)]
            self.appendix.append(["all-limits", "All limits", table(["Limit", "What could not be checked"], [*links, *extra])])
        if self.appendix:
            lines.extend(["## Appendix", "Full operation and client-change tables, request comparisons, caller searches, "
                          "standing differences and the details behind shortened lists."])
            for anchor, title, content in self.appendix:
                rows = [compact_html(content)] if isinstance(content, str) else content
                lines.append(appendix_section(anchor, title, rows))
        return "\n\n".join([*start, *lines, "## Provenance", *versions]) + "\n"

    def counts(self):
        doc = self.doc
        labels = (("operation_count", "Operations changed", "Checked service code and schema"),
                  ("changed_field_count", "Fields changed", "Measured properties in JSON bodies"),
                  ("listed_usage_count", "Client calls affected", "Distinct call sites shown, including possible matches"),
                  ("possible_only_usage_count", "Possible matches only", "Shown calls with possible matches only"))
        rows = [(name, self.count_text(doc["summary"][key]), scope) for key, name, scope in labels]
        for key, name in (("known_unmatched_count", "Requests with no match"),
                          ("unknown_target_count", "Calls with an unknown target")):
            value = " · ".join(f"{side.title()} {self.count_text(doc['request_counts'][side][key], side)}"
                               for side in ("before", "after"))
            scope = ("Readable methods and addresses among client requests checked" if key == "known_unmatched_count"
                     else "Client requests whose method or address could not be read")
            rows.append((name, value, scope))
        return table(["Measure", "Count", "What it covers"], rows)

    def count_text(self, row, purpose=None):
        number = str(row["observed"])
        if purpose:
            number = f"[{number}](#requests-{purpose})"
        if row["total"] is None:
            return f"{number} observed · {self.mark(row['reason'])}"
        return number

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
        actions = [group["review_action"]["text"], *(_RISK_ACTIONS[risk["kind"]] for risk in group["risks"])]
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
            sites = [self.route_site(site, side) for side, entries in group["producer_sites"].items() for site in entries]
            source = self.list("service code locations", sites, complete=complete) or self.mark(
                "source location was unavailable", group["operation_id"].removeprefix("op:"))
            if kind == "Route declaration changed":
                source += "<br>" + self.mark(
                    "Validation or handler edits: Not checked — the two source bodies were not compared.",
                    group["operation_id"].removeprefix("op:"))
            rows.append((kind, self.presence(group["before_presence"]), self.presence(group["after_presence"]), source))
        return rows, whole_operation

    def presence(self, value):
        if value == "unknown":
            return self.mark("operation coverage is incomplete")
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
            parts.append("List item type " + " or ".join(_type(item) for item in shape["items_type"].split(",")))
        if shape["enum"]:
            parts.append("Listed values " + self.list("listed values", [_code(v) for v in shape["enum"]], complete=complete))
        else:
            parts.append("Listed values " + self.mark(
                "Listed values: Not checked — the scan could not distinguish an omitted list from an empty or unreadable one",
                affected))
        return self.list("field properties", parts, complete=complete)

    def request_counts(self, purpose, counts):
        no_match = counts["known_unmatched_count"]
        unknown = counts["unknown_target_count"]
        return (
            f"[{no_match['observed']} requests whose method and address were read had no match](#requests-{purpose})"
            + (f" observed · {self.mark(no_match['reason'])}" if no_match["reason"] else "")
            + f" · [{unknown['observed']} calls had an unknown target](#requests-{purpose})"
            + (f" observed · {self.mark(unknown['reason'])}" if unknown["reason"] else "")
        )

    def operation_calls(self, group, complete):
        chunks = []
        name = group["operation_id"].removeprefix("op:")
        for checked in group["declaration_comparisons"]:
            context = [f"**{checked['side'].title()} requests** · " + self.request_counts(checked["purpose"], checked)]
            if not checked["closest_site_ids"]:
                reason = checked["inventory_reason"] or checked["known_unmatched_count"]["reason"]
                if reason:
                    context.append("Closest matches " + self.mark(reason, name))
                else:
                    context.append(f"None of the {checked['compared']} calls compared had this declaration among its closest matches")
            context.append("Other matching paths " + self.mark(OVERLAPS))
            if checked["inventory_reason"]:
                context.append("Other prefixes " + self.mark("Other prefixes: Not checked — request paths were unavailable", name))
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
        if "service_first" in self.pairs and key in self.pairs["service_first"]["matches"] and not new:
            new = self.pairs["service_first"]["matches"][key]
        risks = [row["kind"] for row in group["risks"] if self.usage_keys.get(row["site_id"], row["site_id"]) == key]
        return self.call_row(call, old, new, " · ".join(risks) or "See below", complete,
                             missing="Corresponding call: Not checked — the scan recorded line numbers only."
                             if "service_first" in self.pairs else None, prior=prior)

    def request(self, call):
        method, original, path = call["requested_method"], call["original_url_text"], call["mapped_path"]
        mapping = call["mapping_source"]
        if not mapping:
            source = "Mapping " + self.mark(MAPPING, site_link(call["site"]))
        elif "builtin" in mapping:
            source = f"built-in {mapping['builtin']} rule"
        else:
            anchor = self.receipts["consumer", mapping["file"], mapping["revision"]]
            source = f"[{_code(mapping['file'])}](#{anchor}) entry {mapping['entry']}"
            if mapping["client_entry"] != mapping["entry"]:
                source += f" · hook entry {mapping['client_entry']}"
            if mapping["delegated_entry"] and mapping["delegated_entry"] != mapping["entry"]:
                source += f" · delegated entry {mapping['delegated_entry']}"
        if original.strip('"\'\x60') == path and not (mapping or {}).get("add_prefix"):
            source += " · path unchanged"
        return f"{method or 'Method unknown'} {_code(original)} → {_code(path)} · {source}".replace("\n", " ")

    def match_cell(self, match, complete, fallback=None):
        if not match:
            return self.mark(fallback) if fallback else match_text(None)
        basic = match_text(match)
        targets = match["target_ids"]
        if len(targets) > 3:
            shown = self.list("matching operations", [x.removeprefix("op:") for x in targets], complete=complete)
            basic = ("Possible match — several declarations fit: " if match["status"] == "possible" else
                     "Closest matches among the declarations read: ") + shown
            if match["status"] != "possible":
                basic += f". Not checked — {match['reason']}"
        if match["status"] == "no_match":
            basic = basic.removesuffix(GATEWAY + ".") + "Registration " + self.mark(GATEWAY)
        elif match["status"] == "partial":
            reason = match["reason"]
            file, separator, detail = reason.partition(": ")
            location = f"{_code(file)} at {_code(match['service_rev'][:8])}" if separator else None
            basic = basic.rsplit("Not checked — ", 1)[0] + self.mark(detail if separator else reason, location)
        elif match["status"] == "not_checked":
            basic = self.mark(match["reason"])
        alternates = [
            row["operation"].removeprefix("op:") +
            (" might fit if the address value agrees" if row["grade"] == "candidate" else
             f" ({row['reason']})" if row["grade"] is None else " also listed at this path")
            for row in match["alternates"]
        ]
        if alternates:
            basic += "<br>Other methods · " + self.list("other methods", alternates, complete=complete)
        return f"Client {_code(match['client_rev'][:8])} with service {_code(match['service_rev'][:8])} · {basic}"

    def call_row(self, call, old, new, action, complete=False, missing=None, prior=None):
        where = _site(call["site"]) + " · " + (_code(call["enclosing_function"]) if call["enclosing_function"] else
                    "Enclosing function " + self.mark("Enclosing function: Not checked — the scan recorded no function name",
                                                       site_link(call["site"])))
        request = self.request(call)
        if prior:
            before_site = prior["site"]
            if before_site["text"] == call["site"]["text"]:
                before_site = dict(before_site, text=None)
            where = "Before · " + _site(before_site) + "<br>After · " + where
            request = "Before · " + self.request(prior) + "<br>After · " + request
        return where, request, self.match_cell(old, complete, missing), self.match_cell(new, complete, missing), action

    def changed_call(self, edit, complete):
        a, b = edit["before_site_id"], edit["after_site_id"]
        old, new = self.pairs["before"]["matches"].get(a), self.pairs["after"]["matches"].get(b)
        current, prior = self.calls[b or a], self.calls[a] if a and b else None
        action = " · ".join(part for part in (edit["kind"], edit["risk"]) if part)
        if edit["reason"]:
            action += " · " + self.mark(edit["reason"], site_link(current["site"]))
        if edit["remaining_after_site_ids"]:
            action += "<br>Other calls after this edit · " + self.list("remaining calls", [
                _site(self.calls[key]["site"]) for key in edit["remaining_after_site_ids"]
            ], complete=complete)
        return self.call_row(current, old, new, action, complete, edit["reason"], prior)

    def client_changes(self, edits, limit):
        if not edits:
            gap = next((row for row in self.doc["warnings"] if row["input_id"].startswith("client:")), None)
            return ["Additional client calls " + self.mark(gap["reason"]) if gap else
                    "No client calls were added, removed or retargeted in the checked source."]
        groups = defaultdict(list)
        for edit in edits:
            method, path = edit["requested_target"]
            groups[f"{method} {path}" if method and path else _UNKNOWN].append(edit)
        visible, link = self.section("All client changes by target", list(groups), limit, "client targets")
        if _UNKNOWN in groups:
            self.mark("The method or address of some changed calls could not be read",
                      f"[{_UNKNOWN}](#{self.unknown_anchor})")

        anchors = {name: self.unknown_anchor if name == _UNKNOWN else f"client-target-{i}"
                   for i, name in enumerate(groups, 1)}

        def group(name, complete=False):
            head = f'#### {name} <a id="{anchors[name]}"></a>'
            rows, row_link = self.section("Calls for " + name, groups[name], 3, "calls", complete)
            content = table(_CLIENT, [self.changed_call(edit, complete) for edit in rows])
            if row_link:
                self._fill(row_link, table(_CLIENT, [self.changed_call(edit, True) for edit in groups[name]]))
            actions = [edit["review_action"] for edit in groups[name]]
            return "\n\n".join(filter(None, (head, content, row_link, "**What to check** · " +
                                                " ".join(dict.fromkeys(actions)))))

        result = [group(key) for key in visible]
        if link:
            index = table(["Client target"], [(f"[{name}](#{anchors[name]})",) for name in groups])
            self._fill(link, [compact_html(index), *(compact_html(group(key, True)) for key in groups if key not in visible)])
            result.append(link)
        if self.doc["change"]["subject_role"] == "client":
            matched = any(row["field_use"]["reason"] == FIELD_USE for row in self.doc["usages"])
            result.append("Client field use · " + self.mark(FIELD_USE if matched else NO_FIELD_USE))
        return result

    def rollout(self):
        rows = []
        interesting = {key for edit in self.doc["call_changes"] for key in
                       (edit["before_site_id"], edit["after_site_id"]) if key}
        interesting.update(key for group in self.doc["impacts"] for key in group["client_site_ids"])
        for key, pair in self.pairs.items():
            matches = [(site_id, match) for site_id, match in pair["matches"].items() if site_id in interesting]
            visible, link = self.section(_PAIR_LABELS[key], matches, 3, "request comparisons")

            def comparison_rows(items, complete):
                return [(site_link(self.calls[site_id]["site"]), self.match_cell(match, complete))
                        for site_id, match in items]

            headers = ["Client call", "Match"]
            if link:
                self._fill(link, table(headers, comparison_rows(matches, True)))
            entries = [f"{source} · {match}" for source, match in comparison_rows(visible, False)]
            if link:
                entries.append(link)
            rows.append((_PAIR_LABELS[key], "<br>".join(entries) or "No affected request recorded for this pair",
                         self.request_counts(key, self.doc["request_counts"][key])))
        check = "Check whether the service and client deploy together or need an interval supporting both versions."
        return table(["Compared versions", "What the reviewer sees", "Requests checked"], rows) + "\n\n" + check

    def callers(self):
        # Reserve the caller evidence for the lead operation or client change.
        keys = []
        if self.doc["impacts"]:
            lead = self.doc["impacts"][0]
            keys.extend([*(risk["site_id"] for risk in lead["risks"]), *lead["client_site_ids"]])
        elif self.doc["call_changes"]:
            lead = self.doc["call_changes"][0]
            keys.append(lead["after_site_id"] or lead["before_site_id"])
        keys = [self.usage_keys.get(key, key) for key in keys]
        keys = list(dict.fromkeys([*keys, *self.usages]))
        if not keys:
            return "Caller search " + self.mark("caller search did not run at this revision for a report row")
        visible, link = self.section("All caller results", keys, 5, "caller results")

        def caller_rows(items, complete):
            rows = []
            for key in items:
                call = self.calls[key]
                if call["callers_status"] != "complete":
                    result = self.mark(call["callers_reason"], site_link(call["site"]))
                else:
                    result = (self.list("possible callers", [_site(site) for site in call["callers"]], complete=complete)
                              if call["callers"] else "No direct call found by this name in the searched files")
                    result += "<br>Searched " + self.list("searched files", [_code(p) for p in call["caller_scope"]],
                                                          complete=complete)
                rows.append((site_link(call["site"]) + f" · {_code(call['site']['rev'][:8])}", result))
            return rows

        headers = ["Client call", "Possible caller found by name"]
        if link:
            self._fill(link, table(headers, caller_rows(keys, True)))
        return "\n\n".join(filter(None, (table(headers, caller_rows(visible, False)), link,
                                           "Other caller paths " + self.mark("the search followed direct names only"))))

    def requests_appendix(self):
        for purpose, counts in self.doc["request_counts"].items():
            keys = [*counts["no_match_site_ids"], *counts["unknown_site_ids"]]
            rows = [(_site(self.calls[key]["site"], excerpt=False), self.request(self.calls[key]), self.match_cell(
                self.pairs[purpose]["matches"][key], True)) for key in keys]
            heading = _PAIR_LABELS[purpose] if "service_first" in self.pairs else purpose.title()
            content = (table(_CLIENT[:2] + ["Match"], rows) if rows else
                       "No unmatched request or unknown target was observed among the rows checked.")
            self.appendix.append([f"requests-{purpose}", f"Requests compared · {heading}", content])

    def standing(self):
        rows = []
        for mismatch in self.doc["standing_mismatches"]:
            site_id = mismatch["after_site_id"] or mismatch["before_site_id"]
            source = (_site(self.calls[site_id]["site"]) if site_id else
                      _code(mismatch["operation_id"].removeprefix("op:")))
            kind, timing = mismatch["kind"], mismatch["timing"]
            if kind.endswith(GATEWAY + "."):
                kind = kind.removesuffix(GATEWAY + ".") + "Registration " + self.mark(GATEWAY, source)
            observed, separator, reason = timing.partition("When this difference began: Not checked — ")
            if separator:
                timing = observed + "When this difference began " + self.mark(reason, source)
            rows.append(((source, kind, timing), mismatch["linked_to_change"]))
        if not rows:
            return "No difference of this kind was observed among the declarations and calls read."
        all_rows = [row for row, _ in rows]
        preview = [row for row, linked in rows if linked][:5]
        if len(preview) < len(all_rows):
            anchor = "report-standing-differences"
            self.appendix.append([anchor, "All standing differences", table(["Where", "What was found", "When observed"], all_rows)])
            link = f"Showing {len(preview)} of {len(all_rows)} · [See all standing differences](#{anchor})"
        else:
            link = ""
        return (table(["Where", "What was found", "When observed"], preview) + "\n\n" if preview else "") + link


def render_markdown(document):
    return Markdown(document).render()


def render_json(document):
    return json.dumps(document, ensure_ascii=False, sort_keys=True, indent=2) + "\n"
