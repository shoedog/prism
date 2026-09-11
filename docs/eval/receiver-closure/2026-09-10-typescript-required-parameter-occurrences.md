# Bounded TS/TSX required-parameter definitions

Base: merged PR308 `f369f20cdcd3463bae86b191bcdd8ec9efd8f6fd`.
Reviewed/tested production checkpoint: `6a53c66a99fe142f343ff6884801e26c0efa5bac`.
Subsequent closeout commits are documentation-only.

## Outcome

Simple required identifier parameters now supply actual-token definitions to
DFG and CPG consumers in TypeScript and TSX. This restores plain and asserted
member argument flow into real TS/TSX callees. Multiline signatures retain
identifier byte offsets while DFG definitions stay anchored to function entry.
Names and occurrences agree; cache version78 →79 invalidates incomplete graphs.

The extractor accepts only a direct identifier pattern in `required_parameter`,
with optional type annotation and comments. An all-child allowlist also checks
unnamed modifiers. Whole-list duplicates, recovery and escaped binding spellings
refuse new definitions. Optional/default/rest/destructured/decorated parameters,
parameter properties, erased `this` and unparenthesized arrows gain no support.
Unicode and dollar identifier tokens are retained by the occurrence API; this
does not claim repair of every downstream generic identifier-span limitation.

Occurrences are non-positional: a supported binding after an unsupported one
can be a local definition. Argument consumers still use the separate slot API.
Destructuring truncates slots; a defaulted middle slot retains a later required
parameter's original position without gaining its own definition. No slot
algorithm, write/shadow confidence, field-only isolation or same-line index
policy changed. No type/class/receiver/executable-owner or closure authority was
added; React.FC and the unresolved react-scripts disposition remain unchanged.

Prism navigation provided a stale caller graph (40 changed paths), not exhaustive
semantic proof. Current-source tracing confirmed DFG/reasoning consumers and the
independent positional API, motivating the dedicated bounded extractor.

## Evidence and review

The archived base plus byte-identical new tests produced8 failures and3 passing
refusal controls. The repaired checkpoint passes all11. Coverage includes both
dialects, function forms, exact token bytes, duplicate/escaped/recovery/modifier
refusals, definition/edge byte equality across full/subset DFG builds, labels,
real TS callees, cross-slot negatives and parallel/serial CPG parity. Same-run
JavaScript controls establish retained write/shadow/field-only behavior.

The initial CPG fixture wrongly assumed optional slots truncate; source showed
they are already supported. A second fixture repeated call arguments on one
line, hitting existing occurrence-index containment behavior. Separate call
lines and a valid default-before-required signature discriminate slot behavior.
Neither issue justified changing production slot/index policy. A failed helper
compilation was inadmissible behavioral evidence and was corrected before GREEN.

Independent review round1 of cap2: APPROVE,0 WRONG/1 SMELL. The reviewer ran
all11 focused tests. The sole SMELL is the historical cache-pin test name, which
still mentions receiver authority/item2; diagnostic naming only, deferred.

## Verification

| Gate | Result |
|---|---|
| Full Rust default / MCP / MCP+detached-owner-audit | 4,123 /4,316 /4,339 passed; one existing ignore each |
| Observers / receiver helpers / authority profiles | 726 /18 /40 passed; no skips |
| Python | 940 passed; one intentional live-evaluation skip |
| All native examples | 26 passed (7 asserted-member,7 DFG census,12 membership) |
| Focused tests | 11 passed; same-test base8 failed/3 passed |
| Formatting / diff / Clippy | pass /pass /completed with warnings |
| Tier-A matrix | 159 passed against freshly built frozen bytes |
| Tier-A quick | INVALID: corpus pin drift,6/30 oracle errors (20% >10%); SUT0 errors |

The full eight-gate runner started and ended on identical clean6a53c66a. Helpers
used the restored pinned archive at `/private/tmp/prism-asserted-upstream-CXalOE`.
Rust's ignore is `resolution_test::slice_elem_variant_reserved`. Python's skipped
live adoption module requires explicit `PRISM_RUN_LIVE_EVALS=1`.

Quick recorded clean corpus/SUT identities and159 matrix passes. Its30 pending
differences (16 oracle-only,14 Prism-only) are unadjudicated, not demonstrated
regressions or accepted accuracy. The PR preserves their exact sites. A base
harness replay independently rejects basef369f20c against pin20c8490591a3; this
classifies the pin limitation, not oracle behavior. No base oracle performance
comparison, rebaseline, adjudication change, full multi-corpus or live-model run
was performed. No real-site recall/receiver coverage gain is claimed.

## Next value checkpoint and custody

Rerun the real parameter/argument-site audit on the pinned public corpus and
the owner-authorized frontend portal. Measure which gaps this repair closes,
then classify optional/default and other remaining forms before further syntax
expansion. Keep default-initializer provenance separate from optional bindings.

Successor: the [real-site value checkpoint](parameter-sites/README.md) now measures
these gains and classifies residual targets. Its two source-backed pre-existing
defects take priority over further optional/default expansion.

[Plan](../../superpowers/plans/2026-09-10-typescript-required-parameter-occurrences.md) ·
[Handoff](../../superpowers/handoffs/2026-09-10-typescript-required-parameter-occurrences.md) ·
[Receipt](2026-09-10-typescript-required-parameter-occurrences-verification.json).
Raw evidence: `/private/tmp/prism-required-params-UgjX70`; archive identity is in
the receipt. Only this run's generated quick reports/snapshot were moved there;
no baseline or unrelated artifact was removed.
