# CommonJS producer export-object barriers

Historical third-increment readout. The [terminal/capture follow-up](2026-09-11-cjs-terminal-proof.md)
repairs the deferred nested-name defect and preserves rejected duplicate-set names
as refusal markers. The results and remaining work below describe032e1824.

Base: `c3d110ef`; third local increment bundled with `4ffe55bf` and `c3d110ef`.
No publication performed. This is a prerequisite repair, not forwarding admission.
Original matrix remains 18 Supported, 14 Gap, 22 Refused and one JS-supported case
with a TS/TSX parser gap.

## Source-backed diagnosis

WRONG: the old collector retained `exports.item = origin` while skipping later
expression/computed/reflective/nested writes or export-object aliases. For example
`const alias = exports; alias.item = () => 0` changes the exported value but the
importer still received Exact origin. Whole replacements could retain discarded
names; unsupported object properties could hide overwrites. Raw-fact and resolved
target fixtures reproduce this, separating producer loss from global fallback.
Decoy functions are present; negatives forbid every Exact target, not just origin.

Fifteen local Node wrapper controls (one safe, fourteen mutations) independently confirm
the export/module/this alias and replacement mechanisms. They model wrapper
execution only, not loading, caching or cross-module mutation. Prism navigation
reported 43 stale paths; current source verified the export resolver and both
resolution/navigation consumers. No LSP evidence claimed.

## Implemented contract

Account for exact ambient occurrences only in simple top-level CJS export
assignment left-hand sides. Unaccounted module/exports uses, aliases, escapes,
computed/reflective/nested writes, eval/with, this, wrapper arguments, undecoded
escaped identifiers/properties and parse recovery revoke temporary CJS facts.
Module/exports occurrences with proven closer lexical bindings do not themselves
revoke ambient exports. Existing syntactic whole-replacement refusal remains
conservative even under a nested shadow. Object replacements require unique keys and identifier
values; methods, accessors, spreads, computed/escaped keys and prototype setters
refuse. Multiple replacements, replacement/member mixes and duplicate CJS exports
discard the entire temporary CJS set. These whole-file barriers are conservative.

Collect ESM separately: discarding unsafe CJS cannot poison independent ESM facts;
a single safe CJS/ESM same-name collision still uses conflict-aware insertion.
Direct member/object/default exports and arrow/function-expression local targets
retain their independent contracts. Skipped-expression telemetry is preserved.
Cache versions: CPG88 / nav49. No new forwarding, class/receiver, closure, React.FC,
package-resolution or react-scripts authority.

## TDD and value

Initial base: **8 passed / 28 failed**; first candidate: **36 passed**. Final
byte-matched `cjs_` selection: **44 passed / 45 failed** on detached base,
**89 passed / 0 failed** on candidate: 27 existing controls and **62 new tests**.
Three JS/TS/TSX epoch tests verify actual argument-use to parameter-token Def
flows through nine producer states, exact origin-only changed sets, full versus
incremental parity, and raw-fact serde round trips with export re-resolution.

Matched **444 synthetic fixture/language/build observations**: **234 Exact
revocations**, **12 ESM Exact restorations**, **198 unchanged**, zero other target
changes. These are not 234 demonstrated wrong-target fixes: conservative refusals
also reject harmless duplicate/escaped keys, unrelated writes and prototype use.
They are not unique real sites or corpus recall. The adjacent baseline receipt
pins source/test/log hashes; full deltas remain in the local evidence directory.

## Review and verification

Two independent rounds: design, then source. Round2 found duplicate temporary CJS
conflicts still erasing ESM facts. This behavior is also present on base, not a
newly introduced runtime regression. At the declared cap it was closed/enumerable:
two RED tests on the pre-fix candidate, one-condition merge-guard repair and removal
of conflict propagation. Full default/MCP then exposed one compatibility defect:
the guard rejected unrelated nested ambient parameter shadows. The same test
passes on unchanged base; pre-hardening full runs retain 4347/4540 passes plus
one failure and one ignore each. No test was re-baselined.

One bounded hardening/verification extension was disclosed: preserve closer
lexical bindings and add explicit escaped-identifier/wrapper-arguments refusals.
Eight new hardening tests failed on pre-fix candidate; all pass after correction,
as do all 62 module-binding tests including the unchanged compatibility control.
The independent targeted-delta review approved W0/S1: conservative rejection of
nested ordinary functions' own arguments can reduce recall. Established Local
and snapshot gaps remain out of scope. No further source edits after approval.

Final-source Rust: default **4356**, MCP **4549**, MCP + detached-owner-audit
**4572 passed**; zero failures and one existing reserved-spec ignore per run.
Examples: **32 passed**. Node: **786 passed**, no skips; authority: **40 passed**; Python:
**940 passed**, one explicit live-adoption skip. Final frozen CLI/MCP/example
hashes are unchanged across helper gates. Fresh-release Tier-A matrix: **159/159**.

Final Tier-A quick reached its five-minute cap (300044ms) without a verdict;
it is **incomplete, not green**. No full multi-corpus run was requested. Its
snapshot and logs are retained locally. Three historical imported-props helper
tests remain unavailable because their pinned real-sites.jsonl input is missing;
only that test file was excluded. No real-corpus recall conclusion is claimed.
Pre-hardening gate outputs and binaries are retained separately, not substituted
for the final-source checks.

## Next proof requirements

Before forwarding, prove the terminal RHS binding and its capture-time value.
The existing CJS Local route still lacks unique module-level declaration proof:
`function outer(){function f(){}} exports.x=f` can offer the nested function to
file/name resolution although no module-local f exists. This independent,
pre-existing WRONG from source review remains open, not downgraded to a smell.
Fix with RED tests while preserving direct arrow/function-expression exports.
Initialization order and reassignment need capture-time treatment: post-export
rebinding is not mutation of the property's already captured value.

Then prove require-time consumer ownership: other consumers mutating shared
exports, cache changes, dependency cycles and partial initialization. A const
destructured local proves none of these. Whole-module/member-expression forwarding
and extra aliases stay separate. This repair does not claim complete CJS soundness.

Evidence: `/private/tmp/prism-cjs-barrier-CBIIas`.
