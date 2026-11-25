"""Join later replay evidence after the live report has been serialized."""

from collections import defaultdict
from dataclasses import asdict
from datetime import datetime, timezone

from .report_markdown import _code, table


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


def join_evaluation(live, replay_result, observation_end, assessments=None, model=None):
    result = asdict(replay_result) if not isinstance(replay_result, dict) else replay_result
    change = live["change"]
    reference = live["inputs"][change["before_input_id"]["client"]]
    if (change["requested_role"] != "producer" or result["commit"] != change["after"]
            or result["parent"] != change["before"] or result["consumer_revision"] != reference["rev"]):
        raise ValueError("Replay subject and parent client revision must match the serialized producer report")
    sites = defaultdict(list)
    for call in reference["calls"]:
        site = call["site"]
        sites[site["file"], site["line"]].append(call)
    elements = {row.get(key) for row in change["elements"]
                for key in ("element_id", "before_element_id", "after_element_id")}
    rows = []
    for row in result["predicted"]:
        call = row["call"]
        occurrences = sites[call["file"], call["line"]]
        element = row["element"]
        linked = None
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
            reason, linked = None, occurrences[0]["site_id"]
        assessment = (assessments or {}).get(row["location_id"], {})
        outcome = row["outcome"]
        first = row["followed_lines"][0] if outcome["touched_in"] and len(row["followed_lines"]) == 1 else None
        property_files = sorted({record["file"] for record in row["followed_lines"]
                                 if record != row["predicted_line"]})
        rows.append({
            "element_id": element, "usage_id": None, "site_id": linked, "join_reason": reason,
            "tracked_client_revision": reference["rev"], "followed_lines": row["followed_lines"],
            "outcome": outcome["kind"], "first_touched_line": first,
            "first_touched_line_reason": None if first else (
                "Not checked — the replay did not identify an individual first edited line"),
            "touching_commit": row["touching_commit"], "lag_days": outcome["lag_days"],
            "lag_commits": outcome["lag_commits"], "release_set": None, "change_release": None, "edit_release": None,
            "release_reason": "Release: Not checked — no ordered release set and included revisions were supplied",
            "reader_assessment": assessment.get("class"), "assessment_evidence": assessment.get("reason"),
            "field_spelled": row["field_spelled"] if element.startswith("field:") else None,
            "property_name_files": property_files,
            "current_line": row["current_line"], "current_property_lines": row["current_property_lines"],
            "predicted_line": row["predicted_line"], "edit_hunks": row["hunks"], "callers": row["callers"],
        })
    return {
        "schema_version": "2", "live_report_id": live["report_id"], "observation_end": observation_end,
        "subject": change["after"], "study_adjudicator": model, "rows": rows,
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


def _assessment(row):
    raw = row["reader_assessment"]
    assessment = _ASSESSMENTS.get(raw, "No study assessment recorded." if raw is None else
                                  "Not checked — this study assessment has no wording in the report.")
    evidence = row["assessment_evidence"]
    if evidence:
        assessment += " Recorded reason · " + evidence
    if not evidence and raw is not None:
        assessment += " Not checked — the study recorded no reason."
    if raw in {"dead_code", "real_mismatch"}:
        assessment += " Runtime reach and field use were not checked."
    spelled = row["field_spelled"]
    if spelled is not None:
        files = row["property_name_files"]
        if spelled:
            assessment += " Property name found in the tracked lines"
            assessment += " · " + ", ".join(_code(file) for file in files) + "." if files else "."
        else:
            assessment += " No such property name found in the searched files. "
            assessment += "Search file list: Not checked — the replay recorded only lines where the name was found."
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
    lines = [
        "# Replay evaluation (uses later history)",
        f"Service subject {_code(document['subject'])}. Last revision examined "
        f"{_code(end['rev'])} · {_date(end['committed_at'])}.",
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
                  "config_missing": "the primary client source or configuration was missing"}
        lines.append("Not checked — " + reason.get(document["skipped_reason"], "the replay did not finish the scan") + ".")
    else:
        lines.append("The replay recorded no locations for this subject.")
    lines.append(f"The study also followed {document['control_count']} comparison call sites.")
    return "\n\n".join(lines) + "\n"
