# Bounded UMD bridge: four retained Library class candidates

Base: merged PR270, `9a34ef62bb48b6abb569ff62ad95e266e3124f51`.
Implementation: `fa67821`; final controls/evidence: `56106b0`.

The source-backed UMD bridge now traces five previously refused annotations in
the fixed public Program. The four watched Library receiver sites at lines155,
160,184,265 retain the defining class anchor in `data/library.ts`. All four remain
`program_unproven`; both authority flags remain false. There is no runtime recall
gain or new runtime consumer. The [compact checked evidence](2026-09-07-callable-umd-bridge-evidence.json)
records source hashes, exact byte/UTF16 anchors and packet comparison results.

## Proof and preserved barriers

The observer distinguishes global-export from module-local binding domains. An
eligible singleton global alias must belong to its exact configured Program
source module, with one direct export= assignment to one same-file non-imported
namespace. Source-module/globalExports/export-table identities and syntax
populations are both checked. Same-name global declarations, augmentation,
duplicate assignments/providers, merged local targets and stars remain refused,
including when skipLibCheck suppresses diagnostics. Explicit imports and local
shadows retain their existing routes; no React/FC spelling heuristic.

The one-time census is keyed by the exact Program, bounded by the existing
acquisition/time/heap limits, and replaced on fresh Programs. Alias and owning
module traversals consume the existing step budget. Schema7/producer0.8.0 refuses
old packets before audited-root access; full recomputation, writes and incomplete
Program guards are preserved. See the [implementation contract](../../superpowers/specs/2026-09-07-callable-umd-bridge.md).

## Same-source public measurement

The current producer and a fresh base-production replay used the same acquired
source, pinned TypeScript5.9.3 and Node24.15.0, installed profile and in-root links.
The base replay worktree was719e86d3; its entire observer directory is byte-equal
to merged main9a34ef62 (`git diff --exit-code`), and its packet reproduces the
previous SHA256 `0c4acbb619db405af129588cd7fe992945de1c6d93ce03c5a423570752c52a04`.
New packet SHA256: `41b2493070badf82040cb794634b262357d9cc1c770ee85f74fa5647caedcf8a`.

Every top-level field except schema/producer/observations is identical. Within
observations, every field except provenance and props_class is identical. Five
provenance chains change from unproven to traced; four class anchors are retained.
Normal validation reproduces the new unproven packet. Census stays59,924 regular
files,9,073 directories,249 links,853,674,175 bytes,591 roots,1,445 Program files,
zero diagnostics,30 observations,53 nested calls and10 lexical links. There are
zero observed Props/class records: closure still reports outside_lookup,
unresolved_module and unsupported_lookup, with264 refusal digests and603 null
resolution targets. No option/config change, installation or source rewrite.

Original public source0642e72cfa2d9a71198200e52f37399384610ee3 stays clean; all
tracked original/acquired bytes and executable bits match, before and after.

## Verification and hypothesis log

- Initial acceptance RED on unchanged main production:29 tests,19 pass/10 fail.
- Final exact-main control: identical32-test file copied into detached9a34ef62,
  production untouched;19 pass/13 fail. All13 fail on the desired new acceptance
  or schema6 pre-I/O behavior, not missing inputs. Changed producer:32/32 pass.
- Full observer suite:141 passed,0 failed,0 skipped.
- Full Rust default:4017 passed,0 failed,1 ignored (28 groups, including doctests).
- Full Rust MCP:4207 passed,0 failed,1 ignored (30 groups, including doctests).
- Pinned authority verifier:40 fixture/profile results, failures=[]; all seven
  audit-helper tests pass. cargo fmt and base-to-HEAD whitespace checks pass.
- Two SELF-PASS rounds, NOT INDEPENDENT; no open in-scope WRONG or SMELL findings.

The initial helper glob failed because the imported-source helper requires five
PRISM_AUDIT inputs in addition to the observer environment. Same-environment base
control failed at the same missing-variable assertion. Supplying the recorded
existing fixture paths produced7/7 passes without any test/source changes. This
was an inadmissible invocation, not a production regression. Its failing captures
are retained rather than silently replaced.

Other separating controls: raw singleton compiler identity versus name collision
is covered by same-name/renamed providers; real provider competition and merging
stay refused under skipLibCheck. Fresh base/new packet comparison separates the
bridge change from source/config drift. The13 exact-base failures and141 passing
observer tests establish the bounded behavior delta. No runtime, Rust, CPG, nav,
cache, Cargo or authority-fixture source changed (`git diff --exit-code` controls).
Tier-A is not triggered by this observer-only slice. No application build, RSS
measurement, private acquisition or full multicorpus evaluation was attempted.

Commands (compiler/profile paths in the handoff):

```sh
PRISM_TYPESCRIPT="$compiler" PRISM_CALLABLE_PROFILES="$profiles" node --test scripts/callable-observations/*.test.mjs
CARGO_TARGET_DIR=/Users/wesleyjinks/code/slicing/target cargo test --offline
CARGO_TARGET_DIR=/Users/wesleyjinks/code/slicing/target cargo test --offline --features mcp
node docs/eval/receiver-closure/verify-callable-authority.mjs "$compiler" "$profiles"
cargo fmt --check
git diff --check origin/main..HEAD
```

Audit-helper full command additionally sets PRISM_AUDIT_TYPESCRIPT to the pinned
compiler, UPSTREAM to `/private/tmp/prism-imported-props-audit-LjHVKt/upstream`,
SLICE to `/private/tmp/prism-indirect-default-VUbv13/excalidraw`, SITES to
`/private/tmp/prism-imported-props-audit-LjHVKt/real-sites.jsonl`, and SOURCE_REPO
to `/Users/wesleyjinks/code/bench-repos/excalidraw` (all names prefixed PRISM_AUDIT_).
Then `node --test docs/eval/receiver-closure/*.test.mjs` selects all seven tests.

## Recovery and custody

The original dirty checkout contained no unique unfinished implementation. All15
dirty files were preserved in local-only commita86bbeb and a verified local bundle
before switching cleanly to main. No hard reset or git clean. Historical ledger
and oracle snapshot remain recoverable, not published. The [recovery report](../../analysis/2026-09-07-dirty-checkout-recovery.md)
enumerates exact merged copies versus historical evidence; no separate completion
slice is needed for this population.

Task root: `/private/tmp/prism-umd-bridge-LqRG69`. Raw packets, source manifests,
base RED, final gates and executable packet comparison are retained in local
`umd-public-evidence.tgz`, SHA256
`0e1a5188796a969f289b73efcecbe97d86ec990479d8c15690d1a70c94e5618d`.
The archive is not published; compact source evidence and executable fixtures are.

Next recommendation: a bounded source/compiler-backed classification of the
remaining Program-closure lookup failures, with negative fixtures before any
resolution expansion. Do not promote these four candidates to runtime authority
or remove closure barriers simply because their class declaration is now known.
