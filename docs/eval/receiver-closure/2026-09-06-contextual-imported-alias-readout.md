# Contextual imported object-alias receiver identity

Base PR257 merge661a95a4ecfc4ced97a2a703a22082531b95527b;
branch feat/contextual-imported-object-alias. Implementatione3fb6ba and frozen
custody48dfc45 pushed; PR https://github.com/shoedog/prism/pull/258 merged
2026-09-06T19:49:56Z as835c4fbc3f7a9d9dcd3e6ec5dbbe96eb5e219f65. Fetched
origin/main matches; its tree equals published98166c7d9bd52bcaa794b1efaa0ba0e733767852.
Full suites complete; frozen quick baseline-invalid. CPG77/navigation sidecar45.
Merge reconciliation is carried by approved successor design/callable-authority-proof.
Gates below are prior implementation evidence, not rerun for this acknowledgement.

## Outcome and scope

The imported Props route now reuses the existing contextual parameter producer.
A direct function annotation or supported module-local callable alias/private
interface can supply an imported non-generic object alias for a required
destructured parameter. Whole-parameter single-binder substitution remains bounded.
The original annotation/argument node supplies type scope; the implementation
parameter supplies write scope; the foreign Props declaration supplies class scope.

Explicit parameter annotations are terminal even when unsupported. No imported
callable signature, interface, barrel, React.FC, hook return or general inference
is admitted. PR257's source-snapshot/ambient/prototype/reflection barriers remain;
no compiler-program or dependency augmentation closure is claimed. Receiver/member
writes, duplicate/shadow barriers and cached-proof replacement remain independent.

## RED, controls and self-review

Unchanged base runtime: explicit imported controls pass, all10 new contextual
positive forms lack Exact. This distinguishes missing producer support from a
foreign class lookup defect. Captures: red.log and red-population.log.

Two unnamed assertion/satisfies fixtures had no indexed call; they were inadmissible
negative probes and were replaced by named-function controls. The first candidate
passed full-build positives but failed an incorrect caller-only subset expectation:
build_direct_subset deliberately excludes target methods outside only_files.
The corrected control proves equal nonempty proof maps, no fabricated partial
edge, and Exact with the target included. Cached incremental merge is separately
tested. No product defect or oracle rebaseline was inferred from either setup issue.

Focused integration7/7:10 positive forms,34 contextual negatives,51 retained
imported negatives (one contextual exclusion promoted), plus27 defining-source/
ownership/snapshot cases replayed contextually, in TS/TSX full/direct-subset.
Disk64 transitions:32 contextual plus32 explicit controls, both directions for
owner A/B, missing declaration, augmentation, module ambiguity, barrel, prototype,
explicit-any override and use-site generic shadow. Four cache test functions pass;
eight sidecar states verify identity/absence and augmentation-membership invalidation.
Previous cache versions76/44 reject.

Pinned TypeScript5.9.3:11/11 new contextual fixtures and24/24 shared audit fixtures.
The new compiler controls distinguish contextual declaration scope from explicit
any and explicit consumer-decoy annotations; these are isolated fixture programs.
Prism skill query found the relevant caller but warned stale; LSP tools absent.
Current source and pinned compiler fixtures supplied the semantic evidence.

Self-review rounds1–3 complete within cap, no open in-scope WRONG. SMELL: the
source-snapshot contract does not prove dependency/program augmentation closure.
No inherited WRONG downgraded; SELF-PASS, NOT INDEPENDENT.

## Gates and real measurement

Full default4016 passed/0 failed/1 existing ignored (28 summary groups);
MCP4206/0/1 (30 groups), including two doctests each. fmt/diff checks pass;
Clippy completes with warnings, not warning-clean. Immediate release rebuild and
Tier-A matrix159/159. Frozen quick completed at48dfc456f244 with corpus_dirty=false
and prism_dirty=false; exit2, baseline-invalid for corpus SHA drift versus
20c8490591a3 and C-name4/6 successful probes. Oracle2/30 errors (6.67%), SUT0,
quiescent, zero stale adjudications; its matrix159/159. One run of cap2, no retry,
no paired base quick and no rebaseline. Raw Exact tp/fp/fn: callers17/0/22,
callees27/3/10; observations, not attributed regressions or a green accuracy gate.
Pinned target-c-method flip_candidate (supplementary Exact5/0/0,30 default-tier
Prism-only sites), module-deps-feature-gated and load-repo-feature-gated missing
literal pins; ambiguous-symbol-contract ok. Full site lists in PR258 and raw JSON.
Fresh same-environment replay with the PR257 runtime and candidate: all2780 raw
real-site records byte-identical,376 Exact. The refreshed605-file source census
validates pinned bytes (not compiler-program closure). Six remaining spans are
unchanged: four React.FC and two useApp/useContext. Served CLI fixture: three new
Exact targets at client.ts:2; explicit-any, written and React.FC remain non-Exact.
Served Exact callers lists exactly direct/generic/callable; direct's callees lists
client.ts:m. Neither query is truncated or has warnings.
Both runs used separate empty cache directories. Initial incompatible CLI flags
and the downstream empty-input audit were inadmissible setup failures, preserved
separately before corrected runs. No full multicorpus or rebaseline authorized.

## React.FC authority — separate recommendation

Authoring preference does not remove analysis semantics. React's guide demonstrates
direct props annotations; FC still exists in the type declarations. The analyzer
must resolve what a program actually uses, rather than require rewriting it.
See [React's guide](https://react.dev/learn/typescript) and
[DefinitelyTyped's declaration](https://github.com/DefinitelyTyped/DefinitelyTyped/blob/master/types/react/index.d.ts).

Recommend a separate compiler-backed provenance design using the project's actual
resolved declaration bytes and TypeScript configuration. Follow use-site import/
namespace aliases to FC/FunctionComponent, instantiate its call signature, and
retain selected props/class identity plus augmentation and cache-input custody.
A same-spelled local React/FC, missing package, ambiguous signature or any/unknown
is not authority. Existing runtime receiver/member/write barriers still apply.
Version-sensitive signatures and component static properties must not be confused
with the instance props shape. This slice does not implement React.FC support.

## Custody

Evidence: /private/tmp/prism-contextual-import-Y1y31N. Archive:
/private/tmp/prism-contextual-import-Y1y31N-evidence.tgz
SHA256:7d08b546b44eb1b509aa80c5c98a26efbec8a1b9ee0ab08e074a30672325c431.
Includes plan/implementation/frozen-source checkpoints, baseline/measured/frozen
binaries, compiler package, full gate logs, cache fixtures, source audit, real/served
replay, raw quick run/report/snapshot. Generated quick artifacts moved recoverably
out of the checkout; original .superpowers/ and eval/snapshots/prism-fb81481dafa7.json
remain untouched. Checkpoint publication fields are historical; this final committed
record supersedes them. No runtime/test edits after implementatione3fb6ba.
