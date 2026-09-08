# Source type/lib reference observations

Owner: "merged, proceed to separate type/lib reference observations and source-backed react-scripts disposition."
Base: PR278 merged `f8c6982e6b4e10fa74a8be281243eec0e6176883`.

## Bounded contract

Add `type_lib_references[]` in schema11/producer0.12.0. Each row represents one
original Program source's triple-slash `types` or `lib` directive: kind, source
hash/name span in UTF16 and bytes, kind-local index, literal name, actual effective
type resolution mode (import/require/null), observed/unproven status, reason,
canonical Program target or null, and matching inclusion boolean. These are input
observations, not symbol/augmentation/runtime identity or closed-universe authority.

Use pinned TS5.9.3 configured-Program caches only. Types use
getResolvedTypeReferenceDirectiveFromTypeReferenceDirective and the Program's
effective default mode unless explicitly overridden by the directive. Libs use
the actual resolvedLibReferences target, including a package replacement or builtin
fallback. Do not call a resolver or pathForLibFile during observation. Require the
actual target SourceFile in the Program and its FileIncludeKind5/7 source/index
record. Census redirectInfo.unredirected; redirected original directives are
unprocessed, never authorized by an inherited AST or another source's cache.

Distinguish unprocessed traversal (noResolve/noLib/redirect/cache absent), unresolved
lookup (missing type cache target or unknown library name), target_not_in_program,
and missing_inclusion. Validate original source text against snapshot bytes before
using references. Existing case/link/epoch/read/heap/time/file limits remain; cap
the ledger at100000 (the packet-array ceiling) with budget_exceeded, never truncate.

An unproven source directive adds `unproven_type_lib_reference`, withholding
dependencies/references/augmentation/resolution and Props/class observations even
if diagnostics are suppressed. This is stricter refusal, not closure admission.
All existing path/module/refused/outside/write/duplicate barriers remain separate.
Unknown lib directives under skipLibCheck/noCheck need behavioral RED, not just a
missing-field assertion. Runtime/class authority stays false.

The ledger deliberately excludes configured/automatic type/lib entry occurrences,
config extends and project-reference traversal. Source directives inside their
loaded files are included. Do not mistake an empty source ledger for complete
channel coverage; those entry channels and causal refused/outside attribution are
separate successors. No application install/config edits or React.FC expansion.

## Schema and validation

Schema11 strictly requires the ledger and producer0.12.0. Historical schema10
producer0.11.0/0.11.1 remains parseable for pinned audit readers but is not upcast;
full current reproduction rejects it. New reference reason cannot appear on
schema10. Parser verifies source/target membership, source hashes/ranges, exact
kind/name span widths, stable unique ordered source/kind/index population, and
status/reason/target/inclusion/refusal consistency before root I/O. Canonical
sorting is by source, kind and index. Omissions/substitutions/mode forgeries that
remain well-shaped must fail full recomputation. New helper is in producerHash.

## react-scripts disposition requirements

Recheck the fixed public source and compiler cache. Enumerate tracked manifests
and lock declarations; read existing Vite environment declarations and build
scripts. Classify the unresolved directive as a source obligation, not a module
literal or automatic-type entry. A stale CRA residue is a hypothesis until source
evidence is recorded. Do not install react-scripts or remove the directive in the
audited application. Recommend owner cleanup review if supported; do not claim
Vite declarations prove general CRA environment equivalence.

## Plan and probes

1. Exact-base RED for observation shape/mode/caches/redirects/tampering and actual
   suppressed-lib false closure. Initial20 tests all fail at absent ledger; repeat
   behavioral assertions before implementation. Record controls separately.
2. Implement source-census/cache helper, schema migration, refusal and tests.
3. Replay fixed public packet, reconcile139 type/lib directives independently with
   prior frozen cache and source spans; inspect every changed prior packet field.
4. Full observer/default Rust/MCP/helper/authority gates, two SELF-PASS reviews
   (NOT INDEPENDENT), cap2, then commit/push/PR. Tier-A only if its source paths change.

Hypothesis: clean diagnostics can hide unknown lib obligations as well as paths.
Falsifier: base already rejects the exact lib fixture without unrelated barriers.
Alternative to source-owned resolution: global cache presence or installed package
presence alone. noResolve/noLib and redirect fixtures discriminate that alternative.
Compiler source:127328–127338 cache read;128801–128843 type inclusion;128852–128904
actual lib target/fallback;128906–128929 lib inclusion;129695–129709 effective mode.
Raw evidence root: `/private/tmp/prism-type-lib-4gqvH5`.
