# Optional/default parameter proof and bounded implementation

Local bundle based on merged PR312 `d9d1cc91`. Bounded implementation and real-site
replay are complete; post-review verification is in progress. The owner approved
multiple local commits and one eventual combined MR. No push or MR in this run.

## Contract and source-backed decisions

Parameter shadow guards, positional slots and authoritative occurrence tokens
are different contracts. JS/TS local-binding guards already include optional,
defaulted and destructured names. Slots already retain simple optional/default
identifiers. Neither fact creates a DFG parameter Def. The occurrence producer
supplies token identity; the DFG additionally needs a bare reference and an owner.
CPG Step5b intersects slot and occurrence bytes with an existing entry Def and
validates its complete owner/path/access/range. It never substitutes a body Def.

First bounded implementation: TS/TSX simple optional identifiers, only in
initializer-free signatures. Retain whole-list duplicate/escape/recovery checks
and the exact parameter-child allowlist; do not admit properties, decorators,
erased `this`, rest or destructuring. Any runtime initializer, including nested
binding-pattern defaults, refuses **new optional occurrences**. Existing required
occurrences and non-TS behavior stay unchanged. This deliberately under-approximates
safe signatures until default parameter environments are handled separately.

Second bounded implementation selected after primary/Opus design review: default
identifiers in JS/TS/TSX, with number/string/boolean/null literals or empty arrays
and objects, only in all-simple-identifier signatures whose defaults are all
supported. This guard applies to newly defaulted occurrences, not old required
ones. Exclude unary/template/asserted/parenthesized expressions, identifiers,
calls, filled containers and destructuring. No default-expression value edges,
React.FC, closure admission, callable-owner expansion or react-scripts change.

## Hypothesis / probe / result log

| Hypothesis and alternative | Probe and observation |
|---|---|
| Existing slots suffice for new token authority; alternative: slots compress omissions | Current `step5b_edges_for_caller` enumerates the original slot index and stops when `i >= args.len()`. Unsupported slots remain `None`; targeted omission/holes regressions are required. |
| A sibling default can overwrite an earlier parameter; alternative: initializer text is inert | Node runtime control `f(a,b=(a="clean"))` returns `"clean"` for `f("input")`, but `"input"` for `f("input","present")`. Captured local runtime evidence confirms the hazard. |
| Existing graph claims exact flow through that write; alternative: CFG completeness downgrades it | Base DFG has both signature Def tokens but labels entry-to-body flow `NameOnly(CfgIncomplete)`. This is **not evidence of a false Exact edge**. An assignment token inside a signature is not inherently an incorrect Def. |
| Every TS default is an assignment-pattern wrapper | Source observer and pinned grammar expose a parameter `value` field as well. The early adviser assumption was incorrect; exact children and fields govern support. |
| Syntax census counts are potential recovered flows; alternative: owners, slots and bare references exclude entries | Pinned compiler matches source parameter ranges after UTF-16→UTF-8 conversion and file hashing. Counts are syntax entries only; native candidate replay is still required. |

Runtime omitted and explicit-undefined arguments both select the default; explicit
non-undefined input skips it. Token identity alone does not prove the selected
runtime payload. A self-reference in a default initializer is a TDZ refusal case
for any later support; no claim that current production admits such a token.
The return-only probe's `CfgIncomplete` result does not prove general safe labels:
RD can be unavailable for that tiny CFG. Nontrivial-body controls must test the
new Def's label explicitly. `Exact` binding evidence is not guaranteed runtime
delivery of the argument's value.

## Value and verification checkpoints

The tested source observer matched all 702 selected entries (443 public,
259 private), with matching file hashes and no selected-file syntax diagnostics.
Public forms: 194 optional / 249 default identifiers; private: 0 / 259. These
counts come from the unchanged production callable inventory, not all source
callables or unique call sites. Initializer-free optional syntax counts are
175 public / 0 private; 158 public entries have an inventoried named owner and
matching slot. Broader inert-default syntax candidates are 191 public / 176 private,
of which 180 / 176 have inventoried named owners and matching slots. This predicate
also includes unary numbers and no-substitution templates, and does not enforce
production's all-simple-sibling guard (for example, `{b} = {}` is allowed as a
sibling). It is a syntax upper bound, not a safe-admission count. The named-owner
aggregates trust census metadata: they do not authenticate enclosing source-function
identity or executable ownership. Parameter ranges and slot spelling/bytes are
source-checked. The observer cannot grant runtime authority.

Fresh base native replay matched the prior graphs exactly. At implementation
commit `0bb567cb`, the byte-identical source populations, loader skips and static
function/parameter/slot shapes are unchanged (628 loaded public files, 1,122 private).

| Change versus merged main | Public Excalidraw | Private frontend |
|---|---:|---:|
| New occurrence tokens | 360 (175 optional, 185 default) | 175 default |
| New exact-token DFG/CPG definitions | 298 | 168 |
| Added / removed Use→Def observations | 404 / 3 | 59 / 0 |
| Net slot-matched flows | +401 | +59 |
| Change in unmatched-slot flows | 0 | 0 |

The default-only increment contributes 185 / 175 tokens, 167 / 168 definitions,
and 114 / 59 flows. Every added flow targets a compiler parameter identifier
matching the inventoried owner/token Def and has a matching source argument ordinal.
There are 364 / 54 single syntax matches and 40 / 5 multiple enclosing-call
matches. The three removed public flows have no matching source argument ordinal.
These are flow observations, not unique calls, compiler-Program callee resolution,
recall, or receiver/executable authority. Source hashes and parse diagnostics were
checked for all selected files. A local verifier initially selected TS mode for
`.jsx`; 45 affected selected files parsed cleanly in the correct mode. Its captured
8-pass/1-fail RED became 9 passing controls before these final classifications.

## Additional bounded repair: comment trivia shifts arguments

**WRONG, reproduced on unchanged base in the same environment:** a named comment
child inside a JS/TS/TSX argument list occupies a positional index. For
`takeRequired(a, /* comment */ b, c)`, the base emits `b→last` with `Exact`
confidence instead of `b→second` and `c→last`. Optional support exposes additional
instances; it did not introduce the underlying extraction defect.

The source pass found three new optional-flow mismatches: two field/base flows
from `AppStateDelta.orderAppStateKeys` to `Delta.calculate`'s fourth parameter,
and one from `appState` to the omitted seventh `opts` parameter in
`bindOrUnbindBindingElement`. These are public Excalidraw snapshot sites in
`packages/element/src/delta.ts:535` and `packages/element/src/binding.ts:1011`.
All280 targets were real parameter tokens; that alone missed the wrong indices.
Thirty-five additional source-span matches have nested-call ambiguity and are
not independently resolved-call proofs.

The repair filters JS/TS/TSX comment nodes consistently in indexed span/text,
line-based vector and Nth-argument APIs, plus the reference walk. It preserves
real expression spans, spread representation, comments inside string expressions,
field/base supplementation and other languages' existing behavior. Four regressions
fail before and pass after the fix; one non-JS control passes both. All17 existing
call-argument-focused tests pass. CPG cache82→83; no call-resolution policy change.
Native replay and full verification follow. Separate local commit; no publication.

The subsequent source read-through found the fifth companion producer: call-site
metadata counted named comments too (`target(/* only */)` reported1 instead of0).
Its own pre-fix test fails; all6 comment regressions/controls pass after using the
same predicate there. CPG83→84 and navigation sidecar45→46 invalidate serialized
call-site counts/fingerprints; this does not expand arity-resolution policy.

After the positional repair, public replay versus merged base is290 added /
3 removed flows (net287); versus optional-only it is13 added /6 removed. Every
added target is a supported compiler parameter token and has at least one matching
source argument ordinal.255 have one syntax match,35 have multiple enclosing call
arguments. All3 removed base flows fail the source ordinal check. This is not
independent compiler-Program callee resolution. Private replay remains unchanged.

Source observer tests: initial18 passed, then primary review captured17 passes /
5 failures for four bounded defects (one duplicate manifestation): optional
sibling initialization, form mismatch, wrong slot spelling and escaping symlinks.
All22 now pass. New-tool initial tests are not RED evidence; the corrective run
is a captured behavioral RED against the saved first implementation. Paths are
checked canonically in an owner-controlled immutable snapshot; no adversarial
concurrent filesystem-mutation safety is claimed.

Raw private rows/source remain local. The source observer emits aggregates only
and cannot authorize runtime edges. No project dependencies were installed. After
older `/tmp` inputs disappeared, six exact-version public package archives were
downloaded and extracted without scripts solely to restore verification inputs.
The TypeScript 5.9.3 module reproduced SHA256
`3ae902c92cc44dace175c0e69e13a4b0899f6983c6121d76b9ab8dd5795e7675`;
both React declaration trees reproduced the committed pins and all 40 authority
controls passed. The compiler supplies syntax here, not a compiler Program
or type-checking authority. Prism navigation reported 41 stale paths and LSP tools
were unavailable; current source establishes the consumer contracts.

Optional checkpoint: the primary parameter-focused run passes55 tests; source
observer22 and cache pin1 pass. Fresh release build and all159 Tier-A matrix cases
pass. Captured optional RED includes7 discriminating failures plus one confounded
explicit-argument fixture (repeated same-line path); an initial zero-test filter
is inadmissible and was corrected. Required controls passing before the change
are not RED. The corrected explicit/omission fixtures and additional shadow and
non-vacuity controls pass. Cache pin RED (81 vs82) is separate from behavioral
proof. These optional-only counts are historical checkpoints, not final totals.

## Implementation review and verification

Default RED captured 12 failures / 8 passing controls; adoption exposed two further
draft defects (declaration order and sparse arrays admitted as empty), then passed
all 22 default tests. Required behavior, omission/undefined, writes, redeclarations,
shadowing, missing-entry refusal, exact bytes, DFG label parity, full/subset and
serial/parallel cases are covered. The parameter-focused suite passes 83 tests.
Cache version is 85; navigation sidecar version is 46. The latter changed only for
the earlier argument-count metadata repair.

Independent bounded production review: APPROVE, WRONG 0 / SMELL 0 at `0bb567cb`.
Source-observer round 1 found one WRONG: uppercase SHA-256 text accepted by the
census schema was rejected by case-sensitive source comparison. Captured RED was
1 failure / 1 mismatch control; normalization now passes both. Three added project
tests cover matching/mismatched uppercase hashes and the explicit inventory-owner
limitation; the project observer suite now has 25 tests. The two review smells are
documented above, without adding owner authority or narrowing the syntax upper bound.
A purely equivalent boolean simplification removes two new Clippy warnings.
Claude Sonnet's default writer hit its session quota; its partial draft was
snapshotted, then completed and reviewed by root. GPT reviewers were used through
the owner's previously approved fallback. Review cap remains two rounds.

Initial full Rust runs: 4,184 / 4,377 / 4,400 passed (default / MCP / owner-audit),
one existing `resolution_test::slice_elem_variant_reserved` ignore each. Node
callable tests 726 and authority controls 40 passed. The full helper invocation
passed 15 and failed 3 because its historical call-site fixture is missing; a
12-archive custody search found no matching copy. The three tests in
`audit-imported-props-source.test.mjs` are explicitly excluded from the largest
runnable final subset: pinned six-span acceptance, changed augmentation bytes,
and added augmentation-file refusal. No expected call-site evidence was fabricated.

Initial Python run: 939 passed / 1 failed / 1 intentional live-adoption skip. The
failure was the 0.2-second warm-handshake assertion under concurrent load. Both
base and candidate passed isolated same-environment controls afterward; this
does not establish a cause or a regression. A quiet full rerun is required below.

Tier-A: fresh builds and all 159 matrix cases pass. Base and candidate quick runs
are INVALID: corpus-pin drift and oracle error rates 0.20 / 0.1333, respectively;
Prism tool error rate is zero in both. Both retain the same pinned outcomes:
`target-c-method` flip candidate; `module-deps-feature-gated` and
`load-repo-feature-gated` missing; ambiguous-symbol contract OK. No rebaseline or
full multicorpus/live-model evaluation was run. Final post-review gates are pending.

## Next bounded work, not included here

Review enumerated separate comment-sensitive positional consumers: `require`
binding/module extraction and mutation analysis in `src/ast.rs`, dependency arrays
in `src/react_hooks.rs`, Express entry arguments, and the shared/independent taint
argument selectors. They are outside the enumerated argument-API repair contract.
Audit them with paired comment/no-comment behavioral controls, starting with
module binding and mutation/write barriers, before changing those subsystems.
The census of helper shapes is not proof of security findings. Do not fold this
cross-subsystem expansion into the completed parameter slice without a new boundary.
Complex defaults, rest/destructuring and owner coverage remain separate; the
measured gains do not justify relaxing those barriers indiscriminately.

Local evidence root: `/private/tmp/prism-optional-default-W7ekZ5`.
