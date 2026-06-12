"""M2 comparison (spec §2.5): site-level (primary) + caller-function level."""
from __future__ import annotations

from dataclasses import dataclass, field

from .model import CallEdge, FunctionDef


def collapse_sites(edges: list[CallEdge]) -> tuple[set, int]:
    """§2.5: same-line multi-calls collapse to one countable site, counted."""
    sites = [(e.call_site.file, e.call_site.start_line) for e in edges]
    return set(sites), len(sites) - len(set(sites))


@dataclass
class SiteResult:
    tp: set = field(default_factory=set)
    fp: set = field(default_factory=set)   # prism_only sites
    fn: set = field(default_factory=set)   # oracle_only sites
    collapsed: int = 0


# Multi-line method chains: prism reports the RECEIVER line, LSP oracles report the
# NAME-token line, which can sit up to a couple of continuation lines later
# (`ctx.cpg\n    .callers_of_in_file(...)`). Without tolerance every such call mints
# a phantom FP/FN pair (adjudication-validated finding, 2026-06-12 sample).
CHAIN_TOLERANCE = 2


def site_compare(prism: list[CallEdge], oracle: list[CallEdge]) -> SiteResult:
    r = SiteResult()
    psites, c1 = collapse_sites(prism)
    r.collapsed = c1
    oracle_windows = sorted({
        (o.call_site.file, o.call_site.start_line, o.call_site.end_line)
        for o in oracle
    })
    matched_oracle_sites = set()
    for f, line in sorted(psites):
        hit = next((
            (of, start, end) for of, start, end in oracle_windows
            if (of, start) not in matched_oracle_sites
            and of == f
            and start - CHAIN_TOLERANCE <= line <= end
        ), None)
        if hit is not None:
            r.tp.add((f, line))
            of, start, _end = hit
            matched_oracle_sites.add((of, start))
        else:
            r.fp.add((f, line))
    osites, _ = collapse_sites(oracle)
    r.fn = {s for s in osites if s not in matched_oracle_sites}
    return r


def caller_fn_sets(edges: list[CallEdge], inventory: list[FunctionDef]) -> set:
    """§2.5 coarse granularity; sites outside any inventoried fn -> module_level."""
    out = set()
    for e in edges:
        f, line = e.call_site.file, e.call_site.start_line
        within = [fd for fd in inventory if fd.location.file == f
                  and fd.location.start_line <= line <= fd.location.end_line]
        best = min(within, key=lambda fd: fd.location.end_line - fd.location.start_line,
                   default=None)
        out.add((f, best.name if best else "<module_level>"))
    return out
