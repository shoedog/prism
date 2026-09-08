# S9 — supported merged pairs qualify; Program remains incomplete

Producer `32ae5bc44000af31bb7cd5994a14de5700bafb02a1089f3ac16a79687e40f88d`
implements the [bounded v3 contract](../../superpowers/specs/2026-09-08-callable-merged-side-effect-admission.md).
The new helper hashes to
`5937c372d4f78414fe5b1bed3c239bc71cf129721d81aa546f9495ef9733c402`.
Schema 18/v1 and 19/v2 helpers remain byte-frozen and explicitly dispatched;
schema 20 fixes v3. No caller chooses a policy relaxation.

## Measured value

The unchanged public population has 6,893 resolution rows:

| Disposition | S8 | S9 |
|---|---:|---:|
| Filesystem-selected Program target | 6,270 | 6,270 |
| Singleton exact ambient | 272 | 272 |
| Singleton wildcard | 231 | 231 |
| Supported merged side-effect pair | 0 | 84 |
| Unproven | 120 | 36 |

The remaining 36 rows are 16 unresolved null-target occurrences and 20 present
filesystem targets outside Program. The latter are separately explained by the
[source-backed D2 audit](2026-09-08-callable-js-program-exclusion.md), not admitted
here. The 14 distinct module-literal gaps and unresolved react-scripts type directive
remain separate populations. Global semantic completeness is **false** for
`type_lib_unproven`, `boundary_encounter` and `resolution_unproven`.

Every previous packet field matches S8, excluding only schema/producer/semantic
fields. All 91,409 ordered basic host operations match transcript SHA-256
`fc4cfa95458d5521bab94c2e9bcf809c674e67acd5e62b119fd4a07a5b3d2eca`.
Full reproduction is valid/unproven; all 1,229 fixed public source files are unchanged.
Final private read-only replay also preserves source and legacy fields, validates
successfully and remains incomplete. Its evidence stays private. No dependencies
were installed, application/config files changed, or runtime/class authority added.
Eligible type-source rows are not receiver recall; this sequence adds no receiver edges.

## Verification chronology and limitations

The original pre-edit four-case source is preserved, SHA-256
`9b6c7d9937d868f27613ad975b4213c966066da3b77876b1c40b50dfacbfef59`.
Its exact-S8 RED has 0/4 passes after source-binding guards, but fails first on
schema 19 versus 20: this is **envelope RED**, not captured row-disposition RED.
Raw RED stdout SHA-256
`234fd7c2da302ff9e025df1bbe00f941ce15b0a318f15b3f833d33a41af7d36b`.
The earlier missing-test invocation is inadmissible setup evidence. A later new-test
syntax error likewise says nothing about production correctness; it was corrected
after rereading the file, with raw output retained.

First full copied run: 567/581 passed. The 14 failures were stale version/policy
expectations, a manual producer-digest census, and mutation controls retaining v2
derived facts when exercising v3 full reproduction. Test migrations preserve old
parser branches and regenerate derived semantic facts only to reach the intended
full-reproduction seam; no production barrier was relaxed. Focused repair 178/178
and full 581/581 then passed. These are compatibility migrations, not attributed
production regressions.

Independent round 1 found WRONG 0 and three test/evidence SMELLs: incomplete pair
provider guards, absent genuine-pair inherited-barrier coverage, and an overstated
baseline-projection count. Its native full run passed 581/581. The reported 30-test
baseline group actually contained 25 projections plus five pair/parser tests;
it is not described as 30 projected fixtures.

Bounded test-only round 2 adds four order/asset and ten genuine-pair barrier legacy
comparisons. Direct before/after row assertions are retrospective evidence, not a
rewritten pre-edit RED. The first unrelated-null fixture used an unchecked missing
side-effect occurrence absent from the captured row population and failed its own
setup; it was replaced by a captured import-type occurrence. No producer change.
Selected-anchor evidence combines exact integration anchor equality with the
separate pinned-compiler characterization in `merged-wildcard-proof.test.mjs` and
the full-reproduction genuine-contributor substitution control. This is not a claim
of a new independent checker query for every integration fixture.

The first expanded copied run passed 591/591. Final round-2 tests add direct
source-fact assertions for every labeled barrier, not only shared aggregate reasons.
Final test-source SHA-256
`51855e9d796b746d7e5c659d970396f47d38203ebd273466c0b1e46b5e434e2e`.
Primary's retrospective final-source four-positive recheck has native totals:
exact S8 0 pass / 4 fail at the actual unadmitted-versus-merged row assertion before
schema checking; S9 4 pass / 0 fail; no skips, cancellations or todo in either run.
This captured row-behavior recheck does not replace the original envelope RED.
Primary public/source checks are complete; final review and clean repository gates
will be recorded before publication. Tier-A is not triggered: no Rust call-resolution,
navigation, CPG or AST code changed.
