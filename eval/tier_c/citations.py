"""Extract code citations (file:line[:symbol]) from arm text (spec §3 citation parity,
§6a investigator). Conservative: a citation is a path with a code-ish extension + line."""
from __future__ import annotations
import re
from .model import Citation

_EXT = r"(?:rs|go|py|js|jsx|ts|tsx|c|cc|cpp|h|hpp|java|lua)"
# path/seg.ext : line [ : symbol ]   — path has no spaces, optional ./
_PAT = re.compile(
    rf"(?<![\w/.])((?:[\w./-]+/)?[\w.-]+\.{_EXT}):(\d+)(?::([A-Za-z_]\w*))?"
)

def parse_citations(text: str) -> list[Citation]:
    seen: set[tuple[str, int, str | None]] = set()
    out: list[Citation] = []
    for m in _PAT.finditer(text):
        file, line, sym = m.group(1), int(m.group(2)), m.group(3)
        key = (file, line, sym)
        if key not in seen:
            seen.add(key)
            out.append(Citation(file=file, line=line, symbol=sym))
    return out


def iter_citation_occurrences(text: str):
    """Yield (start, end, Citation) for EVERY citation occurrence in `text`, in order,
    NOT deduped (unlike parse_citations). D1 (validity.py) and D3 (annotate.py) need the
    exact text position of each occurrence to map it back to its enclosing sentence and
    to insert inline annotation tags — a repeated file:line cited from two different
    sentences is two distinct occurrences for those purposes."""
    for m in _PAT.finditer(text):
        file, line, sym = m.group(1), int(m.group(2)), m.group(3)
        yield m.start(), m.end(), Citation(file=file, line=line, symbol=sym)
