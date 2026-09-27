# Implementer brief: S1b sub-slice `__SLICE__` (S1b-1, S1b-2, S1b-3 or S1b-4)

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
- **Forbidden:** `Language::function_name`, `FunctionId`, call-site ownership, `CallKind`, DFG and Step 5/5b code;
  CJS `Local` production; R3 for named, default or `require` qualifiers (E8); `IndirectResolution` sites (E10); any
  runtime-mutation guard; new dependencies; edits to existing Tier-A fixtures; running on the public or private
  corpora (acceptance belongs to the controller); `CLAUDE.md` (no edit is planned for S1b).
- **The prototype is evidence, not authority.** `prototype/s1b-prototype-v8.diff.txt` builds, reproduces every
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
5. **Synthetic smoke.** `python3 probes/controls_gen.py` in an empty directory (218 scenarios), then
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

**Report checkpoints (mandatory):**
1. After the RED commit: src and tests so far.
2. After the production code is green: src count, and the forecast for the cache bumps.
3. **After every test batch** (every table in §7 you finish): the tests count and the forecast. S1 overran its
   test estimate because batches were written without measurement.
4. At an early stop or report point: the count, the forecast, and the remaining items. Continue only while the
   forecast is within the cap; a forecast above a cap is a stop with the enumerated remainder. Never compress logic
   to fit. Lines should be at most 100 columns.

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
