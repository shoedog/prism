# Implementer brief: S1, span-verified wrapped React exports

You are implementing **exactly** `SPEC.md` in this directory. Read all of it, then `PLANNING-PROBES.md`, then the
repository `CLAUDE.md`, whose conventions are binding: files stay under 600 lines, BTreeMap determinism, and the
Tier-A rule. Proceed only if the dispatch states the owner's §0 answers. If they differ from the recommendations
(D1 = A, D2 = Exact, D3 = S1, D4 = a, D5 = S1 now), stop and ask.

## Ground rules

- **Branch.** Use the implementation branch named in the dispatch, cut from the planning commit that contains this
  packet (SPEC r2). Record the base SHA.
- **Owned paths:**
  - new `src/ast/js_wrapped_export.rs` (with its `mod` line), under 600 lines;
  - the declarator arm in `src/ast.rs:2861-2893`;
  - `src/js_exports.rs`;
  - `src/resolution.rs`: `js_ts_import_member_candidates`, its single call site in `resolve_call_site_full`, and
    the new `DropReason::WrappedExportNonJsx`;
  - `src/call_graph.rs`: `CallSite.jsx_element`, `jsx_element_at`, the three source constructors (`:1368`, `:1800`,
    `:5101`) and `indirect_call_site`;
  - `src/navigation/queries.rs` (call-stats keys and the drop match);
  - `src/cpg_cache.rs` and `src/navigation/call_edge_cache.rs` (version, pin, note);
  - struct-literal fix-ups that the new fields force. Measured in the prototype:
    `src/navigation/partition_site_dump.rs:85` and `src/resolution_disproof.rs:88` (both `#[cfg(test)]`),
    `src/resolution.rs` test `site()`, the `js_exports.rs` unit tests, `tests/name_resolution/consumer_test.rs:91`
    and `tests/navigation/scoped_calls_test.rs:50`;
  - new `tests/integration/js_wrapped_export*_test.rs`, and `tests/integration/main.rs`;
  - `tests/cli/call_stats_test.rs`, `tests/navigation/callers_test.rs`;
  - up to 3 new `eval/fixtures/typescript/*` directories (SPEC §7).
- **Forbidden:**
  - `Language::function_name`, `FunctionId`, call-site ownership, `CallKind`;
  - DFG and Step 5/5b code;
  - the existing list and default `Local` routes (that is S1b);
  - new dependencies;
  - any edit to existing Tier-A fixtures;
  - running on the public or private corpora. Acceptance (SPEC §8.2) belongs to the controller.
- **The prototype is evidence, not authority.** `prototype/wrapped-export-prototype-r2.diff.txt` builds, passes the
  full suite (P16), and reproduces the acceptance yields (P18). It lacks:
  - the tests, cache bumps and pins;
  - the reuse of `extract_import_bindings()` for React imports (SPEC §9: it hand-parses them).

  It also carries a mode switch (`PRISM_PROTO_WRAP_MODE`) that must **not** ship. It names the variant
  `WrappedLocal`; the SPEC name is `SpannedLocal`. Where the prototype and the SPEC differ, the SPEC wins.

## Order of work (TDD)

1. **RED first.** Write T-P1, T-P7, T-J1 and T-O1 (admitted case), and the `forwardref_named_export` Tier-A fixture.
   To let them compile, add the new types with **no behavior**:
   - `SpannedLocal` is never produced;
   - `span` is always `None`;
   - `jsx_element` is always `false`;
   - the new `DropReason` is never returned.

   Capture a behavioral failure with its concrete value, for example "resolved kinds `[]` ≠
   `[(Exact, ImportMember)]`". Compile, setup and zero-test failures are inadmissible. Save the excerpt.
2. **Implement** SPEC §3.1–§3.2 in `src/ast/js_wrapped_export.rs`, returning
   `Result<(String, usize, usize), &'static str>` in the R1–R12 order. Then §3.3, §3.4 and §5. Turn the RED tests
   green.
3. **Add the rest of §7:** T-P2–T-P11, T-J2–T-J4, T-N*, T-R6-*, T-R7-*, T-O2, T-O3, T-S1, T-C1, T-C2 and T-V1, plus
   the two negative Tier-A fixtures. For the span filter, the JSX gate, R5 and R7, revert that production line and
   confirm a positive test fails.
4. **Mutants M1–M11** (SPEC §7). Apply each alone and record which test killed it.
5. **Budget.** Run `cargo fmt`, then count honest lines: non-blank, non-`//`, with `#[cfg(test)]` and `tests/**`
   counted as tests. The caps are **src 420 / tests 650 / combined 1,070**, with early stops at 380, 585 and 960. If
   a cap would be exceeded, stop and return the count plus the remaining items. Do not compress logic. Lines should be
   at most 100 columns; list any unsplittable literal.

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

A synthetic yield smoke **is** allowed. Generate `probes/controls_gen.py` output into a temp directory (not the
repo); it produces 52 scenarios. Run the release binary with `nav --no-cache call-stats --dump-sites` and
`nav functions` on each, and summarize with `probes/controls_summarize.py`. The result must equal
`probes/P18-controls-proto-a.txt` line for line. The only allowed differences are `FunctionId` spans, if your inner-span
computation legitimately differs, and each such difference must be explained.

## Handback

- Base and head SHAs; the file manifest; `git diff --stat <base> -- src tests eval Cargo.toml Cargo.lock`.
- Honest-line counts per bucket, with the method and any lines over 100 columns.
- The RED command and its failing excerpt; the revert-dependency notes; the mutant kill table (M1–M11).
- Suite totals (default, mcp, Tier-A matrix and quick, Node gate), with log paths.
- The synthetic control diff.
- Deviations from the SPEC, each with its reason. Each deviation is a question for the owner, not a decision you
  make.
- Commit on the branch with the session's attribution trailers. Do not push or merge.
