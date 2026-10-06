> **PR-B R1 supersession:** owner STOP-1(a), O1 and O2 are resolved. Planner b10 measurement claims are historical and refuted by F1–F3/W1–W6. Active R1 hypothesis/probe/result log: `~/prism-evidence/js-param-defs/prB/repair-r1/PROBE-LOG.md`; fresh results and exceptions are in MEASUREMENTS-prB-R1.md and HANDOFF-repair-prB-r1.md.

> **R2 supersession:** Historical R1/planner record; current dispatch is committed `bd4c30ff` plus `repair-r2/R2-src.patch`, docs `a0e8504e` plus R2 docs patch. Use `HANDOFF-repair-r2.md` and `MEASUREMENTS-r2.md` for the targeted fold and current verification; R2 measurements and gates are complete on the documented successful-pair set; STOP is none.

> **R1 supersession (2026-10-05):** Historical prototype record. Its line-based all-CORRECT verdict is refuted by both spec reviewers. Active repair custody and byte-span binding evidence are in `HANDOFF-repair-r1.md` and `MEASUREMENTS-prA.md`; use the committed prototype `1b2dfdc9` plus R1 patches. Prior D11/D12 isolation and zero-WRONG claims do not apply to the repaired artifact.

# PR-A probe log (hypothesis → expectation → result)

Base binary: `prism-base-006573d9`, sha256 `d2babb6d…b7af`, built from `006573d9` (main + docs).
Head binaries, all built from the working-tree prototype:
- `prism-head-proto1` / `-prA` / `-prA2` are superseded intermediates;
- `prism-head-prA3` (sha256 `b8838d2ce9c663bc7ed9c80b30c25d7c21f329f840854789ab8b1c009e21e553`) is **final**; it adds D6 rest-last, D11 and D12.

P1–P10 ran on an intermediate head. P9 and P10 were re-run on prA3 with identical results.
Both are in `~/prism-evidence/js-param-defs/bin/`. Fixtures are under `/private/tmp/claude-501/pd/fx*`; every fixture's source is reproduced in the test files.

**P1. Base reproduces both gaps.**
- *Hypothesis:* on base, the bare-arrow and rest formals have no Def.
- *Expected if true:* a base/head `dfg-stats --edges` diff on fixture fx2 shows ADDED rows only, from `cmd`/`z`/`x` (bare) and `rest`/`items` (rest).
- *Falsifier:* base already has those rows.
- *Result:* confirmed. 12 ADDED, 0 LOST.
- Step 5b on proto1: `h(v)` produced `v → cmd` (Exact), and `r(v, v, v)` bound no argument to rest.
- The final prototype removes the bare-formal binding (D12, P11).

**P2. Grammar shapes (pinned tree-sitter, sexp probe).**
- *Hypothesis (EVALUATION §3.2):* TS has a `rest_parameter` node.
- *Expected if true:* the TS sexp of `function r(this: T, a: number, ...rest: string[])` contains `rest_parameter`.
- *Result:* **falsified**. TS/TSX give `required_parameter pattern: (rest_pattern (identifier))`.
- The other shapes:
  - JS: `formal_parameters (identifier) (rest_pattern (identifier))`;
  - bare arrow (JS/TS/TSX): `arrow_function parameter: (identifier)`;
  - `get => get` also parses `get` as an `identifier`;
  - `this: T` is `pattern: (this)`;
  - destructured rest is `rest_pattern (array_pattern …)` or `(object_pattern …)`.
- Consequence: SPEC D3.

**P3. Bare-arrow labels, compared with the parenthesised control.**
- *Hypothesis:* the new bare-arrow rows labelled `nameonly/cfg_incomplete` are an artefact of the new Def.
- *Alternative:* arrow block bodies are `cfg_incomplete` on base already.
- *Discriminator:* the base label for the identical `(cmd) => { exec(cmd); }` (fixture fx3).
- *Result:* base gives `(cmd)` `nameonly/cfg_incomplete`, and head gives the bare form the same. The alternative holds; the label is parity, not an artefact.

**P4. TSX rest label parity.**
- *Expected:* `function many(...items)` gets the same label as `function one(item)` in a `.tsx` file.
- *Result:* both are `nameonly/cfg_incomplete` (fx3). Parity holds.

**P5. Is the RD declaration seed needed (`scope.rs::declaration_seed`)?**
- *Hypothesis:* without `parameter_binding_region` there, the bare-arrow Def is seeded as a non-parameter binding and its labels diverge from the parenthesised control.
- *Expected if true:* rows differ between a T run without the scope change (`prism-head-noscope`) and one with it (`prism-head-proto1`).
- *Falsifier:* the two runs are byte-identical.
- *Result:* confirmed. 9 T rows were RELABELLED Exact → `nameonly/killed`, all `codefixes` `context => { const {…} = context; … }`.
- Fixture fx4 places the bare and parenthesised forms side by side:
  - base: parenthesised `context` 7→10 is `killed@8`;
  - noscope: bare `context` 2→5 is **Exact**;
  - proto1: bare 2→5 is `killed@3`, matching the control.
- Without the seed, prism would mint an Exact for a shape whose identical parenthesised form it refuses. Pinned by `bare_arrow_labels_match_the_parenthesised_control`. Mutant PD-10 is killed.

**P6. Is the second `scope.rs` site (`introduction_is_classified`) needed?**
- *Expected if needed:* mutating it back fails a test or changes rows.
- *Result:* each site was reverted in isolation.
  - Site 321 (declaration_seed) fails the parity test.
  - Site 588 passes all 14 tests.
- Mechanism: `collect_unclassified_binding_lines` skips the root callable's own fields (`node.id() != root_function_id`). The bare formal is the root arrow's `parameter` field, so it never reaches site 588. The change would be an equivalent mutant, so it was **reverted** (SPEC D5).

**P7. T RELABELLED Exact → `nameonly/sameline` (4 rows).**
- *Hypothesis:* the new param Def collides with a same-line declarator Def that the arrow's own pass already owned.
- *Mechanism:* `const file = ts.forEach(files, file => {…})`. The arrow is named `file` by name-inference pattern 3; its line-based `all_lines` include line 4451, so its pass already held the `const file` lvalue Def. The new param Def for `file` on the same line makes `collapsed_groups` mark `(file, 4451)` as SameLine.
- *Expected if true:* both affected T sites have this shape (`fourslashImpl.ts:4451`; `convertToEsModule.ts:88` `const changes = ChangeTracker.with(context, changes => …)`), and no other relabel exists.
- *Result:* confirmed. Both sites have the shape, and they are the only 4 RELABELLED rows.
- Checker (step 1): the Use on the next line binds to the **arrow formal**, not the const. Base's Exact came from the const's bytes, so it was a byte-level misbinding; head's NameOnly is the safe direction.
- Not a STOP; disclosed as pre-existing defect E1 in SPEC §6.

**P8. Static audit of `find_parameters_node` callers (SPEC D2).**
- There are 16 call sites (`grep -rn find_parameters_node src`).
- 12 iterate `params.children()` or `named_children()`; an identifier node would silently yield nothing.
- `ast.rs::collect_js_ts_parameter_bindings` (≈5056) has an `else if … child_by_field_name("parameter")` branch that a `Some(identifier)` would bypass. The S1b F3 bare-arrow binding would then be lost.
- `js_ts_parameter_receiver_binding` (≈3656) would turn `None` into an empty match list.
- Decision: keep `find_parameters_node`, and add `parameter_binding_region` at the 3 consumers that need it.

**P9. SecBench targets on the same harness.**
- *Run:* `probes/secbench_subset.py --select targets`, on base and on head, with main's `eval.secbench.run.measure` and the authenticated R1 inspection (`ba2f57c6…`).
- *Expected:* head traces the 6 rest rows; the 3 arrow rows at least gain their source binding.
- *Result, base:* 7 `reached_function_only` and 2 `prism_error`, matching R1.
- *Result, head:* 7 `traced`, 1 `partial`, 1 `reached_function_only`.
  - All 6 rest rows traced.
  - port-killer traced.
  - is-svg: source bound; next break is B-plain-argument.
  - portprocesses: prism_error becomes partial; next break is D-cjs.

**P10. Payload-specific BFS (r2-opus-A F5).**
- *Run:* `probes/payload_bfs.py` on head targets.
- *Result:* 7 of 7 traced credits reach their accepted sink from the payload formal Def without entering a sibling formal on the signature line.

**P11. First T adjudication (prA2 = without D11/D12) found WRONG rows.**
- *Run:* `adjudicate.cjs` over the 1,126 changed T rows.
- *Hypothesis A (oracle bug):* the WRONG rows come from a callable-start-line mismatch in the oracle.
- *Hypothesis B (real false rows):* the Use binds a different declaration.
- *Discriminator:* read each WRONG site.
- *Result: B.* Two classes, both inherited from pre-existing mechanisms:
  - **(i) Nested formals.** 5 def→use rows, 1 of them Exact:
    - `documentRegistry.ts:197` `.map(name => { … forEach((entry, name) => … name …) })`. The arrow is named by pattern 3.
    - `customTransforms.ts:146` `node => ts.visitNode(node, function visitor(node) {…})`.
    - The reference walk does not fence a nested callable's formals (E3).
  - **(ii) Step 5b into name-inferred callables.** 57 use→def rows; 26 Exact, only 1 checker-CORRECT.
    - `debug.ts:1057 Array(height)` → `inferFromUsage.ts:551 { Array: t => … }`.
    - `incrementalUtils.ts:493 readFile(fileName)` → `:477 compilerHost.readFile = fileName => …`, while the call actually reads the saved original.
    - `deprecations.ts:97 bind(args)` → `:144 { bind: binder => … }`.
    - The base call-site dump shows `Array` → `inferFromUsage.ts:551` is already an **Exact `free_single`** call edge. PR-A only filled the formal slot behind it (E5).
- *Fix:* D11 (nested-formal guard) and D12 (keep the bare formal a Step-5b hole).
- *Re-measure (prA3):* T 1,057 ADDED, all CORRECT; 402 Exact, all EXACT_OK.
- *Cost of the cut,* via a prA2→prA3 head diff: 65 rows.
  - 8 def→use rows: 5 WRONG and 3 correct (`197→198`, `197→214`, `146→147`).
  - 57 use→def rows: 1 correct Exact, 3 checker-WRONG NameOnly, and 53 undecided rows, of which most sampled Exact ones are wrong.
- SecBench targets are unchanged by the cut (P9 re-run on prA3: 7 traced, 7/7 payload-specific).

**P12. A pre-existing test found rest-not-last.**
- The full-suite run on proto failed 3 tests. Two were intended gap pins, updated per the IMPLEMENTOR table.
- `reviewer_optional_inert_complete_allowlist_and_old_path_controls` showed that `(seed = 0, ...rest, value?)` parses without recovery, and the head admitted `rest`.
- *Expected:* a rest that is not last is an early error, so it should be no Def.
- *Fix:* `js_ts_is_last_parameter` (D6), mutant PD-12, and the test `destructured_and_duplicate_rest_stay_refused`.

**P13. Gates on the final prototype (binary prA3 `b8838d2c…`).**
- `cargo nextest run --features mcp`: 5,152 run, 5,152 passed, 1 skipped.
- Authoritative full mutgate: 139/139 killed (7:04).
- Lane run: 18/18 killed.
- Advisory scoped (`--since origin/main --scope fn`): 19/19 killed.

**P14. SB step-2 PRIOR_WRITE rows (2) compared with plain-parameter controls.**
- *Hypothesis A:* the rest Def gets an Exact that the plain-parameter form would not.
- *Hypothesis B:* this is prism's existing Exact semantics (reaching on an unflagged route), so it is parity.
- *Discriminator:* the same shape with a plain formal, on base and head.
- *Fixture fx5:* `function pc(plugins){ if(…){ plugins = plugins[0] } return make(plugins) }` against `function pr(...plugins){…}`.
  - Base gives Def@1→Use@5 **Exact** for the plain form.
  - Head gives the rest form the identical Exact.
- *Full file:* js-data `Mapper.js`, with `...args` and with `args`. Head rest rows equal base plain rows line for line, including 1403→1449 Exact.
- *Result: B.* Disclosed as E6.
- Two further step-2 hits (`options = Object.assign({…}, options)`, `options = {…, ...options}`) were oracle false alarms: the Use sits in the write's own right-hand side. The oracle was corrected.

# PR-B probe log (hypothesis → expectation → result)

Base: main `da0604b3` → `~/prism-evidence/js-param-defs/prB/bin/prism-base-da0604b3` (sha256 `294b18d0…548f`); byte dumper built from the PR-B `probes/byte_dump.rs` against base (`prism-base-da0604b3-bytes`, `184812b3…6b0b`). Heads: `prism-head-b1…b4` superseded intermediates (deleted); historical b5 (`c6a0ddde…d9b6`); final planner head **b10** (`90e17676…1409`) + `prism-head-b10-bytes` (`fc07124d…a81da`); R1 repair evidence is separate. Fixtures: `/private/tmp/claude-501/pdb/fx*` (reproduced in `src/cpg/callback_identity_tests.rs`).

**PB1. Where do anonymous callables live? (census on prism predicates)**
- *Hypothesis:* most X/T anonymous callables sit inside named functions (React components), so a full callback pass duplicates legacy rows.
- *Falsifier:* most have no named ancestor.
- *Result:* mixed. X 1,993 / 4,497 under a named callable, 2,504 with no named ancestor (jest `describe/it` trees); SB 30,384 / 68,828 under named. All 97 SecBench callback sources are top level. → design must handle both: no legacy owner (pure gain) and legacy owner (E9 duplicates).

**PB2. Is a synthetic pass the legacy algorithm?**
- *Expectation:* `f(function(req,res){ var q = req; exec(q) })` gives the same rows as `const handler = function(req,res){…}` on base, including legacy artefacts.
- *Result:* identical wire rows, including the alias twin `req@2 → req@1` (a Use at the formal's own declaration). Parity holds; the artefacts are E4/E11 (later filtered for new passes, PB6).

**PB3. Re-own or duplicate?**
- *Hypothesis A:* fencing the enclosing named pass out of nested callbacks (re-owning) is safe.
- *Falsifier:* a taint path that needs a capture read and a callback-local Def under one owner.
- *Discriminator:* `trace.rs::taint_neighbors` AssignmentPropagation filters `def_fn == function && def_start_line == function_start_line`.
- *Result:* A falsified by mechanism: `function h(cmd){ xs.forEach(i => { const c = cmd + i; exec(c) }) }` — the capture Use of `cmd` stays `h`-owned, a re-owned Def `c` would be callback-owned, so `cmd → c` stops. Chosen: own-scope synthetic passes, legacy coverage unchanged (B-D2). RE-OWNED rows are then 0 by construction (measured).

**PB4. First prototype (b1) on X: is every LOST row the E3 fence, and WRONG?**
- *Expectation:* LOST rows only from the fence; all checker-WRONG; RE-OWNED 0; call sites identical.
- *Result:* LOST 2,689 (all fence), RE-OWNED 0, call sites identical. The original adjudicator returned 2,464 WRONG + 225 UNDECIDED (`alias_shape`). Adjudicator extended (alias twins incl. destructuring, flow-insensitive twins = WRONG E11, member/`this`/global bindings, E7 positions). Final: **2,689 / 2,689 WRONG**.
- RE-OWNED pairing positive control: a one-row base `h` / head `<cb@1:1>` pair with identical bytes is reported `RE-OWNED: 1` by `rowdiff.py` (fixture `rb/rh.jsonl`).

**PB5. X build time +45 % (STOP threshold 25 %): fence or passes?**
- *Hypothesis A:* the E3 fence walk is the cost. *Alternative B:* the ~4.5k synthetic passes.
- *Probe:* same binary with the fence disabled vs synthetic identity disabled (env toggles in a throwaway build, removed afterwards).
- *Expected if A:* no-fence ≈ base. *Result:* no-fence 55.0 s ≈ head 55.7 s; no-synth 38.0 s ≈ base 38.2 s → **B**.
- *Second probe:* `sample` of the head DFG build: `reaching::classify_edge → BindingFacts::relation → use_byte → rvalue_identifier_spans_on_lines`, and `cfg::build_cfg_edges_with_arms` (whole-file CFG) per pass.
- *Fix:* B-D13 memo (file CFG once per file; `use_byte` per pass). DFG phase X: base 19.2 s → memo-only 6.6 s → head 10.5 s. Head build is now faster than base (perf table in MEASUREMENTS).

**PB6. b1 ADDED WRONG classes on X (101 WRONG + 290 UNDECIDED of 16,433)**
- *Hypothesis:* they are inherited legacy mechanisms, not identity bugs.
- *Result (b1 rows, second adjudicator pass, then source inspection):* 94 WRONG = 33 E7 non-reference Uses (JSX attribute names `elements={elements}`, pair keys, callee properties `match.match(…)`, `console.error`) + 61 symbol mismatches (E10 block-scope escape: `const selectedElements` reaching a sibling block, `for (const path of …)` reaching the `path` import; TS index-signature `[key: string]`); 231 UNDECIDED, mostly alias twins later classified as flow-insensitive E11; E4 Uses at `let r;` / `r = …` LHS among the collapsed rows. No owner/identity error.
- *Action:* synthetic-only filters B-D9…B-D12 (legacy unchanged). Historical b5: **X 15,413 ADDED, all checker-CORRECT**; final b10 has **15,415** (R1 review refutes the zero-defect claim), 0 `USE_NOT_READ`.

**PB7. Use-byte selection for synthetic owners (NOT probed)**
- *Concern:* the legacy "first function on the start line" pick in `scope.rs::use_byte` may resolve a zero-width Use of a synthetic pass against another callable that starts on the same line.
- *Status:* not measured. b3 briefly used an owner-exact pick; I could not construct a fixture where the result differs (reasoning: the rvalue filter is per path and the `identifiers_on_line` fallback picks the unique same-name identifier), so it would have been an unkillable mutant and was reverted to the legacy pick. Recorded as SMELL S-B1 for review; it can only change a label, never add an edge.

**PB8. SecBench callback rows on unmodified packages**
- *Expectation (brief):* PR-B alone converts few; the population is joint with PR-C.
- *Result:* base 96 prism_error + 1 reached_function_only; head harness identical (96 prism_error: `callees --location` is `LocationOutOfRange` at an anonymous callable, nav unchanged). Callee-tolerant (classify with empty callees): head 2 partial + 4 reached_function_only. 95/97 payloads are member-only; the 2 bare ones (`bitty`, `shit-server`) stop at an outgoing call from a top-level callback (no call site).

**PB9. Joint counterfactual: is PR-B's identity the missing half?**
- *Probe:* insert one `;void <payload>;` as the first statement of each anonymous source callback (R1 `member` checkpoint without the R1 naming rewrite), remap row bytes, same harness `measure()`.
- *Expected if PR-B is the missing half:* head traces ≈ R1 joint 92, base ≈ 0.
- *Result:* callee-tolerant traced **head 90 / base 1** of 97 (harness 96 prism_error both, O1). Payload-specific BFS 90/90; 89/90 sources have a synthetic owner; 75 distinct source files.

**PB10. Intended re-pin found by the full suite**
- `js_param_defs_tests::curried_arrows_bind_only_the_named_outer_formal` pinned "anonymous inner arrow gains no Def (PR-B)". Re-pinned: `y` is owned by `<cb@1:20>`; `curry` still owns only `x`. No other existing test changed.

**PB11. `call-stats` summary: why did `return_flow.return_input_edges` drop (X 3,863 → 3,789)?**
- *Hypothesis A:* synthetic owners gained or lost return-flow participation (an E5/return-flow leak).
- *Alternative B:* the E3 fence removed zero-width Use anchors of named callees that existed only as endpoints of fenced (checker-WRONG) rows; `assemble_step5c_return_flow` links every callee-owned Use inside a return value to its `ReturnValue` node.
- *Discriminator:* dump every ReturnInput edge on base and head (`probes/return_inputs.rs`) and join the removed ones with the LOST rows' Use endpoints.
- *Result:* B. 74 removed, 0 added; all 74 are zero-width anchors that are Use endpoints of LOST rows. `return_flow_edges` (2,715), every skip/suppression counter and every call site are unchanged. A synthetic owner is never a ReturnValue owner (never a callee).

**PB12. Eligible-sweep regression `prototype-pollution/dot-prop_2.0.0` (traced on base, reached_function_only on head)**
- *Hypothesis A:* the E3 fence removed a checker-correct row on the trace (a LOST-correct STOP).
- *Alternative B:* the base trace rested on an E3 false edge.
- *Discriminator:* the base witness `parameter_results` and the byte checker on that package's LOST rows.
- *Result:* B. Base credit is `set(obj, path, value)` formal `path@24 [396,400]` → sink `path@32`, which sits inside `pathArr.forEach(function (path, index) {…})` and binds the callback's own formal. The byte checker marks all 7 LOST rows of the package WRONG `declaration_span_mismatch`, including `path@24 → path@32`. Head correctly owns line 32's `path` under `<cb@30:18>`; the real flow needs a forEach invocation edge (non-goal). Spurious base credit withdrawn, not a lost capability.

**PB13. SecBench ADDED WRONG rows (b6: 1,018 of 255,962): product defect or oracle artefact?**
- *Hypothesis A:* new identity/containment bugs. *Alternative B:* inherited mechanisms plus TS-checker artefacts specific to JS files.
- *Probe 1:* `define(["require","exports"], function (require, exports) { exports.a = 1 })` through the pinned checker: `getSymbolAtLocation(exports)` in `exports.a = …` returns the synthesized CommonJS export symbol (a `PropertyAccessExpression` declaration), not the parameter; `getSymbolsInScope` returns the parameter. → 509 `declaration_span_mismatch` + most `symbol_mismatch` were this artefact (AMD/UMD `exports`/`module`). Adjudicator: lexical re-resolution for `exports`/`module`; free `exports` is one binding.
- *Probe 2 (remaining 255):* `var PromisePolyfill` declared three times in one IIFE (mithril) and `function validate` + `var validate` in one factory (json-schema) are one ECMAScript binding that TS's JS binder splits → adjudicator `varMerged`. Real product residue: `this.x` Defs reaching `this.x` inside nested non-arrow functions (another receiver), `for (var k in o)` inside a nested callable not seen as a function-scope binder, flow-insensitive alias twins on minified lines → three synthetic-only fixes (b10: `js_ts_this_rebound_between`, for-in `var` in `js_ts_function_scope_binds`, `js_ts_own_alias_target`).
- *Result (b10, final adjudicator):* 255,874 CORRECT, **12 WRONG** (`alias_flow_insensitive`, lodash/phpjs minified lines where two same-name assignments/lvalues share a line: the twin is keyed by `(name, line)` and attaches to the non-alias lvalue). Bounded fix for review: key synthetic alias twins by the lvalue span's start byte instead of its line. Not applied (STOP-1 pending; no further code churn).

**PB14. The single LOST UNDECIDED row (lodash_4.17.15 `x` Def `r@75 [40495]` → Use line 74)**
- *Hypothesis A:* the fence dropped a correct occurrence (LOST-correct STOP). *Alternative B:* base admitted only wrong occurrences on line 74.
- *Probe:* temporary in-crate test (removed) over the real file: legacy `find_path_references_scoped` has line 74, fenced walk does not; `js_ts_reference_fenced` is **false** for both correct occurrences (40393 `var …,r=…`, 40437 `!r`) — they are excluded in base and head alike by legacy `is_shadowed_at`, whose block heuristic treats the block-level `var r` as block-scoped. The only base-admitted occurrences were 40211/40247, the parameter `r` of the sibling function `Ru(n,t,r)`, which the checker binds to that parameter.
- *Result:* B. The LOST row stood for checker-WRONG occurrences only; the oracle's UNDECIDED came from counting occurrences the base walk never admitted. Mechanism-level proof → WRONG; **LOST correct = 0**.

**PB15. STOP-1: nav byte identity**
- *Expectation (B-D7):* no nav byte changes; synthetic Variables filtered.
- *Result:* X 1/241, T 2/81 sampled queries differ, all `ego` (b6). X's diff = 7 zero-width Use anchors and their edges removed by the E3 fence (LOST WRONG rows). Synthetic names never appear. The fence (must-address 2) and nav identity conflict; reported as STOP-1, not chosen.

**PB16. Final perf (b10 vs base, `perf_b.py`)**
- *Expectation:* head ≤ base build time (memo), X RSS up with rows.
- *Falsifier:* any corpus > +25 % build time.
- *Result:* X ×0.75, T ×0.63, lodash ×0.57 build time; RSS X ×1.22, T ×0.72, lodash ×0.50; RD over-cap unchanged (0/0/2). Host load ≈ 9 from other sessions on both sides (alternated runs).
