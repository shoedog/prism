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
| **decorated** double-capture | **IMPLEMENTED (T1-T5, 6 commits `fbae3af..b959adf`) + acceptance GREEN** → **final diff-review running** (bxwmnos7n) | `decorated-double-capture` (wt `/tmp/prism-decorated`), tip `b959adf` |

**Decorated acceptance (deco vs current-main, both have #131):** pydantic `self_receiver` +79 / `qualifier_owner`
+20 / `free_single` +178 Exact (precision buy); large correct **duplicate-edge collapse** (decorated body
double-scan removed: `local_def` 9825→5304, `free_multi` 25293→13579, `unresolved` 35687→30914 DOWN =
no recall loss; `total_call_sites` unchanged = byte-deduped sites); `multi_target_exact_sites` 439→316 DOWN
(dup free-fn Exact collapsed). **Rust(ripgrep)/Go(caddy) byte-identical; Tier-A matrix 40 ok; suite
2470/2563 pass; fmt clean.** Codex caught+fixed a real blast-radius bug: decorated DFG is wrapper-canonical
but `enclosing_function()` returns inner → taint seed identity mismatch broke decorated Flask taint (a
pre-existing sanitizer test failed) → fixed in `synthesize_target_seed_paths` + regression. If diff-review
SHIPs → PR → merge on green CI → slice 2.
| **2** typed receivers | **architect DONE** — awaiting spec (do AFTER decorated) | memo `/tmp/slice2-architect-out.md` |
| **3** import-scoping/free_multi | **architect DONE** — awaiting spec | memo `/tmp/slice3-architect-out.md` |
| **1b** inheritance MRO | **architect DONE** — awaiting spec | memo `/tmp/slice1b-architect-out.md` |

### Architect results + execution order (all 3 done; measured buys are SMALLER than headlines)
**ORDER: decorated (in flight) → 2 → 3 → 1b** (by measured Python buy; all sequential off fresh main).
- **2 typed receivers (~700 Python sites):** owner-lookup hits ~171 FastAPI + ~542 pydantic; **Express ≈0**
  (CommonJS Router, no in-repo ES classes — defer JS). Currently in `dropped_multi_owner` + `r6_single_owner`
  NameOnly. **Design = Option B "hit-or-fallthrough":** open the `recover_simple_ident` (`resolution.rs:320`)
  + `receiver_type_in_fn` (`ast.rs:403`) Rust|Go gates for Python/JS/TS; recover typed params + constructor
  locals + annotations; feed R6 `owner_lookup`; **on MISS fall through to R6 residue, do NOT drop-to-
  ExternalReceiver** (FastAPI has 1,416 syntactic recoveries with no owner hit → a drop would spike).
  Rust/Go byte-identical. First-merge guard: constructor-locals + explicit annotations only (skip
  import-qualified/attribute type syntax, TS structural, CommonJS). Bare owner-key (demote-on-multi safety).
- **3 import-scoping/free_multi (~300 sites; 25k is EDGES not sites):** same-dir is UNSOUND for Python/JS
  (siblings not in scope w/o import); same-file already R4. **Design = Option B import-binding rung:**
  richer `ImportBinding` (local name, module path, **imported member** — aliases currently lose it), resolve
  module path → repo file, add a rung after R4-local/before R5-free-multi; Exact on single candidate, multi
  demotes, external/unresolved fails open to R5. Buy: ~241 pydantic + 64 fastapi import-singletons; residual
  ~2,647 pydantic genuinely ambiguous (stays NameOnly, correct). Named imports first (JS default/CommonJS
  deferred).
- **1b inheritance MRO (16 sites — smallest, LAST):** 12 FastAPI + 4 pydantic + 0 excalidraw in-repo
  inherited self/this; external bases dominate (SCIP). Option A span-keyed `class_bases` (preserve 1a's
  `(file,class_span)` identity), walk bases after same-class miss, external/ambiguous = MRO barriers,
  conservative single-provider Exact. New Tier-A fixture needed (`inherited_override` is mislabeled).

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
