# Implementer brief: S1, span-verified wrapped React exports

You are implementing **exactly** `SPEC.md` in this directory. Read all of it, then `PLANNING-PROBES.md`, then the
repository `CLAUDE.md`, whose conventions are binding: files stay under 600 lines, BTreeMap determinism, and the
Tier-A rule. Proceed only if the dispatch states the owner's §0 answers. If they differ from the recommendations
(D1 = A, D2 = Exact, D3 = S1, D4 = a, D5 = S1 now), stop and ask.

## Ground rules

- **Branch.** Use the implementation branch named in the dispatch, cut from the planning commit that contains this
  packet. Record the base SHA.
- **Owned paths:**
  - new `src/ast/js_wrapped_export.rs`, plus its `mod` line
  - the declarator arm in `src/ast.rs:2861-2893`
  - `src/js_exports.rs`
  - `src/resolution.rs::js_ts_import_member_candidates` only
  - `src/navigation/queries.rs` (call-stats keys only)
  - `src/cpg_cache.rs` and `src/navigation/call_edge_cache.rs` (version, pin, note)
  - new `tests/integration/js_wrapped_export*_test.rs`, and `tests/integration/main.rs`
  - `tests/cli/call_stats_test.rs`, `tests/navigation/callers_test.rs`
  - up to 3 new `eval/fixtures/typescript/*` directories
  - existing tests that construct `ResolvedJsExport` (add `span: None`)
- **Forbidden:**
  - `Language::function_name`, `FunctionId`, or call-site ownership
  - DFG and Step 5/5b code
  - the existing list and default `Local` routes (that is S1b)
  - new dependencies
  - any edit to existing Tier-A fixtures
  - running on the public or private corpora. Acceptance (SPEC §8.2) belongs to the controller.
- **The prototype is evidence, not authority.** `prototype/wrapped-export-prototype.diff.txt` built and ran during
  planning. It lacks the SPEC §3.1 clauses 1 (it had `const` only), 2 (parse recovery), 3 (uniqueness and
  module-scope competitor) and 4 (comment skipping), the line-identity collision refusal (§3.3), the counters, the
  cache bumps, and the tests. It also used `extract_import_bindings()` per declarator; compute it once per file.
  Where the prototype and the SPEC differ, the SPEC wins.

## Order of work (TDD)

1. **RED first.** Write T-P1, T-P7 and T-O1 (admitted case), and the `forwardref_named_export` Tier-A fixture. To
   let them compile, add the new types with **no behavior**: `SpannedLocal` is never produced, and `span` is always
   `None`. Run the tests and capture a behavioral failure with its concrete value, for example "resolved kinds `[]`
   ≠ `[(Exact, ImportMember)]`". Compile, setup and zero-test failures are inadmissible. Save the excerpt.
2. **Implement** §3.1 in `src/ast/js_wrapped_export.rs`. It returns `Result<(String, usize, usize), &'static str>`,
   where the error is the reason key. Then implement §3.2, §3.3 and §5. Turn the RED tests green.
3. **Add the rest of §7:** T-P2 through T-P10, T-N1 through T-N14, T-O2, T-S1, T-C1, T-C2 and T-V1, and the
   negative Tier-A fixture. For each positive test, record which production line it depends on, by reverting that
   line and confirming the test fails. Do this at least for the span filter (§3.3), the provenance clause (§3.1.3)
   and the first-argument rule (§3.1.4).
4. **Mutation spot-checks.** Apply each mutant alone and confirm a test fails. Report which test killed it.
   - (M1) Drop the span filter. T-P7 and T-N11 must fail.
   - (M2) Accept any callee. T-N6 and T-N3 must fail.
   - (M3) Accept `let`. T-N5 must fail.
   - (M4) Pick the last argument. T-P4 must fail.
   - (M5) Demote a collision to NameOnly instead of refusing. T-N11 must fail.
   - (M6) Stop counting a reason. T-O1 must fail.
   - (M7) Skip the cache bump. T-C2 must fail.
5. **Budget.** Run `cargo fmt`, then count honest lines: non-blank, non-`//`, with `#[cfg(test)]` and `tests/**`
   counted as tests. The caps are **src 200 / tests 450 / combined 650**, with early stops at 180, 405 and 585. If a
   cap would be exceeded, stop and return the count and the remaining items. Do not compress logic. Lines should be at
   most 100 columns; list any unsplittable literal.

## Verification (report the totals, from logs)

```bash
cargo fmt --all -- --check
cargo clippy --offline --all-targets --features mcp -- -W clippy::all
cargo test --offline --no-fail-fast                 # base: 4,559 passed / 0 failed / 1 ignored (PLANNING P9)
cargo test --offline --features mcp --no-fail-fast
cargo build --release
cd eval && uv run tier-a --matrix-only --allow-stale-sut      # base 159/159 ok (P10); expect 159 + new fixtures, 0 regressions
cd eval && uv run tier-a --quick --allow-stale-sut            # needs rust-analyzer; if unavailable, report "not run" and why
node scripts/gate-inputs/acquire.mjs && node scripts/gate-inputs/gate.mjs --out target/gate-runs/<name>
```

Use `--allow-stale-sut` only straight after the `cargo build --release` in the same worktree. If the suite surfaces a
failure outside S1's scope, report it as found. Do not fix it or re-baseline it. Paste any Tier-A regression or
flip-candidate verbatim.

A synthetic yield smoke **is** allowed. Run the release binary with `nav --no-cache call-stats --dump-sites` on
`probes/controls_gen.py` output (generate it into a temp directory, not the repo). The diff against base must match
`probes/P6a-controls-proto.txt` on C02, C11, C14, C15, C17, C26 and C27 (Exact), and every other control must be
unchanged.

## Handback

- Base and head SHAs; the file manifest; `git diff --stat <base> -- src tests eval Cargo.toml Cargo.lock`.
- Honest-line counts per bucket, with the method and any lines over 100 columns.
- The RED command and its failing excerpt; the revert-dependency notes; the mutant kill table (M1–M7).
- Suite totals (default, mcp, Tier-A matrix and quick, Node gate), with log paths.
- The synthetic control diff.
- Deviations from the SPEC, each with its reason. Each deviation is a question for the owner, not a decision you
  make.
- Commit on the branch with the session's attribution trailers. Do not push or merge.
