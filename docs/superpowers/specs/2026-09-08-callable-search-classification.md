# S1 — source-backed search-boundary classification

Base: merged PR280, `670bccb0d7fd3181b0405128c68d84fe51de0002`.
This is a diagnostic increment. No worker, schema, closure, runtime or application
behavior changes. The owner-approved sequence is linked from the active handoff.

## Contract

Copy the observer to a disposable task directory and instrument only that copy.
Capture every existing refused/outside `toId` boundary check with occurrence index,
operation tag, normalized path, actual module callback owner when present, and a
bounded compiler stack. Catalog every module callback occurrence, including
synthetic requests, source anchor, mode and eventual target. Do not invoke a second
resolver or read outside the existing virtual host. Abort on capture overflow.

Replay the untouched observer in the same environment. Require equality of every
packet field except producer identity; require module occurrence multiset equality,
the exact deduplicated refusal digest set, and the outside flag. Receipt hashes are
local byte-consistency checks, not cryptographic authentication of an arbitrary
capture. Normal packet validation/reproduction remains the authority for observation
identity; neither artifact grants semantic closure or runtime edges.

## Causal source requirements

Pinned TypeScript 5.9.3, SHA256
`3ae902c92cc44dace175c0e69e13a4b0899f6983c6121d76b9ab8dd5795e7675`:

- `loadModuleFromNearestNodeModulesDirectoryWorker`, lines 46289–46343, performs
  ancestor node_modules directory probes. A path spelling cannot identify its owner.
- `primaryLookup`, lines 44401–44422, probes type-root directories.
- `getPeerDependenciesOfPackageJsonInfo` / `readPackageJsonPeerDependencies`,
  lines 45677–45697, consult peer metadata while assigning package identity.
- Program selects type-reference resolver at 127018–127049 and library resolver at
  127051–127059. Use the **nearest** corresponding stack frame: nested library work
  can retain an outer source-type processing frame.
- Source directives enter at `processTypeReferenceDirectives`, 128801–128819;
  configured/automatic entries enter from `createProgram`, 127131–127143.

Unknown or inconsistent attribution fails the fixed-population audit. Retain raw
occurrences locally; publish compact counts and proof requirements, not local stack
paths. Module callbacks do not constitute a census of all lexical imports: the
compiler can short-circuit ambient names before the host callback.

## Required controls and non-goals

Reject packet drift, receipt byte mismatch, orphan/mismatched module ownership,
omitted refusal coverage and malformed occurrence ordering. Test shared normalized
paths with different owners and nested source/lib frames explicitly. These are new
audit-tool controls and characterization evidence, not claimed production RED.

No diagnostic reason is waived. In particular ancestor probes do not prove external
absence, peer searches are not optional merely because they failed, and a bound
ambient request does not establish runtime asset presence. react-scripts stays
unresolved; the 14 module-literal gaps remain separate.

Review cap two rounds. Full observer/default Rust/MCP/helper/authority verification
and source-preservation checks precede publication. Tier-A is not triggered because
no call-resolution/navigation/CPG/AST code changes.
