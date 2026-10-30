# Slice 1 — exact caller read occurrences

Planning baseline: PR317 merge `9fb6c823e5fa8e6c07ace8c5b41abe9bc5acf090`,
tree `9b4de2b021d0a53fe4d07e50599f97d6c685e9ee`. Planning HEAD at
source review was `ef11cde08377b1256c04e38de5d330e575bae527`. This is an
implementation contract, not evidence that the repair is complete.

Independent architecture review round 1 of 2 closed with `REVISE BEFORE DISPATCH`,
WRONG 0 / SMELL 3. This spec incorporates its finite decisions. Implementation
review has its own cap of two rounds.

## Objective

Recover byte-distinct caller-side producer edges for repeated same-line reads of one
unambiguous simple binding. Preserve the already accepted exact CPG occurrence nodes and
argument-to-parameter edges from PR317. Add no new statement-order, loop, member, alias,
capture, callable-owner, or public query authority.

The accepted two-file fixture is the first behavioral RED:

`origin.js`, exact bytes with no trailing newline, SHA-256
`8b7218305f606ec9b081ab55f6ccd98b7f3bbbadd87f229bc7c8fd5841b7fecb`:

```js
export function item(input){return input;}
```

`app.js`, exact bytes with no trailing newline, SHA-256
`5457bf7b1248b193da83bd293adc26e7ecdddb6018d5b2c075f48fae057583f1`:

```js
import {item} from './origin';
function outer(value){sink(value);return item(value);}
```

On the merged base, JS/TS/TSX each contain:

- caller parameter Def `value` at `[46,51)`;
- raw caller Uses `[58,63)` and `[77,82)`, with a duplicate raw row possible for
  the later call argument;
- exact CPG Variable nodes for both Use spans;
- only caller `Def[46,51) -> Use[58,63)` with
  `NameOnly(CfgIncomplete)`;
- the genuine `Use[77,82) -> callee input Def[21,26)` Exact boundary and the
  callee-entry-to-body edge already accepted by Slice 2.

Desired: one caller producer edge to each authenticated Use span. The later edge gets its
own RD classification and is `NameOnly(CfgIncomplete)` in this one-line fixture. No raw
duplicate amplification, copied label, all-to-all fanout, or precision promotion is allowed.
A separate multiline fixture must demonstrate that the same path can retain an independently
classified `Exact` label.

The preflight PASS probe only characterized current output. It is not fail-first evidence.
The implementer must add the desired missing-edge assertion, apply that identical test patch
to the unchanged merged base in the same environment, and capture the behavioral failure
before production edits.

## Load-bearing architecture

### Keep the legacy identity and graph intact

Do not change `VarLocation` Eq/Ord/Hash, `FlowEdge` equality, existing DFG `edges`,
`labels`, `forward`, `backward`, public query signatures, or first-wins representative
behavior. These are a legacy quotient identity:

```text
(file, function, function_start_line, source_line, AccessPath, access)
```

Bytes remain payload in `VarLocation`, but they do not become its equality identity.
Existing kill/alias equivalence, `DefId`, `DefSite`, solver caps, and line-granular
statement/CFG semantics remain unchanged.

### Add one internal exact producer view

Add an internal serializable exact endpoint key with byte-bearing Eq/Ord/Hash:

```text
(file, function, function_start_line, source_line,
 AccessPath, access, start_byte, end_byte)
```

Add an internal exact edge key made from two exact endpoint keys and store one confidence
per exact edge. Classify every admitted candidate pair and check overlap consistency, then
persist only authenticated supplemental exact pairs absent from the legacy exact endpoint
population. This is primary producer state persisted alongside the DFG. It is not derived
from CPG nodes and must never be keyed by `VarLocation` equality. Use it for exact duplicate
elimination and labels before the legacy RD map reduces same-line locations.

The exact state may be named differently, but it must have these properties:

- crate-internal; no public query or wire API expansion;
- each supplemental edge carries authenticated nonzero source endpoints and its own
  classifier result;
- exact duplicates collapse, byte-distinct edges do not;
- deterministic ordering and serialization;
- complete `empty`, build, subset, file removal, merge, incremental and cache lifecycle;
- file-partition removal drops every exact edge touching the removed file;
- merge cannot retain stale exact facts or borrow confidence from the legacy map;
- supplemental fact count equals distinct newly materialized pair count for a fixed input.

### Producer and RD order

Build a callable-local byte-bearing rvalue index once. Reuse the execution-owner filtering
in `rvalue_identifier_spans_on_lines`; do not scan source once per edge. Add a crate-internal
span-bearing reference helper or indexed join. Keep the public line-returning
`find_path_references_scoped` API and its member semantics unchanged.

A line match alone is insufficient. Authenticate each candidate token by file, named owner,
owner start line, simple path, Use access, nonzero bytes, identifier role, and existing binding
evidence. Deduplicate identical raw observations before producing exact edges.

Run legacy and exact candidates through one solved reaching-definition state for the function.
Capture each exact result before `reaching.rs` reduces labels into the byte-free legacy map.
Never run the full solver once per exact edge. The earlier legacy label is not authority for
the later edge.

### CPG adapter

Give Step 4 one internal adapter over legacy edges and admitted exact labeled facts.

- Resolve every nonzero JS/TS/TSX endpoint through the existing exact occurrence index.
- If a legacy edge and exact fact name the same exact byte pair with the same confidence,
  emit one CPG edge.
- If the two views disagree on the exact pair, refuse and diagnose the new binding expansion
  while preserving all legacy rows and labels. Do not combine confidences, promote/demote a
  retained legacy row, or emit parallel label variants.
- A missing, zero-width, forged, ambiguous, or nonunique exact endpoint refuses only the new
  edge; valid legacy output remains.
- Never enumerate all CPG nodes beneath a legacy key to manufacture producer fanout.

Contains edges, location registration, Step5b argument binding, Step5c/legacy queries and
the exact occurrence index remain as accepted in PR317.

## Admission boundary

The first implementation applies only to JavaScript, TypeScript and TSX.

An admitted target must be a bare simple identifier within an existing unambiguous named
callable owner and have exactly one real source Def:

- an already-supported plain parameter entry; or
- a unique non-aliased local declaration on a strictly earlier source line than every newly
  expanded read.

Parameter entries retain their existing entry-line representation, so the accepted one-line
parameter fixture is admitted even though its reads share the function-start line. Its own
classifier result remains `NameOnly(CfgIncomplete)`. A same-line local declaration/write is
not admitted.

Use a conservative whole-callable gate for control-flow and owner uncertainty. Permit only
ordinary sequential declarations, expression statements/calls and returns. Refuse new exact
expansion for callables containing branches, loops, conditional/short-circuit evaluation,
try/switch, nested callables/captures, unknown/recovery syntax, reflective `eval`/`with`, or
ambiguous owner identity.

Use per-binding uniqueness inside an otherwise eligible callable. Refuse the target binding
when it participates in writes after its entry/unique declaration, update or compound
assignment, redeclaration, shadowing, destructuring, aliasing, or member access. Default,
optional and rest/destructured parameter admission does not expand in this slice. Fields and
members remain on their existing route.

A refusal means only “no new exact producer expansion.” It does not delete old DFG/CPG rows
or claim that the existing graph has no flow.

## Source seams

| File / anchor | Contract |
|---|---|
| `src/ast.rs:7388–7488` | Existing byte-bearing rvalue collection and execution-owner filtering |
| `src/ast.rs:8310–8410` | Reference lookup has captured nodes for shadow checks before returning lines; add an internal span path if needed |
| `src/ast.rs:8507–8522` | Preserve public line-returning helper and intentionally different simple/member behavior |
| `src/data_flow.rs:19–79,90–123` | Keep legacy identities; define internal exact endpoint/edge/confidence state |
| `src/data_flow.rs:128–214,230–284,619–673` | Complete empty/build/subset/remove/merge/adjacency/serialization lifecycle |
| `src/data_flow.rs:451–584` | Raw rvalue spans, preferred legacy Use, parameter/local/alias edge producers |
| `src/cpg/reaching.rs:32–42,113–249,298–310,486–504` | Reuse one solver state; classify exact facts before legacy reduction; do not change kill/binding rules |
| `src/cpg/reaching/scope.rs:871–892` | Real nonzero use bytes only; no zero-width uniqueness authority |
| `src/cpg/build.rs:629–809,873–950` | Exact endpoint validation and deduplicated Step 4 union |
| `src/cpg_cache.rs:216,388–496,576–697` | DFG serialization and genuine predecessor-cache refusal |
| `src/cpg/same_line_occurrence_tests.rs` | Preserve accepted exact nodes, boundaries, ambiguity refusals and warm parity |
| `tests/ast/dfg_test.rs`, `tests/ast/dfg_label_test.rs` | Pin legacy identity/labels and exact producer state |
| `tests/integration/dfg_label_store_test.rs` | Full/subset/incremental/cache label lifecycle |

The implementer may add a focused integration test file and one crate-private test module.
Do not spread production changes outside these seams without a written stop/design decision.

## Mandatory proof population

Assertions compare primitive tuples including file, owner, owner start line, source line,
path, access, bytes, confidence/doubt and multiplicity. `FlowEdge ==`, `VarLocation` set
membership, or `labels.get` using a later byte payload cannot prove exact identity.

### R01 — authentic one-line NameOnly RED

For the exact accepted two-file fixture above, across JS/TS/TSX:

- base must fail the new assertion because the later caller edge is absent;
- candidate has exactly the earlier and later caller endpoint tuples once each;
- both caller labels are independently observed `NameOnly(CfgIncomplete)`;
- later boundary remains Exact and callee-entry/body label remains unchanged;
- raw duplicate observation of the later argument does not duplicate any exact fact or CPG
  edge.

### R02 — Exact parameter and local positives

Use these exact no-trailing-newline fixtures. The parameter fixture is 99 bytes,
SHA-256 `286faa2b12b01b5c06f5435f13576754c418ef21140571b3536f04f9ff25ab10`:

```js
import {item} from './origin';
function outer(value){
prepare();
sink(value); return item(value);
}
```

The local fixture is 85 bytes, SHA-256
`5c98fba3895da5d92840cd02ddcd7d0d8f6725f537529da653a7e37980416250`:

```js
function outer(){
const value=source();
prepare();
sink(value); return item(value);
}
```

Across JS/TS/TSX, base characterization is: parameter Def `46–51`→earlier Use `70–75`
Exact, with direct later Use `90–95` absent and later Use `90–95`→callee input `21–26`
Exact; local Def `24–29`→earlier Use `56–61` Exact, with direct later Use `76–81`
absent. These passing observations characterize the gap; RED is the new exact assertion for
the missing later producer edge. Candidate must retain the first and downstream edges and add
the independently classified Exact producer edge to the later read exactly once. Comments,
filename labels and trailing newlines are outside both source literals.

### R03 — owner, spelling and raw-row identity

Cover same-spelling named owners at distinct start lines, duplicate raw Use observations,
Unicode before the token, comment shifts, and exact call-site/parameter ordinals. No
cross-owner edge or duplicate amplification.

### R04 — discriminating mutations and forged identity

- Change only the later identifier to a different same-width binding: its old edge disappears
  and earlier flow remains.
- Separately reorder and shift source; recompute every affected byte expectation.
- Wrong start, wrong end, inverted/zero range, wrong path, wrong access, wrong owner, and
  ambiguous duplicate owner each refuse the added edge.
- Retain the later CPG node and Step5b edge while deleting its exact producer fact in a
  test-only graph: Step 4 must not recreate caller flow.
- For a coupled exact edge→confidence map, remove the later entry: retaining its CPG node and
  Step5b edge must not reconstruct producer flow or borrow a legacy line label. If the
  implementation separates candidate edges and labels, a missing exact-key label refuses the
  edge.
- Inject a conflicting supplemental confidence for an exact pair already represented by the
  legacy view: refuse every new fact for that target binding, retain all legacy rows/labels,
  and leave a separate unrelated eligible binding unaffected.
- Exercise wrong exact-key association and absent fact/label. Do not require detection of an
  arbitrary valid confidence injected into a private record without an independent witness.

### R05 — preserved exclusions

Capture complete old primitive rows and prove equality for:

- write before/between reads, compound/update assignment, redeclaration and same-line local
  assignment;
- alias, destructuring, shadow and nested capture;
- branch/conditional/short-circuit, loop and try/switch;
- simple and asserted member paths;
- recovery/unknown syntax, reflective `eval`/`with`, zero-width and nested/ambiguous owner;
- optional/default/rest parameter controls;
- at least one non-JS legacy DFG case.

No case may pass by deleting an existing row, changing a legacy label, or broadening its
owner.

### R06 — legacy and lifecycle compatibility

- Pin `VarLocation` equality, ordering and hash equivalence for two byte-distinct payloads.
- Pin legacy DFG `edges`, `labels`, `forward`, `backward`, reachability and first-wins query
  rows byte-for-byte against base for the fixed fixtures.
- Compare full build with subset+merge exact facts and complete CPG tuples.
- Directly call `remove_files` with one deleted/changed and one untouched file, then
  merge/rebuild. The deleted file contributes no stale supplemental fact; the untouched file
  and complete CPG output remain identical.
- Run caller-only incremental epochs: one read → duplicate same-line reads → reordered reads
  → moved line → one read → restored. Each epoch equals fresh; untouched-file rows remain
  identical; revoke/restore has no stale exact fact.
- Verify deterministic repeated and thread-count builds where the existing harness permits.

### R07 — genuine cache and bounded cost

- Current base is CPG cache 96 and navigation cache 53.
- Produce a genuine base96 cache from unchanged merged source and prove a warm base Hit.
- Because the DFG serialized primary state changes, advance CPG cache exactly once to 97.
  Navigation remains 53 unless actual navigation facts change and a separate justification
  is reviewed.
- Candidate directly rejects the genuine96 cache, rebuilds, then produces a real Hit.
- Fresh/rebuilt/warm outputs match for exact facts, legacy views, complete graph nodes/edges,
  labels, Contains, Step5b and queries. Do not simulate a version byte.

Use a fixed synthetic JS/TS/TSX population at 10, 100 and 1,000 eligible callables. Report
cold-build median after one warm-up (five measured runs), graph node/edge delta, exact-fact
count, cache bytes and peak memory if available. This is a bounded engineering measurement,
not real-corpus recall. Node delta should be zero because PR317 already materializes the
occurrences; added Step 4 edges, persisted supplemental facts and authenticated
missing-later-pair count must be equal.
Any superlinear per-edge source scanning, unexpected node growth, duplicate multiplication,
or greater than 25% median slowdown at 1,000 callables parks the change for review rather
than inviting unbounded optimization. Historical real JS/private corpora remain
`input-blocked` and are not substituted.

## Implementation order

1. Snapshot base/status/toolchain and write a hypothesis/probe/result log. For every
   diagnostic, state expected result, falsifier and a plausible alternative first.
2. Stage base-compatible public desired-flow assertions and preservation tuples from R01–R05,
   then apply that identical patch to an unchanged merged base. Capture the complete RED
   population and preservation controls. A compile/filter/environment failure is
   inadmissible. Preserve every failing row rather than stopping at the first aggregate
   mismatch.
3. Add the internal exact endpoint/edge types and complete DFG lifecycle without changing
   legacy identities or public queries.
4. Add the callable-local span index and conservative admission gate. Produce exact candidates
   from authenticated tokens; no source-wide scan per edge.
5. Classify exact candidates in the existing RD solve before legacy label reduction.
6. Add the Step 4 union adapter with exact-pair deduplication and conservative confidence.
   Only after the new internal seam exists, add identity/label/adapter fault-injection tests
   and candidate mutation evidence; their pre-change compile failure is not RED and they do
   not substitute for the public base RED.
7. Complete R06 incremental/parallel parity, then bump cache 96→97 and complete R07 cache/
   cost proof.
8. Freeze source and manifest for independent review. Review cap is two. One diagnosed
   environmental retry per gate class; enumerate first-error-only populations before repair.
9. Run full gates on the accepted frozen source. Do not rebaseline or expand scope to make a
   gate green.

If the chosen implementation requires changing global `VarLocation` equality, the public
DFG/query contract, kill/alias equivalence, general same-line writes, loop/member semantics,
or unsupported owner inference, stop and return to architecture review.

Target at most 700 non-test production changed lines across the named production seams
and at most 1,600 total source/test changed lines, using table-driven fixtures/helpers. These
are slice-size guardrails, not correctness evidence. Stop for controller re-slicing before
review freeze if the estimate or actual diff exceeds either budget. Minimal classifier-result
factoring in the top-level `src/cpg/reaching.rs` is authorized when it reuses the single
existing per-function solve. Within the `src/cpg/reaching/` subdirectory, only test modules are
pre-authorized; production binding/kill modules remain excluded unless a specific internal
helper move is separately justified.

## Verification and evidence

After focused GREEN and independent review, run and report exact totals for:

- full default Rust suite;
- full MCP suite;
- widest owner/compiler audit with every exclusion and same-environment base controls;
- all examples, Clippy, rustfmt and git diff checks;
- active Node/compiler authority population using pinned inputs;
- Python evaluation and real-binary supplements using the pinned Python environment;
- an immediate fresh `cargo build --release` before each Tier-A matrix and quick command,
  passing the actual CLI `--sut-bin <frozen-path>` (or a verified command-builder equivalent)
  with `--allow-stale-sut` only for that fresh build. Freeze the corpus/worktree independently
  or hold a coordination lock throughout quick.

No full multi-corpus run or rebaseline is authorized. If compiler profiles, Python/Tier-A
environment, historical custody inputs, or other required inputs are unavailable, report the
exact excluded population and reason. A timeout or dependency refusal is `INCOMPLETE` or
`INVALID` as produced, never a pass. Preserve raw logs outside the repository until complete,
then copy compact receipts/manifests only.

At each stable point refresh the handoff, source manifest and hashes. The controller owns
commits, publication and merge. No push, PR, merge, adoption or cleanup follows from local
acceptance without separate authority.

## Claim limits

Success proves producer-authenticated exact caller endpoints for the admitted simple-binding
domain with existing confidence. It does not prove:

- a global occurrence-aware public DFG/query API;
- byte-ordered reaching definitions for same-line writes;
- loop-carried or member-read provenance;
- alias, capture, shadow or control-flow expansion;
- optional/default/rest/destructured parameter expansion;
- new caller/callee/receiver/compiler ownership;
- real-corpus recall, value or accuracy improvement.
