# Planning probes: S1b, span-verified JS/TS export and local routes

Grounding for `SPEC.md`. Every mechanism claim is labelled **MEASURED (Qn)**, **READ** (file:line on base
`a6d853f5`), or **ASSUMPTION**. Where reading and measurement both apply, the measurement is cited.

## Custody

- **Base.** `origin/main` `a6d853f5` (merge of #326, S1), clean tree. Release binary
  `prism-base-a6d853f5`, SHA-256 `df22fbcfd566d897a754e2e489a9408a6f751028cd5ed74b695f6adcc2eb808d`,
  `--version` → `slicing 3.1.2 (a6d853f5b399)`.
- **Evidence root.** `/Users/wesleyjinks/prism-evidence/s1b/planning/` (never `/tmp`), with `MANIFEST.sha256`
  (Q21). Binaries under `bin/`; corpus runs under `base-runs/` and `proto-runs/<variant>/`; controls under
  `controls/<variant>/`; logs under `logs/`; the private corpus identity is not in this packet.
- **Prototype.** A detached scratch worktree at `a6d853f5`, never pushed; the squashed diff is
  `prototype/s1b-prototype-v7.diff.txt` (scratch commit `b5737ea2`). Seven binaries were built (v1–v7, Q5–Q23).
  The corpus row-diffs were measured with v5; v6 adds a measurement switch and is identical with the switches unset
  (Q22); v7 adds the implicit `arguments` binding, which changes only callees named `arguments`, and no corpus has
  such a row (Q23). v7 is the authoritative prototype. It is a feasibility instrument, not the implementation: it
  has no new tests and no cache bumps, and it carries three measurement-only switches (`PRISM_S1B_ONLY_A`: S1b-1
  alone; `PRISM_S1B_USECALLBACK`: E4 = a; `PRISM_S1B_BASE_WRITE_SCAN`: the base module write scan).
- **Probe tools** (`probes/`). S1 copies: `rowdiff.py`, `controls_gen.py` (extended), `controls_summarize.py`,
  `honest_lines.py` (gained `--per-file`), `census.py`, `project.py`. New: `jsscope.py` (an independent lexical
  binding auditor in Python tree-sitter, same grammar versions as `Cargo.lock`), `s1b_census.py`,
  `audit_rowdiff.py`, `taxonomy.py`, `controls_diff.py`.
- **How independent the auditor is.** `jsscope.py` re-derives bindings from the ECMAScript scoping rules without
  linking prism, so it catches implementation slips (it found none on X, R and T beyond the parse-recovery class;
  Q12). It was written by the same planner as the prototype, so it shares the planner's *model* of the scoping
  rules. A modelling error common to both would not show up as a disagreement. The manual spot audits (Q12) and the
  reviewer probe classes (SPEC §7) are the check on the model itself.
- **Admissibility.** Every recorded probe exited 0 with empty stderr unless noted. Three probes failed for their own
  reasons and are recorded as inadmissible: the first C105 fixture (Q6), the first prototype suite run (a test
  literal did not compile, Q15), and the v1/v2 T runs (stopped for performance and superseded, Q8).

### Corpora (pins identical to the S1 packet)

| ID | Corpus | Pin | Verified |
|---|---|---|---|
| X | Excalidraw (public, React/TSX) | `~/prism-evidence/inputs/excalidraw-0642e72c/source` | Q0: 1,229 / 1,229 files match `source-file-manifest.sha256` (`1a6c4397…`) |
| F | private React app (JS/JSX/TS) | pinned commit, clean; identity only in the S1 private evidence file | Q0: HEAD equals the S1 pin, 0 dirty paths. **Aggregates only** |
| R | ruff `playground/` | `44f6d1865fd2…`, clean | Q0 |
| T | TypeScript `src/` | `7964e22f2b85…`, clean | Q0 |

## Mechanism

**M1. The ESM local export producers record a bare name** (READ `src/ast.rs:2739-2925`):
- export list `export { f as g }` with no source: `js_ts_forwarded_import(f).unwrap_or(Local(f))` (`:2806`);
- `export default f` (identifier, not a proven class): the same (`:2827`);
- `export [default] function f`: `Local(f)` (`:2859`);
- `export const|let|var f = <arrow | function expression>`: `Local(f)` (`:2891`); other initializers are
  S1's `SpannedLocal` or a counted skip.

Nothing checks what `f` denotes at module scope. After collection, a `Local` whose name is an import is poisoned
into `conflicted` (`:2473`).

**M2. R4c turns the name back into callables by name** (READ `src/resolution.rs:3665-3710`):
`functions.get(local_name)` filtered to the resolved file and non-methods; with S1's span it also filters by
`(start_line, end_line)`. With no span, 1 match is Exact, 2 or more are NameOnly (`:3430-3441`). So a `Local` over a
non-function binding binds any same-name function in the file, including nested and Pattern-2/3/5 over-named ones
(MEASURED Q3, C06, C68, C73). S1's JSX gate keys on `resolved.span.is_some()` (`:3686`), and S1's lowercase-tag
check lives only on this route (`:3689-3695`).

**M3. R3 `ImportQualified` is a stem match** (READ `src/resolution.rs:2548-2583`). For a qualifier `q` in the file's
import map, it takes every non-method function named after the member in any file whose stem equals the module's
last segment (or whose parent directory equals it), and returns them **all as Exact**. The import map
(`collect_js_imports`, `src/ast.rs:1666-1760`) holds default, named and namespace imports and `require`
bindings alike, and it recurses into every scope. MEASURED Q3: X has 15 R3 rows, 6 of them multi-target Exact
(fan-out 2, 9 and 49); T has 26; F 63.

**M4. JS namespace imports are `ModuleImport` bindings** (READ `src/ast.rs:2280-2310`), `eligible: false`. The
existing `receiver_lexically_bound` guard (`src/resolution.rs:2537-2546`) already skips R3 when the qualifier is
lexically bound at the call, which covers a module-scope competitor such as `var Lib` (MEASURED Q9, C112).

**M5. The relative-module resolver does not map `./x.js` to `x.ts`** (READ `src/call_graph.rs:6459-6515`). T's
imports all use `.js` specifiers, so T has 0 `import_member` rows (MEASURED Q2), and R3's stem heuristic happens to
land on the right file for T's namespace imports (`module_last.rsplit('.').last()` is the leftmost piece,
`src/resolution.rs:2556`). MEASURED Q3: T's 24 namespace R3 rows are right by manual audit.

**M6. R4 `LocalDef` binds every same-file free function of that name, all Exact** (READ
`src/resolution.rs:3460-3470`). There is no scope check, so a shadowing parameter, a nested function in another
scope, or a Pattern-2/3/5 over-named function binds, and 2+ candidates give a multi-target Exact row.

**M7. Function naming borrows names** (READ `src/languages/mod.rs:1157-1210`): Pattern 1 (declarator), Pattern 2
(object `key:`), Pattern 3 (any direct argument of a call whose parent is a declarator), Pattern 4 (class field),
Pattern 5 (assignment `a.b = fn`, `a = fn`). Only Pattern 1 (and a declaration's own name) is a lexical binding of
that name.

**M8. JSX elements are call sites** (READ `src/queries.rs:150-163`; the callee is the tag identifier, or the property
of a member tag, `src/languages/mod.rs:709-724`). Member tags carry a qualifier. `CallSite.jsx_element`
(`src/call_graph.rs:499`, `:5513-5523`) marks self-closing and opening elements. A lowercase plain tag binds today
on every rung: MEASURED Q3, X 36 Exact `free_single` rows (`<input>` → a test helper named `input`) and 26
NameOnly `free_multi` rows, F 96 + 1 + 6.

**M9. Existing scope helpers do not cover every binding form.** `collect_js_ts_parameter_bindings`
(`src/ast.rs:4955`) reads the `parameters` field through `find_parameters_node` (`:10592`), so a single unparenthesized
arrow parameter (`x => x()`, field `parameter`) is missed; `js_ts_has_closer_binding` (`:4690`) calls the same
helper (READ; the downstream effect on existing callers is not measured). S1b therefore uses its own collector
(SPEC §3.1), and C101 pins the arrow-parameter case.

**M10. The CJS terminal proof is the precedent** (READ `src/ast/js_cjs_terminal.rs:18-110`): a whole-file error
refuses, the name must have exactly one declaration, the indexed function must have the declaration's exact span,
and later root writes are ordered. S1b applies the same shape to ESM and to local bindings, but by span rather than
by file-wide name uniqueness, so it can re-target.

**M11. S1's admission predicate is reusable** (READ `src/ast/js_wrapped_export.rs:22-115`): `js_ts_wrapped_export`
accepts any `const` declarator (it does not require an export), so nested `const C = memo(fn)` and, with the
wrapper set extended, `const cb = useCallback(fn, deps)` go through the same R5/R6 provenance.

**M12. Resolution feeds the CPG.** CPG Step 5/5b consume `resolve_call_site` (READ `src/resolution.rs`
`filter_func_value_fanout` doc), so removing or re-targeting a `local_def` edge changes Step-5b argument→parameter
DataFlow and return flow. MEASURED Q18: X `dfg_label_exact` 33,946 → 33,919 and the return-flow counters move.

**M13. Telemetry.** The drop match is exhaustive (READ `src/navigation/queries.rs:385-400`); the JS export keys are
at `:630-660`. `js_export_barrel_conflicts` also counts the imported-local poison of M1.

**M14. Cache versions on base.** `CACHE_VERSION = 98` (`src/cpg_cache.rs:219`),
`NAV_CALL_EDGE_CACHE_VERSION = 54` (`src/navigation/call_edge_cache.rs:98`). S1 set both.

## Probe log

`B0` is the base binary, `Pk` prototype vk; every nav command uses `--no-cache`. Output paths are relative to the
evidence root.

| ID | Command | Expected (written before the run) | Actual | Output |
|---|---|---|---|---|
| Q0 | `shasum -a 256 -c ../source-file-manifest.sha256` in X; `git rev-parse HEAD; git status --short` in F, R, T | 0 mismatches; pins match S1; clean | 1,229 / 1,229 OK; all three pins equal S1's; 0 dirty paths | console |
| Q1 | `cargo build --release` at `a6d853f5`; `grep` cache constants | builds; 98 / 54 | builds; `df22fbcf…`; 98 / 54 | `bin/` |
| Q2 | `run_corpora.sh B0 base-runs` (`call-stats`, `--dump-sites`, `functions` per corpus) | rows as in S1 P1 | X 19,219; F 13,299; R 953; T 61,712 rows; stderr empty. T: 0 `import_member` rows | `base-runs/` |
| Q3 | `s1b_census.py <root> base-runs/<c>-dump-sites.jsonl census/<c>-census.json` (F with `--private`) | R4 is where static misbindings concentrate; `import_member` clean (S1 P5) | **`import_member`: 0 flagged** (X 2,516, F 160, R 46). **`local_def` flagged**: X 220 of 2,136 Source rows, F 184 of 735 (plus 6 intrinsic), R 1 of 96, T 980 of 18,348. **Intrinsic JSX tags with targets**: X 62, F 103. **R3**: flagged by the naive mirror on every corpus; manual audit (Q3a) | `census/*-census.{json,txt}` |
| Q3a | manual audit of every R3 row (X, T in full; F privately, aggregates) | — | X: 9 single-target rows plausibly right (member of a named-import object), 6 multi-target rows (≤1 right each). T: 24 namespace rows right (M5), 2 named-object rows. F: 4 non-relative namespace rows right (module-scope exports), 58 named-object rows, 1 multi | console; `census/F-r3-private-audit.txt` (private) |
| Q4 | S1 controls C01–C67 under B0 | identical to S1's `PP3-controls-proto.txt` | identical, 67 / 67 | `controls/base/` |
| Q5 | `controls_gen.py` (150 scenarios) under B0 and P1; `controls_diff.py` | `probes/S1b-controls-expectations-pre-run.md` | every S1b row as pre-registered, with three recorded deviations: C83's base drop is `UnknownName`, not `ImportExternal`; producer-side `local_def` rows in C11, C62, C69–C71, C80, C81 and C85 are also re-targeted (correct, not anticipated); 78 `UnknownName → JsxIntrinsic` relabels | `controls/proto-v1/` |
| Q6 | first C105 (`return <div>{</div>`) | drop | **inadmissible**: the error turned `App` into a root `ERROR`, so no call site existed on either binary. Replaced by C105 (`let x = ;`), C108 (JSX recovery) and C109 (a recovery that hides a closer declaration), expectations appended before the rerun; all three then as expected | `controls/proto-v1/` |
| Q7 | P1 on X, F, R; `rowdiff.py`; `audit_rowdiff.py` | every changed row audited | X 1,086 changed rows: 804 relabels, 134 re-targeted right, 148 removed wrong, **0 removed right**. F: 7 removed rows the auditor calls right | `proto-runs/v1/` |
| Q7a | H1 "the 7 F rows are parse-recovery refusals". Expect: the binding scope `has_error()` in all 7. Falsifier: any without an error (then a write-scan or collector bug) | 7 / 7 | **7 / 7 binding scopes contain a tree-sitter error.** The alternative (a write-scan false positive) is ruled out for these rows because the refusal fires before the write scan | console |
| Q8 | P1 on T | ≈ base time | **stopped**: over 5 minutes of CPU on `call-stats` with no output (an unmemoized per-site scope walk). Superseded by P2 (per-file memo keyed by `(scope id, name)`); P2 controls byte-identical to P1. P2 T `call-stats` took 4 m 38 s against base 4 m 01 s, both under load (Q19 has the clean timing) | `proto-runs/v2-NOTE.txt` |
| Q9 | addendum 2 (C110–C112) pre-registered, then P3 | C110 → top-level `g`; C111 drop; C112 drop | C110 and C111 as expected. **C112 falsified in detail:** base already drops it, because `var Lib` sets `receiver_lexically_bound` (M4). The namespace-provenance set added for it was removed (P4); P4 controls byte-identical to P3 | `controls/proto-v{3,4}/` |
| Q10 | Tier-A `--matrix-only --allow-stale-sut --sut-bin` B0 and P4, in the clean planning clone | 162 / 162 both | **162 / 162 both**, 0 regressions | `logs/tier-a-{base,proto-v4}-matrix.log` |
| Q11 | the same with the three candidate S1b fixtures (SPEC §7) added in the scratch worktree | base: 162 ok + 3 fail; proto 165 / 165 | **as expected**, which also shows the harness exercised the given binary | `logs/tier-a-s1bfix-*.log` |
| Q12 | P4 on X, F, R, T; rowdiff; audit; `taxonomy.py`; manual spot audit of 2 rows per class on X | 0 removed-right outside parse recovery | see §Results. **Removed-right rows are all parse recovery** (F 7, T 53). Manual spot audit agrees on every sampled X row (`<label>` → action `label:` arrows; `<line>` → the geometry `line()`; `updater()` where `updater` is a parameter; `unsub()` → the subscribed listener; the `abort` re-target) | `proto-runs/v4/` |
| Q13 | H2 "T's 53 removed-right rows are parse-recovery refusals". Expect: both files have a root `ERROR`. Falsifier: a clean file | 2 files | **both files carry grammar-gap errors** (`in out` variance annotations; a `symbol:` parameter). A narrower rule that ignores errors confined to other functions would keep only 12 of T's 53 and 3 of F's 7 | console |
| Q14 | P5 (imported-local parity, `ONLY_A` and `USECALLBACK` switches): controls; X, F, R, T under default, `ONLY_A` and `USECALLBACK` | default rowdiffs identical to P4; `ONLY_A` changes only module-scope rows; `USECALLBACK` keeps the `useCallback` rows | default rowdiffs **identical to P4** on X, F, R and T; controls identical to P4. `ONLY_A` touches exactly C06, C21, C62, C68–C83, C96, C97 and the intrinsic rows. `USECALLBACK` keeps X 11 / 11 and F 94 / 98 (the other 4 are parse-recovery refusals). `js_export_barrel_conflicts` back to base (X 13) | `proto-runs/v5*/`, `controls/proto-v5*/` |
| Q14a | T under P5 default, `ONLY_A`, `USECALLBACK` | `ONLY_A` empty; the other two equal | `ONLY_A`: 0 changed rows (`js_export_local_refusals {parse_recovery: 32}`, no edge impact: T has no `import_member`). Default and `USECALLBACK` identical: 1,033 rows (635 re-targeted, 398 removed). `multi_target_exact_sites` 680 → 6; `local_def` Exact edges 20,230 → 17,961; `js_export_barrel_conflicts` 64 unchanged | `proto-runs/v5*/T-*` |
| Q15 | `cargo test --offline --no-fail-fast`: base; P4 | base green; proto: only tests that pin changed representations | base **4,583 / 0 / 1** (29 binaries). First P4 run **inadmissible** (a `JsExportFacts` test literal lacked the new field). Rerun: **4,573 / 10 / 1**; all 10 are by-design pins (§Test impact); P5 removed one (E9a) | `logs/{base,proto-v4}-full-suite.log` |
| Q16 | `honest_lines.py <proto> HEAD --per-file` on the squashed scratch commit | — | v4 659 src; v5 673; v6 680; v7 **681 src / 11 tests**, of which about 14 src lines are the three measurement-only switches (design code about 667) | console |
| Q17 | S1b-a / S1b-b attribution of the prototype lines (per function) | — | SPEC §9 | console |
| Q18 | `nav --no-cache dfg-stats --repo X [--edges]`, B0 and P5 | only edges tied to changed `local_def` rows move | see §DFG | `proto-runs/v5/X-dfg-*` |
| Q19 | clean sequential timing of `call-stats` on T, B0 then P5, no other load | P5 within 15% of B0 | **B0 270.8 s, P5 259.0 s wall; user CPU 503.8 s vs 509.5 s (+1.1%)**: neutral | `logs/timing-T*.txt` |
| Q20 | full suite on P6 and on P7 | the 9 by-design pins fail, the rest pass | P6 **4,574 / 9 / 1** (29 binaries): exactly the 9 by-design pins. P7 **4,574 / 9 / 1**, the same 9. Tier-A with the S1b fixtures on P7: 165 / 165 (`logs/tier-a-s1bfix-proto-v7.log`) | `logs/proto-v{6,7}-full-suite.log` |
| Q22 | P6 with `ONLY_A` + `BASE_WRITE_SCAN` on X, F, R and all controls, against P5 `ONLY_A`; P6 default controls against P5 | identical | **identical** dumps and `js_export_local_refusals` on X, F and R, and identical control summaries; P6 default controls identical to P5. So S1b-1 can use the base `js_ts_module_value_written` (SPEC §3.2). T not run (0 `import_member` rows, 0 `written` refusals) | `proto-runs/v6-onlyA-basescan/`, `controls/proto-v6*/` |
| Q23 | addendum 3 (C113 implicit `arguments`, C114 TS `using`) pre-registered; Python grammar check of `using h = e;` first; then P6 and P7 over all 153 controls; count corpus rows whose callee is `arguments` or `eval` with targets | C113: P7 drops `f`'s call, keeps the arrow's; C114: both drop on P6 and P7 | **as pre-registered.** `using h = e;` parses in TS as `assignment_expression` with no error (in `.js` it is an error). P6 → P7 differs only in C113. Corpus rows with callee `arguments`/`eval` and targets: X 0, F 0, R 0, T 0, so the P5 corpus row-diffs stand for P7 | `controls/proto-v7*/` |
| Q21 | `shasum -a 256` of every evidence file | — | `MANIFEST.sha256`: 7,344 files, SHA-256 `de73ff88…a9bf` | root |

## Results

### Per corpus and per route (P5; rows, and target edges where they differ)

"Wrong" and "right" are static-binding verdicts from the auditor plus the Q12 spot audit. Removed edges are
further split (`taxonomy.py`): **W** statically wrong; **M** may-call (the binding holds a different callable that may
run the target: throttle, debounce, memoize, a rendering HOC, a test mock, or a binding written on some paths);
**P** pass-through (`useCallback`, which returns its argument).

| Corpus | Route | Current Exact in class | Wrong (audit) | Removed | Re-targeted | Net correct-edge change |
|---|---|---|---|---|---|---|
| X | D4 list/default/decl `Local` (`import_member`) | 2,516 rows | 0 | 0 | 0 | 0 |
| X | R3 namespace | 0 rows | – | 0 | 0 | 0 |
| X | JSX intrinsic (any rung) | 36 Exact + 26 NameOnly rows (166 edges) | 62 / 62 | 62 rows | – | 0 right lost; 166 wrong edges gone |
| X | R4 `local_def` (Source) | 2,136 rows (+9 `IndirectResolution`, unchanged) | 220 rows | 86 rows / 112 edges (W 33, M 68, P 11) | 134 rows (138 extra edges removed) | static: 0 right lost; may-call reading: −68; with `useCallback` admitted, P kept |
| F | D4 | 160 | 0 | 0 | 0 | 0 |
| F | R3 namespace | 4 (non-relative) | 0 | 0 | 0 | 0 |
| F | JSX intrinsic | 96 Exact + 1 NameOnly + 6 `local_def` rows | 103 / 103 | 103 rows | – | 0 right lost |
| F | R4 `local_def` | 735 rows (+6 intrinsic, above) | 184 rows (+7 right refused) | 191 rows / 196 edges: W 87, M 4, P 98, parse-recovery right 7 | 1 row (2 extra edges removed) | static: −7 (parse recovery); with `useCallback`: −11 |
| R | D4 | 46 | 0 | 0 | 0 | 0 |
| R | JSX intrinsic | 0 with targets (117 relabels) | – | 0 | – | 0 |
| R | R4 | 96 | 1 (`lazy` loader) | 1 | 0 | 0 |
| T | D4 | 0 (`.js` specifiers, M5) | – | 0 | 0 | 0 |
| T | R3 namespace | 24 (stem-resolved) | 0 | 0 | 0 | 0 |
| T | R4 `local_def` | 18,348 rows | 980 rows (+53 right refused) | 398 rows: W 245 edges, M 172, parse-recovery right 53 | 635 rows (1,799 extra edges removed) | static: −53 (parse recovery); may-call reading: −225 |

- **Relabels** (`UnknownName → JsxIntrinsic`, no edge): X 804, F 387, R 117, T 0.
- **Additions**: 0 on every corpus. The controls show the additions S1b can make (C78 `const f = function g`, C83
  namespace re-export, and the module-binding-audit namespace gap, Q15); none occur on the corpora.
- **R3 non-namespace rows** (named- and default-import qualifiers) are unchanged by design (SPEC §2): X 15, F 59,
  T 2.

### DFG (Q18)

MEASURED Q18: X `--edges` 60,163 → 60,136 lines: **27 removed, 0 added**. Every removed edge is an argument →
parameter DataFlow edge from a call whose `local_def` target S1b-2 removed or re-targeted away (joined by file, target span, and a call line within 6 lines of
the DataFlow source line). `dfg_label_exact` 33,946 → 33,919. All 27 attribute to S1b-2 rows; S1b-1's changes are
JSX sites, which carry no arguments (S1 M6). The `ONLY_A` DFG was not run separately. Output: `proto-runs/v5/X-dfg-{edges.jsonl,diff.txt}`.

### Performance (Q19)

MEASURED Q19 (sequential, otherwise idle machine): T `call-stats` base 270.8 s, prototype 259.0 s wall; user CPU
503.8 s against 509.5 s (+1.1%). The per-file `(scope, name)` memo is required: without it the prototype did not
finish T's `call-stats` in the time base needs (Q8).

### Test impact (Q15, Q20)

The ten P4 failures, each with its disposition:

| Test | Why it fails | Disposition |
|---|---|---|
| `js_export_test::extract_const_arrow_export`, `…const_function_expression_export`, `…default_export_identifier`, `…default_export_named_function`, `…named_export_list_with_rename`; `type_relative_receiver_test::indirect_default_class_facts_do_not_fall_back_to_callable_exports`; `cjs_refusal_state_test::cjs_refusal_raw_serde_and_esm_custody` | assert the fact is `Local(name)`; it is now `VerifiedLocal { … }` with the same local name | update the expected fact (representation only; resolution asserts in the same tests pass) |
| `js_wrapped_export_test::t_j2_t_j4_jsx_gate_scope` | S1 row 1 asserts `<island/>` drops `UnknownName`; it now drops `JsxIntrinsic` | update the reason (SPEC §0 E3) |
| `module_binding_audit_test::esm_namespace_import` | records `import * as ns; ns.item()` as a **Gap**; S1b resolves it (rule R3-1 through the export facts) | move the case to `Supported` (a closed gap) |
| `esm_forwarding_test::forward_type_declarations_and_recovery_refuse` | expected the imported-local poison in `conflicted` | **fixed in P5** (imported locals keep the base path, SPEC §3.2); not a test change |

## Not measured, and why

- **Semantic truth of the may-call class (M).** Whether a call through `throttle`, `memoize` or a HOC "calls" the
  argument is library semantics, not static binding. It is reported, not decided.
- **React's `useCallback` identity contract** is an ASSUMPTION grounded in React's documented API ("returns the
  function you passed" on the first render, the cached one afterwards), as S1's rendering contract was.
- **Sloppy-mode detection.** S1b does not detect `"use strict"` or module-ness; the Annex B rule refuses in both
  modes (SPEC §3.1), so no measurement depends on it.
- **`tier-a --quick`, `--features mcp`, clippy and the Node gate** were not run on the prototype; the implementer
  runs them (IMPLEMENTOR).
- **Incremental versus full rebuild** is argued (every S1b fact is file-local) and pinned by tests in the SPEC, not
  measured on the corpora.
