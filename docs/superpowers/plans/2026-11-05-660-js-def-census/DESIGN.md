# #660 design (round 3, folded, R4 fixes applied): a JS/TS/TSX reaching-definitions def census that is complete by construction

Reviser: Fable 5.1, 2026-11-05 (rounds 1 and 2 by Opus 5.5); round-3 fold of `astra-r3.md` and `fable-r3.md` per `fold-r3.md`
(the pre-fold round-3 text is `DESIGN-r3.md`, its matrix `variant-matrix-r3.tsv`; the folded text before astra's R4 fixes is
`DESIGN-r3fold.md`). §18 is the S0 implementation contract. Base: prism `main` 105599ee (`~/code/prism-660`, untouched; main has
since moved to f4c6297d with CJS-only changes, no RD change). Round 2 is kept as `DESIGN-r2.md` and `variant-matrix-r2.tsv`; round
1 as `DESIGN-r1.md`. sol's reviews are `sol-r1.md` and `sol-r2.md`; the owner's decisions are in `revise-r2.md` and `revise-r3.md`.
Paths below are relative to `~/prism-evidence/jsts-lane/660/design/`.

## 0. What changed in round 3 (finding -> change)

sol's round 2 left three findings open and classified the loop **open-class**: identity and ordering failures kept appearing as
new instances. Round 3 does not patch the three instances. It states one soundness principle (§5.1), enumerates every place the
design keeps `exact` despite a write or compares identities (§5.2, §5.3), and either derives each one from the principle or
removes it. The three open mechanisms are replaced by constructions that have no case list left to extend:

| finding | round-2 mechanism (open) | round-3 construction | where |
|---|---|---|---|
| **W1-R2** identity: escaped member components (`o.x++`) evade a base-name refusal | decode escaped *variable* tokens, refuse the decoded base name | **One identity function** (§4.4): every path is built from raw NAME-token spellings. It is admissible only in an owner whose subtree holds no NAME token containing `\`; otherwise the **whole owner is refused**. In an escape-free owner, raw spelling equals decoded spelling for every token, so every comparison in the owner (kill buckets, candidate matching, collapse, capture references, binding facts, D2-c, D3, dead formals, Annex B) compares decoded identities without decoding anything. No per-component rule exists to get wrong. | §4.4, §5.2, rows w07-w13 |
| **W3-R2** creation: a hoisted declaration exists at scope entry; the line mapping and `path_exists` cannot prove non-execution | D2-c with one order proof (`!path_exists(node(R.start), node(U))`) | **The creation exception is removed** (owner D2-c, recovery #667). A write inside any deferred region R to a binding B that R does not itself bind refuses every edge of B whose use is outside R. No creation anchor, mapping or reachability notion is needed. #667 gets a specification (§6.3) and a measured value (heuristic: 68 edges on excalidraw, 1 on express). | §6.2, §6.3, rows h01-h11, b03 |
| **W4-R2** order: byte order is not evaluation order (`const [b = sink(a)] = (a++, [])`) | byte keys with special cases for `before-target` and `after-body` | **A closed evaluation-order table over every grammar kind** (`census/order.tsv`, 186 rows, certificate check 5): a position takes part in an order proof only when every kind on its node chain is `seq` (source order) and byte order holds; loops, patterns, switch clauses, class bodies, decorators, parameter lists, field initializers, `with` and regions refuse (after the fold, §0.1 item 1-2: the round-3 two-pass `for` model and first-evaluation argument are withdrawn, two one-fact exceptions remain). | §7, rows o01-o39 |
| **S2** reporting | "upper bounds" | every parse-only number is a **heuristic candidate count** (the estimator approximates scopes and node mapping); the production trace of §10 is the measurement | §11 |

### 0.1 Round-3 fold (astra W1-W5, S1; Fable F1-F4, S1-S6)

Stance (controller): where a reviewer falsified a proof, the proof is narrowed to what a reader can check exhaustively, and the
rest refuses; recovery of labels is #670 (ordering) and #667 (creation timing).

| item | finding | fold |
|---|---|---|
| 1 | astra W2 / Fable F1: the first-evaluation witness fails in repeated contexts (`do { if (k++) sink(a); a++; } while (k < 2)`; `for (; c(); a++) a = g()`) | **proof narrowed.** A position whose node chain (§7.1) contains a loop kind takes part in no order proof, use side or def side. One exhaustively checkable exception remains: the single-site operand (the only counting write in N is an `RW`/update/`delete` whose own operand is the read). The two-pass table is gone, and the for-initializer exception the fold first kept was withdrawn after astra R4-W1 (`do { for (a = 10; false;) .. } while ((a++, false)); sink(a)`: a counting write outside the inner `for` shares the node); its labels (c03, d05, a18) go to #670. Rows a04-a08, a17-a20, fb01, fb07, fb09, o35, o37, o38 |
| 2 | astra W1 / Fable F2: `switch` is not source order across clause selection | **proof removed.** `switch_body`, `switch_case`, `switch_default` are `unknown`; the multi-line form is sound through E18 (a case-value site maps to the `switch` header node and kills there, §5.3 E18). Rows a01-a03, fb02, fb03, fb11, o06 |
| 3 | astra W3: the covered check compared two recomputed parses, not what RD's kill machinery uses | **proof narrowed to the stored representation.** A site is covered only when a recorded `DefSite` has the **stored** `path` equal to `ident(site)` and the **same `start_byte`** as the site's target node; anything else adds the canonical kill-only def (§4.4, A7). o23 tests the completed def list and the kill |
| 4 | astra W4: a body `var`/function declaration was taken to bind a parameter-default write | **exclusion narrowed.** A write in a parameter region is inner only when a parameter of that same list declares the name; body declarations never bind it. A strict-mode block function declaration scopes its block only. Rows a10-a14, a12 control |
| 5 | Fable F3: hoisting rules were stated over "body-top-level"/"block-level" | **rule restated by syntactic position**: dead formal = a declaration reachable from the body statement list through `labeled_statement` only; Annex B = any lower position. Rows fb05, a15, fb06 |
| 6 | astra W5 / Fable S5: duplicate table rows accepted; n1 record wrong; method keys/decorators unstated | check 0 rejects every duplicate key in every table (controls n9, n10); `negative_controls.txt` re-recorded with exit codes read directly; method computed keys and decorators are eager, unmapped sites (§4.4) |
| 7 | Fable S1 / astra E11 note: `decorator`, `class_body` reasons false | `decorator`, `class_body`, `class_static_block` -> `unknown`; S0 requires one fixture per `seq` row with a non-trivial argument and one per exception rule (§7.4) |
| 8 | Fable S2, S4: expected labels for unmapped eager sites; missing rows | o18/o19/o20/o24/fb15 -> `cfg_incomplete` via `site-unmapped`; E11 reworded; rows fb07, fb09, fb11, fb12 added |
| 9 | Fable F4, S3, S6; astra E17 note | G5 (try-header exception edges, #673) and G6a/G6b added to §5.4; the header-increment over-kill class named with trace form `kill:for-increment` (§7.3); P qualified by the model assumptions M1-M3 (§5.1) |
| 10 | astra S1 / Fable: slices too wide | S1b split into S1b-1 (identity/coverage wiring) and S1b-2 (order relation + D3); S1c is the scope-exclusion contract; `first_hazard` through S1d; two-round cap per unit (§9) |
| 11 | §16 | adopted: owner-wide escape refusal, `super.x` -> `this.x` may-write, ERROR/MISSING owner refusal; issue text for G1/G2(+G6) in §17 |
| R4 | astra R4-W1 (the for-initializer exception bypasses an enclosing loop's same-node write), R4-S1 (stale text) | E4 removed for this lane (row a20 mandatory; c03, d05, a18 become cost rows; recovery #670); every superseded passage reconciled; §18 added |

Kept from round 2, unchanged beyond what the identity change requires: the W2 certificate (signature, coverage, leaves, target
closure; round 3 adds check 5 for the order table), the S1 deferred-read contract (`capture.rs` regions), slices S0-S1d with
`first_hazard` kept until S1d, the parity tests, the three-corpus cloud gate including TypeScript `src`, D1-D7.

## 1. Goal served

DataFlow label precision for JS/TS/TSX. A wrong `exact` (and hence `asserted`) is worse than an honest `NameOnly`. #660: a
missed write between a recorded def and a read must not leave the read `exact`. The census must be complete by construction, so
that the enumerated one-node guard `first_hazard` (#657) can be deleted.

## 2. Where definitions come from today (unchanged from round 1, cited)

The producer (`data_flow/build/file.rs`; frozen twin `legacy_producer.rs`) assembles `rd_defs` from formals
(file.rs:181-226), lvalue spans (file.rs:91 -> ast.rs:5985/6147/6415/6626, for heads ast.rs:6206/6559), alias twins
(file.rs:263-345) and F1 kill-only nested writes for synthetic owners (file.rs:391-431). It passes the same `rd_defs`
to `unavailable_before_edges` (file.rs:438) and `reaching_definitions_with_exact` (file.rs:719). Both deduplicate
(`preflight.rs:15`, `reaching.rs:249`). The one-node gate is at reaching.rs:282 and preflight.rs:33, capture facts at
reaching.rs:394, `classify_edge` at reaching.rs:571 (both edge loops call it, 416/464), kill sets at 372, the use node's
incoming set at 734-738. Recorded spans miss pattern defaults, updates, member targets in array/rest patterns, `delete`,
wrapped/member for heads, and escaped spellings. The nested-callable statement universe is not the owner's:
`statements_in_function` (statement_traversal.rs:44) recurses into blocks and control-flow bodies only, and function
declarations are not statements, which is why sol's `function set() { a++; } log();` line maps to `log()`.

## 3. Record versus refuse (D1: kill-only now; #669 later)

Recording a missed write as an emitted def adds edges, because emission (`file.rs:526-580`) links every lvalue span to
every reference of its path. Kill-only defs (gen and kill only, never in the `defs` map) are label-only by
construction. The round-1 prototype (`proto/prototype-660.patch`) measured identical edge sets on excalidraw, express
and 95 fixtures, with express -10 exact and excalidraw -41 (`corpus/protob_compare.txt`). Residual edge-set paths are
watched by the gate: the def cap (2048), availability flips above the 1,000,000-emission fallback cap, and
supplemental-exact conflict lines.

## 4. The closed universe and its certificate (W2, closed in round 2; round 3 adds the order table)

### 4.1 Tables (`census/`)
- `table.tsv`: 142 write-relevant (kind, slot) rows: role, order hint, reason (unchanged from round 2; the `order` column is
  now informational, the proof relation uses `order.tsv`).
- `leaves.tsv`: all 33 named leaf kinds; BIND and NAME columns (8 NAME kinds: `identifier`, `private_property_identifier`,
  `property_identifier`, `shorthand_property_identifier`, `shorthand_property_identifier_pattern`, `statement_identifier`,
  `type_identifier`, `undefined`). The BIND set is a subset of the NAME set, so every token that can spell a binding is scanned
  by the escape rule.
- `targets.tsv`: 78 kinds, each with an action and its carrying slots.
- **`order.tsv` (new)**: every named, non-supertype kind of the three grammars (186 kinds) with one evaluation-order class:
  `seq` (61: children evaluated in source order, a write construct writes at its end byte), `loop` (4: `for`, `for_in`,
  `while`, `do`: repeated contexts, no order proof through them), `unknown` (24: the six pattern kinds, the three `switch`
  clause kinds, `class_body`, `class_static_block`, `decorator`, parameter lists, field definitions, `with`, and module-level
  syntax that never sits in an owner), `deferred` (6 callable kinds), `none` (58: type positions, declaration-only syntax,
  trivia), `leaf` (33). §7.1 gives the argument for each non-trivial `seq` row.

### 4.2 Checks (`census/check.py`, exit 1 on any failure)
0. **Duplicates (fold, astra W5).** Every table rejects a duplicate key, identical or not, before any row is read
   (`DUPLICATE-ROW`); the Rust loader of S0 applies the same rule.
1. **Signature.** `signatures/{javascript,typescript,tsx}.sig` (772/1,297/1,351 lines) record every kind, supertype member,
   slot and accepted alternative. Any added or removed line fails until re-reviewed and re-recorded.
2. **Coverage.** Every write-relevant slot has exactly one `table.tsv` row.
3. **Leaves.** Every named leaf has a row; BIND equals the enumeration's name set.
4. **Target closure.** Every kind reachable from a target or binding slot (through `RW`, `DEL`, `WRAP` and carrying slots)
   has a `targets.tsv` action and keeps its target role.
5. **Order (new).** Every named non-supertype kind of each grammar has exactly one `order.tsv` row with a class from the closed
   vocabulary; a `leaf` row must be a leaf and a non-leaf row must not be (`UNCLASSIFIED-ORDER`, `STALE-ORDER`,
   `BAD-ORDER-CLASS`, `ORDER-LEAF-MISMATCH`). A kind the grammar adds therefore fails twice: the signature and the order table.

### 4.3 Negative controls (`census/negative/`, results in `census/negative_controls.txt`)
Round 2's n1-n6 still fail (n1 now also reports `UNCLASSIFIED-ORDER future_target`). Round 3 adds n7 (an order row removed:
`UNCLASSIFIED-ORDER for_in_statement`), n8 (an order class outside the vocabulary: `BAD-ORDER-CLASS`), and the fold adds n9
(a duplicate row with a valid class) and n10 (a duplicate row with an invalid class), both `DUPLICATE-ROW`. Every control's exit
code is read directly (the earlier `exit=0` for n1 was a pipeline's exit, Fable S5). Baseline: `PASS: 0 problem(s); slots 142,
rows 142, leaves 33, target kinds 78, order rows 186`.

In-repo form: §18 (S0): a Rust test that runs checks 0-5 over `Language::node_types_json()` (reusing `NodeTypeSchema`,
`scope.rs:17-38`), with the TSVs and signatures embedded via `include_str!`; the ten mutated inputs become fixtures and are
asserted to fail with the named diagnostic.

**What remains human (stated, not hidden).** Whether each `R`/`N`/`T` role row and each `order.tsv` class is semantically
right. Every non-`R` role, every non-`seq` order class and every `seq` class with a non-trivial argument (§7.1) has a fixture
row. A re-recorded signature is reviewed like code.

### 4.4 The identity function (W1-R2)

Everything the census compares is a **path**: a base name and a sequence of components. One function, `ident`, builds it;
nothing else does. Its domain is a NAME leaf or a member/subscript chain in a target or read position of an owner.

**Admissibility (owner-level precondition).** The owner's subtree, nested callables included, holds no `ERROR` or `MISSING`
node and no NAME-token leaf whose raw text contains `\`. An owner that fails the precondition is **refused as a whole**
(`owner-refuse:parse`, `owner-refuse:escape`): every edge classified in it is `NameOnly(CfgIncomplete)`, availability untouched.
The scan is a leaf walk over the eight NAME kinds; there is no decoding step, so there is no decoder to get wrong and no
undecodable case to handle.

**Conversion** (returns `Path`, `NoWrite` or `Refuse`):
- a NAME leaf -> `Path { base: raw spelling, components: [] }`;
- `member_expression` -> `ident(object)` with the property's raw spelling appended (`property_identifier` or
  `private_property_identifier`);
- `subscript_expression` -> `ident(object)` with the component `[]` appended, whatever the index (the index is a read);
- a WRAP kind (`parenthesized_expression`, `non_null_expression`, `as_expression`, `satisfies_expression`,
  `type_assertion`) -> `ident` of its one non-`TYPE` child; any other shape of wrapper -> `Refuse`;
- an `optional_chain` token anywhere in the chain is ignored for identity (`o?.x` is `o.x`, as `AccessPath::from_expr`
  already rewrites it); the write is a may-write, which a kill-only def models;
- a root of `this` -> base `this`; a root of `super` in a write position -> base `this` (a `super.x = v` write lands on the
  receiver); any other root (a call, `new`, a literal, `await`, a ternary, an arrow...) -> `NoWrite`: no owner binding is
  written. This is the name-based model's answer and the same one the recorded extractor gives (`extract_lvalue_paths`
  returns nothing for a non-identifier base); its limit is aliasing (§5.4, G4);
- a `REFUSE` kind of `targets.tsv` in a `W`/`RW` context, an unclassified kind in a target context, or a chain the rules above
  do not cover -> `Refuse` (owner refusal).

**Regions versus eager class positions (Fable S5).** A `method_definition`'s region is its parameter list and body. Its computed
key and its decorators, like a class's decorators, heritage expression, static field initializers and static blocks, are eager
sites: they run at class evaluation and are mapped by E18 (a class or enum declaration line is not a CFG node, so such a site is
usually unmapped and refuses its base, trace `site-unmapped`; inside a statement span, `const C = class { ... }`, it kills at
that node). For order proofs every one of them is under an `unknown` kind.

*Round-1 fold (S0 review, PR #676).* Region flags are assigned **by child position**, not by the parent's callable class: a
callable's deferred region is entered only through its parameter list(s), its body and a function expression's own name. A
method's `name`/computed key and its decorators (JS: the `decorator` field of `method_definition`; TS: `class_body` decorators)
keep the enclosing region, under both grammars; "eager" never clears an enclosing deferred flag (a key inside an already
deferred callable stays deferred through its ancestor). A method that is itself the owner does not walk its own key or
decorators (they are sites of the enclosing owner). TS **parameter decorators** (`constructor(@dec(a = 1) a)`) are eager and
run in the scope enclosing the callable: a write inside one is a site of the enclosing region, not a parameter-region write,
and §6.2's parameter-list inner-binding rule does not apply to it (it writes the outer `a`). A `delete` whose operand is not a
written reference (a call, assignment, sequence or other refused kind) writes nothing but is still evaluated: its children
are walked as reads, so writes nested in the operand (including closure writes) remain census sites.

*Round-2 fold.* Only the **owner's own** parameter decorators are sites of the enclosing owner and skipped; a nested
callable's parameter decorators (including a class defined inside the owner's parameter defaults) are walked in the region
enclosing that callable. The walk does not descend into kinds that `order.tsv` classes `none` (no runtime evaluation: function,
method, abstract-method, call and construct signatures, interface members, type positions), so the parameters of an overload,
an abstract method or an interface signature are not census sites; a real implementation's parameters are.

**Relations on paths.** Two, and every comparison in the design is one of them:
- `eq(p, q)`: same base, same component sequence, components compared as raw strings (`[]` equals `[]` only). This is the
  relation main already uses for recorded defs (kill buckets, candidate matching, collapse, capture references) and the census
  inherits it there unchanged.
- `affects(w, q)`: same base, and `w`'s components are a prefix of `q`'s (same length allowed), where a `[]` component matches
  any component on either side. A write to `w` changes the storage of every path at or below `w`; a read of a path strictly
  above `w` (a bare `o` after `o.x = v`) reads a reference the write did not change. The census uses `affects` for every
  comparison it introduces: kill-only kills, D2-c, D3, dead formals and name-wide refusals (a name refusal is `affects(base, q)`).

**Why no comparison escapes the function.** Every comparison in the owner is listed in §5.2 with the relation it uses. All of
them compare raw spellings of tokens inside the owner's subtree, or paths `ident` built from them. Tokens outside the owner
never enter: `BindingFacts` declarations come from the owner's defs and its subtree (`scope.rs:164-262`), capture references from
its nested bodies (`capture.rs:50`), kill buckets from its defs; strictness and `arguments`/`eval` detection look outward but
compare against literals through helpers that already decode (`ast_callback_identity.rs:644,678`), and an escaped spelling of
`arguments` or `eval` inside the owner refuses the owner before those helpers matter. In an escape-free owner, raw equality is
decoded equality (a token without `\` denotes itself), so the one identity is the decoded identity.

**Cost.** Owner-wide refusal is stricter than sol's "refuse the receiver's root": rows w10 and w13 show the extra cost
(an escape on another object or in a label refuses the owner). Heuristic count on excalidraw and express: 0 owners, 0 edges
(no escaped NAME token in either corpus). The TypeScript corpus is measured by the gate.

Each census site carries its path, the statement node it maps to (§5.3 E18), its write construct (§7.1) and region flags
(own parameter list, deferred region, `with` body). The round-3 estimator (`census/analyze_r3.py` over
`census/runtime_census_r3.py`) selects exactly the round-3 rows and none of their controls (§11).

## 5. One principle, and every place the design keeps `exact` or compares identities

### 5.1 The principle

**P.** An edge D->U in owner O is labelled `exact` only when prism exhibits an execution of O on which U reads the value D wrote:
(i) D and U name the same binding and the same storage path (`ident`, `eq`); (ii) the CFG has an unflagged route from D's
node to U's node along which no node writes that storage (the reaching-definitions solver over gen/kill sets that the census
has made complete); and (iii) within D's node and within U's node, the evaluation order proves that no same-storage write sits
between D and U on that execution (§7). Every write the census finds whose binding or path is **not proven distinct** from
D's, and whose position is **not proven ordered** away from the D->U interval, demotes the edge. "Proven" means derived by one
of the rules in §5.3; every other answer is "unknown", and unknown refuses.

**P is a claim inside prism's model, not about every runtime** (astra E17 note, Fable). The model assumptions it rests on,
each pinned by rows: **M1** a read executes whenever its CFG node does (line granularity: `if (k) sink(a);` is one node; the
multi-line do-while of a05 is "right" under M1 and only under M1); **M2** paths are names, not objects (G4 aliasing; G1/G2
inherited identity, §5.4); **M3** the CFG's routes are the routes (G5: the try-header exception edge, #673). Within M1-M3 the
rules below are sound; outside them the G rows pin main's behaviour and are out of this lane.

Two consequences shape the whole design. First, every rule below is either a *refusal* (never keeps `exact`, so it needs only a
cost, not a proof) or a *proof* (keeps `exact`, so it needs a derivation from P). Second, the set of writers of an owner-local
binding is closed: sites in O's subtree (the census), direct `eval` (refused), `with` (refused), mapped `arguments` (refused),
and runtime mutation (out of model by the CLAUDE.md rule). A write by other code to a binding O declares has to come through
one of these, so the census plus the three refusals see every write P needs.

### 5.2 Table A: every identity comparison

| # | comparison | compares | relation | unknown branch |
|---|---|---|---|---|
| A1 | kill buckets (`def_buckets.rs:26,86`) and `same_def_binding` | recorded def path vs def path; binding ids | `eq`; binding id, `FlatFallback` matches anything | inherited: an unresolved binding kills everything of its name |
| A2 | kill-only kills (new, S1b) | census write path vs every recorded def with the same base | `affects`; binding: same id or either side `FlatFallback`/unresolved | unknown binding -> kill |
| A3 | candidate matching (`matching_defs`) | edge.from path, line, byte vs def | `eq` (inherited) | fallback to (path, line) |
| A4 | `collapsed_groups` | (path, line) counts over recorded defs only | `eq` (inherited); kill-only defs excluded (§5.3 E1) | — |
| A5 | capture references (`capture.rs:50`) | use (path, line) vs nested-body references | `eq` on raw text (inherited) | — |
| A6 | `BindingFacts` declarations, `visible_declaration`, `relation` | def base vs declaration names | raw string equality (inherited) | `Unresolved`/`FlatFallback` -> conservative paths already in `classify_edge` |
| A7 | covered check for a census site (S1b-1) | `ident(site)` vs the **stored** `DefSite.path` of a recorded def whose `start_byte` equals the site's target node start (astra W3) | `eq` on the representation RD's kill buckets key (`def_buckets.rs:26,86`); binding provenance is identical by construction (the same base at the same byte resolves to the same declaration) | any mismatch -> the canonical kill-only def is added (a duplicate at a recorded site is removed by `deduplicate_definitions` when identical, and harmless otherwise: no edges, no collapse) |
| A8 | D2-c (§6.2): nested write vs edge | write path vs use path; binding B of the write vs the candidate def's binding | `affects`; B = the innermost declaration by scope (§6.2 item 2) | B not found or `FlatFallback` -> refuse every edge of that base |
| A9 | D2-c inner-binding test | write base vs declarations inside R whose scope contains the site | raw equality + the scope rule (§6.2): parameter-region writes are bound only by a parameter of the same list; a strict block function scopes its block | not proven inner -> outer (refuse) |
| A10 | D3 (§7.2): writes in the use node / def node | write path vs use path (use side), vs def path (def side); bindings as A2 | `affects` | unknown binding -> treated as the same binding |
| A11 | dead formals (§6.1) | formal name vs body-top-level `function`/`generator` declaration names, and vs own-parameter-list write bases | raw equality (escape-free owner) | — (refusal) |
| A12 | Annex B (sloppy) | block-level function declaration name vs owner def bases | raw equality | — (refusal) |
| A13 | `arguments`, `eval` detection | token vs literal | existing decoding helpers; an escaped spelling in the owner refuses the owner first | — (refusal) |
| A14 | one-node distinct-bases belt (`js_single.rs:79`) | def bases over the completed def list | raw equality | — (refusal) |
| A15 | `with` body writes | write path -> kill-only def | `ident` | — (refusal: may-write kills) |
| A16 | name-wide refusals (W1 is owner-wide now; `arguments`, Annex B, unmappable site E18) | refused base vs edge paths | `affects(base, q)` | — (refusal) |

No comparison is made on a decoded spelling, on `AccessPath::from_expr` of a census token, or on a path built outside `ident`.
The recorded extractor still builds recorded paths its own way (A1, A3-A6 are inherited); coverage (A7) is a statement about
that stored representation, not about the syntax: where the stored path diverges from `ident` (o23: `o!.x = 2` is recorded with
a different spelling on main), the site is not covered and a kill-only `o.x` is added, which kills the earlier `o.x` def in the
same buckets the recorded defs use. o23's test asserts the completed def list (contains the kill-only `o.x` at line 3) and the
label (`Killed`), with an ordinary-member control (`o.x = 2`: covered, no addition) and a wrapper control (`(o).x = 2`).

### 5.3 Table B: every rule that keeps `exact` despite a write, or that could

| # | rule | kept / removed | argument from P, or recovery |
|---|---|---|---|
| E1 | kill-only defs are excluded from `collapsed_groups` (round-1 lesson 1; keeps `for (let i = 0; ..; i++)` header -> body edges `exact`) | **kept** | collapse exists because two *recorded* defs on one node cannot be ordered and both have edges. A kill-only def has no edge, so the only question is whether it overwrites the recorded def inside the node: that is D3's def side (E3), which asks exactly P(iii) for D's node. Row c03 (`exact`), d04 def side (refused). |
| E2 | D3 use side: the write's own target reference, and the single-site operand | **kept, narrowed** | the target reference of `a = v` is not a read, so it is not a U. The operand read of an `RW`/update/`delete` is a read; it keeps `exact` only when that construct is the **only** counting write in N (then the read happens before the only write, loop or not: `k@2->k@3` in `while (--k)`, c13). With any second counting write in N the ordinary rule applies. |
| E3 | D3 order proof | **kept, narrowed (fold item 1)** | §7.1: an order is proven only when neither position's node chain holds a `loop`, `unknown` or `deferred` kind, every kind on both chains is `seq`, and byte order holds. The witness is then a plain source-order execution of a non-repeated node, which a reader checks from the `seq` rows alone. Rows d01-d03, o08, o11, o13, o14. |
| E4 | `for` header model | **removed** (recovery #670) | the two-pass table was withdrawn (astra W2, Fable F1); the for-initializer exception that replaced it was withdrawn after astra R4-W1: it required nothing of the counting writes *outside* the inner `for` that share its node (`do { for (a = 10; false;) .. } while ((a++, false))`, a20). A `for` header is a `loop` chain like any other: a header def with any other counting write in its node is refused. Rows c03, d05 `i@3->i@4`, a18 (now costs), a19, a20, fb01, a04, o35-o38 (refused). |
| E5 | D2-c creation exception (`!path_exists(node(R.start), node(U))`) | **removed** (recovery #667) | sol W3-R2: a hoisted declaration exists at scope entry, the line mapping can name the wrong node, and `path_exists` ignores flagged edges. Without the exception D2-c needs no notion of creation, mapping or reachability: every edge of the written binding with its use outside R is refused. Rows b03 (now a cost), h01-h03. §6.3 specifies the anchors #667 would need. |
| E6 | D2-c inner-binding exclusion (b02, b05, h10, a12) | **kept, narrowed (fold item 4)** | a write targets the innermost declaration of its name whose scope contains the site; when that declaration is inside R the write is to a binding no edge of O names (`search_refs` is binding-aware, t07-t09). Only a *proven* inner binding excludes (§6.2): for a write in a parameter region, only a parameter of that same list (astra W4: the parameter environment precedes the body environment, so a body `var`/function never binds it, a10/a11); a strict block function declaration scopes its block only (a14); an unproven case is outer and refuses (h09). |
| E7 | dead formals (§6.1) | refusal, kept | a formal named like a body-top-level function declaration, or written in the own parameter list after its binding, never reaches the first statement: every edge from it becomes `NameOnly(Killed)`. A refusal needs no proof. |
| E8 | strictness (D4): `arguments` and Annex B refusals apply only to sloppy code | **kept** (owner D4) | assuming sloppy when strictness is invisible refuses more, never less; the helpers (`js_ts_callable_is_strict`, `js_ts_mapped_arguments_possible`) see prologues, enclosing prologues, modules and class bodies, and treat unknown as sloppy. Rows k01-k05, c05, c06, t25. |
| E9 | Annex B refusal bounded to sloppy block-level function declarations named like an owner def | refusal, kept | strict block functions are block-scoped (t07 control emits no edge). |
| E10 | `is_capture` (B8): a use inside a region with its def outside is refused; a def inside *any* region is not | **kept, with a belt** | the hole it leaves (h05: def in R1, use in R2 passes `is_capture` and is `exact` on main) is closed by D2-c, which refuses every edge of a binding written in R1 whose use is outside R1. As a belt S1a also tightens `is_capture` to "the def lies in the use's innermost region". |
| E11 | eager class members: static field initializers, static blocks, computed keys, decorators, `extends` expressions, enum initializers are census sites at the class's or enum's node **when that node exists** | **kept, reworded (Fable S2)** | they run during ClassDefinitionEvaluation / enum evaluation. A class or enum *declaration* line is not a CFG node, so such a site is unmapped and refuses its base (E18; o18-o20, o24, fb15: `cfg_incomplete`); inside a statement span (`const C = class {..}`, f02) it kills at that node. For order proofs they sit under `unknown` kinds (`class_body`, `decorator`) and never prove. Instance field initializers are regions (f01, f03, h11). |
| E12 | one-node distinct-bases belt | refusal, kept (D6) | rows d06, d07 (recovery #670). |
| E13 | formals are defs at the entry node | **kept** | parameters are bound before the first statement; own-parameter-list writes are E7. |
| E14 | `with`: body writes are kill-only may-writes, body reads are refused | refusal, kept | row o39, c07, t11. |
| E15 | covered check (A7) | **kept, narrowed (astra W3)** | covered means the stored `DefSite.path` RD keys its kill buckets by equals `ident(site)` at the same target start byte; a wrong "not covered" adds a harmless duplicate, a wrong "covered" is impossible because what is compared is the consumer's own key. |
| E16 | kill-only defs are added only for bases the owner defines | **kept** | an edge classified in O has a candidate def in O; a write to a base with no def in O affects no candidate (`affects` needs the same base). |
| E17 | inherited `eq` on recorded defs (A1, A3-A5) | unchanged, **disclosed** | the census does not change main's recorded-def relation. Its known imprecisions are listed in §5.4 (G1-G4) with controls that pin main's behaviour; they are not missed writes and are out of #660. |
| E18 | statement-node mapping of a census site | **kept, by construction** | the site's node is the innermost statement span (by byte containment) among the owner's spans whose start line is in `line_index`; a site that maps to no node refuses its base (`site-unmapped`). Nothing is proven from a mapping that failed. Nested-callable sites are mapped only for their kill (D2-c refuses their edges regardless of node). Because `switch_case`/`switch_default` are statement *wrappers*, not spans (`statement_traversal.rs`, `languages/mod.rs:894`), a write in a case value of a multi-line `switch` maps to the `switch` header node and kills every successor: an over-kill (later case values may never run) that is the reason the multi-line form is sound (fb03, fb11). |
| E19 | `def_line == use_line -> SameLine` | refusal, kept | the same node never carries an `exact` between its own def and use. |

### 5.4 Inherited identity gaps (disclosed, out of #660, controls pinned)

These are not missed writes: the write is recorded, with an identity the extractor chose. The census neither widens nor
narrows them; the rows keep main's label as `expected`, and an issue is proposed for each.

- **G1 prefix writes (CLOSED by #674: recorded defs kill by `affects`).** `o.x = 1; o = {}; sink(o.x)` keeps `o.x@2->o.x@4` `exact` on main (o29): `kill_paths` is keyed by full
  path, so the recorded `o` def does not kill `o.x`. The census's kill-only writes use `affects` (o30: `o++` kills `o.x`); the
  recorded side is a separate label-only change (proposed issue: prefix kills for recorded defs).
- **G2 subscript versus dot (CLOSED by #674, same change).** `o.x = 1; o["x"] = 2; sink(o.x)` keeps `exact` (o27): the recorded subscript write has identity
  `o.[]`. Kill-only subscript writes treat `[]` as a wildcard (o28); the recorded side is the same proposed issue.
- **G3 wrapper spelling in recorded paths.** `o!.x = 2` is recorded with another spelling and does not kill `o.x` (o23).
  Closed for the census by A7: the site is not covered, a kill-only `o.x` is added.
- **G4 aliasing.** `p = o; p.x = 1; sink(o.x)` is the name-based model's limit (CLAUDE.md design decision 2); unchanged.
- **G5 try-header exception edges (Fable F4; issue #673 filed).** `cfg.rs:660-690` adds `try line -> first finally/catch line`
  from the header, so `try {\n a++;\n} finally {\n sink(a);\n}` keeps `a@1->a@6` `exact` on main (fb08): RD's route 3 -> 6 never
  passes the kill at line 4. P conditions on the CFG's routes (M3); the fix (exception edges from each try-body statement, or a
  flagged header edge) is the CFG's, out of #660.
- **G6 two more recorded-def identities (Fable S6).** (a) Annex B.3.5: `try {} catch (a) { var a = 2; } sink(a)` writes the
  catch parameter, while the recorded def is attributed to the hoisted `var`; (b) a recorded write to an immutable inner name
  (`(function g(a) { g = 2; sink(g) })`, class-expression names) writes nothing in sloppy mode. Both are recorded defs with the
  extractor's identity, like #668; in the G1/G2 filing (§17).

- **G7 loop body reaches code after the loop without the header (#693; S1b-1 round 1, Opus W2).** The CFG joins a JS loop
  body to the statement after the loop without passing the header, so a header write (`for (..; ..; a++)`, a `while`
  condition's `--k`) never kills a def made in the body on that route, on main as well as with the census. This is M3
  (the CFG's routes are the routes), not a missed write; the rows keep main's label and the fix is the CFG's (#693).
- **Availability is decided on recorded defs only (S1b-1 round 1; narrows 8 in favour of 10).** The first S1b-1 draft let
  the completed def list flip the one-node distinct-bases belt and the 2,048 def cap. That changes which owners emit
  fallback edges and the cap counters (`dfg_rd_functions_without_cfg` +2 on TypeScript `src`), which 10 forbids for a
  label-only slice. The census never changes availability, cap exemption or any counter: the belt and the cap run on the
  recorded list as on main, and when only the completed list would trip either, RD runs on the recorded defs and every edge
  of the owner is `NameOnly(CfgIncomplete)`, traced `owner-refuse:one-node` or `owner-refuse:def-cap`. The census-only
  availability flip of 8 and its parity test are withdrawn; preflight is unchanged from main.

## 6. Writes not spelled as a target

| item | model | label effect; corpus cost (excalidraw / express; heuristic) |
|---|---|---|
| 6.1 sloppy mapped `arguments` | Name refusal for the owner's formals when `js_ts_mapped_arguments_possible` holds (sloppy, simple parameters, `arguments` reached in the owner or nested arrows). Strictness is `js_ts_callable_is_strict`. Unknown counts as sloppy (TS `alwaysStrict` is invisible). Alias escape (`const args = arguments`) is covered because the refusal is name-wide. | 0 / 0. Rows k01-k05, m19, m20, c05, c06, t25, s05, s06 |
| 6.1b direct eval | Owner refusal via `js_ts_direct_eval_anywhere` (parenthesized, wrapped, commented and escaped callees; nested and default occurrences included). | 0 / 0 |
| 6.1c `with` | Writes in the body become kill-only may-writes; uses in the body are refused. | 0 / 0 |
| 6.1d dead formals | A formal named like a `function_declaration`/`generator_function_declaration` reachable from the body's statement list through `labeled_statement` only (any depth: `l: function a() {}` is hoisted at entry, Fable F3, fb05), or written in the own parameter list after it is bound. Its edges become `NameOnly(Killed{line})`, strict or sloppy (a15). A kill-only entry def cannot do this (round 1, §6 lesson 2). | 0 / 0 |
| 6.1e Annex B (sloppy) | Name refusal for a `function_declaration` (generators too, an over-refusal) in any lower position (inside a `statement_block`, a `switch_case`/`switch_default`, an `if`/`else` clause without a block, B.3.4, or a `labeled_statement` inside those), outside nested callables, named like an owner def. Row fb06 is the cost (right label refused). | 0 / 0 |
| 6.1f deferred reads (B8, S1) | `capture.rs` regions = nested callable nodes (parameter lists included) and instance field initializer values (`field_definition`/`public_field_definition` without `static`). Static initializers, static blocks and computed keys are eager. Reads in a region are `NameOnly(CfgIncomplete)` unless the def is in the same innermost region. | parameter defaults <=1 / 0; field initializers 0 / 0. Rows m32, s11, f01-f03 |
| 6.2 captured writes (D2-c, option A) | below | 411 / 16 (heuristic; §11) |
| runtime mutation (getters/setters, Proxy, `globalThis`, `Reflect`) | out of model (CLAUDE.md runtime-mutation rule) | — |

### 6.2 Captured-write refusal (D2-c, no creation exception; recovery #667)

Regions R of owner O: every nested callable node (the six `deferred` kinds of `order.tsv`, parameter lists included) and every
instance field initializer value of a class in O. For each write site s in R whose `ident` is `Path(p)` with base x:

1. **Inner binding.** s is ignored only when a declaration of x inside R *provably* scopes s:
   - s in a **parameter list** (a default value or pattern of a nested callable's parameters): only a parameter of that same
     list (astra W4: parameters are instantiated in their own environment before the body's `var`/function declarations
     exist; a body declaration never binds a parameter-default write: a10, a11; a12 is the control);
   - s in a **body**: a `var`, a function declaration directly in the body statement list (labelled or not), a parameter or
     the named-function-expression name of the nearest callable enclosing s (within R, R itself included); or a
     `let`/`const`/`class`/`using`/catch parameter whose nearest enclosing block, `for` head, `catch` clause or `switch` body
     contains s; a **block-level function declaration scopes its block only**, in both modes (in sloppy mode the outer write
     goes to the hoisted var binding, which is inside R, so treating it as outer only refuses more: a14).
   Declaration kinds come from the `DECL:*` rows of `table.tsv`, so the list is the certificate's. Any other declaration (a
   sibling block's, h09) proves nothing and s counts as outer.
2. **Binding B (amended, #704 round 1).** The write targets the **innermost declaration of x whose scope contains the site,
   by scope alone, with no TDZ visibility**: a closure runs later, so it writes the `let`/`const`/`class`/block function of its
   scope even when that is declared after the closure (`{ const k = () => { a++ }; let a = 1; .. }` writes the block's `a`, not
   an outer one). Only the innermost declaration counts, never a union (a write inside `{ let a; h(() => a++) }` does not reach
   the enclosing callable's `a`). A parameter's scope is its list and the callable's body **minus its parameters' decorators and
   anything nested in them** (they run in the enclosing scope). When that declaration lies inside R (item 1) the write is
   ignored; for a declaration outside R compare it to the candidate def's binding; the census's `DECL:*` scopes name it inside
   regions (nested parameters are not declarations in `BindingFacts`) and `BindingFacts::binding_by_scope_at` outside them.
   One declaration-scope rule serves both: `captured::LEXICAL_CONTAINERS` (statement block, `switch` body, `for` statement,
   `for-in` statement) scopes `let`/`const`/`class`, a function scopes `var` and parameters, `binding_scope_rule` reads that same
   list, and the narrowest scope wins (a table test pins that the two pick the same declaration). A def binds the written
   declaration exactly when it lies in that declaration's scope minus the narrower same-name declarations nested in it (a def
   there binds those). Known gaps, over-refusal only: a `catch` parameter or `class` declaration outside a region is no
   `BindingFacts` declaration (no recorded def); a nested-callable `var` has the owner's whole scope in `BindingFacts` and ties an owner-level
   declaration (the later one wins). Residuals are filed as #736. No declaration found, or `FlatFallback` (an undeclared name), refuses every edge of x.
3. **Refused edges.** Every edge D->U classified in O with U outside R, whose candidate def's binding is B (or is `FlatFallback`
   /unresolved) and with `affects(p, U.path)`. An `exact` or `sameline` edge becomes `NameOnly(CfgIncomplete)`; a `killed` or other
   `NameOnly` label is kept (round 1, accepted). There is no order proof: the read may follow any
   execution of R, and R's creation time is not modelled.

Covered by this one rule: nested-sourced edges into outer uses (m24, m31), the closure-before-def case (b01), reads before
the closure's creation (b03, now a cost), loops (b04), hoisted declarations (h01-h03, w12 by W1), class methods (h06), async
closures across `await` (o17), TSX handlers (o34), TS parameter-property defaults (o25), nested parameter defaults (m31),
instance field initializer writes (h11), and the cross-region case (h05). The creation-point kill (F1 and named owners'
recorded nested writes) is kept: it only adds `Killed` labels. The rule is label-only and lives in `classify_edge` right after
the capture check, so legacy and supplemental edges get the same label.

### 6.3 What #667 would need (recorded, not pursued)

A creation anchor per region kind: a function or generator *declaration* exists from its scope's entry (owner entry when
body-top-level; a block's entry, which is not a CFG node, otherwise -> unknown); a function expression, arrow, class expression
or method exists when the innermost *statement* span containing its first byte executes (byte containment, not line); a class
field initializer from the class's creation. An anchor inside another region is unknown. The proof would be `!may_reach(anchor,
U)` over a reachability that follows flagged edges too (a second `Reachability` instance over the full graph), plus the §7.1
order proof when anchor and U share a node. Heuristic value on main: 68 edges on excalidraw (411 - 343), 1 on express (the
D2cA - D2cB difference in `corpus/*.analyze_r3.txt`).

## 7. Narrow same-node ordering (D3; recovery #670)

### 7.1 The order relation, narrowed to what a reader can check exhaustively (fold item 1, 2)

**What must be proved.** For an edge D->U with D outside U's node N (use side): an execution of N, entered with D live, on which U
is evaluated before any counting write of N. For D inside N and U outside (def side): an execution of N after which D's value
leaves N. Both are P(iii) under M1.

**Positions.** A read is positioned at itself. A write is positioned at its **write construct**: the nearest ancestor of kind
`assignment_expression`, `augmented_assignment_expression`, `update_expression`, `unary_expression` (`delete`) or
`variable_declarator`; its write moment is the construct's end byte (ties: the larger construct writes later; equal constructs
refuse). A pattern element's write is positioned at the assignment or declarator that performs it, after the whole RHS (o11
stays `exact`). A `for_in_statement` head write has no construct of its own: it is under a `loop` kind and never takes part.

**Node chain.** The ancestors of a position up to and including the statement span E18 maps it to (the innermost statement
span containing it whose start line is a node). A statement that started on an earlier line is not in the chain: its control
flow is the CFG's (back edges included).

**The relation.** `precedes(x, y)` is **proven** iff (a) no kind on either node chain is `loop`, `unknown` or `deferred` (every
kind on both chains, their lowest common ancestor included, is `seq`), and (b) byte(x) < moment/byte(y). Anything else is
**unknown**, and unknown refuses. There is no clause table and no pass model.

**Why this is exhaustively checkable.** Condition (a) means the node contains no repetition (no loop starts on this line that
encloses either position), no phase-reordered construct (patterns, switch clauses, class bodies, decorators, parameter lists,
field definitions, `with`) and no deferred code. In such a node ECMAScript evaluates every `seq` kind's children left to right
and a write construct writes after evaluating its children; a conditional (`if`, `?:`, `&&`, `||`, `??`, `?.`) can skip a
position but cannot reorder one. So on any execution of N, when x is evaluated every position textually before it has either
executed or been skipped, and nothing textually after it has executed. That is the whole argument; the `seq` rows are the only
semantic claims it rests on, and each non-trivial one has a fixture (§7.4). The previous rounds' first-evaluation argument is
withdrawn: in a repeated context the first evaluation of a conditional read can be a later iteration (astra W2, a05), and a
body def is followed by the increment and the condition before any exit (Fable F1, fb01, a04).

**The one exception** (a refusal's complement a reader checks from one fact):
- **Single-site operand (E2).** If the counting writes in N are exactly one site W, an `RW`, update or `delete` construct, and U
  is W's own operand read, the use side is proven: W reads its operand before it writes, and no other write of that storage
  exists in N. Loops do not matter (there is nothing else to run first). Rows c13 `k@2->k@3`, o21 `o.x@2->o.x@3`.

A for-initializer exception (D in `for_statement.initializer` with no later initializer or condition write) was folded and then
withdrawn (astra R4-W1): it did not require the `for` to be N's statement root, so a counting write of an enclosing statement on
the same node (`do { for (a = 10; false;) .. } while ((a++, false))`, a20, `exact` on main) escaped it. Header defs with a
same-node increment (c03, d05 `i@3->i@4`, a18) are refused in this lane; #670 may recover them with the structural restrictions
astra named (the `for` is N's retained statement root, no `loop`/`unknown`/`deferred` below it, counting writes only in that
`for`'s own increment).

**Writes that count.** In node N, every census site (recorded or kill-only; a recorded def and the kill-only def at the same
target bytes are one site) whose path `affects` U's path (use side) or D's path (def side) and whose binding is not proven
distinct (A10). A write in a deferred region on line N is D2-c's and is not ordered here.

### 7.2 Rules (label-only, in `classify_edge`)
- **Use side.** For D->U with U in N and D outside N: if N holds a counting write W whose own target reference is not U, the
  single-site exception does not apply and `precedes(U, W)` is not proven, the label is `NameOnly(SameLine)`. Rows m28, m29, d01, d04 (use
  side), d05 `i@2->i@3`, t23, o01-o07, o12, o14, o35-o38, a01-a03, a05, a07, a17, fb02, fb07.
- **Def side.** For D->U with D in N and U outside N: if N holds another counting write W and `precedes(W, D)` is not proven,
  the label is `NameOnly(SameLine)`. Recorded-recorded pairs keep today's collapse; kill-only defs never collapse (E1). Rows d04
  def side, a04, a06, a19, a20, c03, d05 `i@3->i@4`, a18, fb01, fb09.
- **Controls that stay `exact`.** d02, d03, c02, c13 `k@2->k@3`, a08 (the only write in the loop is D), o08, o11, o13.
- **One-node owners.** The distinct-bases belt stays (E12): d06, d07 stay `NameOnly`; recovery #670.

**S1b-2 implementation decisions** (`reaching/d3.rs`, `js_write_census/counting.rs`): D3 refines a finished `exact` label (a
`Killed`, `CfgIncomplete` or collapsed label is never rewritten) and runs on both the production and the frozen-oracle paths.
(1) The DFG keeps one use per path and line, at its first occurrence, so U stands for every read of its path on the line
(S1a's full-path-then-base lookup, `capture_regions::resolve_key`, over the same read collector): all occurrences must be proven,
write-only targets (plain writes, declarators, `delete`) are not reads, and an occurrence of another binding of the name is not
D's use (`relation_at`). A use anchored only at a write's own target (`value = other`, `var [f] = ..`) has no read to order and
keeps its label (only when no read of the path exists on the line); a use with no occurrence at all is `d3-unknown`. Reads are placed with
the write rule for ORDER (`completion::place`, E18 incl. `on_line`): a read in a later statement of the line has no span of its own, so its
position is unknown and the edge refuses (`a = 2; sink(a);`; `sink(a); a = 2;` is refused too, cost). (2) A counting declaration is a
`let`/`const` declarator (`let a;` stores `undefined`), a `var` declarator with a value, or a `for` head's: hoisted functions, parameters, imports and class names are placed by scope (S1d). A write whose
binding is proven distinct from the use's (or D's) is not counted (A10), probed at the declaration's end for a declarator. (3)
`this` rebound by a static block or field value is not the owner's `this` (the kill-only rule). (4) A counting write whose node
the E18 mapping cannot name makes its base `d3-unknown` for every query, beside placed writes too (over-refusal when the read
provably runs first is accepted). The two sides always both run and the strongest reason wins. Recovery with statement spans kept
per statement (siblings on one line ordered properly) fits #670. (5) The trace names the mechanism of each refused edge (`d3-use`, `d3-def`
for a plain byte order, else `d3-loop` or `d3-unknown`; a plain byte order wins over a loop when both occur).

**Node authority (#696).** Which node a position is in is reaching definitions' answer and nobody else's: D3 keys a counting
write by `StatementLookup` of the line RD is actually handed for that write, namely the E18 statement-start line when the
site becomes a kill-only def (`KillOnly.line`) and the target's own line when a recorded def covers it (`Counting.line`),
never the node E18 names; a write on a line RD maps to no node refuses its base like an unplaced one. A read is in the node of
the line the DFG put the use on. E18 (`place`) stays for `site-unmapped`, for the kill-only def's line and for the order inside
a node (`Position`, `precedes`). Sharing the lookup function is not enough: a kill-only def and a recorded def reach it with
different lines, which is why the line is chosen where the def is built (#699 round 1, W1). Known limit (RD's own line-to-node
rule, #700, not D3's): a statement that starts on the last line of a shorter multi-line one has no node of its own, and its
recorded defs and kills sit at the shorter statement's node, before the defs nested in it. Labels that move toward `exact`
against c6393854 (E18 keying over-refused them): in `g(a = 1,\n a = 2); x(); sink(a)`, edge `a@3->a@4`, for example the use is anchored at
a plain write's own target (nothing to order), is the operand read of the node's one counting write (`a += 2`, E2), or is a
read the node's write provably follows (`g(a = 1,\n a = a + 1)`, the read is ordered before the write): each correct. A third
class was KNOWN WRONG and owned by #674 (G1/G2, §5.4, §17; closed by #674): a def-side edge on a field path whose intervening write is a
recorded prefix or subscript write (`g(o.x = 1,\n o = {}); y(); sink(o.x)`, `o.x@3->o.x@5`; also `o += 2`, a declarator tail,
`o[k] = 2`). c6393854 refused it only because E18 put the prefix write in the def's node; keyed where RD puts it, D3 sees both
nodes clean and RD's path-equal kill never kills `o.x`, so the label is `exact` although `o = {}` always runs between them
(the straight-line `o.x = 1; o = {}; sink(o.x)` is already `exact` on main). Pinned by `prefix_write_in_a_shorter_node_is_killed_by_g1` (was `..._is_the_g1_gap`, `exact`; #674 flipped it to `nameonly/killed`).

### 7.3 Expected cost
Heuristic candidate counts (`corpus/*.analyze_r3.txt`, final rules): use side 68 edges on excalidraw (60 of them refused only
by a `loop` or `unknown` chain, `D3u`; the pre-fold relation selected 8), 1 on express; def side 164 / 12 (1 / 0 while the
for-initializer exception stood: its removal costs the `for (let i = 0; ..; i++)` header -> body edges, the `kill:for-increment`
class's sibling, both recoverable under #670). The loop refusal is the new cost class; it is listed in full (not sampled) by the gate on the first slice
that produces it. A second cost class is not D3's but the census's and is named here because it touches common code (Fable S3):
**`kill:for-increment`**, a kill-only increment in a `for` header node that kills an outer def (`let i = 0;` or a formal, then
`for (; i < n; i++) {\n use(i) }`) on every successor, the first iteration's correct `exact` included; it is inside the
prototype's measured -41 / -10, carries its own trace form so the gate counts it, and its recovery (kill on the back edge only)
is #670.

### 7.4 Fixtures S0 must carry (Fable, fold item 7)
One fixture per `seq` row with a non-trivial argument (`try_statement`, `catch_clause`, `finally_clause`, `if_statement`,
`else_clause`, `ternary_expression`, `binary_expression` with `&&`/`||`/`??`, `member_expression` with `optional_chain`,
`labeled_statement`, `sequence_expression`, `call_expression` (callee before arguments), `new_expression`, `template_string`,
`jsx_element` attributes then children, `object`/`array` literals, `assignment_expression` with a pattern left and a read in
the RHS, `variable_declaration` with two declarators), one per `unknown`/`loop` row that can hold a same-node pair (the six
pattern kinds, the three `switch` clause kinds, `class_body`, `decorator`, `with_statement`, the four loop kinds), and one per
exception variant (single-site operand with and without a second write), plus the withdrawn-exception refusals (a for header
def with an increment, a condition write, a body on the line, and a20). The certificate cannot validate a semantic row; these
fixtures are what does.

## 8. One mechanism for every owner; parity (unchanged in shape)

- `src/ast/js_write_census.rs` (S0): the table-driven walk, `ident` and the two relations (§4.4), the escape and parse
  preconditions, strictness (delegated), eval (delegated), `with` regions, deferred regions, body-level and block declarations,
  the order chains of §7.1. It embeds the TSVs and signatures. Submodule `certificate_tests` holds checks 0-5 and the ten
  negative fixtures (§18). Aim: under 600 weighted lines, tests separate.
- `src/cpg/reaching/census_defs.rs` (S1b-1): `complete(parsed, owner, deduped_defs) -> (Vec<DefSite>, CensusFacts)`. It appends
  kill-only defs for uncovered sites (A7, stored-path coverage) whose base the owner defines (E16), mapped by E18. Called at **both** deduplication
  seams (`reaching.rs:249`, `preflight.rs:15`). `CensusFacts` carries the owner refusal, refused bases, dead formals, `with`
  ranges, region writes with their binding (S1c), the positioned writes of §7.1, and `n_recorded`.
- `classify_edge`, right after the capture check (reaching.rs:613), applies in order: owner refusal -> refused base (W1 is owner-
  level now; 6.1, 6.1e, E18) -> `with`-body use -> captured write (D2-c, S1c) -> dead formal -> D3 use and def side (S1b-2). Each
  is a label.
- Kill-only defs enter `kill_sets` with the `affects` relation (A2); recorded defs keep `eq`.
- `capture.rs` (S1a): parameter lists and instance field initializers become regions; `is_capture` requires the def in the
  use's innermost region (E10 belt).
- `js_single.rs` (S1d): `first_hazard` is deleted. `single_node_cfg_is_complete` = JS family && one statement line && distinct
  bases over the **completed** defs.

**Parity tests** (S1b-1, S1d), unchanged: availability agreement on every matrix fixture; a census-only availability flip
(`(o) => (o.x = 1, delete o.x, use(o))`) charged to the fallback cap identically; label-only refusals never move availability
(same owner with and without each trigger: an escape, `with`, sloppy `arguments`, a captured write, a same-node write); the
completed def lists at the two seams are equal.

## 9. Slices (each label-only, one contract each, two review rounds each; fold item 10)

| slice | contract | depends on | expected label moves (excalidraw / express; heuristic) | cache |
|---|---|---|---|---|
| **S0** certificate admission | Census module (walk, `ident`, relations, node chains; unit-tested against matrix site lists), TSVs incl. `order.tsv`, signatures, checks 0-5, ten negative fixtures, the §7.4 semantic fixtures. No production caller. | — | 0 | no |
| **S1a** deferred reads | `capture.rs`: nested parameter lists and instance field initializers; innermost-region belt. | — | <=1 / 0 | bump |
| **S1b-1** identity and coverage wiring | `census_defs.rs` at both seams, kill-only defs with `affects`, stored-path coverage (A7), E18 mapping and `site-unmapped`, collapse exclusion, owner refusals (escape, parse, `Refuse` kinds). `first_hazard` kept. Expectation: the prototype's -41 / -10 plus the unmapped eager sites. | S0 | about -41 / -10 | bump |
| **S1b-2** order relation and D3 | `precedes` over `order.tsv` node chains, the single-site exception, D3 use and def side. | S1b-1 | 68 + 164 / 1 + 12 | bump |
| **S1c** scope exclusions | D2-c option A: regions, the inner-binding rule of §6.2, binding B, `affects`. | S1b-1 | 411 / 16 (245 of excalidraw's are in one generated emscripten file by the round-2 count; the trace re-counts) | bump |
| **S1d** refusal classes + guard deletion | Strictness-aware `arguments`, eval, `with`, dead formals and Annex B by syntactic position; delete `first_hazard`; one-node rows s01-s15, w03, w04, d06, d07; promotion audit. | S1a, S1b-2, S1c | 0 / 0 refusals on these corpora; promotions only in one-node owners `first_hazard` refused (TypeScript upper bound 17 owners) | bump |

Filed, not in this lane: #667 (creation anchors, §6.3), #670 (D3 recovery: loops, the for-initializer labels c03/d05/a18, patterns, switch clauses,
one-node distinct bases, `kill:for-increment`), #669 (promotion to emitted defs), #673 (G5), and the G1/G2(+G6) issue of §17. Sizes: S0 about
450 lines of walker, identity and chains plus about 300 of certificate tests (TSVs and signatures are data); S1b-1 about 200
plus tests; S1b-2 about 150 plus tests; S1c about 150 plus tests; S1d deletes about 90 and adds about 120. No file over 1,000
weighted lines grows by more than about 15 lines.

## 10. Gate per slice (before merge; unchanged in shape)

Cloud, the #599 recipe; base = merge-base, head = PR head, same environment; excalidraw @0642e72, express @18e5985, TypeScript
`src` at the corpora-v1 pin; **all three corpora** before merge. `compare_edges.py` exits 0 on all three; `over_cap`,
`fallback_functions_capped`, `fallback_edges_capped` unchanged; no conflict line on stderr. `PRISM_RD_CENSUS_TRACE=1` records
for every edge whose label a census mechanism decided: `kill:<form>` (with `kill:for-increment` as its own form, §7.3),
`owner-refuse:escape|parse|target`, `site-unmapped`, `d3-use`, `d3-def`, `d3-loop` (refused by a `loop` chain), `d3-unknown`
(refused by an `unknown` chain), `d2c`, `capture-param`, `capture-field`, `capture-region`, `eval`, `arguments`, `with`,
`dead-formal`, `annexb`. **Every moved edge must carry a mechanism.** Promotions (S1d only) are all listed and hand-checked;
demotions sampled at least 5 per mechanism per corpus, including the accepted over-kill classes (c04 one-line `if`; one-node
distinct bases; b03 reads before creation; o38 nested `for`); every `d3-loop`, `d3-unknown` and `site-unmapped` demotion is
listed in full, not sampled, on the first slice that produces it (Fable). Counters are expectations explained by
the trace. Wall and peak flagged over +3%. Each slice's matrix rows fail on base and pass on head. Cache: next free number at
merge (main 145 at the design base; #627 took 146). Mutation lane `mutants/js-def-census.json`: one mutant per non-`R` table
row, per order class, per exception rule, per target action and per refusal predicate, on cloud probes when the owner asks.

## 11. Estimates (heuristic candidate counts; main exact = 57,366 / 6,222)

The parse-only estimator mirrors the extractor and approximates scopes, node mapping and binding identity in Python
(`census/runtime_census_r3.py`, `census/analyze_r3.py`); its numbers select candidates, they bound nothing (sol S2). The
production trace of §10 is the measurement.

| rule | excalidraw | express | source |
|---|---|---|---|
| kill-only S1b subset (measured, prototype) | -41 | -10 | `corpus/protob_compare.txt` |
| D3 use side / def side (final relation: loops and unknown chains refuse; one exception) | 68 / 164 (pre-fold 8 / 5; 68 / 1 with the withdrawn initializer exception) | 1 / 12 | `corpus/*.analyze_r3.txt` |
| D2-c option A (no creation exception, `affects` relation) | 411 (54 owners) | 16 (7 owners) | same |
| of which #667 would recover (option B anchors) | 68 | 1 | same (`D2cA - D2cB`) |
| W1 owner-wide escape refusal | 0 | 0 | same (`W1o`) |
| S1 instance-field reads / nested parameter defaults | 0 / <=1 | 0 / 0 | same, `corpus/analyze.txt` |
| refusal classes (strict-aware) | 0 | 0 | `corpus/analyze.txt` |
| D6 over-kill (accepted) | 3 (one-node `delete`) + one-line `if` class | 0 | round 1 |

Union of the final rules: 641 / 29 (fold 480 / 17; pre-fold 424 / 16; round 2 566 / 15: the D2-c count fell because `affects` no
longer refuses a bare `o` read after a nested `o.x` write, and rose by the reads before creation, b03 class). The estimator
approximates the loop rule as "a loop that starts on the position's line encloses it" and the inner-binding rule as "any
declaration in the region" (heuristic in both directions, sol S2). The TypeScript corpus is not measured locally; the cloud gate
covers it.

## 12. Claims: verified, unverified, falsifiers

| claim | status | falsified by |
|---|---|---|
| C1 the certificate fails on any schema change, target-role loss, and any kind without an order class | verified: baseline exit 0; n1-n8 exit 1 with the named diagnostics (`census/negative_controls.txt`) | a schema change leaving all three signatures identical (impossible by definition), or a role/order-loss mutation the checks miss |
| C1b semantic rows (roles, `order.tsv` classes) are right | human review; §7.1 argues each non-trivial `seq` row; fixtures o01-o39 | a fixture that writes or reorders through such a row |
| C2 kill-only defs are label-only | verified on the prototype (95 fixtures, 2 corpora) | any `compare_edges` diff, cap/fallback move or conflict line |
| C3 W1 closes escaped-identity loss | argued (§4.4: an escape-free owner has raw = decoded; an owner with an escape is refused) and probed: w01-w13 on main show the defects, the estimator selects every escaped owner | an owner with no `\` in any NAME token in which two spellings denote one name |
| C4 D2-c (A) closes creation timing | argued (no exception, nothing to prove); rows h01-h11, b01-b05 | an outer use of a binding written in a region that stays `exact` |
| C5 the order relation never proves a false order | argued (§7.1: no loop, no unknown, `seq` chains, byte order; one one-fact exception); astra R4 audited all 61 `seq` rows; rows o01-o39, a01-a08, a17-a20, fb01, fb02, fb07, fb09 | a `seq` kind whose children are not evaluated in source order, a write construct that writes before its end, or a repeated/phase-reordered context not classed `loop`/`unknown` |
| C6 census + D3 + refusals + capture subsume `first_hazard` | **unverified** until S1d; the test is every one-node row | any one-node row back at `exact` |
| C7 corpus costs | heuristic candidates | the cloud trace counts |
| C8 `var a;` writes nothing but is recorded (WRONG, IMMATERIAL here) | probe c12; issue #668 | — |
| C9 G1-G6 are main's behaviour and unchanged | verified: o23, o27, o29, fb08 on main | a census change that moves them without a trace mechanism |
| C10 the inner-binding rule never excludes an outer write | argued (§6.2: parameter environment, block scope of block functions); rows a10-a14, b02, b05, h09, h10 | a declaration inside R that the rule accepts as inner while the write reaches a binding of O |

## 13. Self-audit: attempts to break the round-3 contracts

Each attempt is a probe run on main (`probes/cases5.txt`, labels in `variant-matrix.tsv`) and a row with its expected label.

| attempt | outcome | rows |
|---|---|---|
| escapes in a member component, an intermediate component, braced form, another object, a read, a hoisted declaration name, a label | all `exact` on main except the read (no edge); all owner-refused under W1; w10/w13 are the owner-wide cost | w07-w13 |
| hoisted function/generator writing a formal after the read, on a shared line and on its own | `exact` on main; D2-c (A) refuses | h01-h03 |
| def in one nested callable, use in another (`is_capture` hole) | `exact` on main; D2-c refuses | h05 |
| class method writing a captured binding | `exact` on main; kill (S1b) + D2-c | h06 |
| declaration in an unrelated inner block of the region; `var` in an inner block | main kills through the recorded nested def; the scope rule treats the first as outer and the second as inner | h09, h10 |
| instance field initializer writing a formal | `a@3->a@4` `exact` on main although the initializer never runs; D2-c refuses | h11 |
| pattern defaults (array, object), computed key, for-of default, for-of body read after an iterable write | `exact` on main; unknown-order positions refuse | o01-o05 |
| switch case values, do-while | both refused after the fold (`switch` clauses are `unknown`, loops are `loop`); the labels were right here, so these are cost rows | o06, o07 |
| RHS read before a pattern write; read and write both in the RHS | `exact` and right | o08, o11 |
| logical assignment then read; try/finally both orders; labelled block; generator; async closure across `await` | refused or killed as expected; o13 is a control | o12-o17 |
| class static block, static field, decorator, enum initializer (eager) | `exact` on main; kill-only at the class node | o18-o20, o24 |
| `delete o?.x`, `(a as any)++`, `o!.x = 2`, parameter property default, `using`, `accessor` fields | kill, kill, kill via A7 (G3), D2-c; `using r = ..` is an `ERROR` node in tree-sitter-javascript 0.23.1 and an assignment-form `using` token in the TS/TSX grammars: the owner is refused (parse precondition, or the `using` token refusal); `accessor` fields parse and are handled by the instance/static split | o21-o26 |
| subscript write vs dot read, prefix write (recorded) | `exact` on main: inherited G1/G2, unchanged, issues proposed | o27, o29 |
| subscript update, prefix update (kill-only) | `affects` kills | o28, o30 |
| TSX handler write | `exact` on main; D2-c | o34 |
| for header: body read before increment, condition write, increment read, nested for on one line | all refused: a `for` header is a `loop` chain (o35, o37 were kept by the withdrawn two-pass model) | o35-o38 |
| read inside a `with` body on the write's line | refused | o39 |
| **fold:** astra's switch default-before-case and default-between-cases, fall-through; the body-def-then-condition `for`; the conditional first evaluation in a do-while; `break` after another write; `continue` before the use; a zero-iteration single-write loop; the parameter-default write with a body `var`/function; the true inner-parameter and body-local controls; a strict block function; a labelled declaration in strict code; the one-line `while` read-then-write; the for-initializer with a condition write | every falsified proof now refuses (loop/unknown chains); the remaining exception holds on its controls (a08, c13), the scope rule on a12 | a01-a19 |
| **R4:** an initializer def inside a `do`-while whose condition writes the same name on the node | `exact` on main; the for-initializer exception would have kept it; removed | a20 |
| **Fable's probes** fb01-fb15 (imported) | F1/F2 refused by the loop/unknown classes, F3 by the positional hoisting rule, F4 disclosed as G5, S2 labels corrected, S4 rows added | fb01-fb15 |
| tried and found nothing to add (no row): `new.target`, `import.meta`, `globalThis.a`, tagged templates, JSX attribute/children order, short-circuit writes, `yield`, comma in `for` initializers, labels named like variables (`statement_identifier` is not a binding), `declare`, overloads, `satisfies` targets, strict `arguments` in classes/modules (k02, t25) | all covered by `seq`, by a refusal, or by not being a binding | — |

## 14. Variant matrix

`variant-matrix.tsv` has 181 rows (round 2: 103; round 3: 152), the main label measured at 105599ee, the prototype label where
the round-1 prototype ran, the expected label and the slice. Round 3 added w07-w13, h01-h11, o01-o39 and changed b03 to a cost
row; the fold adds a01-a19 (`probes/cases6.txt`) and fb01-fb15 (the Fable reviewer's probes, copied from `fable-r3-probes/`),
and changes o06, o07, o18-o20, o24, o35, o37 (§0.1); the R4 fixes add a20 (`probes/cases7.txt`) and change c03, d05
`i@3->i@4`, a18 to cost rows (181 rows). Rows marked `inherited` pin main's behaviour.

## 15. Reproduction

- Commit main 105599ee; binary `target/release/prism` built in the worktree on 2026-11-04 (verified against the stored m01
  probe: identical output, `probes/cases/m01_update_post/verify.jsonl`).
- Certificate: `python3 -B census/check.py`; negative controls: the commands at the head of `census/negative_controls.txt`
  plus `--order census/negative/n7_order_row_missing.tsv` and `n8_order_bad_class.tsv`.
- Census: `uv run --no-project --with tree-sitter==0.23.2 --with tree-sitter-javascript==0.23.1 --with
  tree-sitter-typescript==0.23.2 python census/runtime_census_r3.py <repo> corpus/<c>.r3.census.jsonl`; edges from
  `prism nav --no-cache dfg-stats --repo <repo> --edges`; then `python3 census/analyze_r3.py` (`census/run_census_r3.sh`).
  Raw edge files are removed after the run; census rows are gzipped (`corpus/*.r3.census.jsonl.gz`).
- Probes: `probes/run_r3.sh` (verify + `cases5.txt` + corpus edges), `probes/run_fold.sh` (`cases6.txt`), `cases7.txt` through
  `probes/run_tag.sh`; `python3 matrix.py`.
- Fold estimates: `census/run_census_r3.sh` then `census/run_analyze_fold.sh` (regenerates edges, analyzes, removes raw files).

## 16. Decisions adopted (fold item 11; both reviewers agree)

1. **W1:** owner-wide refusal on any escaped NAME token, with the parse refusal; TypeScript cost measured by the gate.
2. **G1/G2 (+G6):** one label-only issue, out of #660; text in §17. P is qualified by M1-M3 and the G rows (§5.1).
3. **`super.x = v`** is a may-write to `this.x` (`NoWrite` would be unsound: `[[Set]]` with `this` as receiver).
4. **Grammar-unsupported syntax** (`using`, `accessor`): ERROR/MISSING refuses the owner; a grammar bump fails the signature
   and the order check and re-opens classification; newly parseable syntax is never admitted silently.
   *Round-1 fold:* the TS/TSX grammars (0.23.2) do parse assignment-form `using r = ..` (an anonymous `using` token on an
   `assignment_expression`; the JS grammar 0.23.1 still gives ERROR). That binding is not modelled, so the walker refuses the
   owner when it finds that token (`owner-refuse:target:using`, keyword position only). o26 is therefore `cfg_incomplete`
   under every grammar. `accessor` fields parse and are handled by the instance/static split.

## 17. Issue text for the controller: inherited recorded-def identity gaps (label-only, out of #660)

**Title:** JS/TS reaching definitions: recorded-def identity gaps keep wrong `exact` labels (prefix writes, subscript/dot,
Annex B.3.5 catch parameter, immutable inner names)

**Body.** Four inherited cases where a *recorded* write has an identity under which RD's kill buckets (`def_buckets.rs:26,86`,
keyed by the stored `DefSite.path`) do not kill the def a later read is labelled `exact` from. None is a missed write; #660's
census leaves them unchanged and pins main's labels (rows o27, o29 in `variant-matrix.tsv`; G6 rows to add):

1. **Prefix writes (G1).** `o.x = 1; o = {}; sink(o.x)` keeps `o.x@2->o.x@4` `exact`: the recorded `o` def does not kill
   `o.x`. Fix: a recorded write to `p` kills every def whose path `p` is a prefix of (the `affects` relation #660 uses for
   kill-only defs).
2. **Subscript versus dot (G2).** `o.x = 1; o["x"] = 2; sink(o.x)` keeps `exact`: the subscript write has identity `o.[]`.
   Fix: `[]` is a wildcard component in the kill relation (both directions).
3. **Annex B.3.5 catch parameter (G6a).** `try {} catch (a) { var a = 2; } sink(a)`: the initializer writes the catch
   parameter, while the recorded def is attributed to the hoisted function-level `var a`. Fix: attribute the def to the catch
   binding (or refuse the name in the owner).
4. **Immutable inner names (G6b).** `(function g(a) { g = 2; sink(g) })`, and class-expression names: the recorded write does
   nothing in sloppy mode (throws in strict). Fix: a write to a named-function-expression or class-expression name is `NoWrite`.

All four are label-only (kill relation and def attribution; no emission change); gate as #660's slices (edge sets identical on
the three corpora, per-edge mechanism trace). Related: #668 (`var a;` recorded as a def), #673 (try-header exception edges).

## 18. S0 implementation contract (self-contained; changes no behaviour)

S0 delivers the census's data, its certificate and its semantic fixtures inside the repository, with **no production caller**:
no label, edge, counter, cache format or CLI output changes; the full test suite and the three-corpus edge sets are identical
to base by construction. An implementer needs this section, `census/` and `variant-matrix.tsv`, nothing from the review history.

### 18.1 Files that move into the repository
| from (`~/prism-evidence/jsts-lane/660/design/census/`) | to | role |
|---|---|---|
| `table.tsv` (142 rows: kind, slot, role, order hint, reason) | `src/ast/js_write_census/data/table.tsv` | write-relevant slot roles |
| `leaves.tsv` (33 rows: kind, bind, name_token, reason) | `src/ast/js_write_census/data/leaves.tsv` | leaf census; the 8 NAME kinds drive the escape precondition |
| `targets.tsv` (78 rows: kind, action, carrying_slots, reason) | `src/ast/js_write_census/data/targets.tsv` | target-position actions |
| `order.tsv` (186 rows: kind, order, reason; classes `seq`, `loop`, `unknown`, `deferred`, `none`, `leaf`) | `src/ast/js_write_census/data/order.tsv` | evaluation-order classes |
| `signatures/{javascript,typescript,tsx}.sig` (772 / 1,297 / 1,351 lines) | `src/ast/js_write_census/data/signatures/<lang>.sig` | the reviewed schema signatures |
| `negative/n1..n5/javascript.json` (mutated `node-types.json`), `negative/n6_table_role_loss.tsv`, `negative/n7..n10_*.tsv` | `tests/fixtures/js_write_census/negative/` | the ten negative controls |
| `check.py`, `grammar.py` | stay in the evidence tree (reference implementation; the Rust test is authoritative once it passes the same inputs) | — |

The data files are embedded with `include_str!` from `src/ast/js_write_census/data/` so the certificate test needs no file
system. `src/ast/js_write_census.rs` (the module; aim under 600 weighted lines) holds: the TSV loaders with duplicate rejection
(18.2 check 0), the schema reader (its own serde structs over
`Language::node_types_json()`, `src/languages/mod.rs:78`: `NodeTypeSchema`, `src/cpg/reaching/scope.rs:17-38`, is private and lacks the `named` flag; omitted `multiple`/`required`/`types`/`named` is rejected at deserialization, stricter than `check.py`, which would print `None`), the table-driven walk producing census sites (path by `ident` §4.4,
target start byte, write construct and moment §7.1, region flags, node chain), and `precedes` (§7.1). Everything is
`pub(crate)` and `#[allow(dead_code)]` until S1b-1 calls it. New code only: no file over 1,000 weighted lines grows.

### 18.2 The certificate test (`src/ast/js_write_census/certificate_tests.rs`, `#[cfg(test)]`)
One test per check, each over all three grammars (`Language::JavaScript`, `TypeScript`, `Tsx`), mirroring `check.py`
diagnostic for diagnostic (the Python output is the oracle: `python3 -B -I census/check.py` prints
`PASS: 0 problem(s); slots 142, rows 142, leaves 33, target kinds 78, order rows 186` on the pinned grammars):
- **check 0, duplicates:** every TSV loader rejects a duplicate key (identical or not) before anything is read from the table
  (`DUPLICATE-ROW <file> <key>`); the key is (kind, slot) for `table.tsv`, kind for the other three.
- **check 1, signature:** the computed signature (every `KIND`, `SUPER`, `SLOT` with `multiple`/`required`, and `ALT` line,
  named and anonymous, sorted) equals the embedded `.sig` line set; any added or removed line is `SIGNATURE-ADDED` /
  `SIGNATURE-REMOVED`. Re-recording is a reviewed data change, never automatic.
- **check 2, coverage:** every write-relevant slot (expanded alternatives meeting the NAME leaves, the six pattern kinds,
  `member_expression`/`subscript_expression`, the five WRAP kinds, plus `type_identifier` in `name` slots) has exactly one
  `table.tsv` row with a role in the closed role set and an order hint in `{text, before-target, after-body, deferred}`;
  `UNCLASSIFIED` / `STALE` / `BAD-ROLE` / `BAD-ORDER` otherwise.
- **check 3, leaves:** every named leaf of each grammar has a `leaves.tsv` row (`UNCLASSIFIED-LEAF`), no stale row
  (`STALE-LEAF`), and the BIND set equals {`identifier`, `shorthand_property_identifier_pattern`, `undefined`,
  `type_identifier`} (`BIND-SET-DRIFT`).
- **check 4, target closure:** from every slot whose role is `W`, `RW`, `DEL`, `FORHEAD`, `IMPORTNAME` or `DECL:*`, expand
  alternatives; every kind reached needs a `targets.tsv` action (`TARGET-KIND-UNCLASSIFIED`); for `PATTERN`/`WRAP`/`PARAM`/
  `IMPORTS`/`CONTAINER` kinds each carrying slot must exist (`CARRYING-SLOT-MISSING`) and keep a target role
  (`TARGET-ROLE-LOSS`); stale target rows fail (`STALE-TARGET`).
- **check 5, order:** every named non-supertype kind has exactly one `order.tsv` row (`UNCLASSIFIED-ORDER`, `STALE-ORDER`),
  its class is in the closed set (`BAD-ORDER-CLASS`), and `leaf` rows are exactly the leaves (`ORDER-LEAF-MISMATCH`).
- **negative controls:** one test per fixture, asserting failure *and* the named diagnostic: n1 new composite in a target slot
  (`TARGET-KIND-UNCLASSIFIED future_target`, `UNCLASSIFIED-ORDER future_target`, signature lines), n2 widened target slot
  (signature), n3 widened non-relevant slot (`UNCLASSIFIED labeled_statement label`), n4 renamed carrying slot
  (`CARRYING-SLOT-MISSING`), n5 new wrapper inside a wrapper, n6 table role loss (`TARGET-ROLE-LOSS` in all three languages),
  n7 order row missing, n8 order class invalid, n9 duplicate valid row, n10 duplicate invalid row (`DUPLICATE-ROW`). The JSON
  fixtures replace one language's schema; the TSV fixtures replace one table.

### 18.3 Semantic fixtures (`tests/ast/js_write_census_test.rs`, registered in `tests/ast/main.rs`)
These test the walker and `precedes` as pure functions on small sources (no CPG, no RD, no labels):
- **site lists:** for every `variant-matrix.tsv` fixture whose slice is S1b-1, S1b-2, S1c or S1d, the census sites the walk
  reports (path, line, target start byte, construct kind, region flags, owner-refusal reason) match a committed expectation;
  the one-node `first_hazard` rows (s01-s15) included, so S1d's deletion test already exists;
- **identity:** `ident` on every target shape of `targets.tsv` (NAME leaf, member and subscript chains, each WRAP kind,
  optional chains, `this`/`super` roots, non-reference roots -> `NoWrite`, `REFUSE` kinds -> refusal), and the escape and
  parse preconditions (w01-w13, o26);
- **order, one fixture per `seq` row with a non-trivial argument** (§7.4: `try_statement`, `catch_clause`, `finally_clause`,
  `if_statement`, `else_clause`, `ternary_expression`, short-circuit `binary_expression`, `member_expression` with
  `optional_chain`, `labeled_statement`, `sequence_expression`, `call_expression`, `new_expression`, `template_string`,
  `jsx_element`, `object`/`array`, `assignment_expression` with a pattern left, `variable_declaration` with two
  declarators): each asserts `precedes` for a read-before-write pair (proven) and a write-before-read pair (not proven);
- **order, one fixture per `unknown`/`loop` row that can hold a same-node pair** (the six pattern kinds, the three `switch`
  clause kinds, `class_body`, `decorator`, `with_statement`, `for_statement`, `for_in_statement`, `while_statement`,
  `do_statement`): each asserts that `precedes` is not proven in either direction;
- **the single-site exception:** proven with one `RW`/update/`delete` site whose operand is the read (c13, o21), not proven
  with a second counting write;
- **the withdrawn exceptions:** a `for` header def with an increment (c03), a condition write (a19), a body on the line (fb01),
  and a20, all not proven; the astra and Fable examples a01-a08, fb02, fb07, fb09 not proven.
Each fixture's source is the matrix row's `source` column, so the implementer copies rather than invents.

### 18.4 What S0 must not touch
`src/cpg/reaching.rs`, `src/cpg/reaching/*`, `src/cfg/js_single.rs` (`first_hazard` stays), `src/data_flow/build/*`,
`src/ast.rs`, `src/access_path.rs`, `src/languages/mod.rs` beyond reading `node_types_json()`, the CPG cache version, any
output format, the tier-A baseline, the mutation lanes. No `cfg(test)` seam in production code. The PR's gate is the ordinary
CI run plus `compare_edges.py` identical on the three corpora (trivially, since nothing is called); the mutation lane
`mutants/js-def-census.json` starts with S0's loaders (one mutant per diagnostic) and grows with each later slice.
