# Implementer brief: S1 (Branch P, with the round-3 fold), span-verified wrapped React exports

You are implementing **exactly** `SPEC.md` (r3, Branch P, with the at-cap round-3 fold) in this directory. The spec
review is finished; sol reviews your implementation. Read it all, then `PLANNING-PROBES.md`
and `REPLAN-fable.md` §1–§2.1, then the repository `CLAUDE.md`, whose conventions are binding: files under 600 lines,
BTreeMap determinism, and the Tier-A rule. The owner decisions are recorded in SPEC §0 (D1–D9). If the dispatch
states anything different, stop and ask.

## The model, in one line

S1 proves what a name denotes; it does not prove what the heap holds (SPEC §3.2). The span and JSX gates apply to
the **new R4c `import_member` route only**. Do not touch R3 `ImportQualified` or R4 `LocalDef`: their behavior on
wrapped targets is S1b scope (SPEC §12), and controls C62 and C63 must stay byte-identical to base. Do **not** add any check for
runtime mutation or re-acquisition of the React object: no `require`/`eval`/member-write scans. Those inputs are
pinned as MB1–MB3 **asserting Exact**. A partial runtime guard would re-open the class that the owner closed by
stating the model.

## Ground rules

- **Branch.** Use the implementation branch named in the dispatch, cut from the planning commit that contains this
  packet. Record the base SHA.
- **Owned paths:**
  - new `src/ast/js_wrapped_export.rs` (with its `mod` line), under 600 lines;
  - the declarator arm in `src/ast.rs:2861-2893`;
  - `src/js_exports.rs`;
  - `src/resolution.rs`: `js_ts_import_member_candidates`, its single call site in `resolve_call_site_full`, and
    `DropReason::WrappedExportNonJsx`;
  - `src/call_graph.rs`: `CallSite.jsx_element`, `jsx_element_at`, the three source constructors (`:1368`, `:1800`,
    `:5101`) and `indirect_call_site`;
  - `src/navigation/queries.rs` (call-stats keys and the drop match);
  - `src/cpg_cache.rs` and `src/navigation/call_edge_cache.rs` (version, pin, note);
  - struct-literal fix-ups forced by the new fields (measured in the prototype):
    `src/navigation/partition_site_dump.rs:85`, `src/resolution_disproof.rs:88`, the `src/resolution.rs` test
    `site()`, the `js_exports.rs` unit tests, `tests/name_resolution/consumer_test.rs:91` and
    `tests/navigation/scoped_calls_test.rs:50`;
  - new `tests/integration/js_wrapped_export*_test.rs`, and `tests/integration/main.rs`;
  - `tests/cli/call_stats_test.rs`, `tests/navigation/callers_test.rs`;
  - 3 new `eval/fixtures/typescript/*` directories (SPEC §7);
  - **`CLAUDE.md`: insert the SPEC §3.2.1 paragraph verbatim** at the stated location (owner decision D8). No other
    `CLAUDE.md` edit.
- **Forbidden:**
  - `Language::function_name`, `FunctionId`, call-site ownership, `CallKind`;
  - DFG and Step 5/5b code;
  - the existing list and default `Local` routes (that is S1b);
  - any runtime-mutation or acquisition guard (see "The model");
  - any change to R3 `ImportQualified` or R4 `LocalDef` (S1b, SPEC §12);
  - replacing the ESM-only React import table with `extract_import_bindings()` unless you add the origin check in
    SPEC §3.1 (T-N18 and mutant M13 must still hold);
  - new dependencies;
  - any edit to existing Tier-A fixtures;
  - running on the public or private corpora. Acceptance (SPEC §8.2) belongs to the controller.
- **The prototype is evidence, not authority.** `prototype/wrapped-export-prototype-P3.diff.txt` builds, passes the
  full suite (P34), and reproduces the acceptance yields and all 67 controls (PLANNING-PROBES P30–P34). Its
  `module_scope_declares` is the single recursive walk that SPEC §3.1 P4 requires. Do not split it back into a
  top-level scan plus a separate hoisted scan: that version measured 357 src lines (P30). It lacks the tests, the cache
  bumps and the pins. It names the variant `WrappedLocal`; the SPEC name is `SpannedLocal`. Where the prototype and
  the SPEC differ, the SPEC wins.

## Order of work (TDD)

1. **RED first.** Write T-P1, T-P7, T-J1, MB1 and T-O1 (admitted case), plus the `forwardref_named_export` and
   `react_default_member_write_out_of_model` Tier-A fixtures. To let them compile, add the new types with **no
   behavior**:
   - `SpannedLocal` is never produced;
   - `span` is always `None`;
   - `jsx_element` is always `false`;
   - the new `DropReason` is never returned.

   Capture a behavioral failure with its concrete value. Compile, setup and zero-test failures are inadmissible.
2. **Implement** SPEC §3.1 in `src/ast/js_wrapped_export.rs`, returning
   `Result<(String, usize, usize), &'static str>` in the R1–R12 order (with no R7). Then §3.3, §3.4 and §5. Turn the
   RED tests green.
3. **Add the rest of §7:** T-P2–T-P11, T-J2–T-J4, T-N1–T-N18, T-R6-P1…P5, **T-R6-P4b**, MB2, MB3, T-O2, T-O3, T-S1,
   T-C1, T-C2, T-V1, and the negative Tier-A fixture. **T-J4 is a base-green preservation control**; mandatory
   JSX-gate RED evidence is T-J1–T-J3 only. Each MB test carries a comment naming SPEC §3.2. For the span filter, the
   JSX gate, R5 and R6-P2, revert that production line and confirm a test fails.
4. **Mutants** M1–M9 and M11–M14 (SPEC §7; there is no M10). M14 (a root-only competitor walk) must be killed by
   T-R6-P4b on the C57 and C59 inputs. C58 does not kill it by design (P32). Apply each alone and record the test that killed it.
5. **`CLAUDE.md`.** Insert the §3.2.1 paragraph exactly as written, in a separate commit.
6. **Budget.** Run `cargo fmt`, then count honest lines: non-blank, non-`//`, with `#[cfg(test)]` and `tests/**`
   counted as tests.
   - The caps are **src 350 / tests 680 / combined 1,010** (owner decision, 2026-09-25, after the test-cap stop at
     698; originally 350 / 600 / 950, with the tests report point moved to 650).
   - The early stops are 315 / 650 / 855. They are **checkpoints**: the measured prototype is already 325 src.
     The fold version is 328. At an early stop, report the count and the forecast. Tests are the tightest bucket
     (the original forecast was about 585 against 600, which is historical; measured 698, re-capped to 680), so
     report at 650. A forecast above a cap is a stop, with the remaining
     items enumerated.
   - Do not compress logic. Lines should be at most 100 columns; list any unsplittable literal.

## Verification (report the totals, from logs)

```bash
cargo fmt --all -- --check
cargo clippy --offline --all-targets --features mcp -- -W clippy::all
cargo test --offline --no-fail-fast                 # base: 4,559 / 0 / 1 (P9); prototype: 4,559 / 0 / 1 (P25)
cargo test --offline --features mcp --no-fail-fast
cargo build --release
cd eval && uv run tier-a --matrix-only --allow-stale-sut      # base and prototype 159/159 ok; expect 159 + 3 new, 0 regressions
cd eval && uv run tier-a --quick --allow-stale-sut            # needs rust-analyzer; if unavailable, report "not run" and why
node scripts/gate-inputs/acquire.mjs && node scripts/gate-inputs/gate.mjs --out target/gate-runs/<name>
```

- Use `--allow-stale-sut` only straight after the `cargo build --release` in the same worktree.
- If the suite surfaces a failure outside S1's scope, report it as found; do not fix it or re-baseline it.
- Paste any Tier-A regression or flip-candidate verbatim.

**Synthetic smoke (allowed).** Generate `probes/controls_gen.py` output into a temp directory (67 scenarios). Run
`nav --no-cache call-stats --dump-sites` and `nav functions` on each, and summarize with
`probes/controls_summarize.py`. The result must equal `probes/PP3-controls-proto.txt` line for line. That includes
C62 and C63, which are unchanged from base. Explain any difference.

## Handback

- Base and head SHAs; the file manifest; `git diff --stat <base> -- src tests eval Cargo.toml Cargo.lock CLAUDE.md`.
- Honest-line counts per bucket, the method, the early-stop checkpoint reports, and lines over 100 columns.
- The RED command and its excerpt; the revert-dependency notes; the mutant kill table.
- Suite totals (default, mcp, Tier-A matrix and quick, Node gate), with log paths. The synthetic smoke diff.
- Deviations from the SPEC, each with its reason. Each deviation is a question for the owner.
- Commit on the branch with the session's attribution trailers. Do not push or merge.
