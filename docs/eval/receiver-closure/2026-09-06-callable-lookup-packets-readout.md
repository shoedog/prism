# Refused compiler lookups — packet repair and real census

> Status reconciliation: PR264 merged at f2f8a3c5. The same-date callable-dependency-acquisition spec records the completed successor preflight. Its plan separates package acquisition from snapshot admission and compiler closure; no install or new receiver gain is claimed.

Implemented from merged main54b6755ab1cc4601e61d8e5a0b991a8775c29b76 on
fix/callable-lookup-packets. Implementationbf6d4fc is pushed in [PR264](https://github.com/shoedog/prism/pull/264), targeting main. Owner approved
this successor after PR261/262/263 merged. No runtime authority or resolver,
Rust/CPG/navigation/cache/default dependency changes.

## Result and contract

Schema /4, producer0.5.0 retain unsafe in-root compiler lookup probes as sorted
unique SHA256 digests in snapshot.refused_lookup_sha256, never path IDs. The host
refuses them before snapshot access, adds unsupported_lookup, retains partial
observations and independently blocks dependency/augmentation/resolution closure.
Actual unsupported inventory filenames, .git and outside-root barriers remain.
Full recomputation protects removal/forgery/replacement; strict pre-I/O schema
checks cover digest/order/reason/closure consistency and reject older schemas.

This does not blacklist module specifier names. A configured safe paths mapping
can resolve a virtual name without refusal. Conversely, a refused first mapping
followed by a safe successful mapping must remain unproven even without diagnostics.
Ambient declaration masking is also tested. Hashes are not anonymization, absence
proof, runtime class evidence or an authenticated dependency certificate.

## Hypothesis / probe / result

WRONG, corrected: compiler-produced colon lookup IDs caused parent schema rejection
and discarded otherwise available observations. Alternatives were worker failure
and array-budget overflow. On unchanged merged main, the builtin and virtual-import
fixtures returned worker_failed/zero observations while an ordinary missing import
retained its observation and the closed control remained observed. Captured
public/red.log contains five real failures: two lost-observation cases, two missing
new-field cases and one obsolete-schema pre-I/O case; no setup failures count.

Same-environment public raw-worker diagnostic: no reported worker error/stderr,
30 observations,2863 failed lookup IDs,12 invalid colon-containing IDs (below the
100000-entry schema cap). Parent rejects that raw packet. The corrected producer
retains exactly the raw predecessor's observations and inventory: census.mjs
asserts deep equality, valid lookup preservation and exact refused-probe hashes.
This controls for changed input, heuristic receiver gains and worker failure.

Three SELF-PASS rounds, NOT INDEPENDENT, no agents/restart/cap extension:
round1 five regression/contract cases; round2 twelve cases including tamper,
ambient masking, boundaries, invalid real filenames, controls/length and stale
replacement; round3 safe mapping and refused-then-safe mapping, plus complete
retained suite. Fourteen added tests cover new evidence/schema, restored observations
and negative controls. The safe mapping positive intentionally retains predecessor
behavior; the five captured RED cases establish the changed behavior.

## Gates

| Gate | Fresh result |
|---|---|
| Observer Node suite | 85 passed,0 failed,0 skipped (71 retained +14 added) |
| Pinned compiler fixtures | 40 passed,0 failures |
| Audit/tamper helpers | 4 passed,0 failed |
| Full default Rust | 4017 passed,0 failed,1 existing ignored;28 groups,including2 doctests |
| Full MCP Rust | 4207 passed,0 failed,1 existing ignored;30 groups,including2 doctests |
| cargo fmt --check / git diff --check | passed |

Compiler5.9.3 SHA2563ae902c92cc44dace175c0e69e13a4b0899f6983c6121d76b9ab8dd5795e7675;
explicit pre-existing react18/react19 fixture profiles. Rust commands run offline
in this worktree with the existing shared target directory. The ignored test is
resolution_test::slice_elem_variant_reserved. No Tier-A trigger; no fresh Tier-A,
Clippy-clean, full multicorpus or rebaseline claim.

Exact gate commands (compiler/profiles paths are pinned in the handoff):

```sh
PRISM_TYPESCRIPT="$compiler" PRISM_CALLABLE_PROFILES="$profiles" node --test scripts/callable-observations/*.test.mjs
node docs/eval/receiver-closure/verify-callable-authority.mjs "$compiler" "$profiles"
PRISM_TYPESCRIPT="$compiler" PRISM_CALLABLE_PROFILES="$profiles" node --test docs/eval/receiver-closure/verify-callable-authority.test.mjs docs/eval/receiver-closure/audit-callable-source.test.mjs
CARGO_TARGET_DIR=/Users/wesleyjinks/code/slicing/target cargo test --offline
CARGO_TARGET_DIR=/Users/wesleyjinks/code/slicing/target cargo test --offline --features mcp
cargo fmt --check
git diff --check origin/main..HEAD
```

## Configured real receiver census

Excalidraw clean commit0642e72cfa2d9a71198200e52f37399384610ee3, actual tsconfig.json:

| Measure | Merged base accepted packet | Repaired accepted packet |
|---|---:|---:|
| Annotated function observations | 0 (worker_failed) | 30 |
| Nested calls | 0 (discarded, not absent) | 53 |
| Linked nested bindings | 0 (discarded, not absent) | 10 |
| Refused probes retained | 0 | 12 |
| Observed/candidate class declarations | 0 | 0 |

Repaired packet validates as reproduced/unproven. Reasons: compiler_diagnostics,
outside_lookup, unresolved_module and unsupported_lookup. These are restored
observations, not newly proven runtime targets or an exhaustive application census.

The four library receiver calls in LibraryMenuHeaderContent.tsx at lines155,160,
184,265 are linked but props_class.reason=callable_unproven. Their callable
provenance stops at unresolved_symbol; react and react/jsx-runtime targets are
null and the inventory contains zero node_modules/@types/react files. Missing
configured declaration inputs, not this repaired packet failure, now limit that
measurement. Do not infer an application defect or manufacture compiler closure.

Private read-only actual-config replay validates as reproduced/unproven. Source,
names, paths and detailed results remain local and outside the public archive.
Neither application was modified; no dependencies installed or options rewritten.

## Custody and next recommendation

Evidence: /private/tmp/prism-lookup-packet-YGuUwA/public/ holds RED/round2/final Node,
full Rust logs, compiler/helper outputs, predecessor diagnostic, accepted public
packet/validation and machine-checked census. red-source.tgz and round2-source.tgz
are stable source checkpoints. private/ is excluded from public archives.

The isolated-worktree skill preserved the dirty original checkout and predecessor
artifacts. Next recommendation: a bounded dependency-acquisition/preflight plan
for a lockfile-pinned, dependency-complete real Program, then repeat these exact
receiver spans. Installing dependencies requires separate approval; no spelling
heuristics or runtime/React.FC authority expansion is justified by this result.
