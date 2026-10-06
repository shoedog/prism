# Step-0 census on base: all four gaps (planner, public corpora)

`[MEASURED]` 2026-10-05.
- **Command:** `node probes/census.cjs <TS 5.9.3 typescript.js> <root> <out.json> [--multi]`.
- **Outputs:** `~/prism-evidence/js-param-defs/census/{X,Xi,T,SB}.json`.
- **F:** the controller runs `CONTROLLER-pd.sh census` (aggregate only); its result is not included here.

**Method.**
- The census is syntactic, using the pinned TS 5.9.3 AST and checker.
- The file walk mirrors `repo_loader` (hidden, skip dirs, 2 MiB cap, js/mjs/cjs/jsx/ts/tsx).
- The "named" test mirrors `Language::function_name`'s ECMAScript name-inference patterns 1–5, using the direct parent (parentheses count as a parent).
- Parameter uses are classified by **checker symbol identity**:
  - bare: any non-`p.x` reference;
  - member-only: only `p.x` / `p?.x`;
  - unused.

**Two disclosed approximations.**
- Prism's `has_bare_references` is **name-based**: a same-name identifier in a nested scope counts. So the census's member-only counts are an upper bound on prism's.
- The projected Use rows count distinct Use lines per formal, while DFG rows are per (Def, Use line, label).

Corpora:
- X = excalidraw `0642e72c`;
- Xi = its installed tree (prism skips `node_modules`, so it is identical to X);
- T = `~/code/bench-repos/TypeScript/src`;
- SB = 583 authenticated SecBench package roots.

| Gap (PR) | Quantity | X | Xi | T | SB |
|---|---|---|---|---|---|
| — | files / callables | 627 / 8,612 | 627 / 8,612 | 707 / 21,011 | 25,308 / 199,145 |
| **1 callback identity (PR-B)** | anonymous callables (prism `function_name` = None) | 4,496 | 4,496 | 5,292 | 69,502 |
| | … whose parent is a call argument | 3,909 | 3,909 | 5,094 | 57,274 |
| | … JSX expression / `new` argument / `return` / IIFE | 406 / 49 / 71 / 23 | same | 0 / 3 / 84 / 3 | 46 / 685 / 2,514 / 998ᵃ |
| | anonymous-owner formals used bare (projected new Defs) | 1,199 | 1,199 | 2,187 | 34,188 |
| | projected Use lines for those Defs | 1,887 | 1,887 | 3,012 | 52,873 |
| | of those formals, owner is a call argument | 856 | 856 | 1,624 | 19,224 |
| **2 member-only (PR-C)** | plain formals, named owner, member-only | 1,063 | 1,063 | 4,095 | 21,935 |
| | … distinct member Use paths (projected projection edges) | 3,188 | 3,188 | 7,891 | 64,234 |
| | plain formals, anonymous owner, member-only (needs PR-B first) | 596 (871 paths) | same | 920 (1,516) | 13,325 (40,626) |
| **3 bare arrow (PR-A)** | `x => …` formals | 2 | 2 | 3,556 | 1,845 |
| | named owner: bare / member-only / unused | 0 / 0 / 0 | same | 735 / 1,041 / 5 | 356 / 134 / 4 |
| | anonymous owner (bare-used; needs PR-B) | 2 (0) | same | 1,775 (1,128) | 1,351 (894) |
| | async / curried-inner | 0 / 0 | same | 0 / 5 | 19 / 0 |
| **4 rest (PR-A)** | `...ident` formals (destructured `...{…}`/`...[…]`) | 46 (0) | 46 (0) | 134 (0) | 534 (0) |
| | named owner: bare / member-only / unused | 33 / 8 / 0 | same | 111 / 9 / 2 | 388 / 43 / 34 |
| | anonymous owner (bare-used; needs PR-B) | 5 (5) | same | 12 (12) | 69 (37) |
| | type-annotated / written in body | 42 / 0 | same | 115 / 0 | 39 / 10 |
| **PR-A projection** | new parameter Defs (gap 3 + gap 4, named, bare-used) | **33** | 33 | **846** | **744** |
| | projected Use lines | 38 | 38 | 1,015 | 973 |

ᵃ SB also has 1,709 `CallExpression` callee positions (`!function(){}()`-style minified IIFEs). The full per-parent histograms are in the JSON files.

**What the census says about the PR order (projections; F to follow from the controller).**
- **PR-A is small on X (33 Defs, all rest), large on T (846) and moderate on SB (744).** Its measured delta (`MEASUREMENTS-prA.md`) matches the projection:
  - X: 36 ADDED rows against 38 projected Use lines;
  - T: 1,065 def→use rows against 1,015 projected.
- **PR-B (callback identity) is the largest population** at 1.2 k (X), 2.2 k (T) and 34 k (SB) new Defs. 82–96 % of anonymous callables sit in a call-argument slot.
  - It also unlocks the anonymous half of gaps 3 and 4: 1,128 (T) and 894 (SB) bare-arrow formals, and 37 (SB) rest formals.
  - Expect the RD/perf risk (R6) to be real: there are ~4.5 k (X) and ~5.3 k (T) new DFG passes.
- **PR-C (member-only) has the most edges.** It adds 3.2 k (X), 7.9 k (T) and 64 k (SB) projected projection edges for named owners alone, plus about a third more once PR-B lands.
  - On T, 1,041 of the 1,781 named bare-arrow formals are member-only. They stay refused after PR-A, because PR-A does not relax the member-only guard.

## R1 hidden-source admission correction
`[MEASURED]` `probes/census.cjs` now skips hidden **directories** and admits supported hidden regular files. The `.sample.js` regression changes 0 files on the committed census to 1 on the repaired census. Captures are `repair-r1/census-{X,T,secbench}.json`.

| Corpus | Files | Callables | Named | Anonymous | Bare arrows | Projected PR-A Defs / Use rows (bare + rest) |
|---|---:|---:|---:|---:|---:|---|
| X | 628 | 8,614 | 4,117 | 4,497 | 4 | 33 / 38 |
| T | 707 | 21,011 | 15,719 | 5,292 | 3,556 | 846 / 1,015 |
| SecBench | 25,339 | 199,146 | 129,644 | 69,502 | 1,845 | 744 / 973 |

X's hidden lint-staged file adds two bare arrows and one anonymous owner. It adds no projected PR-A credit: the outer formal is member-only and the inner owner is anonymous. T/SecBench aggregates are unchanged. These counts are projections before actual admission/refusal, not exact row budgets or complete loader parity. Binding ambiguity, E7's property keys and PR-C's member-only proof still affect yield. The R1 D11/D12 cost is measured with byte identities in the repaired measurement record.
