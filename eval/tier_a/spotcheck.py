"""M3 site-level definition spot-check (spec §2.6 verdict table)."""
from __future__ import annotations

import re

from .model import DefTarget, FunctionDef


def _strip_strings_comments(line: str) -> str:
    line = re.sub(r'"(?:[^"\\]|\\.)*"', '""', line)
    line = re.sub(r"'(?:[^'\\]|\\.)*'", "''", line)
    return re.split(r"//|#", line, maxsplit=1)[0]


def find_call_position(line: str, name: str) -> int | None:
    """Column of `name` in CALL position: name( , .name( , ::name( ; fallback any
    occurrence; None if the token is absent entirely."""
    code = _strip_strings_comments(line)
    for m in re.finditer(rf"(?:(?<=\.)|(?<=::)|\b){re.escape(name)}\s*\(", code):
        return m.start()
    m = re.search(rf"\b{re.escape(name)}\b", code)
    return m.start() if m else None


def classify_site(
    line: str,
    seed_name: str,
    defs: list[DefTarget],
    seed: FunctionDef,
) -> str:
    """§2.6 verdict table: confirmed_tp | confirmed_fp | ambiguous | alias_site."""
    if find_call_position(line, seed_name) is None:
        return "alias_site"
    if not defs:
        return "ambiguous"
    for d in defs:
        if (
            d.name == seed.name
            and d.location.file == seed.location.file
            and d.location.start_line <= seed.selection_line <= d.location.end_line
        ):
            return "confirmed_tp"
    if any(d.name == seed.name for d in defs):
        return "ambiguous"
    return "confirmed_fp"
