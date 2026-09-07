# Configured callable observations (opt-in research tool)

This standalone Node tool constructs a bounded TypeScript Program and emits
reproducible observations. **It cannot authorize a Prism edge.** Nothing imports
it from the production resolver, and it adds no default compiler dependency.

Supply the known TypeScript5.9.3 package/lib/typescript.js explicitly. Its bytes
must match the constant in schema.mjs before execution. Dependencies must already
be present inside the project root; the tool never installs anything or runs
project scripts/plugins. Audited roots must be trusted, quiescent local trees.

```sh
node scripts/callable-observations/index.mjs produce "$compiler" "$project" tsconfig.json > "$packet"
node scripts/callable-observations/index.mjs validate "$compiler" "$project" tsconfig.json < "$packet"
```

Use task-specific variables and keep the packet **outside** the audited root:
writing it inside would change the snapshot it purports to describe. Output can
contain private relative filenames, identifiers and type strings; do not publish
an application packet without permission. Validation roots/config are supplied
independently by the caller, never taken from packet fields.

The module exports `produce({root,compiler,config,limits?})` and
`validate(jsonText, sameOptions)`. Library-only limits may be lowered, not raised.
Invalid caller options throw from produce; acquisition/worker limits produce an
unproven packet. CLI invalid options/invalid validation exit1; CLI production of
an unproven observation exits0. Read the JSON, not just the exit code.

## Meaning of the packet

The strict executable v2 schema is `schema.mjs` (`parsePacket`). It freezes these
groups, rejecting unknown fields and unsafe IDs before project access:

| Group | Meaning |
|---|---|
| schema / authorizes_runtime_edge | prism.callable-observation/2; authority is always false; v0/v1 packets reject before root access |
| producer / compiler | Tool-byte digest; required compiler version/hash, whether actually verified, full compiler-lib inventory digest |
| scope | Relative config, direct-annotated-function scope, class_authority=false, compiler host case policy (null before acquisition) |
| status / reasons / closure | observed means this bounded Program completed without the enumerated closure failures; unproven records limitations. Neither means a receiver or class is proven |
| snapshot | Raw byte/file/directory manifest, roots, config reads, Program files, all reads and failed lookup IDs, options digest and outside-lookup flag |
| resolutions / diagnostics | Compiler module-resolution outcomes and anchored diagnostic codes; unresolved dependencies are not automatically application defects |
| observations | Direct variable annotations on arrow/function expressions; annotation/implementation/first-parameter anchors, explicit annotation flag, contextual callable declarations/signatures, direct-body member-call receiver types and method declaration anchors |
| observations.provenance | Bounded defining-source declaration/alias observations, generic use/binder anchors, namespace qualifiers and partial-chain reasons; not a substitution or ownership certificate |
| observations.nested | Nested arrow/function-expression call anchors, enclosing callback anchors, first-parameter binding observations and explicit scope/budget barriers |
| limits | Up to20000 files+directories,128MiB input bytes,depth64,2000 observations,32 provenance steps,8 nested callback levels and128 nested calls per observation,30-second worker timeout;512MiB worker heap and8MiB packet cap |

Anchors carry file-byte hashes and half-open UTF-16/UTF-8 coordinates. Manifest
consistency, range/hash conflicts and impossible statuses are rejected before
recomputation. Validation then rebuilds the entire packet and compares every field;
there is no persisted positive cache. `valid:true, packet_status:unproven` means
the incomplete observation reproduced, **not that closure was established**.
`authorizes_runtime_edge` remains false on every validator outcome.

No authenticated alias-chain certificate, class-identity certificate, runtime write
proof or served-resolution consumer is implemented. A
method declaration or contextual signature is evidence to inspect, not an Exact
target. Explicit-any, merged-overload and post-declaration assertion controls
exercise this distinction.

## Declaration provenance

Producer0.3.0 includes `provenance.mjs` and `nested.mjs` in its byte digest. `provenance.status=traced`
means the supported syntactic chain reached an inline callable type or a singleton,
non-inherited callable interface. It is independent of program closure: even a
traced chain can belong to an unproven packet. Type arguments and parameters keep
their own defining-file anchors; they are not substituted or certified as classes.

Each hop retains reference, declaration, generic argument/binder and immediate
import/re-export alias anchors. Qualified names also retain each namespace use and
binding. Module evidence records the specifier, module declarations, export
assignments/star exports and their binding declarations. Namespace `export =`
traversal requires a direct unique local namespace, never an imported gateway or
a React spelling heuristic. Local bindings and export names are checked separately
for duplicates, including compiler error recovery that exposes only one symbol.

Star exports in any consulted module, unresolved/duplicate/merged declarations,
inheritance, unsupported types/declarations, cycles and step exhaustion retain an
unproven reason and partial evidence. General import-equals/export-assignment
callable paths and conditional/mapped/intersection/union evaluation are excluded.
The visit budget bounds chain traversal; source inventories
are bounded by the existing file/byte/heap/time limits. No runtime write/cache
barriers in Prism are changed.

## Nested lexical bindings

`nested.calls[].binding.status=linked` means a supported nested receiver root
resolves to one supported first-parameter binding of the outer annotated function,
without duplicate bindings or observed direct syntactic writes in that body. It
does not mean its value, class, mutability, effects or runtime target are proved.
Both explicit-any and dependency-incomplete Programs can supply lexical links.

Supported receivers are identifiers and non-optional named property chains.
Supported first-parameter bindings are plain identifiers or flat object elements,
including renamed elements, without rest/default/computed/nested patterns. Shadows,
foreign bindings and unsupported forms retain an unproven reason. Enclosing
callback anchors distinguish lexical scope, including JSX callbacks. Classes,
methods and function declarations are visible unsupported-scope fences; classes
are also excluded from the direct-body inventory, including fields/static blocks.
Depth/call exhaustion emits a barrier; consumers must not interpret a truncated
or fenced census as an exhaustive absence of nested calls.

Direct writes include assignment/update/delete and for-in/of targets, property
and element paths, and destructuring targets. Shorthand assignment value symbols
are distinguished from property symbols. Whole-body writes are conservative across
ordering and nested scopes; writes to distinct shadows do not poison the outer
binding. Writes through aliases, opaque calls and external effects are NOT proved
absent. This is lexical observation tooling, not a runtime write/effect certificate.
All nested anchors and linked-status invariants are checked pre-I/O; complete
recomputation rejects forged or deleted binding/write/barrier evidence.

## Supported boundary and limitations

One project, actual JSON config/extends/include/exclude/options, and installed
in-root regular-file dependencies. The compiler host reads an immutable in-memory
snapshot only. Its lookup/canonicalization policy follows the pinned compiler's
host case policy; ambiguous case-folded inventory collisions are refused.
Internal TS matchFiles/createGetCanonicalFileName APIs are used deliberately under
the exact compiler byte pin, not as an unversioned API promise.

All input bytes/membership are fingerprinted, including unused/untracked files;
this conservatively invalidates more than a minimal dependency cache. `.git`
contents are excluded, with boundary sentinels: explicitly reading/enumerating
them is refused rather than silently claiming an empty input set.
Project references and plugins remain unsupported. Outside-root/absolute-config
lookups, unavailable packages, symlinks, special files, invalid UTF-8 compiler
inputs and budget exhaustion stay unproven. Local extends may traverse upward
within the supplied root, but cannot escape it.

The tool does not rewrite actual project options to manufacture closure. For
example, missing package boundaries and automatic ancestor type/lib lookups may
leave an otherwise compilable project unproven. Synthetic positives explicitly
supply package.json, types=[] and libReplacement=false in their own config.

Two snapshots detect observed changes, not adversarial change-and-revert races or
an atomic filesystem transaction. This is not an OS security sandbox. Mixed
filesystem policies/mounts and hostile concurrent mutation are unsupported. A
future authenticated proof consumer would require a stronger acquisition contract.

## Tests

```sh
PRISM_TYPESCRIPT="$compiler" PRISM_CALLABLE_PROFILES="$profiles" node --test scripts/callable-observations/*.test.mjs
```

`profiles` uses PR259's pinned react18/react19 layouts. Tests create their own
temporary installed-package projects, never install or modify an application.
See the same-date callable-nested-bindings spec/readout for RED captures,
full-project gates, evidence limitations and the separately approved next boundary.
