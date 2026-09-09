# Owner value checkpoint — admission blocks both real repositories

Measured merged PR298 (`0807d7de`), with freshly rebuilt release CLI and MCP.
**Do not broaden activation or prioritize portable packaging yet.** Both approved
real roots refuse before compiler acquisition, so real-receiver eligibility and
recall remain **unmeasured**, not 0%. The synthetic proof route works, at a material
per-call cost. No production source, tests, compiler-worker assets or limits changed.

## Actual public-route admission

| Root | First CLI refusal, repeated 3/3 | Eager MCP startup | Receiver measurement |
|---|---|---|---|
| Pinned installed Excalidraw copy | `input_budget` | Same refusal, no initialize response | Not reached |
| Approved private frontend checkout | `owner_requires_js_ts_only` | Same refusal, no initialize response | Not reached |

Every refused invocation has the named stderr category and empty stdout. This is
not classification from exit status alone. The private source, inventory, commit
and raw measurements stay local; only the coarse disposition is published.

A diagnostic linked to the **same fresh release library** calls the actual
`repo_loader::load_repo`, rather than approximating its census from file extensions:
Excalidraw loads **628 files / 6,928,124 bytes** (26 JavaScript, 312 TypeScript,
290 TSX; no TypeDatabase). `owned_inputs` rejects the count above **512** before
reparsing or calling the compiler. Its bytes are below **8,388,608**; the observed
`input_budget` is therefore the count gate, not the byte gate. See
[`owned_inputs`](../../../src/executable_owner.rs) and
[`replace_session_impl`](../../../src/executable_owner/integration.rs).

The private loader census contains non-JS/TS input and no TypeDatabase, establishing
the other operand of the selected-path refusal. It also exceeds the count cap;
that is a separately observed count, **not a second executed refusal**.

The previous large-repo observer and activated owner route are different contracts.
[`acquisition::reproduce`](../../../src/executable_owner/acquisition.rs) hardcodes
`profile: default`, `links: reject`; it does not adopt `installed`/`in-root`.
The unchanged public dependency tree still has **59,799 regular files,
830,105,343 bytes and 249 links**, independently incompatible with the default
inventory profile. This inventory is not the 628-file Prism loader census.
Directory count is 9,058 **excluding the root** (the earlier 9,059 included it).
These later barriers were not executed through the public owner path here.
Raising 512 alone would not establish admission. No profile override, source
pruning, link flattening, config replacement or dependency installation was tried.

The known real Program gaps, including deliberately unresolved `react-scripts`,
remain governed by the previous source-backed audits. This run did **not** reach
or remeasure that Program. No new real-site owner candidates or refusal histogram
can honestly be reported from these early acquisition failures.

## Synthetic controls: actual edges versus detached observations

All **27 committed fixture variants** were materialized unchanged and queried on
ordinary and owner CLI paths: **54 process queries**. Of 27 owner acquisitions:

| Outcome | Cases | Interpretation |
|---|---:|---|
| Acquired, new `contextual_owner` Exact edge | 2 | Candidate and different genuine input; `src/client.ts::m` |
| Acquired, ordinary Exact evidence unchanged | 18 | No new owner proof; includes two existing typed-parameter controls |
| Refused at `semantic_closure` | 6 | Duplicate method, missing module, react-scripts, required path, automatic entries, noResolve |
| Refused at `input_set` | 1 | Indexed effect outside configured Program |

The second successful fixture is a distinct input version, not an independent
real receiver. Unsupported cases do not necessarily have an empty navigation
response: the opaque-escape control preserves its ordinary `local_def` call;
both explicit controls preserve `typed_param`. None gains `contextual_owner`.

Separately, **27 detached compiler observations** explain the source predicates.
They never authorize an edge themselves. The machine receipt preserves both
acquisition outcome and first detached refusal for every fixture. In particular:

- Optional-property fixture first refuses at `direct_call`; this does not isolate
  the later optional-property check.
- Write/escape/nested fixtures first refuse at `direct_body`; this is not a claim
  that each later write barrier was exercised by this checkpoint.
- Ambient class, prototype effect and augmentation first refuse at `indexed_effect`.
- Six detached candidates survive local shape checks while actual acquisition
  refuses (five closure cases plus the input-set mismatch). Counting candidates
  as admitted edges would be wrong.
- The `skipLibCheck` missing-path control still refuses for
  `required_path_unproven`; react-scripts still refuses with type/lib and diagnostic
  reasons. Neither earlier defect/disposition was bypassed.

These are bounded fixture acceptance checks, not a complete re-execution of the
constructor's per-predicate, cross-epoch or lifecycle test suites.

## Cost: small warm-filesystem samples, not throughput claims

One three-source-file supported fixture, Node **26.0.0**, pinned TypeScript 5.9.3.
Five alternating ordinary/owner CLI pairs; one eager MCP session per mode with
five sequential detailed `nav_callees` requests. Identical requests within each
mode comparison; actual response target/provenance checked.

| Measurement | Ordinary | Owner |
|---|---:|---:|
| CLI median, process start through query | 39.04 ms | 563.73 ms |
| CLI observed range, n=5 each | 38.67–39.35 ms | 550.75–573.44 ms |
| MCP startup through initialize, n=1 each | 39.30 ms | 552.98 ms |
| MCP per-call median, n=5 each | 0.082 ms | 520.42 ms |
| MCP per-call observed range | 0.068–0.178 ms | 515.50–526.76 ms |

CLI samples include process loading, parsing, acquisition, graph construction,
query and output; they do not isolate compiler cost. MCP ordinary calls reuse the
in-memory session while owner calls reacquire by design (`OwnerRuntime::ensure_ready`).
Do not interpret the tiny denominator as a stable speed ratio or infer production
p95, RSS, CPU use, scalability, cold-disk behavior or concurrent-user capacity.
CLI uses Exact filtering; MCP uses detailed ordinary tool arguments. The two
surfaces are separately paired, not identical cross-surface timing workloads.
No full project tests or builds ran concurrently with the dedicated timing pass;
uncontrolled background machine activity remains possible.

Real Excalidraw refusal took 142–157 ms on CLI; these are **failed-admission costs**,
not successful acquisition or real navigation latency. Its single MCP refusal took
537 ms; one sample is not a timing distribution.

## Verification, custody and review

- Fresh release build: `cargo build --release --features mcp --bins`.
- All 27 fixture outcomes, six real CLI refusals, two real MCP startup refusals,
  ten paired CLI timing responses and ten MCP timing responses inspected.
- Local artifact verification: **35 tests passed, 0 failed, 0 skipped**. These
  validate captured measurements, fixture byte reconstruction, actual JSON edges,
  refusal categories, timing responses and binary/compiler/source custody.
- All **1,229 public tracked files** match the pinned original's bytes/execute
  bits before and after. Both real checkouts remain Git-clean; repeated complete
  non-following metadata digests and tracked-content digests match. Dependency
  metadata equality is not a full installed-package content-integrity proof.
- No implementation changes: full Rust/MCP/audit/observer/Python suites were
  **not rerun**, and Tier-A was **not triggered**. Earlier totals are not fresh
  evidence here; no new accuracy baseline, multicorpus run or rebaseline.
- Prism navigation skill returned `SymbolNotFound` for the new entry point.
  Direct source inspection and actual compiled-loader/process evidence supplied
  the fallback. No type-resolved LSP claim or independent-agent review.
- A helper compile used a nonexistent serde_json rlib path: inadmissible setup
  evidence, corrected to a listed dependency before any census conclusion.
- Two self-review rounds: first reviewed gate attribution and proof/observation
  separation; second checked fixed populations, raw responses, timing scope,
  privacy and source preservation. **WRONG 0 in production; SMELL: admission,
  cost and source-bound packaging limits.** These remain documented, not waived.

The [machine receipt](2026-09-09-owner-value-checkpoint.json) records identities,
all fixture outcomes and unrounded samples. Raw scripts, stdout/stderr, fixture
bytes and private inventories are preserved in a **local-only private evidence
archive** identified by the [handoff](../../superpowers/handoffs/2026-09-09-owner-value-checkpoint.md).
No private raw artifact or benchmark source is uploaded to GitHub.

## Next recommendation

Implement a **bounded, non-authorizing admission report** before portable packaging
or broader activation: expose first executed gate and separately measured
input-count/byte/language facts from the existing loaded census; mark downstream
compiler/closure/receiver stages as **not reached**. Tests should cover simultaneous
count/language failures, exact budget boundaries, empty input and missing compiler,
with no profile override or proof bypass. This improves actionable diagnostics;
it does not claim to unlock real recall.

Then use that report for an owner decision on a genuine complete project boundary
and acquisition feasibility. Packaging would only make today's refusals portable;
caching needs a separate freshness/epoch proof. Neither is the next value priority
on the measured roots. Keep closure barriers and the unresolved react-scripts
decision unchanged; do not delete an inconvenient script or declaration to pass.
