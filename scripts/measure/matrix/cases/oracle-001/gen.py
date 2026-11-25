"""Regenerate the input rows from the fixture's UTF-8 source."""

from __future__ import annotations

import json
import re
from pathlib import Path

CASE = Path(__file__).resolve().parent
FILE = "cases.ts"
LINES = (CASE / "repo" / FILE).read_bytes().splitlines(keepends=True)


def endpoint(line, identifier, access, occurrence=1, *, path=None, bounds=True):
    result = {"file": FILE, "line": line, "access": access,
              "path": {"base": path or identifier, "fields": []}}
    if bounds:
        encoded = re.escape(identifier.encode("utf-8"))
        pattern = rb"(?<![\w$])" + encoded + rb"(?![\w$])"
        token = list(re.finditer(pattern, LINES[line - 1]))[occurrence - 1]
        offset = sum(len(text) for text in LINES[:line - 1])
        result.update(start_byte=offset + token.start(), end_byte=offset + token.end())
    return result


rows = [
    {"from": endpoint(3, 'actual', 'use'),
     "to": endpoint(1, 'formal', 'def'), "confidence": "exact", "doubt": None},
]
(CASE / "input.jsonl").write_text(
    "".join(json.dumps(row, separators=(",", ":")) + "\n" for row in rows), encoding="utf-8"
)
