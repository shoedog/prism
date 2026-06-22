# Handoff — Glob-Expansion Slice + Precision-Recovery Roadmap (2026-06-22)

Resume doc for the prism owner-key-collision precision arc after the **cross-crate `use`
resolution** slice. Pairs with the memory record
`~/.claude/.../memory/project_prism_owner_key_collision.md` (the authoritative arc log) and
`docs/ruff-typepath-recovery-roadmap-2026-06-21.md` (now partly superseded — see §4).

---

## 1. Where things stand (one screen)

- **PR #124 OPEN** (`cross-crate-use-resolution` → `main`): per-crate dependency-gated cross-crate
  `use` resolution. Implemented, sound, **codex xhigh final review SHIP (zero findings)**. Owner
  merges **when CI is green**.
- **CI is RED** on one test — `tests/name_resolution/build_wiring_test.rs::malformed_member_manifest_does_not_discard_valid_sibling_manifest`.
  **Another LLM is resolving it** (owns `src/repo_loader.rs` right now — do NOT touch). Root cause +
  agreed fix in §3.
- **Next slice GREENLIT (not started): deferred-glob member expansion.** Verify-first spike done
  (§2). Best built **on top of #124** and in a **fresh session** (this one is ~70%+ context).
- **The headline reframing (§4): the roadmap's per-bucket residue projections are over-estimated and
  the buckets are COUPLED.** ruff resolution is facade-mediated; the real lever is glob expansion +
  cross-crate together.

What #124 delivers on ruff (measured, sound, M2-clean): **+2698 Exact edges, −1526 unresolved,
−1050 multi-owner drops, 0 new collision FPs.** The projected +428 cross-crate *collision* recovery
came in at **+38** (that narrow bucket was over-estimated; see §4). The value is the general
cross-crate resolution win, not the collision count.

---

## 2. The glob-expansion slice (GREENLIT — design + spike result)

**Problem.** prism Phase-1 does NOT expand glob re-exports: `pub use mod::*` is populated as a
"deferred-glob poison edge (no member expansion)" (`src/name_resolution/rust_populator/walk/items.rs`
~`:238`); a deferred glob → `GlobOutcome::Poison` (`src/name_resolution/engine.rs` ~`:244`/`:274`) →
`scope_member_lookup` returns `poisoned()` (`engine.rs` ~`:133`). ruff's crate roots are
**glob-re-export facades** (29 `pub use ::*` lines across crate roots; `ruff_python_ast/src/lib.rs`
opens with 7: `pub use nodes::*; pub use expression::*; …`). So `use ruff_python_ast::Stmt` (where
`Stmt` reaches the root via `pub use nodes::*`) resolves the *crate* (via #124) but the final
segment `Stmt` hits the deferred glob → Poison → #122 (A) declines → no recovery.

**Spike (throwaway, isolated worktree, 2026-06-22) — GREENLIT.** Rough hack in `glob_lookup`:
replace the `BindTarget::Pending => GlobOutcome::Poison` arm with "resolve the glob edge's
`(path, anchor)` via `resolve_path_guarded`, then `scope_member_lookup` the queried `(name, ns)` in
the target scope; single clean in-repo hit → `Hit`; zero → continue; cycle / ambiguous / multi-glob
→ keep `Poison` (recall-conservative — never invent a wrong single)."

Measured (sound — `multi_target_exact_sites` byte-identical, zero wrong Exacts):
- **Glob expansion ALONE on ruff: kind_exact +985** (qualified_owner +678, typed_param +260),
  **unresolved_unknown_name −910**, multi_target 46→46. A large, sound *general* buy.
- Collision `singleton` only **+7** in the spike — BUT the spike worktree was **mis-based off
  `main` (`1b40af0`), NOT the cross-crate branch** (gotcha: `isolation: worktree` branches off the
  default branch, not the current feature branch), so the cross-crate fallback was absent.
- **Synthetic probes proved the mechanism:** an *intra*-crate facade-glob collision recovers
  soundly under the spike; a *cross*-crate facade-glob collision needs **both** #124 (leading crate
  segment) **and** glob expansion (final type segment) — neither alone. So the cross-crate facade
  collision recovery is the **#124 + glob pair**; the combined ruff magnitude is **still unmeasured**
  (do NOT re-claim +428; measure in the slice's own acceptance on a #124 base).

**Verdict:** greenlit independently by the +985 sound general buy, and it completes the cross-crate
collision recovery #124 set up.

**Real-slice design must handle (spike gotchas):**
1. **Cycle guard** — globs can be mutually recursive (`a::*` ↔ `b::*`) and glob-of-glob; integrate
   with the existing `CycleGuard` and prove termination (the spike bailed to Poison on any glob
   re-entry — recall-safe but under-resolves legit diamonds).
2. **Multi-glob ambiguity (highest-risk)** — a name reachable via two *different* globs to two
   *different* items must be `Ambiguous`/`Poison`, never a silent pick. `policy.combine` dedups
   same-target; a real slice must guarantee distinct-target multi-glob → not-a-single.
3. **Visibility** — `pub use` does not launder privacy; a glob only brings `pub` members across a
   module boundary, and the re-export author's vantage matters (mirror the named-`Pending` arm in
   `resolve_rib`).
4. **Nested / multi-segment** — `pub use a::b::*` and glob-of-glob chains (`pub use submod::*` where
   `submod` itself re-export-globs). The spike resolved the full prefix via `resolve_path_guarded`,
   so these are reachable, but need explicit tests.
5. **CACHE_VERSION bump** — resolution-behavior change (the spike relied on `--no-cache`).
6. **Cross-crate prerequisite** — glob expansion is inert for cross-crate paths until #124's
   leading-segment fallback is present; state this dependency + gate ruff acceptance on both.

**Process:** full brainstorm → spec → plan → codex-review loop → implement → acceptance, per the
established pattern (§5). **Verify-first:** before committing to the design, re-run the spike on a
**#124 base** (cross-crate present) to measure the *combined* cross-crate-facade collision magnitude
on ruff — that is the number the spike could not get. Add a **glob-workspace fixture** to every
relevant test surface (see §6 lesson).

---

## 3. CI failure being fixed externally (context, do NOT act)

Test `malformed_member_manifest_does_not_discard_valid_sibling_manifest` (`build_wiring_test.rs`):
fixture `members = ["good", "bad"]` with `bad/Cargo.toml` malformed; expects
`cfg.workspace_members == ["bad", "good"]`. **Root cause = commit `04c6b0b`** (the glob fix):
it replaced the `[workspace].members` declaration recording with parsed-`[package]`-dir recording,
so a member whose own manifest fails to parse (`bad`) is dropped.

**Agreed fix (the other LLM is applying):** keep the parsed-`[package]`-dir recording (glob safety)
AND re-add the declared-member loop **skipping any entry containing `*`** (globs stay covered by the
parsed dirs; non-glob declared members like `bad` survive a malformed manifest → convention fallback
handles `bad/src/lib.rs`). No separate glob-*expansion* code is needed — the parsed-`[package]`-dir
recording already provides the concrete dirs for glob members. Known residual edge (acceptable, not
in the test): a malformed manifest *under* a glob member (`["crates/*"]` + `crates/bad` malformed) is
still dropped (we don't filesystem-glob); `from_convention` still records its `src/lib.rs` root.
**Verification the fix needs (the gap that let this through):** run the **full `--test
name_resolution`** (not just the focused test) + `--lib repo_loader::tests::` + re-confirm the ruff
call-stats glob buy (+985 kind_exact / +38 collision) is unchanged + `cargo fmt --check`. The
original miss: after `04c6b0b` I ran `--lib` (has `repo_loader::tests`) but not `--test
name_resolution` (where `build_wiring_test` lives).

---

## 4. The roadmap reframing (IMPORTANT — supersedes the per-bucket sizing)

`docs/ruff-typepath-recovery-roadmap-2026-06-21.md` sized the post-edition 1326 residue as
independent buckets: cross-crate `use` **+428**, glob-import **+304**, plus correct-keep-all /
poison / downstream / minor. **Two slices now show this sizing is wrong:**
- #122 (A) prune-through-`use`: projected ~1,586 → realized **+0** on ruff (cross-crate-gated).
- #124 cross-crate `use`: projected +428 collision → realized **+38** collision (+2698 *general*).

**Why:** ruff resolution is **facade-mediated** — cross-crate types are reached through crate-root
glob re-export facades (`pub use mod::*`). The "cross-crate +428" and "glob-import +304" buckets are
**COUPLED, not independent**: a cross-crate facade collision needs the leading crate segment (#124)
AND the final type segment through the glob (glob expansion). **Glob-import member expansion is the
real next lever** — it unlocks the glob bucket AND releases the cross-crate collision recovery #124
set up, so it is worth MORE than its standalone +304 estimate. Both (A) (+0) and #124 (+38) under-
delivered for the SAME reason.

**Durable methodology lesson:** the residue-characterization sizing (counting `failopen_demote`
buckets) systematically over-estimates per-slice buy because it assumes buckets resolve independently
and ignores facade-mediation. **Measure the combined buy with a spike on the correct base before
committing to any per-bucket projection.** The general-resolution gains (Exact edges, fewer
unresolved/drops) are the reliable value signal, not the narrow collision counts.

---

## 5. Standing constraints + the review loop (carry forward verbatim)

- Commit trailer: `Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>`.
- PR body ends: `🤖 Generated with [Claude Code](https://claude.com/claude-code)`.
- **No push / no PR until the owner explicitly asks.** Branch off `main` for source changes.
- **Never** stage `eval/` or `docs/eval/` run-artifacts (leave untracked).
- Review loop: brainstorm → spec (codex xhigh review loop) → plan (codex xhigh review loop) →
  a2a-bridge codex `workspace-write` edit-only implement + **host commits** (codex can't write
  `.git`) → host acceptance → final codex xhigh diff review → PR (owner-gated).
- **A separate Opus 4.8 subagent judges + folds codex review findings** to save orchestrator
  context; on disagreement/uncertainty the judge escalates to the owner. BLOCKER/MAJOR → verify →
  fold → re-review; mechanical → fold without re-review.
- codex correctly **halts mid-implementation** if a plan RED claim doesn't match (it caught a real
  plan test-bug this slice — trust that signal; fix the plan, don't force the test).

---

## 6. Environment gotchas (consolidated)

- **a2a-bridge:** `~/code/a2a-bridge/target/release/a2a-bridge run-workflow <id> --config <abs.toml>
  --input <abs.md> --session-cwd /Users/wesleyjinks/code/slicing --out <out>`. `--input` does NOT
  reach codex (reads `prompt_file` only) → bake the task into the prompt. workspace-write can't write
  `.git` → edit-only + host commits. Wrap in `timeout 900`; use a **fresh `[server] addr` port** each
  run (8170–8189 used this arc); verify a real codex-acp spawned (`pgrep -fl 'node.*codex-acp'`). A
  separate concurrent `cancel-tokens` workstream also runs codex (read-only + danger-full-access) —
  don't touch it; match yours by config path / port.
- **`isolation: worktree` (Agent tool) branches off the DEFAULT branch (`main`/`1b40af0`), NOT the
  current feature branch** — the spike was mis-based because of this. For a spike that must build on a
  feature branch, create the worktree manually off the right SHA or have the subagent
  `git reset --hard origin/<branch>` first.
- **macOS:** bare `cargo test --test cli` stalls at `_dyld_start` → `--no-run` then run the freshest
  `target/debug/deps/cli-*`. `--lib`, `--test integration`, `--test ast`, `--test name_resolution`
  run fine. `cargo test` takes ONE name filter before `--`. **A `repo_loader`/`workspace_members`
  change MUST be checked with `--test name_resolution` (build_wiring_test), not just `--lib`.**
- **Tier-A:** `cd eval && uv run tier-a --matrix-only --allow-stale-sut` (seconds); ruff M2 =
  `uv run tier-a --corpus ruff --allow-stale-sut` (NOT `--quick`, which forces prism-only;
  `baseline_invalid=false` is the valid gate; the cross-crate/glob gains land OUTSIDE the adjudicated
  sample so M2 shows 0-regression "no-harm", not scored improvements). Corpora under
  `~/code/bench-repos/`. `--allow-stale-sut` only with an immediate preceding `cargo build --release`.
- **call-stats:** `./target/release/prism nav --no-cache call-stats --repo <repo>` (NOT
  `--format json`; `--no-cache` is a `nav` global before the subcommand). The relevant counters:
  `recovery_typepath.{singleton,failopen_demote}`, `kind_exact`/`kind_nameonly` (per-kind),
  `multi_target_exact_sites` (the collision-FP canary — must stay flat), `unresolved_unknown_name`,
  `dropped_multi_owner`. To attribute a shift (relabel vs drop), diff the FULL breakdown vs a `main`
  worktree build (`git worktree add --detach /tmp/cmp origin/main`).

---

## 7. Deferred items + follow-ons

- **`RustCrateConfig.lib_path` is flat (last-wins repo-wide)** (`mod.rs` ~`:85`) — a workspace with
  multiple members using *explicit custom* `[lib] path` resolves only the last via the `lib_path`
  gate. Recall-conservative; convention `src/lib.rs` roots unaffected (the dominant case). A real fix
  = per-member lib-path capture. Documented in the cross-crate spec/plan; not urgent.
- **Malformed manifest under a glob member** (§3 residual edge) — `from_convention` still picks up its
  `src/lib.rs` root, just unmapped to a member dir. Acceptable; revisit only if a corpus needs it.
- **Glob-expansion slice (§2)** — the next big lever; needs the combined-base spike first.
- **After glob expansion:** re-measure the residue on ruff (the buckets will have shifted); the
  remaining same-name collisions are likely correct-keep-all shadows / poison / downstream-method —
  diminishing returns. Re-characterize before sizing any further slice (per §4's lesson).
- **Pre-#106 receiver-typing / Python gaps** and other arcs are tracked in their own memory files;
  not part of this collision arc.

---

## 8. Key references

- Memory: `~/.claude/projects/-Users-wesleyjinks-code-slicing/memory/project_prism_owner_key_collision.md`
  (authoritative arc log) + `MEMORY.md` index.
- Cross-crate spec/plan: `docs/superpowers/specs/2026-06-21-prism-cross-crate-use-resolution-design.md`,
  `docs/superpowers/plans/2026-06-22-cross-crate-use-resolution.md`.
- Roadmap (partly superseded — see §4): `docs/ruff-typepath-recovery-roadmap-2026-06-21.md`.
- Glob seam: `walk/items.rs` (`UseItem::Glob` → deferred poison edge), `engine.rs` `glob_lookup` /
  `GlobOutcome` / `scope_member_lookup`. Cross-crate machinery (the prerequisite):
  `crate_deps_by_root` (`graph.rs`), `extern_crate_root` + `crate_root_of` (`rust_policy.rs`),
  `lib_root_member_dir` + `crate_deps_by_root` build (`builder.rs`), per-member dep capture
  (`repo_loader.rs::parse_rust_crate_config`).
