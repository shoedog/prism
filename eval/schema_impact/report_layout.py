"""Shared Markdown formatting and the numbered limits for a review."""

from datetime import timedelta
from html import escape
import re

from markdown_it import MarkdownIt


_GFM = MarkdownIt("commonmark", {"html": True}).enable("table")


def compact_html(markdown):
    """Keep long appendix tables readable when rendered with few source lines."""
    return " ".join(_GFM.render(markdown).splitlines())


def appendix_section(anchor, title, rows):
    return "\n".join([f'<div id="{anchor}"><details><summary>{escape(title)}</summary>',
                      *rows, "</details></div>"])


class Limits:
    def __init__(self):
        self.entries = {}
        self.warning_locations = {}

    def warning(self, reason, where):
        self.warning_locations.setdefault(reason.rstrip("."), {}).setdefault(where, None)

    def add(self, reason, where=None):
        # Schema readers supply the affected operation/body before the diagnostic.
        if " · Not checked — " in reason:
            source, reason = reason.rsplit(" · Not checked — ", 1)
            operation, separator, body = source.partition(" · ")
            if separator and body.startswith(operation + " "):
                source = body
            where = f"{source} · {where}" if where and not source.startswith(where + " ") else source
        elif " · JSON fields: Not checked — " in reason:
            source, reason = reason.rsplit(" · JSON fields: Not checked — ", 1)
            reason = "JSON fields: Not checked — " + reason
            where = f"{source} · {where}" if where else source
        reason = reason.rstrip(".")
        if reason.startswith("Not checked — "):
            reason = reason.removeprefix("Not checked — ")
        anchor, locations = self.entries.setdefault(reason, (f"L{len(self.entries) + 1}", {}))
        locations.update(self.warning_locations.get(reason, {}))
        if where:
            locations.setdefault(where, None)
        return f"not checked [{anchor}](#{anchor})"

    def render(self, list_preview):
        rows = []
        for reason, (anchor, locations) in self.entries.items():
            text = reason if "Not checked" in reason else f"Not checked — {reason}"
            affected = list_preview(f"Sites for {anchor}", list(locations))
            rows.append((f'<a id="{anchor}"></a>**{anchor}**', text + "." + (f"<br>{affected}" if affected else "")))
        return rows


def label(value):
    """Plain Markdown text for a commit subject or another one-line label."""
    return re.sub(r"([\\`*_\[\]<>])", r"\\\1", " ".join(value.split()))


MODE = "Comparison of selected versions"


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


def review_first(doc, calls):
    groups, edits = doc["impacts"], doc["call_changes"]
    if groups:
        group = groups[0]
        target = _code(group["operation_id"].removeprefix("op:"))
        kind = next(iter(group["change_kinds"]), "")
        finding = {
            "Operation added": f"The service adds {target}",
            "Operation removed": f"The service removes {target} from the checked operations",
            "Operation fields changed": f"The schema changes the fields for {target}",
            "Successful response statuses changed": f"The schema changes the success statuses for {target}",
            "Route declaration changed": f"The route entry for {target} changes",
        }.get(kind, f"The checked code shows {target} only {'before' if group['before_presence'] == 'present' else 'after'} the change"
              if kind.startswith("Listed only") else f"The comparison for {target} has incomplete evidence")
        ids = list(dict.fromkeys([*(risk["site_id"] for risk in group["risks"]), *group["client_site_ids"]]))
        if ids:
            site = calls[ids[0]]["site"]
            location = site_link(site)
            risk = group["risks"][0]["kind"] if group["risks"] else None
            context = {
                "Client calls a missing route": "which loses its match" + (
                    " if the service ships first" if doc["change"]["subject_role"] == "both" else " in the checked operations"),
                "Existing client call gains a matching declaration": "which gains a match in the checked operations",
                "Client target needs confirmation": "whose target needs confirmation",
            }.get(risk, "which appears in the client comparisons below")
            action = group["review_action"]["text"].rstrip(".")
            return f"{finding}. For the call at {location} {context}, {action[:1].lower()}{action[1:]}."
        action = group["review_action"]["text"]
        checked = group["declaration_comparisons"]
        if checked and all(not row["closest_site_ids"] and not row["known_unmatched_count"]["reason"] for row in checked):
            action = "With no closest match among the calls compared, " + action[:1].lower() + action[1:]
        return f"{finding}. {action}"
    if edits:
        edit = edits[0]
        call = calls[edit["after_site_id"] or edit["before_site_id"]]
        method, path = edit["requested_target"]
        target = f" for {_code(f'{method} {path}')}" if method and path else " with an unknown target"
        finding = {
            "Client call added": "The client adds a call", "Client call removed": "The client removes a call",
            "Client target changed": "The client changes the target of a call",
            "Observed before": "The check found a call before the edit", "Observed after": "The check found a call after the edit",
        }[edit["kind"]]
        return f"{finding}{target} at {site_link(call['site'])}. {edit['review_action']}"
    return "The checked source shows no measured change. Check the coverage before relying on this comparison."


def site_link(site):
    location = _code(f"{site['file']}:{site['line']}")
    return f"[{location}]({site['url']})" if site.get("url") else location


def versions_line(doc):
    change = doc["change"]
    counterpart = doc["inputs"][doc["reference"]["reference_input_id"]]
    role = "service" if counterpart["role"] == "producer" else "client"

    def describe(commit):
        return f"{_code(commit['rev'][:8])} ({commit['committed_at'][:10]}, {label(commit['subject'])})"

    fixed = f"{role} held at the base dated {counterpart['committed_at'][:10]}"
    if change["subject_role"] == "both":
        fixed += " for the single-side check, with both edits also checked together"
    return (f"**Versions checked** · {MODE} · subject {describe(change['subject'])} · "
            f"base {describe(change['base'])} · {fixed}.")


def provenance(doc, receipts, mark):
    change = doc["change"]
    before = doc["inputs"][change["before_input_id"]["service"]]
    after = change["subject"]
    driver = before["tool"]
    result = [
        f"**Subject** · {_code(after['rev'])} · {after['committed_at']} · {label(after['subject'])}",
        f"**Repository** · {_code(before['repo'])}",
        f"**Before** · {_code(before['rev'])} · {before['committed_at']} · {label(before['subject'])} · "
        f"first parent · tree {_code(before['tree'])}",
        f"**After** · {_code(after['rev'])} · {after['committed_at']} · selected commit · tree {_code(after['tree'])}",
        f"**Selection time** · {doc['report_time'] or mark('no selection timestamp was supplied')}",
        "Selection freezes the named commit trees and study inputs for this comparison. This mode does not "
        "establish availability at change time or deployment order.",
        "**Input admission** · Source and schema files came from the selected Git trees. The driver recomputed "
        "inventories and matches from admitted inputs. Study inputs used are listed below.",
    ]
    reference = doc["reference"]
    counterpart = doc["inputs"][reference["reference_input_id"]]
    delta = reference["duration_seconds"]
    offset = ("Both commits have the same time" if delta == 0 else
              f"Checked revision is {timedelta(seconds=abs(delta))} {'earlier than' if delta < 0 else 'after'} "
              "the change")
    role = "Service" if counterpart["role"] == "producer" else "Client"
    result.append(f"**{role} held at the base** · {_code(counterpart['rev'])} · {counterpart['committed_at']} · "
                  f"{label(counterpart['subject'])} · tree {_code(counterpart['tree'])}. {offset}.")
    for (source_role, file, receipt), anchor in receipts.items():
        kind = "Client configuration" if source_role == "consumer" else "Route specification"
        result.append(f'<a id="{anchor}"></a>**{kind}** · {_code(file)} from selection receipt '
                      f"{_code(receipt)}, supplied by the study.")
    driver_revision = (_code(driver["rev"]) + f" · {driver['committed_at']} · {label(driver['subject'])}"
                       if driver.get("committed_at") else
                       mark("driver commit date and subject were unavailable"))
    result.append(
        f"**Driver revision** · {driver_revision} · source receipt {_code(driver['build_identity'])}. "
        "The configuration era comes from each selected client revision’s first-parent ancestry. "
        "The configured request.ts client is primary when present in the configuration."
    )
    return result


def coverage(doc):
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
        "required in its parent, listed values and list item type. Path, query, header, cookie and form parameters, "
        "non-success and default responses, other media and constraints such as numeric bounds are outside this count.",
        "We compare the HTTP method and the path after applying the address mapping shown here. "
        "Among paths that fit, the check prefers those with more matching fixed or variable parts.",
    ]
    if doc["code_changed_with_unchanged_schema"]:
        result.append("The code changed. The checked-in schema file did not change. "
                      "Whether it describes this edit is unknown.")
    return result

