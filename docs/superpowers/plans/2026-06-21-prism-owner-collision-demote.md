# Owner-Key Collision Demote-Not-Drop Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Stop `owner_lookup_in_modules` from emitting a multi-candidate same-primary-owner pool (same-name-type collisions, overloads, inherent+trait same-name dups) at Exact (1.0); demote it to NameOnly instead — removing a full-confidence false-positive class with zero recall loss.

**Architecture:** One added branch in the shared resolution chokepoint `owner_lookup_in_modules` (`src/resolution.rs`). A `pool.len() > 1` pool that is *not* multi-distinct-owner (trait-CHA) is returned via `demoted(pool, QualifiedOwner)` (NameOnly) instead of `exact(pool, QualifiedOwner)`. Confidence-only: the `QualifiedOwner` kind is preserved so every caller relabel (R3b/`Self::`/R6/implicit-this) — all of which mutate `kind` only — carries the NameOnly through unchanged. No new `ResolutionKind`, no `CACHE_VERSION` bump.

**Tech Stack:** Rust, tree-sitter; tests via `cargo test` (`tests/integration/resolution_test.rs`, `tests/cli/call_stats_test.rs`); acceptance via `prism nav --no-cache call-stats` and the `eval/` Tier-A harness (`uv run tier-a`).

**Design-of-record:** `docs/superpowers/specs/2026-06-20-prism-owner-collision-demote-design.md` (read §3, §4, §5, §10, §11, §14 before starting).

**Branch:** Execute on `precision-multitarget-counter` (carries the `multi_target_exact_*` diagnostic counters that are this fix's acceptance instrument, plus the spec). The demote and its counter ship together.

**macOS test note:** bare `cargo test --test cli` may stall at `_dyld_start`. Compile the cli test binary with `--no-run`, then run the produced binary with a module-qualified filter (shown per task). `--lib` and `--test integration` run normally.

---

## File Structure

- **Modify:** `src/resolution.rs` — `owner_lookup_in_modules` final return: add one `else if pool.len() > 1` arm. Sole production change.
- **Modify (test):** `tests/integration/resolution_test.rs` — add 4 resolution-level tests (collision demotes, single stays Exact, inherent+trait demotes, recovered-receiver relabel rides NameOnly through).
- **Modify (test):** `tests/cli/call_stats_test.rs` — invert `call_stats_reports_multi_target_exact_same_name_owner_collision` to assert the demoted (NameOnly) outcome.
- **Verify only:** `tests/integration/resolution_test.rs::r1_type_qualified_call_resolves_to_owner_method_exact` (single owner → still Exact, must NOT flip) and `tests/navigation/callees_test.rs` (adjust only if a `qualified_owner` assertion used a multi-same-owner pool).

---

## Task 1: Core demote — collision pool → NameOnly, single owner stays Exact

**Files:**
- Modify: `src/resolution.rs` (`owner_lookup_in_modules`, the final `Some(if … )` return)
- Test: `tests/integration/resolution_test.rs`

- [ ] **Step 1: Write the failing collision test**

Add to `tests/integration/resolution_test.rs`:

```rust
#[test]
fn owner_collision_pool_demotes_to_name_only() {
    use prism::languages::Language::Rust;
    // Two distinct `Foo` types, each with an associated `make`. A qualified
    // `Foo::make()` keys the bare index ("Foo","make") to BOTH defs; both share
    // primary owner "Foo", so this is NOT trait-CHA. Build WITHOUT a scope graph
    // so resolution reaches owner_lookup_in_modules directly (a complete scope
    // graph would narrow/drop the call upstream before this rung).
    let (cg, _) = build_without_scope_graph(&[
        (
            "a.rs",
            "pub struct Foo;\nimpl Foo {\n    pub fn make() -> Foo { Foo }\n}\n",
            Rust,
        ),
        (
            "b.rs",
            "pub struct Foo;\nimpl Foo {\n    pub fn make() -> Foo { Foo }\n}\n",
            Rust,
        ),
        ("c.rs", "fn run() {\n    Foo::make();\n}\n", Rust),
    ]);
    let site = site_in(&cg, "run", "Foo::make");
    let r = cg.resolve_call_site(&site);
    assert_eq!(r.len(), 2, "both Foo::make defs retained (recall)");
    assert!(
        r.iter().all(|c| c.confidence == ResolutionConfidence::NameOnly),
        "collision pool demoted, not Exact"
    );
    assert!(r.iter().all(|c| c.kind == ResolutionKind::QualifiedOwner));
}
```

- [ ] **Step 2: Run it; verify it fails**

Run: `cargo test --test integration resolution_test::owner_collision_pool_demotes_to_name_only -- --exact`
Expected: FAIL — candidates are currently `Exact` (assertion `collision pool demoted, not Exact` fails).

- [ ] **Step 3: Write the single-owner regression test (guards against over-demote)**

Add to `tests/integration/resolution_test.rs`:

```rust
#[test]
fn owner_single_candidate_stays_exact() {
    use prism::languages::Language::Rust;
    let (cg, _) = build_without_scope_graph(&[
        (
            "a.rs",
            "pub struct Foo;\nimpl Foo {\n    pub fn make() -> Foo { Foo }\n}\n",
            Rust,
        ),
        ("c.rs", "fn run() {\n    Foo::make();\n}\n", Rust),
    ]);
    let site = site_in(&cg, "run", "Foo::make");
    let r = cg.resolve_call_site(&site);
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].confidence, ResolutionConfidence::Exact);
    assert_eq!(r[0].kind, ResolutionKind::QualifiedOwner);
}
```

- [ ] **Step 4: Run it; verify it PASSES (single owner is already Exact today)**

Run: `cargo test --test integration resolution_test::owner_single_candidate_stays_exact -- --exact`
Expected: PASS (this is current behavior; it must stay green after the change).

- [ ] **Step 5: Implement the demote branch**

In `src/resolution.rs`, find the final return of `owner_lookup_in_modules`:

```rust
        Some(if pool.len() > 1 && primary_owners.len() > 1 {
            demoted(pool, ResolutionKind::TraitCha)
        } else {
            exact(pool, ResolutionKind::QualifiedOwner)
        })
```

Replace it with:

```rust
        Some(if pool.len() > 1 && primary_owners.len() > 1 {
            // Multiple DISTINCT primary owners — trait-CHA (dyn Trait). Unchanged.
            demoted(pool, ResolutionKind::TraitCha)
        } else if pool.len() > 1 {
            // Non-trait multi-candidate owner-key ambiguity: >1 candidate under one
            // primary owner name with no scope proof reached here — same-name-type
            // collisions, overloads, or inherent+trait same-name dups. Demote: keep
            // every edge (recall) but not at full confidence. Kind stays
            // QualifiedOwner so caller relabels (R3b/Self::/R6/implicit-this) fire
            // unchanged; only the confidence rides through as NameOnly. Recoverable
            // to Exact once an upstream capability supplies the discrimination.
            demoted(pool, ResolutionKind::QualifiedOwner)
        } else {
            // Single candidate — Exact, unchanged.
            exact(pool, ResolutionKind::QualifiedOwner)
        })
```

- [ ] **Step 6: Run both Task 1 tests; verify PASS**

Run (one substring filter matches both `owner_collision_*` and `owner_single_*`): `cargo test --test integration resolution_test::owner_`
Expected: both PASS (and no other `resolution_test::owner_*` test exists yet at this point).

- [ ] **Step 7: Verify the trait-CHA arm is untouched**

Run: `cargo test --test integration resolution_test::r1_trait_qualified_multi_impl_demotes_to_name_only -- --exact`
Expected: PASS (multi-distinct-owner still NameOnly/`TraitCha`).

- [ ] **Step 8: Commit**

```bash
git add src/resolution.rs tests/integration/resolution_test.rs
git commit -m "fix(resolution): demote same-primary-owner multi-candidate pool to NameOnly

owner_lookup_in_modules emitted a >1-candidate pool with a single primary owner
(same-name-type collisions, overloads, inherent+trait dups) at Exact (1.0) — a
full-confidence false positive. Demote to NameOnly: keep every edge (recall),
keep the QualifiedOwner kind so caller relabels ride the NameOnly through.
Single-candidate and trait-CHA arms unchanged.

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

## Task 2: Discriminating + relabel coverage

**Files:**
- Test: `tests/integration/resolution_test.rs`

- [ ] **Step 1: Write the same-owner inherent+trait test (spec §3 (d), the discriminating case)**

Add to `tests/integration/resolution_test.rs`:

```rust
#[test]
fn owner_inherent_plus_trait_same_name_demotes() {
    use prism::languages::Language::Rust;
    // ONE type `Foo` with an inherent `m` AND a same-named trait-impl `m`. Both
    // register under ("Foo","m") with primary owner "Foo" -> a non-trait-CHA
    // multi-candidate pool (pool=2, primary_owners=1). Confirms the demote covers
    // the accepted same-owner ambiguity set, not just distinct same-named types.
    let (cg, _) = build_without_scope_graph(&[(
        "a.rs",
        "pub struct Foo;\n\
         impl Foo {\n    pub fn m(&self) {}\n}\n\
         pub trait T {\n    fn m(&self);\n}\n\
         impl T for Foo {\n    fn m(&self) {}\n}\n\
         fn run() {\n    Foo::m();\n}\n",
        Rust,
    )]);
    let site = site_in(&cg, "run", "Foo::m");
    let r = cg.resolve_call_site(&site);
    assert_eq!(r.len(), 2, "inherent + trait-impl m both retained");
    assert!(r.iter().all(|c| c.confidence == ResolutionConfidence::NameOnly));
    assert!(r.iter().all(|c| c.kind == ResolutionKind::QualifiedOwner));
}
```

- [ ] **Step 2: Run it; verify PASS (the Task 1 branch already covers it)**

Run: `cargo test --test integration resolution_test::owner_inherent_plus_trait_same_name_demotes -- --exact`
Expected: PASS. If it shows `r.len() == 1` (only one `m` indexed under `Foo`), the fixture didn't produce the dual entry — adjust by confirming both `impl` blocks parse, then re-run; the assertion target is 2 NameOnly candidates.

- [ ] **Step 3: Write the recovered-receiver relabel-rides-through test**

Add to `tests/integration/resolution_test.rs` (uses `build_cfg` + default receiver config — the same builder the existing `slice_a_legacy_parity_p6_typed_param` test uses to recover a `TypedParam` receiver):

```rust
#[test]
fn recovered_receiver_collision_demotes_and_keeps_typed_param_relabel() {
    use prism::languages::Language::Rust;
    use prism::resolution::{ReceiverRecovery, ReceiverRecoveryConfig};
    // Two distinct `Foo` types each with `make`; `run(r: Foo)` calls `r.make()`.
    // P6-lite recovers r:Foo syntactically (no scope graph needed); R6 routes to
    // owner_lookup("Foo","make") -> 2-candidate collision -> demote. R6 relabels
    // kind QualifiedOwner -> TypedParam; the NameOnly confidence must ride through.
    let (cg, _) = build_cfg(
        &[
            (
                "a.rs",
                "pub struct Foo;\nimpl Foo {\n    pub fn make(&self) {}\n}\n",
                Rust,
            ),
            (
                "b.rs",
                "pub struct Foo;\nimpl Foo {\n    pub fn make(&self) {}\n}\n",
                Rust,
            ),
            ("c.rs", "fn run(r: Foo) {\n    r.make();\n}\n", Rust),
        ],
        &ReceiverRecoveryConfig::default(),
    );
    let site = site_in(&cg, "run", "make");
    assert_eq!(
        site.receiver_recovery,
        Some(ReceiverRecovery::TypedParam),
        "fixture must recover the typed-param receiver"
    );
    let r = cg.resolve_call_site(&site);
    assert_eq!(r.len(), 2);
    assert!(
        r.iter().all(|c| c.confidence == ResolutionConfidence::NameOnly),
        "demoted confidence rides through the kind relabel"
    );
    assert!(r.iter().all(|c| c.kind == ResolutionKind::TypedParam));
}
```

- [ ] **Step 4: Run it; verify PASS**

Run: `cargo test --test integration resolution_test::recovered_receiver_collision_demotes_and_keeps_typed_param_relabel -- --exact`
Expected: PASS. If the `receiver_recovery` assertion fails (receiver not recovered), the `r: Foo` param form isn't recovered under the default config — switch the first assertion's expectation by inspecting `site.receiver_type`/`site.receiver_recovery` for the actual recovered value and align the `kind` assertion to the matching `ResolutionKind` (e.g. `ConstructorLocal`); the load-bearing assertion is the two `NameOnly` candidates.

- [ ] **Step 5: Commit**

```bash
git add tests/integration/resolution_test.rs
git commit -m "test(resolution): cover same-owner inherent+trait demote + receiver relabel rides NameOnly

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

## Task 3: Invert the call-stats counter test + sweep for flipped assertions

**Files:**
- Modify (test): `tests/cli/call_stats_test.rs`
- Verify: `tests/integration/resolution_test.rs`, `tests/navigation/callees_test.rs`

- [ ] **Step 1: Invert the existing collision counter test**

In `tests/cli/call_stats_test.rs`, the test `call_stats_reports_multi_target_exact_same_name_owner_collision` currently asserts the collision is counted as multi-target-Exact. After the fix the collision is NameOnly, so it is no longer multi-target-Exact. Replace its assertion block:

```rust
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["multi_target_exact_sites"], 1);
    // Fanout-2 bucket holds exactly this site; keyed by stringified fanout.
    assert_eq!(v["multi_target_exact_fanout"]["2"], 1);
    // Attributed to the qualified-owner kind that minted the colliding pool.
    assert_eq!(v["multi_target_exact_by_kind"]["qualified_owner"], 1);
```

with:

```rust
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    // After demote-not-drop: the same-name `Foo::make` collision now resolves at
    // NameOnly, so it is no longer a multi-target-Exact site, and both edges land
    // in kind_nameonly[qualified_owner] (the unrelabeled `::`-split path).
    assert_eq!(v["multi_target_exact_sites"], 0);
    assert_eq!(v["kind_nameonly"]["qualified_owner"], 2);
    assert!(v["kind_exact"].get("qualified_owner").is_none());
```

Also rename the test to reflect its new role:

```rust
fn call_stats_same_name_owner_collision_demotes_out_of_multi_target_exact() {
```

(update the function name on its `#[test]` line; leave the fixture body unchanged.)

**Also update the SECOND collision test (plan-review BLOCKER).**
`call_stats_shadow_stratifies_type_path_collision_and_runs_narrowing` (same file,
`:77`) uses the *same* `Foo::make` collision and asserts `multi_target_exact_sites
== 1`, `multi_target_exact_shape["type_path"] == 1`, and
`shadow_typepath_narrow["failopen_type_unresolved"] == 1`. After the demote the site
is NameOnly, so it is no longer multi-target-Exact and the shape/shadow block (gated
on ≥2 Exact edges in `src/navigation/queries.rs`) never runs — all three assertions
break. Replace its assertion block:

```rust
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["multi_target_exact_sites"], 1);
    // The colliding site is recognized as a type-path (`T::m`) shape...
    assert_eq!(v["multi_target_exact_shape"]["type_path"], 1);
    // ...and the narrowing shadow runs over it and classifies it (here the owner
    // type cannot be resolved to a single in-repo scope -> fail-open).
    assert_eq!(v["shadow_typepath_narrow"]["failopen_type_unresolved"], 1);
```

with:

```rust
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    // After demote-not-drop the `Foo::make` collision resolves NameOnly, so it is no
    // longer multi-target-Exact: the shape/shadow stratification (gated on >=2 Exact
    // edges) does not run, and both edges land in kind_nameonly[qualified_owner].
    assert_eq!(v["multi_target_exact_sites"], 0);
    assert_eq!(v["multi_target_exact_shape"].as_object().unwrap().len(), 0);
    assert_eq!(v["shadow_typepath_narrow"].as_object().unwrap().len(), 0);
    assert_eq!(v["kind_nameonly"]["qualified_owner"], 2);
```

and rename it:

```rust
fn call_stats_demoted_collision_absent_from_shape_and_shadow() {
```

(The shadow/shape counters stay in the code — they remain the forward instrument for
the completeness-gate follow-on, where ruff gains a scope graph and the shadow again
measures live narrowability; post-demote they are simply empty for collisions.)

- [ ] **Step 2: Build the cli test binary and run the inverted test**

```bash
cargo build --bin prism
cargo test --test cli --no-run
# -type f + executable bit avoids matching the macOS `cli-*.dSYM` directory.
CLI_BIN=$(find target/debug/deps -maxdepth 1 -type f -name 'cli-*' ! -name '*.d' -perm -111 | head -1)
"$CLI_BIN" call_stats_test::call_stats_same_name_owner_collision_demotes_out_of_multi_target_exact --exact --nocapture
"$CLI_BIN" call_stats_test::call_stats_demoted_collision_absent_from_shape_and_shadow --exact --nocapture
```
Expected: PASS (`multi_target_exact_sites == 0`, `kind_nameonly[qualified_owner] == 2`).

- [ ] **Step 3: Sweep the full resolution + navigation suites for assertions that flipped**

```bash
cargo test --test integration resolution_test::
cargo test --test navigation
```
Expected: GREEN. If any test fails because it asserted **Exact on a multi-same-owner pool**, that flip is the intended behavior — update its expectation to `NameOnly` (do NOT weaken the production branch to preserve a stale expectation). The single-owner test `r1_type_qualified_call_resolves_to_owner_method_exact` and any single/distinct-owner cases must stay green untouched. Record each updated assertion in the commit message.

- [ ] **Step 4: Run the remaining cli call-stats tests (no other regressions)**

```bash
"$CLI_BIN" call_stats_test:: --nocapture
```
(`$CLI_BIN` from Step 2.)
Expected: all four call-stats tests PASS — the two updated collision tests (Step 1) plus the two genuinely unaffected ones (`call_stats_reports_kind_counts_and_drops`, `call_stats_reports_embedded_promotion_and_ambiguity`, which use drop / embedded-promotion fixtures, not a same-owner collision).

- [ ] **Step 5: Commit**

```bash
git add tests/cli/call_stats_test.rs tests/integration/resolution_test.rs tests/navigation/
git commit -m "test: invert call-stats collision counter to NameOnly; update any flipped Exact assertions

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

## Task 4: Acceptance — fmt, full suite, Tier-A gate, call-stats before/after

**Files:** none modified (verification + evidence capture).

- [ ] **Step 1: Format + full library/integration suite**

```bash
cargo fmt
cargo fmt --check
cargo test --lib
cargo test --test integration
```
Expected: fmt clean (no diff); all green. Fix any failure before proceeding.

- [ ] **Step 2: Capture the call-stats before/after on the Rust anchors (`--no-cache` is required)**

Build release, then record `multi_target_exact_sites` and the `qualified_owner` exact/nameonly split for each anchor (the `--no-cache` flag is mandatory — `ResolutionConfidence` is serialized on CPG edges, so a cache-on run would serve stale Exact and mask the drop):

```bash
cargo build --release
for r in /Users/wesleyjinks/code/slicing /Users/wesleyjinks/code/bench-repos/ruff /Users/wesleyjinks/code/bench-repos/ripgrep; do
  echo "== $r =="
  ./target/release/prism nav --no-cache call-stats --repo "$r" 2>/dev/null \
    | grep -E '"multi_target_exact_sites":'
done
```
Expected: `multi_target_exact_sites` drops substantially versus the spec §1 baseline. Note **only the `owner_lookup`-routed shapes demote** — the resolution *kinds* `qualified_owner`, `typed_param`, `self_receiver`, `qualifier_owner` fall out of `kind_exact` and into `kind_nameonly` (note `qualifier_owner` is the kind; `qualifier_field` is a `multi_target_exact_shape` key, not a kind); a residual from non-`owner_lookup` Exact-multi paths (`import_qualified`, `local_def`) may remain, so do not expect exactly 0. The load-bearing signal is `kind_exact[qualified_owner]`/`[typed_param]`/`[self_receiver]` dropping toward 0 with the matching `kind_nameonly` rising. For richer detail also dump `multi_target_exact_by_kind` and `kind_exact`/`kind_nameonly`. Paste before/after into the PR description.

- [ ] **Step 3: Tier-A matrix gate (no-LSP, fast)**

```bash
cd eval && uv run tier-a --matrix-only --allow-stale-sut
```
Expected: no `ok → regression` flips; the expected `ok` count is preserved. Paste the matrix summary into the PR.

- [ ] **Step 4: Tier-A quick gate (zero recall regression)**

Clear the prism nav cache first so the SUT re-resolves against the patched binary (the `--quick` SUT path defaults cache-on and shares the `-dirty` git-SHA). The nav store lives under `dirs::cache_dir()/prism/nav` (macOS: `~/Library/Caches/prism/nav`):

```bash
rm -rf "${XDG_CACHE_HOME:-$HOME/Library/Caches}/prism/nav"
cd eval && uv run tier-a --quick --allow-stale-sut
```
Expected: M2 precision/recall on the Rust anchors **unchanged or improved**, **fp not increased**, recall not decreased (NameOnly counts as a resolved edge under the default `--confidence all`). Paste the M2 strata into the PR. If recall drops on any anchor, STOP — that means an edge was dropped, not demoted; re-open the branch logic before proceeding.

- [ ] **Step 5: Commit the evidence note (if any artifact files were produced) and finalize**

If Tier-A wrote no committed baseline changes, there is nothing to add here — do NOT commit `eval/` run artifacts. If the run surfaced an intended baseline matrix change, stage only the sanctioned baseline file under `docs/eval/tier-a/` and commit:

```bash
git status   # confirm no stray eval/ run artifacts are staged
```

- [ ] **Step 6: Independent review (codex xhigh on the diff)**

Dispatch a read-only codex (gpt-5.5, xhigh) review of the branch diff against the spec, confirming: no dropped edges (recall-safe), the relabel paths preserve NameOnly, single-candidate + trait-CHA arms untouched, and the §14 recovery invariant holds (demote stays terminal). Fold blockers (verify → fix → re-review); fold mechanical nits directly.

---

## Notes for the executor

- **DRY/YAGNI:** the entire production change is the single `else if pool.len() > 1` arm in Task 1 Step 5. Tasks 2–4 are tests and verification. Do not add a new `ResolutionKind`, a file-count gate, or a language gate — all explicitly rejected in the spec (§3, §12).
- **Recall is the invariant:** every test asserts the candidate count is preserved (`r.len()` unchanged vs. today) and only `confidence` changes. If any change reduces `r.len()`, it is wrong.
- **Recovery (spec §14):** do not mutate `methods_by_scope`, receiver outcomes, or any cached/serialized state in the demote arm — it must remain a pure confidence change so a future upstream capability re-resolves the same site to Exact.
