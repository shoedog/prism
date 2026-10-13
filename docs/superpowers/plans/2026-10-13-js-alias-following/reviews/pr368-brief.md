# Review brief — slice I1: relative imports with an explicit `.js` suffix, bare-dot specifiers and redundant path segments (JS/TS)

You are reviewing as a senior engineer who will live with this resolver for years. Round 1 of at most 2.

## The goal this change serves (judge materiality against this)

prism (a Rust code-slicing / call-graph tool) resolves a JS/TS call through an import by finding the imported module's file. On main, three ordinary spellings of a relative specifier find no file, so every call through them is dropped:
- `import { f } from "./x.js"` when the file on disk is `x.ts` or `x.tsx` (the standard spelling in TypeScript ESM code);
- `import { f } from "."` / `".."` (the directory's index file);
- `import { f } from "./a//b"` / `"./a/./b"` / `"./a/../b"` (redundant segments).

The goal: these three shapes find their module, and **nothing that resolves today changes**. `Exact` in prism is a static-binding grade (see `CLAUDE.md`, call-resolution section): the name resolves, by the language's import/export syntax as prism models it, to exactly one callable.

## What to read

Worktree: `/Users/wesleyjinks/code/prism-js-i1`, branch `feat/js-relative-import-extension`, four commits on `origin/main` (`git log origin/main..HEAD`, `git diff origin/main`).
- Product: `src/call_graph.rs` (`js_ts_relative_module_candidates`, `js_ts_normalized_module_candidates`, `js_ts_literal_module_candidates`, `is_js_ts_relative_specifier`, and the one-line gate change in `resolve_js_ts_member_module`), cache version bumps in `src/cpg_cache.rs` and `src/navigation/call_edge_cache.rs`, one sentence in `CLAUDE.md`.
- Tests: `src/call_graph/js_relative_candidates_test.rs` (unit, the candidate lists), `tests/integration/js_relative_extension_test.rs` (end-to-end, each new spelling against its extensionless twin).
- Consumers of the candidate list you should read: `resolve_js_ts_relative_module` and its callers, `apply_js_export_resolution`, `js_ts_direct_exported_class_owner` (`src/call_graph.rs`), `src/executable_owner.rs` (~390), `src/js_ts_props.rs` (~172), `src/resolution_js_namespace.rs`, `src/navigation/call_resolve.rs` (~62). Relative-specifier logic that does NOT share the function: `file_matches_module` (`src/call_graph.rs`), `src/js_exports.rs` (`raw_relative_literal`), `src/js_paths.rs` (the tsconfig `paths` lane).

## The contract as implemented

The rule is additive: a specifier that has at least one candidate on main returns the identical list. Only a specifier with no candidate today is normalized (segments resolved inside the repo root, never escaping it) and then tried as: the normalized path with the existing extension list; otherwise, when it ends in `.js` and nothing was found, the same stem with `.ts` then `.tsx`; plus the existing index-file list for a directory. `.`, `..` and a trailing slash name a directory only.

Two deliberate differences from the planning document's text for this slice, which you are asked to judge (questions 2 and 3):
- The plan gave a literal `./x.js` TypeScript's own precedence: `x.ts`, `x.tsx`, `x.d.ts`, then `x.js`. The implementation keeps an existing `x.js` winning, as on main, and substitutes only when nothing resolves today.
- `.d.ts` is never a substitution candidate.
Also not modeled, pinned by tests: `.jsx`→`.tsx`, `.mjs`→`.mts`, `.cjs`→`.cts`.

## Measured effect (controller, release binary of this branch against the frozen main binary, `prism nav call-stats --dump-sites`, every call-site record compared)

Binary: branch head `aa74eae0`. Corpora: X = Excalidraw (pinned checkout), T = the TypeScript compiler's `src`, F = a private React application (aggregate counts only).

| | X | T | F |
|---|---:|---:|---:|
| Call-site records compared | 19,219 | 61,712 | 13,299 |
| Records whose targets or drop reason changed | 7 | 15,341 | 0 |
| Dropped (`UnknownName`) → Exact `import_member` | 7 | 15,244 (13,946 product, 1,298 test) | 0 |
| Receiver-typed method call, NameOnly or dropped → Exact (`typed_param`, `constructor_local`) | 0 | 97 (5 product, 92 test) | 0 |
| Record added or removed; an existing Exact target changed or removed | 0 | 0 | 0 |

Every new Exact target was compared with the TypeScript 5.9.3 checker's resolved declaration for that call (X and T are typed): X 7 of 7 agree; T 15,419 of 15,419 agree (12,644 by declaration position, 2,775 where the checker names an overload signature and the target is that function's implementation). No disagreement, no site left unchecked. The 97 receiver-typed sites are method calls on a value whose class is imported through a `.js`-suffixed specifier; the existing receiver rungs can now see the class.

On the 1,011-fixture variant matrix (a separate test-only PR), this branch changes 17 rows, all module-path rows moving from MISSING to CORRECT, and no other row.

## Questions

1. **Additivity.** Is there any input where a call that resolves on main now resolves differently, to more candidates, or not at all? Consider every consumer above, in particular the ones that require exactly one candidate, and consumers for which "the module was not found" and "the module was found but lacks the export" lead to different outcomes (a call that used to fall through to a later rung and now stops, or the reverse).
2. **Precedence.** Literal-file-first versus TypeScript's order when both `x.js` and `x.ts` (or `x.js` and `x.d.ts`) exist. Which should prism do, given that main already prefers `.js` over `.ts` for an extensionless specifier, and given the static-binding meaning of `Exact`? If you would choose differently from the implementation, name the input, the result under each choice, and what is lost.
3. **`.d.ts`, `.jsx`, `.mjs`, `.cjs`.** Is leaving these out right for this slice? A SMELL unless you can show a wrong edge or a lost existing edge.
4. **New wrong edges.** Name any input where a newly found candidate is not the module the import means: root escapes, a directory and a file sharing a stem (`x.ts` and `x/index.ts`), `x.js.ts`, a directory named `x.js`, case differences, dotfiles, trailing slashes, `..` chains, mixed normalization and suffix substitution, specifiers with query strings or fragments.
5. **Negative controls.** Missing export, type-only import or export, a same-named function in an unrelated file: does any of these now produce an edge?
6. **Other relative-specifier logic.** The tsconfig `paths` lane and import-forwarding keep their own logic and are unchanged. Is leaving them unchanged safe (no inconsistent pair of answers for the same specifier that produces a wrong edge), or does it need a follow-up? Say which.
7. **Tests.** Does each conjunct of the rule have a test that fails without it (the implementer ran 11 manual mutants; the repository's mutation gate does not register these functions)? Which mutant would survive?
8. **Persisted state and docs.** Cache versions, pins, and the `CLAUDE.md` sentence: accurate?

## How to report

Tag every finding **WRONG** (the code provably does the wrong thing: name the input or state and the incorrect result) or **SMELL** (a risk or gap with no demonstrated incorrect behavior). A finding without a concrete failure scenario is a SMELL. Also tag each **MATERIAL** or **IMMATERIAL** to the goal above. List WRONG items first.

For each finding give: the evidence (file and line, or the fixture you ran and its output on this branch AND on main, since a difference from main is what matters); a recommended fix and at least one alternative with its cost; and when the finding would not apply.

End with a verdict: **APPROVE**, **FIX** (list what must change before merge) or **REJECT**, then "checked and found sound" and "not checked" lists.

## Rules

- You may read anything in the worktree and run its tests (`cargo test --lib js_relative_candidates`, `cargo test --test integration js_relative_extension`). To compare behavior on small synthetic fixtures you write, use the two prebuilt release binaries (do not rebuild): this branch `/Users/wesleyjinks/prism-evidence/js-alias/measure/bin/i1b-prism`, main `/Users/wesleyjinks/prism-evidence/js-e5/step0/bin/base-prism`; run `<binary> nav --cache-dir <fresh dir in your scratch> call-stats --repo <fixture dir> --dump-sites` and read the `call_site` records.
- Write scratch files only under your scratch directory (given below). Do not modify, add or delete any file in the worktree; make no git writes of any kind and post nothing to GitHub. The controller folds findings.
- Never execute corpus packages. Never open any directory named `frontend-portal`. Do not open any directory whose name starts with `f-e5-`, `f-alias-` or `F-private`, nor `~/prism-evidence/js-e5/cache`.
- Write your report to the report path given below, and print its final verdict line at the end of your output.
