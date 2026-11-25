"""Shared Markdown formatting and the numbered limits for a review."""

from html import escape
import re

from markdown_it import MarkdownIt

from .report_claims import Record, decide, reasons
from .report_proof import source_record

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
        if isinstance(reason, dict):
            if "confirmed" in reason:
                return " · ".join(dict.fromkeys(self.add(failure, where) for failure in reason["failures"]))
            file = reason["file"]
            location = reason["side"] or ""
            if file:
                location = " ".join(filter(None, (location, _code(file + (f":{reason['line']}" if reason["line"] else "")))))
                if reason["rev"]:
                    location += f" at {_code(reason['rev'][:8])}"
            where = " · ".join(filter(None, (where, location)))
            reason = reason["reason"]
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


MODES = {"version_comparison": "Comparison of selected versions", "live": "Live review",
         "historical_as_of": "Historical review at the change"}


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
    group = next((group for group in groups if group["change_kinds"]), None)
    if group:
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
        if group["claims"].get("Declaration moved", {}).get("confirmed"):
            old, new = (next(row["site"]["file"] for row in group["file_changes"] if row["kind"] == kind)
                        for kind in ("removal", "addition"))
            finding = f"The declaration of {target} moves from {_code(old)} to {_code(new)}"
        elif kind.startswith("Declaration "):
            finding = f"{target} · {kind}"
        if kind.startswith("Listed only") and group["proof_failures"]:
            finding += ". " + " ".join(dict.fromkeys(group["limits"])).rstrip(".")
        ids = list(dict.fromkeys([*(risk["site_id"] for risk in group["risks"]), *group["client_site_ids"]]))
        if ids:
            site = calls[ids[0]]["site"]
            location = site_link(site)
            risk = group["risks"][0] if group["risks"] else {}
            service_first = risk.get("comparison_purposes", [])[-1:] == ["service_first"]
            context = {
                "Client calls a missing route": "which loses its match" + (
                    " if the service ships first" if service_first else " in the checked operations"),
                "Existing client call gains a matching declaration": "which gains a match" + (
                    " if the service ships first" if service_first else " in the checked operations"),
                "Client target needs confirmation": "whose target needs confirmation",
            }.get(risk.get("kind"), "which appears in the client comparisons below")
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
    proof = doc["summary"]["empty"][doc["change"]["subject_role"]]
    if not proof["confirmed"]:
        return reasons(proof) + " Check the affected source."
    return "The checked source shows no measured change. Check the coverage before relying on this comparison."


def site_location(site):
    location = _code(f"{site['file']}:{site['line']}")
    return f"{location} (occurrence {site['occurrence']})" if site.get("same_line_occurrences", 0) > 1 else location


def source_site(site, kind=None, *, excerpt=True):
    proof = decide("Source location", {
        Record("tree source location"): bool(site and all(site.get(key) for key in ("tree", "blob", "file", "line"))),
    })
    if not proof["confirmed"]:
        return reasons(proof)
    location = site_location(site)
    if site.get("url"):
        location = f"[{location}]({site['url']})"
    version = _code(site["rev"][:8])
    text = _code(site.get("text")) if site.get("text") and (excerpt or not site.get("url")) else ""
    return " · ".join(part for part in (kind, location, version, text) if part)


def site_link(site):
    location = site_location(site)
    return f"[{location}]({site['url']})" if site.get("url") else location


def counterpart_sites(doc, match):
    if "repo" not in doc["reference"] or doc["change"]["subject_role"] != "client":
        return []
    service = doc["inputs"][match["service_input_id"]]
    ids = dict.fromkeys([*match["target_ids"], *(row["operation"] for row in match["alternates"])])
    return [site_link(site) for identifier in ids for site in service["operations"][identifier]["sites"]]


def request_counts(purpose, counts, mark):
    no_match = counts["known_unmatched_count"]
    unknown = counts["unknown_target_count"]
    return (
        f"[{no_match['observed']} requests whose method and address were read had no match](#requests-{purpose})"
        + (f" observed · {mark(no_match['claim'])}" if not no_match["claim"]["confirmed"] else "")
        + f" · [{unknown['observed']} calls had an unknown target](#requests-{purpose})"
        + (f" observed · {mark(unknown['claim'])}" if not unknown["claim"]["confirmed"] else "")
    )


def versions_line(doc):
    change = doc["change"]
    reference = doc["reference"]
    counterpart = doc["inputs"][reference["reference_input_id"]]
    role = "service" if counterpart["role"] == "producer" else "client"

    def describe(commit):
        return f"{_code(commit['rev'][:8])} ({commit['committed_at'][:10]}, {label(commit['subject'])})"

    if reference.get("unavailable_reason"):
        fixed = f"{role} counterpart · Not checked"
    elif "repo" in reference:
        fixed = f"{role} held unchanged at {describe(counterpart)}"
    else:
        fixed = f"{role} held at the base dated {counterpart['committed_at'][:10]}"
    if change["subject_role"] == "both":
        fixed += " for the single-side check, with both edits also checked together"
    return (f"**Versions checked** · {MODES[doc['mode']]} · subject {describe(change['subject'])} · "
            f"base {describe(change['base'])} · {fixed}.")


def duration(seconds):
    parts = []
    for unit, width in (("day", 86400), ("hour", 3600), ("minute", 60), ("second", 1)):
        count, seconds = divmod(seconds, width)
        if count:
            parts.append(f"{count} {unit}{'s' if count != 1 else ''}")
    return ", ".join(parts)


def provenance(doc, receipts, mark):
    change = doc["change"]
    reference = doc["reference"]
    subject_side = "client" if "repo" in reference and change["subject_role"] == "client" else "service"
    before = doc["inputs"][change["before_input_id"][subject_side]]
    after = change["subject"]
    driver = before["tool"]
    result = [
        f"**Subject** · {_code(after['rev'])} · {after['committed_at']} · {label(after['subject'])}",
        f"**Repository** · {_code(before['repo'])}",
        f"**Before** · {_code(before['rev'])} · {before['committed_at']} · {label(before['subject'])} · "
        f"first parent · tree {_code(before['tree'])}",
        f"**After** · {_code(after['rev'])} · {after['committed_at']} · selected commit · tree {_code(after['tree'])}",
        f"**{ {'version_comparison': 'Selection time', 'live': 'Report time', 'historical_as_of': 'Change time'}[doc['mode']]}** · "
        f"{doc['report_time'] or mark('no selection timestamp was supplied')}",
        "**Input admission** · Source and schema files came from the selected Git trees. The driver recomputed "
        "inventories and matches from admitted inputs. Study inputs used are listed below.",
    ]
    if doc["mode"] == "version_comparison":
        result.append("Selection freezes the named commit trees and study inputs. Availability at change time and "
                      "deployment order are unknown.")
    counterpart = doc["inputs"][reference["reference_input_id"]]
    role = "Service" if counterpart["role"] == "producer" else "Client"
    if tip := reference.get("selection_history_tip"):
        result.append(f"**Searched history tip** · {_code(reference['branch'])} · {_code(tip)} · "
                      f"{reference['selection_history_tip_committed_at']}.")
    if reason := reference.get("unavailable_reason"):
        result.append(f"**{role} counterpart** · {_code(counterpart['repo'])} · Not checked — {label(reason)}.")
    else:
        delta = reference["duration_seconds"]
        offset = ("Both commits have the same time" if delta == 0 else
                  f"Checked revision is {duration(abs(delta))} {'earlier' if delta < 0 else 'later'} than the change")
        details = (f"{_code(counterpart['rev'])} · {counterpart['committed_at']} · "
                   f"{label(counterpart['subject'])} · tree {_code(counterpart['tree'])}. {offset}.")
        if "repo" in reference:
            result.append(f"**{role} held unchanged** · {_code(counterpart['repo'])} · {details}")
            result.append(reference["selection_reason"].rstrip(".") + ".")
            if reference["selected_by"] == "commit_date":
                result.append("The selected commit is the last on the counterpart HEAD’s first-parent history "
                              f"with a committer date at or before {after['committed_at']}.")
            if delta > 0:
                result.append("This comparison uses code from after the change.")
        else:
            result.append(f"**{role} held at the base** · {details}")
    for (source_role, file, receipt), anchor in receipts.items():
        kind = "Client configuration" if source_role == "consumer" else "Route specification"
        source = next(row for snap in doc["inputs"].values() if snap["role"] == source_role
                      for row in snap["config_sources"] if row["file"] == file and row["revision"] == receipt)
        if receipt:
            origin = (f"supplied by the study from {_code(source['study_rev'])}, tree {_code(source['tree'])}"
                      if source.get("study_rev") else "supplied by the study, read at capture time, "
                      "not shown to have existed at the change time")
            result.append(f'<a id="{anchor}"></a>**{kind}** · {_code(file)} · {_code(receipt)} · {origin}.')
        else:
            result.append(f'<a id="{anchor}"></a>**{kind}** · {mark(source["error"], _code(file))}.')
    driver_revision = (_code(driver["rev"]) + f" · {driver['committed_at']} · {label(driver['subject'])}"
                       if driver.get("committed_at") else
                       mark("driver commit provenance was not admitted for this report"))
    result.append(
        f"**Driver revision** · {driver_revision} · source hash {_code(driver['build_identity'])} · "
        f"{driver['origin']}. The configuration era comes from each selected client revision’s first-parent ancestry. "
        "The configured request.ts client is primary when present in the configuration."
    )
    if any(source.get("selected_by") == "single supplied file" for snap in doc["inputs"].values()
           for source in snap["config_sources"]):
        result.append("A single supplied client configuration file is used directly.")
    return result


def coverage(doc, mark):
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
    scope = doc["summary"]["schema_coverage"]
    if not scope["confirmed"]:
        location = _code(service["schema"]["source"]["path"])
        result[2] = f"**Schema path** · {location}. Changed fields: Not checked. {mark(scope)}."
        if len(doc["field_limits"]) == 1 and service["schema"]["status"] in {"path_absent", "unselected", "omitted"}:
            result[2] = f"**Schema path** · {_code(service['schema']['source']['path'])}. {doc['field_limits'][0]}. {mark(scope)}."
    for index, role in enumerate(("service", "client")):
        snapshots = {change[f"{side}_input_id"][role]: side for side in ("before", "after")}
        for key, side in snapshots.items():
            snapshot = doc["inputs"][key]
            proof = decide("Source coverage", {Record("source inventory"): source_record("Source coverage", snapshot, side)})
            if not proof["confirmed"]:
                result[index] += " " + mark(proof) + "."
    if reason := doc["reference"].get("unavailable_reason"):
        limit = mark(reason)
        if service["snapshot_kind"] == "unavailable":
            result[0] = "**Source of operations** · " + limit
            result[2] = "**Schema path** · " + limit
        else:
            result[1] = "**Client scope** · " + limit
        result[3] = "**Request matching** · " + limit
    if doc["code_changed_with_unchanged_schema"]:
        result.append("The code changed. The checked-in schema file did not change. "
                      "Whether it describes this edit is unknown.")
    return result

