# S10 — first production consumer authority contract

Status: design contract only. No production reader, subprocess, CLI option, default
dependency, proof constructor, cache migration or Exact edge is implemented here.
The standalone callable observer and validator still authorize no class/runtime edge.
Implementation requires a separately approved bounded slice and the RED gates below.

## Outcome and scope

The first future consumer may improve a narrowly supported static receiver owner
only after authenticating compiler observations and deriving a new internal proof.
Its input scope is direct TS/TSX variable annotations on arrow/function expressions,
their original implementation/parameter, and supported directly owned nested member
calls. Explicit implementation-parameter annotations remain terminal under existing
rules. Imported contextual annotation identity is source-backed; React/FC names are
never an authority shortcut. Existing imported-Props recovery is a separate route
and must retain its own proof and effect barriers.

Out of first scope: JS consumer assertions/JSDoc, inferred returns/hooks, wrapper
calls including memo/forwardRef/styled, arbitrary type evaluation, constructor flow,
declaration-to-runtime package matching, inheritance/structural ambiguity, ambient
runtime declarations, and any expansion of unsupported ownership/effects. A later
implementation must enumerate its smaller accepted shapes explicitly; this contract
does not grant all shapes for which the observer retains candidate anchors.

`Exact` means Prism's static source call-graph confidence. It does not promise the
identity of a runtime object, dynamic override selection, execution or reachability.
The new route must preserve existing conservative executable-owner rules, not claim
that a TypeScript class instance type proves a concrete runtime object. A declaration
file class or printed receiver type is not executable member ownership.

## Authority pipeline and refusal

1. A trusted opt-in invocation supplies project root, config, acquisition profile,
   link policy and compiler independently of serialized packet fields. It binds
   current pinned producer/compiler bytes and compiler-library inventory. No packet
   value selects roots, installs packages, runs project scripts/plugins or changes
   configuration. Existing bounded acquisition and quiescent-root requirements apply;
   two snapshots are not an adversarial atomic transaction.
2. Parse the exact versioned schema before root I/O. Fully reproduce every source,
   lookup, config, entry and binding fact under those independently supplied options.
   Structural validity is not source authentication and a same-shaped packet is not
   authority. Budget/unavailable/stale/unknown outcomes are refusals, never partial
   positive proof. A current producer cannot validate a historical packet as current.
3. Require the supported fixed semantic-closure policy to be complete. Keep every
   config/entry/path/type/lib/diagnostic/outside/refused/unproven-row barrier. Preserve
   distinct asset inventory, filesystem resolution, Program membership and semantic
   provider identity. Do not discharge public react-scripts or a missing peer by
   observing an unrelated receiver. Receiver-local proof islands require new design.
4. Derive a NEW internal proof for the actual call. Recheck direct contextual
   annotation/implementation ownership, callable alias/generic provenance, exact
   parameter binding, required own Props property, instantiated class declaration,
   executable class/member ownership, lexical scope and all effect/write barriers.
   Reuse validated compiler identities, not rendered type names. Neither old
   `props_class.status=unproven` nor a retained candidate anchor can be relabeled as
   proven. Old packet status/closure/Props fields remain historical observations.
5. The ordinary resolver consumes only this internal proof at the shared resolution
   seam. Never store a compiler class guess in `pre_resolved_target`, which already
   has distinct ParameterCallback meaning. Preserve independent old routes; absence
   of opt-in evidence must not reduce their results. A refuted compiler candidate
   must not fall through to a name-only path claiming the same newly proven owner.

Proof construction is an architectural prerequisite, not a serialization feature.
The first implementation must publish exact constructor predicates before coding;
if executable owner/member evidence cannot be established, refuse that shape rather
than importing an ambient declaration as an executable target.

## Internal proof identity and lifecycle

The future internal type must be unforgeable outside its trusted constructor and
scoped to the current analysis session/epoch. Its authenticated content includes:

| Identity group | Required binding |
|---|---|
| Acquisition | caller-supplied root/config/profile/links; compiler/producer/library fingerprints; canonical host policy; full snapshot and config/options identity |
| Policy | supported schema, semantic policy, constructor and resolver ownership-rule versions; no caller-selected relaxation |
| Call | caller file and original byte digest; call, receiver, annotation, implementation and parameter anchors with exact UTF-8/UTF-16 ranges |
| Type provenance | actual defining declarations, aliases/generic substitutions, Props/property/class identities; same-Program binding and relevant census |
| Executable owner | source-backed class/member symbol and declaration identity plus existing duplicate, augmentation, shadow, write and opaque-effect barriers |
| Epoch | one session's source/dependency/config/lookup membership, including negative facts; no mixing genuine facts from different inputs |

Matching strings, equal call spans or even two independently genuine observations
do not permit splicing. The exact storage mechanism (opaque owner-held token or
equivalent scoped object) belongs to the implementation design; it must demonstrate
two-genuine-object substitution refusals, not just malformed JSON rejection.

Positive → different positive → unproven transitions replace prior state. Never
union old and new proofs. Dependency/config/augmentation or negative-lookup changes
invalidate a proof even if call bytes remain unchanged. Cache load must not restore
an obsolete positive after opt-in disappears, compiler/policy changes or acquisition
fails. No persistent positive cache exists in the observer today.

## Current production seams and integration obligations

The [source-backed foundation](../../eval/receiver-closure/2026-09-08-callable-production-foundation.md) records exact current line locations.
The later implementation must bind all of these consumers or stay unshipped:

- `ast.rs` receiver binding and contextual-parameter classification, preserving
  explicit annotations, receiver writes and terminal materialized identities.
- `js_ts_props.rs` and `call_graph.rs` existing imported-Props proof collection,
  installation, clearing and replacement; their indexed-source effect barriers.
- Both full and subset CallSite construction, then `resolve_call_site_full`.
- Served navigation session callers/callees and confidence filtering, not only an
  internal classifier or an overlay available to one query path.
- CPG construction/trace consumers of singleton Exact. Mixed/multi/NameOnly results
  cannot acquire exact interprocedural return flow through the new route.
- Incremental replacement and persisted CPG/navigation sidecars. Current versions
  are CPG77 and navigation-call45; S10 changes neither. The implementation must
  select an explicit migration/rejection strategy under then-current cache formats.

## RED-first acceptance matrix for a later implementation

Capture every behavioral RED on the exact production base in the same environment.
Passing existing refusals are compatibility controls, not new-route RED evidence.

| Family | Required positive / negative evidence at public seam |
|---|---|
| Intended gain | one bounded contextual receiver absent on base gains only its intended executable member; supported existing explicit/local and imported-Props routes unchanged |
| Ownership | duplicate/merged/augmented or shadowed declarations; explicit any/unsupported parameter annotation; optional/union/unknown/structural/inherited class/property; receiver/member writes; unsupported arrow-field scope/effects all refuse |
| Closure | unrelated unresolved module/path/type/lib, uncertain automatic entries, noResolve, plugins/references, outside/refused/eventless-null row and absent-Program target prevent proof |
| Custody | two genuine A/B proofs with same names/spans but different member, alias target, source/config/epoch reject every mixed substitution |
| Tampering | malformed/unknown schema, forged source hash/ranges/UTF encodings, option/policy drift, caller-root substitution and budget failures reject before inappropriate I/O/authority |
| Lifecycle | dependency remove/restore, new duplicate/augmentation, equal-byte alias retarget, configuration/compiler/library change; positive/different/unproven sequences remove obsolete edges |
| Consumer parity | full/subset and fresh/incremental/cache roundtrip yield equal confidence/targets; served callers+callees and CPG traces agree; no stale Exact dataflow after refusal |
| Nonclaims | wildcard asset present/absent, ambient class and unrelated runtime package are never proof of executable target; no React.FC spelling or dynamic-dispatch guarantee |

Run complete observer, Rust default/MCP (including doctests), receiver-helper and
authority-profile suites. Production call-resolution/navigation/CPG/AST changes also
require immediate same-worktree release rebuild, Tier-A matrix and quick controls.
Report regressions/flip candidates; no silent rebaseline or full multi-corpus run.
Public-seam tests, not internal green alone, establish the intended integration.

## Value checkpoint and follow-on recommendation

The fixed public Program remains incomplete under strict policy; private read-only
reproduction also remains incomplete. Type-source eligibility counts are not receiver
recall and no receiver edges have been added by this sequence. Preserve the owner's
unresolved react-scripts disposition. Do not manufacture a complete benchmark by
installing, excluding or waiving it.

Recommended next decision after this contract is a bounded executable-owner proof
design with one constructible direct contextual fixture and terminal negatives,
followed by implementation only if the owner accepts the demonstrated scope/value.
Canonical proof of the audited out-of-Program JavaScript cases is a separate option,
not permission to weaken whole-Program closure in the first consumer.
