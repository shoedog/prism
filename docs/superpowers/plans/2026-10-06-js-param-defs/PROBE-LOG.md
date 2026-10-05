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
