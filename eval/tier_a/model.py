"""Schemas + normalization (spec §2.1). Adapters convert AT THEIR BOUNDARY;
comparison code never sees raw LSP or prism JSON. All lines 1-based inclusive,
files repo-relative POSIX."""
from __future__ import annotations

from dataclasses import dataclass


@dataclass(frozen=True, order=True)
class Location:
    file: str
    start_line: int
    end_line: int


@dataclass(frozen=True)
class FunctionDef:
    name: str | None          # None = anonymous; excluded from matching (§2.1)
    kind: str                 # oracle-side semantic: function|method|constructor
    container: str | None     # enclosing symbol from hierarchical documentSymbol
    location: Location
    selection_line: int       # name-token line (LSP selectionRange; prism: start_line)
    selection_char: int = 0   # name-token column, RAW 0-based LSP character — fed
                              # straight back into prepareCallHierarchy positions;
                              # prism-side records keep the 0 default


@dataclass(frozen=True)
class DefTarget:
    location: Location
    name: str | None
    kind: str | None


@dataclass(frozen=True)
class CallEdge:
    direction: str            # "caller" | "callee"
    seed: FunctionDef
    other_def: Location | None
    other_name: str | None
    call_site: Location
    resolution_kind: str | None = None


def from_lsp_line(line0: int) -> int:
    """LSP is 0-based; everything internal is 1-based."""
    return line0 + 1


def tie_break(cands: list[FunctionDef]) -> FunctionDef:
    """Deterministic pick: smallest span, then lowest start_line, then file (§2.1)."""
    return min(cands, key=lambda r: (r.location.end_line - r.location.start_line,
                                     r.location.start_line, r.location.file))


def match_by_selection(oracle_fd: FunctionDef,
                       prism_records: list[FunctionDef]) -> FunctionDef | None:
    """§2.4 matching primitive: name equality + oracle selection_line contained in
    the prism record's [start_line, end_line]. Anonymous never matches."""
    if oracle_fd.name is None:
        return None
    cands = [r for r in prism_records
             if r.name == oracle_fd.name
             and r.location.file == oracle_fd.location.file
             and r.location.start_line <= oracle_fd.selection_line <= r.location.end_line]
    return tie_break(cands) if cands else None
