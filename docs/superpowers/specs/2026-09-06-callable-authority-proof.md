# Compiler-backed callable authority — design and fixture slice

Approved after PR258 merge835c4fbc. Scope: design, pinned compiler characterization,
negative fixtures, and read-only real-source audits. No runtime React.FC expansion,
proof consumer, new default compiler dependency, CLI flag or cache-version change.
Three SELF-PASS rounds, NOT INDEPENDENT. No subagents, full multicorpus or rebaseline.

## Claim and correction

Source-defined contextual types from PR258 remain bounded and valid. External
callable authority requires an independently established compiler-program context.
An FC spelling, package version, type assertion, symbol-at-call result, or clean
diagnostic list alone is insufficient. In particular, a consumer-side assertion on
an imported function does not contextually annotate that function's implementation.

## Plan

1. Pin TypeScript5.9.3 and React declarations19.0.10, plus a separate18.x fixture
   profile; verify package integrity and transitive declaration inputs. Do not
   install application dependencies or execute application scripts.
2. Characterize direct contextual annotations, alias/namespace provenance, explicit
   overrides, augmentation/overloads, missing packages, path substitutions, any,
   optional receivers and assertions. Preserve current Prism non-Exact guards and
   independent explicit/local positive controls. These intentionally pass on base;
   no RED-first production claim. New audit tooling gets failing regression controls.
3. Specify non-authorizing observation format and future producer/consumer proof
   obligations, including exact source/UTF-16↔UTF-8 mapping and cache replacement.
4. Audit pinned public receiver sites and an owner-provided application read-only.
   Keep private source, paths and detailed reports outside the public repository.
5. Run compiler/audit tests and full Rust default/MCP suites; confirm src/ unchanged.
   Tier-A runtime gates not triggered by docs/test-only changes; carry the existing
   baseline-invalid limitation without presenting inherited results as fresh.
6. Snapshot evidence, refresh handoff, commit/push/PR under standing authorization.

## Architecture, proof format and acceptance

### Authority is a chain, not a React spelling rule

Proposed future sequence (not implemented or authorized by this document):

`immutable configured TS Program → contextual declaration observation → verified
source/class identity → existing Prism member/write proof → served projection`

Each arrow is a separate obligation. A later consumer must not bypass an earlier
failure by retrying spelling, assignment, global-name or cached-positive fallback.
React annotation-style advice is irrelevant to this analysis contract: analyze the
actual annotation when present, without recommending its use or assuming its shape.

1. **Program custody.** An opt-in, read-only producer uses an explicitly supplied,
   byte-pinned compiler and installed declarations. It loads the actual tsconfig,
   extends chain, project references, include/exclude membership, compiler libraries,
   package metadata/exports, paths/baseUrl, typeRoots/types and effective options.
   It never installs packages or executes project scripts, plugins or configuration
   code. Missing dependencies, unsupported references/resolution, unstable reads or
   exhausted budgets yield `unproven`, never a guessed default program. Dependency
   availability is not inferred from a lockfile. Untracked inputs and negative
   module lookups matter too; Git HEAD alone does not identify a program.
2. **Declaration origin.** Resolve import aliases and namespace members to original
   declarations, retain the alias chain, all merged declarations, and instantiated
   call signatures. A path-mapped fake `react`, local FC or React namespace has no
   authority from its spelling. A separately proven local callable route may still
   work under existing rules; a homonym is not inherently an invalid TypeScript
   type. Package version alone cannot certify declaration bytes or augmentation
   closure. `skipLibCheck` and zero diagnostics are not closure proofs.
3. **Implementation context.** Anchor the annotation, arrow/function expression,
   parameter binding and call in the same source snapshot. Only the direct
   implementation-context route is a first-consumer candidate. An explicit
   parameter annotation is terminal, including explicit `any`; do not fill it from
   an outer contextual signature. Reject multiple/ambiguous call signatures,
   unsupported generic substitution, conditional/intersection/union transformations
   and merged callable declarations in the first bounded consumer. The producer
   may report these observations without accepting them.
4. **Receiver origin.** Relate the selected required own property of the effective
   parameter type to a declaration-backed class identity that Prism already
   supports. Preserve defining-file/import provenance and class/member spans.
   A structural interface method, return type string, or method symbol reported at
   a call is not class ownership. `any`, `unknown`, optional/nullable receivers,
   unsupported inheritance/merging and competing owners stay unproven. Compiler
   assignability is not a promise about the runtime object or overriding methods.
5. **Prism proof and projection.** Existing duplicate, lexical shadow, write,
   member-ownership and full/subset barriers remain mandatory. Compiler observations
   may eventually supply a candidate receiver identity, not an asserted edge.
   Future persistence must carry verified provenance into every consumer, with
   full/incremental/round-trip/sidecar parity and an explicitly reviewed cache
   version transition. Nothing in this slice changes CPG77/navigation45.

Assertions of existing/imported functions, `satisfies` applied after a declaration,
hook/context return flow, JS wrappers, forwardRef/memo/styled components and imported
callable aliases remain outside the first proposed consumer. In particular, a cast
at a TS consumer does not annotate an imported JS implementation, even if its call
signature is accepted and `checkJs: false` leaves that implementation undiagnosed.

### Observation v0 proposal — deliberately non-authorizing

This is a design sketch, not a deployed wire schema. The fixture verifier emits
its own documented characterization report, not a conforming future proof packet.
Before implementation, the producer slice must freeze and test a strict schema.

| Field group | Required contents / interpretation |
|---|---|
| Envelope | `schema: prism.callable-observation/0`, `authorizes_runtime_edge: false`, producer version/build digest, compiler version and compiler/library digests |
| Snapshot | Opaque root ID; effective option/config digests; complete sorted input manifest with relative IDs, byte hashes and roles; root/project membership; dependency resolution decisions and failed lookup inputs |
| Closure | Explicit booleans for stable snapshot, dependencies, references, augmentation and resolution coverage; `observed` or `unproven`; enumerable reason codes; diagnostic codes and anchored locations |
| Source anchors | File ID, source-byte hash, node kind, half-open UTF-16 and UTF-8 spans for annotation, implementation, parameter, receiver and call |
| Declaration evidence | Alias-chain anchors, every callable declaration anchor, instantiated signature and parameter/property declaration anchors, explicit-parameter flag, candidate class anchor or null |
| Limits | Actual counts and configured byte/file/depth/time budgets; any truncation or budget hit makes the observation unproven |

Paths are manifest-relative identifiers, not authority to open arbitrary host
files. Preserve original bytes; no newline or Unicode normalization. Convert
TypeScript UTF-16 offsets against those exact bytes and verify the extracted text;
our emoji/CRLF fixture exercises differing byte/code-unit offsets. Future consumers
must reject unknown schema/version, absolute/escaping IDs, symlink escape, malformed
or conflicting anchors, mismatched hashes and incomplete closure before attempting
resolution. Producer booleans and hashes are claims to validate, not authentication.
An untrusted packet must never self-authorize an Exact edge.

Reason vocabulary to freeze next: `missing_dependency`, `unsupported_resolution`,
`incomplete_augmentation_closure`, `merged_callable`, `multiple_signatures`,
`explicit_parameter`, `assertion_not_context`, `unsupported_producer`,
`unproven_class`, `ambiguous_owner`, `stale_snapshot`, `budget_exceeded`.

### Cache and invalidation requirements for a future consumer

The cache key must include producer/schema/compiler/library bytes, effective
configuration, input contents AND membership, package resolution metadata, negative
lookups and complete augmentation closure. Adding a previously absent declaration
or dependency can invalidate an unchanged source file. File mtimes, package versions,
lockfiles or the verifier's `program_input_sha256` alone are insufficient.

Recompute and **replace** the epoch's candidate identity. Never union stale A with
fresh B, retain A after a positive→unproven transition, or reuse a proof merely
because a call's span is unchanged. Missing evidence must not disable unrelated
existing source-backed routes, but an explicit parameter or ambiguous observation
must not fall through into an inappropriate contextual route.

Required future acceptance tests (requirements, NOT executed consumer tests here):

- A→B class/import changes and reverse; full/subset/cache round-trip/served parity.
- Add/remove augmentation, overloaded signature, duplicate declaration, lexical
  shadow, receiver/member write; keep call bytes constant and require invalidation.
- Dependency removal/restoration; missing→present module lookup; paths/exports,
  config inheritance, references, compiler/lib and declaration-byte changes.
- Explicit `any` or decoy annotation with an otherwise valid outer FC context;
  assertions and post-declaration `satisfies` must not retroactively annotate bodies.
- Stale/malicious packets, out-of-root IDs, corrupted spans, emoji/CRLF, truncated
  manifests and budget exhaustion: fail closed, no stale persisted ownership.
- Exact-target consumers, callers/callees and conservation telemetry, not only an
  internal classifier; captured base RED before any production behavior change.

### This slice's acceptance and follow-on boundary

The committed corpus has20 cases, characterized under two isolated declaration
profiles and guarded through current Prism TS/TSX full/subset construction. Two
independent explicit/local source-backed controls retain Exact;18 cases retain no
Exact. These guards intentionally pass unchanged production code. New audit helper
regressions have captured RED→GREEN evidence for symlink refusal and byte-pinning.

The source census recognizes direct React-import-linked type references only; it
does not chase arbitrary aliases/barrels or resolve lexical shadowing. It reports
actual tracked bytes, dirty status, parse errors and refused leaf symlinks, and
sets `compiler_program_checked: false`. It is a trusted-worktree research helper,
not the secure configured-program producer or a complete module-input manifest.

Next recommendation: separately approve a bounded, non-authorizing configured-
program producer and strict packet validator with closure/invalidation fixtures.
Keep it outside Prism's default runtime and establish real installed-program
evidence before proposing an Exact-edge consumer. Hook and JS-wrapper analysis are
separate later slices; this design does not authorize either.
