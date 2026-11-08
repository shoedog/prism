# Review brief: PR #769 rebase delta (issue #749; the nav-gaps lane's PR C)

Goal the task serves: `nav_callers` and the Rust resolution ladder resolve qualified cross-crate calls made from a workspace member's binaries (`src/bin/*.rs`, `src/main.rs`) to the member's dependencies as rustc does, binding rustc's file and never a decoy. The PR's pre-rebase content (head a02641a3, base 22926d70) was reviewed over two rounds and approved; it is NOT under review again. #770 (#765) has since made `#[path]` placement rustc's, so PR C was rebased onto main ace5a9c3 and its interim placement guard retired. Review ONLY what the rebase changed.

Worktree: /Users/wesleyjinks/code/prism-nav-c at head 110caf7f (do not edit, do not run cargo; memguard refuses in your sandbox; never open or list eval/secbench). Files for you in `.review-769/`:
- `769-prev-delta.diff`: the approved PR against its old base (22926d70..a02641a3).
- `769-new-delta.diff`: the PR against its new base (ace5a9c3..110caf7f).
- `769-new-commit.diff`: the one commit added by the rebase (110caf7f), which is the substantive change.
Compare prev-delta with new-delta for the conflict resolutions in the seven carried commits; read the new commit in full.

What the implementer reports (verify, do not trust):
1. The guard `src/resolution/rust/bin_dep_guard.rs` lost its PLACEMENT half (it read `ScopeGraph::misplaced_mount_files`, which #770 removed) and KEPT its block-shadow half: a bin-root qualified call whose first segment is shadowed by a block-scope `use .. as alias` / `mod alias` between the site and the module refuses (rows C18/C18b; without it they bind the dependency, a wrong Exact; this is #768's mechanism, which #765 did not fix). `ScopeGraph::bin_dep_entries` and one call in `self_calls.rs` remain. About 100 lines.
2. Rows C17, C17b, C17d changed from asserting refusal/decoy to asserting rustc's file `a/src/q.rs::f@1`; W2a, W2b, R3, R1, R2 un-ignored, all bind rustc's file; `rust_cross_crate_bin_test` 31 passed, 0 ignored.
3. Lane `mutants/rust-bin-root-deps.json`: BRD-10 and BRD-11 (the placement guard's mutants) removed; BRD-12/13/14 re-anchored on the shadow-only guard; 12 mutants, plan-only 12/12, all hand-killed.
4. Conflict resolutions: `tests/integration/main.rs` keeps both new module lines; `rust_populator/builder.rs` keeps the bin-dep keying loop and drops `misplaced_mounts`; `graph.rs`, `binding_index_tests.rs`, `resolution/rust.rs`, `self_calls.rs`, `path_anchor.rs` took main's; caches CPG 164 -> 165, nav 82 -> 83 (pin tests, `owned.version`, CLAUDE.md "currently v165").
5. §4 re-measurement on the frozen subject ace5a9c3 (minus eval/secbench): Rust gained 51 (new_exact 51), relabel 1 (`main.rs:808` `qualified_owner` -> `constructor_local`, same target), lost = demoted = ambiguous = unmatched = retarget = promoted = 0; other unchanged; identical sites and targets to the pre-rebase measurement. Evidence `~/prism-evidence/nav-gaps/raw/pr-c-rebase/`.

Questions, in order:
- WRONG? Does any resolved hunk re-add machinery main removed in #770 (`misplaced_mount_files`, `Builder::misplaced_mounts`, the mount refusal of `path_anchor::guard`) or drop something #770 added (per-occurrence population, `multi_occurrence_files`, `is_convention_root` rule, E1/E2 code)? Name the hunk.
- WRONG? Is the kept shadow half of the guard exactly the shadow rule and nothing else, and is it still reached for every bin-root dependency path (`self_calls.rs` call site), with no second code path that admits a shadowed alias? Read `bin_dep_guard.rs` in full against C18/C18b. Is its refusal still fail-closed when `bin_dep_entries` is empty or the file is attached to no crate?
- WRONG? The C17/C17b/C17d rewrites: with the decoy file still present in the fixture at the old `<stem>/` location, do the rows assert rustc's target by function identity, and is `a/src/q.rs::f@1` indeed where rustc places `#[path]` for that fixture (read the fixture and apply the Reference rule)? A loosened or decoy-free assertion is WRONG.
- WRONG? Lane: removed IDs are exactly the placement guard's; each remaining anchor's snippet exists verbatim in the tree (spot-check BRD-12/13/14 and two others); no mutant edits a version constant.
- MATERIAL/SMELL: the measurement gate for THIS PR is lost = demoted = ambiguous = unmatched = 0 (it adds edges only); any nonzero is WRONG with the row. Does the relabel row matter?
- Docs: CLAUDE.md no longer says the guard covers placement or that #765 is pending; the kept shadow rule is described where #768 is.

Tag each finding WRONG or SMELL and MATERIAL or IMMATERIAL to the goal above; a finding without a concrete failing input (file, fixture, call, wrong result) is a SMELL; give the fix and when it would not apply. Verdict: APPROVE / FIX (closed findings) / REJECT (open class). Use prism on the worktree (nav_callers / nav_callees / nav_nodes_at) where it helps and end with the line "prism: n queries". Static review only. One review round (declared cap).
