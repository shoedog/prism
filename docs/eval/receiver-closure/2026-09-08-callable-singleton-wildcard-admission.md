# S8 — singleton wildcard qualifies without asset/runtime authority

Producer020fb7b4e9c7491835c1fc65f38db527722954e7b68b2670ad3e8ba3ca733e0a
implements the [bounded v2 contract](../../superpowers/specs/2026-09-08-callable-singleton-wildcard-admission.md).
The byte-frozen v1 helper remains1ac30091a62bc03e267f3ff55c906a5e26db06dfc937752dfcabf6bbeb20e4b4;
new v2 helper82c55f11d4e0a6e10a5ae7ca0fc379a1177c75f00995a04eed9b42a3685e651c.
Historical schema18 is explicitly dispatched to v1; schema19 fixes v2.

The public replay has6893 rows:6270 filesystem-selected,272 exact ambient,
231 singleton wildcard,120 unproven (84 merged pairs not yet admitted,16 unresolved
null rows,20 present targets outside Program). Completeness remains false for
type_lib_unproven,boundary_encounter,resolution_unproven. No real receiver gain is
claimed. The owner-settled react-scripts directive and14 module-literal gaps remain
separate, unresolved populations.

All previous fields match S7; all91409 ordered host operations match transcript
SHA256fc4cfa95458d5521bab94c2e9bcf809c674e67acd5e62b119fd4a07a5b3d2eca.
Full reproduction is valid/unproven;1229 fixed public source files are unchanged.
No installation, application/config change, outside waiver or runtime/class authority.

## Verification and correction chronology

Captured pre-edit frozen-S7 expanded run14 tests:11 compatibility passes and3
intended behavioral failures for absent asset, present asset and query specifier.
All3 fail at unadmitted_binding versus singleton_wildcard after source-binding and
strict-boundary setup assertions. RED log SHA256
f62981e34475a6c56e2e667fff05a9c54ff1358b7c0d9da864328f6a4cea98e7.
Custody SMELL: original14-case pre-edit test source was not separately preserved;
its captured output remains, while the published integration file has18 cases.
No reconstructed source is labeled original.
Primary's retrospective final-source selection independently rechecked those same3
positive cases: frozen S7 fails3/3 and candidate passes3/3, native totals and no skips.
Final test-source SHA2560000deca020b31cec255f5f330f578082899aa8dec2b9db7d4209451fd086294.
This is not a replacement pre-edit snapshot. The first recheck parser counted both
diff and repeated actual-value renderings; native output had exactly3 failures.
Correcting that diagnostic parser and preserving its original raw capture changed
no implementation. Fresh corrected recheck logs retain the exact three-case result.
The earlier augmentation row-count fixture failed its own setup and is inadmissible;
the corrected fixture preserves both actual occurrences. Initial candidate13/14
retained a baseline-only expectation, then replaced it with an explicit historical
schema18/v1 parser control. No production relaxation was used to satisfy that test.

Focused50/50 passed. An optional mixed-comparator run used S4/S5-era projections
that retained semantic_closure; its25 failures showed the authorized v1/v2 policy
difference and are inadmissible as legacy-field parity. That truncated run is
retained as a contemporaneous summary, not complete raw output. Primary's fresh
native S7-aware comparison passed25/25 with0 failure/skip/cancel/todo; stdout
SHA2560dbac05b63e1fb1251ce94f45d69f0e9978bb5cb428dbeff8a4371f9ef2b769f.
An incomplete-environment full invocation is retained as inadmissible, not a regression. First complete-env
full570/571 had one test-only manual digest census missing the v2 helper. Its update
does not change producer020fb7b4. A subsequent dot-report count was not used as native
aggregate evidence; a fresh full native TAP run confirmed571 pass,0 failures,
cancelled,skipped or todo. Native stdout SHA256
1cb75a166baef55d903974c27557c70446387d27ee67a00e2849c6ca5c770048.
Intermediate records remain distinct and summary records are not raw-output claims;
no manual footer is represented as a native runner aggregate.

Independent review round1/2 ACCEPT98, WRONG0/SMELL0;65 independent focused controls
passed, including history, empty refusal, tamper before root I/O, reproduction, cap
and portability. Main source review and public transcript checks agree.
[Full repository gates](2026-09-08-callable-singleton-wildcard-gates.json) passed on
clean8fc6624:571 observer,4017 Rust,4207 MCP,18 helpers,40 authority; doctests included,
one known ignored per Rust run; fmt/diff passed. Tier-A not triggered: no Rust
call-resolution/navigation/CPG/AST changes. Local evidence archive8599628 bytes,
SHA2561c01ca111a07a7273e649d8e0c934e490ded77de186f7dea3a9a13ad41feb41f.
