# Prism JS/TS/React/Node tracing coverage against the survey space — 2026-10-31

**Decision this serves:** how much of the surveyed defect/CVE space prism traces today, per capability area A–J and per workstream 1–7 (`../../../eval/jsts-workstream-value/surveys/taxonomy.md`), as the input to the plan in issue #604.

**Method.** Read-only. Source read at main `533f061a`; the release binary (`slicing 3.2.0 (533f061a942a)`) was run on hand-written fixtures (`fixtures/`, 13 files: CJS Express app, ESM TS helper, TSX React tree, async shapes, parameter forms) and on two local corpora (express `18e5985`; excalidraw `0642e72`, the lanes' X). Raw outputs are in the private evidence store (not committed); the Appendix lists each with its command and key numbers. Survey weights come from [`survey-cve-luna.md`](../../../eval/jsts-workstream-value/surveys/survey-cve-luna.md) and [`survey-defects-sonnet.md`](../../../eval/jsts-workstream-value/surveys/survey-defects-sonnet.md); lane history from internal lane notes (private) and the merged PR list; today's agent-facing gap analysis (`../gap-analysis/ANALYSIS.md`) is referenced, not repeated. No cargo; caches deleted.

**Claim tags.** *measured* = a number from a prism run or an evidence file; *source-verified* = read at the cited file:line; *probe-verified* = observed on the fixtures in `fixtures/` or on the corpora named above (raw output in the private store); *judgment* = my estimate.

**Measurement context (measured, from the gap analysis and the private `tracing-trial/` store):** injecting prism+LSP evidence into prompts moved agent T2 F1 by +0.018 (p≈0.31); tracing-trial tool access changed nothing beyond noise. Every coverage gain below is therefore argued as recall or precision a reviewer would see in prism's own output (a finding, a witness, a resolved edge), not as agent lift.

---

## 1. Per-area summary

| Area | Grade | One line |
|---|---|---|
| A intra-function | **partial (high)** | Exact/NameOnly-labelled def-use with kill, same-line, loop-carried and capture doubts; one-statement bodies have no CFG and label everything `cfg_incomplete` (X: 26% of labelled edges). |
| B parameter binding | **partial (med)** | Plain, default and parenthesised-arrow formals bind Exact arg→param; destructured formals have no Def, rest and bare-arrow formals have a Def but no call-site binding, and an options-object argument `f({k: v})` never binds. |
| C member flow | **partial (med)** | Field-sensitive reads (`req.body.cmd`), member-only formal projections; dynamic-key reads collapse to the base, dynamic-key writes are not Defs, spread is untracked, no prototype-pollution sink. |
| D cross-module | **partial (high / low)** | ESM relative, barrels, tsconfig `paths`, wrapped exports, top-level CJS destructure resolve Exact (X: 8,100 `import_member`); CJS module-as-function, `m.f()` require, require-in-function, package `exports`, import qualifiers and `new X()` do not. |
| E compiler ownership | **partial (low)** | tsconfig/jsconfig discovery exists for `paths` only; no project ownership; `.mts`/`.cts` not admitted. |
| F React | **partial (low)** | JSX element → component edges (incl. imported, wrapped) and `dangerouslySetInnerHTML` sink; no attribute→prop data flow, no `useState`/`useContext`/effect provenance, destructured props have no Def. |
| G Node runtime | **partial (med)** | Express/Koa/Fastify/NestJS handler sources, `child_process`/`fs`/`eval`/SSRF/deserialisation/SQL sinks; sources need a per-file framework import, no `process.env`/stream/emitter/`require`-loading model, `query` substring false positive, open redirect missing (#590). |
| H async/callback | **partial (low)** | Anonymous callables are owners, captured reads flow (NameOnly), `await` flows Exact; no callback-argument following (#434), no promise-value → `.then` parameter flow, no middleware `next` chain, no emitter registration. |
| I type-driven | **none (by design)** | TS annotations are used only syntactically (typed receivers, props proofs); no checker in the product. |
| J not a tracing problem | separate | ReDoS, supply chain, config, pure logic: counted apart, never as a gap. |

## 2. Per-area detail

### A. Intra-function data flow — partial (high)
- **Covered.** Reaching-definition labels `Exact | NameOnly(Killed|SameLine|CfgIncomplete|AliasUnstable|CallNameOnly)` (`src/cpg/flow_confidence.rs:15-34`, source-verified); loop-carried subset (`CLAUDE.md`, `dfg-stats`). Fixture: `req` def → `req.body.cmd` use Exact; `const f = await …; readFileSync(f)` Exact (probe-verified on `fixtures/`, `dfg-stats --edges` and `taint-reaches` on `handler2`; raw output in the private store).
- **Partial.** A function whose body is a single statement gets no CFG, so every edge in it is `NameOnly(CfgIncomplete)` via the fallback at `src/cpg/build.rs:1071-1078` (source-verified); seen on six one-statement fixtures (`runCmd`, `readConfig`, `later`, `one`, `d2`, `d3`; probe-verified) and never on a two-statement one. Excalidraw: 3,708 functions without CFG, 21,977 `cfg_incomplete` of 84,773 labelled edges = 26%; express: 1,353 / 2,204 of 7,672 = 29% (measured). Same-line collisions are a labelled doubt (X 4,484), not a loss. Known-gaps §6 lists the member-only LeftFlow residuals (#438).
- **Missing.** Nothing structural; the gap is label precision (the `asserted` tier and `--min-confidence exact` exclude these edges), not recall. Statement-level dependence over MCP is gap 4 of the agent-facing analysis, not repeated here.

### B. Parameter / argument binding — partial (med)
- **Covered.** Plain and default formals: arg→param edge Exact across files (`ts/index.ts:6 req → read.ts:3 opts`, probe-verified); parenthesised arrow (`params2.js:31 → :10`); PR-A Defs for bare-arrow and rest formals (#350); member-only base Def (#390); callback-identity Defs for anonymous formals (#358). Lane js-param-defs: X/T/SB +1,920 rows, 0 checker-wrong (measured, internal lane notes (private)).
- **Partial.** Rest and bare-arrow formals have a Def but **no call-site binding** (`d3(...args)`, `const d4b = x => …`: no `use → def` edge, `taint-reaches` NotReached; probe-verified). An **options-object argument `f({ name: tainted })` never binds** to the callee parameter, while `f(req.query)` does (`index.ts:5/7` NotReached vs `:6/:8` Reached; probe-verified). This is the library-entry shape the CVE survey names ("exported function's argument or options object").
- **Missing.** Destructured formals have no Def at all (`d1({ cmd })`, `Html({ html })`; known-gaps §6 "destructured formals … not handled"; probe-verified). Excalidraw census: object-pattern formals 355/4,600 = 7.7%, 296 in TSX components (measured, synthesis).

### C. Member / property flow — partial (med)
- **Covered.** `AccessPath` field paths with `?.` and `[]` sentinel (`src/access_path.rs:32,153`); per-path projection edges from member-only formals (#390, cache v118+); `a.cmd` → local → sink traced (`d6`, probe-verified).
- **Partial.** `source[key]` reads collapse to base `source` (fixture `merge.js`, probe-verified: paths have `fields: []`); `target[key] = source[key]` is a Use of `target`, not a Def of a path, so a merge loop carries no write provenance (probe-verified). Spread provenance untracked (`language-analysis-gaps.md` gap 6, deferred). Tier-A Node member recall 0/84, TS member recall 15.9% (measured, `docs/eval/jsts-workstream-value/README.md`).
- **Missing.** No prototype-pollution sink or key check: `grep __proto__|prototype` over `src/algorithms/taint.rs`, `src/frameworks/` finds nothing (source-verified). The class is 11.3% of the 895-set and 327 SecBench items.

### D. Cross-module resolution — partial (high for ESM/TS, low for CJS apps)
- **Covered.** ESM relative imports incl. `.js`→`.ts`, bare dots, barrels and `export *` conflict refusal (#368, S1b), wrapped exports (#325/#326), const-alias following (#372), returned-value following (#382), tsconfig `paths` (#338/#342: X 3,121 Exact, private-corpus 2,942 correct; measured), top-level `const { f } = require('./x')` (`named.js:5 → cmd.js:3` witness, probe-verified). Excalidraw: `import_member` 8,100 Exact + 520 NameOnly of 44,963 sites (measured).
- **Partial.** `const merge = require('./lib/merge')` (module-as-function, `module.exports = merge`) → `UnknownName`; `const m = require('./lib/cmd'); m.runCmd()` → `UnknownName` (probe-verified). The same CJS destructure **inside an anonymous Express handler** drops `LocalBindingUnproven` (`app.js:6`; the A2 barrier excludes the CommonJS authority route, known-gaps "Anonymous callers"; probe-verified). Namespace stem fallback gives wrong Exact (#577, #371 item 1). Express corpus: 4,236 `LocalBindingUnproven`, 1,818 unproven aliases, only 300 resolved sites of 11,482 (measured; most are external packages, but the lib's own `require('./router')` shapes are in there).
- **Missing.** Bare specifiers beyond `paths`: workspace packages and `exports` (PKG, parked #352); `require`/`await import()` inside functions (#436); imported class/object qualifiers `C.m()`, `new C()` (S2, parked #353; X +132 was the last positive state); `new X()` not recorded (#437); `import x = N.y`.

### E. Real-project compiler ownership — partial (low)
- **Covered.** Nearest `tsconfig.json`/`jsconfig.json` discovery with `extends` and per-package configs for `paths` (`src/js_paths.rs:277-286`, source-verified); experimental `--owner-config`/`--owner-compiler` proofs (`src/cli.rs:377`).
- **Missing.** Project ownership of a file, mixed JS/TS admission rules, `references`; `.mts`/`.cts` are not in the extension map (`src/languages/mod.rs:35-37` lists `js|mjs|cjs|jsx`, `ts`, `tsx`; source-verified). Owner census capped at 512 inputs; real roots refused (synthesis). 7 SecBench conversions needed dist/build admission (measured).

### F. React-specific provenance — partial (low)
- **Covered.** `<Html />` resolves Exact to the imported component, `<Child />` to the local one, intrinsics dropped as `JsxIntrinsic` (probe-verified on `fixtures/`, `callees` and `call-stats --dump-sites`; raw output in the private store); wrapped exports (`memo`/`forwardRef`) from JSX sites; imported props-type receiver proofs (`src/js_ts_props.rs`); hook results followed one step (#382: X `returned_proven` 115, measured); `dangerouslySetInnerHTML` + `__html` sink (`src/algorithms/taint.rs:928,8526-8558`).
- **Partial.** `useState` destructure gives `text` a Def; captured `setText` in an effect is a NameOnly capture read (probe-verified). No attribute→prop binding: `<Html2 html={text} />` → `props.html` NotReached even with a non-destructured formal (probe-verified). Known-gaps §4: `useContext`, `useMemo` objects, two-hook chains not followed; `React.lazy` binds the loader arrow (wrong).
- **Missing.** Provider→consumer context edges (`Child` gets nothing from `Ctx.Provider`, probe-verified), effect dependency arrays, `href`/`src` `javascript:` sink (`grep href` in taint.rs: none; only a generic `redirect` bare pattern at `taint.rs:46`), Server Actions. The 2026-04 React plan's layers 3–6 (`jsx-tsx-react-plan.md`) were never built (source-verified: no `react_hooks.rs`).

### G. Node runtime — partial (med)
- **Covered.** Framework detection per file (`src/frameworks/js_ts/{express,koa,fastify,nestjs}.rs`), `req`/`request` first-param sources for detected handlers (`taint.rs:1906-1916`), Express route-registration caller edges for bare-identifier handlers (`src/framework_entries/express.rs`); sinks: `exec/execSync/spawn(shell:true)/spawnSync`, `readFile*/writeFile*`, `eval`/`Function`, `fetch/axios/got/get/post/request` (SSRF, with module-binding checks `taint.rs:7816-8013`), `yaml.load` unsafe schema, `unserialize/deserialize`, SQL `query/execute/raw` (`taint.rs:920-1135`, source-verified). Fixture: handler `req.query.dir` → `exec` finding at `app.js:16` (probe-verified).
- **Partial.** Sources exist only in a file that imports the framework: `named.js` (handler in a helper module) gets no `req` source; `--taint-source` is the workaround (probe-verified). Express entries record 0 on the express repo itself (`site.index` member handlers, inline arrows; measured). `query` bare pattern fires on `req.query` reads (`index.ts:8` reported as its own sink; probe-verified). The review CLI's `taint` finding list on the fixtures stayed inside the diff file while `nav taint-reaches` produced the cross-file witness (`index.ts:8 → read2.ts:5`; probe-verified, mechanism not investigated).
- **Missing.** `process.env`, `http.IncomingMessage` outside detected handlers, streams/pipes, `EventEmitter`, `JSON.parse`-of-body as a source, `require` of user paths, `res.redirect` (#590 approved, unscheduled), `path.join` without a root check as a traversal pattern (the `sanitizers/path.rs` recogniser exists; the "no root constraint" rule does not).

### H. Async / callback / higher-order — partial (low)
- **Covered.** Anonymous callables own their calls (`<anon@L:C>`, #406/#409); callback formals get Defs (#358); a capture read inside a deferred callback flows as `NameOnly(CfgIncomplete)` and `taint-reaches` crosses it (`async.js:5 → :6` Reached; probe-verified); `await` flows Exact; the reasoning layer flags `await`, `yield`, callbacks and `.then()` as unmodelled so `NotReached` is not read as proof (`src/languages/mod.rs:1028-1062`, source-verified).
- **Partial.** Callback *invocation* is unresolved: `later(cb) { setTimeout(cb) }` has no edge to the arrow, `cb()` through a parameter drops `UnknownName` (#434; census: T 232 / X 13 named NameOnly targets, +~125/~70 anonymous; measured). `.then((f) => …)` gets a Def for `f` but no flow from the promise value (probe-verified).
- **Missing.** Express middleware `next()` chain and `req` mutation across middleware; emitter `on(name, cb)` registration; promise rejection paths; `this` in arrows (#425: X 162 sites, 0 Exact; measured).

### I. Type-level — none, by design
- TS annotations feed receiver typing syntactically (`typed_param` 81 Exact, `field_typed` 33 on X; measured) and props proofs. There is no checker; `any` leakage, narrowing and generics are out of model. The pinned TS 5.9.3 checker exists only as an eval oracle (SecBench, `--owner-compiler`). Options are in the plan (issue #604).

### J. Not a tracing problem — separate
- ReDoS, supply chain, build/config, single-expression logic. `primitive_slice` covers some crypto/secret fingerprints; nothing here is counted as a gap.

---

## 3. Class mapping and weighted coverage

**Method.** Each survey class is counted once with its own prevalence weight (CVE: Kluban 895-set shares; defects: arXiv:2601.21186 bucket shares). Area grades are the numeric form of §2 (A .75, B .50, C .45, D .60, E .20, F .25, G .50, H .30, I .05). Two scores per class: **primary** = the grade of its decisive area (the surveys' exclusive attribution), and **chain** = the minimum grade over the first three listed areas, J excluded, because a source→sink trace fails at its weakest required hop. Buckets on chain: covered ≥ .65, partial .30–.65, uncovered < .30. J-primary classes are listed but excluded from the denominator. Splits inside a survey bucket (XSS 70/30 server/React, Tooling/Config 72/28 J/E, API misuse 70/30 B-C/H, Missing case 50/50) are judgments and are the main sensitivity. Script: `coverage_calc.py` (`python3 -I coverage_calc.py` reproduces the tables below).

### CVE space (895-set weights; ~22% unclassified and malicious packages excluded)

| Class | Weight | Areas (primary first) | WS | Primary | Chain | Bucket |
|---|---:|---|---|---:|---:|---|
| XSS (server/template) | 17.8 | A/G/B/D/H | 6 | 0.75 | 0.50 | partial |
| XSS (React-specific) | 7.6 | F/A/B/D/H | 5 | 0.25 | 0.25 | uncovered |
| Code/command injection | 18.7 | G/A/B/D/H | 6 | 0.50 | 0.50 | partial |
| ReDoS | 13.5 | J | — | n/a | n/a | J |
| Prototype pollution | 11.3 | C/B/D/A/G | 3 | 0.45 | 0.45 | partial |
| Path traversal | 4.7 | G/B/A/D/H | 6 | 0.50 | 0.50 | partial |
| Auth / logic | 2.2 | A/B/D/H/G | 2 | 0.75 | 0.50 | partial |
| SSRF | 1.9 | G/H/A/B/D | 6 | 0.50 | 0.30 | partial |

Tracing-relevant weight 64.3 (J 13.5 apart). **Weighted coverage: primary 54%, chain 46%. By chain bucket: covered 0%, partial 88%, uncovered 12%.** Nothing in the CVE space is "covered" end to end today; almost everything is traceable in part, and the React-XSS slice is not.

### Defect space (TS-ecosystem 633-bug weights)

| Class | Weight | Areas | WS | Primary | Chain | Bucket |
|---|---:|---|---|---:|---:|---|
| Tooling/Config (J part) | 20.0 | J | — | n/a | n/a | J |
| Tooling/Config (E part) | 7.8 | E/D | 4 | 0.20 | 0.20 | uncovered |
| API misuse (B/C part) | 10.1 | B/C/D | 3 | 0.50 | 0.45 | partial |
| API misuse (H part) | 4.3 | H/B/D | 6 | 0.30 | 0.30 | partial |
| UI bug (React) | 12.5 | F/C/H | 5 | 0.25 | 0.25 | uncovered |
| Type error | 12.4 | I/B/C | — | 0.05 | 0.05 | uncovered |
| Logic error / Test fault / Missing feature / Missing case (J half) | 26.6 | J | — | n/a | n/a | J |
| Missing case (A half) | 4.1 | A/B/C | 3 | 0.75 | 0.45 | partial |
| Async/Event | 3.6 | H/G/A | 6 | 0.30 | 0.30 | partial |
| Runtime exception | 1.2 | C/B/D | 3 | 0.45 | 0.45 | partial |
| Error handling | 0.8 | H/A/G | 6 | 0.30 | 0.30 | partial |

Tracing-relevant weight 56.9 (J 46.6 apart; the survey itself says this sample is J-heavy). **Weighted coverage: primary 29%, chain 26%. By chain bucket: covered 0%, partial 43%, uncovered 57%** — of which type errors (12.4, by design) and the E share of Tooling/Config (7.8) are 20 points.

### By workstream

| WS | Scope | Status today | Evidence | CVE share / chain cov. | Defect share / chain cov. |
|---|---|---|---|---:|---:|
| 1 | Exact parameter-token binding | **done** | [synthesis](../../../eval/jsts-workstream-value/surveys/synthesis.md); PR-A/B/C lane | — | — |
| 2 | Parameter forms (B) | **partial** | bare-arrow + rest Defs shipped (#350); destructured, options-object, rest/bare-arrow *binding* missing (probe) | 3% / 50% (B is also a hop in every CVE chain) | folded into 3 |
| 3 | Occurrence + member flow (A, C) | **partial** | member-only Def (#390/#404); dynamic keys, writes, spread, PP sink missing | 18% / 45% | 27% / 45% |
| 4 | Compiler ownership (E) | **not started** | PKG/S2 parked (#352/#353); owner census capped | — | 14% / 20% |
| 5 | React provenance (F) | **partial (low)** | element edges, wrapped exports, hook results one step; props/state/context flow missing | 12% / 25% | 22% / 25% |
| 6 | Node checkpoint (D, G, H) | **partial** | CJS top-level destructure, framework sources/sinks, anonymous owners; CJS module-as-function, require-in-function, promise/callback flow, package exports missing | 67% / 49% | 15% / 30% |
| 7 | Readiness / measurement | **partial** | Tier-A JS quick (#346), SecBench harness (#347; 113/373 eligible traced end to end), alias matrix 1,169 rows, tiered mutgate; no production-readiness review | enabler | enabler |

---

## 4. Probe findings not already in the known-gaps doc or an issue

All probe-verified on `fixtures/` with the 533f061a binary; each needs a corpus count before it becomes a slice.

1. **Options-object argument does not bind** (`f({ k: tainted })` → callee `opts` no edge; `f(req.query)` binds). Area B; blocks library-API entry traces.
2. **Rest and bare-arrow formals have a Def but no call-site binding** (`d3(...args)`, `const g = x => …`). Area B.
3. **JSX attribute → component parameter has no data-flow edge**, destructured or not. Area F/B.
4. **CJS destructured import inside an anonymous handler drops** (`app.post('/', (req,res) => runCmd(req.body.cmd))`, `LocalBindingUnproven`), while the same call from a named function is Exact. Area D/H; the commonest Node app shape.
5. **CJS module-as-function and `m.f()` require shapes are `UnknownName`.** Area D.
6. **One-statement functions have no CFG**, so their edges are all `NameOnly(CfgIncomplete)` (X 26%, express 29% of labelled edges). Area A label precision.
7. **Framework sources are per-file**: a handler defined in a module that does not import express gets no `req` source. Area G.
8. **`query` bare sink pattern fires on `req.query` reads.** Area G precision.
9. **Review-CLI `taint` findings stayed in the diff file** on the fixtures where `nav taint-reaches` found a cross-file witness. Consumer-level; mechanism not investigated (could be sink scope, not descent).
10. **Dynamic-key writes (`t[k] = s[k]`) are Uses, not path Defs.** Area C; the prototype-pollution write.

## 5. Not verified
- Whether finding 9 is scope or descent (the review algorithm internals were not read).
- The exact CFG condition behind finding 6 (`reaching.rs:224 cfg_edges.is_empty()` is the likely site; the builder was not read).
- Survey weights and their splits were not re-derived from the sources; the sensitivity is stated in §3.
- The private React corpus F was not run (owner-only); F numbers quoted are from internal lane notes (private).

---

## Appendix. Probe index

Raw probe outputs (the `fx-*`, `express-*` and `excalidraw-*` files named below) are held in the private evidence store and are not committed. This table is the committed record: each row gives the command and the key numbers this document cites. Commands are abridged; the binary was the release build `slicing 3.2.0 (533f061a942a)`, and every run used a throwaway `--cache-dir` that was deleted afterwards. Fixture line references in this document are against `fixtures/` as committed here.

Corpora: express `18e5985` (local checkout); excalidraw `0642e72` (the lanes' X).

| Raw file (private) | Command | What it shows |
|---|---|---|
| `fx-call-stats.jsonl` | `nav call-stats --repo fixtures --dump-sites` | per-site outcome and drop on the fixtures (CJS drops in an anonymous handler, module-as-function, JSX intrinsics) |
| `fx-dfg-edges.jsonl` | `nav dfg-stats --repo fixtures --edges` | labelled DataFlow edges; arg-to-param edges; `cfg_incomplete` on one-statement bodies; no edge for destructured, rest, bare-arrow or object-literal args |
| `fx-dfg-stats.txt` | `nav dfg-stats --repo fixtures` | counters including `dfg_rd_functions_without_cfg` |
| `fx-callees.txt`, `fx-callers.txt` | `nav callees` / `callers --symbol ...` | JSX element and import edges |
| `fx-taint-reaches.txt`, `fx-taint-reaches-2.txt` | `nav taint-reaches --source f:l --sink f:l` | witness or NotReached per shape (the first file has off-by-one pairs for `params2.js`; the second is correct) |
| `fx-review-taint*.txt`, `fx-review-finding-shape.txt`, `specs/` | `prism --repo fixtures --diff <json spec> --algorithm taint --format json` | review-CLI findings (intra-file on these fixtures; the `query` pattern on `req.query`) |
| `express-call-stats.json`, `express-dfg-stats.json` | `nav call-stats` / `dfg-stats` on express `18e5985` | 11,482 sites, 300 resolved, 4,236 `LocalBindingUnproven`; 29% `cfg_incomplete` (2,204 of 7,672 labelled edges); 1,353 functions without CFG |
| `excalidraw-call-stats.json`, `excalidraw-dfg-stats.json` | `nav call-stats` / `dfg-stats` on excalidraw `0642e72` | kind histogram; 26% `cfg_incomplete` (21,977 of 84,773 labelled edges); 3,708 functions without CFG |
| `coverage_calc.py` | `python3 -I coverage_calc.py` | the weighted roll-up tables in section 3 (reproduces them; prints to stdout) |

### Recipe for the review-CLI diff specs (issue #602)

The 20 raw spec files held 14 distinct specs (the others repeat one with a different run label), each one line long. The spec for line N of fixture file F is:

```json
{"files":[{"file_path":"F","modify_type":"Modified","diff_lines":[N]}]}
```

with `F` relative to `fixtures/` (for example `node/app.js`). The seed list: `node/app.js` 6 and 16; `node/named.js` 5 and 6; `node/async.js` 5 and 11; `node/params2.js` 28, 33 and 34; `react/App.tsx` 5, 6 and 11; `ts/index.ts` 4, 6, 7 and 8. Run each with `prism --repo fixtures --diff <spec.json> --algorithm taint --format json`.
