# Implementation brief — slice L: follow `const` aliases to the callable they name (JS/TS)

You are implementing one test-first slice in the prism repo (Rust; code slicing / call-graph tool). Work ONLY in the git worktree `{{WORKTREE}}` (branch `{{BRANCH}}`, cut from `{{BASE}}`). Read its `CLAUDE.md` first.

## Goal (what this serves)

prism resolves a JS/TS call `f()` or element `<F />` by asking what the name is bound to. Today, when the nearest binding is a local alias — `const f = imported;`, `const f = N.f;`, `const { f } = N;` — the binder answers "alias, keep base behavior". Base behavior never looks at what the alias names. The site is then either dropped, or resolved by spelling: to a same-file function that happens to share the alias's name, or to the one function in the whole repo with that name. That second path produces wrong Exact edges; an earlier attempt to refuse it by evidence rules was reviewed twice and each round found new alias shapes it mishandled.

The owner's direction replaces that with a positive rule: **the nearest binding decides, and an alias is followed to what it names.** This slice adds the following of `const` aliases. It adds edges and replaces a wrong edge only where the alias is now proven. It does NOT yet stop the by-name search for aliases it cannot follow; that is the next slice (R), reviewed separately. Say so in your commit message and report: this slice adds binding proof and does not yet fulfil the "no name search for bound names" objective.

## Grounding (verified on main; start here, then verify yourself)

- The binder: `src/ast/js_binding.rs` (`JsBinding`, `js_ts_scope_binding`, `js_ts_classify`), `js_binding_site.rs` (`js_ts_site_binding`, the scope index), `js_binding_walk.rs` (the site walk), `js_binding_values.rs` (`js_ts_classify_call`, the React-wrapper admission, `js_ts_holds_no_function`, `js_ts_has_default`), `js_binding_writes.rs` (the scoped write resolver, `js_ts_scoped_written`), `js_binding_namespace.rs` (`js_ts_namespace_binding`: proves that an identifier is one unwritten program-scope namespace import and returns its module path). The comments cite "SPEC §3.x"; find that spec under `docs/superpowers/` (grep for `§3.8`) and keep its ordering and invariants (B0, B1, B2, M1, M2, NoFn).
- `js_ts_classify` returns `JsBinding::Alias` for a declarator whose value is anything but a function, an admitted wrapper call, or a literal that provably holds no function. A written binding is `MayCall` before that point.
- `src/call_graph.rs` (~5680–5740) maps `JsBinding` to the serialized `JsLocalBinding` on each `CallSite` (`Alias` → `MayCall("alias")`).
- `src/resolution.rs`, the `None`-qualifier arm of `resolve_call_site` (~3398 onward): `Callable(t)` → `js_ts_local_callable` (same-file callable by registered name and span); then R4c import-member (JS/TS branch uses `js_ts_import_member_candidates`, with a `js_ts_function_locals` shadow guard); then same-file by-name (R4); then a lexical guard; then the repo-wide name search (R5, `free_single`/`free_multi`). The namespace-qualified route (`N.f()`) is in the qualifier arm (~2565–2620) and uses `JsLocalBinding::NamespaceImport` and `js_ts_namespace_outcome` (`src/resolution_js_namespace.rs`).
- A warning already in the code (`js_ts_classify_call` doc): an earlier change called `js_ts_site_binding` from inside classification and recursed without bound on a self-referential declarator. Your follow must be bounded and cycle-safe by construction.

## Contract (this is the spec)

**Which declarations are alias links.** A declaration is followed only when ALL hold; otherwise the outcome is exactly what main gives today (`Alias`, `MayCall`, `Refused(..)`, …):
1. It is the single declaration of the name in the nearest declaring scope and passed the existing checks in their existing order (clean scope, sealed recovery, not written).
2. It is a `variable_declarator` in a `const` declaration (not `let`, not `var`, not an assignment or `using` form).
3. After unwrapping parentheses and TypeScript `as` / `satisfies` / non-null / angle-bracket assertions (NOT assignment chains), its value is one of:
   - **(a) an identifier** `X`, with the declarator's name an identifier: `const a = X;`
   - **(b) a namespace member** `Q.m`, with the declarator's name an identifier, `Q` an identifier, `m` a plain property name, no optional chaining, no computed access: `const a = N.f;`
   - **(c) an identifier `Q` destructured flat by an object pattern**, where the name is bound directly in that pattern either as shorthand `{ f }` (key `f`) or as a rename `{ f: a }` (key `f`), with no default anywhere in the pattern (the existing conservative cut), no nesting on the path to the name, not a rest element, not a computed or string key.

**What a link resolves to.**
- For (b) and (c): `Q` must be proven by `js_ts_namespace_binding` to be one unwritten program-scope value namespace import with a module path. The result is "member `m` (or the key) of that module's namespace". Anything else about `Q` (an object literal, a parameter, a `require` result, a default import, a type-only import, a shadowed or duplicated `N`) → not followed, outcome unchanged from main.
- For (a): look up `X`'s binding at the identifier node in the initializer (the same lookup a call site at that position would get):
  - a proven callable → that same callable (including its `wrapped` flag, so a wrapped React value still binds from JSX sites only);
  - a well-formed import binding → "the import local `X`";
  - another alias link → follow it;
  - anything else (parameter, `let`, written, call result, conditional, unbound, …) → not followed, outcome unchanged from main.
- **Bounds:** at most 8 alias links in one chain, and a visited set keyed by declaration identity; exhaustion or a cycle → not followed. All links are in one file.

**How the site then resolves.**
- Terminal "same-file callable" → exactly as `Callable(t)` does today.
- Terminal "import local `X`" → exactly as a direct call to `X` at program scope would resolve through the JS/TS import-member route (named, renamed, default, re-export barrel — whatever that route supports today), WITHOUT the caller-locals shadow guard for the site's spelling (the binder has already proven what the alias names). Zero candidates → dropped; never continue to the same-file-by-name or repo-wide rungs.
- Terminal "member `m` of module `P`'s namespace" → exactly as a qualified call `N.m()` through that namespace import resolves today (the namespace export proof). Not resolvable (external package, missing export, type-only, an export the export proof does not accept) → dropped; never continue to the by-name rungs.
- Keep the resolution kind of the route that produced the target (`local_def`, `import_member`, `import_qualified`). Exact only for exactly one target, exactly as those routes grade today.
- Call sites and JSX element sites both. Records with origin other than `Source`, member-callee sites, `new`, and `super` are untouched.

**Must not change.**
- Module export summaries: `js_ts_local_export_target` (D4) must return for an alias link exactly what it returns for `Alias` today (an `export const x = f;` stays unproven). Following exported aliases across files is a later slice.
- Any site whose binding is not an alias link. Any non-JS language (byte-identical output).
- Nothing in this slice refuses the by-name rungs for unfollowed aliases.

**Persisted state.** `JsLocalBinding` is serialized. Bump the CPG cache version and the nav call-edge cache version, update their pins, and cover the new variants with the existing serialization/round-trip and cache hit/miss test patterns.

## Test first

1. **Matrix.** `tests/fixtures/js_alias_matrix/` holds enumerated fixtures with pinned expectations (read its README). Before touching product code, run `cargo test --test integration js_alias_matrix` and confirm it is green on your base. After implementing, run it: it will list every row that changed. Then bless (`PRISM_BLESS_JS_ALIAS_MATRIX=1`) and read the `expected.jsonl` diff row by row. Expected: rows of shapes `ident`, `chain2`, `chain8`, `ns-member`, `ns-member-paren`, `ns-member-as`, `ns-member-nonnull`, `ns-member-satisfies`, `ns-destructure`, `exported-ns-member`, `block-shadow`, `inner-shadows-import`, `shadows-same-file-fn` (if present) move to `CORRECT` in every scope, form and naming that has a call-site record. Rows of `ns-member-exported-alias` and `external-ns-member-alias-trap` may move from `WRONG` to `MISSING` / `NONE-OK` (the alias is followed and the route correctly finds nothing). Every other shape must be byte-identical: `let-stable`, `var`, `let-reassigned`, `conditional`, `wrapper-call`, `bind`, `array-destructure`, `default-destructure`, `fallback-or`, `obj-member`, `deep-member`, `element-member`, `declared-then-assigned`, `for-of-element`, `object-param-destructure`, `alias-of-param`, `catch-binding`, `chain9`, all `react-*`, hook, `require-*`, `await-import-*`, `ts-import-equals-member`, parameter, import-path, global and member-alias rows. The bless guard must not need an `accepted_changes.jsonl` entry; if it does, stop and report the row.
   Report the diff as a table: shape → count of rows by (old verdict → new verdict). Any changed row outside the expected shapes is a finding to report, not to bless silently.
2. **Focused tests**, written before the implementation and shown RED on the base for each new behavior: binder-level tests beside the existing `src/ast/js_binding_*_tests.rs` (outcome per link form; each "not followed" condition: `let`, `var`, written anywhere in scope including a nested closure, duplicate declaration, default in pattern, nested pattern, rest, computed key, optional chain, assignment-chain value, `Q` not a namespace import, `Q` shadowed by a parameter or inner declaration, cycle, 8 links succeed, 9 refuse, parse-error scope); resolution-level tests asserting target identity, kind and confidence (alias name different from the target's name; a same-named decoy function in another file; a same-named decoy function in the same file; JSX and call forms; `wrapped` terminal from a call site is not bound; zero-candidate import local drops and does not reach the by-name rungs); D4 unchanged for an exported alias; serialization round-trip; cache version pins.
3. Keep every Rust file under 600 lines: put the follow logic in a new module (for example `src/ast/js_binding_alias.rs`) and the resolution helpers in a new file beside `src/resolution_js_namespace.rs` rather than growing `js_binding.rs` or `resolution.rs`.

## Verify (report exact totals and verbatim summary lines)

- `cargo fmt` then `cargo fmt --check`; the clippy invocation CI uses (see `.github/workflows`) with `--features mcp` on all targets.
- Full suite: `cargo nextest run --all-targets --features mcp --no-fail-fast` and `cargo test --doc --features mcp`. A failure outside your change: check it on the base commit in the same environment and report it; do not fix or re-baseline.
- `cargo build --release && cd eval && uv run tier-a --matrix-only --allow-stale-sut`; report regressions and flip candidates, never re-baseline.
- `python3 scripts/mutgate/mutgate.py --since {{BASE_REF}} --scope fn` (advisory); kill real survivors in your new code with tests and list any you judge equivalent with the reason.
- State what you did not verify.

## Boundaries

- Static work only inside this worktree. Do not read or open any directory named `frontend-portal`, anything under `~/prism-evidence/`, or any other checkout. No corpus runs: the controller measures on the corpora.
- Git: commit locally in small logical commits (tests first). Do NOT push, open PRs, or touch other branches or worktrees. End each commit message with these two lines exactly:
  `Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>`
  `Claude-Session: https://claude.ai/code/session_01CeCpr7mLfpQeq9vEEKBFhQ`
- No planning or analysis documents. Add one sentence to the call-resolution paragraph of `CLAUDE.md`: a JS/TS local alias resolves Exact only when a bounded chain of unique, unwritten `const` bindings reaches one callable, an import, or a namespace-import member; spelling alone is not binding evidence.
- No scope growth. Not in this slice: `let`/`var` aliases, object-literal members, call results (hooks, factories), parameters and arguments, `require`/dynamic `import()` aliases, `import x = N.y`, exported aliases across files, the refusal of by-name rungs. If the contract cannot be met as written, or meeting it changes a row it says must not change, stop and report with the evidence rather than widening or narrowing it yourself.

## Report back (concise, facts only)

1. Commits and `git diff --stat {{BASE_REF}}`.
2. RED evidence per new behavior (test name, failing message on base).
3. The matrix diff table and any row outside the expected set.
4. The design in five lines: new binder outcomes, where the follow lives, how the bound and the visited set work, which resolution helpers were factored and reused.
5. Verification totals for each command; mutation survivors.
6. Anything in the contract you could not meet, anything in the grounding that turned out wrong, and what you did not verify.
