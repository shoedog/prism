# Edition Anchoring-Class Uniformity — Cold-Session Execution Handoff

**Date:** 2026-06-21
**Branch:** `edition-anchoring-uniformity` (off `main` @ `64a0b1e`, which is the squashed #122 (A))
**Status:** spec + plan BOTH codex-xhigh SHIP. **Ready to execute.** No code written yet.

## TL;DR — what to do next

Execute the 3-task plan
[`docs/superpowers/plans/2026-06-21-edition-anchoring-uniformity.md`](../plans/2026-06-21-edition-anchoring-uniformity.md)
via the proven **a2a-bridge codex `workspace-write` edit-only + host-commit** pattern (Tasks 1–2),
then run Task 3 acceptance on the host, then a final codex xhigh diff review, then PR (owner-gated).
The plan has complete verbatim code for every step. **No push/PR until the owner explicitly asks.**

## The slice in one paragraph

prism's scope-graph disproof (the #121/#122 same-name owner-`::` collision recovery) bails
keep-all on `!graph.edition_uniform` (`src/resolution.rs:1337`). `edition_uniform` is computed
all-identical (`src/repo_loader.rs:359`). ruff is a mixed-edition workspace (`{2021, 2024}`) so the
disproof is gated off entirely → 0 recovery. This slice (a) parses `edition = { workspace = true }`
inheritance (today the table mis-parses to 2015), and (b) relaxes `edition_uniform` to
**anchoring-class** uniformity (all-2015 OR all-2018+), because prism anchors only at the 2015/2018+
boundary (`rust_policy.rs:82`, `is_2018_plus`). Result: the unchanged disproof runs on pure-2018+
mixed workspaces. **Verified buy: +260 sound collision recoveries on ruff, 0 new collision FPs.**

## Verified context (do NOT re-derive — it's measured)

- **Root cause** confirmed via a throwaway spike (force `edition_uniform=true` on ruff): recovery
  jumps `singleton` 0→260, `failopen_demote` 1586→1326, `kind_exact[qualified_owner]` +260,
  `kind_nameonly[qualified_owner]` −1092 (wrong-owner edges pruned), `multi_target_exact_sites`
  46→46 (no new FPs).
- **Soundness** (codex-verified): `RustPolicy` branches ONLY at `edition >= 2018`; no 2021/2024
  behavior. Within the 2018+ class every crate anchors identically → any 2018+ global edition is
  correct → no real edge mis-dropped.
- **The recall-safety MAJOR** (codex caught, Opus-judge verified + folded): prism collects ALL
  `Cargo.toml` repo-wide into ONE manifest set, so a multi-workspace repo with roots on opposite
  anchoring sides could mis-resolve a 2015 workspace's `{workspace=true}` crates to 2024 →
  `editions_seen` falsely all-2018+ → wrong recovery. **Fix (in the plan):** collect ALL
  `[workspace.package]` editions into a `BTreeSet workspace_editions` and gate on the two-term AND
  `anchoring_class_uniform(editions_seen) && anchoring_class_uniform(workspace_editions)`. No
  residual hole (all three 2015 sources caught; codex re-review SHIP).
- **Follow-on slices** (NOT this one; see [`docs/ruff-typepath-recovery-roadmap-2026-06-21.md`](../../ruff-typepath-recovery-roadmap-2026-06-21.md)):
  cross-crate `use` resolution (+428 on ruff, the largest residue bucket — needs the crate-name→root
  map persisted into `ScopeGraph`), then glob-import resolution (+304). The other ~594 of ruff's
  1,586 are correct keep-all shadows / poison / downstream-method / minor.

## The change (what the plan implements)

All in `src/repo_loader.rs::parse_rust_crate_config` + a cache bump + 2 comment fixes:
- **Task 1** — pre-scan collecting `workspace_editions: BTreeSet<u16>` (+ a representative scalar);
  resolve `{ workspace = true }` against the representative; replace `edition_uniform = len()<=1`
  with the two-term AND; add the `anchoring_class_uniform` helper. Driven by 3 repo_loader unit
  tests + 1 end-to-end integration behavior test (RED→GREEN). Insertion points: pre-scan before the
  per-manifest loop (`:287`), edition block (`:301-311`), uniform line (`:359`), helper after
  `parse_edition` (`:367-375`). `BTreeSet`/`parse_edition` already in scope (`:9`/`:367`).
- **Task 2** — `CACHE_VERSION` 16→17 (`src/cpg_cache.rs:60`) + the pin test
  `cache_version_is_16_*` → `cache_version_is_17_*` (`:564-568`) + reword the two `edition_uniform`
  doc comments (`src/name_resolution/rust_populator/mod.rs:89`, `src/name_resolution/graph.rs:89`)
  from "agreed on one edition" → "same anchoring class (2015 vs 2018+)".
- **Task 3** — acceptance (no commit): full test surface, `fmt --check`, Tier-A `--matrix-only`
  (0 regression), ruff M2 `--corpus ruff` (NOT `--quick` — it forces prism-only; expect
  `baseline_invalid=false`, 0 regression), and the `call-stats` +260 delta on ruff.

## Execution mechanism (the proven #122 pattern)

codex under `workspace-write` edits + runs `cargo` but CANNOT write `.git` → **host commits**.
Per-task commit messages + exact `git add` sets are IN THE PLAN. Trailer:
`Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>`. NEVER stage `eval/` or
`docs/eval/`.

**a2a-bridge:** binary `~/code/a2a-bridge/target/release/a2a-bridge`; configs in
`~/code/a2a-bridge/examples/`; prompts in `~/code/a2a-bridge/prompts/`. Invocation:
```
cd ~/code/a2a-bridge && timeout 900 ./target/release/a2a-bridge run-workflow <id> \
  --config <abs.toml> --input <abs.md> --session-cwd /Users/wesleyjinks/code/slicing --out <out>
```
Edit-only impl config (copy an existing one, e.g. `a2a-bridge.use-import-prune-impl-codex.toml`):
`sandbox_mode="workspace-write"`, `sandbox_workspace_write.writable_roots=["/Users/wesleyjinks/code/slicing","/Users/wesleyjinks/.cargo"]`, `approval_policy="never"`, `effort="high"`.
Review config: `sandbox_mode="read-only"`, `model="gpt-5.5"`, `effort="xhigh"`.

**GOTCHAS (all hit this session):**
- `--input` does NOT reach codex (it reads only `prompt_file`) → BAKE the task subset into a
  chunk-specific prompt file (point codex at the plan + say "do Task 1 and Task 2 ONLY, leave
  uncommitted, no git").
- a2a-bridge/codex-acp had ONE transient spawn hang (1h+, zero output). ALWAYS wrap in
  `timeout 900` and, ~8s after dispatch, confirm a real codex-acp spawned:
  `pgrep -fl 'node.*codex-acp' | grep -v 'eval\|zsh'`. Use a fresh `[server] addr` port each run
  (this session used 8170–8175).
- A SEPARATE concurrent workstream `cancel-tokens-impl` runs codex in `~/code/a2a-bridge` from
  another session (it showed as a `danger-full-access` codex). Don't touch it; match YOUR process
  by its config path / `read-only`-or-your-`workspace-write`.
- macOS: bare `cargo test --test cli` stalls at `_dyld_start` → use `--no-run` then run the freshest
  `target/debug/deps/cli-*` binary. `--lib`/`--test integration`/`--test ast` run fine.
- `cargo test` accepts only ONE test-name filter before `--` → use a broad module filter
  (`cargo test --lib repo_loader::tests::`).
- Shell flakiness: `tr`/`wc`/`python3`/`jq`/`head`/`awk` intermittently "command not found";
  `grep`/`sed`/`cat`/`git`/`cargo` reliable. Read files with the Read tool, not `cat | tail`.

## Review loop (owner's standing pattern)

codex implement(high) / review(xhigh) gpt-5.5. **A separate Opus 4.8 instance judges + folds
codex review findings to save the orchestrator's context; on disagreement/uncertainty the judge
escalates to the owner.** FLAWED/blocker → verify → fold → re-review; mechanical → fold without
re-review. Spec + plan already passed this loop (SHIP). After execution: one final codex xhigh diff
review of the branch, fold findings, THEN PR (owner-gated).

## Corpora + measurement

ruff = `~/code/bench-repos/ruff` (50-crate workspace, `[workspace.package] edition="2024"`,
crates mix explicit `2021` + `{workspace=true}`). call-stats: `prism nav --no-cache call-stats
--repo <ruff>`. The pre-change baseline snapshot is `/tmp/cs-branch-ruff.json` (may be gone in a
cold session — rebuild from `main`/`64a0b1e` if needed). Tier-A: `cd eval && uv run tier-a
--matrix-only --allow-stale-sut` (seconds) and `uv run tier-a --corpus ruff --allow-stale-sut`
(minutes, rust-analyzer). prism-self `--quick` is `baseline_invalid` on any feature branch
(SHA-drift) — inconclusive, not a regression; ruff M2 is the real validator.

## Branch commits (in order)

```
9fca429 docs(plan): edition anchoring-class uniformity implementation plan   (+ 3 mechanical nit folds)
133d1dd docs(spec): fold codex MAJOR — workspace-edition SET term
77b9042 docs(spec): edition anchoring-class uniformity
c78abf1 docs: ruff type-path collision recovery roadmap
64a0b1e feat(resolution): prune owner collisions ... use import   (= origin/main, #122 (A))
```

## Memory pointers

`~/.claude/.../memory/project_prism_owner_key_collision.md` — the full #120/#121/#122 + this
edition-redirect record (mixed-edition gating, +260, the cross-crate framing correction). Index:
`MEMORY.md`.
