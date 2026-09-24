# Independent planning review — round 1 of 2

Date: 2026-09-19  
Artifact: `/private/tmp/prism-post319-successor-planning`  
Frozen artifact-manifest SHA-256: `3530a0895f3dc40ba3a6693b6cab054771119b458fd9b720510a7d2e5dfd6990`  
Normative SPEC SHA-256: `6517922d2ee00238c9f939e449ba9e181914593c36f649297ecae4676479a684`  
Review cap: round 1 of 2

## Verdict

**REVISE BEFORE DISPATCH — 2 WRONG, 2 SMELL.**

The selected population and scope boundary are sound: independent projection reproduced exactly 12 sites, 11 members, and 37,040 bytes; all frozen artifact hashes pass; the 30-case native probe contains 30 parse-clean cases and 33 function rows; and native production/Cargo/build inputs are source-equivalent between `f79bb954` and merge `30e13053`. The two WRONG findings are confined to the `next_action` eligibility rule. They do not require native API, graph, cache, or production changes.

## WRONG

### W1 — `slots:null` can be promoted as an observed suffix gap

**Concrete input/state.** Use the frozen JavaScript `duplicate` fixture, `function take({x}, x){return x;}`, with the exact function selector, `object_ordinals:[0]`, and `later_required_ordinals:[1]`. The authenticated baseline is parse-clean, uniquely named `take`, has `slots:null`, and has binding `x` at bytes 19–20 contained by source ordinal 1.

**Incorrect result.** SPEC line 88 asks only for a selected later binding and “no slot assigned to that same ordinal.” If `null` is treated as a collection with no assigned ordinal, this candidate yields `next_action:"bounded_entry_and_call_proof"` even though the native slot API explicitly refused positional authority for the whole list. That contradicts SPEC lines 38, 83, 92, 103 and 109, which preserve `None` as refusal and say this duplicate is characterization rather than approval.

**Mechanism.** The predicate does not distinguish `Some(prefix without ordinal)` from `None`. Absence inside a known prefix is an observation; absence inside an unavailable result is not.

**Bounded correction.** Require `slots` to be non-null before a candidate may satisfy the positive predicate. A clean candidate with `slots:null` must defer with a fixed reason such as `native_slot_authority_unavailable`; it must not be folded into the positive gap set. Add full-record JS/TS/TSX duplicate controls and a direct predicate control showing that null never promotes even when a later binding exists.

### W2 — compiler-supplied ordinal labels can promote a non-object native shape

**Concrete input/state.** Use the frozen parse-clean `array` fixture, `function take([x], later){return later;}`, but give its valid synthetic manifest the schema-accepted selection evidence `object_ordinals:[0]` and `later_required_ordinals:[1]`. The native baseline is uniquely named `take`, reports raw parameter 0 as an array pattern, reports `slots:[]`, and reports `later` at ordinal 1.

**Incorrect result.** The current predicate yields `bounded_entry_and_call_proof`, describing a candidate from the selected object-before-required cohort, although the native raw parameter observation disproves that the selected earlier ordinal is an object pattern. `object_ordinals` is never consulted by line 88. This violates SPEC line 63’s rule that supplied ordinal arrays are selection evidence rather than native authority and that native classification must not assume the desired gap from those arrays.

**Mechanism.** Exact callable matching authenticates the node span/kind, but it does not authenticate the compiler-derived parameter-shape labels. The positive rule checks only the later binding and slot absence.

**Bounded correction.** Make next-action eligibility representation-aware without adding a new parser: every referenced selected ordinal must exist in the emitted native `parameters`; each `object_ordinals` member must have the native object-pattern shape; and the triggering `later_required_ordinals` member must have the native ordinary required identifier shape. A mismatch or out-of-range ordinal defers with a fixed selection/native-shape mismatch reason. Add the array-as-object case above, an out-of-range selector case, and one positive object fixture with full output/totals. Keep site status based only on parse/match/name as currently specified.

## SMELL

### S1 — the parent/worker request has no executable wire contract

SPEC says authenticated retained strings are sent in one request and requires worker bad-JSON/protocol tests, but no frozen artifact defines the request schema, field order, source-to-path association, selector association, or which side supplies the hashes copied into output. Two implementations can comply with the prose while pairing `files[]` and `sources[]` positionally or by path; a reorder then produces different observations from the same authenticated inputs.

Before dispatch, define one small request schema and validation order. Prefer records keyed by canonical path, each carrying its already-authenticated retained source, validated language, member identity, and that path’s selectors, plus the manifest/native hashes needed in canonical output. Require exact fields, unique paths/selectors, referential closure, caps, and refusal before parse. Add swapped-order, missing-source, duplicate-path, orphan-selector, extra-field, bad-JSON, and oversized-request controls. This stays within the existing worker/launcher files and does not broaden scope.

### S2 — `script_kind` versus filename language is ambiguous

The frozen member schema stores `script_kind`, while SPEC line 63 only says “language from supported final extension.” It does not say whether `script_kind` is ignored, trusted, or checked against the extension. A synthetic `.tsx` member labeled `TypeScript` can therefore be parsed under different grammars by two conforming implementations, changing diagnostics and match status.

Define and validate the exact mapping `.js→JavaScript`, `.jsx→Jsx`, `.ts→TypeScript`, `.tsx→Tsx`; reject a manifest whose `script_kind` disagrees, and send only that validated value to the worker. Add one accepted row per extension and at least one mismatch refusal. The public 11-member manifest already conforms, so this is a contract repair, not a population change.

## Verified controls and feasibility

- `shasum -a 256 -c artifact-manifest.sha256`: all 12 frozen entries pass.
- Independent projection from the saved compiler packet and parent manifest equals `SITE-MANIFEST.json`: 12 sites, 11 members, 37,040 bytes. The first projection comparison was invalid because the review script read a nonexistent parent `script_kind`; after explicit extension mapping it matched exactly. The invalid attempt is retained in the hypothesis log and was not used as evidence.
- Baseline log: 30 `CASE` rows, 33 `FN` rows, no nonzero parse-error row.
- `git diff --exit-code f79bb954 30e13053 -- src Cargo.toml Cargo.lock build.rs`: empty.
- The 450 helper / 450 test / 900 total plan remains feasible if the finite wire-schema and predicate controls are data-driven. No cap increase is recommended. Preserve the specified allocation/early-stop checkpoint before implementation.
- No public source was parsed and no repository, production, planning-author, or dependency file was changed by this review.

## Required round-2 closure

1. Correct the positive predicate for non-null slot authority and native shape agreement.
2. Add the exact negative/positive controls in W1/W2 to the finite test contract and both prompts.
3. Specify the request wire schema/validation and exact extension-to-script-kind rule with the S1/S2 controls.
4. Refresh the artifact manifest and handoff. Preserve all current scope exclusions, exact 12-site population, budgets, and the distinction between characterization, behavioral RED, next proof, and production support.

**Final round-1 verdict: REVISE BEFORE DISPATCH — 2 WRONG / 2 SMELL; finite corrections, architecture unchanged.**
