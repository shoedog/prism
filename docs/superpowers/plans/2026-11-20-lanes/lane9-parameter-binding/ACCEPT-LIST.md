# E1 argument-follow accept-list, version 2

**Status:** amendment under design review (owner decision of 2026-11-27: go to review as priced). The decision, the prices, the accepted gate miss and the follow-ups are in DESIGN §5 E1 (accept-list v2 amendment).

Amends [DESIGN §5 E](DESIGN.md#e-calls-through-a-parameter) for E1 and pricing before E2. A contribution identifies the callable if its grounded activation reaches the read. Union independently witnessed contributions by occurrence, including for exported or escaping holders. §5 E governs discovery, E2's NameOnly grade, payload and index lifetime.

## Admission

**E1 product = the baseline grounding (including `holder_at`) AND this list.** A derivation that `holder_at` refuses is pricing-only (shadow); E2 decides whether to activate it. The S0 row and the "Ten priced holder refusals" table below describe the shadow judge and are shadow-only. S0 also refuses a receiver-form Source call (`this.m`, `super.m`, or any member call other than a static native ESM namespace member): a resolved `this.m` is an `Exact` singleton of the declaring class and does not see an in-repo subclass override, so no dispatch witness exists in v2 (counter `source`; #972 adds one). Measured cost: zero baseline-credited target sites on Excalidraw and TypeScript (the `this`/`super` hops occur only in derivations the baseline did not credit: 6 on Excalidraw, 71 on TypeScript).

Require S0, value provenance and applicable O checks per hop, path and timing at invocation, and F1–F4 at the terminal. Peel only transparent parentheses and proved erased TS wrappers. Refuse parse recovery and uncertain witnesses.

| Form | Requirement and evaluation order |
| --- | --- |
| S0 · holder | Full-span ordinary `Source` call. Its **complete** resolution has one undropped `Exact` target equal to the holder ID. Join that ID to a unique parsed implementation matching the holder's file and byte span at every hop. Refuse synthetic, derived, pre-resolved and constructor grounds. |
| S1 · supplied | Exact zero-based runtime position in a spread-free argument list. Skip a proved erased TS `this`. Require ordinary positional delivery, including a supported optional call on its taken arm. An adapter with unproved delivery refuses. All actuals finish before entry. |
| S2 · default | Trigger an outer or element default only on a proved omission, intrinsic `undefined`, `void` of a primitive literal, own data proved undefined or an O1 null-prototype miss. Evaluate outer before inner, within the same caller. `null`, opaque input and absent callers supply no trigger. |
| S3/S4 · carrier and inert values | Fresh object or dense array literal directly supplied, created in an admitted initializer or nested in place. Its entries are S4. S4 is a primitive literal, intrinsic `undefined`, `void` of a primitive literal, F1–F4 or S3. Refuse evaluated calls, assignments, branch expansion, sequence and opaque effects in this grammar. |
| S5 · relay | One transfer between distinct lexical owners. The upstream P1 is the downstream P1 slot's **entire** supplied actual at the proved operand occurrence. Apply S0–S4 and O1–O5 at both hops and I2 to the transfer. A formal found in a default, literal entry or value arm supplies no relay. |
| P1 · bare | The peeled callee of `cb(...)` or `cb?.(...)` is the exact runtime formal by owner, declaration bytes and position. Zero selection steps. The callee read precedes payload evaluation. Shadows, aliases, members and JSX tags supply no P1 read. |
| P2/P3 · own key | One or two own data steps, including renames, in the tracked formal's object pattern (P2) or in a single-declarator direct-body `const` that snapshots a P1 carrier before invocation (P3). P3's entire initializer is that carrier. Prove the exact leaf binding and each source key's ECMAScript string from an identifier, string, supported number or bracketed string/number literal. |
| P4 · array index | A P2/P3 **object pattern** can select a present canonical index (0 through 2³²−2) of a dense literal as one own step, bypassing `@@iterator`. Array patterns, rest, holes and spreads refuse. |
| I0 · supplied P1 | Any placement that reads the initialized formal, including nested executions. No property selection or taken S2 may supply the value on this contribution. |
| I1 · paths and defaults | Required for P2/P3 (including P4) and values sourced from a taken S2. In an ordinary synchronous non-generator holder, the invocation is the entire direct statement expression, direct return operand or expression-arrow body. Prefix may contain directives, empties, ordinary hoisted function declarations, single-name `const` with S4 initializers and the P3 snapshot. |
| I2 · transfer | Keep I1's placement and prefix on the downstream call **in the relay body**. Read the exact P1 operand and require S4 for its other actuals. |
| J0/J1 · JSX flag | J0 is ordinary at every end. J1 marks JSX at the invocation or any incoming hop, with the role(s) recorded even on refusal. V2 refuses J1 as `jsx_attribute`. The renderer controls when the component runs and what props/positions it receives. Exact and static attributes cannot prove delivery. Count spreads, `key`/`ref`, children, `defaultProps` and legacy context here. JSX position ≥1 supplies no `Missing` witness. |

Check the full formal list. Require unique runtime names, bare formals and at most the tracked P2 pattern, with one key per level and optional defaults. Skip a default only on proved non-undefined input. Otherwise require S4 even for unrelated formals. Refuse an evaluated read of a formal of this or an enclosing owner anywhere in that initializer, including literal entries, computed keys and arms. Formal/pattern initialization is left to right. Earlier bindings may have values, and self/later dependencies can hit the TDZ. Track defaults supplying the value separately from unrelated inert or skipped ones.

| Condition | Required proof |
| --- | --- |
| O1 · own data | Private literal tree, ≤2 container levels. Static data entries, proved shorthand and ordinary concise data methods. Check every entry. Last data key wins. Refuse getters, setters, holes, spreads, dynamic keys and custom prototypes. Permit colon `__proto__: null`, including quoted spelling. Bracketed `['__proto__']` defines own data. Each read needs a final own descriptor or a null-prototype miss with immediate S2. Ordinary-prototype absence proves no undefined read. |
| O2 · no tracked write | Resolve occurrences throughout entry, body and descendants, including deferred and later text. Refuse assignments (including compound/logical), updates, destructuring, `for in/of` writes, initialized redeclarations, hoisted replacements, uncertain inventories and binding seams. Allow uninitialized `var` only if same binding and write-free. Ignore proved shadows/types. Supplied P1 allows extra proved value reads. The stricter P2/P3, S2 and S5 inventory allows only declarations and proved invocation/operand/snapshot reads. P3 carrier gets its single snapshot use. Refuse carrier alias, mutation, method calls, delete, reflection and escape. |
| O3 · lexical lookup | Prove the scope of the read, relevant source occurrence, carrier, copy and terminal. Refuse relevant `with` and potentially direct intrinsic `eval`, including in defaults and descendants. Keep v1's exemption for `eval` proved an initialized immutable non-callable primitive. |
| O4 · arguments | Resolve implicit objects across defaults and nested captures. Refuse potentially live mapped exposure of a supplied sloppy-simple slot, including alias, reflection and escape. No detachment or disjoint-index exemption. Supplied P1 permits proved unmapped exposure. P2/P3, S2 and S5 retain the blanket holder-owned/captured implicit object refusal. Even unmapped slots can alias a carrier. Ignore proved shadows or separate implicit objects. |
| O5 · provenance | O2/O4 keep the fresh tree private through each own read. I1 keeps the invocation in the original synchronous activation. Only the scalar crosses S5. Preserve this activation's value recipe and the terminal's initialization witness. |

### Terminal callable forms

| Form | Witness |
| --- | --- |
| F1 | Inline function expression, arrow, generator expression or ordinary concise data method at its own definition. Prove a unique representable callable ID and byte span. Async/generator function values can be terminals. |
| F2 | One hoisted function declaration in an ESM module or proved declarative local scope. Require lexical identity, initialization, no writes and static lookup. Classic-script global property bindings and Annex B dual declarations refuse as terminals. |
| F3 | Initialized immutable bare `const` with up to eight immutable lexical links ending at F1, F2 or F4. Prove each initializer/read in its own execution order, without an unproved callable or control-flow seam. |
| F4 | A named value import or static native ESM `ns.member` with a canonical relative module witness to a direct named export of an F2 declaration. Prove the source binding, export name, unique initialized implementation and stability. |

F1–F4 prove terminal values only. Wrapper results, re-export adapters, mutable exports, receiver projections, arbitrary call results and bound functions supply no terminal. A grounded ordinary caller can contribute even if another caller or relay fails.

## Why the new forms work

| Newly admitted form | Soundness |
| --- | --- |
| I0 after prefix or inside a larger expression | Entry initializes the lexical cell once from this caller's supplied value, and prefix and sibling expressions cannot replace it under O2–O4. No key lookup, default dependency or other-caller substitution is involved. |
| I0 branch, switch, loop, repeated reads or other body regions | A branch can skip the read and a back edge can repeat it, but every execution reads the same initialized cell. There is no carrier to mutate before the next read. |
| I0 nested or escaped closure | Each closure captures the cell of its own holder activation. Closure creation and later or repeated calls retain that cell, and O2–O4 bar subsequent rebinding. |
| I0 async/generator, including nested resumptions | The body reads its own initialized cell. Scheduling and suspension preserve it without default evaluation or other-caller substitution. |
| I0 deferred body in an entry initializer | Allow only when the closure's binding has no occurrence in the formal list other than its declaration (O4's blanket refusal covers `arguments`); a read before initialization is a TDZ error and never yields another callable. The eventual read captures the same stable cell. A direct read in the holder's initializer still fails S2/S4. |
| I0 extra proved value reads or provably unmapped arguments | Copying the function value or replacing an unmapped index cannot change the tracked cell. Property effects on that function value cannot replace its admitted callable identity. |
| S0 by Exact, including imports, aliases and locals | `Exact` supplies static callee identity for this occurrence under the project's contract. The matching unique implementation and S1 delivery bind the actuals to this activation. Value, mutation, timing and default proofs still run on the same caller at every hop. |

S0 uses Prism's static-binding model. Runtime replacement of global/module properties and host bindings lies outside `Exact`. Production `ground` already checks the full resolution and adds `holder_at`. The shadow judge adds a stricter `callable` re-witness. Use the resolver result joined to `unique_owner` and implementation bytes. A caller-index hint supplies only a candidate. [Static-binding contract](../../../../../CLAUDE.md#L502) · [Grounding](../../../../../src/resolution/js_ts/arg_follow.rs#L460) · [Shadow judge](../../../../../src/resolution/js_ts/arg_follow/accept.rs#L156)

### P1 failure modes and existing proof coverage

| Unsafe or unproved case | Existing proof and v2 requirement |
| --- | --- |
| Shadow, wrong formal or replacement | `param_slot::parameter` resolves the read and records declaration, owner, position and `replaced`. Require the exact formal even inside a nested scope. Reject duplicate slots and parameter/body seams. |
| Write in a closure, default, loop or after a prior read | Baseline/shadow stability use `replaced` and `js_ts_scoped_written`. Its file-wide index joins formal-list/function scopes for resolved writes. Entry-only `js_ts_formal_written` and `Before` cannot certify I0. |
| Mapped `arguments` or uncertain ownership | Reuse ownership/mapping evidence in `arguments_instability` and the shadow's reference scan. Require whole-scope O4 for I0, including for captured references and later writes. |
| Direct eval, `with` or unknown scope | `index_eval_sites` indexes descendants and `dynamic_for` checks affected scopes. Use shadow `dynamic` and prove occurrence lookup. |
| A default reads an earlier, self or later formal | `default_binding`, `initialized_before` and active defaults handle direct baseline cases. Selected literal leaves can bypass them. Use `default_dependency` across evaluated syntax and preserve S2 provenance. Direct formal-list reads refuse, including TDZ. Deferred reads there need post-initialization proof. |
| Heap, alias or other-caller value | `body_read` and `Selection` distinguish heap reads from P1. `ReadTiming` and `body_path_stable` concern heap reads. `Terminal::Parameter` alone proves no relay or caller correlation. |

[Formal identity](../../../../../src/ast/js_ts/param_slot.rs#L182) · [Write index](../../../../../src/ast/js_binding_writes.rs) · [Baseline defaults](../../../../../src/resolution/js_ts/arg_follow/value.rs#L333) · [Shadow proofs](../../../../../src/resolution/js_ts/arg_follow/accept.rs#L436)

### Ten priced holder refusals

All ten reportedly clear the ordinary Source/Exact singleton check. **Pass** below assumes the target matches the unique holder span. A real collision or mismatch still incurs `holder_identity`. The notes establish no later path, timing, stability, delivery or terminal refusal for these occurrences.

| Example | S0 in v2 and witness removed |
| --- | --- |
| `findLastIndex` | Pass. Other-package import needs no F4. |
| `exportToCanvas` | Pass. Imported holder needs no F4. |
| `withCaretPositionPreservation` | Pass. Imported holder needs no F4. |
| `arrayToObject` | Pass. Imported holder needs no F4. |
| `mapFind` | Pass. Imported holder needs no F4. |
| `_exportToCanvas` | Pass. Match the aliased implementation `exportToCanvas`. |
| `eachDiagnostic` | Pass. Sibling import needs no F4. |
| `codeFixAll` | Pass. Sibling import needs no F4. |
| `getFormattingScanner` | Pass. Imported holder needs no F4. |
| `addNewNodeForMemberSymbol` | Conditional. Same-file. Source audit cannot identify the refused hop. |

The shadow emits S0 after the hop's file/span/owner check and unions form IDs across hops. F1 labels the terminal. A one-hop record with S0 and `holder_identity` is inconsistent with this judge. Across two hops an S0 pass and a shadow callable mismatch can coexist. A baseline-positive pricing event has already passed production's owner/span checks. The supplied summary lacks the row/chain needed to distinguish a second-hop failure from a mismatch between the reported records.

## One decision point and counters

`Context::follow` alone inserts into `result.targets` or admits S5 recursion. Helpers supply evidence/refusals. Keep the exact caller chain, operand and value recipe. Baseline admits `Callable` and recurses on `Terminal::Parameter` without the shadow conjunction. Apply the v2 checks before either action. Limit to two caller lookups, with an active-slot cycle check. Deduplicate validated target IDs before `ArgIndex::record`, which keeps the full 1–8 set and refuses all at 9+. Rebuild/version the index before activation.

| Counter key | First failure |
| --- | --- |
| `source` | Unusable Source syntax, argument AST or delivery protocol, synthetic/pre-resolved ground or production construction. |
| `holder_outcome` / `holder_identity` / `no_caller` | Dropped, non-singleton, non-Exact or wrong-holder result / missing, conflicting or mismatched implementation/span / no indexed callers. `holder_identity` no longer counts a missing F1–F4 holder witness. |
| `jsx_attribute` | J1 at either end, checked before ordinary delivery/position. |
| `read_timing` | I1/I2 placement, owner kind, prefix or I2 sibling failure, or unproved I0 post-initialization execution. I0 body placement itself never incurs it. |
| `entry_write` / `dynamic_binding` / `mapped_arguments` / `stale_path` | Tracked write/seam / dynamic source or holder scope / potential live map exposure / forbidden carrier or other tracked use, including refused unmapped exposure for a carrier. |
| `unknown_spread` / `path_step` | Argument spread / unsupported slot, pattern, key, path depth or own array index. |
| `default_dependency` / `default_form` | Evaluated formal dependency, including TDZ / other unproved initializer. Check dependency before general S4 shape. |
| `hidden_literal_key` / `undefined_lookup` | Unproved literal construction / unproved descriptor, property read or undefined trigger. |
| `opaque_terminal` / `stale_terminal` / `callable_collision` | Unsupported value / unproved terminal binding, scope or initialization / ambiguous callable ID. |
| `relay_form` / `cycle` / `depth` | Unproved S5 operand / repeated active slot / third caller lookup. |
| `fanout` / `no_target` | Site exceeded the deduplicated cap / empty fallback when there is no proof reason. |

No v2 counter renames or additions. V1 adds `read_timing`, `default_dependency`, `default_form` and `relay_form` to baseline. Check in this order: source, holder, J1, O2–O4 (`entry_write`, `dynamic_binding`, `mapped_arguments`, `stale_path`), then path and default shape, then I1/I2 (`read_timing`, including its prefix check), then terminal, relay, depth and fanout. `jsx_attribute` is checked first and overrides any later reason; its meaning widens (J1 at any incoming hop, not only the invocation), which changes `js_arg_follow.reasons` for consumers. Record `js_arg_follow.reasons` per failed contribution even at credited sites, `empty_reasons` per empty eligible site and `fanout` per capped site. I0 and J0/J1 are form dimensions.

## Exclusion and regression rows

Test each finding at the exact site and declaration/target spans. Assert first refusal and one surviving control. K is the independent direct P1 ground `keep(verified)`. R1/R2 labels follow v1 and the r3 summary.

| Finding or variant | Excluding condition and evaluation order | One control |
| --- | --- | --- |
| R1 #1 · mapped `arguments`, alias, relay, dynamic source | O2–O4/S5. Entry binds the supplied object or scalar. A mapped write, even via an alias, or direct eval can change that binding before the body snapshot or relay operand. Check the source binding as well as the copied leaf. | Same snapshot or relay with a stable supplied binding. |
| R1 #2 · prototype mutation | O1/O5. A write, delete, descriptor change or mutating call changes a prototype object before the actual attaches it with colon `__proto__`. The later property lookup can see the changed value or an accessor. A const reference does not freeze it. | Explicit own data key. |
| R1 #3 · computed key | P2/P3. Earlier default, body or mapped-slot writes can change the key before the key expression runs and converts it for `Get`. Only static keys qualify. | Bracketed string literal key. |
| R1 #4 · reserved JSX keys | J1. Element creation precedes renderer delivery. Reserving `key` or `ref` can omit that property before the holder reads it. | Ordinary object with the same own key. |
| R2 W1 · default reads another formal | S2/S4, O2/O3. Earlier formals and pattern elements initialize first and can have written the binding before a later entry/body default reads it. Refuse the dependency and unsupported effects. | Closed callable initializer. |
| R2 W2 · deferred read | I1/I2, O1. Holder initializes first, and later closure execution or resumption can read changed storage or prototype. Refuse deferred carrier/default observations and deferred transfers. | Synchronous private own read. |
| R2 W3 · self-default crash | S2/S4. Reading the binding within its own initializer precedes its initialization and throws. Reject before recursive evaluation. | Closed initializer on omitted actual. |
| R2 W4 · immaterial | Reported folded. The supplied summary gives no mechanism, order or fixture. | No behavioral row can be derived. |
| R3 A-F1 · a literal default reads a formal from the wrong caller | S2/S4 and S5. In `holder(stale)`, `a` initializes before `{cb: a}`. `holder(fallback,{cb:verified})` skips that default. No selected formal can recurse across callers. Cover `[cb]=[a]` (array pattern also refused by P4), a defaulted `p={cb:a}` copied in the body, nested body default and ternary arms. Literal entries/arms read at evaluation, and array entries evaluate before iteration. In `h({cb}={cb:a},a)` the read precedes `a`'s initialization and throws. | Shared site retains only `verified` from the supplied second caller. |
| R3 A-F2 · deferred own-key rerun | I1, O2/O4. First `g()` snapshots and invokes the old property, then writes or deletes it. Second `g()` sees the replacement or takes the default. Repeat for `arguments[0].cb` and delete+default. `Before::new` cuts off later text in that execution owner and misses the next run. | Direct-body snapshot of an unmodified own key. |
| R3 B-F1 · member assignment in earlier default | S2/S4, O2. `a` binds the actual, `x=(a.cb=fallback)` writes it, then `{cb}=a` reads the changed property. Formal-dependent carrier default also refuses. | K. |
| R3 B-F1 · call or mutator method | S2/S4, O2. After entry binds `a`, `m(a)` or `a.m()` can mutate the receiver before the later selection. Calls and carrier escape refuse. | K. |
| R3 B-F1 · delete | S2/S4, O2. Earlier `delete a.cb` removes the own data before the later `Get` can fall through or trigger a default. | K. |
| R3 B-F1 · `arguments[i]` alias | O4/S4. The actual object also appears in the arguments slot. An earlier default changes its member through that reference before the selection, including with an unmapped slot. | K. |
| R3 B-F1 · closure defined in a default and called later | O2 and I1. The default creates a closure retaining the carrier, and a later default or body prefix invokes it before the snapshot. Scan its body even though creation alone performs no write. | Inert closure with no carrier capture or call before P3. |
| R3 B-F1 · same-pattern sibling default | P2/P3, S2/S4, O2. The earlier sibling's default executes before the later sibling reads its key and can modify the shared source. The single-entry grammar excludes the sibling. | One own-key pattern with a closed default. |
| R3 B-F1 · body read after entry mutation | O2, S2/S4. In `h(p,x=(p.cb=fallback)){const {cb}=p; cb()}` the entry write precedes even the first body statement. Baseline body path checking omits formal-list mutations. | Same P3 snapshot with `x=0`. |
| R3 B-F1 · array mutation | P4, S2/S4, O2. `x=a.shift(), [cb]=a` shifts the array before the pattern consumes its iterator. Array patterns and the mutating initializer both refuse. | Own index `{0:cb}` on an unmodified dense literal. |
| R3 A-F3 · legacy context | J1. The renderer can provide position 1 after element creation. JSX syntax cannot prove omission and trigger S2 there. | Ordinary call with explicit position 1. |
| R3 A-F4/B-F2 · `defaultProps` | J1. A legacy renderer can supply absent or undefined props before formal destructuring decides whether to run its JavaScript default. | Ordinary explicit object with a proved own undefined key and a closed S2 default. |
| R3 B-F3 · exported prototype guard coverage | O1/O5. Module evaluation exposes a mutable prototype object. The importer can mutate it before an inheriting actual is constructed and entry looks up the key. Pin behavioral refusal. | Independent own-key ground at the same read. |
| R3 S1 · redundant W3 guards | S2/S4. Test both bare and enclosing-pattern self-initializers. Both evaluate the self-read before binding initialization. Assert semantic refusal and no abort, without requiring both old guards to survive. | Closed pattern default. |
| R3 PR metadata | `PERF_PLACEHOLDER`, CPG `194→195` and counts have no runtime ordering. Reconcile with pricing receipts before a PR update. | Receipt-backed count comparison. |

Positive deltas use one case per I0 placement family and one cross-file and same-file Exact ground. Pair a nested/suspended P1 read with a captured write, a mapped slot, a shadow or a formal-list TDZ negative. For S0 reuse the dropped, NameOnly, multiple and wrong-holder fixtures, plus same-ID owner collision/span mismatch. Pin independent-ground survival and the 8→9 cap. J1 rows retain the ordinary J0 controls.

## Pricing

V1 retained X **0/115** and T **2/383** baseline target sites, with **565/1,382** eligible sites. Treat these as supplied receipts. Reprice with opt-in `PRISM_E_ARG_FOLLOW_PRICE=1` on the same inputs with fresh indexes. Price Exact candidates even where baseline `holder_at` refused.

| Measure | Receipt |
| --- | --- |
| Unit | Same deduplicated `SiteKey` denominator (file, full call bytes, effective source callee). Join each pre-dedup derivation by slot, ordered caller/operand spans, value recipe and terminal ID/bytes. Preserve independent successful routes. |
| Delta | Count kept/withheld derivations and affected sites by first reason and form, including I0/I1/I2 and J0/J1. Report old refusals separately, new/unmatched derivations explicitly, and ID losses separately from sites losing all credit. Reconcile the two independently capped unions against `eligible_sites`, `target_sites`, empty and fanout. |
| JSX comparison | Emit J0 and J1 separately, with both JSX endpoint roles. Report V2 JSX-out and a separate JSX-in sensitivity retaining baseline J1 routes, labeled as pricing only. Price any future delivery proof under the same flag. |

### Priced results (accept-list v2, shadow judge, same inputs as v1)

| Corpus | Baseline target sites | Kept | Withheld | Kept share |
| --- | ---: | ---: | ---: | ---: |
| Excalidraw (X) | 115 | 14 | 101 | 12% |
| TypeScript (T) | 383 | 285 | 98 | 74% |
| Package corpus (SB) | 1,016 | 881 | 135 | 87% |
| Go-side control (G) | 0 | 0 | 0 | none |

Withheld target sites by first reason. X: `jsx_attribute` 82, `read_timing` 11, `fanout` 4, `path_step` 2, `default_dependency` 2. T: `read_timing` 71, `relay_form` 9, `path_step` 5, `opaque_terminal` 4, `fanout` 3, `default_dependency` 2, `stale_terminal` 2, `stale_path` 1, `default_form` 1. SB: `read_timing` 79, `stale_terminal` 50, `stale_path` 5, `relay_form` 1. Sites newly credited that baseline did not credit: X 45, T 122, SB 166 (see the follow-ups in DESIGN §5 E1). Receipts: `price/v2/{X,T,SB,G}.md` in the lane evidence directory, produced by the shadow judge at commit `8bedd57a` (branch `wip/param-binding-e1-price`, over `6215c5e2`). These numbers are the shadow union. The E1 product (baseline grounding AND v2) recomputed from the same records keeps Excalidraw 18 of 115, TypeScript 288 of 383, package corpus 881 of 1,016 (the shadow's four Excalidraw and four TypeScript `fanout` withholds exist only because pricing-only derivations join the union; one TypeScript site is kept only by a pricing-only derivation). The Excalidraw JSX-in sensitivity is 15 of 115: JSX hops that are admitted become `read_timing`, so proving JSX delivery alone would not recover them.

The gate "at most one third withheld on Excalidraw and on TypeScript" is **missed on Excalidraw** (101 of 115 withheld by the shadow, 97 by the product; 82 of the shadow's are JSX attributes as the first reason) and met on TypeScript (98 of 383, 26%). The owner accepted the miss on 2026-11-27; it is recorded, not hidden.
