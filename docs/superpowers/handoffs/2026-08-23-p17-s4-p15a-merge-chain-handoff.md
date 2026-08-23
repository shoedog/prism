# Handoff — #17-narrow / #14-slice-4 / #15(a) merge chain + Ox calibration close-out

**Written:** 2026-08-23T15:40:00-06:00 · **By:** Claude Fable 5 session_01XLS1m6xaPSo7sZu2F8FtA2 (controller) · **Provider:** claude
**Workspace:** ~/code/slicing · main · **Measured state:** `[MEASURED]` HEAD 18b585a · Tree DIRTY (2 untracked: `.superpowers/`, `eval/snapshots/prism-fb81481dafa7.json` — pre-existing) · Probe `git status --short` + per-clone loop · Output inline in the 15:38 probe (ledger)
**Predecessor:** same session, pre-compaction (continuation of the 2026-08-22 review-path lane handoff §8)
**Truth ordering:** measured live state > explicit owner/contract authority within its scope > this handoff for current operational state > earlier handoffs and non-authoritative summaries. A conflict between tiers stays OPEN in §0 — never resolved by document class alone.
**Provenance:** written live by the worker. `[MEASURED]` claims were probed by this writer; `[INHERITED]` claims were not.

## 0. Gating facts — settle these before starting anything below

**(a) Lane ownership** — this session (controller) owns the lane; one bridge implementer context still open: `fable-itemP17-impl-sol-20260823` (idle; docs fix done). `[MEASURED]` session-status probes 15:3x — **RESOLVED (this writer), 15:40**
**(b) Custody exposure** — `[MEASURED]` all four lane clones dirty=0 unpushed=0 (p17-narrow f245bdf; p14s4-sol 38b85f2; p14s4-ox 66e70e0; p15a-ox f286805); durable artifacts 304 MB at `~/code/prism-lane-artifacts/2026-08-23-p14s3-p17/`; ledger `.superpowers/sdd/progress.md` is UNTRACKED (single copy on disk; not in git) — **OPEN** (tolerated all session; copy lives only on this machine)
**(c) In flight / irreversible** — `[MEASURED]` one Monitor live: `ba02hvr8q` (PR #186 CI settle; CI will report on the CONFLICTING head — see §1 step 1). No builds/oracles running. — **RESOLVED (probe 15:38)**
**(d) Authorization granted but not exercised** — owner standing rules, verbatim from memory/ledger: "merge on reviewer approval + CI green → squash-merge, report each"; "#1 but use sol and Ox instead of terra. if edits are needed use sol instead of terra" (#17 v9 close-out); "One targeted fix wave + final confirm" (#184 B1, exercised); "continue using Ox as a spec and code reviewer" (standing). #184 merge order AFTER #17; #185 last.

## 1. Resume order
1. ~~Resolve PR #186 conflict~~ **RESOLVED 15:45** `[MEASURED]`: merged origin/main into the branch, sole conflict was the spec file, kept branch v9, lib smoke 714/0, pushed head `97b1e88`. Remaining: wait for CI on 97b1e88 (the live checks monitor polls the PR and follows the new head). Original instruction (for reference only): in `~/code/slicing-p17-narrow`: `git fetch origin && git merge origin/main` — the conflict is `docs/superpowers/specs/2026-08-22-p17-narrow-concrete-receiver-direct-design.md` (main has v7 via #183; branch has v9). Keep the BRANCH side (v9) verbatim (`git checkout --ours` on that file after confirming it is the only conflict; inspect any others individually). Then `cargo test --lib` smoke (~1 min), push, wait for CI (re-arm a checks monitor for #186).
2. **Merge #186** when CI green (approvals complete: Ox APPROVE at cap; terra's items resolved by owner-ratified v9 + sol∥Ox confirms + docs fix f245bdf): `gh pr merge 186 --squash --delete-branch`. Main becomes CPG 47 / sidecar 16.
3. **Rebase #184** (`~/code/slicing-p14s4-sol`): `git fetch && git rebase origin/main` — expect conflicts in `src/cpg_cache.rs` + `src/navigation/call_edge_cache.rs` version constants (+ their pin tests) and possibly the #17 spec file if touched: resolve pins to **CPG 48 / sidecar 17**, pin tests to match; run full `cargo test` (expect 3349/0/1 ± merge-skew) + `cd eval && uv run tier-a --matrix-only --allow-stale-sut` after `cargo build --release`; push (force-with-lease after rebase); CI; `gh pr merge 184 --squash --delete-branch`.
4. **Merge #185** (`~/code/slicing-p15a-ox`, no cache changes): re-check `gh pr view 185 --json mergeable` after step 3 (may need a trivial rebase); CI green → squash-merge.
5. **Roadmap docs PR**: branch from new main, run `python3 <scratchpad-or-artifacts>/scripts/roadmap-patch-p17-s4-p15a.py docs/analysis/prism-post-plan-roadmap.md` (tokens already filled: #186/#185/#184 texts; rows 14, 15, 17), commit, PR, sol ∥ Ox docs review (standing rule), CI, merge.
6. **Final owner writeup** in chat: merge-chain outcomes + the full Ox calibration verdict (14 comparison entries + 3 implementation trials; source `ox-calibration.md` in the durable dir).
7. **Memory + cleanup**: update `project_tier_c_part_d_run.md` + `MEMORY.md` (merges; #17b next candidates: population telemetry now on main; #16 next; slice-4 done); release `fable-itemP17-impl-sol-20260823`; `git -C ~/code/slicing worktree remove ~/code/slicing-p15a-timing-wt`; remove clones `slicing-p17-narrow`, `slicing-p14s4-sol`, `slicing-p15a-ox` (guard: dirty=0/unpushed=0 re-probe) and optionally `slicing-p14s4-ox` (parked calibration branch is pushed) + `slicing-p14-nested-module-identity` (spec merged via #182); KEEP `slicing-p4b-go-dot-import` (owner).

**STOP conditions:** any CI failure on a merge candidate; any conflict outside the named files in steps 1/3; `gh pr merge` refusing for a reason other than CI-pending; any re-verification totals differing from §2's evidence.

## 2. State ledger

| Item | State | Evidence / correction |
|---|---|---|
| #17-narrow (PR #186, head f245bdf, base 514cfe3) | next (conflict→merge) | `[MEASURED]` p17e: 3350/0/1; tier-a 104; oracle caddy/prometheus/etcd TRUE, hugo = 1 documented env-gap waiver (tocss.go:122, `//go:build extended`); direct-lane audit 199/0/1 (p17d AND p17e, seed 23, 50/corpus); R3 legacy byte-parity per site; cache 47/16; design v9 (ca26ba9) + docs fix (f245bdf); reviews: r1–r3 + v9 confirms per PR body (`p17e/pr-body-P17.md` in durable dir) |
| #14 slice 4 (PR #184, head 38b85f2) | pending (merge after #186; rebase pins→48/17) | `[MEASURED]` s4sol5: 3349/0/1; tier-a 104; gates TRUE 0/7/33/0; terra r3 APPROVE-after-fix + final confirm APPROVE; CI green on 38b85f2; mergeable=MERGEABLE (pre-#186) |
| #15(a) (PR #185, head f286805) | pending (merge last) | `[MEASURED]` gate: 3296/0/1 (=main+tests); tier-a 104; 5 corpora + 4 manifests RAW-BYTE identical; quiet timing etcd −42% / prometheus −34%; CI green |
| Ox slice-4 calibration branch (66e70e0, pushed) | parked | `[MEASURED]` cap round open-class (terra 2W / sol 6W compositions); oracle site-for-site = sol's at its fix-1 head; record in `ox-calibration.md` |
| Ledger + calibration log | done (living) | `[MEASURED]` `.superpowers/sdd/progress.md`; `ox-calibration.md` (scratchpad + durable dir) |
| Durable artifacts | done | `[MEASURED]` `~/code/prism-lane-artifacts/2026-08-23-p14s3-p17/` 304 MB (oracle baselines incl. `oracle-s3b-etcd.json` = #17 acceptance baseline; ctrl514/ctrl8444; scripts incl. `verify-P17.sh`, `direct_lane_audit.py`, `roadmap-patch-p17-s4-p15a.py`; briefs; reviews; PR bodies) |

## 3. Corrections to standing documents and memory

| Location | Stale or false assertion | Correction |
|---|---|---|
| `MEMORY.md` + `project_tier_c_part_d_run.md` | "#17 impl IN FLIGHT (sol)"; slice 4 unmerged | `[MEASURED]` #17 = PR #186 (conflict→merge); slice 4 = PR #184 merge-ready; #15(a) = PR #185 — NOT yet corrected; §1 step 7 is the work item |
| `docs/analysis/prism-post-plan-roadmap.md` rows 14/15/17 | slice 4 "next"; #15 open; #17 design-of-record only | Patch script staged (`roadmap-patch-p17-s4-p15a.py`, tokens filled) — apply in §1 step 5 |

## 4. Open work

| # | Work | State | Exact next action | Blocked by | Identifiers |
|---:|---|---|---|---|---|
| 1 | #186 conflict + merge | next | §1 steps 1–2 | — | PR 186 (head 97b1e88); clone `slicing-p17-narrow`; monitor ba02hvr8q (polls the PR — still valid) |
| 2 | #184 rebase + merge | pending | §1 step 3 | #186 merged | PR 184; pins 48/17 |
| 3 | #185 merge | pending | §1 step 4 | #184 merged | PR 185 |
| 4 | Roadmap docs PR | pending | §1 step 5 | merges | patch script in durable `scripts/` |
| 5 | Owner writeup (Ox calibration) | pending | §1 step 6 | merges (for final numbers) | `ox-calibration.md` |
| 6 | Memory + releases + clone cleanup | pending | §1 step 7 | merges | ctx `fable-itemP17-impl-sol-20260823`; worktree `slicing-p15a-timing-wt` |
| 7 | Roadmap follow-ups (recorded, not started) | parked | in the docs PR text (already in patch) | — | #17b (telemetry live on main after #186); #16; qualified embedded interfaces; wave-5 externality under partially-proven module graphs; oracle tag-set coverage |

## 5. Invariants and traps — do not do these
- Never re-run `verify-*` against stale controls: controls are `ctrl514-*` (main@514cfe3, resolution ≡ 18b585a); after #186 merges any NEW same-base comparison needs freshly generated controls from new main.
- Never let a bridge implementer push — controller pushes after every wave (`git log @{u}..HEAD` check; slice-3 sat 10 commits single-copy once).
- Ox/opencode `build` turns end at checkpoints and die on >~1-min commands → continuation loop (same context, "commit + STATUS before stopping"); a 103-byte `.out` with `network_error` in `.err` = no turn ran, just retry; a hung session (idleAgeMs ≫, window ~0) → `session cancel`, keep the working tree, fresh context.
- `session inject` lands at the NEXT turn start, never mid-turn.
- No backticks inside double-quoted shell echo strings (command substitution ate ledger lines twice); `git cherry-pick` has no `-q`.
- Cache versions: exactly ONE transition per PR (main 46/15 → #186 47/16 → #184 48/17). Never per-wave.
- The oracle waiver list is exactly ONE site (hugo tocss.go:122, env gap); anything else newly blocking is a real finding.
- Severity discipline: probe before accepting a reviewer WRONG (three were downgraded this session by controller probes: qualified-embed relabel; #15(a) scoped-branch coverage; terra W1s).

## 6. Identifiers

| Item | Verbatim |
|---|---|
| PRs | `#186` (#17, 97b1e88) · `#184` (slice 4, 38b85f2) · `#185` (#15a, f286805) |
| main | `18b585a` (docs) ≡ resolution `514cfe3` |
| Clones | `~/code/slicing-p17-narrow` · `~/code/slicing-p14s4-sol` · `~/code/slicing-p14s4-ox` (parked) · `~/code/slicing-p15a-ox` · keep `~/code/slicing-p4b-go-dot-import` |
| Worktree to remove | `~/code/slicing-p15a-timing-wt` |
| Durable dir | `~/code/prism-lane-artifacts/2026-08-23-p14s3-p17/` |
| Scratchpad | `/private/tmp/claude-501/-Users-wesleyjinks-code-slicing/a3bf14f1-6b47-464b-ba09-fc62e2ad7efb/scratchpad` (session-scoped; durable copies exist) |
| Open bridge ctx | `fable-itemP17-impl-sol-20260823` (release at step 7) |
| Bridge client | derive live: `lsof -nP -t -iTCP:18080 -sTCP:LISTEN` → `lsof -a -p <pid> -d txt` → `.../a2a-bridge`; dispatch-ox: durable `scripts/dispatch-ox.sh` |
| #17 acceptance baseline | durable `oracle-baselines/oracle-s3b-etcd.json` |
| Verify scripts | durable `scripts/verify-P17.sh` (P17_CLONE env + label) · `scripts/direct_lane_audit.py` (run from a clone's `eval/`) |

## 7. Refutation verdict and owner questions

**§2c verdict:** SURVIVED · claim: "PR #186's branch is fully verified (suite/tier-a/oracle/audit) and only the spec-file merge conflict with main separates it from merge-ready" · pass: SELF-PASS (NOT INDEPENDENT) · evidence tier: TEST-BACKED · record: p17e logs + `gh pr view 186` probe (15:38, ledger)

**Questions the owner owes an answer to:** None. (All standing decisions recorded in §0(d); the merge chain is fully authorized.)

## RESOLUTION (2026-08-23 17:10 — this session)
Chain COMPLETE. #186 merged d422b0f; #184 merged 097a01a (post-#186 3-way merge 0028af1 controller-resolved: alias_resolver threaded through record_interface_type/extract_type_alias; pin-file mis-resolution via whole-file --ours caught by lib-test E0063 and redone per-hunk — see ledger lesson; re-verified 3409/0/1, tier-a 104/104, oracle vs p17e ALL TRUE); #185 merged 4e0c60f; roadmap #187 merged d8f992c (owner; docs review waived). Main = CPG 48 / sidecar 17. Bridge ctx released; timing worktree removed; lane clones removed (kept slicing-p4b-go-dot-import); memory updated. Combined-main battery (suite + tier-a + #185 byte-identity vs s4m) launched at scratchpad mainc-battery.log — SETTLED GREEN: 3415/0/1, tier-a 104/104, etcd+prometheus BYTE-IDENTICAL vs s4m (#185 preservation holds in combination). Combined main fully verified. This handoff is a local+durable record (not committed to main).
