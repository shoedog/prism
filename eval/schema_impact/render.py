"""Render a single replay result and its reader labels as Markdown."""

from dataclasses import asdict

from .replay import ReplayResult


def report(result: ReplayResult | dict, labels: dict) -> str:
    row = asdict(result) if isinstance(result, ReplayResult) else result
    commit = row["change_commit"] or {}
    lines = [
        f"# Schema impact at `{row['commit']}`",
        "",
        commit.get("subject", ""),
        "",
        f"Producer parent `{row['parent']}`",
        "",
        "## Changes and producer locations",
        "",
    ]
    for change in row["changes"]:
        element = change["element"]
        lines.append(f"- `{change['kind']}` on `{element}`")
        if change.get("old_shape") is not None or change.get("new_shape") is not None:
            lines.append(f"  - Shape `{change.get('old_shape')}` → `{change.get('new_shape')}`")
        sites = row["producer_locations"].get(element, [])
        for site in sites:
            lines.append(f"  - `{site['file']}:{site['line']}`")
        if not sites:
            lines.append("  - No source location recovered.")
    lines.extend(["", "## Consumer locations", ""])
    if row["skipped_reason"]:
        lines.append(f"Replay skipped with `{row['skipped_reason']}`.")
    elif not row["predicted"]:
        lines.append("No bound client calls were predicted.")
    for location in row["predicted"]:
        call = location["call"]
        kind = location["outcome"]["kind"]
        label = labels.get(location["location_id"])
        category = label.get("class") if isinstance(label, dict) else label
        spelling = "yes" if location["field_spelled"] else "no"
        lines.append(f"- `{call['file']}:{call['line']}` for `{location['element']}` "
                     f"— `{kind}`, class `{category or 'unlabelled'}`, field spelled {spelling}")
    lines.extend(["", "## What happened next", ""])
    for location in row["predicted"]:
        outcome = location["outcome"]
        site = location["predicted_line"]
        prefix = f"- `{site['file']}:{site['line']}`"
        if outcome["touched_in"]:
            touch = location["touching_commit"] or {}
            subject = f" ({touch['subject']})" if touch.get("subject") else ""
            lag = f", +{outcome['lag_commits']} first-parent commits" if outcome["lag_commits"] is not None else ""
            lines.append(f"{prefix} touched in `{outcome['touched_in']}`{subject}"
                         f"{lag}, {outcome['lag_days']:.2f} days after the change.")
        else:
            current = location["current_line"]
            endpoint = "pin" if outcome["kind"] == "present_at_pin" else "consumer head"
            lines.append(f"{prefix} survived to the {endpoint} at `{current['file']}:{current['line']}`.")
    if row["control"]:
        lines.extend(["", f"Followed {len(row['control'])} control call sites."])
    return "\n".join(lines) + "\n"
