"""Render the live report and its complete tables from the version-2 document."""

from datetime import timedelta
from collections import defaultdict
import json
import re

from .report_analysis import EARLIER, FIELD_USE, NO_FIELD_USE
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


def _code(text):
    if text is None or text == "":
        return "—"
    delimit = "`" * (max((len(part) for part in re.findall(r"`+", str(text))), default=0) + 1)
    if str(text).startswith("`") or str(text).endswith("`"):
        text = f" {text} "
    return f"{delimit}{text}{delimit}"


def table(headers, rows):
    def line(items):
        return "| " + " | ".join(str(item).replace("|", "&#124;").replace("\n", "<br>") for item in items) + " |"
    return "\n".join([line(headers), line(["---"] * len(headers)), *(line(row) for row in rows)])


def _site(site, kind=None):
    if not site:
        return "Not checked — source location was unavailable"
    location = _code(f"{site['file']}:{site['line']}")
    if site.get("url"):
        location = f"[{location}]({site['url']})"
    version = _code(site["rev"][:8])
    excerpt = _code(site.get("text")) if site.get("text") else ""
    return " · ".join(part for part in (kind, location, version, excerpt) if part)


def _declaration(site):
    if site["kind"] == "annotation":
        function = "Handler named in a comment " + _code(site["function"])
        return _site(site, "Route annotation") + " · " + function
    handler = (_site(site["handler_site"], "Handler") if site["handler_site"] else
               "Handler location: Not checked — only the declaration was found")
    return _site(site, "Route declaration") + " · " + handler


def _type(value):
    if value.startswith("["):
        return " or ".join(_type(item) for item in json.loads(value))
    if not value:
        return "No type was recorded"
    return _TYPES.get(value, f"{_code(value)} (meaning: Not checked)")


def _format(value):
    if not value:
        return "No format was recorded"
    meaning = _FORMATS.get(value)
    return f"{_code(value)} ({meaning})" if meaning else (
        f"{_code(value)}. Not checked — the schema gives no meaning the reader recognizes for this format."
    )


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
        for group in doc["impacts"][:5]:
            for row in group["fields"][:3]:
                if key := self.field_key(row):
                    self.shared_fields[key].setdefault(group["operation_id"], row)
        for group in doc["impacts"]:
            for row in group["fields"]:
                if (key := self.field_key(row)) in self.shared_fields:
                    self.shared_fields[key].setdefault(group["operation_id"], row)
        self.appendix = []

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
        self._fill(link, "\n".join(f"- {value}" for value in values))
        return "<br>".join([*visible, *([link] if link else [])])

    def render(self):
        doc = self.doc
        role = doc["change"]["subject_role"]
        direction = {"service": "Service change checked against client code",
                     "client": "Client change checked against service code",
                     "both": "Service and client changed together"}[role]
        operations, client_changes = doc["impacts"], doc["call_changes"]
        lines = [f"# {direction} · Comparison of selected versions",
                 "An operation is a method and path, such as GET /star-proofs. The service declares it. "
                 "A client call is code that sends a request.",
                 "## Review first", self.review(), *self.counts(), "## Versions checked", *self.versions(),
                 "## Coverage", *self.coverage(), "## Changes by operation"]
        if role == "both":
            lines.append("### Service changes")
        if role in {"service", "both"}:
            visible, link = self.section("All operations", operations, 5, "operations")
            lines.extend(self.operation(group) for group in visible)
            if link:
                lines.append(link)
                self._fill(link, "\n\n".join(self.operation(group, complete=True) for group in operations))
            if not operations:
                gaps = [row for row in doc["warnings"] if row["input_id"].startswith("service:")]
                lines.append(
                    f"No difference among the declarations read. Additional changes: Not checked — {gaps[0]['reason']}."
                    if gaps else "No method or path was added, removed or changed in the checked declarations."
                )
                if doc["summary"]["changed_field_count"]["total"] == 0:
                    lines.append("No difference in the measured field declarations.")
        if role in {"client", "both"}:
            lines.extend(['<a id="client-changes"></a>', "### Client changes", *self.client_changes(client_changes)])
        if role == "both":
            lines.extend(["### Both sides in one change", self.rollout()])
        lines.extend(["## Who calls this code", self.callers(), "## Limits of this check", self.limits(),
                      "## Other differences at these versions", self.standing()])
        if self.appendix:
            lines.append("## Appendix")
            for anchor, title, content in self.appendix:
                lines.extend([f'<a id="{anchor}"></a>', f"### {title}", content])
        return "\n\n".join(lines) + "\n"

    def review(self):
        groups, edits, warnings = self.doc["impacts"], self.doc["call_changes"], self.doc["warnings"]
        if warnings:
            first = warnings[0]
            prefix = f"Not checked — {_code(first['file'])} at {_code(first['rev'][:8])}: {first['reason']}. "
        else:
            prefix = ""
        if self.doc["change"]["subject_role"] == "both":
            affected = {key for edit in edits for key in (edit["before_site_id"], edit["after_site_id"]) if key}
            affected.update(key for group in groups for key in group["client_site_ids"])
            gaps = [label for purpose, label in (("service_first", "service ships first"),
                                                 ("client_first", "client ships first"))
                    if any(key in affected and match["status"] == "no_match"
                           for key, match in self.pairs[purpose]["matches"].items())]
            finding = ("A request has no match among the declarations read when the " + " or the ".join(gaps) + ". "
                       if gaps else "The rollout table compares both edits and each side shipping alone. ")
            return prefix + finding + "Check whether they deploy together or need an interval supporting both versions."
        if groups:
            group = groups[0]
            name = group["operation_id"].removeprefix("op:")
            finding = (group["risks"][0]["kind"] if group["risks"] else
                       group["change_kinds"][0] if group["change_kinds"] else group["limits"][0])
            if finding.startswith("Listed only"):
                own = (group["producer_sites"]["before"] or group["producer_sites"]["after"])[0]
                finding += " at " + _site(own) + ". " + group["limits"][0]
            return f"{prefix}{finding} · {_code(name)}. {group['review_action']['text']}"
        if edits:
            edit = edits[0]
            call = self.calls[edit["after_site_id"] or edit["before_site_id"]]
            return f"{prefix}{edit['risk'] or edit['kind']} · {_site(call['site'])}. {edit['review_action']}"
        if prefix:
            return prefix + "Check this source before relying on the comparison."
        return "The checked source shows no measured change. Check any behavior outside the scope below."

    def versions(self):
        doc = self.doc
        change = doc["change"]
        before = doc["inputs"][change["before_input_id"]["service"]]
        after = change["subject"]
        driver = before["tool"]
        result = [
            f"**Subject** · {_code(after['rev'])} · {after['committed_at']} · {after['subject']}",
            f"**Repository** · {_code(before['repo'])}",
            f"**Before** · {_code(before['rev'])} · {before['committed_at']} · first parent · tree {_code(before['tree'])}",
            f"**After** · {_code(after['rev'])} · {after['committed_at']} · selected commit · tree {_code(after['tree'])}",
            f"**Selection time** · {doc['report_time'] or 'Not checked — no selection timestamp was supplied'}",
        ]
        reference = doc["reference"]
        counterpart = doc["inputs"][reference["reference_input_id"]]
        delta = reference["duration_seconds"]
        label = ("Both commits have the same time" if delta == 0 else
                 f"Checked revision is {timedelta(seconds=abs(delta))} {'earlier than' if delta < 0 else 'after'} "
                 "the change")
        role = "Service" if counterpart["role"] == "producer" else "Client"
        result.append(f"**{role} held at the base** · {_code(counterpart['rev'])} · {counterpart['committed_at']}. {label}.")
        sources = set()
        for snapshot in doc["inputs"].values():
            for source in snapshot["config_sources"]:
                identity = (snapshot["role"], source["file"], source["revision"])
                if identity in sources:
                    continue
                sources.add(identity)
                kind = "Client configuration" if snapshot["role"] == "consumer" else "Route specification"
                result.append(f"**{kind}:** {_code(source['file'])} from selection receipt "
                              f"{_code(source['revision'])}, supplied by the study.")
        result.append(
            f"**Driver revision** · {_code(driver['rev'])} · source receipt {_code(driver['build_identity'])}. "
            "The configuration era comes from each selected client revision’s first-parent ancestry. "
            "The configured request.ts client is primary when present in the configuration."
        )
        return result

    def coverage(self):
        doc = self.doc
        change = doc["change"]
        service = doc["inputs"][change["before_input_id"]["service"]]
        client = doc["inputs"][change["before_input_id"]["client"]]
        result = [
            f"**Source of operations** · {service['operation_authority']} under {_code(service['scope'])}. "
            "The source locations identify declarations in the checked code.",
            f"**Client scope** · production JavaScript and TypeScript under {_code(client['scope'])}. "
            "Test files are excluded.",
            f"**Schema path** · {_code(service['schema']['source']['path'])}. "
            "Named fields in JSON request bodies and successful JSON response bodies. Compared type, format, "
            "required in its parent, listed values and list item type. Parameters, non-success responses, "
            "other media and constraints such as numeric bounds are outside this count.",
            "We compare the HTTP method and the path after applying the address mapping shown here. "
            "Among paths that fit, the check prefers those with more matching fixed or variable parts. "
            "{…} stands for one variable part of the address.",
        ]
        if doc["code_changed_with_unchanged_schema"]:
            result.append("The code changed. The checked-in schema file did not change. "
                          "Whether it describes this edit is unknown.")
        return result

    def counts(self):
        doc = self.doc
        labels = (("operation_count", "Operations"), ("changed_field_count", "Changed fields"),
                  ("listed_usage_count", "Client calls shown"), ("possible_only_usage_count", "Possible matches only"))
        result = [f"**{label}** · {_count_text(doc['summary'][key])}" for key, label in labels]
        result.extend(f"**{label}** · " + self.request_counts(purpose, counts)
                      for purpose, counts in doc["request_counts"].items()
                      for label in [_PAIR_LABELS[purpose] if doc["change"]["subject_role"] == "both" else purpose.title()])
        return result

    def operation(self, group, complete=False):
        identifier = group["operation_id"]
        method, path = parts(identifier)
        old, new = group["producer_sites"]["before"], group["producer_sites"]["after"]
        source = self.list("service code locations", [*(_declaration(s) for s in old), *(_declaration(s) for s in new)],
                           complete=complete) or "Not checked — declaration location was unavailable"
        kinds = group["change_kinds"]
        rows = [(self.list("changes", kinds, complete=complete) if kinds else "Declaration comparison",
                 _presence(group["before_presence"]), _presence(group["after_presence"]), source)]
        if "Route declaration changed" in kinds:
            rows.append(("Retained declaration", "Entry text read", "Entry text read",
                         "Validation or handler edits: Not checked — the two source bodies were not compared."))
        for status in group["status_removals"]:
            rows.append((f"Successful response status removed · {status['status']}", "Listed", "Absent",
                         _site(status["site"], "Schema response status")))
        for status in group["status_additions"]:
            rows.append((f"Successful response status added · {status['status']}", "Absent", "Listed",
                         _site(status["site"], "Schema response status")))
        chunks = [f"#### {_code(f'{method} {path}')}"]
        if "{}" in path:
            chunks.append("{…} stands for one variable part of the address.")
        if method == "*":
            chunks.append("This declaration lists every method. " + WILDCARD)
        if group["limits"]:
            chunks.append(self.list("limits for this operation", group["limits"], complete=complete))
        if group.get("message_coverage"):
            bodies = []
            for record in group["message_coverage"]:
                message = ("request body" if record["root"].startswith("req:") else
                           "response body " + record["root"].rsplit(":", 1)[1])
                bodies.append(f"{record['side'].title()} {message} · application/json from {record['media_source']}.")
            chunks.append(self.list("message coverage", bodies, complete=complete))
        for pair in group["response_root_pairs"]:
            chunks.append(f"**Successful response status changed, {pair['before_status']} → {pair['after_status']}**")
        visible, link = self.section("change rows", rows, 3, "change rows", complete)
        self._fill(link, table(_SERVICE, rows))
        chunks.extend([table(_SERVICE, visible), link])
        if group["fields"]:
            distinct = [row for row in group["fields"] if next(
                iter(self.shared_fields.get(self.field_key(row), {})), identifier) == identifier]
            fields, link = self.section("fields for " + method + " " + path, group["fields"], 3, "fields",
                                        complete, shown=distinct)
            if fields:
                chunks.append(table(_SERVICE, [self.field_row(group, row, complete) for row in fields]))
            if link:
                self._fill(link, table(_SERVICE, [self.field_row(group, row, True) for row in group["fields"]]))
                chunks.append(link)
            linked = any(group["operation_id"] in match["target_ids"] and match["status"] in {"match", "possible"}
                         for side in ("before", "after") for match in self.pairs[side]["matches"].values())
            chunks.append(FIELD_USE if linked else NO_FIELD_USE)
        chunks.extend(self.operation_calls(group, complete))
        if any(change["kind"] == "Client call removed" for change in group["call_changes"]):
            chunks.append("The checked after client no longer has this call at its before site. "
                          "[See client changes](#client-changes).")
        chunks.append(group["review_action"]["text"])
        return "\n\n".join(part for part in chunks if part)

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
        sites.append("Field location: Not checked — only the operation was located" if len(sites) else
                     "Field location: Not checked — the schema operation site was unavailable")
        same = self.shared_fields.get(self.field_key(row), {})
        if not complete and len(same) > 1:
            changes.append(f"Same change in {len(same) - 1} more operations")
        return (_field_label(row["after"] or row["before"]) + " · " +
                self.list("properties changed", changes, complete=complete),
                self.shape(previous, complete), self.shape(following, complete),
                self.list("field source locations", sites, complete=complete))

    def shape(self, shape, complete):
        if shape is None:
            return "Absent"
        parts = [_type(shape["type"]),
                 "Required when its parent is present" if shape["required"] else "Optional in its parent"]
        if shape["format"]:
            parts.append("Format " + _format(shape["format"]))
        if shape["items_type"]:
            parts.append("List item type " + " or ".join(_type(item) for item in shape["items_type"].split(",")))
        if shape["enum"]:
            parts.append("Listed values " + self.list("listed values", [_code(v) for v in shape["enum"]], complete=complete))
        else:
            parts.append("Listed values: Not checked — the scan could not distinguish an omitted list from an empty or unreadable one")
        return self.list("field properties", parts, complete=complete)

    def request_counts(self, purpose, counts):
        no_match = counts["known_unmatched_count"]
        unknown = counts["unknown_target_count"]
        return (
            f"[{no_match['observed']} requests whose method and address were read had no match](#requests-{purpose})"
            + (f". Unmatched requests: Not checked — {no_match['reason']}" if no_match["reason"] else "")
            + f" · [{unknown['observed']} calls had an unknown target](#requests-{purpose})"
            + (f". Total: Not checked — {unknown['reason']}" if unknown["reason"] else "")
        )

    def operation_calls(self, group, complete):
        chunks = []
        for checked in group["declaration_comparisons"]:
            chunks.append(f"**{checked['side'].title()} requests** · " + self.request_counts(checked["purpose"], checked))
            if not checked["closest_site_ids"]:
                if checked["inventory_reason"] or checked["known_unmatched_count"]["reason"]:
                    chunks.append("Closest matches: Not checked — " +
                                  (checked["inventory_reason"] or checked["known_unmatched_count"]["reason"]))
                else:
                    chunks.append(f"None of the {checked['compared']} calls compared had this declaration among its closest matches.")
            chunks.append(OVERLAPS + ".")
            if checked["inventory_reason"]:
                chunks.append("Other prefixes: Not checked — request paths were unavailable")
        occurrences = [*(risk["site_id"] for risk in group["risks"]), *group["client_site_ids"]]
        keys = list(dict.fromkeys(self.usage_keys.get(key, key) for key in occurrences))
        if keys:
            visible, link = self.section("calls for " + group["operation_id"].removeprefix("op:"), keys, 3, "calls", complete)
            chunks.append(table(_CLIENT, [self.operation_call(group, key, complete) for key in visible]))
            if link:
                self._fill(link, table(_CLIENT, [self.operation_call(group, key, True) for key in keys]))
                chunks.append(link)
        if group["prefix_hints"]:
            hints = group["prefix_hints"]
            chunks.append(self.list("other-prefix hints", [
                f"Possible match under another prefix — both addresses end in {hint['common_suffix']}. "
                f"Check the leading parts. {_site(self.calls[hint['site_id']]['site'])}. "
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
        return self.call_row(call, old, new, " · ".join([*risks, group["review_action"]["text"]]), complete,
                             missing="Corresponding call: Not checked — the scan recorded line numbers only."
                             if "service_first" in self.pairs else None, prior=prior)

    def request(self, call):
        method, original, path = call["requested_method"], call["original_url_text"], call["mapped_path"]
        mapping = call["mapping_source"]
        if not mapping:
            source = MAPPING
        elif "builtin" in mapping:
            source = f"built-in {mapping['builtin']} rule"
        else:
            source = f"{_code(mapping['file'])} entry {mapping['entry']} at {_code(mapping['revision'])}"
            if mapping["client_entry"] != mapping["entry"]:
                source += f" · hook entry {mapping['client_entry']}"
            if mapping["delegated_entry"] and mapping["delegated_entry"] != mapping["entry"]:
                source += f" · delegated entry {mapping['delegated_entry']}"
        if original.strip('"\'\x60') == path and not (mapping or {}).get("add_prefix"):
            source += " · path unchanged"
        return f"{method or 'Method unknown'} {_code(original)} → {_code(path)} · {source}"

    def match_cell(self, match, complete, fallback=None):
        if not match:
            return fallback or match_text(None)
        basic = match_text(match)
        targets = match["target_ids"]
        if len(targets) > 3:
            shown = self.list("matching declarations", [x.removeprefix("op:") for x in targets], complete=complete)
            basic = ("Possible match — several declarations fit: " if match["status"] == "possible" else
                     "Closest matches among the declarations read: ") + shown
            if match["status"] != "possible":
                basic += f". Not checked — {match['reason']}"
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
                    "Enclosing function: Not checked — the scan recorded no function name")
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
        action = " · ".join(part for part in (edit["kind"], edit["risk"], edit["reason"], edit["review_action"]) if part)
        if edit["remaining_after_site_ids"]:
            action += "<br>Other calls after this edit · " + self.list("remaining calls", [
                _site(self.calls[key]["site"]) for key in edit["remaining_after_site_ids"]
            ], complete=complete)
        return self.call_row(current, old, new, action, complete, edit["reason"], prior)

    def client_changes(self, edits):
        if not edits:
            gaps = [row for row in self.doc["warnings"] if row["input_id"].startswith("client:")]
            return [f"Additional client calls: Not checked — {gaps[0]['reason']}." if gaps else
                    "No client calls were added, removed or retargeted in the checked source."]
        groups = {}
        for edit in edits:
            method, path = edit["requested_target"]
            key = f"{method} {path}" if method and path else "Changed client calls with an unknown target"
            groups.setdefault(key, []).append(edit)
        result = []
        visible, link = self.section("client changes by target", list(groups), 5, "client targets")
        def group(label, complete=False):
            rows, row_link = self.section("client calls for " + label, groups[label], 3, "calls", complete)
            content = table(_CLIENT, [self.changed_call(edit, complete) for edit in rows])
            if row_link:
                self._fill(row_link, table(_CLIENT, [self.changed_call(edit, True) for edit in groups[label]]))
            css_id = '<a id="unknown-changed-calls"></a>\n\n' if label.startswith("Changed client") else ""
            return f"{css_id}#### {label}\n\n{content}" + (f"\n\n{row_link}" if row_link else "")
        result.extend(group(key) for key in visible)
        if link:
            self._fill(link, "\n\n".join(group(key, True) for key in groups))
            result.append(link)
        return result

    def rollout(self):
        rows = []
        interesting = {key for edit in self.doc["call_changes"] for key in
                       (edit["before_site_id"], edit["after_site_id"]) if key}
        interesting.update(key for group in self.doc["impacts"] for key in group["client_site_ids"])
        for key, pair in self.pairs.items():
            matches = [match for site_id, match in pair["matches"].items() if site_id in interesting]
            entries = [
                _site(self.calls[site_id]["site"]) + " · " + self.match_cell(match, False)
                for site_id, match in pair["matches"].items() if site_id in interesting
            ]
            rows.append((_PAIR_LABELS[key], self.list("request comparisons", entries) if matches else
                         "No affected request recorded for this pair"))
        return table(["Compared versions", "What the reviewer sees"], rows)

    def callers(self):
        keys = list(dict.fromkeys(row["site_id"] for row in self.doc["usages"]))
        if not keys:
            return "Not checked — caller search did not run at this revision for a report row."
        rows = []
        for key in keys:
            call = self.calls[key]
            if call["callers_status"] != "complete":
                result = f"Not checked — {call['callers_reason']}."
            else:
                result = (self.list("possible callers", [_site(site) for site in call["callers"]])
                          if call["callers"] else "No direct call found by this name in the searched files")
                result += "<br>Searched " + self.list("searched files", [_code(p) for p in call["caller_scope"]])
            rows.append((_site(call["site"]), result))
        visible, link = self.section("caller results", rows, 5, "caller results")
        self._fill(link, table(["Client call", "Possible caller found by name"], rows))
        return table(["Client call", "Possible caller found by name"], visible) + (
            "\n\n" + link if link else "") + "\n\nNot checked — the search followed direct names only."

    def limits(self):
        limits = [EARLIER, GATEWAY, OVERLAPS]
        if self.doc["unobserved_reason"]:
            limits.append(self.doc["unobserved_reason"])
        for warning in self.doc["warnings"]:
            limits.append(f"Not checked — {_code(warning['file'])} at {_code(warning['rev'][:8])}: {warning['reason']}.")
        if self.doc["change"]["subject_role"] == "client":
            used = any(row["field_use"]["reason"] == FIELD_USE for row in self.doc["usages"])
            limits.append(FIELD_USE if used else NO_FIELD_USE)
        if any(not all(row["requested_target"]) for row in self.doc["call_changes"]):
            limits.append("[Changed client calls with an unknown target](#unknown-changed-calls)")
        records = list(dict.fromkeys([*limits, *self.doc["field_limits"]]))
        result = self.list("limits", records, limit=5)
        for purpose, counts in self.doc["request_counts"].items():
            heading = f'<a id="requests-{purpose}"></a>'
            keys = [*counts["no_match_site_ids"], *counts["unknown_site_ids"]]
            rows = [(_site(self.calls[key]["site"]), self.request(self.calls[key]), self.match_cell(
                self.pairs[purpose]["matches"][key], True)) for key in keys]
            if rows:
                anchor = f"requests-{purpose}"
                self.appendix.append([anchor, f"Requests compared · {purpose}",
                                      table(_CLIENT[:2] + ["Match among the declarations read"], rows)])
            else:
                result += "\n\n" + heading + f"\n\n**Requests compared · {purpose}** · No such request observed."
        return result

    def standing(self):
        rows = []
        for mismatch in self.doc["standing_mismatches"]:
            site_id = mismatch["after_site_id"] or mismatch["before_site_id"]
            source = (_site(self.calls[site_id]["site"]) if site_id else
                      _code(mismatch["operation_id"].removeprefix("op:")))
            rows.append(((source, mismatch["kind"], mismatch["timing"]), mismatch["linked_to_change"]))
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


def _presence(value):
    return {"present": "Listed", "absent": "Absent from the checked declarations",
            "unknown": "Not checked — declaration coverage is incomplete"}[value]


def _count_text(row):
    if row["total"] is None:
        return f"{row['observed']} observed. Total: Not checked — {row['reason']}."
    return f"{row['total']} · {row['scope']}."


def render_markdown(document):
    return Markdown(document).render()


def render_json(document):
    return json.dumps(document, ensure_ascii=False, sort_keys=True, indent=2) + "\n"
