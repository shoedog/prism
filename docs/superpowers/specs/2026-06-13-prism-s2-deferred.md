# S2 — Deferred Work (execution-discovered)

Follow-ups surfaced during S2 subagent execution. None are correctness/identity
regressions — byte fields are additive (never in any key/Ord/Eq/Hash), so these are
span-*quality* or coverage refinements. Priority L unless noted.

## Task 4 — span extraction

1. **Alias-resolved destructuring Defs carry a zero-width line anchor, not the raw
   occurrence span.** (Priority L.) For `const { name } = device` the resolved-alias Def
   (`device.name`) is registered with `line_start_byte` for both ends.
   *Why acceptable now:* this is the spec §3 "synthesized / alias-resolved (no clean node)
   → best-effort line anchor" case; byte is additive, so data flow / de-conflation /
   identity are unaffected. Only the Task-6 wire byte for these specific occurrences is
   coarse (zero-width) rather than the raw extent.
   *Fix sketch:* thread the raw destructuring occurrence's node span into the alias-
   resolved `VarLocation` before the line-anchored fallback registers it (so it wins the
   byte-excluded dedup), per the §3 alias raw/resolved rule. Reviewer flagged in Task 4.

2. **Multiline-signature same-line body references.** (Priority L.) `has_bare_references`
   excludes references on each parameter's declaration line; when a signature closes and
   the body starts on the same physical line (`def f(\n    x): return x`), a legitimate
   same-line body use can be dropped from the line-anchored scan.
   *Why acceptable now:* real-span rvalue extraction still captures the use; only the
   line-anchored *supplement* over-excludes, and same-line-signature-and-body is rare.
   *Fix sketch:* compute the parameter byte range and exclude only that range (not the
   whole line), preserving same-line body rvalue spans.

## Task 4 — coverage bookkeeping

3. **New `lang_python` test target.** ✅ RESOLVED in Task 10 — verified **no change
   needed.** Task 4 added `tests/lang/python/{main.rs,span_test.rs}` and a
   `[[test]] name = "lang_python"` target (Python previously had no per-language target).
   The `span_test.rs` files are span-*extraction* tests, not algorithm tests, so they do
   not belong in the algo×language matrix; Python's algorithm coverage already comes from
   `tests/algo/*` (via `make_python_test`). `coverage_test::*` (all 4) and
   `umbrella_completeness_test` pass as-is (the new files are registered in their lang
   `main.rs`), so the matrix does not under-report.
