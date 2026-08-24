# Handoff — next 10 increments (post merge-chain: #186/#184/#185/#187)

**Written:** 2026-08-23T17:05:45-06:00 · **By:** Claude Fable 5 session_01XLS1m6xaPSo7sZu2F8FtA2 (controller) · **Provider:** claude
**Workspace:** ~/code/slicing · main · **Measured state:** `[MEASURED]` HEAD d8f992c (= #187) · 3 untracked/dirty entries (`.superpowers/`, eval snapshot, uncommitted handoffs — see §0b) · Probe `git status --short` this write
**Predecessor:** `2026-08-23-p17-s4-p15a-merge-chain-handoff.md` (RESOLVED — chain complete, combined main verified green: 3415/0/1, tier-a 104/104, etcd+prometheus byte-identical vs s4m)
**Truth ordering:** measured live state > explicit owner/contract authority within its scope > this handoff > earlier handoffs and summaries. Conflicts stay OPEN in §0.
**Provenance:** written live by the worker; all `[MEASURED]` claims probed this session.

## 0. Gating facts

**(a) Base state** — `[MEASURED]` main = d8f992c, CPG 48 / sidecar 17; suite 3415/0/1; tier-a 104/104; #185 byte-identity holds in combination (mainc-battery.log, durable dir). No open verification items. No live monitors, background tasks, or bridge contexts.
**(b) Custody** — durable dir `~/code/prism-lane-artifacts/2026-08-23-p14s3-p17/` holds oracle baselines (incl. `oracle-p17e-*.json` = the CURRENT delta baselines — post-#17 main ≈ p17e content, but note s4m/mainc outputs = post-#184 main, the freshest controls), ctrl/site dumps, scripts (`verify-P17.sh`, `verify-S4M.sh`, `direct_lane_audit.py`, `leafdiff.py`), ledger copy, ox-calibration.md. UNCOMMITTED in-repo: this handoff + the merge-chain handoff + `.superpowers/sdd/progress.md` (ledger; untracked single-copy, durable copy exists) — increment 9 decides their custody.
**(c) Standing rules (owner)** — merge on reviewer approval + CI green → squash-merge, report each; every spec review sol ∥ Ox, every code review terra ∥ Ox, compare + log (`ox-calibration.md`); sol/terra gate; Ox = standing reviewer + bounded implementer (byte-identity-gated / strong-reviewer loop), never sole implementer for precision routing; controller pushes after every implementer wave; declared review caps, disclosed extensions, park open-class; bridge impl via dedicated `~/code` clones (`--mode agent` codex / `build` opencode); kept clone: `slicing-p4b-go-dot-import` (feeds increment 7).
**(d) Fresh controls before ANY resolution-touching increment** — regenerate 5-corpus `call-stats --no-cache` + 4 manifests on main@d8f992c as the same-base controls (s4m/mainc files cover etcd+prometheus; caddy/hugo/ripgrep controls are pre-#184). Oracle delta baselines: run `dispatch_oracle.py` on the fresh main manifests first to cut `oracle-mainD-*.json`; gate every increment's delta against those, not p17e.

## 1. The next 10 increments (recommended order; 1–5 share the Go-precision harness context)

1. **#17b measurement pass** (owner: separate, measured FIRST) — size the unproven/external-receiver bare-fallback population per corpus with the shipped telemetry (`go_unproven_receiver_bare_fallback_{sites,hits,edges}` in call-stats) + a `--dump-sites` sample audit (reuse `direct_lane_audit.py` pattern: are the bare-ladder hits sound?). Deliverable: a measured brief (population, estimated false-edge rate, recall at risk) for the owner's go/no-go on terminal-R3. NO implementation in this increment. Size S.
2. **#16 package-qualified interface identity** — key the interface table by `(package_dir, clause, name)` (P10-style) instead of bare name; kills the Iterator/WriteClient/http.Handler conflations (roadmap row 16 site list); also closes #17's "qualified embedded interfaces" label-only follow-up. Gate: same-base control + oracle delta (row-16 over_approx list → 0, no recall transitions). Size M. Spec review sol ∥ Ox first (architectural: touches the dispatch table key).
3. **#18 RTA empty-live fallback gating** — fire `NonLocalConstructionFallback` only on proven candidate identities, or demote fallback-minted edges to NameOnly + `fallback_minted_edges` telemetry. Measured hazard: prometheus 131 minted / 14 over_approx (slice-3 pre-fix). Gate: oracle delta. Size S–M.
4. **R1(b) on-demand promoted routing** — consume the slice-4 promoted-selector snapshot (on main, serialized, unconsumed) in #17's R1(b) lane: route `concrete_promoted_deferred_drop` sites through profile-safe snapshot verdicts. This closes #17's last deferred lane. Gate: the four counterexample axes (qualifier/fields/methods/selector-names) as red-first fixtures + oracle delta. Size S–M. NOTE: the R1(b) comparator was judged open-class after 4 axes — implement ONLY the snapshot-verdict consult (ProfileConflict ⇒ drop stays), no new comparator logic.
5. **Oracle tag-set coverage + externality proof** — teach `dispatch_oracle.py` build-tag awareness (removes the standing hugo tocss.go:122 waiver) and add Ox's wave-5 externality proof under partially-proven module graphs as an oracle check. Harness-only. Size S.
6. **#13 sound Level-3 callbacks, Go-first (measure gate)** — owner-gated on a measured Go case: sweep corpora for typed-`func`-parameter HOF invocation populations (functional options, `Walk(fn)`, handler wrappers); if a real population measures, implement the binding-aware value resolver per row 13's spec constraints. Size M (measurement S; impl M).
7. **#4b Go dot-import resolution** — deferred-with-redesign-inputs spec (`2026-08-21-go-dot-import-resolution-deferred.md`); measured recall: 4 zap `observer.New` prism_fn sites. Clone `slicing-p4b-go-dot-import` kept for it. Redesign against the inputs, sol ∥ Ox spec review, then implement. Size S–M.
8. **#2 return-flow taint** — callee-return → caller-LHS edge construction (`x = f(user)`); P14's declared non-goal, the biggest remaining reasoning-layer gap. Needs Step-5b-class edge work + TaintConfig plumbing. Size M–L; spec first.
9. **Docs/custody/hygiene** — pipeline lesson 18 (never whole-file `--ours/--theirs` on a hunk conflict; `checkout -m` + per-hunk) into `docs/superpowers/pipeline-lessons.md`; commit both uncommitted handoffs; decide ledger custody (track `.superpowers/sdd/` or keep durable-copy discipline); cosmetic warning sweep (`build_pool` import, `plain_entry`, test unused-imports). Ride-along candidate on any of increments 2–5's PRs. Size S.
10. **Strategic fork decision brief [OWNER]** — roadmap §2: A Python/JS receiver-typing (highest per-language value; Py 54–65% / JS 92% unresolved) vs B Tier-C Part-C fork-B continuation (Part-D refuted on 11-task corpus; Part-C ROI was owner-affirmed) vs C Java breadth. Deliverable: decision brief with current per-language maturity numbers + cost estimates; owner picks. Do NOT start any fork lane without the decision. Size S (brief only).

**STOP conditions:** any increment whose same-base control shows unexplained deltas outside its own mechanism; oracle gate FALSE without a documented waiver decision; open-class findings at a declared cap (park + escalate, never extend silently).

## 2. State ledger

| Item | State | Evidence |
|---|---|---|
| Merge chain #186/#184/#185/#187 | DONE, verified | merge-chain handoff RESOLUTION; mainc-battery.log (3415/0/1, 104/104, byte-identity) |
| #14 (all 4 slices), #15(a), #17-narrow | DONE on main | roadmap rows 14/15/17 (#187) |
| #17b / #16 / #18 / R1(b)-consume / tag-set | OPEN — increments 1–5 | roadmap rows 16/17/18 + row-17 follow-ups |
| #13 / #4b / #2 | OPEN — increments 6–8 | roadmap rows 13/4b/2 |
| A2 (row 12) | PARKED (design + sol review exist) | `2026-08-21-java-ts-parameter-materialization-design-PARKED.md` |
| Rows 8 / 9 | DORMANT (gated on measured case / Python initiative) | roadmap |
| Ox calibration | CLOSED verdict logged | `ox-calibration.md` (durable); writeup 2026-08-23 |

## 3. Corrections owed to standing documents

| Location | Item |
|---|---|
| `docs/superpowers/pipeline-lessons.md` | lesson 18 (checkout -m rule) not yet committed — increment 9 |
| repo | both 2026-08-23 handoffs uncommitted — increment 9 |
