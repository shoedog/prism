# Python Maturity — Autonomous Loop Handoff (2026-06-23)

**Live continuation tracker** for the overnight autonomous loop. Updated at each milestone (after each
spec, each plan, slice midpoint, slice end). If the session compacts or the owner returns, THIS is the
source of truth for loop state. Pairs with memory `[[project_prism_measurement_maturity]]`.

## Mandate (owner, 2026-06-23, owner asleep)
Loop to **complete**: the **decorated double-capture** slice (in flight) + **1b** (inheritance MRO) +
**2** (typed receivers) + **3** (import-scoping/free_multi). **Sub-slices allowed.** **Authorized to open
PRs after the review pipeline settles, and merge when CI passes (may merge before coverage settles).**
No owner questions while asleep — a genuine design fork gets a best-judgment call + a flag here for morning,
never a block. One stuck slice gets parked (documented here), not allowed to block the rest.

## Pipeline per slice (the loop)
spec → codex spec-review (xhigh) → **fold to sound** (re-review until no BLOCKER/MAJOR) → `writing-plans`
→ codex plan-review → **fold to sound** → **codex-implement** (effort=high, workspace-write; it CANNOT
write `.git` → orchestrator commits per-task after verifying) → **verify** (`cargo test` + `cargo fmt
--check` + acceptance) → codex diff-review (xhigh) → **fold to sound** → **open PR** → **merge on green CI**
(rebase-merge; coverage may be unsettled) → sync main → next slice off fresh main.

**Acceptance per slice (the gates):** the per-corpus buy (call-stats bucket rises) + canary
`multi_target_exact_sites` byte-flat + **Rust/Go (ripgrep, caddy) call-stats byte-identical** (owner
accepts this in lieu of `--quick`) + Tier-A `--matrix-only` 0-regr + suite green. Build both binaries via a
git WORKTREE (never swap the binary mid-measurement).

**Standing constraints:** explicit `git add <paths>` (never `-a`); NEVER stage `eval/` or `docs/eval/`;
commit trailer `Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>`; PR body ends
`🤖 Generated with [Claude Code](https://claude.com/claude-code)`; verify codex's work before committing
(its output is not safety-classified). See `[[feedback_workflow_preferences]]`.

## Slice status

| Slice | Stage | Branch / artifacts |
|---|---|---|
| **1a** self same-class | **MERGED** (#131, rebase, main `184208a`) | — |
| **decorated** double-capture | spec rev 1 written → spec-review SHIP-WITH-FIXES → **folding to rev 2** | `decorated-double-capture` (wt `/tmp/prism-decorated`), spec `b1f79db` |
| **1b** inheritance MRO | architect RUNNING (port 8221) | — |
| **2** typed receivers | architect RUNNING (port 8219) | — |
| **3** import-scoping/free_multi | architect RUNNING (port 8220) | — |

## Decorated slice — design + open review findings (folding to spec rev 2)
**Design:** wrapper-canonical — at extraction, skip the inner `function_definition` when its parent is a
`decorated_definition`; keep the wrapper as the single `FunctionId`. Removes the duplicate id / CPG node /
double body-scan; fixes free-fn duplicate-Exact + decorated-method NameOnly demotion (~20% pydantic
methods). **Spec-review (SHIP-WITH-FIXES) findings being folded into rev 2:**
- **BLOCKER:** the unwrap companion is incomplete — a single `unwrap_decorated(node)` helper must be used by
  `find_parameters_node` (`ast.rs:3922`), `function_body_node` (`:2607`), `statements_in_function`
  (`:3097`), `statement_spans_in_function` (`:3112`), `return_value_nodes` (`:2828-2893`). Centralize.
- **MAJOR:** NOT Python-only — C++ has the same wrapper/inner shape (`template_declaration` +
  `function_definition`, `queries.rs:129-133`). Reword to "Python decorator wrapper"; defer C++ template
  canonicalization; add a C++ template **canary (no-change)**.
- **MAJOR:** inventory currently drops the wrapper / keeps the inner (`navigation/inventory.rs:34-56`);
  wrapper-canonical **inverts** that — decide the contract, update its test, note start-line/kind churn.
- **MAJOR:** the manual fallback collector `collect_functions_manual` (`ast.rs:466-474`, reachable via
  `:286-288`) reintroduces the duplicate — apply the canonical filter there too, or centralize before
  `FunctionInfo`.
- MINOR: start-line shifts `def`→decorator (nav `nodes_at` churn); acceptance add helper + C++ + free-fn
  `LocalDef` + inventory tests.

## Environment / ops
- a2a-bridge: `~/code/a2a-bridge/target/release/a2a-bridge run-workflow <id> --input /tmp/sr-input.md
  --config <abs.toml> --session-cwd <repo> --out <abs> 2>err`, wrap `timeout`. `--input` does NOT reach
  codex (task in `prompt_file`). codex implement config = effort `high` + `sandbox_mode="workspace-write"`;
  review/architect = `xhigh` + `read-only`. Ports used this loop: 8210-8221 → **next ≥8222**.
- call-stats: `./target/release/prism nav --no-cache call-stats --repo <ABS>` → JSON. C/C++/large-TS DON'T
  complete. Acceptance corpora: fastapi, pydantic, express, excalidraw (Python/JS that complete); Rust =
  ripgrep, Go = caddy (byte-identical inertness check).
- Worktrees: `/tmp/prism-decorated` (decorated slice). Main tree `/Users/wesleyjinks/code/slicing` on main.
- Architect memos (raw codex output): `/tmp/{slice2,slice3,slice1b}-architect-out.md` (ephemeral —
  formalize into specs before relying on them).

## Next action
Fold the decorated spec-review into rev 2 (BLOCKER + 3 MAJORs above) → re-review → (sound) → writing-plans.
Then process the 1b/2/3 architect outputs into specs as they complete. Update this handoff at each
milestone.
