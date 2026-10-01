# Implementer brief: S1b sub-slice `__SLICE__` (S1b-1, S1b-1b, S1b-2a, S1b-2b, S1b-3 / 3a / 3b or S1b-4)

> **Owner decisions (SPEC §0): Option K, OQ1–OQ7 = (a), and after spec round 3 "targeted fold, then implement".**
> S1b-2 dispatches as **S1b-2a** and **S1b-2b**; S1b-3 carries the evaluation-context table. The owned paths and caps
> are in the "Re-plan" blocks and the **S1b-2a dispatch** section below; the r2 "S1b-2" rows are superseded.
> **S1b-1 is unchanged by the re-plan** (OQ6). S1b-1b (the F4 collector fix) exists only if the owner answers OQ8 (a).

You implement **exactly one sub-slice** of `SPEC.md` in this directory, as the controller names it at dispatch, on
the owner's §0 answers (the controller pastes them into the dispatch; if they differ from a recommendation, the
owner's answer wins). Read `SPEC.md` in full, then `PLANNING-PROBES.md` (mechanisms M1–M14), then `CLAUDE.md`,
whose conventions are binding: files under 600 lines, BTreeMap determinism, the Exact static-binding paragraph, and
the Tier-A rule (you touch `src/resolution.rs`, `src/js_exports.rs` and `src/ast*`). If the dispatch contradicts the
SPEC, stop and ask.

## The model, in one paragraph

Exact is a static-binding grade. S1b makes every JS/TS route bind a name only to the one callable its binding
holds, found by the nearest scope that declares it (SPEC §3.1's table), or refuse. **May-call bindings (SPEC §3.1 M1,
M2; owner E5) keep base behavior exactly**: never change their rows. Runtime mutation, `eval`, host globals and
`require`/`import()` re-acquisition are out of model: add **no** guard for them. `with` is in model. Do not add
refusals the SPEC does not list, and do not drop edges the SPEC does not drop: the owner's rule is "remove only wrong
edges".

## Ground rules

- **Branch** from the planning commit that contains this packet (S1b-1), or from the merged previous sub-slice.
  Record the base SHA.
- **Owned paths, by sub-slice.**
  - S1b-1: the intrinsic guard at the top of `resolve_call_site_full` and the `JsxIntrinsic` /
    `LocalBindingUnproven` `DropReason` variants; `src/navigation/queries.rs` (drop match, keys); the shared-collector
    fixes F1–F3 in `src/ast.rs` (`collect_js_ts_binding_pattern_names`, `collect_js_ts_parameter_bindings`); caches;
    tests; `s1b_jsx_intrinsic_tag_refused`.
  - S1b-2: new `src/ast/js_binding*.rs` (the §3.1 table, fail-safe, sealed-error rule, classification, module
    terminal), **split across files so each stays under 600 lines**; the four producer lines in
    `collect_js_ts_export_statement`; `src/js_exports.rs`; `js_ts_import_member_candidates` (the `wrapped` gate,
    deleting S1's lowercase check); the `local_route` flag on S1's predicate; caches; tests;
    `s1b_list_export_nested_decoy_refused`.
  - S1b-3: `src/ast/js_binding*.rs` (site walk, J1/J2, nested scope arms, scoped write index, memo);
    `src/ast/js_wrapped_export.rs` (`useCallback` on the local route); `CallSite.local_binding` and the three source
    constructors in `src/call_graph.rs`; the R4 arm of `resolve_call_site_full`; caches; tests;
    `s1b_param_shadow_local_def_refused`.
  - S1b-4: the R3 block of `resolve_call_site_full` and its helpers (`js_ts_export_candidates` shared with R4c); the
    qualifier proof at extraction; caches; tests; `s1b_namespace_nested_decoy_refused`; the
    `module_binding_audit_test::esm_namespace_import` update.
  - **Re-plan (owner OQ3 = a):**
    - **S1b-2a**: `src/ast/js_binding*.rs` scope index, D/P/W rows, classification, B0 **including leaf kinds**, B1
      with **structural brace tokens** and **delimited-child sealing** (SPEC §3.1 B1 note), the module terminal; the
      derivation probe's second table; **0 corpus rows** (row-diff byte-identical to base on X, R, T, F). No site
      walk, no `CallSite` change, no producer wiring.
    - **S1b-2b**: the four producer lines, `js_exports.rs` (`VerifiedLocal`, `wrapped`, the barrel key), the R4c gate,
      `ModuleExportName` (StringValue for string-literal export **and** import specifiers, `src/ast.rs` collectors);
      **0 corpus rows**.
    - **S1b-3**: the site walk **driven by the E-table** (`E(kind, field)` consulted at every step; J1, J2, J3; the
      `with` object; enum bodies as T10), the **leave predicate** (same-file partner index by first name segment,
      script-file check, dotted-name segments), the **positional fail-safe** (`Unchecked(reason)` +
      `local_binding_unchecked_position`, OQ1 a), the J2 split by holder, the Annex-B marker consumers, the scoped
      write index, `CallSite.local_binding`, `useCallback`, and the Rust `E_TABLE` equality test (SPEC §7 C-35).
      Row-diff expected byte-identical to v8d's S1b-3 expectations (the unproven set is empty on the corpora, RP3,
      and the r3 fold changes 0 rows, Q42).
- **Forbidden:** `Language::function_name`, `FunctionId`, call-site ownership, `CallKind`, DFG and Step 5/5b code;
  CJS `Local` production; R3 for named, default or `require` qualifiers (E8); `IndirectResolution` sites (E10); any
  runtime-mutation guard; new dependencies; edits to existing Tier-A fixtures; running on the public or private
  corpora (acceptance belongs to the controller); `CLAUDE.md` (no edit is planned for S1b).
- **The prototype is evidence, not authority.** `prototype/s1b-prototype-v9.diff.txt` (r3; it replaces v8's) builds, reproduces every
  control and every expected row-diff, and passes the suite apart from the 9 by-design pins (PLANNING-PROBES Q26–Q35).
  It is feasibility code: `js_binding.rs` is one file of about 900 lines (split it), and it carries measurement-only
  switches (`PRISM_S1B_UPTO`, `PRISM_S1B_BROAD_E6`, `PRISM_S1B_NO_FAILSAFE`, `PRISM_S1B_SCOPED_MODULE_SCAN`,
  `PRISM_S1B_E6_TYPESPACE`, `PRISM_S1B_DEBUG`; **do not port them**), no tests and no cache bumps. Where it and the
  SPEC differ, the SPEC wins. The module terminal (S1b-2) uses the base `js_ts_module_value_written` after F1–F3
  (identical to the scoped scan on all four corpora and all controls, Q28); S1b-3 adds the scoped scan for nested
  scopes.

## Order of work (TDD)

1. **RED first.** Write the sub-slice's RED rows (SPEC §7: S1b-1 A-1, A-3; S1b-2 B-1, B-2, B-3; S1b-3 C-1, C-2, C-3;
   S1b-4 D-1, D-4)
   and its Tier-A fixtures. Add the new types with no behavior so they compile. Capture a behavioral failure with
   its concrete value; compile, setup and zero-test failures are inadmissible.
2. **Implement** the sub-slice's SPEC sections. Turn RED green.
3. **The rest of the sub-slice's §7 table**, both grammars per row, exact targets with decoys. Update the existing
   tests SPEC §7 lists for this sub-slice, and no others; if any other existing test fails, stop and report it.
4. **Mutants**, each applied alone, recording the test that kills it. A surviving bounded mutant is a coverage gap:
   add the killing row.
5. **Synthetic smoke.** `python3 probes/controls_gen.py` in an empty directory (240 scenarios since r3), then
   `probes/run_controls.sh <bin> <gen_dir> <out_dir>` and `probes/controls_diff.py` against the reference summary for
   your sub-slice (base is `probes/S1b-controls-base.txt`):
   - S1b-1: base, except the intrinsic relabels and removals and the F1–F3 rows (C124 declaration, C126);
   - S1b-2: `probes/S1b-controls-proto-v8-upto1.txt` (the prototype's switch 1 covers S1b-1 plus S1b-2);
   - S1b-3: `probes/S1b-controls-proto-v8-upto2.txt`;
   - S1b-4: `probes/S1b-controls-proto-v8.txt`.
   Explain any difference row by row. Also run `python3 probes/grammar_closure.py` (exit 0).

## Budget and checkpoints

Honest lines: after `cargo fmt`, non-blank, non-`//`; `#[cfg(test)]` and `tests/**` are tests
(`probes/honest_lines.py <repo> <commit> --per-file` on a squashed commit).

| Sub-slice | src cap / early stop | tests cap / report point | combined |
|---|---|---|---|
| S1b-1 | 60 / 54 | 200 / 180 | 260 |
| S1b-2 | 620 / 560 | 750 / 675 | 1,370 |
| S1b-3 | 330 / 300 | 600 / 540 | 930 |
| S1b-4 | 185 / 165 | 330 / 300 | 515 |

These are the proposed caps (SPEC §9, owner E13); the dispatch states the owner's answer.

**Re-plan caps (owner OQ3 = a; OQ9 re-caps open, SPEC §9):** S1b-2a **510** / 465 src, **560** / 500 tests; S1b-2b **140** / 125,
**200** / 180; S1b-3 **480** / 430, **700** / 630; S1b-1 and S1b-4 unchanged. Synthetic smoke for the re-plan
sub-slices: 2a and 2b against `probes/S1b-controls-base.txt` plus S1b-1's rows (2a) and
`probes/S1b-controls-proto-v8-upto1.txt` (2b); S1b-3 against `…-upto2.txt` **plus** the RP1 fixtures
(`~/prism-evidence/s1b/replan/fixtures/`, expected results in `RESULTS.md` with the re-plan's *correct* column, not
v8's).

**Report checkpoints (mandatory):**
1. After the RED commit: src and tests so far.
2. After the production code is green: src count, and the forecast for the cache bumps.
3. **After every test batch** (every table in §7 you finish): the tests count and the forecast. S1 overran its
   test estimate because batches were written without measurement.
4. At an early stop or report point: the count, the forecast, and the remaining items. Continue only while the
   forecast is within the cap; a forecast above a cap is a stop with the enumerated remainder. Never compress logic
   to fit. Lines should be at most 100 columns.

## S1b-2a dispatch (the collector at module scope; SPEC §3.1, §3.1a closure, §6, §7, §9)

- **Base:** the merged S1b-1 head (or S1b-1b's, if OQ8 = a). Record the SHA.
- **Caps:** **src 510 / early stop 465; tests 560 / report point 500.** **Disclosed forecast: the v9 prototype's 2a
  share measures 557 src (SPEC §9, OQ9 recommends 615 / 555).** Until the owner raises the cap, expect the forecast to
  cross 465: that is a **stop with the enumerated remainder** at checkpoint 2 or earlier, never compression. If the
  dispatch carries the owner's OQ9 answer, those caps apply instead.
- **Owned paths:** new `src/ast/js_binding*.rs` (split so each file is under 600 lines): `CLASSIFIED` (the 186-kind
  allowlist, leaves included), `JsBinding`, `JsTerminal`, `JsBindingCache`, `Strictness` and `js_ts_strictness` (the
  Annex-B predicate, SPEC §3.1 D1), the declaration walk (D, P, W rows, the Annex-B marker only under the predicate),
  the module-scope index, classification B0–B3 and M1/M2, B1 with structural brace tokens and delimited-child sealing,
  `js_ts_scope_clean`, `js_ts_using_declaration`, `js_ts_import_value_names`, and the module terminal; the `mod` line in
  `src/ast.rs`. **Not in 2a:** the site walk, `E_TABLE`, `CallSite`, the producer lines, `js_exports.rs`,
  `ModuleExportName`, any resolution rung, any cache bump (SPEC §6: 2a changes no persisted byte; the review checks
  `git diff` over the serde types and cache writers is empty).
- **`dead_code`:** the module is compiled but not called from production code in 2a. Put one
  `#![allow(dead_code)] // S1b-2b and S1b-3 wire these; remove the allow there` at the top of each new file (not per
  item, not a crate-wide allow). **Do not add a test-only entry point:** the tests are `#[cfg(test)]` unit tests in a
  sibling file (the repo's `src/*_tests.rs` pattern), calling the module's `pub(crate)`/`pub(super)` API directly.
- **RED rows (unit level: the module terminal's result for the exported name, not an edge; 2b re-asserts them as
  edges):** B-4 C74, C75 (`Refused`, B2; C74 `duplicate_declaration`); B-5 C78 (`Callable(g's span)`); B-6 C79 (`Refused("parse_recovery")`),
  C149, C150 (sealed: `Callable`); B-8 C143, C144, C146 (TS: the value binding, not the type); B-10 the closure unit
  test; **B-14** RP2-a header error → `Refused`; **B-15** RP2-b string brace → `Callable`, both quote forms; **B-18
  (r3, sol W1) strictness:** `js_ts_strictness` is Strict for a module (`import`/`export`, `.mjs`, `.mts`), a program
  or enclosing-function `"use strict"`/`'use strict'` directive and class code, Sloppy for a `.cjs` script, Unknown for
  a `.js`/`.ts` script without a directive; the declaration walk records the Annex-B marker for C163's block function
  and not for C111 (module), C161 (directive) or C164 (generator); Unknown yields the annex set, not a marker (C159,
  C160). Each row in both grammars (JS and TSX) unless TS-only.
- **Mutants:** B-M2 skip B2; B-M3 broad B1; B-M4 drop B1 (i); B-M5 `interface`/`type` as declarations; B-M8 one
  allowlist kind removed; B-M9 raw-text brace test; B-M10 header errors sealed; **C-M22** unconditional Annex-B marker
  (killed by B-18's C111/C161 rows); **C-M23** marker for generator/async declarations (killed by C164's row).
- **Closure probe:** `python3 probes/grammar_closure.py --rust src/ast --kinds-only` must exit 0 (2a has no runtime
  `E_TABLE`; S1b-3 drops `--kinds-only`). Check the exit status directly, not through a pipe. Also report that removing
  one kind from `CLASSIFIED` makes it exit 1.
- **Smoke:** 0 rows. `probes/run_controls.sh` with your binary and with the base (S1b-1 head) binary on the same
  `controls_gen.py` output (240 scenarios): the two summaries must be byte-identical. Corpus row-diffs are the
  controller's (byte-identical to the base on X, F, R, T).
- **Suites:** the Verification block below; `cargo clippy` must be clean without any allow beyond the file-level
  `dead_code` one.

## S1b-3a dispatch (the collector at every scope; SPEC §3.8, owner decisions 2026-09-27/28/29)

Read SPEC §3.8 first (the normative delta against the landed collector, including (11) what drops and why, (12) P1
and (13) the file split). The prototype is `prototype/s1b3-prototype-v12.diff.txt` (clean build of scratch
`0968ef78` on `3961cc21`; evidence, not authority; its switches `PRISM_S1B3_NO_SITES` and `PRISM_S1B3_ASCII_P1` are
**not ported**).

- **Base:** main with S1b-2b merged (`3961cc21` or its merge). Record the SHA.
- **Cache:** **102 / 58**, with the cross-commit row (a cache from the parent's binary is rejected; the rebuilt output
  equals `--no-cache`). 3a changes persisted D4 facts (C166–C168, C172).
- **Caps (owner-approved 2026-09-29):** src **700** / early stop **630**; tests **740** / report point **670**.
  Measured on v12: 596 src (×1.1–1.2 landing growth ≈ 656–715: report at 630 with the forecast, stop at 700 with the
  enumerated remainder; never compress). Tests forecast ≈ 710.
- **Owned paths:** `src/ast/js_binding*.rs` split per §3.8 (13): `js_binding_walk.rs` (new), `js_binding_site.rs`
  (new), `js_binding_writes.rs` (new), `js_binding_recovery.rs` (new, or in `js_binding_checks.rs`),
  `js_binding_values.rs` (new, moved from `js_binding.rs`); `js_ts_scope_binding`; `js_ts_classify` (M2 every kind,
  M1 pattern declarators, `Import`, `Alias`, NoFn, the default test, the D4 alias arm); B1 containment in
  `js_binding_checks.rs`; the deletion of `js_ts_written_unseen`; `JsBindingCache` fields; P1 in
  `collect_js_ts_binding_pattern_names` (`src/ast.rs`) with preservation tests; `mod` lines; the cache bump; 2a's
  unit tests it changes (C150; D5 → `Import`). `js_ts_site_binding` has no production caller yet: one file-level
  `#![allow(dead_code)] // S1b-3b wires the call-site binding` in `js_binding_site.rs` only. **Not in 3a:**
  `CallSite`, `call_graph.rs`, `resolution.rs`, `queries.rs`, `useCallback`. New test files per §3.8 (13).
- **RED rows (SPEC §7), unit level (the collector's answer), both grammars:** C-36, C-37, C-38, C-39, C-40, C-45,
  C-46, C-47, **C-48** (defaults), **C-49** (trivia), the **C-5/C-6 E5 guards (moved here as unit rows)**, the walk
  rows C-2–C-4, C-7, C-9–C-17, C-24–C-35, and C-35's `E_TABLE` equality test.
- **Mutants:** C-M2a/b, C-M3–C-M11, C-M15–C-M32, C-M38–C-M45 (as they apply to the collector).
- **Closure probe:** `python3 probes/grammar_closure.py --rust src/ast` (no `--kinds-only`) exits 0; one `E_TABLE`
  row removed exits 1. Read the exit status directly.
- **Smoke:** `probes/controls_gen.py` (287 scenarios) + `probes/run_controls.sh`; `probes/controls_diff.py` against
  `probes/S1b-controls-head-3961cc21.txt`: only the C166–C168 and C172 D4 rows change (the same as v12 with
  `PRISM_S1B3_NO_SITES=1`). **Corpus: 0 rows on X, F, R, T** (the controller's acceptance).

## S1b-3b dispatch (the call-site wiring; SPEC §3.6, §3.8 (8)(9))

- **Base:** merged S1b-3a. Record the SHA.
- **Cache:** **103 / 59** (`CallSite.local_binding`), with the cross-commit row. S1b-4 then 104 / 60.
- **Caps (owner-approved 2026-09-29):** src **220** / early stop **200**; tests **1,320** / report point **1,190**.
  Measured on v12: 161 src (×1.33 ≈ 214). Tests forecast ≈ 1,260 (36 enumerated scenarios × 30 + 120, plus 2
  round-2 scenarios); report at 1,190 with the forecast.
- **Owned paths:** `CallSite.local_binding` + `JsLocalBinding` (`Callable`, `MayCall(reason)`, `Unproven(reason)`,
  `Position(reason)`, `Unchecked`) and the three source constructors (one `JsBindingCache::for_sites()` per file)
  plus the test-only `CallSite` literals; `js_local_binding_at`; `resolve_call_site_full` (Callable first, R4 drop on
  any `Unproven`, R5 drop for `not_callable` / `duplicate_declaration` / `import_parse_recovery` where base would
  bind); `js_ts_local_callable`; the `useCallback` `local_route` flag; `local_binding_unchecked_position` and
  `local_binding_may_call`; removing 3a's `dead_code` allow; the cache bump; `s1b_param_shadow_local_def_refused`
  (Tier-A; RED on `3961cc21`, green on v12).
- **RED rows (SPEC §7, §9), end to end, both grammars:** the 36 scenarios §9 enumerates (C-1, the six outcome
  representatives, C-8, C-18, C-20, C-41, C-42 with C179/C181, C-43, C-44, the alias pins C174–C177 and C182–C184,
  C-5's impostor twins, C114) plus 2 round-2 scenarios (one default from C185–C188, one trivia from C189). The C-5 and
  C-6 guards are 3a's unit rows, not duplicated here.
- **Mutants:** C-M1, C-M12–C-M14, C-M33–C-M37.
- **Smoke:** controls against `probes/S1b-controls-proto-v12.txt` (identical, apart from rows explained one by one);
  RP replay equal to v9's except RP2-c (S1b-4).
- **Corpus acceptance** (controller): SPEC §8's S1b-3 r4 row, including the owner-accepted parameter value-flow
  cost (`probes/param_valueflow.py`: X 6, F 2, T 5 lost under the single-caller rule, lower bounds);
  `probes/expected/S1b-3-r4-{X,R,T}.json`.

## Verification (report totals from logs)

```bash
cargo fmt --all -- --check
cargo clippy --offline --all-targets --features mcp -- -W clippy::all
cargo test --offline --no-fail-fast                 # base a6d853f5: 4,583 / 0 / 1 (29 binaries)
cargo test --offline --features mcp --no-fail-fast
cargo build --release
cd eval && uv run tier-a --matrix-only --allow-stale-sut      # base 162/162; expect +1 new per sub-slice, 0 regressions
cd eval && uv run tier-a --quick --allow-stale-sut            # needs rust-analyzer; else report "not run" and why
node scripts/gate-inputs/acquire.mjs && node scripts/gate-inputs/gate.mjs --out target/gate-runs/<name>
```

- `--allow-stale-sut` only straight after the `cargo build --release` in the same worktree.
- A failure outside the sub-slice's scope is reported as found, never fixed or re-baselined.
- Paste Tier-A regressions or flip candidates verbatim.

## Handback

- Base and head SHAs; `git diff --stat <base> -- src tests eval Cargo.toml Cargo.lock`.
- Honest lines per bucket and per file, with every checkpoint report.
- RED: the command and the failing excerpt with its concrete value, per new path.
- The mutant kill table.
- Suite totals (default, mcp, clippy, Tier-A matrix and quick, Node gate) with log paths.
- The synthetic smoke diff against the sub-slice's reference summary.
- Deviations from the SPEC, each with its reason; each is a question for the owner.
- Commit on the branch with the session's attribution trailers. Do not push or merge.

> **READ historical caps:** tests 820 (2026-09-29), then 920 (2026-09-30), src 700.
> The later owner decision 2026-09-30 abolishes all numeric LOC caps; see SPEC §0.

## S1b-4 dispatch

READ (2026-10-01): this is a **draft, not dispatchable**. OQ-S1b4-1 must be
answered, prototype custody provided, the SPEC §8 pending row measured, fresh
r2 expected rows and the proto reference summary committed, and the two-round
plan review completed before the controller dispatches Sonnet. Never infer
approval from a recommendation or a base-only reference.

READ: base is `915fca43d84ea1730959453091fbf8ae97763af8` (S1b-3b merged).
Implementation branches from the reviewed planning packet bound to that source
base; record both SHAs. Cache is **104 / 60** with a real parent-binary cache
transition. No LOC caps or numeric stop-and-ask apply. Forecast is SPEC §9,
320–440 src / 1,000–1,400 tests if namespace alias preservation is approved.
Review cap is **2 rounds**, stated before dispatch; at the cap classify
converging vs open-class and follow the owner convergence discipline.

READ / ASSUMPTION (owned implementation paths, pending measured prototype):
- `src/ast/js_binding_namespace.rs` (new, syntax inventory and proof reusing the
  landed core); `js_binding.rs` (per-file namespace memo only), `src/ast.rs`
  (module wiring). No E_TABLE, scope/write/recovery/wrapper redesign.
- `src/call_graph.rs`: NamespaceImport variant of JsLocalBinding and extension
  of `js_local_binding_at` at the three existing source constructors; optional
  narrow E7 helper factoring from the current relative path calculation.
- `src/resolution.rs`: namespace R3 route and shared
  `js_ts_export_candidates(candidate_file, member, site)` projection, preserving
  R4c's behavior; E7 fallback. Split the new route into a focused sibling module
  if needed for readability, without another resolution ladder.
- `src/navigation/queries.rs` only if the existing local-binding maps need
  integration beyond their current MayCall/Position match. Count once per site.
- `src/cpg_cache.rs`, `src/navigation/call_edge_cache.rs` and their version pins;
  integration/AST/navigation/CLI tests; Tier-A fixture named below.
- **Conditional on OQ-S1b4-1 preservation:** `src/ast/js_binding.rs` local-export
  fact production and `src/js_exports.rs` plus graph/cache metadata, solely to
  retain a namespace-only alias-opacity signal through the existing bounded
  traversal. D4 targets/counters must remain unchanged. This ownership is not
  granted until the owner's answer is folded into SPEC §3.4.

READ (TDD rows): implement D-1–D-10 from amended SPEC §7 with exact
`(file, registered name, start, end)` assertions, both JSX and TSX end to end.
The old 287 and new C190–C220 controls are generated by controls_gen.py.
Base RED evidence is `target/plan-s1b4/base/controls-r2/`; D-1 C190/C191 shows
base's 2-target decoy fanout; D-2 C214 shows an ordinary wrapped call binding;
D-3 C192/C193 and RP2-c show missing renamed/barrel exports; D-4 C201/C202/C203
and C210 show duplicate/recovered/unsealed/with namespace authority; D-5
C215/C216/C217 and C133 show non-exported, non-sibling and wrong-directory
results. Confirm each actual base row, not just these labels, before attributing
RED. C206/C208/C209 are E_TABLE-position guards unless their base output
demonstrates a behavioral RED. C197–C199/C218/C219 are E5 preservation guards,
C211–C213 are non-goal guards, C220 is the owner-policy guard. Tests containing
type-only/import-equals syntax in .jsx are recovery twins, not valid JavaScript
semantics. The .tsx twins carry the TS semantic assertion.

ASSUMPTION (Tier-A fixture to create):
`eval/fixtures/typescript/s1b_namespace_nested_decoy_refused/` has util.ts with
nested f at line 2 and exported f at a distinct span, and app.ts importing
`* as ns` and calling ns.f. Seed the nested f@2; expect callers = [], exact =
false, forbid_resolution_kind = "import_qualified". This must fail behaviorally
on 915fca43 and pass on the implementation. Add a separate positive unit/nav
assertion that the exported f remains Exact; an empty-callers fixture alone
does not prove recall. Update only
`module_binding_audit_test::esm_namespace_import` if its Gap changes to Supported.

READ (mutants): D-M1–D-M14 are enumerated in SPEC §7. Apply each alone and
record an actual killing test/output. D-M2 must distinguish clean import node
identity from another import binding; D-M5 must kill refusal of a written
import and a written class shadow; D-M8 must preserve an outer import despite
an inner parameter write; D-M10 must test the parameter-default body-only
shadow. D-M6 separately removes span and wrapped checks; D-M11 leaves an
indexed sibling with a missing export and a same-name other-directory decoy.
D-M12/D-M13 are conditional on the alias policy and need direct/named/star
opacity plus a D4 preservation twin. D-M14 includes cached vs fresh output
after qualifier/write/export edits. Setup failures are inadmissible.

READ (reference names to fill before dispatch):
- Synthetic head: `probes/S1b-controls-s1b4-r2-proto.txt` (**not generated yet**).
- Public acceptance: `probes/expected/S1b-4-r2-{X,R,T}.json` (**not generated yet**).
- RP reference: new head replay's correct column, with all four RP2-c twins
  Exact to lib:f@1 via StringValue member g, and the other 42 scenarios at base.
- Measurement/audit record: `S1b-4-MEASUREMENTS.md`; F aggregates from controller.
The original 287 summaries stay identical except rows explained individually.
Do not use controls_diff.py alone: it ignores function-inventory lines and can
miss unpaired trailing rows. Compare full scenario sections and dumped site
keys, and preserve the original generated-source hash comparison.

READ (verification commands; run in the implementation worktree):

```bash
cargo fmt --all -- --check
cargo clippy --offline --all-targets --features mcp -- -W clippy::all
cargo test --offline --no-fail-fast
cargo test --offline --features mcp --no-fail-fast
python3 docs/superpowers/plans/2026-09-26-s1b-export-route-span-verify/probes/grammar_closure.py --rust src/ast
cargo build --offline --release
cd eval
uv run tier-a --matrix-only --allow-stale-sut
uv run tier-a --quick --allow-stale-sut
```

READ: the release rebuild must immediately precede Tier-A in the same worktree.
For the Node gate, run the repo's normal acquire/gate commands only when its
host/cache requirements are available; record any refusal, no baseline edits.
READ base totals in this session: default 4,750/0/1; mcp 4,943/0/1; matrix
165/165. Clippy finished with inherited warnings; do not call it clean.
These are base controls, not implementation receipts.

READ (smoke, set P to this packet's absolute path; keep outputs under target):

```bash
mkdir -p target/plan-s1b4/impl-controls
cd target/plan-s1b4/impl-controls
python3 "$P/probes/controls_gen.py"
bash "$P/probes/run_controls.sh" "$BIN" "$PWD" "$OUT"
python3 "$P/probes/controls_diff.py" "$P/probes/S1b-controls-s1b4-r2-proto.txt" "$OUT/SUMMARY.txt"
bash "$P/probes/replan/replay_rp.sh" "$BIN" "$RP_OUT"
```

READ (cache acceptance): generate the C190 parent nav cache with the recorded
915fca43 binary using `nav --cache-dir "$CACHE" call-stats --repo "$FIXTURE"`;
then run the same command with the head. Compare head warm/rebuilt output with
head `nav --no-cache call-stats --repo "$FIXTURE"`, including exact exported
target, decoy absence and binding counters. Assert parent version 59 is rejected
by 60. CPG's independent cache row must similarly reject parent 103 at 104;
record a real parent cache artifact plus the head rejection/rebuild receipt,
not only a changed version-pin test. Test cold/full-hit/partial-hit and an
incremental edit of the qualifier, a reaching write and the producer export.

READ (controller corpus acceptance): use `run_dumps.sh` with explicit X R T or
F; never its default list in planner/implementer automation. Run rowdiff.py,
audit_s1b4.py and valueflow_guard_s1b4.py, hand-audit every changed identity,
then compare the exact expected JSON. `CONTROLLER-S1b4.sh` contains the private
F commands and returns aggregates only. Neither planner nor implementer reads F.

READ (handback): base/plan/head SHAs; clean status, source patch and source/binary
SHA256; honest lines by file/bucket; RED output per new path; every mutant and
its killing receipt; smoke/RP/full-section diff; complete corpus dumps/diffs,
row classes and independent flow audit; cache cross-commit receipts; suite
totals/logs and exactly excluded checks; Tier-A regressions/flip candidates;
owner-policy deviations; refreshed handoff. Commit stable points, do not push.
If Git writes remain controller-only, hand back the docs/patch snapshot and
explicit-path commit instructions; never report an uncreated commit SHA.
