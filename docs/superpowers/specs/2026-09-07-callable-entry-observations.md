# Configured/automatic type and lib entry observations

Owner approved the bounded next slice and publication of the keep-unresolved
react-scripts decision with it. Base: merged PR279
`ca473cd198d3a906a9ba9b14402118a4bbcd952a`.

## Contract and boundaries

Schema12/producer0.13.0 adds `type_lib_entries[]`, separate from source-written
`type_lib_references[]`. Rows contain kind (types/lib), origin
(configured/automatic/default), kind-local index, effective name, mode (null for
this pinned entry channel), status/reason, canonical Program target and matching
inclusion boolean. Names for configured libs are the compiler-normalized filenames,
not invented JSON source spans. Config owner remains scope.config; options identity
and captured config files bind effective configuration, not its declaration origin.

Use TS5.9.3 Program-retained automatic names/resolutions, resolvedLibReferences.actual
and FileIncludeKind8/6. Configured types retain repeated option occurrences; automatic
types retain compiler enumeration order and repeats. Kind8 binds typeReference/name,
not an invented inclusion index. Kind6 configured libs bind option index; default
libs bind the indexless inclusion and the pinned host's default filename. Require
exact SourceFile object membership, not merely inventoried files. No resolver,
filesystem enumeration or pathForLibFile call during observation. No inherited
source directive cache can authorize an entry.

Unprocessed entries (e.g. no roots, noLib, no-default-lib, absent cache/inclusion),
unresolved type cache results, absent Program targets and missing matching inclusion
remain unproven. An unprocessed default row does not assert the cause of suppression
or an unsatisfied dependency. An empty automatic census is not evidence of complete
ancestor lookup coverage. Types=[] and lib=[] retain their empty populations.

This is additive observation only: **no change to existing reasons, closure bits,
Props/class or runtime authority**. An observed entry is input identity, not semantic
closure; an unproven row is not automatically an obligation (intentional disabling
is possible). Closure-policy treatment and causal outside/refused attribution are
separate. No config inheritance location proof, project-reference traversal, app
edits/installs, react-scripts provisioning/waiver or React.FC expansion.

Strict parser: unique canonical kind/index order, legal kind/origin/mode combinations,
target Program membership and status/inclusion consistency. Full reproduction binds
every row, omissions, repeated indices/names, target substitutions, option/cache
epochs and selection mode. Historical schemas10/11 stay audit-readable without
invented rows, never current-valid. Ledger cap100000 aborts, never truncates;
existing packet/worker/inventory budgets apply. Helper bytes enter producer hash.

## Source proof and verification plan

Pinned compiler: automatic discovery44537–44564; Program entry processing127131–127182;
cache accessors127259–127260; type inclusion128823–128843; lib actual target128852–128904.
LSP skill source fallback: no LSP tools exposed in this session.

1. Real pinned-Program RED on exact base: configured/automatic types, disabled and
   repeated options, typeRoots exclusions, conditional exports, configured/default
   libs, replacement/fallback, incidental membership, missing providers and forgery.
   These are missing-observation contract REDs, not false-closure repair claims.
2. Implement additive helper/schema/worker wiring, then rerun identical tests.
3. Cache-double guard tests for impossible-to-force public states; label as controls.
4. Replay fixed public source against exact-base packet, compare every prior field,
   validate reproduction and preserve source/lock bytes. react-scripts stays unresolved.
5. Full observer, default Rust/MCP, helper/authority gates. Two SELF-PASS rounds
   (NOT INDEPENDENT), cap2; classify at cap. Tier-A only if its source paths change.
6. Publish decision, implementation, evidence and refreshed handoff together; no merge.

Probe expectation: retained entry caches and inclusion identify entry targets without
new reads or closure changes. Falsifiers: observation adds probes or selects a target
without matching entry inclusion. Alternative: incidental source imports/lib references
can include the same file; disabled-entry fixtures discriminate it.

Raw evidence root: `/private/tmp/prism-entry-observations-iZY1n0`.
