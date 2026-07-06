"""claim_count = the recall denominator (spec §6a, codex new-2): how many substantive
code-claims an output makes, so under-citing (claims without citations) is penalized.
Heuristic proxy: sentences that reference a code entity (identifier-ish / path / call).

Known false positives (acceptable — this is a diagnostic heuristic, and FPs inflate the
denominator => UNDERSTATE recall, the conservative direction): words ending in common
suffixes (reset/offset/subset/dialog/catalog), and mid-sentence proper nouns (Alice/Docker).
"""
from __future__ import annotations
import re

_SENT = re.compile(r"[.!?\n]+")
# Code-ish token: function call, snake_case, non-sentence-starting PascalCase,
# file path, or lowercase word ending in a common library/module suffix.
_CODE = re.compile(
    r"\b[a-z_][a-z0-9_]*\(\)"                       # function call: compile()
    r"|\b[a-z_][a-z0-9_]*_[a-z0-9_]+\b"             # snake_case: foo_bar
    r"|(?<=[\w]) [A-Z][a-zA-Z0-9]+"                  # non-sentence-starting PascalCase: Glob, Foo
    r"|[\w/.-]+\.[a-z]{1,4}\b"                        # file path: src/a.py
    r"|\b[a-z][a-z0-9]*(?:set|map|db|buf|ctx|cfg|err|srv|rpc|api|sdk|io|fs|cmd|cli"
    r"|env|uri|url|uuid|fmt|log|str|ptr|len|cap|idx|pos|crc|tcp|udp|msg|req|res"
    r"|arg|args|opt|opts)\b"                           # library-suffix: globset, hashmap
)

def count_claims(text: str) -> int:
    n = 0
    for sent in _SENT.split(text):
        if sent.strip() and _CODE.search(sent):
            n += 1
    return n


def sentence_spans(text: str) -> list[tuple[int, int, str]]:
    """(start, end, sentence_text) for each _SENT-delimited span in `text`, so a
    citation's character offset can be mapped back to its enclosing sentence (D1
    validity.py / D3 annotate.py). Uses the SAME splitter as count_claims so "sentence"
    means the same thing across the recall-denominator and validity/annotation axes."""
    spans: list[tuple[int, int, str]] = []
    pos = 0
    for m in _SENT.finditer(text):
        spans.append((pos, m.start(), text[pos:m.start()]))
        pos = m.end()
    spans.append((pos, len(text), text[pos:]))
    return spans
