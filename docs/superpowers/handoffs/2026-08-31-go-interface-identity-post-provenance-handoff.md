# Handoff — #16 Go interface identity after receiver provenance

**Refreshed:** 2026-08-31T02:36:21Z · **By:** Codex `/root` · **Provider:** codex
**Workspace:** `/Users/wesleyjinks/code/slicing-16-post-provenance` · `a-go-interface-identity-post-provenance` · **Base:** `[MEASURED]` `b7a5cf934a44060de98588837b3c8c75ddffdc37` (`origin/main`, PR #214 merge)
**Predecessor:** receiver-provenance Slice 3 PR #213 / `31250f7e`, closeout PR #214 / `b7a5cf93`, and preserved #16 PARK commit `ea74558f`
**Truth ordering:** current measured state > explicit owner/design authority within scope > v16 design > this handoff > earlier handoffs and summaries. Conflicts remain open until remeasured.

## 0. Gating facts

**Lane ownership:** `[MEASURED]` fresh dedicated branch/worktree created from PR #214 merge; no subagent dispatched. The primary `slicing` worktree and every receiver-provenance worktree remain untouched.
**Custody exposure:** `[MEASURED]` v12, v13, and PARK were replayed as separate commits `e87f9e7`, `0a1c474`, and `ab4e5da`; reviewed v14 is `c64ccd42cde9074e5903cb451d4059423d71e0fa`; the parked Task 1 consult/census is `b529d295`; the owner-authorized v15 and corrected v16 designs are `e13cfaf` and `ca44422`; the public RED checkpoint is `c017395`; the production switch is `4f74ace`; the fully verified cache/review checkpoint is `cf5def8`; and PR #215 merged as `d21c8cb4ae4f4a2a6b43ce66ea4ed5a76f8a15a9`. The exact-base worktree `/private/tmp/slicing-p16-base-b7a5cf93` is intentionally dirty only with replayable test/pin overlays. The historical `/Users/wesleyjinks/code/slicing-16c1-sol` clone remains dirty and untouched on `c1-bare-walk-consult` at `1900682c`, base `0ca571c5`, with three modified source files (`627 insertions / 130 deletions`) plus one untracked spec whose SHA-256 is `e45a41e3edbdc00f2d48700601dacfabb8027870275222ac681d65be73a86c3c`.
**In flight / irreversible:** none.
**Authority:** owner repeatedly said `authorized`, explicitly authorized push/merge, directed successor continuation, and on 2026-08-31 explicitly authorized the constructible-source-fixture plus exact-base RED substitute for the zero natural-corpus population. Task 4 and both implementation review rounds passed; all relevant non-coverage CI passed; PR #215 is merged. Coverage was not awaited by owner direction.
**Template limitation:** the referenced `bootstrap/handoff-template.md` was not found under readable steering, Codex, Claude, or code roots. This handoff uses the repository's established eight-section shape instead.

## 1. Resume order

1. Require base ancestor `b7a5cf934a44060de98588837b3c8c75ddffdc37` and read v16 plus `docs/superpowers/plans/2026-08-31-go-interface-identity-post-provenance.md`.
2. Preserve replayed commits `e87f9e7`/`0a1c474`/`ab4e5da`; do not restart or flatten the reviewed artifact.
3. Preserve the dirty old clone without writes; it is historical census custody, not an implementation base.
4. Preserve v14/plan/handoff checkpoint `c64ccd42` and Task 1 parked checkpoint `b529d29595bac1c7040be94992bf01bcd320eb97`.
5. Read `docs/superpowers/handoffs/2026-08-31-go-interface-identity-task1-census-evidence.md` before acting. Preserve its measured zero population; do not present it as positive behavior coverage.
6. Read `docs/superpowers/handoffs/2026-08-31-go-interface-identity-task2-red-evidence.md` and `docs/superpowers/handoffs/2026-08-31-go-interface-identity-task4-evidence.md`. This lane is closed at PR #215 / `d21c8cb4`; resume from the successor roadmap, not this implementation branch.

**STOP conditions:** ownerless rows invoke the candidate consult; receiver text or a global index selects identity; a third consumer appears; production scope exceeds the plan; static table retirement or CPG movement becomes necessary; corpus coverage is incomplete; a probe selects zero tests or fails for its own reason; or review reopens the proxy-for-provenance class.

## 2. State ledger

| Item | State | Evidence / next action |
|---|---|---|
| Slice 3 prerequisite | done | `[MEASURED]` PR #213 merged as `31250f7e`; PR #214 reconciled custody as `b7a5cf93`. |
| Old #16 artifact | preserved, not reusable as-is | `[MEASURED]` v13 PARK required A first; dirty old clone is eight days and multiple cache/schema waves behind current main. |
| v13 replay | done | `[MEASURED]` spec-only commits replayed cleanly at `e87f9e7`, `0a1c474`, `ab4e5da`. |
| Current consumer census | done for structure | `[MEASURED]` shared terminal predicate precedes resolver/manifest; exactly two independent consumers; sidecar reuses resolver. The surviving owner-bearing `Unproven` seam now uses the exact-owner consult and cannot retry the bare table. |
| v14 design | reviewed at cap with bounded preflight correction | Round 1: one `WRONG`, zero `SMELL` (ownerless census tautology), fixed. Round 2: one `WRONG`, zero `SMELL` (`arg_count` type), fixed. Disclosed scoped confirmation found no further item. Source-reality preflight then found one closed artifact `WRONG`: the combined census partition omitted owner-bearing routes that exit before the legacy arm. The complete route tree was enumerated and the contract now uses two exhaustive ledgers; no code had started. |
| Executable plan | committed | `[MEASURED]` v14, plan, and initial handoff checkpoint is `c64ccd42`; start Task 1. |
| Shared consult and dormant census | implemented, not production-routed | `[MEASURED]` checkpoint `b529d295`; focused census tests `5/5`; existing promoted/S4 compatibility `49/49`; all-target compile passed. Normal ripgrep manifest is byte-identical to exact base. |
| First census attempt | inadmissible, retained | Manifest-local `legacy_bare` was a wrong proxy for the resolver attempt set. A constructible test proved the resolver can hit legacy while the manifest says `unproven_drop`. Artifacts remain at `/private/tmp/p16-census-task1-20260831`. |
| Corrected current-base census | complete attempt set, zero natural population | `[MEASURED]` resolver telemetry admitted candidates. Five corpora: `8,871` prerequisite (`46` ownerless, `8,825` owner-bearing), `0` candidates. Independent whole-graph call-stats also reported `0/0/0` legacy sites/hits/edges on all four Go corpora. Retained as mandatory population/control evidence under v16. |
| Source-reachable replacement probe | admissible positive population | `[MEASURED]` valid Go fixture `/private/tmp/p16-source-fixture-v1`: exact base legacy `1/1/1`, incorrect `decoy.Wrong.Run`; candidate consult `invalid_drop`, terminal `app.worker`. |
| Public RED matrix | complete | `[MEASURED]` Go public fixtures candidate `5/5`; exact base `1/5`, with all four expected behavior selectors failing on `decoy.Wrong.Run`. Sidecar candidate `1/1`, base `0/1`; pin candidate `1/1`, base `0/1`. Both one- and two-callback fixtures compile as Go. |
| Production switch | complete | `[MEASURED]` resolver and manifest share `go_proven_interface_outcome`; sidecar remains resolver-derived; CPG stays `54`, sidecar is `23`. Production checkpoint `4f74ace`; verified cache/review checkpoint `cf5def8`. |
| Verification / review | complete | `[MEASURED]` focused `15/15`; full suite `3,510/0/1`, exit `0`; ordinary Clippy passes and strict debt exactly matches base; matrix `104`, controlled quick invalid; ten corpus pairs byte-identical; four oracle deltas pass at `1.0/1.0`; #17b `0/0/0`; round 2 closed at zero open findings. |
| Publication | merged | `[MEASURED]` PR #215 merged at `d21c8cb4`; Format, Clippy, Test Suite, and Language Coverage Matrix passed. Coverage was in progress and intentionally not awaited. |

## 3. Hypothesis-probe-result log

| Hypothesis | Expected if true | Falsifier / alternative | Result |
|---|---|---|---|
| H1: A removed the old identity-establishment open class from #16's input. | Every recovered Go site without `receiver_owner_identity` drops before either consumer. | Any ownerless recovered Go site reaches `iface_key`/`interface_impls`. | `[MEASURED]` supported: the prerequisite ledger found 46 ownerless terminal rows and zero candidate rows; the ownerless no-invocation unit test passed. |
| H2: a #16 defect remains reachable after A. | An owner-bearing `Unproven` route can still reach the bare table after earlier screens miss. | Every constructible source state exits before legacy; a zero declared-corpus population alone does not falsify structural reachability under v16. | `[MEASURED]` supported: valid return-typed concrete `app.S` with a func field and caller-local `type S` reached exact-base legacy `1/1/1`, which minted unrelated `decoy.Wrong.Run` instead of registered `app.worker`. |
| H8: every consult route is publicly reachable at the legacy seam. | Source fixtures can make the seam return all five consult routes. | Declaration-proven interface owners exit through the earlier carried-interface path. | `[MEASURED]` falsified: interface-owner fixtures produced legacy `0/0/0`; resolver control flow exits before the seam. v16 retains five-route direct-unit coverage and narrows public REDs to source-reachable concrete-owner `invalid_drop`. |
| H3: resolver and manifest are the only independent consumers. | Resolver owns edge selection; manifest independently mirrors it; sidecar calls resolver. | A third path independently mints receiver-interface edges. | `[MEASURED]` supported by bounded reference census; LSP MCP was unavailable, so exhaustive symbol references plus compiled tests remain required. |
| H4: v13 can resume without another provenance proxy. | New consult accepts only `&GoOwnerIdentity` and has no receiver text or global fallback. | Any valid target requires re-resolving `recv_ty` or selecting an identity by name. | `[MEASURED]` supported by A's contract and v14 signature; corpus census is the behavioral discriminator. |
| H5: the existing walk is reusable but incomplete for #16. | Direct satisfiers already follow caller profiles/live selection; promoted satisfiers require the R1(b) snapshot. | Existing walk already includes promoted supply, or extraction changes existing S4 outputs. | `[MEASURED]` shared extraction and guarded promoted supply pass; five-corpus manifests and call-stats are byte-identical to exact base, and all four oracle deltas pass. |
| H6: the old clone cannot be the active lane. | It is stale and dirty with large uncommitted code against pre-A base. | It is clean, current-base, and contains the merged A contract. | `[MEASURED]` falsifier observed; fresh lane created instead. |
| H7: v14's identity rule is closed at the declared cap. | Findings are bounded and no proxy-for-provenance class remains. | A new identity proxy or fallback is required. | `[MEASURED]` supported: the two capped findings and the post-cap preflight artifact correction are closed; the latter repaired census population accounting, not identity authority. Zero `SMELL` remains. |
| H9: the source-valid concrete-owner fixture reaches the legacy seam. | Exact base reports legacy `1/1/1` and mints the decoy. | The fixture exits earlier or is invalid Go. | `[MEASURED]` supported: Go compilation exits `0`; exact base resolves `decoy.Wrong.Run`; the candidate resolves the registered app callback or drops when unregistered. |
| H12: manifest owner labels collapse same-file callback fanout. | Resolver and identities contain two callbacks while manifest fanout is one. | Registration extraction supplies only one callback or fanout is already two. | `[MEASURED]` supported, then fixed: watched candidate RED showed two identities with fanout one; terminal rows now count exact identities and pass `1/1`. |
| H13: the two-callback source and sidecar path is real. | Go accepts both packages; no-cache/cold/exact-CPG/sidecar-hit and refresh evidence retain both callbacks. | Compiler rejection or loss of one serialized or refreshed target. | `[MEASURED]` supported: Go test exits `0`; candidate sidecar and refresh `2/2`; exact base `0/2` for the legacy omission/decoy; deserialized live set contains `S`. |
| H14: the production switch conserves current corpora. | Candidate/base artifacts are byte-identical and oracle deltas have no new Exact sites. | Any target/count/hash delta, oracle blocker, or #17b population. | `[MEASURED]` supported: ten artifact pairs are byte-identical; four oracle gates pass at full coverage; #17b is `0/0/0` on every Go corpus. |

## 4. Task table

| # | Task | State | Next action | STOP owner |
|---:|---|---|---|---|
| 0 | Durable design/lane custody | done | Preserve checkpoint `c64ccd42`. | None |
| 1 | Candidate consult + census harness | complete checkpoint | Preserve corrected resolver-witness harness, zero-population evidence, and replacement-floor authority. | Census/custody drift |
| 2 | Public RED matrix | done | Preserve `c017395` and exact-base overlay evidence. | Invalid fixture or zero exact-base attempts |
| 3 | Production switch/cache | done | Preserve `4f74ace` and verified cache checkpoint `cf5def8`. | Scope expansion |
| 4 | Full verification/corpora/oracles | done | Preserve Task 4 evidence and retained `/private/tmp` artifacts. | Evidence drift |
| 5 | Review rounds 1–2 | done at cap | Zero open `WRONG`; zero open in-scope `SMELL`. | Open-class finding |
| 6 | Publication/closeout | done | PR #215 merged as `d21c8cb4`; preserve this docs-only reconciliation. | None |

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
| v14 design/plan/handoff checkpoint | `c64ccd42cde9074e5903cb451d4059423d71e0fa` |
| Task 1 parked checkpoint | `b529d29595bac1c7040be94992bf01bcd320eb97` |
| Slice 3 publication | PR #213 · `31250f7eec33391ebda9065ca5222eceffdf3cc4` |
| Slice 3 closeout | PR #214 · `b7a5cf934a44060de98588837b3c8c75ddffdc37` |
| Historical dirty clone | `/Users/wesleyjinks/code/slicing-16c1-sol` · `c1-bare-walk-consult` · `1900682c` · base `0ca571c5` |
| Historical WIP diff | `src/go_promoted_snapshot.rs`, `src/navigation/queries.rs`, `src/resolution.rs` · `627 insertions / 130 deletions` |
| Historical untracked spec SHA-256 | `e45a41e3edbdc00f2d48700601dacfabb8027870275222ac681d65be73a86c3c` |
| Task 1 evidence | `docs/superpowers/handoffs/2026-08-31-go-interface-identity-task1-census-evidence.md` |
| Task 2/3 evidence | `docs/superpowers/handoffs/2026-08-31-go-interface-identity-task2-red-evidence.md` |
| Task 4 evidence | `docs/superpowers/handoffs/2026-08-31-go-interface-identity-task4-evidence.md` |
| Public RED checkpoint | `c017395860964df02577fd021f8f18c84aade99b` |
| Production switch checkpoint | `4f74ace724ae766a285480de7211b6c4159ef425` |
| Verified implementation checkpoint | `cf5def8c840b54e151f507732f737d6bc6355f2f` |
| #16 publication | PR #215 · `d21c8cb4ae4f4a2a6b43ce66ea4ed5a76f8a15a9` |
| Exact-base RED worktree | `/private/tmp/slicing-p16-base-b7a5cf93` · detached `b7a5cf93` · intentional test overlays |
| Corrected candidate binary | `7164d872d8e49585fc892887146c85a4ae3a24c42c5a9bba55df14aa0cc8ee79` |
| Exact-base control binary | `631bedb9a3ac8904574e552b886318fffe802e1df8a3489d82cb027c3c6a48a1` |
| Corrected census artifacts | `/private/tmp/p16-census-task1-v2-20260831` |
| Exact-base cache pins | CPG `54`; sidecar `22` |
| Candidate cache pins | CPG `54`; sidecar `23` |
| Corpus pins | ripgrep `82313cf9`; Caddy `77e9ce74`; Prometheus `505095b6`; etcd `61d518f5`; Hugo `a00b5c72` |
| Review cap | design `2` plus disclosed scoped confirmation and bounded preflight artifact correction; implementation `2` |

## 7. Current evidence and exclusions

Current evidence closes Tasks 2–6: source fixtures compile as Go; exact base reports legacy `1/1/1` and wrong `decoy.Wrong.Run`; focused candidate selectors pass `15/15`; full suite passes `3,510/0/1` across `28` summaries; formatting/check/ordinary Clippy pass; strict Clippy exactly matches same-environment baseline debt; Tier-A matrix lists `104` cases and quick's sole invalid reason is controlled on exact base; all ten corpus artifact pairs are byte-identical; four oracle deltas pass with `1.0/1.0` coverage and no blockers/new Exact sites; #17b remains `0/0/0`; cache parity covers no-cache/cold/exact-CPG/sidecar-hit plus incremental refresh and a nonempty deserialized live set; round 2 closes at zero open findings. PR #215 relevant CI passed and merge commit `d21c8cb4` is confirmed. Coverage was still in progress at merge and is not claimed or required. The zero-selected probes, malformed jq projection, guessed filenames, pipeline failure, and truncated first full-suite aggregate remain excluded.

## 8. Owner direction and review verdict

Owner direction included push/merge authority after gates and explicit authorization of the fixture substitute. The current verdict is **CLOSED AND MERGED** at PR #215 / `d21c8cb4`. Both implementation review rounds converged within cap with zero open `WRONG` and zero open in-scope `SMELL`; all relevant non-coverage CI passed. Proceed from the successor roadmap.
