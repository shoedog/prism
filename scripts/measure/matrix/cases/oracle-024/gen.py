"""Regenerate call-site rows from the fixture's UTF-8 source."""

from __future__ import annotations

import json
from pathlib import Path

CASE = Path(__file__).resolve().parent
FILE = "cases.ts"
LINES = (CASE / "repo" / FILE).read_bytes().splitlines(keepends=True)


def function(name, first, last):
    return {"file": FILE, "name": name, "start_line": first, "end_line": last}


def site(name, line, call, caller, targets, *, drop=None):
    call_bytes = call.encode("utf-8")
    start = sum(len(text) for text in LINES[:line - 1]) + LINES[line - 1].index(call_bytes)
    return {
        "record_kind": "call_site", "callee_text": name, "source_callee_text": name,
        "call_kind": "Call", "origin": "Source", "caller": function(*caller),
        "source_span": {"file": FILE, "line": line, "start_byte": start,
                        "end_byte": start + len(call_bytes)},
        "resolved_targets": [{"function_id": function(*target), "confidence": confidence,
                              "kind": "local_def"} for target, confidence in targets],
        "exact_target": None, "drop": drop,
    }


def write(name, rows):
    (CASE / name).write_text(
        "".join(json.dumps(row, separators=(",", ":")) + "\n" for row in rows), encoding="utf-8"
    )


write('input.jsonl', [
    site('inner', 20, 'inner()', ('outer', 18, 21), [(('inner', 19, 19), 'exact')]),
])
write('base.sites.jsonl', [
    site('inner', 20, 'inner()', ('outer', 18, 21), [(('outer', 18, 21), 'exact')]),
])
