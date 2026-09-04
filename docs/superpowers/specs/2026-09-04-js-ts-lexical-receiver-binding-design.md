# JS/TS lexical-scope-aware receiver binding

**Status:** implemented; focused gates and review round 1 green
**Recorded:** 2026-09-04
**Exact base:** `5051918f61c99fda83eb18936992fb62025b7669` (PR #230 merge)
**Scope:** prerequisite fact and fail-closed import-qualifier guard for simple identifier receivers in JavaScript, TypeScript, and TSX

## 1. Decision and slice boundary

Implement item 3 of the owner-selected receiver queue as a prerequisite only. For a qualified call such as `api.m()`, record whether the simple identifier receiver `api` is lexically bound at that call. An exact imported-module edge is forbidden when the receiver is locally bound.

This slice does not recover a receiver type and does not emit `TypedParam` or `ConstructorLocal`. TypeScript typed-parameter/annotated-local recovery and JavaScript/TypeScript `new`-constructor recovery remain item 4. Bare factory calls, imported type ownership, general assignment flow, bare imported-member calls, and a general-purpose JS/TS scope graph remain excluded.

## 2. Demonstrated wrong behavior

On exact base, `import api from './api'; function run(api: Foo) { api.m(); }` emits one `Exact` `ImportQualified` edge to `api.ts::m`. The parameter is the receiver binding, so that target is wrong. The same mechanism affects JavaScript parameters and in-scope local declarations.

The alternative hypothesis—that parameter/local names are not extracted—is false: `ParsedFile::js_ts_function_local_bindings` records them. The wrong edge occurs because the `ImportQualified` resolver arm does not consult a call-position-aware receiver-binding fact. The existing whole-function set cannot supply that fact because it makes a nested block binding suppress an import after the block has ended.

## 3. Binding authority

Add a defaulted, serialized `CallSite.receiver_lexically_bound` derived fact. It is considered only when all of these hold:

1. The caller language is JavaScript, TypeScript, or TSX.
2. The call has a simple identifier receiver node; composite expressions remain false/unmodeled in this increment.
3. The binding is introduced by an enclosing function parameter, a function-scoped `var`, or a lexical declaration whose scope contains the call.

Lexical declarations include `let`/`const` declarators and destructuring patterns, catch parameters, class declarations, and function declarations. Scope boundaries include the function body, statement blocks, `for` variants, `switch`, and `catch`. A lexical declaration applies throughout its scope even when textually after the call: an earlier reference is in the temporal dead zone and therefore cannot denote the import. A `var` applies throughout its containing function, including when declared in a nested block or after the call. Bindings in enclosing functions/blocks are visible to nested callables; bindings declared only inside a nested callable do not reach an outer call. Ended nested and sibling blocks do not leak.

TypeScript represents `for (const api of items)` as a `for_in_statement` whose `left` is the identifier and whose declaration keyword is an unnamed token. Header binding classification must therefore inspect the `var`/`let`/`const` token directly rather than assuming a `variable_declarator` wrapper. Named function-expression and class-expression self names apply only inside their own bodies.

Parse-recovery uncertainty encountered while classifying the receiver's containing function is fail-closed for import qualification: set the fact and suppress the exact import edge.

## 4. Consumer and persistence

Compute the fact during full and direct-subset call-site extraction from the existing receiver AST node and function node. Synthetic/unqualified/non-JS/TS sites default false. `CallGraph::resolve_call_site` must skip the `ImportQualified` arm when the fact is true; later legacy ladders remain unchanged in this prerequisite slice.

The fact is derived metadata and is excluded from `CallSite::cmp_key`, matching other receiver-classification metadata. Older serialized rows default false, but accepting them would preserve stale wrong topology. Advance CPG cache 57 to 58 and navigation call-edge cache 26 to 27. Round-trip tests must prove the fact and the changed absence of the exact imported-module edge survive both paths.

## 5. RED/GREEN acceptance

RED first on exact base:

- TypeScript and JavaScript parameter receivers suppress `ImportQualified` while unrelated parameter names do not.
- An in-scope `let`/`const`, destructuring binding, catch binding, class/function declaration, and function-scoped `var` suppress the imported-module exact edge.
- `let`/`const` after the call in the same scope and `var` after the call still suppress it.
- A binding in an ended nested or sibling block and a binding inside a nested callable do not suppress the outer import.
- Enclosing parameter/block captures and named function/class self bindings suppress the import; `for`/`switch` declarations stop at their lexical boundary.
- Full and direct-subset builds agree on the fact and resolution outcome.
- CPG/navigation cache pins fail at 57/26 and round trips preserve the new behavior.
- Existing typed-parameter, annotated-local, constructor-local, and bare-factory no-recovery assertions remain negative until item 4.

Every new classifier branch needs a positive binding case plus a non-reaching scope control. The base test command must show the wrong exact edge before implementation; passing pre-existing assertions that encode that edge is not RED evidence.

## 6. Verification and review

Required gates: focused JS/TS RED/GREEN; full JavaScript, TypeScript, TSX, import-resolution, subset/incremental, and cache targets; format/diff/check/configured Clippy; full default and `mcp` suites with totals; release build; Tier-A matrix-only; immediate second release build; Tier-A quick. Tier-A pin drift is reported, never re-baselined.

Review cap: two rounds. At the cap, recurring/open-class binding-form or scope-boundary findings park the artifact. Closed, non-repeating findings receive targeted fixes on the same artifact.
