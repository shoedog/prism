# Bounded non-authorizing admission diagnostics

Base: PR299 merge `3e1306a3`. Owner approved the recommended next slice.
Two self-review rounds; no delegates. Stop on open-class findings or scope expansion.

## Contract

Existing CLI/API and eager MCP startup acquisition errors retain their category prefix and append
`; owner_admission=<compact JSON>`, schema `prism.owner-admission/1`.
Served MCP build_failed payloads retain the old bounded cause and carry a separate
owner_admission field from an internal typed error, not parsed text. The field
has a 2048-byte serialized cap; oversize is explicitly owner_admission_omitted.
The default runtime supplies no diagnostic and retains its existing payload.
Selection syntax/cache/startup-policy errors remain outside this report.
No new mode, flags, compiler discovery, installation, source selection or policy.

Report fixed aggregate input facts from the already loaded repository: loaded and
JS/TS file counts, JS/TS byte count, language counts, TypeDatabase presence,
existing file/byte limits and comparison booleans. These are observations, not
successful gate evaluations, complete filesystem inventory or proof authority.
No paths, source, receiver names or raw compiler diagnostics in the report.

Explicit phases: load_inputs, select_inputs, prepare_inputs, locate_inputs,
compiler_evidence, owner_mapping, session_build. On error show the failed phase;
earlier phases completed, later phases not_reached. Compiler_evidence groups
acquisition/reproduction/closure/census validation; failed does not mean every
internal step ran or that the compiler executed. Do not infer phases from reason
strings. Reporting never gates execution or supplies a serialized proof.

Preserve active-slot invalidation before fallible work, actual gate order, all
limits, first refusal category, full input census, duplicate/write/closure/cache
barriers, default behavior and the private staging seam's original error strings.
Share existing numeric constants with the reporter to prevent threshold drift.

## Tests and gates

Capture API/CLI report-absence RED on unmodified production code. Cover simultaneous
language/count failures, exact/overflow file and byte boundaries, empty inputs,
missing compiler, load failure, parse failure, and compiler-stage failure without
claiming downstream mapping ran. Check real MCP startup and served failure reports,
including stale-slot invalidation via inherited lifecycle tests. Preserve all
existing positive Exact controls and private per-predicate/substitution fixtures.

Full default Rust, MCP, audit-feature, observer, helpers and authority suites;
Python eval suite. Format/clippy. Release rebuild plus Tier-A matrix and quick
conservatively required for shared owner-session construction edits. Do not
rebaseline; same-environment base controls before attributing failures.

## Hypothesis / probe / result

- Source expectation: replacing selected sessions reaches the existing gates in
  order. Alternate explanation for input_budget is bytes, not count; diagnostic
  tests independently vary both. Read integration/acquire and keep order intact.
- Prism skill callers query returned SymbolNotFound; direct source consumer
  inspection replaces unavailable graph evidence, not a no-callers conclusion.
- Initial RED command selected nonexistent integration_test target: inadmissible
  setup result. Corrected to the listed integration target before behavioral claims.
- Captured corrected RED: three API/CLI cases fail for missing report, 0/3 pass;
  the same cases pass after wiring. Existing refusal categories match before/after.
- Round 1 closed WRONG: served MCP cause clamps at256 bytes and truncates the new
  report. Initial test erroneously read error rather than cause (inadmissible);
  corrected test captures truncated JSON and EOF. A typed error plus separate
  bounded MCP field fixes this without changing the untrusted-text cap. Transport
  boundary/oversize/default controls and fresh failure/restore checks added.
