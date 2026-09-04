# Handoff — prism Phase 0 branch (`phase0-sarif-targets-api`): SARIF · `prism targets` · `prism::api` · README truth pass

**Written:** 2026-09-04T11:40 local · **By:** session_015U8HwBTAFzFzqJbbq82JBT (Claude Fable 5.1 controller; implementers Sonnet 5 / Opus 5 / codex gpt-5.6-sol; reviewers Opus 5 / Sonnet 5 / codex gpt-5.6-terra) · **Provider:** claude (+ codex via the a2a-bridge)
**Workspace:** `shoedog/prism` worktree `~/code/slicing-phase0` · branch `phase0-sarif-targets-api` · **Measured state:** `[MEASURED]` HEAD `9040fc2` + this docs commit (push/PR state) · Tree CLEAN · Probe `git status --short` · Output empty
**Predecessor:** none — first in lane (the roadmap docs and the controller ledger live in `~/code/tools`)
**Truth ordering:** measured live state > explicit owner/contract authority within its scope > this handoff for current operational state > earlier handoffs and non-authoritative summaries. A conflict between tiers stays OPEN in §0 — never resolved by document class alone.
**Provenance:** written live by the controller. `[MEASURED]` claims were probed by this writer; `[INHERITED]` claims were not.

## 0. Gating facts — settle these before starting anything below

**(a) Lane ownership** — another agent works in the main checkout `~/code/slicing` on `main`; this branch was built entirely in the worktree `~/code/slicing-phase0` (plus a codex implementer clone `~/code/slicing-phase0-sol` and a detached review copy `~/code/slicing-phase0-review`). `[MEASURED]` `git worktree list` — **RESOLVED** (worktree isolation; never write to `~/code/slicing`)
**(b) Custody exposure** — `[MEASURED]` branch pushed to `origin` (`git@github.com:shoedog/prism.git`) and PR #229 opened (https://github.com/shoedog/prism/pull/229) on 2026-09-04 after the owner authorised push + PR ("1 - authorized"). The clone and the review copy are disposable duplicates. Not merged. — **RESOLVED** (merge remains the owner's call)
**(c) In flight / irreversible** — `[MEASURED]` nothing in flight; the final review, its fix wave and the scoped re-review are complete. — **RESOLVED**
**(d) Authorization granted but not exercised** — owner (2026-09-04): "proceed autonomously delegating to subagents"; "authorized to go over review cap as needed"; disputed sol findings may be adjudicated by a separate sol judge seat. Push + PR were authorised by the owner on 2026-09-04 and exercised (PR #229); merge is not authorised.

## 1. Resume order
1. Read `~/code/tools/LEDGER.md` (dispatch log + every ruling) and the archived SDD ledger `~/code/tools/reviews/prism-phase0-sdd/progress.md` (tasks 1–5 carry `complete` lines; the git-ignored `.superpowers/sdd/` workspace was deleted after archiving, per the SDD skill).
2. The final review is folded (spec §11, last entry). The SDD workspace evidence (task briefs/reports, the fix-wave report, the progress ledger) is archived durably at `~/code/tools/reviews/prism-phase0-sdd/` and the review texts at `~/code/tools/reviews/phase0-*.md`.
3. With the owner's authorisation only: `git push -u origin phase0-sarif-targets-api` and open the PR with the body at `~/code/tools/reviews/phase0-pr-body.md`.
4. Next roadmap work is item 2 (DataFlow confidence via reaching definitions): spec v3 + plan v1 in `~/code/tools/specs/2026-09-04-prism-item2-dataflow-confidence-{spec,plan}.md`; the plan owes seven controller rulings and a re-alignment to spec v3 before its Task 1.

**STOP conditions:** any write to `~/code/slicing`; any push/merge without owner authorisation; touching a spec §6-forbidden file (`src/algorithms/**`, `src/cpg/**`, `src/cpg_cache.rs`, `src/navigation/**`, `src/resolution*.rs`, `src/call_graph.rs`, `src/ast.rs`, `src/languages/**`), a cache constant, or `Cargo.toml` dependencies; a byte-control diff (`scripts/phase0-byte-control.sh` must report 1598/1598 identical against `~/code/tools/bin/prism-base-c220525`).

## 2. State ledger

| Item | State | Evidence / correction |
|---|---|---|
| Design spec v5 (settled) | done | `docs/superpowers/specs/2026-09-04-prism-phase0-sarif-targets-api-design.md`; §11 review record (sol r1–r3, Opus r1; later amendments per task review); reviews mirrored in `~/code/tools/reviews/phase0-spec-*.md` |
| Plan v3 | done | `docs/superpowers/plans/2026-09-04-prism-phase0-sarif-targets-api.md` |
| Task 1 `finding_confidence` | complete | `[MEASURED]` commits `a894b3f`, `a7cf31e`; 18 unit tests |
| Task 2 `--format sarif` + allow-list | complete | `[MEASURED]` `68c74d4`, `f627d0a`; 11 CLI + 13 unit tests; SARIF validates against the official 2.1 schema (0 errors, `~/code/tools/logs/closeout/closeout-gates.log`) |
| Task 3 `prism::api` + byte control | complete | `[MEASURED]` `444b673`, `7a3c04b`; 10 integration tests; `scripts/phase0-byte-control.sh` 1504/1504 → 1598/1598 identical at every later head (controller-run logs `~/code/tools/logs/task*-byte-control-controller.log`); cache-decision control identical |
| Task 4 `prism targets` | complete | `[MEASURED]` `ab4d656`, `fdb432c`, `08dc291`; 7 CLI + 13 mapping tests; targets validate against `docs/contracts/targets.schema.json` (0 errors); 65-row `ABSENCE_PAIRS` with the mechanical counterpart rule |
| Task 5 README truth pass + `src/cli.rs` + gate | complete | `[MEASURED]` `c8a7dba`, `cd16609`; `readme_test` 3/3; doc-test 2/2; `--help` diff vs base = only the new subcommand/format lines |
| Roadmap row | done | `docs/analysis/prism-post-plan-roadmap.md` item 19 (`afaf3eb`) |
| Full suite | measured | base `c220525c`: 3543/0/1; final head `602a6ed`: **3810/0/1 across 29 binaries + 2 doc-tests** (controller, `~/code/tools/logs/closeout/full-suite-final.log`, `doctests-final.log`) |
| Tier-A `--matrix-only` | measured | 104/104 on both the branch and base binaries (`~/code/tools/logs/closeout/tier-a-matrix-{branch,base}.log`) |
| Final whole-branch review | complete | terra + Opus (`With fixes`) → fix wave `374b888`/`07f001e` → Sonnet scoped re-review: all addressed, no new breakage (`~/code/tools/reviews/phase0-final-review-terra.md`, ledger) |
| PR | **opened, not merged** | https://github.com/shoedog/prism/pull/229 (body from `~/code/tools/reviews/phase0-pr-body.md`); branch pushed to `origin` |

## 3. Corrections to standing documents and memory

| Location | Stale or false assertion | Correction |
|---|---|---|
| `README.md` (12 claims) | `slicing` binary, six MCP tools, three formats, `tests/fixtures/cve/`, cache scope, framework list… | `[MEASURED]` corrected in `c8a7dba`/`cd16609` and pinned by `tests/cli/readme_test.rs` |
| `skills/prism-code-slicing/SKILL.md` | "binary is named `slicing`" + `slicing …` examples | `[MEASURED]` corrected in `cd16609`; gate asserts no fenced `slicing ` line |
| `CLAUDE.md` | module maps / formats / severity vocabulary | `[MEASURED]` updated in `c8a7dba` (api/, targets/, finding_confidence.rs, output/sarif*.rs, cli.rs; seven formats; four severities) |
| spec §2.3.3 "`parse_diagram_cap` stays in main.rs" | predates the `src/cli.rs` split | `[MEASURED]` it lives in `src/cli.rs` (a `value_parser` must be in the library crate); recorded in the Task 5 report — spec sentence NOT yet amended (work item) |
| `~/code/tools/04-prism-plan-roadmap.md` §2.2 | targets `confidence: exact|scoped|nominal`; `-a …,angle,…` | superseded by `docs/contracts/targets.schema.json` (`exact|nameonly|unlabeled`); `angle` constructs no findings — owner's document, not edited |

## 4. Open work

| # | Work | State | Exact next action | Blocked by | Identifiers |
|---:|---|---|---|---|---|
| 1 | Final review fold | done | — | — | spec §11 |
| 2 | Push + PR | blocked | owner authorisation, then `git push -u origin phase0-sarif-targets-api`, `gh pr create --title "Phase 0 interfaces: --format sarif, prism targets, prism::api, README truth pass" --body-file ~/code/tools/reviews/phase0-pr-body.md` | owner | — |
| 3 | Spec §2.3.3 sentence about `parse_diagram_cap` | done | — | — | `ffcf6bd` |
| 4 | Follow-ups (spec §9) | filed | multi-run `paper` gap; clap `name` rename with `tests/cli/version_test.rs:15` + `eval/tier_a/sut.py::parse_version`; `angle`/`delta` findings; lossy anchors; structured `FindingHint`; typed call edges via `api`; `--strict` live exit-3 case once a fallible producer exists | — | — |
| 5 | Roadmap item 2 | drafted | rule on the plan's seven items; re-align plan to spec v3; open a new branch/worktree | owner scheduling | `~/code/tools/specs/2026-09-04-prism-item2-*` |

## 5. Invariants and traps — do not do these
- Never write to `~/code/slicing` — another agent owns it; use the worktree.
- Never let `text/json/paper/review/mermaid/callers` change bytes, stderr or exit codes — run `scripts/phase0-byte-control.sh <base-bin> target/release/prism` (base binary preserved at `~/code/tools/bin/prism-base-c220525`); a `DIFF` is a STOP.
- Never read parse quality from `SliceFinding.parse_quality` in a serializer — it is `None` for clean AND unannotated findings; use `finding_confidence::parse_quality_for` with the sparse map + parsed files.
- Never bump a cache constant in this branch; item 2 owns the 55→56 transition.
- Codex implementers cannot commit inside a git worktree (sandbox cannot write `.git/worktrees/*`) — use the clone `~/code/slicing-phase0-sol` (`git reset --hard` to the worktree head first) and integrate with `git fetch <clone> <branch> && git cherry-pick FETCH_HEAD`; never force-add `.superpowers/**`.
- Codex reviewers carry a verification hook that runs `cargo test` despite read-only briefs — give them the detached review copy as cwd and tell the hook it does not apply.
- Test totals only from a complete log via `awk` over `test result:` lines.
- The review path parses only diff-listed files: a cross-file caller must be listed in the JSON diff (empty `diff_lines`) to exist in the CPG (fixture trick used in `tests/fixtures/targets/diff.json`).

## 6. Identifiers

| Item | Verbatim |
|---|---|
| exact base | `c220525c6746d635d99a7a084791cfad4f0276d9` |
| branch head at writing | `cd166095cc8a83cf40ef0256a15a1fc0439eb8d4` |
| worktree / clone / review copy | `/Users/wesleyjinks/code/slicing-phase0` · `/Users/wesleyjinks/code/slicing-phase0-sol` · `/Users/wesleyjinks/code/slicing-phase0-review` |
| base binary | `/Users/wesleyjinks/code/tools/bin/prism-base-c220525` (sha256 `299f02c4f15c4e7d…`) |
| controller ledger / handoff / verification | `/Users/wesleyjinks/code/tools/{LEDGER.md,HANDOFF.md,VERIFICATION.md}` |
| targets contract | `docs/contracts/targets.schema.json` (authoritative; mirror `~/code/tools/contracts/targets.schema.json`) |
| SDD evidence archive | `~/code/tools/reviews/prism-phase0-sdd/` (briefs, reports, fix-wave report, progress ledger) |
| bridge | `http://127.0.0.1:18080`, release `07aee33e487fdce6`; codex contexts `tools-phase0-*-20260904` |

## 7. Refutation verdict and owner questions

**§2c verdict:** SURVIVED · claim: "the branch adds SARIF, targets, and the api facade without changing any existing output byte, cache decision, or resolution result" · pass: INDEPENDENT (per-task adversarial seats, two final whole-branch seats, controller-run byte control at every head incl. the final one, Tier-A matrix on both binaries) · evidence tier: TEST-BACKED · record: `~/code/tools/VERIFICATION.md`, `~/code/tools/logs/closeout/`

**Questions the owner owes an answer to:**
1. Authorise `git push` of `phase0-sarif-targets-api` and opening the PR (body drafted)?
2. Schedule roadmap item 2, and rule on the seven items in its plan's "Controller Rulings Required Before Dispatch"?
3. The clap `name = "slicing"` rename (follow-up) — coordinate with the two version-grammar consumers now or later?
