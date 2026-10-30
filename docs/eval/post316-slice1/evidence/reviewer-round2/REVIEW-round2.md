# Slice 1 review — round 2 and authorized cap closure

APPROVE — WRONG: 0 / SMELL: 1. **Final evidence reconciliation completed.** See `FINAL-ACCEPTANCE.md` for verified full-suite totals and explicit Tier-A/exclusion limits. The earlier pending-evidence status is superseded.

## Artifact identity

Reviewed frozen commit `8e2f64ff73fb2218bfc103ffc49d03212d662008` against base `f5350044a18bf95f3deb51a41b4b09e9d585b134`; all ten source-manifest hashes matched its isolated archive. At the two-round cap, controller classified the review as converging and authorized ONE bounded test/evidence closure supplement. That supplement changes only `tests/integration/receiver_self_binding_test.rs`, SHA-256 `f7cf6853d233063e11dab8c2d0a51bf7b886e3cd1308101b18a0d3d43dc93eb8`. All four production files remain byte-identical to 8e2f64ff. Controller must record the resulting checkpoint and refresh manifest/handoff; source approval binds to these exact bytes, not later changes.

## Findings and cap classification

- Original WRONG W1 is **fixed**: explicit class-context legacy routing preserves class computed-key, method-body and outer inventories. Fixed-byte method+outer oracles pass all eight routes in JS/TS/TSX. This is mechanism-backed closure of the finding, not a downgrade based on failure to reproduce.
- The overbroad `owner.has_error()` gate was removed after source-ancestry evidence separated clean calls from sibling recovery syntax. Retained receiver-resolution production is untouched; existing recovery refusal checks remain.
- **SMELL S1 remains nonblocking** at `src/ast.rs` owner validation: `all_functions()` reconstructs the full inventory for every capture. No demonstrated performance failure. Do not expand this slice solely for that concern.
- No remaining demonstrated WRONG. No new production class of findings. Cap classification: **converging, closed-enumerable**. The sole authorized closure supplement pinned four retained receiver owners and site spans; it is independently verified. No restart or broad third review is requested.

## Round-one mandatory gaps — disposition

| Requirement | Closure evidence |
|---|---|
| C01/C02/C03/C04 | Eight-query/manual route fixtures, own defaults, eager outer and retained inner calls; fixed complete graph site metadata and exact Call/Return/DataFlow endpoints across JS/TS/TSX. Candidate 8/8 pass; identical tests on unchanged base 2 pass/6 behavioral fail. |
| C05/C06 | Unicode computed-key callback fixture pins name/parameter/body ancestry, root inventory and all eight routes; recovery fixture records parse errors and retains unrelated clean owner while refusing borrowed nested ownership. |
| C07 | Same-line colliding raw owners remain byte-distinct after Unicode; full/skeleton/subset refuse colliding graph owners and preserve exactly one unrelated direct call. |
| C08/C09 | Python/root and class preservation tests pass both base and candidate. Supplied Rust macro 46/46, contained-rvalue 3/3, predecessor capture/parameter/same-line refusal 1/1 retained and executed. |
| C10 | Frozen tests cover full/skeleton/subset, fixed metadata/edges, bincode, and caller-only incremental epochs. Independent reviewer harness additionally verifies actual CPG-cache HIT equality for complete site, resolution, Call/Return/DataFlow and DFG rows in all three dialects. Parallel full and distinct serial subset emitter agree; one-thread/four-thread executions of the same full builder agree. The latter is scheduling determinism, **not** a second serial implementation. The preexisting Rust infra test also proves determinism only; its misleading name is not used as independent serial evidence. |
| C11 | Exact navigation function identities/bytes/lines/ordinals/reasons/multiplicity and outer empty result: candidate 1 pass, identical test on unchanged base 1 behavioral fail. |
| C12 | Independently verified base writer production hashes, writer source+binary hashes, untouched pristine v93/v52 cache hashes. Supplied genuine run directly rejects v93 and rebuilds cached navigation equal to uncached. Independent direct v52-loader probe preserves embedded fingerprint and asserts all other predicates equal before rejection, isolating the version guard on genuine base bytes; cache remains unchanged. |
| Exact16 reconciliation | Round-one same-environment controls proved all16 candidate failures absent on base. Revised receipt maps recovery repairs vs intended nested-owner changes. Retained raw receiver binding and exact callee/refusal assertions survive. Four permanent owner vectors now pin `ns@[73,87)`, `ns@[100,114)`, `client@[133,151)`, `client@[140,158)` at line2 and exclude `run`, across each dialect/full/subset path. Independent module rerun 36/36 passes. |

## Independent executions

| Log | Selected result | What it establishes |
|---|---|---|
| `round2-focused.log` | 8 passed / 0 failed | Frozen ownership/class/recovery/collision/metadata oracles |
| `round2-base-focused.log` | 2 passed / 6 failed | Same test patch on unchanged production authenticates behavioral RED; preserved class/Python controls pass |
| `round2-navigation.log` | 1 passed / 0 failed | Complete navigation rows |
| `round2-base-navigation.log` | 0 passed / 1 failed | Base extra outer relation violates same exact oracle |
| `round2-receiver-self.log` | 36 passed / 0 failed | Frozen receiver controls and resolution/refusal preservation |
| `round2-independent-evidence.log` | 2 passed / 0 failed | Genuine v52 isolated guard; CPG-cache/full/subset/scheduling parity with printed complete tuples |
| `round2-receiver-hardening.log` | 36 passed / 0 failed | Exact authorized permanent test-only closure, hash above |

All reported runs selected nonzero tests; output and assertion rows were inspected. Invalid historical probes are excluded. Reviewer did not run the full project suite; controller/implementor gate logs are supplied evidence and must be evaluated separately.

## Evidence custody

- `round2-evidence-harness-cpg.rs` SHA-256 `6d8e68fd7315f7882ee8d9bad777f5a6906b6293dadc03a72e34e192bc3af2d2`: isolated test-only endpoint/cache/scheduling harness, append location `src/cpg/nested_execution_owner_audit_tests.rs`.
- `round2-evidence-harness-nav.rs` SHA-256 `a24c67b5704227b02232fc5ff9e6e123e78b4b03a4f1cb86613dd4ce0ea9f249`: isolated test-only direct genuine-v52 loader harness, append location `src/navigation/call_edge_cache.rs`. It references the immutable local evidence path and is evidence tooling, not a portable fixture.
- `round2-cache-provenance-independent.json`: exact verified writer/source/binary/cache hashes.
- `round2-receiver-vector-probe.log`: independently observed full retained CallSites preceding the worker's fixed-literal hardening.
- `round2-evidence-manifest.sha256`: review source/log hash custody.

Preserve these small harnesses and logs in the lane evidence store. No worker production source was edited by the reviewer.

## Final gate status and limitations

Observed supplied default-final log completed **4,518 passed / 0 failed / 1 ignored** on 8e2f64ff before authorized receiver-test hardening. The worker is running the post-hardening full default and remaining required gates. This report does not represent those unfinished logs as green. Remaining acceptance evidence: post-hardening default completion, MCP, widest audit, examples, fmt/clippy/diff, Node/authority/Python and Tier-A immediate rebuild plus matrix/quick report status, same-environment controls for unexpected failures, exact exclusions, and final custody reconciliation. Unavailable/INVALID gates must be named rather than silently skipped.

Source ownership and preserved callee/DFG evidence are established for the bounded fixtures. Class correctness remains explicitly excluded; unindexed callable/identity-collision refusals remain deliberate. No measured public FullFlow accuracy increase, publication, CI, merge or operator authority is claimed.

## Final reconciliation

Accepted source commit `07ed5ceb6098f2b5a21f5c05008a2c823a5617d7` matches all ten final manifest hashes including the authorized receiver hardening. Remaining gate evidence has been inspected; final totals and exact limitations are recorded in `FINAL-ACCEPTANCE.md`. Earlier prospective/pending gate statements above are historical review context and are superseded by final acceptance.
