# Handoff — #16 Go interface identity after receiver provenance

**Written:** 2026-08-31T00:30:03Z · **By:** Codex `/root` · **Provider:** codex
**Workspace:** `/Users/wesleyjinks/code/slicing-16-post-provenance` · `a-go-interface-identity-post-provenance` · **Base:** `[MEASURED]` `b7a5cf934a44060de98588837b3c8c75ddffdc37` (`origin/main`, PR #214 merge)
**Predecessor:** receiver-provenance Slice 3 PR #213 / `31250f7e`, closeout PR #214 / `b7a5cf93`, and preserved #16 PARK commit `ea74558f`
**Truth ordering:** current measured state > explicit owner/design authority within scope > v14 design > this handoff > earlier handoffs and summaries. Conflicts remain open until remeasured.

## 0. Gating facts

**Lane ownership:** `[MEASURED]` fresh dedicated branch/worktree created from PR #214 merge; no subagent dispatched. The primary `slicing` worktree and every receiver-provenance worktree remain untouched.
**Custody exposure:** `[MEASURED]` v12, v13, and PARK were replayed as separate commits `e87f9e7`, `0a1c474`, and `ab4e5da`. v14, its plan, and this handoff are currently the only uncommitted successor work. The historical `/Users/wesleyjinks/code/slicing-16c1-sol` clone remains dirty and untouched on `c1-bare-walk-consult` at `1900682c`, base `0ca571c5`, with three modified source files (`627 insertions / 130 deletions`) plus one untracked spec whose SHA-256 is `e45a41e3edbdc00f2d48700601dacfabb8027870275222ac681d65be73a86c3c`.
**In flight / irreversible:** none.
**Authority:** owner repeatedly said `authorized`, explicitly authorized push/merge, and directed successor continuation. Production switching remains gated by v14 census, REDs, verification, and two-round review.
**Template limitation:** the referenced `bootstrap/handoff-template.md` was not found under readable steering, Codex, Claude, or code roots. This handoff uses the repository's established eight-section shape instead.

## 1. Resume order

1. Require base ancestor `b7a5cf934a44060de98588837b3c8c75ddffdc37` and read v14 plus `docs/superpowers/plans/2026-08-31-go-interface-identity-post-provenance.md`.
2. Preserve replayed commits `e87f9e7`/`0a1c474`/`ab4e5da`; do not restart or flatten the reviewed artifact.
3. Preserve the dirty old clone without writes; it is historical census custody, not an implementation base.
4. Commit the v14/plan/handoff checkpoint, then implement only Task 1's removable census harness.
5. Do not add public-behavior REDs or switch production consumers until the complete current-base census passes.

**STOP conditions:** ownerless rows invoke the candidate consult; receiver text or a global index selects identity; a third consumer appears; production scope exceeds the plan; static table retirement or CPG movement becomes necessary; corpus coverage is incomplete; a probe selects zero tests or fails for its own reason; or review reopens the proxy-for-provenance class.

## 2. State ledger

| Item | State | Evidence / next action |
|---|---|---|
| Slice 3 prerequisite | done | `[MEASURED]` PR #213 merged as `31250f7e`; PR #214 reconciled custody as `b7a5cf93`. |
| Old #16 artifact | preserved, not reusable as-is | `[MEASURED]` v13 PARK required A first; dirty old clone is eight days and multiple cache/schema waves behind current main. |
| v13 replay | done | `[MEASURED]` spec-only commits replayed cleanly at `e87f9e7`, `0a1c474`, `ab4e5da`. |
| Current consumer census | done for structure | `[MEASURED]` shared terminal predicate precedes resolver/manifest; exactly two independent consumers; sidecar reuses resolver. Owner-bearing `Unproven` routes still retain a bare table lookup. |
| v14 design | reviewed at cap | Round 1: one `WRONG`, zero `SMELL` (ownerless census tautology), fixed. Round 2: one `WRONG`, zero `SMELL` (`arg_count` type), fixed. Disclosed scoped confirmation: zero further findings. |
| Executable plan | drafted | Commit with v14 and this handoff, then start Task 1. |
| Current-base corpus census | pending | Build the dormant shared consult and `PRISM_P16_CENSUS`; do not switch production. |
| Public RED matrix | pending | Starts only after census acceptance. |
| Production / verification / review | pending | No production code changed in this lane yet. |
| Publication | authorized but gated | Push/merge only after all plan gates and relevant non-coverage CI are green. |

## 3. Hypothesis-probe-result log

| Hypothesis | Expected if true | Falsifier / alternative | Result |
|---|---|---|---|
| H1: A removed the old identity-establishment open class from #16's input. | Every recovered Go site without `receiver_owner_identity` drops before either consumer. | Any ownerless recovered Go site reaches `iface_key`/`interface_impls`. | `[MEASURED]` supported structurally by `go_receiver_owner_is_terminally_unproven` placement in resolver and manifest; Task 1 census must re-prove dynamically. |
| H2: a #16 defect remains reachable after A. | An owner-bearing `Unproven` route can still reach the bare table after earlier direct/interface screens miss. | All owner-bearing sites route through an exact-owner interface path before the bare arm. | `[MEASURED]` supported by current resolver and manifest control flow; population size remains pending census. |
| H3: resolver and manifest are the only independent consumers. | Resolver owns edge selection; manifest independently mirrors it; sidecar calls resolver. | A third path independently mints receiver-interface edges. | `[MEASURED]` supported by bounded reference census; LSP MCP was unavailable, so exhaustive symbol references plus compiled tests remain required. |
| H4: v13 can resume without another provenance proxy. | New consult accepts only `&GoOwnerIdentity` and has no receiver text or global fallback. | Any valid target requires re-resolving `recv_ty` or selecting an identity by name. | `[MEASURED]` supported by A's contract and v14 signature; corpus census is the behavioral discriminator. |
| H5: the existing walk is reusable but incomplete for #16. | Direct satisfiers already follow caller profiles/live selection; promoted satisfiers require the R1(b) snapshot. | Existing walk already includes promoted supply, or adding it changes existing S4 outputs. | `[MEASURED]` current `go_visible_s4_implementers` is direct-only; old WIP demonstrates a bounded extraction shape, but must be reimplemented against current per-owner semantics. |
| H6: the old clone cannot be the active lane. | It is stale and dirty with large uncommitted code against pre-A base. | It is clean, current-base, and contains the merged A contract. | `[MEASURED]` falsifier observed; fresh lane created instead. |
| H7: v14 review is closed at the declared cap. | Findings are bounded, non-repeating, and fixed; scoped confirmation finds no further issue. | A new open-class provenance proxy or contradictory gate appears. | `[MEASURED]` supported: two closed `WRONG` fixes, zero `SMELL`, scoped confirmation clean. |

## 4. Task table

| # | Task | State | Next action | STOP owner |
|---:|---|---|---|---|
| 0 | Durable design/lane custody | in progress | Commit v14, plan, handoff; refresh this row with checkpoint. | None |
| 1 | Candidate consult + census harness | pending | Compile signature, extract walk, add guarded promoted supply and pre-terminal census. | Open/invalid corpus transition |
| 2 | Public RED matrix | pending | Enumerate selectors and capture intended exact-base failures. | Compile discrepancy or zero selection |
| 3 | Production switch/cache | pending | Switch two consumers, retain sidecar derivation, bump only `22→23`. | Scope expansion |
| 4 | Full verification/corpora/oracles | pending | Run complete plan sequence with same-environment controls. | Unattributed failure |
| 5 | Review rounds 1–2 | pending | WRONG before SMELL; park on open class at cap. | Open-class finding |
| 6 | Publication/closeout | pending | Push/open/CI/merge, then reconcile durable state. | Relevant CI failure |

## 5. Invariants and traps

- `receiver_owner_identity` is the only identity authority. Never rebind `receiver_type` in the caller namespace.
- The terminal predicate stays before both consumers; the new consult must not accept `Option<GoOwnerIdentity>`.
- Keep one consult and two direct consumers; sidecar remains resolver-derived.
- Preserve current per-owner conflict/uncertainty behavior while extracting the walk.
- Promoted snapshot failures exclude only that owner unless no exact satisfier survives.
- Empty or arity-empty interface results may try the existing func-value-field route exactly once; never retry the bare table.
- CPG stays `54`; sidecar alone advances to `23`.
- Census names and counts from v8/v13 are historical sentinels, not current acceptance evidence.
- Do not treat generated Tier-A or oracle output as source changes or rebaseline it.

## 6. Identifiers

| Item | Verbatim |
|---|---|
| Exact base | `b7a5cf934a44060de98588837b3c8c75ddffdc37` |
| Branch | `a-go-interface-identity-post-provenance` |
| Worktree | `/Users/wesleyjinks/code/slicing-16-post-provenance` |
| Design | `docs/superpowers/specs/2026-08-23-go-package-qualified-interface-identity-design.md` |
| Plan | `docs/superpowers/plans/2026-08-31-go-interface-identity-post-provenance.md` |
| Handoff | `docs/superpowers/handoffs/2026-08-31-go-interface-identity-post-provenance-handoff.md` |
| Replayed v12 | `e87f9e7` |
| Replayed v13 | `0a1c474` |
| Replayed PARK | `ab4e5da` |
| Slice 3 publication | PR #213 · `31250f7eec33391ebda9065ca5222eceffdf3cc4` |
| Slice 3 closeout | PR #214 · `b7a5cf934a44060de98588837b3c8c75ddffdc37` |
| Historical dirty clone | `/Users/wesleyjinks/code/slicing-16c1-sol` · `c1-bare-walk-consult` · `1900682c` · base `0ca571c5` |
| Historical WIP diff | `src/go_promoted_snapshot.rs`, `src/navigation/queries.rs`, `src/resolution.rs` · `627 insertions / 130 deletions` |
| Historical untracked spec SHA-256 | `e45a41e3edbdc00f2d48700601dacfabb8027870275222ac681d65be73a86c3c` |
| Current cache pins | CPG `54`; sidecar `22` |
| Candidate cache pins | CPG `54`; sidecar `23` |
| Corpus pins | ripgrep `82313cf9`; Caddy `77e9ce74`; Prometheus `505095b6`; etcd `61d518f5`; Hugo `a00b5c72` |
| Review cap | design `2`; implementation `2` |

## 7. Current evidence and exclusions

Current evidence is structural and historical only. No v14 production behavior, RED, candidate corpus, full-suite, Tier-A, or oracle claim exists yet. Slice 3's `54/54`, `3,496/0/1`, Tier-A, and five-corpus evidence proves the prerequisite composition, not #16 acceptance. The failed v13 lookup in the stale clone and the earlier wrong-path cache probe were self-failing and discarded; their corrected current-repository probes are the only observations used here.

## 8. Owner direction and review verdict

Owner direction is to proceed through the successor and includes push/merge authority after gates. v14 design verdict: **APPROVE AFTER ROUND 2 plus one disclosed scoped confirmation** — `WRONG`: zero remaining; `SMELL`: zero. Confidence decreases if the current-base census shows owner-bearing R3 is empty or loses an admitted sound implementer; it collapses if any ownerless site invokes the consult or any correct result requires a text/index identity proxy.
