# Spec review — P8 (param slots) / P9 (pointer embeds) / P10 (owner partitions) — gpt-5.6-sol @ xhigh, read-only, 2026-08-21

Reviewed main @ 5fe4529. Verdicts: P8 FIX, P9 FIX, P10 FIX. Designs: scratchpad design-P8/P9/P10 (copied below the review in this doc on merge).

I’ll treat this as a literal read-only review: source/diff/lockfile inspection only—no edits, builds, tests, cache writes, or provider/network calls. I’m using `prism-nav` for the cross-module call/dependency census and `lsp-nav` for type-resolved references, then I’ll reconcile both against exhaustive `rg` results and the pinned grammar sources. I’ll also do a brief memory pass for the earlier Items A–D review conventions.The structural navigator is available; the type-resolved LSP tool advertised by the skill is not callable in this session, so I’ll use Prism plus a literal, repository-wide reference census and direct type/source reads. No memory entry matched this repository or the earlier A–D review.The P8 census is now complete at HEAD: six direct `function_parameter_names` production callers, three production `function_parameter_occurrences` callers, two Step‑5b `compute_param_names` copies plus its focused tests, and `FunctionInfo.param_names` is consumed only by the Step‑5b fallback. The key review question is therefore not missing textual callers, but whether each positional site’s lookup and argument-binding semantics actually honor the proposed fail-closed contract.Two design-level hazards are now grounded. P9’s extraction fix would make invalid `*Iface` visible to the existing S4 route builder, which does not inspect `is_pointer`; the proposed negative test cannot pass without an explicit gate. P10’s consult rule also cannot use the boolean visibility predicate alone: HEAD’s already-safe typed-fact path deliberately requires the detailed/certain result before admitting Exact. I’m continuing through the P5/P11 second copies and upstream provider collapses before ranking them.Reviewed pristine `main` at `5fe45299abe38238c4e979302890ff7f570c142f`. Read-only inspection only; no builds, tests, cache writes, or corpus runs.

## P8 — Fail-closed parameter slots

Confidence: high. The compression bug and both supplied bad cases follow directly from HEAD.

### Grounding and census

`function_parameter_names` and its byte-bearing sibling iterate parameter-list children and append only successful single-name extractions (`src/ast.rs:5863-5904`). Go’s shared `parameter_declaration` arm returns only the first identifier when no `declarator` field exists (`src/ast.rs:6381-6393`, `src/ast.rs:6439-6449`).

The pinned grammars are Go 0.23.4, Java 0.23.5, JS 0.23.1, Python 0.23.6, Rust 0.24.2, and TS 0.23.2 (`Cargo.lock:959-1038`). Relevant shapes:

- Go grouped names are repeated `name` fields; variadics have their own node (`tree-sitter-go-0.23.4/grammar.js:240-258`).
- JS formal parameters are pattern or assignment nodes; patterns include rest/object/array forms (`tree-sitter-javascript-0.23.1/grammar.js:541-586`, `:1146-1172`).
- TS replaces formal parameters with `required_parameter`/`optional_parameter`; the pattern may be `this` (`tree-sitter-typescript-0.23.2/common/define-grammar.js:381-384`, `:659-678`).
- Rust parameter lists include `self_parameter`, `variadic_parameter`, `_`, bare types, and pattern-bearing parameters (`tree-sitter-rust-0.24.2/grammar.js:662-701`).
- Java has `receiver_parameter`, `formal_parameter`, and `spread_parameter` (`tree-sitter-java-0.23.5/grammar.js:1215-1246`).
- Python includes defaults, tuple-pattern defaults, splats, and typed splats (`tree-sitter-python-0.23.6/grammar.js:636-651`, `:680-702`, `:935-943`).

The positional-consumer census is complete:

- `function_parameter_names`: eager `FunctionInfo` (`src/ast.rs:521-545`), Level 3 (`src/call_graph.rs:1827-1885`), primitive slice (`src/algorithms/primitive_slice.rs:297-347`), peer consistency (`src/algorithms/peer_consistency_slice.rs:179-207`), and two quantum name-set sites (`src/algorithms/quantum_slice.rs:583-592`, `:662-673`).
- `function_parameter_occurrences`: Step‑5b normalization (`src/cpg/build.rs:24-52`), DFG parameter definitions (`src/data_flow.rs:258-293`), and reasoning seeds (`src/reasoning/seeds.rs:253-276`).
- `compute_param_names`: production and serial-reference Step‑5b loops (`src/cpg/build.rs:896-951`, `:1061-1114`) plus focused tests.
- `FunctionInfo.param_names`: populated at `src/ast.rs:102,534` and consumed only by the Step‑5b fallback at `src/cpg/build.rs:52`.

### Findings

1. **WRONG — The proposed Go recall test cannot pass because `b` still has no DFG parameter Def.**  
   For `func f(a, b string, c int)`, the new slots would be `[a,b,c]`, but the design explicitly leaves DFG on compressed `function_parameter_occurrences`. That API emits only `a,c`; DFG creates parameter nodes only from those occurrences (`src/data_flow.rs:265-293`). Step‑5b then looks up a Def node for `b` before emitting the edge (`src/cpg/build.rs:935-951`, mirrored at `:1097-1113`). Result: argument 1 no longer incorrectly reaches `c`, but it does not reach `b` either.  
   Required fix: separate “runtime slots” from “all real binding occurrences,” and expand Go grouped names in the latter. DFG and reasoning seeds must consume the expanded binding occurrences. The claim that those consumers remain byte-identical is false.

2. **WRONG — `None => skip` is insufficient at Level 3 because the function and incoming-call lookups are name-only.**  
   Level 3 retrieves the first definition using `find_function_by_name` (`src/call_graph.rs:1835-1839`) and then consults every raw incoming site under `self.callers[caller_id.name]` (`:1870-1881`). Two same-named functions with different signatures can therefore use the wrong parameter list and each other’s call sites even if both slot lists are individually perfect.  
   Concrete case: module A defines `invoke(cb)`, module B defines `invoke(x, cb)`, and only B is called as `invoke(safe, target)`. A’s `cb()` can be synthesized toward `safe`. Level 3 must bind the exact `FunctionId` and only incoming sites whose resolved target includes that identity.

3. **WRONG — Duplicate simple JS formals defeat the advertised “never compress / never false edge” invariant.**  
   Valid non-strict JS can contain `function invoke(cb, cb) { cb() }`. With `invoke(safe, target)`, runtime `cb` is `target`; the proposed slots are `["cb","cb"]`, while Level 3 uses `.position(...)` and selects index 0 (`src/call_graph.rs:1865-1868`), minting `cb → safe`. Reject duplicate binding names for positional name lookup or model the binding’s effective slot explicitly.

4. **WRONG — Rust `self_parameter` is a missing pseudo-parameter case.**  
   The Rust grammar places `self_parameter` in the same list as runtime parameters, but method-call syntax does not supply an explicit receiver argument (`tree-sitter-rust-0.24.2/grammar.js:662-701`). Treating every unhandled child as `None` would drop all Rust method positional processing, contradicting the promised Rust byte-identity. It must be explicitly skipped, like TS `this` and Java receiver parameters.

5. **SMELL — The grammar contract is not complete enough to implement consistently across the six languages.**

   | Language | Required handling not pinned by the design |
   |---|---|
   | Go | Grouped names; reject/fail-close variadic, unnamed, and blank-identifier slots unless holes are represented. |
   | JS | Single-identifier arrow parameters, duplicate names, default/rest/destructuring, and nested ERROR/MISSING traversal. |
   | TS | `required_parameter`, `optional_parameter`, `this`, typed rest, and typed destructuring. |
   | Rust | Skip `self_parameter`; fail-close `_`, bare-type, variadic, and non-simple patterns; cover closure parameters. |
   | Python | Defaults are representable, but `*args`, `**kwargs`, tuple-pattern defaults, keyword-only boundaries, and reordered keyword arguments are not ordinary index-to-index mappings. |
   | Java | Simple `formal_parameter` is representable, `receiver_parameter` is pseudo, and `spread_parameter` is variadic. HEAD currently handles none of these node kinds. |

6. **SMELL — Whole-function `None` is safe but unnecessarily coarse at the single-slot consumers.**  
   Both Step‑5b loops already skip `None` correctly, and eager `FunctionInfo.param_names: Option<_>` is sufficient as a fallback contract. Primitive slice and peer consistency inspect one known parameter only (`src/algorithms/primitive_slice.rs:317-347`; `src/algorithms/peer_consistency_slice.rs:188-207`); a per-slot `Vec<Option<ParameterOccurrence>>` could retain proven slots despite an unrelated unknown later slot. Whole-function skip is nevertheless sound there. Level 3 needs per-binding identity, not merely per-slot handling.

7. **SMELL — Telemetry and acceptance are under-plumbed.**  
   `call_stats` receives only `&CallGraph`, not parsed files (`src/navigation/queries.rs:156-172`), so `param_slots_unknown` must be serialized as per-file/per-language state or recomputed during every whole-program pass. The design does not cover `empty`, skeleton/full/subset construction, `remove_files`, or `merge` for that state. Also, all four indirect levels share `CallSiteOrigin::IndirectResolution` (`src/call_graph.rs:1691-1698`, `:1922-1943`), so a general resolution-kind histogram does not discriminate Level‑3 removals; add a Level‑3-specific count if the corpus delta is intended as causal evidence.

### Ranked changes

1. Add distinct runtime-slot and binding-occurrence APIs; expand Go grouped bindings into DFG/seeds.
2. Make Level 3 exact-`FunctionId`-based and reject ambiguous/duplicate binding-to-slot mappings.
3. Pin every grammar shape above, including Rust `self`, Go variadics, JS duplicates, and Python splats.
4. Specify lifecycle-safe telemetry storage and add full-vs-incremental equality tests.
5. Add cache pin updates: CPG 41→42 and nav sidecar 10→11, with their version tests.
6. Add negative and positive tests at each positional consumer, not only API-level extraction tests.

**Verdict: FIX.**

## P9 — Pointer-embedded Go fields

Confidence: high. The supplied extraction mechanism is exact.

### Grounding

`extract_one_field` takes the first non-name child as `type_str`; for `*Listener`, that child is the anonymous `*` token (`src/type_providers/go.rs:631-659`). `strip_pointer("*")` becomes empty and the embedding is dropped (`src/type_providers/go.rs:661-669`, `:1901-1904`).

The pinned Go grammar emits exactly `optional('*')` followed by a type identifier, qualified type, or generic type (`tree-sitter-go-0.23.4/grammar.js:365-380`). Named `l *Listener` instead carries its complete pointer type as the field’s `type` child, so the supplied distinction is correct.

### Findings

1. **WRONG — The design’s `*Iface` negative test will fail without an explicit semantic gate.**  
   After extraction, `GoEmbeddedField { name: "Iface", is_pointer: true }` reaches `embedded_interface_routes`. That loop ignores `is_pointer`, recognizes `Iface` in `data.interfaces`, and produces S4 candidates (`src/type_providers/go.rs:1495-1529`). Resolution can then mint Exact `InterfaceDispatch`. Tree-sitter accepts this syntactic shape even though Go’s type checker rejects embedding a pointer to an interface. Explicitly skip `embedded.is_pointer` whenever the target is an interface, in both routing and any satisfaction-membership path.

2. **WRONG — “Exactly as the non-pointer path” preserves the wrong implicit field name for qualified and generic embeds.**  
   `strip_pointer` removes only `*`; it does not reduce `pkg.Listener` or `Listener[T]` to the selector field name `Listener` (`src/type_providers/go.rs:1901-1904`). Thus `*pkg.Listener` would be stored under field key `pkg.Listener`, while source selectors use `s.Listener`. S2 misses the field, and the field fails to shadow a promoted method named `Listener`.  
   Store two values: implicit selector name `Listener`, and raw declared type `*pkg.Listener`/`*Listener[T]`.

3. **WRONG — The acceptance condition “drops do not rise, nothing else changes” rejects correct shadowing behavior.**  
   Embedded fields participate in method-shadowing through the pseudo-field list (`src/type_providers/go.rs:1716-1725`). Example: `S` embeds `*Listener` and `D`, while `D` has a method named `Listener`. Correct Go selection says the field shadows `D.Listener`; main can incorrectly promote the method because the pointer field is absent. The fix should remove that false Exact and may increase drops. Acceptance must allow explained removals as well as promotion gains.

4. **SMELL — The proposed child-loop repair is more fragile than the grammar’s named `type` field.**  
   `node.child_by_field_name("type")` already identifies the type node. Detect the preceding bare `*` separately and construct the raw type from those two facts. “Next non-tag/comment child” risks malformed-input tokens and relies on node kinds even though `tag` is a field name, not necessarily the child’s kind.

5. **SMELL — Cache/incremental coverage is incomplete.**  
   This changes promoted aliases, S2 field facts, S4 routes, and resolved nav topology. A CPG cache bump is correct, but repository convention also bumps `NAV_CALL_EDGE_CACHE_VERSION` (`src/navigation/call_edge_cache.rs:21-37`, pin at `:359-361`). Build identity independently protects current binaries (`src/navigation/call_edge_cache.rs:52-63`), so omission is not an immediate stale-load proof, but it violates the explicit topology-version convention. Add an incremental test where the embedding-defining file changes while the consumer remains untouched; both complete and incremental paths reapply the provider at `src/call_graph.rs:1262-1276` and `src/cpg/build.rs:343-366`.

### Ranked changes

1. Add the pointer-interface exclusion before S4/interface satisfaction.
2. Split implicit field name from raw declared type; test qualified and generic embeds.
3. Add shadow-removal and value-vs-pointer method-set poles.
4. Add full/incremental parity and serialized cache round-trip tests.
5. Bump both CPG and resolved-call-edge sidecar versions; permit explained drop increases in corpus acceptance.

**Verdict: FIX.**

## P10 — GoOwnerIdentity clause/build partitions

Confidence: high. The identity cut is directionally correct, but the consult contract is not safe as written.

### Grounding and second-copy census

The current identity is exactly `(package_dir, name)`, with the P13 limitation documented in source (`src/resolution.rs:216-237`). `go_field_types` and S4 are single-valued maps (`src/call_graph.rs:572-606`) and are populated by assignment from `GoTypeProvider` (`src/call_graph.rs:2814-2839`), so last-writer partition loss is real.

There are seven production `resolve_go_owner_identity` call sites:

- P5 composite/assignment registrations: `src/call_graph.rs:2995-3004`, `:3045-3056`
- P11 return/field recovery: `src/go_receiver_index.rs:223-236`, `:538-547`
- P5 invocation: `src/resolution.rs:1098-1124`
- S4 resolution: `src/resolution.rs:1763-1777`
- S4 manifest parity copy: `src/navigation/queries.rs:520-543`

The lifecycle copies are `empty`, skeleton, full, and direct-subset initializers (`src/call_graph.rs:609-672`, `:681-886`, `:1229-1276`, `:3841-3886`); whole-program clearing/rematerialization (`:1460-1541`, `:2687-2699`, `:2814-2886`); merge’s deliberate non-merge contract (`:1641-1670`); and full/incremental application (`:1262-1276`, `src/cpg/build.rs:323-366`). Serialization includes the entire `CallGraph` (`src/cpg_cache.rs:179-182`), with CPG version 41 and sidecar version 10 (`src/cpg_cache.rs:132`, `src/navigation/call_edge_cache.rs:37`).

### Identity cut

Adding `package_clause` to `GoOwnerIdentity` is the right cut. A Go namespace is directory plus package clause; `foo` and `foo_test` are not build variants of one package. Clause filtering only at consult time would require every positive-only P5 key and every registration key to carry declaration provenance and to be filtered perfectly forever. Structural identity makes cross-clause joins impossible by construction.

Build profile must not enter identity. Mutually exclusive files still declare the same logical package/type; their values need defining-file/profile provenance and consult-time filtering.

The P5 S1 func-value-field lane must move in the same PR. It shares the identity type, and leaving it semantically unmigrated preserves false registrations and false `FuncValueField` edges across `foo`/`foo_test` and build partitions (`src/type_providers/go.rs:147-163`; `src/call_graph.rs:2990-3067`; `src/resolution.rs:1090-1128`).

### Findings

1. **WRONG — The specified predicate can either drop every imported fact or admit uncertain Exact facts.**  
   `go_same_package_visible` returns only a boolean (`src/go_build_profile.rs:94-97`). Directly comparing caller and imported declaration profiles rejects them because their clauses differ (`:103-113`). HEAD’s safe typed-fact path instead rewrites the caller clause for cross-package visibility and then requires `visibility_allows_exact` (`src/go_receiver_index.rs:239-279`; exactness helper at `src/go_build_profile.rs:134-138`).  
   Concrete failures:

   - A `main` caller of imported `pkg.T` is filtered out solely because `main != pkg`.
   - A malformed/unparsed build expression can remain “visible” but uncertain; accepting its singleton value would mint Exact despite P13’s existing certainty floor.
   - A `foo_test` file using qualified `foo.T` targets the ordinary `foo` clause in the same directory; directory-only “same package” detection still rejects it.

   Use one shared detailed helper that takes the resolved target identity/reference mode, rewrites only package namespace for qualified imports, and rejects any survivor lacking exactness certainty.

2. **WRONG — Re-keying the output lanes does not repair S4’s upstream last-writer collapse.**  
   `GoTypeData.structs`, `interfaces`, and `methods` remain bare-name maps (`src/type_providers/go.rs:128-140`). `interfaces` is explicitly last-writer-collapsed, and `interface_name_owners` records only directories, not clauses (`:176-194`). `embedded_interface_routes` then reads that collapsed interface and filters own methods only by directory (`src/type_providers/go.rs:1495-1519`, `:1538-1553`).  
   Concrete case: `foo` defines `Doer{Prod()}` and `Holder{Doer}`; later `foo_test` defines `Doer{Test()}`. Adding clause to `Holder`’s route key does not stop the route builder from reading the `foo_test` interface and donating `Test` to the `foo` holder. S4 extraction and its shadow/method inputs must become clause/profile-aware before the final map is built.

3. **WRONG — A positive-only per-profile representation is insufficient for P5.**  
   `go_func_typed_fields` stores only fields whose type starts with `func(` (`src/type_providers/go.rs:152-157`). For linux `Command.Run func()` and windows `Command.Run int`, storing only the linux positive fact makes an unconstrained consult appear to have one surviving func-typed value; the conflicting non-func declaration is invisible. Store every relevant field declaration with a typed boolean/raw type and defining file, then require all visible declarations to agree.

4. **WRONG — P5 registration targets themselves require profile filtering.**  
   S3 collects every registration sharing the field key without examining the registration site’s profile (`src/resolution.rs:1118-1128`). Linux and Windows registration files assigning different handlers therefore fan out together even after the field-type lane is filtered. `RegistrationRecord` already carries `site.file` and `enclosing.file` (`src/call_graph.rs:257-269`); filter registrations using the invocation caller’s profile and the same certainty helper before applying the 1–3 target cap.

5. **WRONG — “Recall loss is bounded by the measured conflict count” is not true quantitatively.**  
   The counter counts conflicting owner identities, not affected call sites (`src/call_graph.rs:2759-2811`). One conflicting owner can feed arbitrarily many S2/S4/P5 consults. The count bounds the support set of affected owners, not the number of lost edges or sites.

6. **SMELL — Flattened `(profile,value)` facts do not encode absence.**  
   A build-specific struct definition that omits field `f` or S4 method route `M` must disagree with another definition that contains it. Store per-owner-declaration snapshots, or explicit present/absent facts, so a positive fact from one visible definition cannot stand in for every visible definition.

7. **SMELL — Missing/empty clauses need an explicit fail-closed rule.**  
   `extract_go_file_profile` uses an empty string when no package clause is proven (`src/go_build_profile.rs:61-75`), and exactness currently rejects empty clauses (`:90-92`). The new identity resolver must return `None`, not an identity containing `package_clause: ""`, for a bare or qualified reference whose clause cannot be proven.

8. **SMELL — Telemetry ownership is unspecified.**  
   Consult-time counters should travel through immutable `ResolutionOutcome.telemetry`, as current partition counters do (`src/navigation/queries.rs:214-230`), not mutate `CallGraph` during queries. Build-time P5/S2 counters need per-file or whole-program-rematerialized storage. Also redefine `go_owner_identity_profile_conflict`: once clause enters the key, cross-clause conflicts disappear from the old counter unless it deliberately retains a legacy `(dir,name)` diagnostic.

### Test gaps

The proposed P11 S2/S4 shapes are necessary but insufficient. Add:

- P5 `foo`/`foo_test` func-vs-nonfunc field definitions, both file orders.
- Linux/Windows func-field definitions plus distinct registrations; linux invocation must see only linux target, Windows only Windows, uncertain/unconstrained conflict must drop.
- S4 interface definitions split across `foo`/`foo_test` and across build partitions, testing both resolver and manifest copies.
- Qualified imported `pkg.T` resolution across directories and from `foo_test` back to ordinary `foo` in the same directory.
- Missing/unparsed clause/build expressions: never Exact.
- Same-value duplicate visible facts: permitted only when every survivor passes the certainty floor.
- Bidirectional incremental parity: change declaring file with consumer retained, then consumer with declaration retained.
- Round-trip tests that preserve multiple facts, registration provenance, new telemetry, and clause-bearing identities.

### Ranked changes

1. Add clause to identity, but implement a shared detailed/certain local-vs-qualified visibility helper.
2. Convert S2 and S4 upstream provider data to per-declaration snapshots; eliminate clause/profile last-writer collapse before route generation.
3. Migrate P5 in the same PR: all-field typed state, profile-aware registration creation, and profile-filtered registration targets.
4. Define missing-clause behavior and presence/absence semantics.
5. Wire both S4 consult copies through one helper and pin resolver/manifest equality.
6. Add lifecycle, serialization, CPG-version, and sidecar-version tests.
7. Replace the recall-bound claim with measured affected-owner, affected-site, and affected-edge counts.

**Verdict: FIX.**

**OVERALL: FIX — all three mechanisms identify real defects, but P8 cannot deliver its Go positive pole, P9 would mint an invalid pointer-interface route, and P10’s current filtering/provider/P5 contract can still produce false Exact edges or erase valid imported ones.**

---

## Item P8 — Fail-closed parameter slots (JS/TS default/rest/destructured; Go grouped declarations)

**Problem (WRONG on main, found by the A2 spec review).** `ParsedFile::function_parameter_names` (src/ast.rs ~L5868)
and `function_parameter_occurrences` (~L5887) return compressed vectors: each per-parameter helper returns
`Option<String>` and `None`s are dropped. JS `assignment_pattern` (default), `rest_pattern`, `object_pattern`/
`array_pattern` have no arm at the parameter-extractor level; Go `parameter_declaration` (`a, b string`) returns only
the FIRST identifier (the arm at ~L6381 "C/C++/Java" also matches Go's node kind). Positional consumers then mis-slot:
`function invoke(x = 0, cb){cb()}; invoke(safe, 0)` → Level-3 callback resolution (src/call_graph.rs ~L1835-1885)
finds `cb` at index 0 and mints a FALSE `cb → safe` edge; Go `func f(a, b string, c int)` → Step-5b binds argument 1
to `c`. Positional consumers (sol's census): `compute_param_names` + both Step-5b loops (src/cpg/build.rs),
Level-3 callback resolution, `primitive_slice` positional index (~L317-347,407), `peer_consistency_slice` first-slot
(~L188-207), eager `FunctionInfo.param_names` (src/ast.rs ~L95-103,521-534). Non-positional: `quantum_slice` name
sets, DFG Defs (src/data_flow.rs ~L265-287), reasoning seeds (src/reasoning/seeds.rs ~L253-267).

**Design.**
1. New `ParsedFile::function_parameter_slots(&Node) -> Option<Vec<String>>` (+ byte-bearing twin
   `function_parameter_slot_occurrences -> Option<Vec<ParameterOccurrence>>`): one entry per RUNTIME slot in
   declaration order; Go grouped declarations contribute every name; returns `None` if ANY runtime slot is
   unrepresentable (JS/TS `assignment_pattern`/`rest_pattern`/destructuring, TS typed parameters, Java, or any
   `ERROR`/missing node inside the parameter list — parse recovery is fail-closed). Non-runtime pseudo-params
   (TS `this`, Java receiver) are NOT slots (they are unhandled today and would make the list `None` — acceptable
   until A2 adds them explicitly).
2. Positional consumers switch to the slot API and SKIP the function when `None` (no edge beats a wrong edge):
   `compute_param_names` (both Step-5b copies), Level-3 callback resolution, `primitive_slice`, `peer_consistency`
   (first slot), eager `param_names` fallback (store `Option`). Name-set / Def / seed consumers keep
   `function_parameter_names` (names are still real names; no positional meaning) — byte-identical there.
3. Go grouped names are the ONLY recall increase (unambiguous). No new materialization for JS/TS/Java.
4. Cache: persisted DFG edges + synthetic Level-3 CallGraph sites change → ONE `CACHE_VERSION` bump; resolved-call-
   edge sidecar version bump per its topology-change convention.
5. Telemetry: count functions whose slots are `None` per language (`param_slots_unknown`) so the recall cost is
   measurable; Level-3 edges removed are visible in the call-stats kind histogram.

**Safe failure direction.** Fail closed (lose positional edges on mis-slotted functions); never compress.

**Tests (both poles).** JS: default-param callback case → NO `cb → safe` edge (red on main); rest + destructured
variants; a plain-identifier JS function keeps its callback edge (positive). Go: grouped `func f(a, b string, c int)`
→ Step-5b arg1→b, arg2→c (red on main: arg1→c); P5 Go callback tests unchanged. TS typed params → positional
consumers skip (today absent). Python/Rust/Java existing parameter tests byte-identical. Parity oracle. Parse-error
parameter list → `None`.

**Acceptance.** Full suite; tier-a matrix 0 regressions with every fixture flip enumerated; call-stats before/after on
a JS/TS corpus (false Level-3 callback edges removed — histogram delta reported) and on caddy/prometheus (Go: grouped-
name Step-5b recall; resolution counts byte-identical or explained); the JS false edge reproduced end-to-end
before/after with `prism nav callers`.
## Item P9 — Pointer-embedded Go fields (`*Listener`) are dropped by `extract_one_field`

**Mechanism (confirmed).** `src/type_providers/go.rs::extract_one_field` (~L632-676) collects `field_identifier`
children as names and the FIRST other child as `type_str`. tree-sitter-go emits an embedded pointer field as a bare
`*` token followed by `type_identifier`/`qualified_type` (grammar: `seq(optional('*'), choice(type_identifier,
qualified_type), tag?)`), so `type_str == "*"`, `strip_pointer` yields "", and the embed is silently dropped
(fails safe; affects shipped embedding promotion #95 and P11 S2/S4 lanes).

**Design.** In the child loop, when the first non-name child is the bare `*` token, record `is_pointer = true` and
take the NEXT non-tag/comment child as the type (keep `strip_pointer` for the `pointer_type`-node case so named
fields `l *Listener` are unchanged). Emit `GoEmbeddedField { name, is_pointer: true }` and the `(name, "*T")` field
entry exactly as the non-pointer embed path does. No other behavior change.

**Tests (both poles).** `struct S { *Listener }` → embedded Listener is_pointer (red on main: absent); `struct S {
*pkg.Listener }` qualified; `struct S { Listener }` unchanged; `struct S { l *Listener }` unchanged (named, not
embedded); resolution-level: `s.Serve()` where `Serve` is a method of `Listener` promoted through the pointer embed
→ Exact (red on main); P11 S2 field-typed receiver through the pointer embed; S4 embedded-interface via pointer
embed of an interface is NOT a thing in Go (`*Iface` embed is invalid) — assert nothing minted.

**Acceptance.** Full suite; tier-a 0 regr; call-stats on caddy + prometheus: embedding-promotion Exact count rises,
drops do not rise, nothing else changes (report the histogram delta); one `CACHE_VERSION` bump (persisted CallGraph
struct facts).
## Item P10 — GoOwnerIdentity clause / build-partition blindness (P13 M1)

**Mechanism (confirmed).** `src/resolution.rs::GoOwnerIdentity { package_dir, name }` (L231) intentionally omits
package clause / build profile (P13 note L225-229). Lanes keyed by it — `CallGraph.go_field_types:
BTreeMap<(GoOwnerIdentity, field), String>` (P11 S2, single value per key → last-writer-wins across partitions),
the S4 embedded-interface map, the S1 func-value-field index — can therefore cross `foo`/`foo_test` and build
partitions; `count_go_owner_identity_profile_conflicts` (call_graph.rs ~L2759) measures owners whose declaring files
span >1 profile signature (clause|is_test|goos|goarch|build_expr): etcd 1, prometheus 5. P13 already profile-filters
the same-package consult sites (`go_build_profile.rs::go_same_package_visible` L95) — this item brings the
identity-keyed lanes to the same standard.

**Design.**
1. Identity gains `package_clause: String`. `resolve_go_owner_identity(type_text, file, imports, package_basenames,
   go_file_profiles)`: bare `T` → (dir(file), clause(file), T) — Go scoping: a bare name is the caller's own package
   (a `foo_test` file's bare `T` is a `foo_test` type; it must write `foo.T` for the production type); `pkg.T` →
   (dir, the dir's single ORDINARY (non-test) clause, T) — `None` if the dir has ≠1 ordinary clause. Every lane
   constructor/populator uses the DECLARING file's clause; every consult site the caller's.
2. Build partitions are NOT identity: lane values become per-declaring-profile entries (e.g. `BTreeSet<(GoBuildProfile
   key, value)>`); each consult site filters by `go_same_package_visible(caller_profile, decl_profile)` and accepts
   only ONE distinct surviving value (precision floor: >1 → drop, 0 → drop).
3. Second-copy census (doctrine 6): `CallGraph::empty`/skeleton/full/subset builders, `remove_files`, `merge`,
   serialization + the pinned cache-version test, `apply_go_interface_dispatch` / `apply_go_receiver_indices`
   populators, all P5 S1/S3 and P11 S2/S4 consult sites. One `CACHE_VERSION` bump; edge-index sidecar bump per convention.
4. Telemetry: keep `go_owner_identity_profile_conflict` (diagnostic); add `go_owner_identity_partition_drop`
   (consult-time drops due to conflicting survivors) and `..._partition_recovered` (a conflict resolved by the filter)
   so BOTH poles are visible in `prism nav call-stats`.

**Safe failure direction.** Drop, never fan out; a formerly-Exact edge that crossed a partition becomes a drop (that
was a false Exact). Recall loss is bounded by the measured conflict count.

**Tests (both poles).** Same dir: `foo` struct `T{f Conn}` and `foo_test` struct `T{f *Mock}` → a `foo` caller's
field-typed receiver `t.f.Dial()` resolves to `Conn.Dial` only (red on main if last-writer was the test file; assert
both orders by fixture file naming); `x_linux.go`/`x_windows.go` both declare `T` with different `f` types → caller in
a `//go:build linux` file resolves the linux value, an unconstrained caller with two visible conflicting values →
drop (counter increments), a single-visible case recovers; S4 embedded-interface same shapes; qualified `pkg.T`
with a dir containing `foo` + `foo_test` clauses → ordinary clause chosen; non-Go byte-identical; P5/P11 existing tests
green; full-vs-incremental rebuild equality; serialization round-trip.

**Acceptance.** Full suite; tier-a matrix 0 regr; call-stats on prometheus (and etcd if cloned under
~/code/bench-repos): partition counters populated, Exact counts unchanged or each delta explained, drops unchanged
outside the new partition-drop counter; caddy/ripgrep byte-identical.
