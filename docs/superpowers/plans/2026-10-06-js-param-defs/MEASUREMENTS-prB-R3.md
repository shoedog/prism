> **Current authority (R4, 2026-10-06): STOP; PR-B PARKED.** R4 repaired ordered conditional parameter sources and bounded early errors. Six invoked-default design cells still require callable timing/captured side-effect design; 18 main-correct synthetic rows remain lost. No adoption or shipping claim. Read MEASUREMENTS-prB-R4.md and HANDOFF-repair-prB-r4.md; earlier results remain historical. Source base 1af4301f; docs base a3dda010.

# PR-B R3 STOP — further default-source binding-environment instance

PR-B is **PARKED** under the explicit R3 brief. The retained draft is **not accepted**. One owner-authorized principled fold was attempted; no additional review round, restart or source repair followed the STOP. Source base `f8c768b35340d32b92533099091a8536b5b26284`; docs base/HEAD `35d780e1b0fc0be426b2d95b3014b65249456d92`; branch `plan/js-param-defs-prB`. Entry724 source/test/mutant files are byte-identical to the proto. Session turn metadata confirms `gpt-6.1-sol`.

## WRONG R3-SOURCE-1 — skipped defaults need more than the syntactically last source

```javascript
function h(f, d = (f = 5), e = (f = 7)) {
 var f;
 use(f);
}
```

Node synthetic-only verdicts: `h(1)` reads7; `h(1, undefined, 0)` reads5; `h(1, 0, 0)` reads1. Thus Def19 and formal11 can reach Use55. Same-environment frozen main emits both full rows with `NameOnly(SameLine)` in JS/TS/TSX. The unchanged proto emits neither; the draft emits only Def32→Use55. **Six main-correct rows remain lost** in this diagnostic population. This is a residual proto loss, not an R3-introduced attribution. No corpus table receives these synthetic counts.

The draft's `max_by_key(start_byte)` selects the final syntactic default write. A supplied later argument skips that write, leaving an earlier source reaching the body. Alternatives ruled out: Node falsifies that all defaults run; complete current graph output falsifies admission by an ordinary non-copy pass. Confidence100/100 for these concrete losses. Confidence would collapse if the captured main/current endpoint rows or exact Node results were false; all are retained with source bytes and hashes. General frequency is unknown.

The bounded design direction is to propagate the parameter environment's reaching sources through default-skipping branches before copying into the body. **Not implemented:** the brief says a further instance parks PR-B. An independent review/adoption claim is not made.

## Partial fix retained

- Copy binding detection calls exactly `js_ts_function_scope_binds`, the fence predicate. Body-entry identity handles destructuring, for-in/of and Annex-B catch var binders.
- The bespoke inline-if bypass and may-reaching admission filter are replaced by completed unconditional statement-write dominance for dropping copies; admitted rows use the ordinary RD classifier label.
- Default-written source selection is attempted but is incomplete as shown above.
- `grade_parameter_environment_copy` isolates S1 policy and returns the RD grade unchanged.

Only3 source files differ from proto: `src/ast_callback_identity.rs`, `src/data_flow.rs`, `src/cpg/callback_identity_tests.rs`. No mutant/cache change was completed; existing cache111 and obsolete R2b mutation anchors cannot support adoption of this stopped draft.

## C1 table — nine rows, draft labels and Node verdicts

All9 target endpoints appear in the first draft in all3 languages (27 cases). Labels below are observed or constrained by the complete full-row regression loop. These are draft observations, not accepted grading. The closure label is k3 (the enclosing line-level statement); the newly sourced default row is Exact while the regression expects SameLine. Both discrepancies remain explicit.

| C1 fixture | Def→Use bytes (.js) | Main label | Draft label | Node verdict |
|---|---:|---|---|---|
| multi-line short circuit |11→51|NameOnly(Killed k2)|NameOnly(Killed k3)|PASS:1|
| one-line short circuit |11→49|NameOnly(CfgIncomplete)|NameOnly(CfgIncomplete)|PASS:1|
| ternary |11→54|NameOnly(Killed k2)|NameOnly(Killed k3)|PASS:1|
| try write / g throws |11→70|NameOnly(Killed k2)|NameOnly(Killed k4)|PASS:1|
| uncalled nested closure |11→68|NameOnly(Killed k2)|NameOnly(Killed k3)|PASS:1|
| var array destructuring |11→28|Exact|Exact|PASS:1|
| for-in var |11→28|Exact|Exact|PASS:1|
| default-written formal |19→42|NameOnly(SameLine)|Exact, regression fails|PASS:5|
| Annex-B catch var |11→84|NameOnly(CfgIncomplete)|NameOnly(CfgIncomplete)|PASS:1|

`main-c1.bytes.jsonl` and `proto-c1.bytes.jsonl` retain full values, access/path/owner/end bytes and all rows. `red-c1.log` enumerates27/27 missing targets on the unmodified proto source plus the new test. `green-attempt-1.log` proves27 target endpoints restored but6 label assertions fail across3 languages. The existing nested RHS negative also fails: `function h(f, fn=null) { var f=(f=2,f); }` gains a formal11→collapsed0 row. That failure has no completed same-environment main attribution and is reported as a failing regression, not an asserted cumulative ADDED WRONG. No follow-on fixes were applied.

## Full corpus tables

No R3 corpus measurement was started before STOP. Historical R2b counts are not transferred.

| Corpus | ADDED C/W/U | LOST C/W/U | RELABELLED C/W/U | RE-OWNED C/W/U |
|---|---|---|---|---|
|X|NOT RUN|NOT RUN|NOT RUN|NOT RUN|
|Xi|NOT RUN|NOT RUN|NOT RUN|NOT RUN|
|T|NOT RUN|NOT RUN|NOT RUN|NOT RUN|
|SecBench|NOT RUN|NOT RUN|NOT RUN|NOT RUN|

LOST CORRECT=0 and ADDED WRONG=0 are **not established**. The additional synthetic source-selection population demonstrates6 LOST CORRECT rows, independent of corpus frequency. No TS oracle can override the exact Node/source fixture result; this is the parameter-environment blind spot requiring E12-style ES fixture authority.

## Gates, binaries and exclusions

- C1 pre-change regression:0 passed/1 failed,27 missing matrix rows; unchanged source is proto-bound.
- First callback suite:48 passed/2 failed of50. No full-suite result.
- Additional diagnostic:0 passed/1 failed; all3 languages enumerated, two missing sources per language.
- Synthetic Node:9/9 C1 cases and3/3 source-selection invocations PASS.
- `cargo fmt --all` completed before the final diagnostic test was added. No current fmt check after STOP.
- Nextest, doctests, mutants, clippy235/235, Tier-A matrix and TS/Node quick: **NOT RUN — mandatory STOP**. Rust quick likewise not run; the brief assigns its sandbox-limited rerun to the controller.
- Non-JS byte identity, call-site identity, nav removal proofs, SecBench/O1, performance and full corpus tables: **NOT RUN — mandatory STOP**.
- R3 production CLI/byte binaries: **NOT REBUILT — mandatory STOP**. The exact last compiled unit-test executable is frozen at `bin/prism-r3-stopped-unit-tests`, SHA256 `5743afb29824d8e3750e029bbe6e921434f5d33e8fadb6a6c10fa5f7847c07c1`. It is not a shipping binary. `bin/sha256.json`, controls-binding.json and stopped-source-manifest.json bind provenance.
- No Git writes, network installs, corpus execution, frontend-portal access, review dispatch or denied-command retries.

## Files and proposed custody commit messages

`R3-src.patch` is relative to f8c768b3 and retains the3 changed source files plus both R3 tests. Suggested **custody-only WIP** message: `wip(js-param-defs): retain stopped R3 RD fold and skipped-default reproducer`. Do not adopt or ship it.

`R3-docs.patch` is relative to35d780e1 and contains R3 report/handoff and current-authority supersession notices. Suggested message: `docs(js-param-defs): park PR-B on R3 skipped-default STOP`.

Evidence: `/Users/wesleyjinks/prism-evidence/js-param-defs/prB/repair-r3`. `STOP.json`, `PROBE-LOG.md`, `HANDOFF.md`, entry/stopped snapshots and final-custody-check.json preserve this retained artifact. Controller owns any Git custody commit. Owner must decide whether to reopen the parameter-source design; this worker made no extension decision.
