"""Join later replay evidence after the live report has been serialized."""

from collections import defaultdict
from dataclasses import asdict
from datetime import datetime, timezone

from ._replay_client import ClientSources
from ._replay_git import LineFollower
from ._ts_syntax import EXTENSIONS
from .config import ClientSpec, Config
from .consumer_ts import ClientCall
from .replay import Outcome, ReplayResult, _locations
from .report_inputs import _source_tree, _tree
from .report_layout import _code, label, table
from .report_proof import measured_operation_sides
from .schema import Change


_ASSESSMENTS = {
    "coordinated": "The study adjudicator linked the edits to the same feature.",
    "fix_after_release": "The study adjudicator identified a follow-up fix.",
    "dead_code": "The study adjudicator found no direct call by name in the checked files.",
    "real_mismatch": "The study adjudicator found a source call by name and could not find the changed element in the checked service sources.",
    "unrelated_edit": "Unrelated edit.",
    "false_prediction": "The study adjudicator judged this call unaffected.",
    "cannot_tell": "The study adjudicator could not determine the effect.",
}


def _date(value):
    return datetime.fromisoformat(value).astimezone(timezone.utc).isoformat() if value else None


def replay_counterpart(live, history):
    """Follow the producer report's bound client sites in the client repository."""
    change = live["change"]
    reference = live["inputs"][live["reference"]["reference_input_id"]]
    if change["requested_role"] != "producer" or reference["role"] != "consumer":
        raise ValueError("Counterpart replay requires a producer subject and a fixed client")
    selected = reference["rev"]
    result = ReplayResult(change["after"], change["before"], [], consumer_revision=selected)
    if selected not in history.position:
        result.skipped_reason = "reference_outside_history" if selected else "no_consumer_revision"
        return result

    for group in live["impacts"]:
        kinds = set(group["change_kinds"])
        if kinds & {"Operation added", "Operation removed", "Route declaration changed",
                    "Successful response statuses changed"}:
            result.changes.append(Change("operation", group["operation_id"]))
        result.changes.extend(Change("field", (field["after"] or field["before"])["field_id"])
                              for field in group["fields"])
    if not result.changes:
        return result

    bindings = defaultdict(set)
    for comparison in live["comparisons"]:
        for site_id, match in comparison["matches"].items():
            if match["status"] == "match":
                bindings[site_id].update(match["target_ids"])
    bound = []
    for row in reference["calls"]:
        site = row["site"]
        call = ClientCall(site["file"], site["line"], row["requested_method"], row["original_url_text"],
                          row["mapped_path"], row["url_class"], row["target_reason"],
                          row["response_type"], row["enclosing_function"])
        bound.extend((call, operation) for operation in sorted(bindings[row["site_id"]]))
    if not bound:
        return result

    change_time = datetime.fromisoformat(change["subject"]["committed_at"]).timestamp()

    def outcome(touched):
        if touched is None:
            return Outcome("present_at_pin")
        days = (history.seconds[touched] - change_time) / 86400
        return Outcome(_day_bucket(days), touched, lag_days=days)

    config, = reference["config_sources"]
    cfg = Config(config["aliases"], [ClientSpec(**{k: v for k, v in entry.items() if k != "entry"})
                                    for entry in config["entries"]])
    files = {file: record for file, record in _tree(history.repo, reference["tree"]).items()
             if file.endswith(EXTENSIONS)}
    with _source_tree(history.repo, files) as (snapshot, selected_sources):
        checked = [snapshot / row["path"] for row in reference["file_status"]
                   if row["status"] == "complete" and selected_sources.get(row["path"]) is not None]
        sources = ClientSources(snapshot, cfg, reference["scope"], files=checked)
        follower = LineFollower(history, selected, snapshot)
        _locations(result, bound, sources, follower, outcome, controls=False)
    return result


def join_evaluation(live, replay_result, observation_end, assessments=None, model=None):
    result = asdict(replay_result) if not isinstance(replay_result, dict) else replay_result
    change = live["change"]
    reference = live["inputs"][change["before_input_id"]["client"]]
    if (change["requested_role"] != "producer" or result["commit"] != change["after"]
            or result["parent"] != change["before"] or result["consumer_revision"] != reference["rev"]):
        raise ValueError("Replay subject, parent and checked client revision must match the serialized producer report")
    sites = defaultdict(list)
    for call in reference["calls"]:
        site = call["site"]
        sites[site["file"], site["line"]].append(call)
    elements = defaultdict(set)
    removed_operations = {row["element"] for row in result["changes"] if row["kind"] == "op_removed"}
    for group in live["impacts"]:
        operation = group["operation_id"]
        for side in measured_operation_sides(group):
            if operation in removed_operations and not group["claims"].get("Operation removed", {}).get("confirmed"):
                continue
            elements[operation].add((operation, side))
        for field in group["fields"]:
            for side in ("before", "after"):
                if field[side]:
                    elements[field[side]["field_id"]].add((operation, side))
    pairs = {pair["purpose"]: pair for pair in live["comparisons"]}
    after_purpose = "service_first" if "service_first" in pairs else "after"
    rows = []
    for row in result["predicted"]:
        call = row["call"]
        occurrences = sites[call["file"], call["line"]]
        element = row["element"]
        linked, matched = None, []
        if len(occurrences) > 1:
            reason = "Not checked — more than one call shares this line"
        elif not occurrences:
            reason = "Not checked — no call from the checked client revision was recorded on this line"
        elif (occurrences[0]["requested_method"], occurrences[0]["mapped_path"], occurrences[0]["original_url_text"]) != (
                call["method"], call["pattern"], call["url_text"]):
            reason = "Not checked — the request recorded in the review differs from the tracked call"
        elif element not in elements:
            reason = "Not checked — the live review has no measured change for this element"
        else:
            key = occurrences[0]["site_id"]
            matched = [(operation, side) for operation, side in sorted(elements[element])
                       if (match := pairs["before" if side == "before" else after_purpose]["matches"].get(key))
                       and match["status"] == "match" and operation in match["target_ids"]]
            if matched:
                reason, linked = None, key
            else:
                reason = "Not checked — the call does not have a verified match to this element’s operation on its recorded side"
        assessment = (assessments or {}).get(row["location_id"], {})
        outcome = row["outcome"]
        lag = outcome["lag_days"]
        observed = row["touching_commit"]
        if observed and observed.get("date"):
            lag = (datetime.fromisoformat(observed["date"]) -
                   datetime.fromisoformat(change["subject"]["committed_at"])).total_seconds() / 86400
        bucket = outcome["kind"]
        if outcome["touched_in"]:
            if outcome["touched_in"] == change["after"]:
                bucket = "same_commit"
            elif lag is not None:
                bucket = _day_bucket(lag)
            else:
                bucket = "unknown_edit_time"
        first = row["followed_lines"][0] if outcome["touched_in"] and len(row["followed_lines"]) == 1 else None
        property_files = sorted({record["file"] for record in row["followed_lines"]
                                 if record != row["predicted_line"]})
        rows.append({
            "element_id": element, "usage_id": None, "site_id": linked, "join_reason": reason,
            "join_comparisons": [{"operation_id": operation, "side": side} for operation, side in matched],
            "tracked_client_revision": reference["rev"], "followed_lines": row["followed_lines"],
            "outcome": bucket, "first_touched_line": first,
            "first_touched_line_reason": None if first else (
                "Not checked — the replay did not identify an individual first edited line"),
            "touching_commit": row["touching_commit"], "lag_days": lag,
            "lag_commits": outcome["lag_commits"], "release_set": None, "change_release": None, "edit_release": None,
            "release_reason": "Release: Not checked — no ordered release set and included revisions were supplied",
            "reader_assessment": assessment.get("class"), "assessment_evidence": assessment.get("reason"),
            "assessment_scope": assessment.get("scope"), "assessment_revision": assessment.get("revision"),
            "field_spelled": row["field_spelled"] if element.startswith("field:") else None,
            "property_name_files": property_files,
            "current_line": row["current_line"], "current_property_lines": row["current_property_lines"],
            "predicted_line": row["predicted_line"], "edit_hunks": row["hunks"], "callers": row["callers"],
        })
    return {
        "schema_version": "2", "live_report_id": live["report_id"], "observation_end": observation_end,
        "subject": change["after"], "study_adjudicator": model, "rows": rows,
        "subject_metadata": change["subject"],
        "change_tag": (result["change_commit"] or {}).get("earliest_release_tag"),
        "control_count": len(result["control"]), "controls": result["control"],
        "skipped_reason": result["skipped_reason"],
    }


def _line(site):
    return _code(f"{site['file']}:{site['line']}")


def _element(identifier):
    if not identifier.startswith("field:"):
        return _code(identifier.removeprefix("op:"))
    root, _, path = identifier.removeprefix("field:").partition("#/")
    kind, _, operation = root.partition(":")
    if kind == "req":
        return f"Request body · {_code(operation)} · {_code(path)}"
    operation, status = operation.rsplit(":", 1)
    return f"Response body ({status}) · {_code(operation)} · {_code(path)}"


def _day_bucket(days):
    if days < 0:
        return "earlier"
    return "within_14_days" if days <= 14 else "later"


def _assessment(row):
    raw = row["reader_assessment"]
    assessment = _ASSESSMENTS.get(raw, "No study assessment recorded." if raw is None else
                                  "Not checked — this study assessment has no wording in the report.")
    evidence = row["assessment_evidence"]
    if raw in {"dead_code", "real_mismatch", "coordinated", "fix_after_release"}:
        revision, scope = row.get("assessment_revision"), row.get("assessment_scope")
        if not all((evidence, revision, scope)):
            missing = ", ".join(name for name, value in (("searched revision", revision),
                                ("searched scope", scope), ("supporting record", evidence)) if not value)
            assessment = f"Not checked — the assessment lacks {missing}."
        else:
            assessment += f" Searched {_code(scope)} at {_code(revision)}."
    if evidence:
        assessment += " Recorded reason · " + label(evidence)
    elif raw is not None and "Not checked" not in assessment:
        assessment = "Not checked — the study recorded no supporting reason."
    if raw in {"dead_code", "real_mismatch"}:
        assessment += " Runtime reach and field use were not checked."
    spelled = row["field_spelled"]
    if spelled is not None:
        files = row["property_name_files"]
        if spelled:
            assessment += " Property name found in the tracked lines"
            assessment += " · " + ", ".join(_code(file) for file in files) + "." if files else "."
        else:
            assessment += " Property name search: Not checked — the replay supplied no complete list of searched files."
        assessment += " Actual field use: Not checked — the search recorded spelling only."
    return assessment


def _first_edit(row, end):
    outcome, touch = row["outcome"], row["touching_commit"]
    if outcome in {"present_at_pin", "present_at_head"}:
        if row["current_line"]:
            return (f"Tracked lines remained unchanged through {_code(end['rev'])}, {_date(end['committed_at'])}. "
                    f"The tracked call line was at {_line(row['current_line'])}.")
        return "Not checked — the replay did not record the final tracked line."
    phrases = {
        "same_commit": "A tracked line changed in the same commit.",
        "within_14_days": "A tracked line changed within 14 days.",
        "later": "A tracked line changed later.",
        "earlier": "The recorded edit is dated before the service change.",
        "unknown_edit_time": "Time from the service change: Not checked — the edit interval was unavailable.",
    }
    sentence = phrases.get(outcome, "Not checked — the replay outcome could not be interpreted.")
    if touch:
        sentence += f" {_code(touch['commit'])} · {_date(touch['date'])} · {touch['subject']}."
        if touch["earliest_release_tag"]:
            sentence += f" Tag containing this commit · {_code(touch['earliest_release_tag'])}."
        if row["first_touched_line"]:
            sentence += f" Tracked line · {_line(row['first_touched_line'])}."
        else:
            sentence += " " + row["first_touched_line_reason"] + "."
        if row["edit_hunks"]:
            sentence += " The recorded source diff is in the evaluation JSON."
    else:
        sentence += " Not checked — the replay supplied no metadata for the edit."
    return sentence


def render_evaluation(document):
    end = document["observation_end"]
    subject = document.get("subject_metadata", {})
    subject_label = _revision_label(document["subject"], subject)
    end_label = _revision_label(end["rev"], end)
    lines = [
        "# Replay evaluation (uses later history)",
        f"Service subject {subject_label}. Last revision examined {end_label}.",
        "Lines the study tracked include the client call and, for a field, lines spelling its property name "
        "in the same file or in identified importers of the enclosing function. An edit to any tracked line "
        "can be the first recorded edit.",
        "The study adjudicator is the model that assessed the subsequent evidence separately from the change review. "
        + (f"Model recorded by the study · {document['study_adjudicator']}." if document["study_adjudicator"]
           else "Not checked — the study did not identify the model."),
        "Release: Not checked — no ordered release set and included revisions were supplied.",
    ]
    if document["change_tag"]:
        lines.append(f"Tag containing the service commit · {_code(document['change_tag'])}.")
    rows = []
    for row in document["rows"]:
        followed = ", ".join(_line(site) for site in row["followed_lines"]) or "Not checked — tracked lines unavailable"
        source = _element(row["element_id"]) + "<br>" + followed
        if row["join_reason"]:
            source += "<br>" + row["join_reason"]
        lag = row["lag_days"]
        lag_text = f"{lag:.2f} days" if lag is not None else "—"
        if row["lag_commits"] is not None:
            lag_text += f" · {row['lag_commits']} first-parent commits"
        rows.append((source, _first_edit(row, end), lag_text, _assessment(row)))
    if rows:
        lines.append(table(["Lines the study tracked", "First recorded edit", "Time from service change",
                            "Study adjudicator’s assessment and evidence"], rows))
    elif document["skipped_reason"]:
        reason = {"no_client_import": "no import could be linked to the configured client",
                  "config_missing": "the primary client source or configuration was missing",
                  "no_consumer_revision": "the report has no checked client revision",
                  "reference_outside_history": "the checked client revision is outside the observation pin’s first-parent history"}
        lines.append("Not checked — " + reason.get(document["skipped_reason"], "the replay did not finish the scan") + ".")
    else:
        lines.append("The replay recorded no locations for this subject.")
    lines.append(f"The study also followed {document['control_count']} comparison call sites.")
    return "\n\n".join(lines) + "\n"


def _revision_label(revision, metadata):
    date = _date(metadata.get("committed_at")) or "date unavailable"
    subject = label(metadata["subject"]) if metadata.get("subject") else "subject unavailable"
    return f"{_code(revision)} · {date} · {subject}"
