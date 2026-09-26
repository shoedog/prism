# Implementer brief: S1b sub-slice `__SLICE__` (S1b-1, S1b-2 or S1b-3)

You implement **exactly one sub-slice** of `SPEC.md` in this directory, as the controller names it at dispatch, on
the owner's §0 answers (the controller pastes them into the dispatch; if they differ from a recommendation, the
owner's answer wins). Read `SPEC.md` in full, then `PLANNING-PROBES.md` (mechanisms M1–M14), then `CLAUDE.md`,
whose conventions are binding: files under 600 lines, BTreeMap determinism, the Exact static-binding paragraph, and
the Tier-A rule (you touch `src/resolution.rs`, `src/js_exports.rs` and `src/ast*`). If the dispatch contradicts the
SPEC, stop and ask.

## The model, in one paragraph

Exact is a static-binding grade. S1b makes every JS/TS route bind a name only to the one callable its binding
holds, found by the nearest scope that declares it (SPEC §3.1), or refuse. Runtime mutation, `eval`, host globals
and `require`/`import()` re-acquisition are out of model: add **no** guard for them. `with` is in model (it is a
scope that declares every name). Do not add refusals the SPEC does not list, and do not drop edges the SPEC does not
drop: the owner's rule is "remove only wrong edges".

## Ground rules

- **Branch** from the planning commit that contains this packet (S1b-1), or from the merged previous sub-slice.
  Record the base SHA.
- **Owned paths, by sub-slice.**
  - S1b-1: new `src/ast/js_binding.rs` (the declaration collector and the terminal, SPEC §3.1; under 600 lines) and
    its `mod` line; the four producer lines in `collect_js_ts_export_statement` (`src/ast.rs`, M1);
    `src/js_exports.rs` (§3.2 model); `js_ts_import_member_candidates` in `src/resolution.rs` (§3.3); the intrinsic
    guard at the top of `resolve_call_site_full` and the two `DropReason` variants (§3.5); `src/navigation/queries.rs`
    (drop match, keys); both cache files; struct-literal fix-ups forced by new fields; the tests named in SPEC §7; the
    S1b-1 Tier-A fixtures.
  - S1b-2: `src/ast/js_binding.rs` (nested scopes, the per-site walk, the memo, the scoped write scan);
    `src/ast/js_wrapped_export.rs` (the `useCallback` entry for the local route only, if E4 = a); `CallSite` and the
    three source constructors in `src/call_graph.rs`; the R4 arm of `resolve_call_site_full`; queries; caches; tests;
    the S1b-2 Tier-A fixture.
  - S1b-3: the R3 block of `resolve_call_site_full` and its helpers; queries (if any); caches; tests; the S1b-3
    Tier-A fixture; the `module_binding_audit_test::esm_namespace_import` update.
- **Forbidden:** `Language::function_name`, `FunctionId`, call-site ownership, `CallKind`, DFG and Step 5/5b code;
  CJS `Local` production; R3 for named, default or `require` qualifiers (E8); `IndirectResolution` sites (E10); any
  runtime-mutation guard; new dependencies; edits to existing Tier-A fixtures; running on the public or private
  corpora (acceptance belongs to the controller); `CLAUDE.md` (no edit is planned for S1b).
- **The prototype is evidence, not authority.** `prototype/s1b-prototype-v7.diff.txt` builds, reproduces every
  control and every expected row-diff, and passes the suite apart from the by-design pins (PLANNING-PROBES Q14–Q15).
  It is feasibility code: it has an uncached and a cached scope walk side by side, three measurement-only environment
  switches (`PRISM_S1B_ONLY_A`, `PRISM_S1B_USECALLBACK`, `PRISM_S1B_BASE_WRITE_SCAN`; **do not port them**), no
  tests and no cache bumps. Where it and the SPEC differ, the SPEC wins. In S1b-1 the module terminal uses the base
  `js_ts_module_value_written` (SPEC §3.2; MEASURED identical to the scoped scan on X, F, R and every control, Q22).
  S1b-2 adds the scoped scan for nested scopes; it may then serve module scope too only if the S1b-1 tests and
  controls stay identical.

## Order of work (TDD)

1. **RED first.** Write the sub-slice's RED rows (SPEC §7: S1b-1 A-1, A-3, A-11; S1b-2 B-1, B-2, B-3; S1b-3 C-1, C-4)
   and its Tier-A fixtures. Add the new types with no behavior so they compile. Capture a behavioral failure with
   its concrete value; compile, setup and zero-test failures are inadmissible.
2. **Implement** the sub-slice's SPEC sections. Turn RED green.
3. **The rest of the sub-slice's §7 table**, both grammars per row, exact targets with decoys. Update the existing
   tests SPEC §7 lists for this sub-slice, and no others; if any other existing test fails, stop and report it.
4. **Mutants**, each applied alone, recording the test that kills it. A surviving bounded mutant is a coverage gap:
   add the killing row.
5. **Synthetic smoke.** `python3 probes/controls_gen.py` in an empty directory (153 scenarios), then
   `nav --no-cache call-stats --dump-sites` and `nav functions` per scenario (`probes/run_controls.sh <bin>
   <gen_dir> <out_dir>`), `probes/controls_summarize.py`, and `probes/controls_diff.py` against the reference
   summary for your sub-slice (base is `probes/S1b-controls-base.txt`):
   - S1b-1: `probes/S1b-controls-proto-v7-onlyA.txt` **minus** the R3 app rows (C62, C80–C83), which stay base
     until S1b-3;
   - S1b-2: `probes/S1b-controls-proto-v7.txt` (E4 = b) or `probes/S1b-controls-proto-v7-uc.txt` (E4 = a), still
     minus the R3 app rows;
   - S1b-3: the full P7 summary for the owner's E4 answer.
   Explain any difference row by row.

## Budget and checkpoints

Honest lines: after `cargo fmt`, non-blank, non-`//`; `#[cfg(test)]` and `tests/**` are tests
(`probes/honest_lines.py <repo> <commit> --per-file` on a squashed commit).

| Sub-slice | src cap / early stop | tests cap / report point | combined |
|---|---|---|---|
| S1b-1 | 300 / 270 | 600 / 540 | 900 |
| S1b-2 | 340 / 305 | 680 / 610 | 1,020 |
| S1b-3 | 120 / 108 | 300 / 270 | 420 |

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
cd eval && uv run tier-a --matrix-only --allow-stale-sut      # base 162/162; expect +1..3 new, 0 regressions
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
