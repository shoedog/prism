# Lane 8: one taint answer from both engines — design

**Stage D design of record, 2026-11-25.** The measurement base is `9d1a7d06f653`. The four matrix shards at `7ccb0eba04f1` contain 415 rows. The row decisions are in [STAGE_D_ROWS.md](../../../../../tests/fixtures/taint_parity/STAGE_D_ROWS.md). The implementation seats apply that table to the JSONL as their groups ship. Decision 2 is settled on branch A2. Decision 6 remains with the owner (§8). Historical probe names below refer to the two design reviews of 2026-11-21.

## 1. Intent

Prism answers "does this value reach that sink" in two ways, and they disagree. Review taint
(`--algorithm taint`) follows every data-flow edge and matches a catalog of sinks. The reasoning layer
(`prism nav taint-reaches`, the MCP tool `taint_reaches`) walks with the trace engine, which crosses a
function boundary only under strict conditions, and has no catalog: the caller must already know the sink's
line. An agent using the MCP tool can be told "boundary" for a flow that review reports, and cannot ask
"which dangerous sinks does this reach" at all.

The lane does three things: makes every difference between the engines either disappear or carry a named,
tested reason; removes the differences that are defects of the trace engine; and gives the reasoning layer
a sink catalog.

## 2. What exists today (verified on `origin/main` at 1179055f; PR #841 at 493ab5ee)

| Fact | Reference |
|---|---|
| Default review taint is a breadth-first walk over all data-flow edges: no depth limit, no confidence gate, no check that a cross-function edge belongs to a call. Labels only grade the finding. | `src/cpg/cfg_queries.rs:205`, `src/cpg/query.rs:707` |
| The trace engine is used by review only with `--taint-return-flow`, and always by the reasoning layer and the MCP tool. It crosses into a callee only when the edge ends at a definition, exactly one call site *spelled like the callee* in the caller matches the source occurrence (same line, or the occurrence's bytes inside the site; a site of another name on the line is not counted), that site's resolution includes an Exact edge to the target (one of several Exact targets suffices), and the depth is under 2 (shared with return ascent). Ascent is allowed when *any* call in the destination function resolves singleton-Exact to the callee. The visited set is first-come: a node is never re-expanded. A refused hop is recorded as a boundary and not followed; other losses (the depth-0 CFG filter, a suppressed shortcut) record nothing. | `src/cpg/trace.rs:729`, `:823`, `:839`, `:526`, `:533`, `:25` (line references at `ec242f97`); `src/reasoning/taint_reaches.rs:54` |
| Return-flow edges are built only for a call that resolves singleton-Exact; a name-only call gets none (`return_flow_skipped_nameonly`). | `src/cpg/step5c.rs`, `src/cpg/step5c_facts.rs` |
| An object-literal leaf binds to every formal read whose static chain is comparable with its key path; a computed read (`p['k']`, `p[k]`) has the empty chain, so it is fed by every leaf. | `src/cpg/object_literal_args.rs` (`read_chain`, `comparable`) |
| Probe matrix, 26 fixtures in three languages: default review and `taint-reaches` agree on 20 and disagree on 6. Five are descent refusals: an object-literal argument bound to a member read (the edge ends at a use; two fixtures), two call sites of the callee on one line, a name-only callee, a call chain three deep. One is sink granularity (a use that shares the sink's line). Review in trace mode agrees with `taint-reaches` on 25 of 26. Both engines miss a captured variable read in a nested function. | research section 6 |
| Wrong results today (review probes on a0f2c427): (a) the trace engine returns into an unrelated caller: `entry` passes `t` to `id`, `separate` reads `safe = id('fixed')`, and `safe` is Reached; (b) `run(...[], t)` into `run(a, b)` reaches `b` (the producer binds positionally past a spread; lane 9 slice A); (c) exploration order decides depth: a formal reached first at depth 2 blocks a later depth-1 route, so an in-budget path is BoundaryExited; (d) a recursive `f({a:x}, false)` reaches a later read `p.a` as intra-function DataFlow at depth 0; (e) a non-simple return seeded inside the callee is dropped silently by the depth-0 CFG filter; (f) `pick({bad:t, clean:'fixed'})` has an Exact edge into the read `p['clean']`; (g) review reports a flat sink with a fixed operand on a line reached only at a parameter Def, and a certified constant-return call's left side through the same-line shortcut. | probes `return_one`, `spread_direct`, `diamond`, `self_literal_after`, `return_expr`, `literal_index`, `flat_def_line`, `constant_return_flat` |
| In the cited probe fixtures, findings that are `exact` in default mode can become `unlabeled`, because descent and assignment hops mark the evidence as crossing unlabelled edges. An object-literal binding edge carries the worse of Exact and the callee read's own label. | `src/finding_confidence.rs:348-379`; `src/cpg/object_literal_args.rs:523` |
| The trace engine has no notion of a sanitizer; the reasoning layer adds one on the witness chain and accepts a sanitizer of any category, by documented design (its line query names no sink category). Review cuts per category on the flow path and, as an advisory, by presence in the source function's body. | `src/reasoning/sanitizer_walk.rs`; `src/sanitizers/matching.rs` (`sanitizer_call_site`); `crates/prism-review/src/algorithms/taint/cleansing.rs` |
| PR #841 adds an optional crossing policy threaded through the walk functions. It limits only edges that enter a context file (a file added by import depth) to a confidence ceiling of Exact or name-only; `--taint-context-depth` is that import depth. The descent rule still requires Exact whatever the policy says, the depth cap and review's choice of engine are unchanged, and the reasoning layer passes no policy and has no context files. With its defaults, every difference above remains. | PR #841 (`src/cpg/taint_policy.rs` at 493ab5ee) |
| The sink catalog lives in the review crate: the flat pattern list, the positional classifier with its scope resolvers and module tables (`taint_sinks/`, 23 files, 7,322 raw lines with tests), the per-language structured tables, the matchers. The matchers call into review's cleansing and source discovery (the operand check reaches framework parameters, request aliases and `seeds.rs`), so their module closure is measured, not a set of helpers: from `taint/sink_match.rs` and `taint_sinks/` it is 48 files and 17,241 raw lines plus 12 items of `taint.rs` and one function of `seeds.rs`; with the sink loop's own source facts (framework sources, wrapper sinks) about 18,300 of the 19,146 lines in 51 of 53 files. Only `taint/report.rs` names another review module (`crate::output::mermaid::safe_node_id`). Review's FlowPaths are single-source fans, and its sink predicates read every edge of the fan and take the CPG (`taint/cleansing.rs`). A static table's argument index is not always the operand review checks (`render_template_string`: index 1; review checks the tainted keyword's template variable). The core crate holds the sink pattern type, the sanitizer recognisers and nearly empty per-framework sink lists that nothing reads. | research section 5; `taint/python/sink_tables.rs:31`, `taint/python/sinks.rs:49`; closure measured by `use` and qualified paths at 9ba4249d; `src/cpg/cfg_queries.rs:281`, `taint/cleansing.rs:477` |
| `taint-reaches` answers BoundaryExited only when the sink is a boundary's target or reachable from it inside that function; a sink one or more functions further is NotReached, with the warning in another function (r2 probes `b4`, `b5`). The trace has no call-argument facts: `CallSite` holds `arg_count` and a Go-only `arg_spread`, the graph holds no `ParsedFile`, a DataFlow edge carries a label only. | `src/reasoning/shape.rs:91`; `src/call_graph.rs:97`, `src/cpg/build.rs:302` |
| The `prism` binary is built from the review crate, so `prism nav taint-reaches` can reach the catalog. The MCP server is a separate crate that depends on core only, and a layering test forbids it to name the review crate. | `tests/integration/layering_test.rs`; `crates/prism-mcp/Cargo.toml` |
| On real diffs, the cross-file sinks that issue #602's measurement added were right 10 to 23% of the time, against 13 to 17% for in-diff sinks. | issue #602, M2 |

## 3. Goals, non-goals, anti-goals

Goals:

1. A committed parity matrix at occurrence level (5.1): for each fixture, what each engine answers at each
   parity level and, where they differ, the named reason. It is the contract; any later change that moves a
   cell fails a test.
2. The trace engine crosses the hops that are as well founded as the ones it crosses today, on evidence that
   belongs to the hop (5.2): an object-literal or props binding edge into a read its key path supports; a call
   site singled out among several on one line; a callee whose spelling at the site differs from its
   definition's name. It stops returning into callers it did not enter, and its depth accounting stops
   depending on exploration order.
3. Every lost sink, in either direction (the trace engine loses what review reports, or reaches what review
   does not), and every refused hop reports one reason from one closed list (5.3), in `taint-reaches` output
   and in the census.
4. The same crossing knobs (confidence ceiling, depth) on the core walk both clients of the trace walker call,
   with today's defaults; `taint-reaches` and the MCP tool expose them, and tests drive trace-mode review with
   given knobs through the core API. Default review has no descent gate and gets none.
5. A catalog mode for the reasoning layer: from a source, the catalog sinks reached, each with its rule,
   operand, verdict and witness; first in the CLI, then in the MCP tool.

Non-goals: replacing review's walk with the trace engine (it would drop findings review reports today);
new sinks or sources; captures in nested functions; callbacks (lane 9); improving the precision of review
taint (its wrong findings found here are pinned and filed, 9); cross-file context (PR #841); building return
edges for name-only calls (a graph change, 5.4).

Anti-goals: loosening descent by removing a condition instead of replacing it with an equally strong one;
a catalog copy in the core crate that can drift from the review crate's; changing default answers of either
engine without a matrix row that says so; widening any wrong result of section 2.

## 4. Approaches considered

For the engine differences:

**A. One engine.** Make review use the trace engine. Measured consequence: five of the six disagreeing
fixtures lose their finding, and trace-mode findings lose their `exact` grade. Rejected.

**B. Make the walk identical by loosening the trace engine to follow every edge.** It would stop being a
witness-producing, Exact-gated engine, which is what the reasoning layer's verdicts mean. Rejected.

**C. Keep both, pin the differences, fix the defects** (chosen). The engines answer different questions
(review: could this reach; trace: here is a justified path). The matrix records which cells differ and why;
the defect classes are fixed in the trace engine; the knobs let either client of the walker run under the
other's policy.

For the catalog:

**A trait in core, implemented in the review crate and passed in** (chosen for the CLI). The `prism` binary
gets catalog mode with no move. How the MCP server gets an implementation is decision 2 (section 8), the
last slices of the lane. The owner chose A2 on 2026-11-24; the four branches considered:

- **A2. A small crate below both binaries** (chosen, owner 2026-11-24). A new workspace crate
  `crates/prism-taint` (package `prism-taint`, library `prism_taint`, depending on `prism`) holds the taint
  module's closure: the matchers call into source discovery, so nothing smaller than the module closure can
  move, which is nearly the whole taint module (about 18,300 of 19,146 lines, 51 of 53 files, section 2),
  with the catalog implementation. `run.rs` (orchestration), `report.rs` (the one file naming
  `crate::output`) and the IPC detection of `flat.rs` stay in review. `prism-review` and `prism-mcp` depend
  on the new crate, core is unchanged, and the documented layout becomes four crates. The layering test gains
  one allowed crate name for the MCP crate, a rule that the new crate never names `prism_review` or the
  review crate's `algorithms` module, and a rule that core never names `prism_taint`. Untangling, done in
  the review crate before the move: the `taint.rs` items the closure names into a closure module, the
  format-wrapper detection out of `flat.rs`, I-8's sink loop out of `run.rs`, source detection taking the
  graph and files instead of `&CpgContext` (I-8), and restricted visibility rewritten to `pub`, so that the
  move itself rewrites only `use` paths. Once lane 7 S2-R lands, the registry lives in core (`prism::rule_registry`, re-exported by review) and the private `SinkHit.rule` and `cwe` fields have accessors. On this checkout the fields are public. I-9.0 verifies the actual API, imports and closure on the I-8 head before the move. Cost of the MCP slices: 7.5 days (PLAN I-9).
- **A. Move into the core crate.** The same closure into a core module not named `algorithms`; core then holds
  review's sink rules. The owner's earlier choice (2026-11-21), made before the closure was measured; 9 days.
- **B.** The MCP crate depends on the review crate through one adapter file and a narrow layering exception;
  2.5 days.
- **C.** CLI only, MCP pending; 0.5 day.

## 5. Architecture

### 5.1 The parity matrix

Read `rows.jsonl`, `rows-t2a.jsonl`, `rows-t2b.jsonl` and `rows-t2c.jsonl` under `tests/fixtures/taint_parity/`. `STAGE_D_ROWS.md` records the disposition of all 415 input IDs, the slice prerequisites and CURRENT cells, reason amendments, and additional control fixtures to materialise. Distinguish a required future cell from an observation on the pinned producer. A row names occurrences, not lines: the
source occurrence (file, line, bytes), the call site and argument slot or the return it tests, and the sink
operand (file, line, bytes). Parity has three levels, asserted separately:

- **Walk:** the trace walker's reached nodes and witness for the row's source occurrence as the only root,
  under given knobs. `taint-reaches` and trace-mode review are two clients of one walk, so with equal knobs
  they must give the same walk; this is asserted only on rows whose inputs are genuinely identical (the
  source line holds one occurrence and no framework source; flag `walk_comparable`). The cell is the
  aggregate, in the existing priority (`Reachability::aggregate`), of the verdicts of every Variable node on
  the sink line whose byte span intersects the operand's span (a zero-width node at the operand's start
  included); one operand can have several nodes (`p` and `p.a` over the same bytes, r2 probe `a1`). The row
  lists the node spans it expects, so a change in node minting fails the well-formedness test, not the verdict.
- **Finding:** review's `taint_sink` finding at the sink (file, line) and its grade, for default and trace
  mode. Review's roots (every variable on the source line, framework sources), its root exclusion and its
  cleansing differ from the walk, so a difference from the walk level carries a reason.
- **Catalog:** the multiset of (file, line, rule, operand, verdict) catalog mode reports (5.6).

Each level has its own expectation on every row (an explicit `no witness` or seed error where that is the
answer), a reason from 5.3 where it differs from another level, in either direction, and a truth label. A row whose expectation
is a known wrong answer of today's engines (section 2) names the issue or the slice that fixes it. One
data-driven test per level reads it. The 26 research fixtures and the review probes seed it; the census adds
the shapes real code shows.

### 5.2 Crossings

**Baseline at 9d1a7d06f653.** In `src/cpg/trace.rs`, `descend_target` accepts a DataFlow to a Def, indexes `callers` by the destination function's **name**, counts matching sites before resolution, then uses `.any(Exact && target_matches)`. Its cache includes the occurrence start and the callee owner but omits the occurrence end and target node. `return_flow_ascent_allowed` searches the entire destination function for *any* singleton-Exact call to the returning owner. In `taint_trace_from_root` the depth comparison short-circuits before `descend_target`, and the first insertion of a node into the root's `enqueued` set wins. In `object_literal_args.rs`, `formal_info` takes Uses one DataFlow edge from the formal Def. `collect` emits leaf → Use directly. `read_chain` stops at a subscript so a computed read directly on the formal has the empty chain. In `step5c.rs` the failed `step5c_facts::singleton_exact` test skips the site before edge creation. A name-only return has no ReturnFlow edge at either ceiling.

**Hop evidence.** A crossing consumes one existing edge and one positive certificate. Keep an immutable syntax view over the exact parsed files used for this graph, shared by review, navigation, catalog and the sanitizer exclusion walk. Read call arguments, lexical owners, value versions, target formals, and RHS/LHS result slots there. Read endpoints, labels, the resolver's candidate records and formal neighbours from the graph. No new edge metadata or cache format. Implement the view beside `AstOrderView`; give it file/owner/span descriptors, never opaque node indices into a graph it does not own. Preserve public graph-only entry points: without a usable view, every interprocedural crossing refuses (`no_matching_site` for missing site evidence), including empty-stack ascent. Local paths still work. The no-view entry must also fence recursive binding candidates and cut synthetic inputs whose value role it cannot prove; an unclassifiable same-function producer edge must not bypass the fence. Keep ordinary local Def→Use flow. Test each entry, including cache-loaded graphs and bypass re-walks.

1. **(i) Identify the occurrence.** Start from the reached edge endpoint, not just the query's root line. Bind a file, executing caller, half-open site and actual-value spans, slot, formal/read node, and target function identity. For an ordinary argument use its own bytes in the selected actual. The AST must show that the actual supplies this value. Containment in a receiver, callee token or an intervening callable is insufficient. For a result, identify the RHS call of the assignment to this destination and the result component it consumes.
2. **(ii) Identify one emission.** Enumerate sites of that caller, including aliases. Retain only occurrences and target/slot pairings consistent with the edge. Zero matches refuse, multiple indistinguishable emissions refuse. For a lane-3 JSX source, replay the producer's lookup key (file, caller identity, value line, full access path or its base, Use). Inspect value expressions of the effective attributes of each opening in that owner. Require exactly one surviving (opening, attribute leaf, slot, target) compatible with the reached node and its value version. A key token at nonzero width, another attribute's token, and a zero-width declarator anchor can all be line-keyed. Neither width nor line equality identifies the value. Do not search an enclosing owner's locals or promote a signature Def to the missing Use.
3. **(iii) Certify one dispatch.** Require the selected target in the complete site-local resolution pool. A singleton Exact suffices for resolution. For a larger scalar pool all grades must be Exact and all targets must belong to one receiver/member dispatch. Require the same receiver type/selector and resolver-backed membership for *each* target (Go interface satisfiers including proven embedded method paths, or the same method in a proved override hierarchy). Compare the certified set with the whole candidate set. Recover the callee’s declaring receiver and method signature for each ID and check membership in the receiver’s interface implementation, embedded promotion or base/override relation. A missing hierarchy fact refuses the entire pool. A common name or common resolution-kind tag alone proves neither membership nor completeness. Mixed grades, free/local same-name pools, unknown membership and unproved unions refuse `not_exact` at either ceiling. A synthetic mixed vector is a unit case. The current resolver grades each pool uniformly. Literal, JSX and return producers retain their singleton prerequisite.

   On the pinned fixtures, d01 exposes two `interface_dispatch` implementations and d02 one `embedded_promotion`. Both descend. d03's source declares an override but the pinned callee query reports just Base.run via `typed_param`. Resolution passes for that target. Rule (iv) maps `t` to `Base.run`’s `cmd`, so d03 stays Reached d1. Do not synthesize a Sub.run edge. w09 exposes two `local_def` targets of `f`. The conditional assignments give a feasible flow, but no receiver dispatch certificate. Refuse both. Test a real or constructed override pool independently of d03.
4. **(iv) Certify the slot.** Through the selected actual *i*, require ordinary positional arguments, prove a one-to-one mapping using all original signature positions including holes, and check that the existing edge ends at the corresponding formal or its certified read. Normally actual *i* maps to declared formal *i*. The selected formal must bind one positional name (including Python positional-only). A default on that name permits an explicitly supplied positional value. Python uses it even for `None`. JS/TS carries the supplied value on the path where it is defined. An omitted argument or a JS/TS value proved `undefined` takes the initializer and supplies no such witness. A later spread, rest or Go variadic tail cannot move a fixed prefix. A preceding or selected expansion, destructuring at the selected slot, selected rest/variadic slot, keyword, method expression, call/apply/bind adapter or unproved offset/decorator mapping refuses `unproven_slot`.

   For Python `recv.m(...)`, certify actual *i* → declared formal *i+1* only when the receiver and class-body declaration prove descriptor binding. This requires a plain undecorated method accessed through an instance, or a method decorated solely by the built-in `@classmethod` accessed through the class or an instance. Verify the implicit first positional formal and the edge’s actual destination. A plain method accessed through the class supplies no implicit instance. A method decorated solely by the unshadowed built-in `@staticmethod` binds no receiver through either the class or an instance. Require actual *i* → formal *i* and refuse the *i+1* offset. Prove the decorator binding at class definition, the selected descriptor at dispatch and the actual destination. Refuse other, stacked, shadowed or uncertain decorators (`unproven_slot`). This preserves both base static descents without treating a decorator's spelling as proof. A Go bound receiver is declared outside the parameter list. Never count the receiver as an explicit actual. A JSX edge uses its separate certificate for the producer’s props position 0 and selected key/local, never the ref position. Ordinary destructured calls still require a slot proof they lack here.
5. **(v) Certify the value version.** Match the lexical binding and prove the reaching version at that occurrence. Use byte order and def/use facts for writes, shadowing, kills and competing reads. A same-line redefinition prevents substituting a later occurrence for the earlier value. Discard candidates disproved by version facts. Zero consistent emissions give `no_matching_site` under (ii). If the graph conflates versions and order cannot isolate the emission, refuse `ambiguous_site`. Cache static proofs within one graph/view by relation, emitting caller/site/value start **and end**, both endpoint identities, target and slot, label/ceiling if consulted. Keep order/version, context policy, exclusions, stack and budget outside that cache unless also keyed. Cache failures only at the same specificity.

**Standalone name-only guard (I-guard).** Under Exact, inspect the binding edge’s own label before site lookup or a proof-cache hit. A `call_nameonly` or `builtin_callback` doubt refuses `not_exact` even when an Exact sibling names the target. A read-side `SameLine` doubt alone does not trip the guard. Count the legacy same-spelling sites in the executing caller first, using the base predicate (site starts on the source line **or** contains its bytes). Zero refuses `no_matching_site`, two or more refuse `ambiguous_site`. Only for a singleton test containment of the ordinary actual's bytes in that site, including on a single line. Failure refuses `no_matching_site`. Filtering by containment before counting would admit l02/l03 early. Keep this admission gate through I-walk until I-same-line permits a unique emission certificate. I-walk may refine the cause of an already refused hop when version facts disprove *all* candidates (l07). That refinement authorizes no new descent. A producer line key can only nominate candidates. Leave uncertified line-keyed widenings refused.

Report the following default answers in the I-guard PR. `n01` and `n02` are NotReached on the base fixtures, which lack the lane-9 edges. With those edges present the guard returns `BoundaryExited / not_exact`. Pin this on constructed edges until the producers land. `s2-js-alias-same-line`, `s2-js-alias-multiline-overlap` and `s3-ts-optional-default-competitor` move from Reached d1 to `BoundaryExited / no_matching_site` until I-spelling. In each, the alias is the only site for the target that contains the tainted actual. Assert these intermediate cells in I-guard. Keep l01’s positive witness at its second `t` and l06’s refusal. The guard needs only the edge label and existing site bytes, so lane 9 can depend on it before I-walk.

**Binding.** Trust the existing producer's leaf/read pairing. It already compares key paths by prefix in either direction, applies duplicate/spread and write suppression and bypasses the formal Def to avoid tainting siblings. Verify the existing edge, proven slot and owner, and that the Use is a direct DataFlow neighbour of that very formal Def with the same binding. At its bytes accept a read of a static dotted chain or a bare consumption. Reject any subscript on the observed path (literal included), a write target or a read executing inside a nested callable, invoked or otherwise (`target_not_binding`). Use the lexical execution owner of the target bytes. Missing edge means `no_graph_edge`; never recreate a producer's path match. I-walk owns the recursive fence before **any** local expansion, independently of I-binding. For an argument→formal Def or producer-paired formal Use in its own executing function, record `self_param` and refuse descent. With partial or absent syntax use the graph's owner, parameter/neighbour and recursive-site facts to fence possible self-bindings even when they cannot be disambiguated. Preserve provable local Def→Use. At every entry with and without syntax, b04 is BoundaryExited, never Reached d0. In b04b the earlier read cannot follow the seed on this local path: NotReached, no sink boundary. A hop refusal alone cannot assign that sink a reason. I-binding edits no lane-3 file.

**Two call sites.** l01 proves `g` from its own second `t` [166,167). l02/l03 can likewise distinguish separate actuals. l04, `g(g(t))`, is an explicit exception to (i). The AST identifies `t` as supplying only the inner actual. The de-duplicated edge to the shared formal carries no emitting site. Without independent evidence of which site emitted this edge, refuse `ambiguous_site` before selecting the inner site by (i). Choosing the narrowest span is insufficient. A later separately proved return of the inner call can supply a distinct outer actual. No outer-site credit from the bytes of the inner argument. l05's Python Use [74,78) fits only the first site [72,79); the second occurrence supplies no graph Use. Admit the first and pin the bytes, with no second-site witness. Under a collapsed line key that could denote either emission refuse both. l06 refuses the name-only edge even though the other call is Exact. In tr2-js-rebind the first occurrence before the write can be proved and its witness must use it. In l07 the reached version occurs in the first call's second slot. The first call’s tainted slot supplies `b`, while the second call’s slot for `a` reads the new, fixed version. No site supplies `a` from this root. Refuse `no_matching_site` once the version facts are applied.

**Spelling.** Enumerate by caller identity, resolve the precise site, and compare target identity across imports, default exports, namespace members and aliases. The callee's definition spelling is never a site filter. Apply (i)–(v) to renamed calls, including w07. Before I-same-line, apply the legacy count to sites of each candidate's own spelling in the executing caller before testing containment. Prove which spelling supplied the reached actual and require a single consistent emission across all spellings. This allows a proved alias next to a differently spelled fixed call while preserving the collision gate on two sites of one spelling. Wrappers require an existing, proven edge at each crossing. A resolver-supported JSX wrapper uses only the target and props binding admitted by lane 3.

**Returns and Depth.** Push (file, caller, full site span, callee) on descent. On ReturnFlow recover the exact call assigned to the destination Def and match the return component, including tuple position and ignored/discarded results. With an entered frame, require equality to the top and pop it. An unrelated site is a `return_not_entered_site` **cut**, even if the budget is exhausted. An empty stack allows an origin inside the callee to ascend each individually certified existing result edge, including further ascents after a pop. A discarded result cannot return to a neighbouring assignment. A recursive return is a `self_return` boundary. Charge one for every accepted descent or ascent and none for intraprocedural propagation. The displayed `descent_depth` counts only descents. For an existing synthetic ReturnValue, apply the depth-zero CFG test at its owning return statement. At **every** depth and in every walk, also prove the selected Use contributes to that exact returned component before traversing `ReturnInput`. The return-site/slot/frame proof still applies to the subsequent ReturnFlow. Local assignments and existing shortcut suppression keep their own rules.

Step 5c first seeks a matching direct Use for an identifier or member path. Otherwise it may mint a ReturnValue keyed by owner, return bytes and child slot and link **every** owner-local, contained semantic Use. Its filter excludes hidden Uses, type syntax, callee labels and keyword names. Containment alone still accepts a discarded expression or an input to a constant function. Use the pinned syntax view, exact binding, bytes and reaching version. Peel only value-preserving parentheses. Apply the following closed allowlist to the whole path from that Use to the selected component. An unknown shape, missing view or missing proof cuts this local edge with `unproven_return_input` (no interprocedural boundary). Never turn a cut into a positive attribution or bypass it on a sanitizer re-walk.

| Returned expression | Does this Use supply the result? | Traversal on an existing edge |
|---|---|---|
| Identifier | The value of that exact identifier does. A different occurrence of the same name proves nothing. | Admit the exact whole-value Use. Preserve ordinary direct ReturnFlow. |
| Binary operation | A direct string operand of built-in concatenation with a fixed string suffix carries data on the feasible primitive-string path (`v + '!'` in w05 and its Python control). An arbitrary operator or subexpression can discard it. | Admit only that form, with the selected identifier as the entire left operand and the right operand a string literal. Require compatible language semantics and a feasible primitive-string binding. An unconstrained parameter in w05 can receive a string; the witness describes this path. A known non-string value cannot use that assumption. Refuse proved overloads, coercion-only paths, annihilating operators and nested calls. |
| Call | An argument is evaluated. The callee can return a constant. | Refuse the synthetic shortcut even for a known identity wrapper. A separately produced, certified call result may travel its own route at full cost. |
| Tuple, array or object display | A surviving element can contribute to the aggregate, an overwritten value or sibling cannot prove a selected component. | Refuse aggregate summaries. Keep Step 5c's existing arity, blank and individual result-slot checks. |
| Conditional | A feasible value arm can supply the result. A test alone gives only control dependence. | Refuse both arms and condition in this allowlist. |
| Template / f-string | A plain interpolation may contribute characters. A tag, formatter or nested expression can ignore the input. | Refuse interpolation summaries, including plain ones. |
| Await | The result comes from the awaited operation. The input could be a thenable or awaitable that returns a constant. | Refuse the summary. |
| Spread | Only surviving elements or properties can contribute. Iteration, overwrite and position need separate proof. | Refuse. |
| Member read | The exact selected member supplies its value. Its base, key or sibling alone is insufficient. | Preserve Step 5c's direct matched-path endpoint. Refuse a synthetic fallback for a member. |
| JavaScript comma sequence | Only the last value is returned. Earlier operands can also change the version of the last one. | For the last expression only, recurse into this allowlist with its own version proof. Refuse every discarded operand. Python tuple syntax uses the display rule. |
| JSX | A surviving prop or child may contribute to the element. A tag, discarded prop or callback body does not prove a returned scalar value. | Refuse synthetic JSX summaries. The opening→formal producer has its separate certificate. |

This tightens previously traversable synthetic edges as well as the proposed CFG repair. At Exact/2, `tr2-js-wrapper` loses its real flow: the base used the argument inside `id(x)` as the result of `wrap`. There is no separately assigned inner result edge to that return component. Record the recall cost, keep `truth: flows` and retain the default-review finding. An arbitrary expression containing a call never earns a two-crossing summary.

For each root, keep separate states for (node, full entered stack). Also distinguish initial CFG-restricted traversal from post-crossing traversal when the stack is empty. Store the greatest remaining budget for each such state. Re-expand on improvement. Keep an immutable parent **per state version**, including the edge, certified bytes and predecessor version. Distinct equal-budget stacks at a formal remain live. On ties select minimum crossings, then shortest witness, then stable lexicographic file/byte/site/relation/endpoint order. Propagate a better canonical tie or select it from the final predecessor graph. This avoids hybrid chains and cyclic parents. Buffer boundaries and cuts by root, state version and attempted edge/site. Publish only those supported by final nondominated expansions, retract superseded records and suppress sink warnings when a valid witness wins. Retain independently live refusals in the ledger. Exercise reversed enumeration order. w01/w02 reject wrong and discarded returns, w03/w04 select the in-budget diamond, w05 retains a root-local synthetic return, w06–w09 exercise certificates. w10 and w10b select two and one descents at budget 3; w10c refuses the first descent at 0; w10d refuses ascent after two descents at budget 2.

I-walk asserts the two independent stacks and exact return destinations on separately identifiable sites (the two multiline calls in `a1-js-equal-budget-return-sites`, querying `exec(first)` and `exec(second)` in separate variants), or on constructed site certificates. Its public mixed-layout control must still refuse the colliding first actual. In **I-same-line**, use the following literal text (LF, one trailing LF). For the witness to `b`, assert seed [40,41), actual [79,80), entered and returned-through site [74,82), callee return Use [24,25), returned-to Def [70,71), terminal [91,92), and exactly one descent and one ascent. The other actual [60,61), site [57,62) and Def [53,54) must never be spliced into that witness. Also assert the independent result for `a` using its own site. For the independent result replace only the `b` in `sink(b)` with `a`, keeping the offsets. Require actual [60,61), site [57,62), return [24,25), Def [53,54) and terminal [91,92). The base reaches both results but splices the second actual into the witness for `a`.

```js
function id(v) { return v; }
function f(t) {
  const a = id(t); const b = id(
 t
);
  sink(b);
}
```

### 5.3 Reasons

Use one primary reason per refused hop and per lost **or gained** sink identity (repository revision, file, projected line, direction, compared root sets/options). Preserve the root, state, from/to, site/slot when known and sink operand for each contributing path. This is the closed vocabulary, in priority order **within each applicable stage**:

| Stage | Ordered reasons and evidence |
|---|---|
| Existing crossing | `self_param`, `self_return`; then `return_not_entered_site` for a certified wrong return (cut); then `depth_cap`, the edge-own `not_exact` guard, `target_not_binding`, `no_matching_site`, `ambiguous_site`, site/pool `not_exact`, `unproven_slot`, `policy_ceiling`. The last is #841's independent context filter. A version-refuted last candidate gives `no_matching_site`, ahead of `ambiguous_site`, which applies to indistinguishable surviving candidates or unresolved versions. A missing view cannot certify any return or binding. |
| Local path and producer | `unproven_return_input` for an existing synthetic input cut by the value-role test (record Use/return/component and rejected AST role), `shortcut_suppressed` for the certified return shortcut cut, `cfg_pruned` for a rejected local successor, `unmodelled` for a specifically identified missing construct (capture, callback), otherwise `no_graph_edge` for a demonstrated missing binding/result. Exhausted search alone proves none of these. |
| Comparison of clients | `review_no_return_flow`, `review_no_recovered_def_use`, `review_cfg_filter`, `review_context_policy` when the successful path needs that difference. `sanitizer_model` for incompatible cleansing evidence, `operand_granularity` for line/fan or seed-occurrence projection (record both root identities when review uses a different root). Use `uncertified_flow` when the reporting client reaches the actual operand through an unsound dispatch, slot, field pairing or local value transfer. Bind both endpoints, relation, site/version and the incompatible runtime binding or value; replay a control severing that transfer while preserving the sink. It can explain a shared wrong witness against truth. `matcher_admission` requires a reached candidate and a demonstrated sink-rule admission difference: record the configured rule set or the masking source range and predicate, and a matched positive control. `review_context_policy` requires actual context-loading or crossing-policy evidence. Use `unmodelled` plus the inventory number for an exclusive per-root-inexpressible emitter in catalog comparisons. |

A boundary must reach the requested sink for **that root**: for a same-owner recursive candidate exclude CFG-impossible local continuations, including direct target identity at an earlier unreachable read (b04b). Then test the target itself, then an unconditional forward continuation from that exact target node to the operand (including intervening functions). Memoize by graph, target and continuation constraints. The unconditional DFG is a candidate index: check occurrence identity, proven wrong-site cuts and local order before binding a sink (a line-keyed VarLocation can represent multiple byte occurrences); a warning in the same function cannot assign a cause. For each (root, sink operand) check direct target identity first, then test at most 128 distinct (boundary target, continuation constraints) pairs in canonical file/byte/endpoint order. Count each pair once, including cached answers, so warming the cache cannot change the cutoff. If a required 129th test remains, emit `incomplete` for that attribution. Count it as `unexplained` under Explained and withhold primary-reason credit. Excluded sanitizer hops and dominated visits are bookkeeping, not flow boundaries.

For sink attribution first seek a discriminating path/decision difference at identical inputs, or a controlled minimal relaxation. Include joint barriers as secondary evidence. On a necessary sequence choose the first blocking hop from the root and its primary reason. For alternative complete explanations choose least number of relaxations, then shortest causal path, then the stage and reason order above and canonical byte identities. Prove sufficiency before tie-breaking. For `s3-js-recursive-return-distal`, relaxing depth alone suffices on the direct suffix. Walk and trace-review loss both get `depth_cap`; the recursive hop retains `self_param` in the ledger. In particular a valid return-gain witness must need return flow to explain a gained line. Keep ambiguous or unbound candidates in diagnostics. Report `unexplained` as a failing audit status until one cause is established; its required count is zero. No `other`, `unknown`, or joint label is a product reason.

### 5.4 Knobs

`taint-reaches` gains `--edges exact|nameonly` (default `exact`) and `--depth <0..=3>` (default 2, counted as
in 5.2; 3 is the deepest the matrix exercises); the MCP tool gains the same optional inputs. Trace-mode review
keeps its CLI unchanged: the core walk takes the policy, review passes the default, and tests pass other knobs
through the core API. At `nameonly` admit at most a homogeneous singleton name-only resolution for that same proved binding; all occurrence, origin, slot, recursion, frame and budget checks still apply. A derived callback with no provable actual/formal mapping still refuses. PR #841's policy cannot carry them (it governs context files only), so the lane adds a descent
policy of its own beside it, reusing that PR's ceiling type with another meaning, stated at the flags: #841
compares a context-entering edge's own label (a `sameline` or `cfg_incomplete` doubt fails `Exact`), this
policy the call's resolution grade plus the two doubt kinds of 5.2. The ceiling governs the resolution grade the hop
evidence accepts at descent and at ascent, and only where the graph has an edge: Step 5c builds no return
edge for a name-only call, so `nameonly` does not promise name-only return flow; that loss carries
`no_graph_edge`, and building such edges is a follow-up (a graph change: stored edges, `CACHE_VERSION`, an
edge differential). Equal knobs give equal answers at the walk level of 5.1 only.

### 5.5 Confidence in trace mode

Keep the existing evidence classifier. Crossing or assignment propagation can make a finding `unlabeled`; this is not a blanket grade for trace mode. Of 53 surviving population-A sink lines, default review had **21 exact, 2 nameonly, 30 unlabeled** and trace review had **24 exact, 2 nameonly, 27 unlabeled**. These are marginal histograms of the same survivors, not a transition matrix. Record both findings' actual grades. Changing how a crossing contributes confidence remains decision 4's follow-up.

### 5.6 The catalog

Use the existing matcher on **one complete single-root fan** and the graph, parsed files, run configuration, seed membership and framework facts used by review. Batch many fans in one invocation, retaining their identities. Distinguish the *matcher decision* (including the projected line and review cleansing), the *operand it decided*, and the *walk verdict from this root*. The line-parity oracle compares decisions; it does not require that the graph has a witness to each operand. A row's finding columns can include framework or other line roots. Its catalog column selects only the marked occurrence.

| Decision field | Contract |
|---|---|
| Origin and location | Root occurrence ID, inventory family, file and **projected finding line**, plus the actual call, constructor, assignment, attribute or identifier span. Use the matched row span when the predicate has only a row. A call need not exist. |
| Operand | Tagged `LineOnly` (wire operand null, verdict on any fan target at the row), `Argument` (whole selected positional/semantic argument subtree), `KeywordValue` (the selected template variable's value), or `JsxAttribute` (whole attribute). The latter three carry half-open UTF-8 **file** offsets from the matcher. A qualifying framework source range is an admission fact. For the flat call or assignment gates it does not silently become a selected argument or RHS. |
| Classification | Actual classifier tag or registry ID, CWE, optional sanitizer category, strength, and `review_cleansed`. Structured IDs use `structured-<category>` pending lane 7. Preserve a legacy identifier rule, discovered wrapper name and configured pattern identity even without a `SinkHit` (use family-qualified matched pattern as ID until registered). A flat category derives only from a mapped CWE (79, 89, 918, 502, 78, 22). Weak entries retain weak strength. |
| Verdict and witness | For an operand, aggregate only intersecting nodes in this root's trace, on the operand's own source lines. Keep the exact decision even for NotReached or BoundaryExited, with no invented chain. For a line-only hit return its line witness and label the scope. Sanitized needs matching-category evidence on this root's paths, including the same-category bypass re-walk under identical knobs. An unresolved bypass can yield BoundaryExited. Body-presence cleansing or a hit in another fan cannot alone prove Sanitized. A recognizer of a different category or a hit on a categoryless rule is advisory. |

A review predicate can accept the correct text on insufficiently precise graph evidence. For c04 a fan reaches the first `x` on the line; the matcher can select the second but the operand verdict is NotReached. For family 22 the classifier can recognize an inline request argument external to the fan; retain its decision and judge the selected bytes from the requested root. No decision from an independent root may be borrowed. Conversely an accepted `review_cleansed` candidate can have an unsanitized bypass. Keep root-specific verdicts through aggregation and clipping. Count syntax arguments without comment/trivia nodes (a1-js-sanitizer-comment-slot). A paired guard may establish catalog Sanitized only with its actual checked operand, dominating success branch and no raw bypass in that fan (s3-go-paired-clean-independent-fan). Body scans alone remain advisory.

For the real-code gate, run ordinary trace-review finding projection from an internal exact-node-root entry. Retain framework discovery, range masks and matcher facts while bypassing root expansion. Identify each root on the same graph, including the root of a synthesized fan if explicitly selected. Assert root-set equality before comparing any findings. In `tr5-js-two-roots`, the ordinary line seed reaches `exec(u)` from `u` only; a singleton `t` run has no decision at that sink. Keep both comparisons as controls.

Extract review's fan loop once; both adapters use the same graph/file identities, run facts, dispatcher ordering, operand selection, range masks and cleansing. Project each admitted continuation row separately (c12). Deduplicate identical (root, file, projected line, sink span, rule, operand kind/span) records, then group the root results for display. At a matched occurrence prefer the structured operand decision over a duplicate flat fallback for the same sink. Keep separate occurrences and independently selected structured categories. Use the inventory's flat tag when only the flat route admits the occurrence. Preserve the underlying admissions so a cleansed structured hit cannot hide an independently surviving review line. Configure strength floors identically in a gate run. Build one classifier per query; measure JS/Python/Go before proposing a shared cache.

**Emitter contract.** The inventory has seven writes, 29 ordinary `taint_sink` families and eight companion entries. Families 1–12 are flat/legacy/configured, 13–19 structured (including JSX, template text/keyword and multiline projection), 20–22 add request facts, 26 uses the Go framework table, and 28 accepts a positive originating fan with seed-row facts. These 24 are expressible. Test 9/10 with native legacy/C-wrapper inputs outside the five-language matrix and 11 with an actual nonempty API configuration; default-config emptiness does not exercise the positive route. For 28 isolate the seed predicate from the ordinary structured write in a matcher test.

| Unsupported ordinary family | Missing input in a per-root decision | Fixture (pin empty for this route) |
|---|---|---|
| 23 Independent JS inline YAML | Scan of every admitted file, even without any root | e2-js-inline-yaml |
| 24 Inline Flask positional | Source-line seed event, including one with no fan | e2-python-inline-flask-positional |
| 25 Inline Flask template keyword | Source-line seed event plus template-keyword match | e2-python-inline-flask-keyword |
| 27 Go no originating fan | Seed membership and proof the set of originating fans is empty | e2-go-no-originating-path |
| 29 Go secondary inline source | Negative fold over **all** originating fans before separate inline search | e2-go-mixed-inline |

Expose a structured coverage diagnostic identifying these five families and their missing inputs on CLI/MCP, including an empty result. Preserve normal seed errors. Do not scan the repository to pretend to identify individual omitted hits. Independent supported hits on the same line still count. Entry 30 is the separate Bash `unquoted_expansion` category. Entries 31/32 synthesize upstream target/IPC roots; do not silently add them to an explicit-root query. Entries 33–35 are shared fan suppressions, 36 belongs to the unsupported source-event routes and 37 arbitrates Bash occupancy. They are outside the 29-family denominator.

Core owns `SinkCatalog: Sync`, a batch `sink_decisions(cpg, files, rooted_fans) -> CatalogDecisions` interface. Each rooted fan contains its actual root ID, seed membership and complete FlowPath. The result contains the decisions and coverage diagnostics above. Pass configured run facts through the implementation. The review crate supplies the first adapter to `prism nav taint-reaches --sinks catalog`; I-9 moves that implementation to `prism-taint` for both binaries (A2).

### 5.7 Where the code goes

`src/cpg/trace.rs` (1,138.5 weighted lines: it must not grow), so new logic goes in siblings:
`src/cpg/trace_descent.rs` (hop evidence and reasons), `src/cpg/trace_frames.rs` (walk state: entered sites,
remaining budget, re-expansion); `src/reasoning/` (catalog mode, knobs, the syntax view beside `order.rs`'s
`AstOrderView`, the boundary-target test in `shape.rs`), `crates/prism-review/src/` (the
trait's implementation, CLI flags), `crates/prism-mcp/src/` (inputs; the golden transcripts change and are
re-recorded with the reason in the commit), tests under `tests/` for the matrix. In the I-9 slices (decision
2, branch A2) the trait's implementation and the closure it calls move to `crates/prism-taint/src/`, and
review keeps re-exports so its paths hold.

## 6. Measurement

**Stage D change log — measured overrides (2026-11-24).** The original #602 list was unavailable. M selected 20 qualifying diffs in each of six repositories. On 120 paired diffs there are 56 default and 68 trace sink lines, 53 survivors (53/56), three losses and 15 gains. Two Directus Redis lines (140, 159) returned under `not_exact`; Superset helpers.py:4635 required `not_exact+depth`. Under the default Exact/2 knobs none is a measured binding, same-line or spelling recovery.

The census patch logs pid, root/from/to (owner, line, access, path and bytes), relation, depth, refusal/cut, **first** failing condition and site features (line/containment lists, alternate spelling, target access, grades; `slot_if_known` is null). Its unconditional relaxations give upper bounds, not certificates. Across six frontiers the raw 2,987 records are 2,479 depth (2,105 descent, 374 ascent), 357 CFG, 63 site-not-exact, 56 shortcut, 29 self-param and 3 cheaper revisits. `classes.json` reports 2,455 classifiable depth refusals and the census flags 24 refusal records with incomplete identity. Keep both denominators. Among depth-refused descents, 267 also have name-only and 68 also have Use targets. Neither overlap is a new finding. Same-line and spelling have zero observed or latent members. Baseline control replay covered the two diffs with losses, not all 120.

The census close note and `classes.json` agree on eight gains in Superset a813fb906c78 and seven across five Apache Answer revisions. The note attributes the gains to return flow. Use the ledger for identity (Superset queries.py:666–669, 671–672, 674–675; Answer template_controller.go:381 at 791b5f74, :384 separately at 88c5ef66, ace02c49 and c7b37a15, and connector_controller.go:135,184,194 at 06a0f742). The candidate reason is `review_no_return_flow`. Attribute each line by §5.3 before granting Explained credit. Neither `unknown` nor `other` is an accepted waiver. The three loss records also lack bound boundary identities. Preserve the joint relaxation as evidence even when reporting one primary reason.

**Decision 6.** Population A gives zero recoverable sink lines to all three descent slices. Binding has 68 latent candidates, all already over budget. Each optional slice costs about three engineering days plus census, Sound sample, Benchmark and runtime checks (nine days together). The matrix case for keeping them is a reproducible occurrence-level improvement for supported bindings, collisions and aliases, including lane 3. The yield case for deferral is no measured default recovery; no default increase or 95% Sound pass can be inferred from empty samples. If kept, binding has the only latent population and goes first; ties retain same-line then spelling. The owner decides dispatch. Apply the Recovered floor and empty-Sound default in §§7–8 on each actual slice parent. Guard, walk repairs and knobs have independent reasons to proceed. Keep Exact/2 defaults and the maximum selectable budget of 3. The census did not authorize a change of ceiling, cap, grade, or benchmark threshold. At round-1 review the orchestrator was measuring the population-B base. Its subsequent 2026-11-25 census receipt supplies per-class counts. I-guard cannot merge before a comparable B base exists unless the owner explicitly accepts Benchmark as unmeasured for the named slices. Compare each head against the admitted base for Benchmark.

**Measurement method (Stage M).** On the replacement 120-diff population described above, for every sink
that default review taint reports, run review in trace mode and record whether the finding survives and, if
not, the boundary. Every run freezes its commit, loaded-file manifest, options, roots and binary. The unit is
a path: source occurrence, call site and slot, sink operand. Yield is counted by sink line. A lost
finding is classed by the relaxation that recovers it, joint relaxations included (`binding+spelling`), or
`unknown`. A relaxation crosses its hop class unconditionally and the log records, per refused hop, the first
failing condition and the observable features (target access, sites on the line, whether containment singles
one out, the spelling of the resolving site, grade, depth): the census measures an upper bound, which is
enough for the ordering rule and decision 6 (an empty upper bound is an empty class), and it is not a
hand-written prototype of 5.2. The relaxation patch is applied to the base commit only and never ported.
Review parses only the diff's files, so population A observes a crossing only when caller and callee are
both in the diff and cross-file classes (spelling, cross-file bindings) would read near zero for a reason of
measurement: the same log is also collected from `taint-reaches` frontier mode on the full checkout for the
census's frozen sources, and a class's size for the ordering rule is the larger of the two. The measured overrides above govern decision 6. The apparatus is
first run on synthetic fixtures (two roots and two sites on one sink line; a joint barrier) and must change
the right cell. The benchmark's harness records trace-mode review, not default review, so it supplies the
Benchmark gate's paired `taint-reaches` non-regression only, not classes.

**Crossing ledger (product diagnostic, PLAN I-1).** The walk logs each accepted crossing, each synthetic ReturnInput traversal (Use, return component, owner and state), and every refused
hop with its reason under an environment switch, tested like the other trace diagnostics. From the census's
frozen sources, `taint-reaches` frontier mode on the full checkout at each commit gives the whole-repository
ledger; a slice's new crossings are its difference against the parent build, whether or not the sink line is
new. Nothing is re-ported to a slice head.

**After each slice:** the census again, against the slice's parent and the lane baseline; the benchmark's
traced count through `taint-reaches` (it must not fall), and every benchmark case whose outcome becomes
traced is read against its ground truth (a new false witness raises the count); `taint-reaches` frontier
wall time on population A's largest repository against the parent; and a labelled sample of min(100, all)
new crossing paths from the whole-repository ledger (with the review-side ledger beside it), read blind to the
slice with the new hop and its code shown, each labelled twice: "new hop real" (the gate) and "whole path
real" (reported; old line-granular hops can fail it), adjudications kept.

## 7. Gates

| Gate | Threshold (default) |
|---|---|
| Matrix | every row's active expectations hold at each level with their reasons (use the asserted CURRENT cell of §8 for a deferred group or admission); with equal knobs, trace-mode review and `taint-reaches` give the same walk on every `walk_comparable` row |
| Explained | in the census after the lane, every default-review sink that the trace engine does not reach, and every trace-mode finding that default review does not report (the gained list), has a reason from the closed list of 5.3; `unexplained` is zero |
| Recovered | At least 53 of the 56 frozen default sinks must survive at Exact/2. Measure I-walk, then each retained slice against its actual parent and the fixed baseline. A flat 53/56 meets the floor with zero recoveries. Prove every named recovery on its stated population. For a default recovery the identical default-review sink (repo, commit, file, line) must be absent in parent trace and present in head trace with a real newly certified path at unchanged defaults, roots, files and options. Publish before/after identities including losses; a new fixture, knob-only or latent path earns no population-A credit. |
| Sound | at least 95 of 100 sampled new crossing paths have a real new hop (a smaller population: all of it, at most 5% not real); the whole-path label is reported; a known false witness is a WRONG whatever the tally; the descent slices have an empty sample at Exact/2 on the census frontiers. For a measured empty population record 0 new paths, 0 observations and no percentage, with the two compared heads. The decision 6 default permits an unmeasured empty Sound sample. This gives no evidence of a 95% pass. Missing ledgers are not empty populations |
| Benchmark | traced cases through `taint-reaches` do not decrease in any class; every case that becomes traced has been read and none is a false witness |
| Catalog | on the matrix, catalog mode reports exactly each row's expected multiset of (file, line, rule, operand, verdict), an empty list included; every (file, line) where it differs from review's `taint_sink` findings carries a reason from the closed list in the row; and on real code (population A's repositories at their census commits, the census's frozen sources, one whole-repository graph through the API) the catalog's (file, line) set of decisions not `review_cleansed` equals trace-mode review's `taint_sink` lines, using a review comparison entry seeded by exactly the selected graph-node IDs, with normal framework/source-range facts and matcher configuration preserved; assert equal root sets before walking and compare each root's projection at the same floor, except for individually identified contributions exclusive to families 23–25, 27 or 29. Compare before result caps. A supported `(root, projected line)` cannot be excused by an unsupported hit on that line |
| Defaults | with default knobs, review taint's findings are unchanged on the population |

## 8. Owner decisions

| # | Question | Default applied | Cost of changing later | Latest cheap point |
|---|---|---|---|---|
| 1 | Start before PR #841 merges | no; wait for it | starting early means rebasing three files | at start |
| 2 | How the MCP server gets the catalog | **Owner, 2026-11-24: branch A2, a new crate `crates/prism-taint` below both binaries that holds the taint module's closure (section 4).** Chosen with the measured size in hand (section 2: the matchers call into source discovery, so the closure is nearly the whole taint module, about 18,300 of 19,146 lines in 51 of 53 files). It replaces the owner's earlier choice of branch A (2026-11-21), made before the closure was measured. Estimates of the alternatives, recorded: A 9 days, B 2.5, C 0.5; A2 is 7.5 (PLAN I-9) | A2 is a structural change to the crate layout and adds a crate; moving back means a second move-only sequence | decided |
| 3 | Default of `--edges` for `taint-reaches` | `exact`, as today | flipping to name-only changes agents' answers | — |
| 4 | Grade of trace-mode findings | retain the current classifier and mixed grades (5.5) | one slice and every trace-review cell | any time |
| 5 | Depth cap | 2, adjustable by flag up to 3 | — | — |
| 6 | Which descent slices should ship? Does the owner require a strict default recovery or a nonempty Sound sample? | Dispatch remains an owner question. Unless the owner specifies stricter gates, the lane applies the §7 Recovered floor of 53/56 plus proof of each claimed gain, and reports a measured empty Sound sample as unmeasured. This replaces the previous strict-rise requirement: all three measured losses need a non-default relaxation. Nonempty Sound samples retain the 95% threshold. Record any override by slice before dispatch. | Three days per retained slice (§6) | end of stage M for ordering, decision required before dispatch |

Use the per-level `prerequisites` conjunction in STAGE_D_ROWS, including combinations of binding, same_line, spelling and an actual #596 edge. For every deferred group, keep its target expectations and reasons as data. Materialise and assert CURRENT cells on the chosen build at every level, with the measured verdict/grade or catalog decisions and the applicable reason, even for dependent rows owned by another group. Use the convention in STAGE_D_ROWS. Matrix and Acceptance check CURRENT until the necessary slice ships.

## 9. Risks and accepted limits

- **More reach means more to be wrong about.** Each descent change replaces a refusal with a proof of equal
  strength; the labelled sample checks new crossing paths on real code.
- **Known wrong results on main** (section 2 (a) to (g)) are pinned as rows with their truth. The trace
  engine's (a), (c), (d), (e) are fixed in the lane, and (b) is refused by the slot proof until lane 9 removes
  the edge. In particular I-walk must fence (d) before local expansion on every entry, with and without a usable syntax view. b04 has a `self_param` boundary and b04b has no earlier-read sink warning, even if I-binding never ships. (f) stays a wrong edge that no crossing may use, and (f) and (g) are filed at close-out. No slice
  widens one.
- **The catalog inherits review's sink precision** (in-diff sinks were right 13 to 17% of the time in one
  measurement). Catalog mode reports rule, operand and witness so a caller can filter; the lane does not make
  sinks more precise.
- **Golden transcripts of the MCP server change** when its tool gains inputs and a mode.
- **Differences that remain by design:** captures, callbacks, name-only return flow, anything review's
  unconditional walk follows that has no hop evidence. They stay in the matrix with their reasons.

Recorded, not pursued: a review-consistent `taint-reaches` query mode
(another public contract). Dropped from the lane as not changing the goal-level result, each a follow-up with
the decision it would support: the trace-mode grade slice (decision 4); descent flags on the review command
(only if a user of trace-mode review other than the benchmark harness asks for them); labelled samples of new
crossings under non-default knobs (only if the owner asks to flip decision 3 or 5); `--depth` above 3 (after
a wall-time measurement). Pre-existing defects found by the round-2 probes, pinned as rows and filed at
close-out: a mixed formal's second member read on a line gets no binding edge (r2 probe `a3`); default
review reports `exec(p.b)` when only `p.a` is tainted (`a1`; catalog mode inherits it); in Python and Go the
second occurrence of a name on a line never binds (`b1p`); Python keyword arguments get no edge (`b2`).

**Accepted losses on the pinned producer.** js06 misses the nested capture and js07 the callback parameter flow (`unmodelled`). go04 lacks a graph path to the selected `c` in the chained `exec.Command(c).Run()` form (`no_graph_edge`); plain Go command controls remain positive. Name-only return edges and other documented missing producer edges are separately pinned. An unsupported catalog emitter is a coverage limit, not a false witness. Accept a real gain due solely to default review omitting return edges once demonstrated; none of the 15 ledger entries is accepted *because* it is `unknown`. Preserve the history and truth of every input `known_wrong` in STAGE_D_ROWS; clear an active mark when the recorded base cell already matches truth and the certified target. A wrong witness remains a defect even where its public verdict stays the same. Matching computed-key reads remain deliberate losses. For each of the 82 input rows in the three descent groups, deferral keeps the target as data and requires an asserted CURRENT cell and reason on the chosen build (§8). This also applies across groups. For f03 assert `target_not_binding` before I-binding and `not_exact` afterward. For the alias rows assert `no_matching_site` from I-guard until I-spelling.

| Base-accepted crossing | Stage D disposition and row |
|---|---|
| (a) Bound Python instance method | Certified by (iv). `d03` pins an instance call. I-walk also tests `self.run(t)` and `r = Runner(); r.run(t)` with the actual entering `cmd` in `run(self, cmd)`. Both reached it at depth 1 on `prism-base`. `tr6-py-bound` pins the independent `not_exact` refusal for `Runner().run(t)` at Exact. |
| (b) Supplied positional value to a defaulted formal | Certified by (iv). `tr1-js-default` and `s1-py-default` stay Reached d1. `s3-ts-optional-default-competitor` has the same positive target once spelling proves its optional alias. The intermediate loss is site identity (§5.2 I-guard). |
| (c) Classmethod called through the class | Certified by (iv). `s1-py-cls` stays Reached d1 through `Runner.run(t)` to `x` after the implicit `cls`. |
| Unshadowed built-in static method through class or instance | Certified by (iv) without a receiver. `d-py-static-class` and `d-py-static-instance` retain their base Reached d1 path to formal 0. |
| Synthetic call-return summary | `tr2-js-wrapper` is a deliberate recall loss. Its call argument cannot certify the result of `wrap`, even when the inner function happens to return it. |
| (d) All-Exact pool without one dispatch | Deliberately refuse both `w09` local targets under (iii), `not_exact`. A feasible flow that the base reached is lost because no receiver/member proof covers that pool. |
| (e) Typed Python receiver with an override in source | `d03` stays Reached d1 for the singleton Exact `Base.run` edge. No `Sub.run` edge is present. A multi-target override still requires the complete dispatch proof in (iii). |

**Enumeration close.** E stopped while still producing rows. Site identity/slots, returns/budgets and sanitizers/source fans have a row for each independent factor at the last tier. This does not assert Cartesian-product coverage. Apply the following rules also to members absent from the matrix.

| Open class | Rule for any member | Limits |
|---|---|---|
| JSX ownership | Enumerate executable openings × effective attribute leaves × lexical value owners × producer-admitted component identities. Apply the actual line-index lookup and (i)–(v) to the reached Use. Last effective own key wins within its opening; respect spreads, namespace-export identity and the producer's direct React wrapper admission. A callback-local use can qualify. | No synthetic captures across one or more owners, signature-only source, type-only use, shadowed or unproved wrapper, or sibling-opening evidence. |
| Formal reads (including JSX) | Vary formal position/binding, read AST kind, lexical execution owner, write/kill and key relation. Follow only the producer's paired edge to the formal/local Def or to a direct read under Binding. Apply any documented pattern-local mapping to that local specifically. | No phantom destructured or rest edges, computed read, write, nested callable read, or sibling-field precision beyond the graph. Missing producer stays pending only when an identified upstream change is actually expected. |
| Route-handler shapes | Generate matched pairs varying framework activation, registration shape/provenance, receiver binding, signature position/pattern, owner/shadow and root projection. Use review's actual source discovery, ranges and file admission once. Hold the explicit occurrence fixed and identify any separately admitted framework root by ID. | No new route recognizer, arbitrary adapter inference, transfer of a framework fan to the marked root, or claim that lexical recognition proves runtime registration. Unrecognized registrations provide no implicit seed. |
