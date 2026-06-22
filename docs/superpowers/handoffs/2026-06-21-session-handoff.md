# Session Handoff — Prism Precision-Recovery Arc (2026-06-21)

Session-level resume doc. Pairs with the task-specific
[`2026-06-21-edition-anchoring-uniformity-execution-handoff.md`](2026-06-21-edition-anchoring-uniformity-execution-handoff.md)
(the immediate next-work detail). This is the strategic/narrative layer.

## Where things stand (one screen)

- **Shipped to `main` this session:** PR **#122** (A) "prune owner collisions through a single
  in-repo `use` import" — squash-merged (`main` tip `64a0b1e`).
- **Plan-ready, awaiting owner go to execute:** the **edition anchoring-class uniformity** slice
  on branch `edition-anchoring-uniformity` (spec + plan both codex-xhigh SHIP). Verified +260 on
  ruff. → see the execution handoff.
- **Sequenced follow-ons (not started):** cross-crate `use` resolution (+428 on ruff), then
  glob-import resolution (+304). See [`docs/ruff-typepath-recovery-roadmap-2026-06-21.md`](../../ruff-typepath-recovery-roadmap-2026-06-21.md).
- **Nothing pushed beyond #122.** Edition branch is local-only; PR stays gated on the owner's ask.

## The arc this session (what happened + why it matters)

This continues the same-name owner-key collision precision work
([[project_prism_owner_key_collision]] in memory). Three collision-recovery slices were already
in flight; this session shipped (A) and then chased "why does ruff still recover ~nothing."

1. **#122 (A) shipped.** Pending-import arm in the `ScopeResolution` disproof: a leading type
   segment reached through a single `use`/re-export that resolves to one in-repo `Item` is treated
   as directly bound, so the existing prune recovers the collision. Full pipeline (spec→plan→codex
   reviews→a2a-bridge codex edit-only impl→gates→codex xhigh SHIP). **KEY HONEST FINDING:** realized
   buy was **ruff +0** / ripgrep +9 / prism +1 — far below the spec's ~1,586 pre-gate. The owner was
   told plainly; the "cross-crate `use` unlocks ruff" framing in the PR was an *under-verified
   inference* (the FIRST over-claim).

2. **The redirect (verify-first paid off twice).** Owner chose "ship (A) + greenlight cross-crate."
   Before designing cross-crate, investigation (throwaway spikes + a `failopen_demote_reason`
   classifier probe) found the REAL blocker was **NOT cross-crate** and **NOT edition value** — it
   was **mixed-edition gating**: ruff is `{2021, 2024}`, prism's global `edition_uniform` bail
   (`resolution.rs:1337`) gates the whole disproof off on any non-identical-edition workspace. The
   "cross-crate unlocks 1,586" framing was wrong a SECOND time. Lesson reinforced: **measure the buy
   before committing to a design; the owner's repeated "spike-verify first" calls caught both
   over-claims.**

3. **The 1,326 residue characterized** (so follow-ons are sized, not guessed): 428 cross-crate
   `use`, 316 correct keep-all shadows, 304 glob, 161 poison, 82 downstream-method, ~35 minor.
   Cross-crate IS a real lever (+428) — just behind the edition prerequisite, not 1,586.

4. **Edition slice designed + reviewed.** Two-edit fix (parse `workspace=true` inheritance + relax
   `edition_uniform` to anchoring-class 2015-vs-2018+). codex spec review caught a real recall-safety
   MAJOR (multi-workspace mis-resolution) → Opus judge verified + folded the `workspace_editions`
   SET-term → codex re-review SHIP. Plan written, codex-reviewed (core faithful; 3 mechanical nits
   folded). **Ready to execute.**

## Standing constraints (carry forward verbatim)

- Commit trailer: `Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>`.
- PR body ends: `🤖 Generated with [Claude Code](https://claude.com/claude-code)`.
- **No push/PR until the owner explicitly asks.** Branch off `main` for source changes.
- **Never** commit `eval/` or `docs/eval/` run-artifacts during feature work (only sanctioned
  baselines).
- Review loop: codex implement(high)/review(xhigh) gpt-5.5 via a2a-bridge; **a separate Opus 4.8
  instance judges + folds review findings to save the orchestrator's context; on
  disagreement/uncertainty the judge escalates to the owner.** FLAWED/blocker → verify → fold →
  re-review; mechanical → fold without re-review.
- Brainstorm → spec (codex review loop) → plan (codex review loop) → execute (a2a-bridge codex
  edit-only + host commits) → final codex xhigh diff review → PR.

## Environment gotchas (consolidated)

- **a2a-bridge**: `~/code/a2a-bridge/target/release/a2a-bridge run-workflow <id> --config <abs.toml>
  --input <abs.md> --session-cwd /Users/wesleyjinks/code/slicing --out <out>`. `--input` does NOT
  reach codex (reads `prompt_file` only) → bake the task into the prompt. codex `workspace-write`
  can't write `.git` → edit-only + host commits. ONE transient spawn hang occurred (1h+, no output)
  → wrap every run in `timeout 900` and verify `pgrep -fl 'node.*codex-acp'` ~8s after dispatch; use
  a fresh `[server] addr` port each run (8170–8175 used this session).
- A **separate concurrent workstream** `cancel-tokens-impl` runs codex in `~/code/a2a-bridge` from
  another session (appeared as a `danger-full-access` codex). Not ours — don't kill it.
- macOS: bare `cargo test --test cli` stalls at `_dyld_start` → `--no-run` then run freshest
  `target/debug/deps/cli-*`. `--lib`/`--test integration`/`--test ast` fine. `cargo test` takes ONE
  name filter before `--`.
- Shell flakiness: `tr`/`wc`/`python3`/`jq`/`head`/`awk` intermittently "command not found";
  `grep`/`sed`/`cat`/`git`/`cargo` reliable. Prefer the Read tool over `cat | tail`.
- Tier-A: `cd eval && uv run tier-a --matrix-only --allow-stale-sut` (seconds);
  `uv run tier-a --corpus ruff --allow-stale-sut` (minutes, real validator). `--quick` forces
  prism-only (cli.py:732) and prism-self is `baseline_invalid` on feature branches (SHA-drift =
  inconclusive, not a regression). Corpora under `~/code/bench-repos/`.

## Next actions (in order)

1. **Execute the edition slice** (when owner says go) — per the execution handoff: a2a-bridge codex
   edit-only Tasks 1–2 + host commits → Task 3 acceptance (test surface, Tier-A matrix, ruff M2 for
   the +260) → final codex xhigh diff review → PR.
2. **Cross-crate `use` resolution** (+428) — the next slice; needs the crate-name→root map persisted
   into `ScopeGraph` (schema + cache). Brainstorm → spec → plan → execute.
3. **Glob-import resolution** (+304) — after cross-crate.

## Key references

- Roadmap: `docs/ruff-typepath-recovery-roadmap-2026-06-21.md`
- Edition spec: `docs/superpowers/specs/2026-06-21-prism-edition-anchoring-uniformity-design.md`
- Edition plan: `docs/superpowers/plans/2026-06-21-edition-anchoring-uniformity.md`
- Execution handoff: `docs/superpowers/handoffs/2026-06-21-edition-anchoring-uniformity-execution-handoff.md`
- Memory: `~/.claude/.../memory/project_prism_owner_key_collision.md` (+ `MEMORY.md` index)
- Disproof gate: `src/resolution.rs:1337`; edition compute: `src/repo_loader.rs:359`; anchor split:
  `src/name_resolution/rust_policy.rs:82`; cache: `src/cpg_cache.rs:60`.
