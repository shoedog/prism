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

The module exports `produce({root,compiler,config,profile?,links?,limits?})` and
`validate(jsonText, sameOptions)`. Select `profile: "installed"` explicitly for
the larger bounded inventory; omission selects `default`. Limits may only lower
the selected profile. `links: "in-root"` separately enables bounded canonical
links; omission selects `reject`. CLI optional final arguments are profile, then
link policy (for example `installed in-root`). Neither is inferred from the tree.
Validation independently selects its profile; packet fields never select it.
Invalid caller options throw from produce; acquisition/worker limits produce an
unproven packet. CLI invalid options/invalid validation exit1; CLI production of
an unproven observation exits0. Read the JSON, not just the exit code.

## Meaning of the packet

Producer0.21.0 retains the path-completeness repair: required triple-slash
paths are checked independently of `skipLibCheck`/`noCheck` diagnostics. Each
original-source directive needs a compiler-cache target and matching source/index
inclusion record. Missing, self-referential, unsupported or unprocessed paths add
`unproven_path_reference`, withholding dependency/reference/augmentation/resolution
bits and class candidates. Ordinary failed search candidates remain harmless.
See the [bounded repair](../../docs/superpowers/specs/2026-09-07-callable-required-path-completeness.md).

Source-written `types` and `lib` directives now have separate observations in
`type_lib_references[]`. Original-source name spans and kind-local indices retain
actual effective type modes and compiler-selected targets, including lib replacement
and builtin fallback. Observed requires Program membership plus source/index
inclusion. Redirect originals and disabled traversal remain unprocessed; unresolved,
unprocessed, absent-Program and missing-inclusion cases add
`unproven_type_lib_reference` and withhold the same four completeness bits/class
candidates. No second resolver runs. Ordinary failed optional searches stay separate.

`type_lib_entries[]` separately records effective configured types/libs, compiler
automatic type discovery and the default lib entry. Rows retain kind, origin,
index, effective name, null entry mode, actual Program target and matching entry
inclusion. Types use the retained name/mode cache and kind8 name inclusion;
configured libs use cached actual targets and kind6 option indices; default libs
use indexless kind6 selection. Repeated names retain occurrence indices. Configured
lib names are normalized compiler filenames, not JSON source spans.

Entries are additive observations and do not change existing reasons or closure
bits. Unprocessed entries do not prove why traversal was suppressed or that an
obligation is missing. Even an observed packet is not full reference-channel or
semantic-closure authority: observed config origins, search attribution and entry
completeness are separate proof layers, not a Program-closure policy. Loaded
sources' directives ARE included in the source ledger, not invented as entries.
The public `react-scripts` directive is now explicitly unresolved, not installed,
substituted or waived. Both runtime/class authority flags remain false.
Historical schema10 (producer0.11.0/0.11.1), schema11 (producer0.12.0),
schema12 (producer0.13.0), schema13 (producer0.14.0), schema14 (producer0.15.0), schema15 (producer0.16.0), schema16 (producer0.17.0), schema17 (producer0.18.0), schema18 (producer0.19.0) and schema19 (producer0.20.0) packets
remain parseable without invented reference/entry rows for pinned audits, but
cannot validate as current output; validation
recomputes the producer identity and every field.

`search_provenance` adds actual module callback occurrences and outside/refused
boundary encounters. Module rows reuse existing literal anchors after observation;
synthetic requests keep null anchors. Boundary rows retain occurrence order,
operation, opaque normalized-probe digest and module/type/lib owner or explicit null owner.
Repeated paths do not collapse. Identity conversion is distinct from a host
operation. Config/discovery work without an active resolver context stays unattributed,
not waived. A context stack covers resolver execution through target projection;
no second resolver or new module cache is introduced. Each population is capped
independently at100000 and overflow fails closed. Structural consistency does not
authenticate event ownership: full reproduction checks every new field.

Type observations separately retain actual callback batches, each source/configured/
automatic occurrence, and resolver executions. Duplicate names within a batch share
an execution; source revisits retain separate batches and executions. The pinned
shared type cache and per-batch cache behavior are preserved. Source coordinates and
targets are canonical file identities; configured/automatic containing-file addresses
preserve lexical config-directory aliases and case and need no file membership.
Per-batch indices use numeric order even though older row serialization is lexical.
Suppressed or rootless rows do not invent callbacks. Parsing checks exact per-batch
row coverage; reproduction authenticates repeated callback count/order and ownership.
Entry-obligation policy remains separate.

Library searches preserve the separate pinned library resolution cache and actual
hook invocation order. Each search records its lexical synthetic address, resolver
target and Program-selected replacement/builtin fallback. The selected fallback can
be absent from the inventory/Program. Source/configured beneficiaries require exact
existing positive inclusion; they are not causal demands. A shared search can serve
several beneficiaries, while a missing fallback can serve none. Default selection
and libReplacement:false do not fabricate searches; transitive source-lib searches
remain real. Per-search and total relationships are bounded before insertion, with
separate retained-record guards. Full reproduction authenticates search census,
actual selections and beneficiaries; attribution never proves outside absence.

`config_provenance` observes one bounded local relative-string extends chain and
the nearest explicit origins of types,lib,typeRoots,noLib,libReplacement,noResolve
and target. It retains text from existing reads and uses an always-miss recording
config cache; no extra host operations or caching are introduced. Unsupported chains,
duplicate properties, null resets and unprovable selections yield an empty unproven
observation, without changing prior diagnostics, reasons or closure. This is not
automatic-type discovery completeness or whole-Program validity.

`entry_obligations` separately classifies each existing entry as selected, disabled
or unproven. Only no roots or an observed explicit noLib:true can disable an old
unprocessed row. Source no-default-lib suppression remains unknown. With roots,
implicit types remain automatic_discovery_unproven even if all listed automatic
names resolve or no names are listed. Explicit types:[] is a configured choice.
The parser recomputes the whole field; no existing closure bit or reason changes.

`semantic_closure` implements the [strict merged-side-effect contract](../../docs/superpowers/specs/2026-09-08-callable-merged-side-effect-admission.md)
as an additive type-source completeness field. Fixed policy merged-side-effect-v3
retains selected Program targets, observed singleton exact-ambient and singleton
wildcard bindings, and admits only the existing observed empty-block/shorthand
side-effect pair. Assets and typed/value imports do not gain authority. Byte-frozen
v1/v2 classifiers govern historical schema18/19; newer policy is never selected by
a caller or retroactively applied to older packets. Every outside/refused event, config,
entry, source-reference and diagnostic barrier remains independently mandatory.
No old status/reason/closure/Props field changes; even semantic complete can coexist
with old unresolved_module and program_unproven. Full reproduction authenticates
the facts; no runtime/asset authority or production consumer is added.

The strict executable v20 schema is `schema.mjs` (`parsePacket`). It freezes these
groups, rejecting unknown fields and unsafe IDs before project access:

| Group | Meaning |
|---|---|
| schema / authorizes_runtime_edge | prism.callable-observation/20; authority is always false; historical schemas10/11/12/13/14/15/16/17/18/19 remain readable, earlier schemas reject before root access |
| producer / compiler | Tool-byte digest; required compiler version/hash, whether actually verified, full compiler-lib inventory digest |
| scope | Relative config, acquisition profile, link policy, direct-annotated-function scope, class_authority=false, compiler host case policy (null before acquisition) |
| status / reasons / closure | observed means this bounded Program completed without the enumerated closure failures; unproven records limitations. Neither means a receiver or class is proven |
| snapshot | Raw byte/file/directory manifest, link spelling hashes and canonical targets, roots, config reads, Program files, reads and safe failed lookup IDs, refused-lookup digests, options digest and outside-lookup flag |
| resolutions / diagnostics | Compiler module-resolution outcomes and anchored diagnostic codes; unresolved dependencies are not automatically application defects |
| type_lib_references | Source-written types/lib occurrences only: kind-local index, name anchor, effective mode, selected Program target, inclusion and refusal. Canonical serialized source/kind/index ordering; at most100000 rows, then budget_exceeded, never truncation |
| type_lib_entries | Effective configured/automatic/default entries: kind, origin, index, name, null mode, selected Program target, inclusion and observation reason. Canonical serialized kind/index ordering; separate100000-row cap; no closure-policy effect |
| search_provenance | Module occurrences, type batches/occurrences/executions, actual lib searches/positive beneficiaries, and outside/refused boundary events with nullable module/type/lib owners; exact row and legacy aggregate reconciliation; no complete lexical or search-channel census |
| config_provenance | Bounded root-first canonical config anchors, exact extends edges, seven ordered option-origin/value digests; all and only successful config reads; unproven chains have empty arrays |
| entry_obligations | Exact ordered entry dispositions and conservative config/automatic-universe/row completeness; independently recomputed, no old closure effect |
| semantic_closure | Fixed merged-side-effect-v3 type-source completeness (schema18 retains v1, schema19 retains v2), exact ordered resolution dispositions and strict aggregate barriers; recomputed from validated facts; no old status/closure or receiver authority effect |
| resolutions[].lookup | Actual request anchor/context, checker declarations and configured-Program exact-name provider/augmentation census; observed means singleton exact-ambient binding only, never filesystem or closure authority |
| resolutions[].lookup.wildcard | Independent nullable single-star binding observation: pattern, original-source providers, relevant augmentations and matching-provider census; never asset-existence or closure authority |
| resolutions[].lookup.merged_wildcard | Separate nullable side-effect-only empty-block/shorthand pair observation; checker-ordered source shapes and actual selected value declaration; old wildcard refusal remains unchanged |
| observations | Direct variable annotations on arrow/function expressions; annotation/implementation/first-parameter anchors, explicit annotation flag, contextual callable declarations/signatures, direct-body member-call receiver types and method declaration anchors |
| observations.provenance | Bounded defining-source declaration/alias observations, generic use/binder anchors, namespace qualifiers and partial-chain reasons; not a substitution or ownership certificate |
| observations.nested | Nested arrow/function-expression call anchors, enclosing callback anchors, first-parameter binding observations and explicit scope/budget barriers |
| observations.nested.calls[].props_class | Instantiated contextual Props/property/class declaration observations, generic binder/argument anchors, and explicit unsupported/program barriers; never runtime class authority |
| limits | Up to20000 files+directories,128MiB input bytes,depth64,2000 observations,32 provenance steps,8 nested callback levels,128 nested calls per observation,8 Props type arguments,30-second worker timeout;512MiB worker heap and8MiB packet cap |

The table describes default ceilings, unchanged from the prior producer. Explicit
`installed` raises inventory entries to100000, hashed input to1GiB, worker timeout
to120 seconds, V8 old-space to1024MiB and packet cap to32MiB. It bounds an individual
materialized read to32MiB (`read_bytes`); default read_bytes is128MiB, its existing
total-input ceiling. Hashing an unused larger file does not materialize it. Depth
and observation/provenance/nested limits remain unchanged. Heap caps are not RSS
caps. Larger profile selection does not grant link support or Program closure.
When links are enabled, they count as entries; `link_steps` bounds each resolution
to32 traversals and can be lowered. Metadata boundary sentinels also obey budgets.

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

Producer0.18.0 includes `inventory.mjs`, `provenance.mjs`, `nested.mjs`, `props-class.mjs`, `exact-ambient.mjs`, `wildcard.mjs`, `merged-wildcard.mjs`, `required-paths.mjs`, `type-lib.mjs`, `entries.mjs`, `search-provenance.mjs`, `identity-domains.mjs`, `lib-search.mjs`, `config-provenance.mjs` and `entry-obligations.mjs` in its byte digest. `provenance.status=traced`
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

A type-only UMD qualifier may traverse one eligible `export as namespace` in an
external declaration file, then one `export =` alias to a direct same-file local
namespace. Global-export and local-name populations are separate. The configured
Program must contain exactly one global provider and no same-name global binding
or augmentation; duplicate/merged/imported targets and star gateways are refused,
including when `skipLibCheck` suppresses diagnostics. Compiler alias-table identity
and canonical Program source ownership are required, not just a syntax census.
For this gateway, `module` anchors the owning SourceFile (not a module specifier),
with separate assignment and local-binding anchors in the existing record fields.
Each alias and the owning-module traversal consume provenance steps. One lazy
Program-keyed population census is bounded by acquisition bytes/files and worker
timeout/heap; it is not reused across Programs or snapshots. No compiler option is
changed: `allowUmdGlobalAccess` value-use diagnostics do not gate type-only use.
Traced UMD candidates still cannot overcome receiver writes or incomplete Program
closure; both authority flags remain false. Schema11 requires its reference ledger;
schema10 remains historical-only and is never upcast.

Star exports in any consulted module, unresolved/duplicate/merged declarations,
inheritance, unsupported types/declarations, cycles and step exhaustion retain an
unproven reason and partial evidence. General import-equals/export-assignment
callable paths and conditional/mapped/intersection/union evaluation are excluded.
The visit budget bounds chain traversal; source inventories
are bounded by the existing file/byte/heap/time limits. No runtime write/cache
barriers in Prism are changed.

## Exact-ambient lookup observations

Filesystem `target` is unchanged. A separate `lookup.status=observed` requires
an actual import, export-from, import-type or import-equals StringLiteral bound
by the pinned checker to one exact top-level ambient module with a body in a
non-external declaration file. The canonical configured Program must contain
one same-name provider and no augmentation; a separate full AST census catches
duplicates hidden by symbol merging/error recovery. Excluded inventory files
are not providers. The census lives only for this Program and snapshot.
Package-ID redirects are censused through their original parsed SourceFiles,
preserving each file's bytes/anchors instead of shared AST parent pointers.

In the exact-ambient lane, wildcard/merged bindings, require/dynamic-import requests, unsupported providers,
missing symbols and augmentation declaration names remain unproven. Candidate
declarations are retained independently of status. Synthetic JSX requests have
`context=synthetic`, null request anchors and no checker binding query. Multiple
source files can generate the same synthetic specifier; occurrences remain separate.

None of these observations clears unresolved-module, refused/outside-lookup,
diagnostic, receiver-write or class barriers. The schema rejects null filesystem
targets without their existing closure refusal before audited-root access; full
recomputation checks every anchor, census and binding. No new closure policy,
runtime consumer, persistent positive cache or React.FC authority is supplied.

## Singleton wildcard observations

`lookup.wildcard` is null without a uniquely named valid single-star checker
binding. Otherwise it retains candidate pattern/providers/augmentations/matches.
Its status is independent of the existing exact-ambient status: a supported
wildcard remains `lookup.reason=non_exact_binding` while
`lookup.wildcard.status=observed`. Filesystem results are unchanged.

A positive needs one actual supported literal use, one checker declaration,
one original-source top-level provider with a body in a non-external .d.ts,
no matching augmentation or exact-name provider, and no other matching wildcard
provider. Matching is case-sensitive prefix/suffix matching with TypeScript's
minimum-length rule. Competing patterns are refused even when the compiler can
pick a longest-prefix winner. Merged/shorthand/nested/redirected duplicates and
unsupported request contexts stay unproven. Pattern matches are memoized only
within the current configured Program, never across epochs.

No asset path or existence fact is inferred. Tests observe the same binding with
and without asset bytes. Every old unresolved/refused/outside/diagnostic/write/
class barrier and both false authority flags remain unchanged. Strict schema
checks run before root access; full recomputation validates every field.

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

## Bounded merged wildcard observations

`lookup.merged_wildcard` is separate from the old wildcard disposition, which
continues to refuse duplicate providers. A positive new record requires an actual
side-effect-only import (no clause/attributes/modifiers) and exactly two same-pattern
top-level non-external declaration-file providers: one empty ModuleBlock and one
bodyless shorthand. No exact provider, competing match or relevant augmentation
may coexist. The actual request symbol, contributor symbols, original-source
census and selected `valueDeclaration` must agree by identity in this Program.

`declarations` preserves checker order with `{declaration,shape}` entries;
`selected` records the actual selected declaration, never an order-based guess.
Shapes are empty_block/shorthand/other, with other always ineligible. Value/type
imports, typed merges, other pairs, third/nested/excluded providers and unsupported
ownership retain refusals or no candidate. Source/body/context/membership changes
invalidate validation, including genuine selected-declaration substitutions.

All schema9 fields and ordering are preserved before the new lane is compared.
Filesystem targets, asset inventory and closure/refusal/write/class barriers stay
independent. In particular, an observed pair still belongs to an unproven Program
with a null filesystem target. Historical fixed-packet audit commands use their
explicit pinned digest, not the current packet validator's schema.

## Supported boundary and limitations

### Instantiated Props/property declarations

`props_class.status=observed` requires the existing linked lexical binding, traced
callable provenance, one contextual signature and a complete configured Program.
Its actual first-parameter type supplies Props; outer generic argument order and
React/FC names do not. Explicit implementation annotations are excluded.

Support one selected property from a plain parameter or flat destructured/renamed
binding, through a singleton non-inherited interface or type literal (including
generic literal aliases). Record Props/property declarations and instantiated
generic binder/argument pairs. The selected property must directly reference a
class or that Props declaration's type parameter. Its instantiated type must be a
singleton non-generic, non-inherited ClassDeclaration instance. Constructor types,
structural interfaces, merging, optional/computed/index/accessor properties and
arbitrary type evaluation remain unproven. Imports retain compiler-resolved
defining-file anchors; intermediate type-argument aliases are not certified.

Incomplete Programs retain candidate anchors with `program_unproven`. Class
declaration identity does not prove a runtime value, subclasses, member stability
or opaque effects. Both authority flags remain false. Strict anchor/status checks
precede audited-root I/O; full recomputation replaces stale class evidence.

### Acquisition

One project, actual JSON config/extends/include/exclude/options, and installed
in-root regular-file dependencies. Acquisition streams every file into a hash/size
inventory using 64KiB chunks without retaining all input bytes. Compiler-requested
reads and anchor checks load inventoried content on demand and verify its captured
size/hash before use; a second full inventory checks the epoch. The new
`inventory.mjs` module participates in the producer digest. Default limits are
unchanged. Its lookup/canonicalization policy follows the pinned compiler's
host case policy; ambiguous case-folded inventory collisions are refused.
Internal TS matchFiles/createGetCanonicalFileName APIs are used deliberately under
the exact compiler byte pin, not as an unversioned API promise.

All input bytes/membership are fingerprinted, including unused/untracked files;
this conservatively invalidates more than a minimal dependency cache. `.git`
contents are excluded, with boundary sentinels: explicitly reading/enumerating
them is refused rather than silently claiming an empty input set.
Project references and plugins remain unsupported. Outside-root/absolute-config
lookups, unavailable packages, unsupported symlinks, special files, invalid UTF-8 compiler
inputs and budget exhaustion stay unproven. Local extends may traverse upward
within the supplied root, but cannot escape it.

With explicit `links: "in-root"`, relative file/directory link chains resolve
against the captured physical inventory, without expanding linked subtrees or
opening link targets. Link records include the original spelling's byte hash and
canonical target ID. Only leading `../` segments are supported; embedded dot/
parent segments, absolute/unsafe targets, cross-root targets, dangling targets,
link-resolution cycles and exhausted traversal budgets are refused. UTF-8 filename
bytes (including a leading BOM) remain literal. Canonical identity means path
identity, not hard-link/inode identity or a runtime class certificate.

The compiler's realpath hook and enumeration receive captured canonical IDs;
actual options are not rewritten. `preserveSymlinks` with links is refused.
Distinct Program source files mapping to one canonical ID are refused, not merged
into a singleton declaration. Case-folded collisions include link names. Complete
recomputation catches link retargeting even when old/new target bytes are equal.
Both default link refusal and the independent closure barriers remain intact.

The tool does not rewrite actual project options to manufacture closure. For
example, missing package boundaries and automatic ancestor type/lib lookups may
leave an otherwise compilable project unproven. Synthetic positives explicitly
supply package.json, types=[] and libReplacement=false in their own config.

Compiler probes that cannot be represented as safe in-root IDs (for example
`node:url` with baseUrl resolution) are refused before the host consults its
snapshot. `snapshot.refused_lookup_sha256` records sorted unique SHA256 hashes of
the normalized virtual-root probe strings, not paths or proof of file absence.
Nonempty refusal evidence adds `unsupported_lookup` and independently prevents
dependency/augmentation/resolution closure, even when ambient declarations or a
later safe paths-mapping fallback resolve the import without diagnostics.
Partial observations remain available; Props/class candidates stay unproven.

This is not a module-name blacklist: a virtual specifier that resolves through a
safe paths mapping without refused probes can remain observed. Actual unsupported
inventory names still fail acquisition; outside-root and .git barriers are
unchanged. The schema checks digest/order/reason/closure consistency before root
access, and full recomputation rejects forged or removed refusal evidence.
Opaque hashes are not anonymization; packets remain potentially private data.
This repairs the inherited whole-packet rejection documented in the Props/class
readout. See the lookup-packets readout for merged-base controls and real census.

Two snapshots detect observed changes, not adversarial change-and-revert races or
an atomic filesystem transaction. This is not an OS security sandbox. Mixed
filesystem policies/mounts and hostile concurrent mutation are unsupported. A
future authenticated proof consumer would require a stronger acquisition contract.

## Tests

```sh
cargo build --release --example project_membership_census
PRISM_TYPESCRIPT="$compiler" PRISM_CALLABLE_PROFILES="$profiles" \
  PRISM_MEMBERSHIP_NATIVE="$PWD/target/release/examples/project_membership_census" \
  node --test scripts/callable-observations/*.test.mjs
cargo test --example project_membership_census
```

`profiles` uses PR259's pinned react18/react19 layouts. Tests create their own
temporary installed-package projects, never install or modify an application.
See the same-date callable-lookup-packets spec/readout for RED captures,
full-project gates, evidence limitations and the separately approved next boundary.

## Separate project membership observations

`membership.mjs` observes the approved complete audit root and selected logical
config without changing schema20, the native loader or executable ownership.
Build the research native helper above from the **same checkout** first. Its path
is an explicitly trusted executable to run, not untrusted packet data. Its hash
binds reproduction; it is not an attestation of arbitrary third-party binaries.

```sh
native="$PWD/target/release/examples/project_membership_census"
node scripts/callable-observations/membership.mjs observe \
  "$compiler" "$project" tsconfig.json "$native" > membership.json
node scripts/callable-observations/membership.mjs validate \
  "$compiler" "$project" tsconfig.json "$native" < membership.json
```

Optional final arguments are the existing `default|installed` profile and
`reject|in-root` link policy, defaulting to `default reject`. They apply only to
this research invocation, not Rust owner acquisition. No installs, emits, config
plugins or project-reference traversal are introduced. Keep private raw artifacts
local; they contain source identities, inventory and existing compiler evidence.

`prism.project-membership/1` is always `authorizes_runtime_edge:false`:

- `status:observed` means reproduced observations, **not** complete semantic
  closure, successful parsing or eligibility. `payload.packet` retains the entire
  unchanged observer packet and all unresolved/effect/search/type/lib obligations.
- `selection` retains config-chain file/hash identity and byte anchors plus exact
  spellings for files/include/exclude. The existing compiler supplies roots; no
  replacement glob engine or flattened config is used. Duplicate JSON properties,
  unsupported config provenance, plugins/references and configDir selection
  interpolation are unavailable in this bounded lane.
- `program` partitions actual compiler members into descriptive repository,
  dependency and compiler domains. Separate fields record declarations, JSON,
  configured-root/native membership, actual native language support and parse
  counts. `inventory_aliases` lists audited links targeting a member/ancestor;
  it does not assert the compiler traversed any particular link.
- `native` is the actual full-root loader census and complete skip ledger, never a
  supplied subset. `native_status:incomplete` preserves parse-error and refusal-skip
  evidence. Ignored directories remain explicit; `observed` is not full filesystem
  coverage. Differences retain roots outside Program, Program outside roots/native,
  and native inputs outside Program, including unsupported Program suffixes/JSON.
- `status:unavailable` has `payload:null` and a bounded reason, not complete empty
  sets. Compiler/native/compiler acquisition shares the existing profile deadline
  and per-process output ceilings; final envelope must fit that same packet cap.

Program identities reuse the existing worker's byte-backed link/case
canonicalization. Native arrays use matching UTF-16 order. Both compiler runs must
agree exactly; native bytes must match that inventory, and helper/producer hashes
must remain unchanged. `parseMembership` only checks structure/consistency.
`validateMembership` independently reacquires and compares the entire artifact;
unavailable observations never validate as membership. Same-byte restoration can
reproduce an observation, not a live Rust proof/cache epoch. Existing non-hostile
filesystem assumptions apply; this is not an atomic snapshot or security sandbox.

Do not use the raw native helper without a complete bounded prior snapshot and
subprocess limits: it is an implementation component, not a standalone acquisition
authority. The helper's stdin IDs only classify parser-language support and never
select files for loading. No new navigation CLI/API/MCP flags or runtime consumer.
