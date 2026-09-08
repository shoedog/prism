# S7 — exact ambient rows qualify; the real Program remains incomplete

Producer b75dbd6dc5fdb1f083d134fa132a2b89e2091f14f0c69f6f245149d2ad4def2e
implements the [strict exact-ambient contract](../../superpowers/specs/2026-09-08-callable-exact-ambient-admission.md).
The public replay contains6893 rows:6270 filesystem-selected,272 exact ambient,
351 unproven (315 unadmitted wildcard/merged,16 unresolved null rows,20 targets
outside Program). Completeness remains false for type_lib_unproven,
boundary_encounter and resolution_unproven. This is not receiver recall.

Every old field matches S5. All91409 ordered host operations match, transcript
SHA256fc4cfa95458d5521bab94c2e9bcf809c674e67acd5e62b119fd4a07a5b3d2eca.
Full reproduction is valid/unproven and1229 public source files are unchanged.
Private read-only replay with the existing larger profile also reproduces, preserves
old fields and source, and remains incomplete. No dependencies were installed;
private packet/paths/counts remain outside public custody. Default-profile refusal
from the first checkpoint is not used as compatibility evidence here.

## RED and correction chronology

On exact frozen S6-equivalent source (S5 plus unwired accepted helper), the pre-edit
run had40 tests:16 passing compatibility controls and24 intended failures. The
failing assertions were23 missing-semantic-field seams and one helper digest seam;
the15 helper tests plus history control already passed. Captured log SHA256
68a092eda977b4abf06a9b1f7102eb0623e1b3484b9578e2e95b30d9834edacc.

Transcript-observed initial implementation38/40 exposed a missing0.19 path-refusal whitelist entry and
an augmentation fixture expecting one row despite two genuine callback occurrences.
Raw worker output retained valid classifier evidence while parser refusal isolated
the version guard. The fix preserves path refusal; the fixture now asserts both
rows. Focused40/40 and exact preserved-base old-field parity40/40 followed.

Transcript-observed first admissible full run536/546 enumerated10 stale test migrations, not ten policy
defects: historical clones delete the new current-only field, while shaped
lookup/path/wildcard reproduction mutations recompute the dependent semantic field
so they still reach full reproduction. Current-version/digest expectations migrate
with the envelope. Transcript-observed targeted173/173 and archived final standard-env
full546/546 passed. The three intermediate runs38/40,536/546 and173/173 were not
saved as standalone log files; that is an evidence-custody limitation, not claimed
archive coverage. Original RED and final focused/base/full greens are file-backed.
The original baseline and RED remain unchanged; no tests were re-baselined away.

Independent review round1/2 ACCEPT98, WRONG0/SMELL0,40 focused controls. A separate
review invocation included index.test without its required profile environment;
that guard failure is inadmissible, not a regression. Normal integration tests use
only standard compiler/profile settings; historical baseline comparisons are
optional extra assertions, not claimed to run without that explicit baseline.

## Value checkpoint 2

Continue the bounded S8/S9 proof coverage and S10 contract; do not promise real
receiver gains. Strict synthetic cases now distinguish semantic completeness from
old null-target closure, while the fixed real Programs honestly remain incomplete.
The owner-settled react-scripts directive,14 module-literal gaps and all boundary
encounters are unchanged. D2 separately audits the20 present non-Program JS targets;
no automatic membership or closure waiver is included here.

Full repository gates passed on clean59fd6eb00d64ef958849237379e1413799494dd2:
546 observer,4017 Rust,4207 MCP,18 helpers,40 authority,fmt/diff; one known ignored
per Rust run, doctests included. The [gate receipt](2026-09-08-callable-exact-ambient-gates.json)
binds raw logs and archivec00ec9c8. Published as [PR289](https://github.com/shoedog/prism/pull/289),
pushed48de24e against PR288; feature-base CI is not
scheduled by the main-only workflow. No remote-green claim or auto-merge.
Tier-A is not triggered; no Rust resolution/navigation/CPG/AST changes.
