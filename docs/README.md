# Prism Documentation

This directory is organized around current feature docs first, with historical review
artifacts separated into `docs/archive/`.

## Current Map

| Area | Location | Use |
|------|----------|-----|
| CPG | `docs/features/cpg/` | Current CPG architecture, implementation status, and retained design history. |
| Query layer | `docs/features/query-layer/` | Active navigation, import-resolution, evidence, and MCP refresh plans/designs. |
| Accuracy harness | `docs/eval/tier-a/` | Committed Tier-A baselines and run artifacts. |
| Superpowers specs/plans | `docs/superpowers/specs/`, `docs/superpowers/plans/` | Canonical final specs and implementation plans per slice. |
| How-to docs | `docs/how-to/` | Operational documentation for maintainers. |
| Historical review artifacts | `docs/archive/review-artifacts/` | Code-review, plan-review, spec-review, and provenance records. |

## Status Conventions

- **Current:** A document that should be used for planning or implementation now.
- **Historical:** Retained analysis or original design text with a current status note.
- **Archived:** Review/provenance record retained for auditability, not a source of current implementation direction.

## Benchmark Notes

Use fresh benchmarks when planning new speed work. Historical comparisons against
pre-`CpgContext` behavior are useful as context, but they are not current speedup
targets after the shared CPG build landed.
