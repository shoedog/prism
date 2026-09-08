# Configured/automatic type and lib entry observations

Base: merged PR279 `ca473cd198d3a906a9ba9b14402118a4bbcd952a`.
Published in [PR280](https://github.com/shoedog/prism/pull/280): implementation97b5524,
gate recordfbc2421; final publication note is docs-only.
Schema12/producer0.13.0 adds a separate `type_lib_entries[]` ledger without changing
existing reasons, closure bits or receiver observations. See the
[bounded contract](../../superpowers/specs/2026-09-07-callable-entry-observations.md).

## Source-backed design

Explicit types and discovered automatic types share TS5.9.3's retained name/mode
cache. Both use undefined mode (serialized null), even under NodeNext package
format, and kind8 inclusion binds the type name rather than an invented source or
option index. Duplicate names retain their option/enumeration occurrence indices.

Configured libs use normalized option filenames, cached `actual` targets (including
replacement/fallback), and kind6 option-index inclusion. Default libs instead use
indexless kind6 selection, not a lib cache inferred from their name. Actual target
SourceFile object membership is required. No second resolver or extra host probes.

Unprocessed rows do not prove suppression cause or an unsatisfied obligation.
Disabled traversal and incidental membership are distinct. Config effective values
are observations, not declaration-location or inheritance proofs. Config inheritance
locations, entry-channel closure policy and causal outside/refused lookup coverage
remain separate. Historical schemas10/11 stay audit-readable, never current-valid.

## Fixed public measurement

The exact-base replay reproduces PR279's packet SHA256
`25ea7dba031a0f4d9a1d6f3022d68a8e4ed6c8c495a93dabcc664150f51bbca5`.
A separately instrumented exact-base worker captures retained caches/inclusions;
its packet equals the base in every field except instrumentation's producer hash.
The read-only source audit reconciles all five rows and verifies all1,229 tracked
source files against manifest353187a6…cb6d33190901093549b61e28efeda60cbb.

| Kind | Effective configured name | Actual target |
|---|---|---|
| types | vitest/globals | project/node_modules/vitest/globals.d.ts |
| types | @testing-library/jest-dom | project/node_modules/@testing-library/jest-dom/types/index.d.ts |
| lib | lib.dom.d.ts | compiler/lib.dom.d.ts |
| lib | lib.dom.iterable.d.ts | compiler/lib.dom.iterable.d.ts |
| lib | lib.esnext.d.ts | compiler/lib.esnext.d.ts |

All five are observed; the fixed config selects no automatic or default entries.
Automatic/default behavior is covered by real-Program fixtures, not claimed as a
public observation. Every prior packet field is identical except schema/producer;
the new ledger is additive. This includes snapshot, reads/failed lookups, compiler,
scope, diagnostics, reasons, closure, module outcomes and receiver observations.
Validation reproduces the same **unproven** packet, not complete closure.

The139 source directives remain138 observed plus the unresolved react-scripts row.
All6,893 module outcomes,603 nulls,264 refused digests, outside lookup,30 callable
observations and53 nested calls remain unchanged. The14 module-literal gaps remain
separate. Both runtime/class authority flags remain false.

## Owner decision retained

**Keep react-scripts unresolved and document.** Preserve the pinned Excalidraw source
and negative case; no dependency installation/bootstrap, directive removal, shim,
substitution, exclusion or waiver. The owner regards it as obsolete CRA residue and
does not want its dependency graph installed. Package availability is not provider
presence in the fixed installation. Package age, lifecycle dates, React19 compatibility
and vulnerability claims are not independently established here or required by the
analyzer's refusal. This settles disposition without changing the application.
The [prior readout](2026-09-07-callable-type-lib-observations.md) records the decision;
its historical machine-readable audit remains unchanged.

## Verification and review

Initial23/23 tests failed on the exact base's missing entry channel. Expanded27/27
real-Program tests likewise fail on exact base and pass with this implementation;
these are observation-contract REDs, not false-closure repair claims. Thirteen
cache-double guards are separate edge controls. Final targeted40/40 and full
observer375/375 pass. Seven fixed-public audit controls accept the genuine packet
and reject omissions, target/origin/inclusion substitutions and react-scripts erasure.
Default Rust4017/0/1 and MCP4207/0/1 include doctests; the existing reserved SliceElem
case is the sole ignored test in each run. Helpers7/7, authority40/failures=[],
historical source-audit controls5/5, cargo fmt --check and git diff --check pass.
Evidence is in the [source/cache audit](2026-09-07-callable-entry-evidence.json);
the [gate record](2026-09-07-callable-entry-gates.json) captures hashes, totals and
the20,685,315-byte raw archive. Observer375/375 was rerun on committed97b5524;
publication custody is tracked in the
[handoff](../../superpowers/handoffs/2026-09-07-callable-entry-observations.md).

Hypothesis/probe/result log:

1. Expected only new rows/schema/producer to change; competing mechanism was extra
   host probing. Exact-base/current/capture comparison confirms all prior source,
   snapshot/read/lookup/receiver fields identical. No new completeness claim.
2. **WRONG test fixture, corrected:** es2022.full was used as a lib directive name
   to arrange incidental membership. Pinned libMap excludes it; exact base/current
   both report2726 and omit the file. A path reference creates the intended fixture;
   the corrected test confirms unprocessed default selection. Helper unchanged.
3. Full run initially368/371: three migration expectations needed schema12, the
   entry helper in producerHash, and removal of the new field when constructing a
   schema10 fixture. Exact-base controls pass2/2 and24/24. All three expectations
   corrected; final full run375/375 includes four additional entry regressions.

4. A helper invocation omitted PRISM_AUDIT_* variables:4 pass,1 setup failure.
   This was inadmissible evidence about the change. Corrected complete invocation
   supplies pinned upstream/slice/sites/source-repo and passes7/7; no code changes.

Two SELF-PASS rounds (NOT INDEPENDENT), cap2, no extension:

1. Cache/default selection, occurrence/inclusion ownership and mode review. The
   four added real-Program cases and cache guards pass; no open implementation WRONG.
2. Parser migration, producer hash, public/source custody, tampering and scope
   review. Full gates and historical controls pass; prior packet fields unchanged.
   **SMELL, bounded limitation:** unprocessed rows do not establish suppression
   cause or an obligation; config location and closure treatment remain deferred,
   explicitly documented. No open WRONG or new policy authority.

Source/compiler fallback under the LSP
skill was required because no LSP tools were exposed. No Rust resolution/navigation/
CPG/AST edits; Tier-A not triggered. No app scripts, install, React.FC expansion,
closure admission, Rust recall or production resolution changes in this slice.

Next: bounded causal classification of remaining outside/refused lookup probes,
keeping failed search candidates, required inputs and closure admission separate.
Raw captures: `/private/tmp/prism-entry-observations-iZY1n0`.
