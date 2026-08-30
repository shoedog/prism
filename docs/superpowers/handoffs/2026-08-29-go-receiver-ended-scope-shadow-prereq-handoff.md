# Handoff — Go receiver ended-scope shadow prerequisite

**Written:** 2026-08-30T22:48:31Z · **By:** Codex `/root` · **Provider:** codex
**Workspace:** `/Users/wesleyjinks/code/slicing-a-receiver-provenance-scope-prereq` · `a-receiver-provenance-scope-aware-shadow-prereq` · **Measured state:** `[MEASURED]` HEAD `45566ee` · Tree CLEAN before this authorization refresh · Probe `git status --short --branch; git log -4 --oneline` · Output captured in the authorization turn
**Predecessor:** Codex `/root` continuation of the Slice 3 review STOP at `a9ff284ab7fbb67516324841c5da3a156c2c8d0b`
**Truth ordering:** measured live state > explicit owner/contract authority within its scope > this handoff for current operational state > earlier handoffs and non-authoritative summaries. A conflict between tiers stays OPEN in §0 — never resolved by document class alone.
**Provenance:** written live by the worker. `[MEASURED]` claims were probed by this writer; `[INHERITED]` claims were not.

## 0. Gating facts — settle these before starting anything below

**(a) Lane ownership** — `[MEASURED]` `git worktree list --porcelain` showed this newly created dedicated branch/worktree and no prior owner; no subagent was dispatched — **RESOLVED 2026-08-29 by owner-authorized lane creation**
**(b) Custody exposure** — `[MEASURED]` plan/initial handoff are committed at `85099b1`, planning reconciliation at `5c57fa0`, compiled REDs at `f379e50`, RED reconciliation at `e992c4c`, bounded implementation at `bcd8a0b`, implementation reconciliation at `dda67af`, full-suite/Clippy reconciliation at `ed8841e`, and minimized corpus RED/handoff at `c52c233`; Slice 3 remains clean at `a9ff284` — **RESOLVED 2026-08-30 by corpus-RED checkpoint**
**(c) In flight / irreversible** — `[MEASURED]` no build, test, editor, or remote mutation is in flight — **RESOLVED 2026-08-29**
**(d) Authorization exercised** — owner verbatim: `authorized`, first for the scope-aware receiver prerequisite and again for the bounded `src/resolution.rs` owner-aware partition follow-on. Earlier standing authorization also permits push/merge after gates are green.
**(e) Production authority boundary** — `[MEASURED]` fresh exact base retains the Prometheus line-302 `schema` edge while the AST candidate loses it after owner recovery; the minimized candidate RED reproduces the owner-aware routing defect. Production scope now permits `src/ast.rs` plus the bounded `src/resolution.rs` follow-on and no other semantic surface — **RESOLVED 2026-08-30 by owner authorization**

## 1. Resume order

1. From `/Users/wesleyjinks/code/slicing-a-receiver-provenance-scope-prereq`, require base ancestor `7fc719ae21ba130c554c318c3f8306093a804c92`, HEAD at or after `ed8841e`, and read `docs/superpowers/plans/2026-08-29-go-receiver-ended-scope-shadow-prereq.md`.
2. Preserve RED checkpoint `f379e50` and implementation checkpoint `bcd8a0b`; do not fold the parked assignment/reuse/field-alias gaps into this lane.
3. Preserve full-suite `3,489/0/1`, same-environment Clippy results, release/Tier-A artifacts, and the fresh exact-base/candidate Prometheus manifests.
4. Keep the authorized routing repair in `src/resolution.rs` plus focused tests: an unrelated build-tag collision must not erase an independent exact satisfier, while the conflicting owner itself remains excluded and a conflict-only population still drops.
5. Rerun all gates and the declared two-round review with `WRONG` before `SMELL`; only then publish/merge, rebase Slice 3, and reverify it.

**STOP conditions:** production scope expands beyond `src/ast.rs` and the authorized `src/resolution.rs` follow-on; assignment/reuse/field-alias routing changes; an in-scope declaration becomes invisible; the Prometheus line-302 sound edge is not restored; a probe fails for its own reason; or candidate attribution lacks a same-environment exact-base control.

## 2. State ledger

| Item | State | Evidence / correction |
|---|---|---|
| Exact base/lane | done | `[MEASURED]` dedicated branch/worktree created at Slice 2 merge `7fc719ae`; Slice 3 is preserved separately at `a9ff284`. |
| Failure mechanism | done | `[MEASURED]` ordinary Go binding scan counts prior declarations by byte position; its scope filter is enabled only with the special same-scope-reuse mode. Prometheus parameter `b unsafeLabelAdder` is poisoned by inner-loop `b := ...` whose block ends before `b.Add(...)`. |
| Population census | done | `[MEASURED]` removed sound rows were enumerated. The prerequisite addresses only the supported typed-parameter false shadow; explicit existing contracts park non-direct reuse, assignment retyping, and local field-alias gaps. |
| Plan/handoff | done | `[MEASURED]` plan and initial handoff committed at `85099b1`; this refresh reconciles that checkpoint. |
| RED | done | `[MEASURED]` registered names enumerated after discarding a zero-selection probe. Exact-base positive run selected three and failed `0/3`, each with recovered `I`, owner `None`, and false shadow; exact active-shadow control passed `1/1`. Checkpoint `f379e50`. |
| Production | done | `[MEASURED]` checkpoint `bcd8a0b`: `src/ast.rs` splits declaration visibility from reuse enablement and filters Go `:=`/`var`; the obsolete sibling-scope expectation now requires exact closure-parameter owner. |
| Verification/review | blocked | Focused owner suite `9/9`, fix3 controls `4/4`, function-literal positive `1/1`; full suite has 28 summaries totaling `3,489 passed / 0 failed / 1 ignored`, exit `0`; candidate/exact-base Clippy each exit `0` with 243 diagnostic headers. Release build and Tier-A matrix were green; Tier-A quick was behaviorally clean but invalid only for expected corpus-SHA drift. Fresh Prometheus control proves a candidate regression: base line 302 fanout `1` to `schema/labels.go`, candidate fanout `0`. Minimized candidate RED selected `1`, failed `0/1`. |
| Publication | pending | Push/PR/merge after green gates; then Slice 3 rebase/reverification. |

### Hypothesis-probe-result log

| Hypothesis | Expected if true | Falsifier / alternative | Result |
|---|---|---|---|
| H1: line 302 is poisoned by an ended inner scope. | Source shows typed parameter, inner same-name declaration ends before call; ordinary walk lacks an always-on Go scope filter. | Call is inside the inner block or the ordinary path filters to call-containing scopes. | `[MEASURED]` true: source lines 184/198-204/302 and `src/ast.rs` control flow confirm the mechanism. |
| H2: all removed sound rows require the same bounded repair. | Other rows are also supported typed parameters poisoned only by non-enclosing declarations. | Existing tests/specs explicitly park different producer forms. | `[MEASURED]` false: same-scope interface reuse, `=` assignment, and field aliases are separate parked mechanisms. Scope remains the ended-declaration prerequisite. |
| H3: the closed ended-scope population is `{}` `:=`, `if` initializer `:=`, and nested `var`. | Independent tests fail on the same exact owner/shadow fields while the active inner shadow stays negative. | Any positive passes on base, compilation/selection fails, or active shadow loses its negative. | `[MEASURED]` supported: after discarding incomplete and zero-selection probes, the registered three-test run was `0/3`; the one-test active-shadow control was `1/1`. |
| H4: the existing unrelated-sibling fixture is an obsolete instance of the same defect, not an intentionally unsupported closure parameter. | Independent tests already require in-repo function-literal parameters to recover; corrected expectation fails on exact base and passes on candidate. | Closure parameters are consistently parked or candidate lacks an exact owner. | `[MEASURED]` supported: fix5/qualified-fix2 contain positive contracts; fresh exact base selected one and failed with no type/owner; candidate carries exact owner `p.C`, interface-dispatches to both structural implementers, and emits the manifest row. |
| H5: the bounded scope split has no suite-wide or Clippy regression. | Full no-fail-fast suite is green; candidate/base Clippy warning class/count match. | Any full-suite failure, candidate-only diagnostic, or changed-region warning absent on base. | `[MEASURED]` supported so far: persistent PTY run completed 28 targets at `3,489/0/1`, exit `0`; candidate/base Clippy both exit `0` with 243 headers, including the same pre-existing `walk_receiver_bindings` arity warning (`12/7` base, `13/7` candidate). |
| H6: the candidate's lost Prometheus edge is caused by the AST change, not corpus/binary drift. | Fresh exact base on the same pinned corpus retains the edge; provenance-bound candidate loses it. | Fresh base also loses the edge, or corpus/binary identities differ. | `[MEASURED]` supported: Prometheus HEAD `505095b`; fresh base release SHA `6e5c1a40...` produced line-302 fanout `1` to `schema/labels.go`; candidate release SHA `e80dea82...` produced fanout `0`. |
| H7: an unrelated build-tag owner collision globally aborts owner-aware structural satisfaction. | A minimal fixture with exact `schema.Good`, colliding tagged `labels.ScratchBuilder` declarations, and an ended receiver shadow recovers owner `api.Adder` but resolves no edge. | The fixture retains `schema.Good`, or removing owner recovery still loses the Prometheus edge. | `[MEASURED]` supported: corrected selector ran exactly one candidate test; owner was exact and shadow false, but resolved files were `{}` instead of `{schema/good.go}`. The initial zero-selection command is discarded as inadmissible. |

## 3. Corrections to standing documents and memory

| Location | Stale or false assertion | Correction |
|---|---|---|
| Slice 3 plan/handoff on branch `a-receiver-provenance-slice3-terminal-owner-predicate` | All supported valid positives are owner-bearing, so every remaining ownerless recovered Go site is terminally unproven. | `[MEASURED]` pinned Prometheus line 302 is a supported typed-parameter positive made ownerless by an ended inner-block declaration. The Slice 3 handoff was corrected at `b3a51f6` and reconciled at `a9ff284`. |
| Memory Task 4 candidate summary | Candidate verification passed without recording the later review STOP. | `[MEASURED]` aggregate gates passed, but review found the line-302 WRONG. Memory is not being edited because the owner did not request a memory update; this handoff is the current operational correction. |

## 4. Open work

| # | Work | State | Exact next action | Blocked by | Identifiers |
|---:|---|---|---|---|---|
| 1 | Planning custody | done | Preserve checkpoint `85099b1`. | None | base `7fc719ae` |
| 2 | RED contract | done | Preserve `f379e50` and the `0/3` plus `1/1` outputs. | None | `receiver_owner_carrying_test.rs` |
| 3 | Bounded repair | done | Preserve checkpoint `bcd8a0b`. | None | `src/ast.rs` |
| 4 | Owner-aware dispatch collision repair | in progress | Partition conflicting concrete owners in `src/resolution.rs`; preserve the exact satisfier and exclude only the conflicting owner; add conflict-only resolver/manifest negative. | None | Prometheus `decoder.go:302` |
| 5 | Verification/review | pending | After an authorized repair, rerun full suite, Clippy control as needed, release/Tier-A, Prometheus corpus, and two-round review. | Work item 4 | review cap `2` |
| 6 | Publication and Slice 3 continuation | pending | Merge prerequisite, rebase Slice 3, rerun its gates. | Review approval | Slice 3 `a9ff284` |

## 5. Invariants and traps — do not do these

- Never enable same-scope-reuse handling merely by enabling declaration visibility — one boolean currently conflates those behaviors.
- Never filter Go `=` assignments solely because their statement block ended — an assignment can update an outer binding; this slice changes declarations only.
- Never treat every removed sound/recall row as this defect — non-direct reuse, assignment retyping, and field aliases have distinct parked contracts.
- Never accept an edge-only GREEN — exact owner population is the prerequisite that keeps Slice 3's terminal predicate safe.
- Never recover the Prometheus edge by suppressing its exact owner or falling back to the bare-name table. The acceptable repair retains owner `prompb/io/prometheus/client::io_prometheus_client::unsafeLabelAdder` and partitions conflicting implementers without erasing independent exact satisfiers.
- A broad template search timed out without an exit status → discard it; the installed template was re-read from `/Users/wesleyjinks/.codex/handoff-template.md` via a narrow search.
- Long `functions.wait` captures terminated Cargo without final status → use a persistent PTY session and poll its session id; only the PTY run with exit `0` is admissible.

## 6. Identifiers

| Item | Verbatim |
|---|---|
| Base | `7fc719ae21ba130c554c318c3f8306093a804c92` |
| Branch | `a-receiver-provenance-scope-aware-shadow-prereq` |
| Worktree | `/Users/wesleyjinks/code/slicing-a-receiver-provenance-scope-prereq` |
| Plan | `docs/superpowers/plans/2026-08-29-go-receiver-ended-scope-shadow-prereq.md` |
| Handoff | `docs/superpowers/handoffs/2026-08-29-go-receiver-ended-scope-shadow-prereq-handoff.md` |
| Planning checkpoint | `85099b1` |
| Planning reconciliation | `5c57fa0` |
| RED checkpoint | `f379e50` |
| RED reconciliation | `e992c4c` |
| Implementation checkpoint | `bcd8a0b` |
| Implementation reconciliation | `dda67af` |
| Full-suite/Clippy reconciliation | `ed8841e` |
| Corpus-RED checkpoint | `c52c233` |
| Exact-base sibling RED worktree | `/private/tmp/slicing-scope-prereq-base-7fc719ae` |
| Candidate Clippy log | `/private/tmp/slicing-scope-prereq-clippy-candidate.log` |
| Exact-base Clippy log | `/private/tmp/slicing-scope-prereq-clippy-base.log` |
| Slice 3 branch | `a-receiver-provenance-slice3-terminal-owner-predicate` |
| Slice 3 worktree | `/Users/wesleyjinks/code/slicing-a-receiver-provenance-s3` |
| Slice 3 preserved HEAD | `a9ff284ab7fbb67516324841c5da3a156c2c8d0b` |
| Corpus evidence | `/private/tmp/slicing-s3-receiver-oracle.UU5Ufy` |
| Prometheus pin | `505095b64b43dd76baf08839e1800a8d473c97e0` |
| Fresh exact-base Prometheus manifest | `/private/tmp/slicing-scope-prereq-prometheus-base-fresh.json` · SHA-256 `24374a126c1abe5c390ad7a8be8e06b29c26654906db372afb55d2274f5270ce` |
| Candidate Prometheus manifest | `/private/tmp/slicing-scope-prereq-prometheus-manifest.json` · SHA-256 `21d08780560d43bcc482a463d39ba3c3e9e7da761cbd3267be137ebc6c6ab681` |
| Review cap | `2` |

## 7. Refutation verdict and owner questions

**§2c verdict:** NOT RUN — initial planning checkpoint precedes the compiled RED and independent review · claim: "Filtering Go declarations to scopes that contain the call restores supported typed-parameter owners without changing active shadows or parked producer forms" · pass: SELF-PASS (NOT INDEPENDENT) · evidence tier: STATIC-ONLY · record: this handoff's hypothesis log

**Questions the owner owes an answer to:** None.
