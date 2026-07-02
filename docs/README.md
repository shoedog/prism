# Prism Documentation

This directory is organized around current feature docs first, with historical review
artifacts separated into `docs/archive/`.

## Current Map

| Area | Location | Use |
|------|----------|-----|
| CPG | `docs/features/cpg/` | Current CPG architecture, implementation status, and retained design history. |
| Query layer | `docs/features/query-layer/` | Active navigation, import-resolution, evidence, MCP refresh, and MCP serving plans/designs. |
| Language coverage | `docs/features/language-coverage/` | Current language expansion, coverage, and language-specific analysis plans. |
| Type system | `docs/features/type-system/` | Current multi-language type-system design material. |
| Documentation organization | `docs/features/documentation/` | Documentation routing rules, cleanup plans, and move inventories. |
| Accuracy harness | `docs/eval/tier-a/` | Committed Tier-A baselines and run artifacts. |
| Superpowers specs/plans | `docs/superpowers/specs/`, `docs/superpowers/plans/` | Canonical final specs and implementation plans per slice. |
| How-to docs | `docs/how-to/` | Operational documentation for maintainers. |
| MCP guide | `docs/MCP.md` | Stable agent-facing setup and user guide. |
| Archive | `docs/archive/` | Historical analysis, plans, review artifacts, and operational incident notes. |

## Status Conventions

- **Current:** A document that should be used for planning or implementation now.
- **Historical:** Retained analysis or original design text with a current status note.
- **Archived:** Review/provenance record retained for auditability, not a source of current implementation direction.

## Benchmark Notes

Use fresh benchmarks when planning new speed work. Historical comparisons against
pre-`CpgContext` behavior are useful as context, but they are not current speedup
targets after the shared CPG build landed.
