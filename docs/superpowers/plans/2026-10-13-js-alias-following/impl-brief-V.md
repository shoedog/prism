# Implementation brief — slice V: follow a value returned by an in-repo function (JS/TS)

You are implementing one test-first slice in the prism repo (Rust; code slicing / call-graph tool). Work ONLY in the git worktree `{{WORKTREE}}` (branch `{{BRANCH}}`, cut from `{{BASE}}`). Read its `CLAUDE.md` first.

## Goal (what this serves)

In React and hook-style code the callee of a call is very often a value another function returned:

```ts
const { save, cancel } = useEditor();     // save() and cancel() are called below
const [value, toggle] = useToggle();
const handler = makeHandler();
```

prism does not follow that. The binder classifies `save` as "alias / may-call, keep base behavior", and base behavior searches for a function named `save`. Measured on three codebases, that name search is right more often than not, because the returned key usually has the same name as the function it holds; but it is a coincidence of spelling, it fails whenever the names differ or collide, and it gives nothing at all when the destructure and the call are in the same function (a lexical guard drops those). A census that followed one returned-value step by syntax alone reached a named in-repo callee on 71 call sites of a private React app, 98 of Excalidraw and 14 of the TypeScript compiler, with zero disagreements against the TypeScript checker on the two typed codebases (1,678 and 106 agreeing sites).

This slice makes that step a proof. It adds edges and replaces a coincidental or wrong edge where the returned value is proven. It never removes an edge: a site it cannot prove keeps exactly today's behavior. Say so in your commit message and report.

## Grounding (verified; start here, then verify yourself)

- The binder and slice L, which you are stacked on: `src/ast/js_binding.rs` (`js_ts_classify`: a destructuring declarator with a call value is `MayCall` when the call has an inline function argument, else `Alias`; an identifier declarator with a call value goes through `js_ts_classify_call`, where admitted React wrappers become `Callable`), `src/ast/js_binding_alias.rs` (`js_ts_follow_alias`: how a `const` alias chain is followed to `JsFollow::{Callable, ImportLocal, NamespaceMember}`; the whole-pattern admission rule for object patterns), `src/call_graph.rs` (`JsLocalBinding` and its `Alias*` variants, `js_local_binding_at`, the stand-down in `compute_indirect_call_sites`), `src/resolution_js_alias.rs` (how each terminal resolves: in-file callable by registered name and span; import local through the import-member route; namespace member through the per-export proof; a terminal that is not proven keeps main's behavior), and the navigation index fix that counts a followed-alias site once for its target (`src/navigation/mod.rs`, `build_resolved_call_edges`).
- Per-file facts that already cross files: the JS export facts (`src/js_exports.rs`, built in `src/ast.rs`, resolved in `CallGraph::apply_js_export_resolution`, persisted with the CPG cache). The returned-value facts you add are the same kind of thing: computed per file from syntax at build time, stored on the call graph, read by the resolver.
- prism names an inline function after what it is assigned to: a declarator (`const doIt = () => …`), an object-literal property (`doIt: () => …`), a method (`doIt() {}`). A bare inline function with no such name has no identity in the call graph and cannot be a target.
- The matrix (`tests/fixtures/js_alias_matrix/`, read its README): the rows with `step: "returned"` and the older local shapes `hook-result-destructure`, `custom-hook-array-result`, `hook-result-two-keys`, `hook-result-two-slots` are this slice's test list.

## Contract (this is the spec)

**Which bindings are followed.** The site's callee is a bare identifier whose nearest binding (by the existing binder walk; one declaration; clean scope; never written) is a `const` declarator of one of these forms, where `H` is a bare identifier and the call is not `new`, not optional-chained, not awaited:
- (a) `const x = H(...)`: the value is what `H` returns;
- (b) `const { k: x } = H(...)` or `const { x } = H(...)`: key `k` of what `H` returns. The object pattern must pass the same whole-pattern admission as slice L (every entry a shorthand identifier or a plain-key pair with an identifier value; no rest, nesting, computed/string/numeric keys or defaults anywhere);
- (c) `const [a, x] = H(...)`: slot `i` of what `H` returns. Every element before and at the slot is a plain identifier or a hole; no rest element, no default, no nested pattern anywhere in the pattern.

Where today's classification of such a declarator is an admitted React wrapper (`Callable`), nothing changes: that route stays first.

**Which function `H` is.** `H`, looked up by the binder at its own position, must be proven by an existing route to be exactly one in-repo function: a same-file callable, an import local that the import-member route resolves to exactly one function, or what slice L's alias follow reaches. A parameter, a member call, an external package, an unresolved import, several candidates → not followed.

**What `H` returns (the returned-value fact).** Computed per function from syntax:
- Take the function's value-returning statements: `return <expr>;` anywhere in its body but not inside a nested function or class; for an arrow with an expression body, that expression. `return;`, `return null`, `return undefined` are ignored. A generator or an `async` function has no fact (its call does not yield the returned value).
- Each return expression, after unwrapping parentheses and TypeScript assertions (`as`, `satisfies`, non-null, angle-bracket; `as const` included), must be one of: an object literal; an array literal; an identifier bound in that function to a unique, unwritten `const` whose value (unwrapped) is an object or array literal (one such step, no chains); or, for form (a) only, an expression that is itself a callable value as defined below.
- Selecting from an object literal by key `k`: the property is `k: <value>`, shorthand `k`, or a method `k() {}`. The literal must have no computed key, no getter or setter, no duplicate of `k`, and no spread element AFTER the property (a spread before it is fine: the later key wins). Selecting from an array literal by slot `i`: the element exists and no spread element occurs at or before it.
- The selected value must be a **callable value** in `H`'s file: an inline function expression or arrow that has an identity (named by its property key or method name); or an identifier whose binding at that position is a proven callable (`Callable`, including the admitted `useCallback(inline)` form), a program-scope import local, or what slice L's follow reaches (a callable, an import local, a namespace member). A parameter of `H`, another call's result, a conditional, anything else → no fact for that key.
- All value-returning statements of `H` must select the same callable identity for the requested key or slot. If they differ, or one of them does not yield one, there is no fact.
- Exactly one returned-value step: if the selected value is itself the result of another call (`const { doIt } = useInner(); return { doIt };`), there is no fact in this slice.

**How the site resolves.** `H` is resolved to its one function; that function's fact for the path gives a callable value in `H`'s file; the value resolves as slice L's terminals do, in the context of `H`'s file (an in-file callable by name and span; an import local or namespace member through the import routes as seen from `H`'s file). The result is Exact only for exactly one target with a source span, and keeps the kind of the route that finally produced it. A wrapped React value binds from JSX element sites only, as elsewhere. If any step is not proven, the site resolves exactly as it does on your base (same rungs, same outcome, same drop reason).

**Consumers.** A site resolved this way is one `Source` record with one target: it must appear once in the navigation callers index for its target whatever the spelling (extend the fix slice L made), once as a CPG call edge, and once in `resolved_caller_edges`.

**Persisted state.** The facts and any new `JsLocalBinding` variant are persisted: bump the CPG cache version and the nav call-edge cache version, their pins, the anchored cache-version mutants in the older lanes, and extend the existing round-trip and version-miss tests.

**Must not change.** Any site not of forms (a)–(c). `let`/`var` bindings, written bindings, awaited calls, member calls on a call result, `new`. Module export summaries. Non-JS output. Nothing in this slice refuses a name search.

## Test first

1. **Matrix.** Run it green on your base. After implementing, list the differing rows, bless, and report the diff as a table (shape → old verdict → new verdict; field-only changes separately).
   Expected to become `CORRECT` in both scopes and both languages: `object-shorthand-local-fn`, `object-renamed-key-decoy`, `object-const-arrow`, `object-usecallback`, `object-import-value`, `object-inline-arrow-prop`, `object-method`, `object-renamed-binding`, `array-slot`, `value-is-function`, `ident-object`, `arrow-expression-body`, `default-export-hook`, `same-file-hook`, `jsx-component`, `spread-before-key`, `early-null-return`, and the older `hook-result-destructure`, `custom-hook-array-result`, `hook-result-two-keys`, `hook-result-two-slots` (all namings, including the two-decoy naming, whose NameOnly candidates disappear).
   Expected unchanged, byte for byte (each is outside the contract; the row's `contract` text says why): `usememo-object`, `spread-after-key`, `divergent-returns`, `param-value`, `nested-hook`, `await-async`, `external-hook-trap`, `hook-is-parameter`, `let-binding`, `reassigned-binding`, `member-call-on-result`, `wrapper-call`, `react-usestate-setter`, `react-usecontext-destructure`, and every row outside the shapes named above.
   No ledger entry should be needed (nothing regresses by the guard's definition). If one is, or a row outside the expected set changes, stop and report.
2. **Focused tests**, written first and shown RED on your base: fact-level tests (each return form; each refusal: spread after the key, computed key, getter, duplicate key, divergent returns, a return inside a nested function ignored, async and generator functions, a returned parameter, a returned call result, a `let` or written intermediate identifier, an array spread before the slot); binder-level tests for forms (a)–(c) and each not-followed condition; resolver-level tests asserting the full record set at the site (origin, target file/name/span, grade, kind) for: a hook in another file imported by name, by default import and through a barrel; a hook in the same file; a returned key whose name differs from the function's, with a same-named decoy elsewhere (must bind the real function, not the decoy); a decoy with the key's name when the fact is absent (must behave as on the base); JSX; the one-edge-per-consumer checks.
3. **Mutation lane** in the existing format (`mutants/`): at least one mutant per conjunct of the contract (const only; unwritten; whole-pattern admission; `H` must be exactly one function; nested-function returns excluded; null returns ignored but other disagreements not; spread-after refusal; computed-key refusal; slot index; one step only; span required; fall back to base when unproven; counted once in the callers index). All KILLED (authoritative).
4. Keep every Rust file under 600 lines: new modules for the facts, the binder follow and the resolver part.

## Verify (report exact totals and verbatim summary lines)

`cargo fmt --check`; the clippy invocation CI uses, `--features mcp`, all targets; full suite `cargo nextest run --all-targets --features mcp --no-fail-fast` and `cargo test --doc --features mcp` (known unrelated failures may appear: `example/parameter_slot_characterization …d7…`, load-sensitive `mcp::…::lazy_*`; report, do not fix); the matrix test once with `RAYON_NUM_THREADS=4`; `cargo build --release && cd eval && uv run tier-a --matrix-only --allow-stale-sut` (report regressions and flip candidates, never re-baseline); `python3 scripts/mutgate/mutgate.py --plan-only` exit 0 and your lane authoritative. State what you did not verify.

## Boundaries

- Static work only inside this worktree. Do not read or open any directory named `frontend-portal`, anything under `~/prism-evidence/`, or any other checkout. No corpus runs: the controller measures on the corpora, including build time and memory.
- Git: commit locally in small logical commits (tests first). Do NOT push, open PRs, or touch other branches or worktrees. End each commit message with these two lines exactly:
  `Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>`
  `Claude-Session: https://claude.ai/code/session_01CeCpr7mLfpQeq9vEEKBFhQ`
- No planning or analysis documents. Add to the call-resolution paragraph of `CLAUDE.md` one or two sentences stating the rule (which bindings, exactly one in-repo function, one returned-value step, object/array literal selection, unproven keeps the earlier resolution).
- No scope growth. Not in this slice: more than one returned-value step; `useMemo` and other wrappers around the returned object; values that are the called function's parameters (argument following); `useContext`; awaited results; member calls on a call result; refusing any name search. If the contract cannot be met as written, or meeting it changes a row it says must not change, stop and report with the evidence.

## Report back (concise, facts only)

1. Commits and `git diff --stat {{BASE_REF}}`.
2. RED evidence per behavior.
3. The matrix table and any row outside the expected set.
4. The design in a few lines: what the fact is and where it is computed and stored; the new binder outcome; how the resolver chains `H` → fact → terminal; what you reused from slice L.
5. Verification totals; lane results; the size of the persisted facts on the largest test repo you built, if easy to read.
6. Anything in the contract you could not meet, anything in the grounding that turned out wrong, and what you did not verify.
