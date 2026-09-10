# Complete project-boundary feasibility

Published as [PR302](https://github.com/shoedog/prism/pull/302).

Base: PR301 merge `5adf0394`. This is a source-backed design checkpoint, not
production wiring or a real-receiver recall result. The
[receipt](2026-09-09-project-boundary-feasibility.json) records fresh counts and
identities; the [plan/spec](../../superpowers/plans/2026-09-09-project-boundary-feasibility.md)
defines the next non-authorizing observations and required negative fixtures.

## Decision

Keep the complete acquisition root. Next implement bounded, non-authorizing
**logical-project membership and input-domain observations**, before selecting
inputs or expanding admission. None of the measured package boundaries is a
drop-in owner acquisition target. Do not raise 512 alone, remove inconvenient
files, flatten inherited configs, switch the production acquisition profile, or
equate a dependency declaration with an executable body.

The fixed public population is the unchanged root and six declared core package
configs at Excalidraw `0642e72cfa2d9a71198200e52f37399384610ee3`. The app has a
manifest but no separate tracked tsconfig; its context is covered by the root.
Examples/dev-docs are not asserted to be receiver-value candidates in this audit.
The approved private frontend has one tracked root package/config boundary,
no installed dependencies, and no declared workspace split. Its raw paths,
source identities and inventories remain local.

## Measured sets are not interchangeable

Native counts use the actual rebuilt Prism loader on each directory. Compiler
observations select each existing config **within the same complete public audit
root**, using pinned TypeScript 5.9.3 and the existing `installed`/`in-root`
research profile. They do not use or change Rust owner acquisition policy.

| Existing config | Native directory files | Configured roots | Repository Program files | Dependency files | Compiler files | Whole Program |
|---|---:|---:|---:|---:|---:|---:|
| Root | 628 | 591 | 595 | 768 | 82 | 1,445 |
| common | 27 | 19 | 406 | 313 | 86 | 805 |
| element | 73 | 50 | 407 | 313 | 86 | 806 |
| excalidraw | 415 | 340 | 428 | 317 | 86 | 831 |
| fractional-indexing | 2 | 2 | 2 | 216 | 86 | 304 |
| math | 23 | 16 | 406 | 313 | 86 | 805 |
| utils | 8 | 5 | 407 | 317 | 86 | 810 |

Here "repository" is a descriptive partition: `project/` members outside
`node_modules`; it includes JSON and declarations and is **not** an approved
executable-source domain. Dependency paths do not establish package authority.
The three Program columns partition the observed Program exactly. Configured
roots are entry inputs, not its transitive/effect closure. All seven packets have
verified compiler identity and stable snapshots; **all seven remain semantically
closure-incomplete**. Missing resolutions and boundary obligations are retained.
One snapshot digest covers all seven: the complete public inventory was identical.

Five package Programs reach repository files outside their package: common 386,
element 357, editor 86, math 390, utils 402. The math config inherits
[`packages/tsconfig.base.json`](https://github.com/excalidraw/excalidraw/blob/0642e72cfa2d9a71198200e52f37399384610ee3/packages/tsconfig.base.json#L13),
whose aliases target sibling source. Its
[`global.d.ts:1`](https://github.com/excalidraw/excalidraw/blob/0642e72cfa2d9a71198200e52f37399384610ee3/packages/math/global.d.ts#L1)
references vite types and imports the editor's global/CSS declarations;
[`src/range.ts:1`](https://github.com/excalidraw/excalidraw/blob/0642e72cfa2d9a71198200e52f37399384610ee3/packages/math/src/range.ts#L1)
imports common. The compiler resolves these exact cross-package targets in the
receipt. Directory size therefore cannot establish isolation.

The tiny fractional-indexing package still acquires 216 dependency and 86 compiler
files. All 2,780 rows in the fixed historical real-site inventory are in the editor
package, none in fractional-indexing. This is historical population placement,
not a fresh receiver census, owner eligibility count, or recall measurement.

Whole-root correspondence also fails independently of the 512-file limit:
the Program includes three repository JSON files and `excalidraw-app/vite.config.mts`
that the native census does not contain. Conversely 37 native files are outside
the repository Program. The editor package has 88 repository Program files missing
from its directory census (86 outside plus 2 JSON), and 75 native files outside its
Program. Dropping either side to manufacture equality would erase evidence.

## Unchanged owner-route controls

Fresh release binaries built from merged source; one replay per case:

| Root / config selection | First failed phase | Actual refusal |
|---|---|---|
| Full public root / root config | prepare_inputs | input_budget |
| Full public root / math config | prepare_inputs | input_budget |
| Math directory / its original config | compiler_evidence | semantic_closure |
| Editor directory / its original config | prepare_inputs | parse_error |
| Private root / root config | select_inputs | owner_requires_js_ts_only |

All five emit a non-authorizing admission report, fail acquisition and produce
empty stdout. Selecting the math config without rerooting still loads 628 files:
config selection is not a Prism input-selection API. Rerooted math's separate
observer packet verifies stable bytes but reports `invalid_config`, unproven config
provenance and outside lookup; its parent config is outside that acquisition root.
The aggregate owner phase does not establish which later proof gates would pass.

The native public census contains parse-error counts in two files:
`packages/excalidraw/tests/clipboard.test.tsx` (1) and `setupTests.ts` (2).
The editor's original config excludes tests, while its native directory census
still includes the first file. These are measured parser refusals, **not an
attributed source-syntax or Prism-parser defect**. Classify both against the pinned
compiler before any parser fix; deleting/excluding them just to pass is not a fix.

Private compiler observation under the existing default profile returned an empty
`budget_exceeded` refusal packet. Its Program membership is **unmeasured**, not 0.
The packet does not discriminate timeout, output or other resource mechanisms;
no larger profile or installation was tried. The root owner refusal precedes
compiler acquisition independently of that research observation.

## Source-backed barriers beyond the first refusal

- [Acquisition](../../../src/executable_owner/acquisition.rs) hardcodes
  `default`/`reject`, reproduces twice, compares the full actual loader map, and
  budgets every supported-language source in the authenticated project inventory.
  Even a hypothetical Program-only selection cannot bypass that full inventory
  budget or dependency/link policy.
- [Input validation](../../../src/executable_owner.rs) requires exact equality
  between supplied Prism inputs, detached sources and **every** `project/` Program
  member, including dependencies under project/node_modules. The
  [loader](../../../src/repo_loader.rs) skips node_modules. These are existing
  conservative domain constraints, not a newly introduced wrong-output defect.
- [Detached ownership](../../../scripts/callable-observations/direct-owner.mjs)
  independently scans all project Program files for indexed effects. Ambient/module
  declarations and mutation patterns remain barriers even when a class body is
  elsewhere. Relabeling a declaration as a dependency must not erase those effects.
- No observation authorizes a receiver edge. Duplicate/write/cache and same-epoch
  ownership barriers remain unchanged. React.FC remains separate. Root/editor
  packets still retain the unresolved `react-scripts` directive in
  [`packages/excalidraw/react-app-env.d.ts:1`](https://github.com/excalidraw/excalidraw/blob/0642e72cfa2d9a71198200e52f37399384610ee3/packages/excalidraw/react-app-env.d.ts#L1).
  Its absence from other configs is not permission to prune it from root/editor.

## Options, costs and checkpoints

1. **Recommended: membership observations first.** One bounded implementation
   slice records complete sets/differences and source-backed selection reasons;
   another proof slice is needed before any selection or domain-aware ownership
   wiring. Highest near-term value: makes the real blockers explicit without
   claiming closure or paying installation costs. Stop if any field cannot be
   completely reproduced under the existing observation budget.
2. **Keep whole-root ownership and engineer larger acquisition.** Preserves the
   present census contract but needs resource/profile/link work, parser refusal
   classification, dependency/JSON/module-suffix correspondence, and closure/effect
   proof. Multi-slice work, no defensible time-to-recall estimate yet. Raising
   only the file limit has no measured standalone value.
3. **Use a genuinely isolated package as a control.** Fractional-indexing is a
   useful membership edge case, not a demonstrated receiver-value win; parent
   config and automatic dependency obligations still need custody. A fabricated
   self-contained fixture is a control, never a replacement public-repo claim.

Before any production-selection proposal, checkpoint on full set reconciliation,
source-backed parser/membership gap classification, dependency/effect retention,
and at least one eligible real-site candidate. If that candidate remains blocked,
report the barrier rather than broadening to React.FC or relaxing closure.

## Verification and limits

Fresh native censuses 8; complete public compiler observations 7; representative
owner refusals 5; rerooted math configuration control 1. Both real roots' tracked
bytes, pins, clean status and non-following filesystem metadata match before/after.
No real-source edits, pruning, config rewrites, installs, dependency scripts or live models.

Observer tests and 16 passing local artifact checks are recorded in the receipt.
The latter reconcile raw packets/native censuses and reject receipt substitutions;
they do not implement the future membership fixture matrix. A numeric-substring
privacy check initially matched a public digest; typed-value checks and injected
privacy negatives corrected that local validator without changing the evidence.
The first
observer invocation omitted the required profile variable: 586 passed and 5 test
files failed setup. This is an inadmissible suite result, not a regression claim;
the corrected invocation passed all 694 tests with no skips. Rust release build passed.
This documentation/design slice changes no runtime/test implementation: full
Rust/Python suites are not rerun; Tier-A is not triggered, and no current accuracy
baseline or green CI claim is inherited.

Prism structural callers navigation returned SymbolNotFound. Direct source,
native loader and compiler observations are the fallback; unavailable graph
coverage does not establish absence of callers. Two self-review rounds are
recorded in the handoff; this is not independent review.
