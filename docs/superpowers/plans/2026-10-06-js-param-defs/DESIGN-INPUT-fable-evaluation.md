# Independent evaluation — three prism lanes in one controller, and the JS/TS parameter-Def gaps lane

**Written:** 2026-10-05 · **By:** Fable 5.1 (independent evaluator, read-only) · **Repo read:** `~/code/prism-secbench` at `2777599b` (= `origin/main`) · **Evidence read:** the three controller handoffs, `survey/synthesis.md`, SecBench `R1.md` + product probes, reviews `r1-{opus,sol}-A.md` / `r2-opus-A.md`, Tier-A `r1`/`r2-readout.md`, `OQ-s2.md`, `pipeline-lessons.md`, the memory files MEMORY.md links, global + repo `CLAUDE.md`. No git writes; `frontend-portal` not opened.

Every mechanism claim below names the file and symbol I read. Where I did not run a probe, the claim is tagged `[STATIC]`.

---

## 1. Verdict: can one controller session run all three lanes concurrently?

**It can be made to limp; it should not.** Run **lane 1 (parameter-Def gaps) in this session**, **lane 3 (measurement) in a separate session now**, and **lane 2 (dist/build admission) after lane 1's cache bump merges — and only after lane 3 has measured it**. Lane 2 is not the small, independent slice it looks like (§1.3).

### 1.1 Context and compaction risk
Session `0ec85e7b` ran S2, PKG, MEAS-A and MEAS-B between 2026-10-03 and 10-05. The handoffs show the cost: `HANDOFF-controller-2026-10-04.md` has five stacked "CURRENT STATE v1..v5" blocks, and `-04b.md` four UPDATE blocks, because state changed faster than compaction. Two controller errors in that window are classic context-loss errors, both self-recorded: the S2 brief applied S1b-4's "head never adds a target" rule lane-wide (handoff 10-03 §3), and the R2 brief put all of F4 out of model when only the dynamic half was (handoff 10-04). Pipeline lesson 17(4) already says: past ~80% of a window, dispatch to a fresh context. Three concurrent lanes each with planner → prototype → F → spec review ×2 → repair → impl → impl review ×2 → mutgate → merge is roughly 3× the state the session just failed to hold cleanly for two lanes.

### 1.2 Shared clones and disk
`df`: 125 GB free of 926 (87% used). Idle clones still carry `target/`: `prism-s2-plan` 17 GB, `prism-tiera` 5.8 GB, `prism-secbench` 4.6 GB, `prism-pkgres` 3.7 GB; twelve `prism-*` clones exist. Handoff 10-03 §5: "Disk has filled up four times." Each lane needs a planner clone, an implementer clone and two reviewer clones, each building release + nextest (~4–6 GB). Three lanes ≈ 12 live clones ≈ 50–70 GB plus SecBench run output and Tier-A SUT caches. That is inside the margin only if every idle `target/` is deleted first and nobody runs the full mutgate in two clones at once.

### 1.3 Cache and branch conflicts — lanes 1 and 2 collide by construction
- `src/cpg_cache.rs:236` `CACHE_VERSION = 106`, bumped **12 times since 2026-09-16** (`git log -G"CACHE_VERSION: u32 = "`), i.e. once per slice. Lane 1 changes DFG rows and function identity → must bump. Lane 2 changes `BUILTIN_SKIP_DIRS` (`src/repo_loader.rs:15`) → must bump `SKIP_POLICY_VERSION` (`cpg_cache.rs:238`, part of the cache key) and, if the file universe changes the scope graph, `CACHE_VERSION` too. Two open PRs editing the same constant and the same doc-comment list guarantee a hunk conflict on the exact lines that lesson 18 (whole-file `--ours` silently discarded auto-merged sidecar fields) was written about. Pipeline lesson 10: one cache transition per PR. Corollary: **one open PR owns a cache-key constant at a time.**
- Lane 2 is also not precision-neutral `[STATIC]`. Admitting `dist/`/`build/` admits a second copy of every exported symbol in packages that ship both `src/` and built output (the Tier-A r1 readout already had to exclude "browser/minified dist duplicates" from the Node stratum). The resolution ladder demotes a single in-repo owner and **drops** multi-owner collisions (CLAUDE.md "precision floor"; `free_single` in `src/call_graph.rs`). So blanket admission can turn existing Exact callers in `src/` into dropped collisions — exactly the lesson-17 shape (an edit that passes the suite and removes Exact edges en masse), detectable only by the same-base `call-stats` control. It also admits minified bundles, which is where the SecBench 120 s timeouts and "repeated CPG cache-miss rebuilds" live (R1.md, timeout section). Lane 2 therefore needs a measured admission predicate (e.g. admit a built directory only when `package.json` `main`/`exports` points into it **and** no source sibling resolves) and a census of collisions on X/T/F before any code. That is a lane-3 measurement item first, a product slice second.
- Branch conflicts between lane 1 and lane 3 are nil if lane 3 is held to `eval/`, `docs/`, `scripts/` (MEAS-A and MEAS-B both shipped with empty product diffs; keep that rule).

### 1.4 Reviewer and model-lane load
Standing pair Opus-5.5 + gpt-6.1-sol, cap 2 (owner 2026-09-30). Per lane: spec review (2 reviewers × ≤2 rounds) + impl review (same) + sol repairs ≈ 8–12 dispatches, each a clone and a handback the controller must read. Three lanes ≈ 30 dispatches in flight across a few days. Two known failure modes compound at that volume: the OpenAI cyber filter killed a sol repair on MEAS-A because the corpus is exploit code (handoff 10-04b UPDATE c), and codex "model at capacity" retries. Lane 3 touches exploit inputs continuously; lane 1 does not.

### 1.5 The private-corpus F bottleneck
Only the controller runs F (`~/code/frontend-portal`; handoff 10-03 §5 "Never open the private corpus F in reviewer or planner briefs"). The F script (`CONTROLLER-s2.sh`) needs a pinned base binary, a freshly built head binary, a NEW `PRIVATE_EVIDENCE_ROOT`, and a stable tree for the run's duration (lesson 12: edits during a live run invalidated a full-corpus run). Lane 1 needs F per gap (§3.6), lane 2 needs F for the admission census and collision diff, lane 3 needs F for the React-props/destructuring census (F is the only React corpus). These are serial on one machine and one person-equivalent of attention. This is the hard constraint that makes "all three concurrently" fiction: the lanes would simply queue on F inside one context.

### 1.6 Owner attention
S2 consumed nine owner decisions (S2-O1…O9) in two days. Lane 1 carries at least three owner-level decisions (§3.7: member-only accepted cost; synthetic identity visibility in nav/MCP; PR split). Lane 2 carries one (admission predicate). Lane 3 produces decisions rather than code. Three lanes in one session means the owner's escalations arrive interleaved from one voice with no lane isolation — the controller handoffs already show that owner questions were buried under in-flight state.

### 1.7 Failure modes this project has already paid for
- **Yield collapse from refusal cuts:** lane P ×3 (`feedback_measure_cut_yield_first`), S2 ×6 (`OQ-s2.md`: "Yield has dropped to 0 six times"). Lane 1's member-only guard is a refusal; relaxing it is the inverse — a yield that may bring false flows. Both directions must be measured per gap.
- **Over-claimed readouts caught by review:** Tier-A r1's "most TS sampled members resolve to nonconcrete" was refuted by r2 (10/63 real); MEAS-A r1 drew six WRONGs. Every readout in lane 1 needs the two-reviewer round, not a narrow confirmation.
- **Fix-delta defects:** 10 of 11 codex re-reviews found a defect in the fix itself (lesson 1).
- **Silent capability deletion passing every gate:** lesson 17 (interface dispatch 1761→42).
- **Custody:** implementer commits single-copy for 6 h (`feedback_bridge_model_lanes`); codex sandbox cannot write `.git`.

### 1.8 Recommendation
| Lane | Where | When | Why |
|---|---|---|---|
| 1 Parameter-Def gaps | **this session** | now | product slice; needs F custody and the owner's model decisions; the only lane that bumps `CACHE_VERSION` |
| 3 Measurement (Node ingress/async, React props/hooks, server-side destructuring) **plus** the lane-2 admission census | **separate session B** | now, in parallel | eval/docs only; no product diff; no cache bump; its own clones and evidence root; needs one controller F run per census (request protocol §2.2) |
| 2 dist/build admission | **this session, or session C** | after lane 1 merges and lane 3's admission census is in | second cache transition; admission predicate chosen on measured collision/timeout cost |

---

## 2. Hand-off process for separate sessions

### 2.1 Ownership table (write it into every handoff §0(a) and §6)
| Resource | Session A (controller-1, lane 1) | Session B (lane 3) | Session C (lane 2, later) |
|---|---|---|---|
| Branches | `plan/js-param-defs*`, `proto/js-param-defs*`, `feat/js-param-defs-*` | `meas/*` only; product tree untouched | `plan/dist-admission`, `feat/dist-admission` |
| Files | `src/**`, `mutants/js-param-defs.json`, `docs/superpowers/plans/2026-10-06-js-param-defs/` | `eval/**`, `scripts/**`, `docs/superpowers/plans/2026-10-06-meas3/`, `docs/eval/**` | `src/repo_loader.rs`, `src/cpg_cache.rs` (bump only after A merges) |
| Cache-key constants | owns `CACHE_VERSION` (106→107) and `NAV_CALL_EDGE_CACHE_VERSION` (62→63 if nav output changes) while its PR is open | none | owns `SKIP_POLICY_VERSION` (and `CACHE_VERSION` 107→108) after A's merge |
| Clones | `~/code/prism-pd-{plan,impl,rev-opus,rev-sol}` | `~/code/prism-m3-{plan,rev-opus,rev-sol}` | `~/code/prism-adm-*` |
| Nav/CPG cache dir | `--cache-dir ~/prism-evidence/js-param-defs/cache` | `--cache-dir ~/prism-evidence/meas3/cache` | own dir |
| Evidence root | `~/prism-evidence/js-param-defs/` | `~/prism-evidence/meas3/` | `~/prism-evidence/dist-admission/` |
| Handoff file | `~/prism-evidence/HANDOFF-controller-<date>.md` (template `~/.claude/handoff-template.md`) | `~/prism-evidence/HANDOFF-meas3-<date>.md` | `~/prism-evidence/HANDOFF-adm-<date>.md` |
| F runs | **runs them** | requests them | requests them |
| Merges | its own PRs on clear + green | eval-only PRs on clear + green (may merge any time; never conflicts with A) | after A |

Rules: (1) a session touches only its file set; a cross-set need is an owner question, not an edit; (2) before dispatching anything, `df -h` ≥ 60 GB free and delete idle `target/` dirs you own; (3) tag every `codex exec` with `-C <your clone>` so `pgrep -fl "codex exec"` shows ownership; (4) refresh your handoff at every stable point, newest block first; (5) merge order A → C; B is independent.

### 2.2 F-run custody rule (controller-only, serialized)
Requesters write `~/prism-evidence/f-requests/<lane>-<n>.md` containing: purpose and decision served; base sha + binary sha; head sha + binary path; script path (a `CONTROLLER-*.sh`-shaped, aggregate-only script committed on the requester's branch); expected aggregate schema; `PRIVATE_EVIDENCE_ROOT` name. The controller runs **one F at a time**, with no tree edits or rebuilds in the SUT clone during the run (lesson 12), and writes the aggregate JSON plus a one-line `[MEASURED]` receipt back to `f-requests/<lane>-<n>.result.json`. Only aggregates leave `~/prism-evidence/<lane>/`. If the owner prefers, session B may be granted F custody for **read-only census queries** (`nav dfg-stats`, `call-stats --dump-sites`) with the same aggregate-only rule — the risk is lower than a base/head diff, but it is the owner's call, not mine.

### 2.3 Recommended models per role
| Role | Model | Why |
|---|---|---|
| Planner (lane 1 spec + LLD) | Opus-5.5 subagent (owner 2026-09-25) with gpt-6.1-sol as the second spec reviewer | Opus is strongest on value-flow/semantics audits (reviewer experiment 2026-09-30); lane 1 is a DFG-semantics change |
| Planner/implementer (lane 3 harness) | **Opus-5.5**, not sol | MEAS-A's sol repair was killed by the cyber filter on exploit inputs; lane 3 reads exploit files continuously |
| Implementer (lane 1) | Sonnet-5.5 (owner 2026-09-29) | standing; the brief carries the WHY so it can catch prescription bugs (lesson 16) |
| Repairs | gpt-6.1-sol xhigh (`codex exec … -s workspace-write`) | standing; strongest on grammar/escape classes |
| Reviewers | Opus-5.5 ∥ gpt-6.1-sol, cap 2; narrow fold-confirmation = one reviewer | owner's standing pair |
| Re-plan on REJECT or non-convergence | Fable | owner 2026-09-25 |
| Blind adjudication labeller (§3.5 step 5) | the **owner** for the 40-row sample (~1 h), else gpt-6-astra or kimi-k3 | the labeller must not be the implementer or either reviewer; Sonnet's κ 0.36 on MEAS-A shows outcome labels are unreliable — the sample must ask mechanism questions |

### 2.4 Ready-to-paste prompts

**Session A resume (lane 1):**
```
You are the prism controller for lane "JS/TS parameter-Def gaps" ONLY. Read, in order:
~/prism-evidence/HANDOFF-controller-2026-10-04b.md, ~/prism-evidence/fable-eval/EVALUATION.md §3,
~/.claude/CLAUDE.md, repo CLAUDE.md. Lane 3 (measurement) runs in a separate session with its own
handoff ~/prism-evidence/HANDOFF-meas3-*.md and never edits src/; lane 2 (dist/build admission) is
deferred until this lane's cache bump merges. You own CACHE_VERSION (106->107) and
NAV_CALL_EDGE_CACHE_VERSION (62->63 only if nav output changes); one transition per PR. Only you run
private F; serve ~/prism-evidence/f-requests/*.md one at a time, aggregates only, no tree edits during
a run. Clones ~/code/prism-pd-{plan,impl,rev-opus,rev-sol}; evidence ~/prism-evidence/js-param-defs/.
Models: Opus-5.5 plans; Sonnet implements; gpt-6.1-sol repairs; Opus-5.5 + gpt-6.1-sol review (cap 2);
Fable re-plans on REJECT. Measure-first per EVALUATION.md §3.6; PR order §3.6 (shapes -> callbacks ->
member-only). STOP for the owner on: any accepted-cost or model change (member-only guard), a same-base
control that is not byte-identical on non-JS/TS corpora, an Exact-edge mass drop, or open-class findings
at the cap. Refresh ~/prism-evidence/HANDOFF-controller-<date>.md at every stable point.
```

**Session B (lane 3 measurement):**
```
You are a prism measurement controller for lane "meas3": measure what is still unmeasured so the owner can
rank JS/TS work on data. Product tree is READ-ONLY for you: edit only eval/**, scripts/**, docs/eval/**,
docs/superpowers/plans/2026-10-06-meas3/**. Never bump any cache version. Never open ~/code/frontend-portal;
request F census runs via ~/prism-evidence/f-requests/meas3-<n>.md (see EVALUATION.md §2.2). Clones
~/code/prism-m3-{plan,rev-opus,rev-sol} off origin/main; branches meas/*; evidence ~/prism-evidence/meas3/;
cache --cache-dir ~/prism-evidence/meas3/cache. Read first: ~/prism-evidence/fable-eval/EVALUATION.md §1.3
and §2, docs/superpowers/plans/2026-10-04-secbench-ground-truth/R1.md, the r2-opus-A review, Tier-A
r2-readout.md, ~/prism-evidence/survey/synthesis.md, ~/.claude/CLAUDE.md. Deliverables, each a plan-doc
readout with [MEASURED] receipts and a two-reviewer round (Opus-5.5 + gpt-6.1-sol, cap 2):
 (1) Admission census for dist/build: on X, T, SecBench and (via F request) F — files under dist/|build/,
     minified share, symbols that would collide with a source sibling, projected Exact-edge loss under the
     current precision floor, and build-time/timeout risk. Output: a recommended admission predicate.
 (2) Node ingress/async beyond callbacks: SecBench seeds the handler formal directly; design and run a
     bounded GT mode that seeds at the real ingress (http/express/connect registration site, event
     'request') and counts how many of the 136 HTTP rows need ingress recognition vs the handler formal.
 (3) React props/hooks: on X and (F request) F, census of props access shapes (destructured params,
     props.x member-only, useState/useEffect captures) and which of lane 1's four gaps each would need.
 (4) Server-side destructuring: pick one modern Node corpus (owner approves the choice) and run the
     SecBench inspector census for destructure/rest/spread parameters.
Use Opus-5.5 as planner/implementer here (sol repairs on exploit inputs were killed by the cyber filter);
sol reviews. Write ~/prism-evidence/HANDOFF-meas3-<date>.md from ~/.claude/handoff-template.md at every
stable point. Tag every claim [MEASURED]/[INHERITED]; no readout claims beyond its denominator.
```

**Session C (lane 2, after lane 1 merges):**
```
You are the prism controller for lane "dist/build admission" ONLY. Preconditions (verify, do not assume):
lane 1 PRs merged on main with CACHE_VERSION 107; ~/prism-evidence/meas3/admission-census readout merged.
You own SKIP_POLICY_VERSION (2->3) and CACHE_VERSION (107->108), one transition per PR. Implement the
admission predicate the census recommended (default: admit a built directory only when package.json
main/exports points into it AND no source sibling resolves; never admit minified bundles); byte-identical
same-base controls on Go/Rust/Python corpora; same-base `nav --no-cache call-stats` diff on X/T and an F
request for F; an Exact-edge mass drop is a STOP. Models and gates as EVALUATION.md §2.3; evidence
~/prism-evidence/dist-admission/; clones ~/code/prism-adm-*.
```

---

## 3. Lane 1 — JS/TS parameter-Def gaps

### 3.1 HLD
**Goal (decision served):** convert SecBench's demonstrated lower bounds into shipped tracing: 20 member-only + 6 rest + 3 unparenthesised-arrow single conversions and the 92-raw/34-cluster joint HTTP population (R1.md, r2-opus-A F1–F3), measured on the unmodified packages; plus the X/T/F row deltas with per-row correctness.

**Non-goals (disclose in SPEC §0):** the legacy `arguments` object (one conversion; not a formal — needs a synthetic Def); destructured parameters (`({a}) =>` — stays refused, pinned by test; it is the ws5 React-props item); callback *invocation* edges (a registration is not an invocation — consumer-visibility doctrine; anonymous callees get no Step-5b arg→param edges because no call site resolves to them); anonymous `function*` expressions (`generator_function` is not in `function_node_types()`, `src/languages/mod.rs:109`; adding it changes the function table and nav — separate slice); native ingress/source recognition (lane 3).

**Soundness contract.** Exact is a static-binding grade (S2-O7; CLAUDE.md). A parameter Def is a static binding fact and is always sound to *register*. Risk lives in the **edges** and their **labels**: a Def→Use edge is Exact only when `reaching_definitions_with_exact` (`src/data_flow.rs`, call at ~line 872) proves the single reaching definition; `exact_read_is_plain_required_parameter` (`src/ast.rs:7768`) keeps rest/optional/destructured out of the byte-distinct caller-read expansion and must stay that way. The safe failure direction (doctrine 7): a missing Def forfeits a trace; a wrong edge mints a false flow — every ambiguity resolves toward *no edge*.

**Components touched.** `src/languages/mod.rs::function_name` (1066–1207: anonymous callbacks return `None` unless a declarator/pair/field/assignment parent names them); `src/data_flow.rs` (565: `None => continue` skips the whole callable; 590–626 parameter registration; 602 member-only guard); `src/ast.rs` (`find_parameters_node` 10720: only the `parameters` field; `extract_param_name_node` 11486: `rest_pattern` falls to `_ => None`; `has_bare_references` 8880; `is_shadowed_at` 8796; `exact_read_is_plain_required_parameter` 7768); `src/parameter_slots.rs` (`typescript_parameter_bindings` 22: admits only `required_parameter`/`optional_parameter`; `slots` 268 already handles the arrow `parameter` field — port it); `src/reasoning/seeds.rs:256` (**second copy of the member-only guard** — doctrine 6, change in lockstep); `src/cpg/build.rs::compute_param_def_nodes` (152: by-name lookup); `src/cfg.rs:154` (already iterates `all_functions()` without a name filter, so CFG/RD exists for anonymous callables — good); `src/cpg_cache.rs` (one bump); `src/navigation/*` only if anonymous callables become visible in `symbol-spans`/`nodes-at`.

**How each gap fits the CPG/DFG model.**
| Gap | Mechanism today `[STATIC, confirmed by probes in r2-opus-A]` | Model change |
|---|---|---|
| 1 Callback argument | `function_name` → `None` for `f(function(req){…})`, `f(x => …)`, `s = f(function…)`, JSX `onClick={e=>…}` (parent `arguments`/`jsx_expression`, no declarator) → `data_flow.rs:565` skips the callable entirely: no Defs, no edges, no RD. Nested inside a named function its body Uses are attributed to the **enclosing** function (its `all_lines` spans the callback; the reference walk fences only on shadowing, `ast.rs:8778`). | Give JS/TS anonymous callables a **synthetic, non-referenceable identity** (`<cb@L:C>` keyed by byte span) so they get their own DFG pass. Do **not** extend name inference to the callee name (`createServer`) — name inference ≠ referenceability (doctrine 4) and it would collide with real functions. |
| 2 Member-only formal | `has_bare_references` false → no Def (data_flow 602; seeds 256). Even with a Def, `input` → `input.cmd` is NotReached (r2-opus-A §Priority 4): no edge from a base Def to field Uses exists. | Register the base Def and add **projection edges** `Def(p) → Use(p.<path>)` only where the Use's base identifier binds to `p` (shadow-aware), no intervening Def of `p` or of that exact path kills, and the callable is not parse-recovered. Never add edges between two different field paths — that is the guarantee field isolation was protecting (`dev.name` ↛ `dev.id`). |
| 3 `x => …` | `find_parameters_node` returns `None` (arrow's single param is the `parameter` field, not `parameters`) → zero occurrences; `slots()` already handles it. | Handle the `parameter` field in `find_parameters_node`/`function_parameter_occurrences`, `has_bare_references`'s parameter exclusion, and the JS arm of `exact_read_is_plain_required_parameter` (parent is `arrow_function`, not `formal_parameters`). |
| 4 Rest | JS: `rest_pattern` → `None`; TS: not `required_parameter` → dropped. | JS: `rest_pattern` → its identifier child; TS: admit `rest_parameter` whose `pattern` is a plain identifier. The Def is the engine-created array; `slots()` still stops at rest (positional binding stays off — correct, variadic); `exact_read_is_plain_required_parameter` keeps excluding it. |

### 3.2 LLD per gap

**Gap 3+4 — binding shapes (one PR).**
- `ast.rs::find_parameters_node`: for JS/TS/TSX, `.or_else(|| node.child_by_field_name("parameter"))` on `arrow_function`; `function_parameter_occurrences` must treat a bare `identifier` params node as a single occurrence (it iterates `children`, so special-case). `typescript_parameter_bindings` (parameter_slots 22) likewise: when `params` is an identifier, return it (no `named_children`).
- `extract_param_name_node`: `"rest_pattern" => named identifier child, else None` (object/array patterns inside rest stay refused). TS: add an arm in the `filter_map` for `rest_parameter` with identifier `pattern` and no `value`/decorators.
- `exact_read_is_plain_required_parameter` JS arm: `parent.kind() == "formal_parameters" || (parent.kind() == "arrow_function" && parent.child_by_field_name("parameter") == Some(node))`. Rest stays excluded (its parent is `rest_pattern`).
- Edge cases: `async x => …` (same node kind, async is an unnamed child — fine); `x => y => …` (two callables, each one param); TS `(this: T, x)` — `this` pattern kind is not `identifier`, stays excluded; `...args` with a type annotation; duplicate names across rest and a prior formal (`has_duplicate_js_ts_bindings` already refuses).
- Data: none new. Cache: `CACHE_VERSION` +1. Tests: `src/cpg/required_parameter_tests.rs` siblings for arrow-bare and rest in JS/TS/TSX; negative: destructured rest, `this` param, duplicate bindings.

**Gap 1 — callback identity (one PR).**
- `FunctionInfo.name == None` today (`ast.rs:171`). Add `synthetic_name: Option<String>` computed at parse time for JS/TS/TSX only, shape `<cb@{start_line}:{start_col}>` (no identifier can spell it). `data_flow.rs:565` uses `function_name` **or** the synthetic name; `owner_counts` keyed the same way (span makes it unique). `VarLocation.function` carries the synthetic name.
- **Containment:** the enclosing named function's pass must now **fence at nested callable boundaries for the callback's own formals**: a Use of `req` inside the callback whose binding is the callback's formal must not receive an edge from the enclosing function's `req` Def. `is_shadowed_at` (8796) walks to the function boundary checking re-declarations; verify it treats a nested callable's formal parameters as re-declarations — if it does not, this is a constructible false-Exact (`function h(req){ list.forEach(function(req){ use(req) }) }`). Captured reads (`x` declared in the enclosing function, read in the callback) keep today's rows byte-identical and the B8 PROVISIONAL `NameOnly(CfgIncomplete)` label; the callback pass must not re-emit them as its own Uses (duplicate rows would inflate `dfg-stats` and break parity tests `dfg_label_parity`).
- `compute_param_def_nodes` (cpg/build 187): by-name lookup; synthetic names are never callees, so no change needed — pin with a test that an anonymous callee yields `None`.
- Resolution/nav: `functions_named` (seeds), the owner index, `free_single`, `callers`/`callees` must never admit synthetic names (regression: `prism nav callers --symbol "<cb@3:18>"` → SymbolNotFound; a real function named like the callee must not gain a caller). `symbol-spans`/`nodes-at`: decide with the owner whether anonymous callables are listed (Tier-A r1 counts "anonymous Prism definitions TS 4,459 / Node 159" **separately** from the named denominators — listing them changes the harness frame; default: **not listed**, nav output byte-identical, `NAV_CALL_EDGE_CACHE_VERSION` untouched).
- Edge cases: top-level callback (no enclosing function — the SecBench shape) gets its own pass; nested-statement and assignment-RHS callbacks (r2-opus-A F1); `return foo(function(req){…})` already registers (the review's self-critique) — keep it on the same path; IIFE `(function(){…})()`; `.map(x => …)` = gaps 1+3 together; `useEffect(() => {…}, [deps])` = identity with zero Defs, body reads are captures (unchanged rows); JSX `onClick={e => …}` parent `jsx_expression` — the synthetic identity is parent-agnostic so it works, add TSX fixture; class field `handler = () => {}` and `exports.h = () => {}` already named by inference — must keep their inferred name, not a synthetic one (test: name inference wins when present); async callbacks (`async (req) => {}`) same kinds; generator expressions out of scope (non-goal); default params in callbacks reuse `javascript_inert_default_occurrences` unchanged.
- Cache: `CACHE_VERSION` +1 (second transition, separate PR). Performance: excalidraw has ~4,459 anonymous callables (Tier-A r1) that now each get a DFG/RD pass — measure build time on X and hugo-class corpora before review; RD def/line caps may fire more often (`RdOutcome::Unavailable`), which is a NameOnly label, not an error, but count it.

**Gap 2 — member-only base Def (one PR, last).**
- `data_flow.rs:602` and `seeds.rs:256`: replace `has_bare_references` with a three-valued classifier `ParameterUse::{Bare, MemberOnly, Unused}` in one shared helper (doctrine 6). `MemberOnly` registers the Def and collects **field Use spans whose base identifier is the formal** (reuse `find_path_references_scoped` on the base path; the existing field-qualified `rvalue_spans` already carry `p.x` paths).
- Edges: `Def(p) → Use(p.<path>)` with the Use's own path, routed through the same `get_use` so RD labels them like every other edge (lesson 15: same classifier, never a projection). Exact only if RD proves it; a `p = …` reassignment or `p.<prefix> = …` write before the Use kills.
- What stays refused: `p[expr]` computed access (no static path), `p` escaping into `Object.assign`/spread (still no bare read? — `...p` is a bare read, it already registers), parse-recovered callables (`contains_recovery`).
- Edge cases: shadowing of `p` by a block-scoped `let p` (is_shadowed_at); optional chaining `p?.x` (`member_expression` with optional chain — same node kind in tree-sitter, include); `this.x` is not a formal — unchanged; TS parameter properties (`constructor(private p)`) — already excluded by the allowlist in `typescript_parameter_bindings`; destructured member (`({a}) => a.b`) — no Def today, stays non-goal.
- Cache: +1. Owner decision needed before dispatch (§3.7, risk R1).

### 3.3 Keeping each change JS/TS-only with byte-identical controls
- Every new branch gated on `matches!(parsed.language, JavaScript | TypeScript | Tsx)`; the synthetic identity is computed only for those grammars (`None => continue` stays for Rust `impl_item`, C templates, Terraform, Lua).
- Controls run **same-base** (lesson 17) on the non-JS corpora the project already uses — R `~/code/bench-repos/ruff/playground` (Python), caddy/etcd/prometheus (Go), prism self (Rust): `nav --no-cache dfg-stats --edges` JSONL and `call-stats --dump-sites` must be **byte-identical** base vs head; the CPG cache round-trip and `parallel_equality`/`dfg_label_parity` tests stay green; Tier-A `--matrix-only` 178/178. Any non-identical byte on a non-JS corpus is a STOP, not a footnote.
- On JS/TS corpora (X, T, F, SecBench) the delta is expected and is the thing measured (§3.5).

### 3.4 Review gates and slice sizing
Per PR: spec review by Opus ∥ sol (cap 2) → fold → Sonnet implements from the committed prototype → round-tier gate (touched tests + one `nextest --features mcp` + advisory scoped mutgate `--since main --scope fn`) → impl review Opus ∥ sol (cap 2) → sol repairs → **re-review the fix delta** (lesson 1; ask "what did the restructure carry?" — lesson 2) → pre-merge: full nextest, authoritative mutgate (`mutants/js-param-defs.json`, ≤15 min), Tier-A quick (`--lang ts,js --sample 12`, VALID ~4 min, r2 baseline: TS callers 90.3 %/36.7 %, Node 100 %/13.8 %), corpus controls, F. Merge on clear + green. Convergence: if round 2 surfaces a new *family* (not a new instance), park that gap and ship the others — the gaps are independent by construction, which is the point of splitting.

### 3.5 The "old row vs new row correctness" method
For every DFG row (Def, Def→Use edge, label) that differs base→head on X, T, SecBench and F, classify **ADDED / LOST / RELABELLED / RE-OWNED** and adjudicate with oracles in this order; the first oracle that decides, decides.

| Step | Oracle | Decides | Rule |
|---|---|---|---|
| 1 | **TS checker symbol identity** (pinned `typescript.js` 5.9.3, `~/prism-evidence/native-positional-gap/gate-inputs/…`; `getSymbolAtLocation` on the Use's base identifier → `declarations[0]` byte span) | binding of every ADDED/RE-OWNED Def→Use edge and every member-only projection | symbol's declaration span == the Def's parameter span ⇒ **new row correct**; different declaration (outer `req`, a `let p` shadow, a rest/destructure element) ⇒ **new row WRONG** (shadowing or identity bug). A LOST row whose checker binding was to the formal ⇒ **old row was correct, head regressed** (STOP). |
| 2 | **Write-reference check + CFG reachability** (checker `findReferences` with `isWriteAccess`; prism `cfg_queries.rs` reachability from entry on the *base* binary) | **Exact labels** on new rows | any write to the formal (or to the exact field path) between entry and the Use ⇒ Exact is WRONG (must be NameOnly); unreachable Use ⇒ edge WRONG. Prism's RD is the thing under test, so the write check is independent of it. |
| 3 | **SecBench demonstrated traces** (`conversions-final/`, 126 rows; harness `--no-build --binary <head>`) | behavioural correctness per gap | each credited conversion's **unmodified** package must now trace on head: ws3 20, rest 6, arrow 3, joint 92/34 after gap 1+2; a conversion that still needs its rewrite is a residual gap (recorded, not WRONG); a package that traced on base and not on head is a regression (STOP). Re-run the r2-opus-A BFS payload-specific check so no credit is a sibling-parameter artefact (its F5). |
| 4 | **Tier-A tsserver call hierarchy** (quick, `--lang ts,js`) | RE-OWNED rows and any nav visibility change | caller/callee precision must not fall below the r2 CIs; synthetic identities must not appear in the named frame. |
| 5 | **Blinded adjudication sample** | rows no oracle decides: member-only projection edges whose *flow* (not binding) is in question; capture-vs-formal rows in callbacks | 40 rows stratified 8 per class, presented **without** base/head provenance, as mechanism questions ("Is `p` at L_def the binding for `p.x` at L_use, and is `p` reassigned on any path between?"), two labellers (owner + one model not used in the lane). Accept a class only if κ ≥ 0.6 **and** ≥ 90 % "correct"; else park that class. MEAS-A's κ 0.36 on *outcome* labels is why the questions must be mechanism-level. |

Decision table: new row correct & old absent ⇒ gain; old correct & new absent ⇒ regression (STOP); both exist and differ (RELABELLED/RE-OWNED) ⇒ step 1/2 picks; neither decidable ⇒ step 5 or refuse (safe direction).

### 3.6 Measure-first steps and PR split
0. **Census on base** (planner, X/T/SecBench; controller F): counts of anonymous call-argument callables (by parent kind), member-only formals (and how many of their field Uses are on sink-like lines), bare-arrow params, rest params; projected row deltas per gap. Excalidraw already gives rest 13 / bare arrows 2 (synthesis), so gaps 3+4 are tiny on X and matter on SecBench.
1. Prototype each gap as a **separate commit on one proto branch**, measure the delta **per commit** (same-base controls, §3.5 steps 1–3) so attribution is per mechanism — the joint 92 need gap 1 then gap 2 measured on top of gap 1.
2. Non-JS byte-identity controls per commit (§3.3).
3. Tier-A quick per PR; SecBench rerun per PR on the unmodified packages.
4. F per PR (controller), new `PRIVATE_EVIDENCE_ROOT` each.

**One PR or four? Neither — three, in this order:**
1. **PR-A "binding shapes" = gaps 3 + 4.** Same functions (`find_parameters_node`, `extract_param_name_node`, `typescript_parameter_bindings`), no identity or model change, ~9 SecBench rows; splitting them would cost a second cache transition for a handful of rows.
2. **PR-B "callback identity" = gap 1.** Structural (function identity, containment fence, nav visibility, perf on 4.5 k callables); needs its own review loop.
3. **PR-C "member-only base Def" = gap 2.** A model change with the field-isolation risk, the largest row delta (133 affected incl. later stacks, R1.md) and an owner accepted-cost decision; last so its measurement is not confounded with PR-B and so PR-B's identity is in place for the joint HTTP population.

Why not one PR: a two-reviewer cap of 2 will not converge on identity + model + grammar changes at once (S2's history), and one measurement cannot attribute a row delta to a mechanism (lesson 17). Why not four: PR-A's halves are the same code path. Each PR = one cache transition (lesson 10).

### 3.7 Risks
| # | Risk | Class | Mitigation |
|---|---|---|---|
| R1 | **Member-only relaxation adds false flows.** The guard exists so a base-only Def cannot carry taint between unrelated fields (data_flow 594–596). Projection edges keep that, but a *field-specific* seed (`req.body` tainted, `req.headers` not) that is seeded at the **parameter Def** would now reach every field. React props (`props.x`) and Express `req` are exactly this shape — and F is React. | model change → owner | projection edges only from a param Def to Uses of *that* path; taint seeded by access path must match Uses by exact path (verify in `taint_reaches`); disclose as accepted cost if any seed mode is base-only; measure F false-flow sample in §3.5 step 5. |
| R2 | Synthetic callback identities leak into resolution (callers/owner index/`free_single`) → false Exact callers or symbol-seed hits. | WRONG class | non-spellable name; negative tests on every seed/resolve path; `call-stats` same-base diff on X/T must show zero new call edges. |
| R3 | Nested callables double-count Uses (enclosing + callback) or the enclosing walk gains edges into callback formals (shadow). | WRONG class | containment fence + `is_shadowed_at` test both directions; `dfg-stats` parity tests. |
| R4 | Cache-key conflict with lane 2; two bumps in one branch. | process | ownership table §2.1; one transition per PR. |
| R5 | Nav/Tier-A frame change if anonymous callables become visible. | measurement | default invisible; owner decides; pin with symbol-spans golden tests. |
| R6 | Build-time regression on arrow-heavy corpora. | perf | measure X/hugo build time per commit; RD cap counters reported. |
| R7 | Yield lower than the SecBench lower bound (joint 92 are clone-heavy: 34 clusters). | expectation | report dedup clusters, not raw; the standalone 20/6/3 are the honest number. |
| R8 | Fix-wave regressions (lesson 1/2) and reviewer open-class streams. | process | re-review fix deltas; park a gap, never restart the artefact. |

---

## 4. Three-line verdict
1. One session cannot reliably run all three lanes concurrently: the private-F custody and the shared cache-version constant serialize lanes 1 and 2 by construction, and the session's own handoffs show it already lost context across two lanes.
2. Run lane 1 here; lane 3 in a separate eval-only session now (with the lane-2 admission census added to it); lane 2 after lane 1 merges, with a measured admission predicate — blanket dist/build admission risks dropping existing Exact callers through owner-key collisions.
3. Parameter-Def gaps: three PRs, not one or four — binding shapes (rest + bare arrow) → callback identity → member-only base Def, each with one cache transition, per-mechanism same-base measurement, and the TS-checker-first old-row/new-row adjudication.
