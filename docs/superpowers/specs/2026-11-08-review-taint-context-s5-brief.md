> Current integration authority — 2026-11-12: This product line is integrated onto freshly fetched main `4f9e3a3eb8d2fc30c59d3741a494eafd6b1d3ac3` in `feat/review-crossfile-context-main-sync-20261112`. All old acceptance/test totals below are inherited historical receipts on their original bases, not validation of this integration. Before each next product slice, synchronize with current main and record the exact resulting base. Every delegate, regardless of provider, runs affected/dependent slice checks; run the full workspace suite before merge. This owner cadence supersedes older per-slice full-suite wording below. Keep required scoped gates and named Tier-A triggers. The twelve historical slices need one final-tree affected integration set, not twelve repeated runs. Preserve pinned historical controls and use the new main base separately for integration regression controls. No push/merge is authorized by this document.

# S5 brief: complete consumer integration and activation

Status: prepared, not dispatched. Depends on reviewed S1-S4; public B stays unavailable until all adapters pass. Root architects/orchestrates, delegated agents implement. Two review rounds per artifact; reconfirm bounded file/line plan before editing.

## One preparation path

Use S2 typed execution and S4 owned context bundle through shared review-owned preparation. Options live on non-exhaustive AlgorithmParams, never core ReviewOptions or exhaustive TaintConfig. High-level review obtains loading authority from original ReviewInputs; additive authorized-preloaded entry points accept that captured handle without alternate repository root.

Preserve old run_algorithm/run_review signatures. Valid imports requested without loading authority returns baseline taint and a visible diagnostic naming the authorized entry point; no context bundle or claim of expansion. Validate before this fallback. Invalid options: Result-returning single/prepared/review APIs return Err; run_review records AlgorithmError for taint, omits taint's result and continues other algorithms on original inputs/context. Preserve existing algorithms_run bookkeeping. When no taint is selected, imports is unused: retain non-taint results and emit one ignored diagnostic. CLI parser still rejects malformed/range-invalid values.

CLI single, multi and targets share preparation. Preserve single annotation versus raw nested multi findings and annotated aggregate findings. Finalize diagrams exactly once. Use one flattened context option group for ReviewArgs and TargetsArgs:

```
--taint-context off|imports          default off
--taint-context-edges exact|nameonly  default nameonly
--taint-context-depth 1..4           default 2
--taint-context-max-files N          default 64
```

Fixed byte/time guards stay8MiB and cooperative20seconds. Callers output still executes no taint or loading: off stays note-free, explicit imports emits one stderr ignored diagnostic without stdout changes. This is distinct from S2's no-flag callers negative.

## Source views and formats

ReviewRun.context owns union files/sources/parse-quality/warnings; ReviewOutcome.inputs remains original. Add a borrowed view selected per algorithm/finding: taint uses completed context maps, others use original maps. Diff metadata always original. Compose run diagnostics once and retain S2 typed notes separately from ordinary warnings.

- filter_result_findings gets an internal view-based implementation and keeps its public wrapper.
- SarifInputs gets optional context maps and per-finding selection. Preserve current schema and normal notifications; no indiscriminate enlarged map for mixed runs.
- targets::project gets an additive context-aware adapter. Use selected maps for parse grading, dependency hints and enclosing site; preserve original diff SHA/files.
- JSON/review select sources per result and populate existing parse_quality fields explicitly; current formatters initialize those maps empty.
- Text selects matching sources. Paper/Mermaid use completed blocks/diagrams and S2 diagnostic channels.

## Statistics and measurement

Keep engine-owned stats even when profiler disabled: rounds started/completed; attempted targeted-source files/actual targeted-source bytes read; completed admitted files/bytes/depth; eligible and loaded-unresolved boundary calls; skips by reason; depth/files/bytes/time bounds; perturbation-dropped and additional sinks/sources before output filtering; discovery/read-parse/build/analysis/total elapsed nanoseconds. Attempted bytes include overflow sentinels, rejected reads and abandoned partial rounds; admitted counters describe only returned completed bundle. The8MiB guard caps retained/staged context source, not cumulative I/O. Existing advisory Go hint reads are not included in the targeted-source byte counter or bounded by it; discovery/total elapsed still covers their work. Do not label that counter all filesystem I/O. Emitted SARIF findings determine measured output population.

Expose optional stats through a narrow generic scalar counter seam in core's existing profiler, inactive when disabled. Review computes the values; core records counter rows. Preserve session ownership, existing phase/timing schema, worker behavior and atomic publication. No new profiler/timer framework, arbitrary JSON extension or direct writes to its output file. Keep primary timing separate from profiling if instrumentation affects it.

## Split and activation checks

| Slice | Deliverable | Estimated production lines |
|---|---|---:|
| S5a | Internal shared preparation, authority/legacy routing, source views | 150-250 |
| S5b | SARIF/targets/filter/render adapters | 200-300 |
| S5c | Optional stats export and complete integration tests | 80-150 |
| S5d | CLI/API exposure, defaults/help/docs, final path verification | 100-180 |

S5d activates only after all required consumers work. Keep helpers in focused modules; main.rs is near its size boundary. Tests need exact-checkpoint RED/GREEN and positive/negative controls: single/multi/targets/high-level/authorized-preloaded/unsupported-preloaded; invalid options; every format; off/on; nominal/scoped; accepted confidence floors; mixed/duplicate algorithms; no-taint/callers; partial bounds; context edit on baseline cache hit. Assert actual evidence and non-taint output identity, not just serialized finding text.

Explicitly cover S4's context descriptions, witness related_files, licensed-source backlinks and context block text in the format matrix. Verify LoadedUnresolved and unfollowed boundary notes, deterministic Bound details, uncapped eligible loading frontier and --scoped-cpg round-zero-only behavior. Documentation destinations are CLAUDE.md's architecture paragraph, current review usage/help/README paths and skills/prism-code-slicing/SKILL.md; explain advisory hints, bounds and deferred languages/import forms without implying full cross-file coverage.

Full workspace tests with totals, formatting, file sizes and scoped authoritative/new/changed/dependent mutation lanes. Tier-A only when its named core trigger is touched; no unrelated harness repair. Then execute the separate preserved120-diff three-arm protocol, report exclusions and strict usefulness/noise/cost. Do not call historical M2 fresh proof. Root owns commits and publication decisions; no delegated push/merge.

## Provenance

Based on evidence_audit's read-only S5 map and propagation refinement, reconciled with original #602 and S2/S4 ownership contracts. No S5 implementation/runtime evidence yet.
