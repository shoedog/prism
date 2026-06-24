"""claim_count = the recall denominator (spec §6a, codex new-2): how many substantive
code-claims an output makes, so under-citing (claims without citations) is penalized.
Heuristic proxy: sentences that reference a code entity (identifier-ish / path / call)."""
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
