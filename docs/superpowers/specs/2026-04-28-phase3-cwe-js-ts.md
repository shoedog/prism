# Phase 3 - CWE Coverage: JavaScript / TypeScript

**Status:** Approved for implementation planning.
**Parent:** Eval-team Phase 3 asks in `~/code/agent-eval/analysis/eval-team-aiohttp-fixture-ready-phase3-asks-20260428.md`.
**Scope decision for eval:** six-CWE scope, framework order `[nestjs, fastify, express, koa]`, JSX `__html` as an explicit unsafe sink, coarse NestJS DTO taint, canonical middleware signature source detection.
**Review status:** Eval-team approved the scope decisions and confirmed Prism should implement all six CWEs while eval curates a 10-fixture C3 subset. Prism self-review approved after the §5-§7 tightenings incorporated below.

**Source-of-truth references:**
- Eval-side architectural handoff: `~/code/agent-eval/analysis/prism-cwe-coverage-handoff.md` §10 Q4.
- Eval Phase 3 ask: `~/code/agent-eval/analysis/eval-team-aiohttp-fixture-ready-phase3-asks-20260428.md`.
- Phase 2 Python spec: `docs/superpowers/specs/2026-04-26-phase2-cwe-python.md`.
- Phase 2 status: `STATUS-prism-cwe-phase2.md`.
- Phase 1/1.5 guardrails: per-arg DFG, target-scoped seeds, per-call flat suppression ranges, CFG-aware paired checks.

---

## 1. Scope Confirmation

### Q1 - CWE scope

Decision: **six CWEs for Phase 3 MVP**.

Phase 3 should cover:

| CWE | JS/TS focus |
|---|---|
| CWE-79 | Express/Fastify/Koa/NestJS response rendering and React/TSX `dangerouslySetInnerHTML`. |
| CWE-89 | SQL query construction through common DB clients and ORM raw-query APIs. |
| CWE-918 | Server-side HTTP requests through `fetch`, `axios`, `got`, `node-fetch`, `undici`, and Node `http/https`. |
| CWE-502 | Unsafe deserialization / object materialization APIs that can execute code or instantiate attacker-controlled objects. |
| CWE-78 | Node `child_process` command execution. |
| CWE-22 | Node `fs` / response-file APIs with path traversal risk. |

Rationale: framework detection is shared across all six families, and CWE-22 / CWE-78 JS sink lists are small and well-known. Splitting them into Phase 3.5 would add planning overhead without materially reducing engine work. This also aligns with the original handoff language: "same six-CWE coverage for Node.js ecosystem."

Eval alignment: Prism implements all six CWE families in Phase 3; eval-team C3 may still curate a 10-fixture subset as long as it covers at least one fixture per CWE and at least one per framework through overlap.

### Q2 - Framework set and ordering

Decision: **`[nestjs, fastify, express, koa]`**, specific-to-general.

| Framework | Why in Phase 3 |
|---|---|
| NestJS | TypeScript decorator-heavy shape; exercises reusable decorator AST logic from Phase 2 FastAPI. |
| Fastify | Common modern Node framework; route registration is structured and less ambiguous than generic Express middleware. |
| Express | Most common Node ecosystem target; broad middleware and route-handler shapes. |
| Koa | Middleware-first context object shape; should run after more specific frameworks to avoid broad `ctx` heuristics winning early. |

Ordering mirrors the Phase 1/2 rule: prefer more-specific framework detectors before generic detectors. Express and Koa should not mask NestJS / Fastify when a file imports multiple libraries or includes compatibility wrappers.

Tradeoff: most-common-first ordering would put Express earlier, but that makes mixed-framework files noisier because Express-style receiver and middleware shapes are broader. Specific-first keeps disambiguation cost lower.

### Q3 - Sub-feature decisions

Decision summary:

| Topic | Decision |
|---|---|
| JSX-aware XSS | Model `dangerouslySetInnerHTML={{ __html: tainted }}` as an attribute-position CWE-79 sink. React text children are implicitly escaped and not sinks by default. |
| NestJS DTO taint | Coarse variable-scoped taint for `@Body() dto: CreateDto`; field-sensitive DTO taint is Phase 3.5 unless eval produces a discriminator. |
| Middleware handlers | Treat canonical `(req, res, next)` and `(ctx, next)` handler signatures as request-source scopes when tied to framework route/middleware registration. |

These choices deliberately mirror Phase 2:

- JSX `dangerouslySetInnerHTML` matches the Phase 2 "explicit unsafe primitive is the sink" model for Jinja2 `| safe` / `Markup`.
- Coarse NestJS DTO taint matches Phase 2's coarse Pydantic taint. The source is the DTO variable, not every variable on the signature line.
- Middleware signature broadening matches the post-Phase-2.5 Django `def view(request)` broadening, but must stay framework-context gated.

---

## 2. Problem Statement

Phase 1 shipped Go coverage for CWE-78 and CWE-22. Phase 2 shipped Python coverage for CWE-79, CWE-89, CWE-918, and CWE-502, then Phase 2.5 closed eval-driven gaps. Phase 3 extends the same framework-aware CWE model to JavaScript, TypeScript, and TSX.

The JS/TS work is not just a sink-list addition:

1. **Framework sources are callback/decorator based.** Express/Fastify/Koa route handlers receive request objects through function parameters; NestJS receives sources through parameter decorators like `@Body()`, `@Query()`, and `@Param()`.
2. **TSX XSS sinks are attribute-position sinks.** `dangerouslySetInnerHTML` is already in the flat sink list, but Phase 3 needs structured handling so `__html` is the tainted value, not the entire JSX attribute line.
3. **Node sink APIs often share names with safe APIs.** `exec`, `request`, `send`, `render`, and `open` are broad names in the existing flat list. Structured JS/TS sinks need import-aware matching and per-call flat suppression to avoid expanding false positives.
4. **URL allowlist sanitization needs parser-awareness boundaries.** Phase 2's URL allowlist paired-check can transfer to JS/TS, but parser-disagreement bypasses are not line/AST-granularity issues. The MVP should avoid claiming parser equivalence that it cannot prove.

---

## 3. Goals and Acceptance Criteria

| # | Criterion | Measure |
|---|---|---|
| 1 | One vulnerable positive per covered CWE family | New JS/TS/TSX taint tests across CWE-79, CWE-89, CWE-918, CWE-502, CWE-78, CWE-22. |
| 2 | Framework detection without per-run config | Tests for NestJS, Fastify, Express, and Koa positive/negative detection. |
| 3 | Framework sources are target-scoped | Request/DTO params taint only the bound variable or access path, not unrelated same-line variables. |
| 4 | Structured sinks do not regress Phase 1/2 guardrails | Full `cargo test`; Phase 1 and Phase 2 suppression suites stay green. |
| 5 | JS/TS sanitizer suppression is conservative | New Phase 3 suppression suite reaches >=80% sanitized suppression as the pinned floor, with 100% as the target; unsanitized mirror detection target is 100%. |
| 6 | TSX XSS semantics are pinned | `dangerouslySetInnerHTML.__html` tainted fires; JSX text content with tainted value does not fire by default. |
| 7 | Eval C3 fixture pass | Run against eval-team C3 fixtures after curation; record expected vulnerable/fixed behavior in `STATUS-prism-cwe-phase3.md`. |

Initial in-tree fixture target: 12 sanitized + 12 unsanitized JS/TS fixtures. This should cover six CWE families with at least two framework shapes represented in each direction. Eval C3 may use 10 fixtures; in-tree coverage can be slightly broader to pin engine seams.

---

## 4. Engine Reuse and Deltas

Reusable Phase 1/2 pieces:

- `SinkPattern { call_path, category, tainted_arg_indices, semantic_check }`.
- Target-scoped seeds via `TaintSeed { target: Option<AccessPath> }`.
- Synthetic handler-param FlowPaths for parameters used only through field access.
- Per-arg DFG (`arg_is_tainted_in_path`) and complex-expression conservative recursion.
- Per-call flat suppression ranges. Do not reintroduce PR #73-style line-wide suppression.
- CFG-aware paired-check architecture for branch-sensitive sanitizer proof.

Required Phase 3 additions:

- `src/frameworks/js_ts/{mod,nestjs,fastify,express,koa}.rs` or equivalent module naming. Prefer one JS/TS framework namespace over separate JS and TS trees because handlers and sinks mostly share AST node kinds.
- JS/TS import and require resolution helpers for framework and sink receiver binding:
  - ESM: `import express from "express"`, `import { Controller, Get } from "@nestjs/common"`.
  - CommonJS: `const express = require("express")`, `const { exec } = require("child_process")`.
  - Namespace imports: `import * as cp from "child_process"`, `const cp = require("child_process")`.
- Decorator traversal for TypeScript/NestJS:
  - Class decorators: `@Controller("/base")`.
  - Method decorators: `@Get`, `@Post`, `@Put`, `@Delete`, `@Patch`, `@All`.
  - Parameter decorators: `@Body()`, `@Query()`, `@Param()`, `@Headers()`, `@Req()`, `@Request()`.
- Function/callback extraction for route registration:
  - Express: `app.get(path, handler)`, `router.post(path, handler)`, `app.use(handler)`.
  - Fastify: `fastify.get(path, handler)`, `fastify.route({ method, url, handler })`.
  - Koa: `router.get(path, handler)`, `app.use(async (ctx, next) => ...)`.
- Structured TSX sink selector for `dangerouslySetInnerHTML`.
- JS/TS literal helpers for strings, template strings, and static object keys where semantic checks depend on them.

TSX `dangerouslySetInnerHTML` extraction sketch:

1. Find a `jsx_attribute` whose `name` field is `dangerouslySetInnerHTML`.
2. Enter the outer `jsx_expression` value.
3. Enter the inner object literal / object expression.
4. Find a property/pair whose key is `__html`.
5. Evaluate taint on that value expression, not on the whole JSX attribute line.

This is the one Phase 3 sink selector that is not a normal call-expression matcher, so it should have dedicated AST tests.

---

## 5. Framework Source Model

### 5.1 NestJS

Detection:

- Import from `@nestjs/common`.
- `@Controller(...)` class decorator plus method route decorators.
- Prefer decorator AST, not substring matching.

Sources:

- NestJS sources are **decorator-driven and type-agnostic**. A parameter decorator is the source signal; the TypeScript type annotation is informational only.
- `@Body() body: CreateDto` -> coarse target-scoped seed for `body`.
- `@Body("field") field: string` -> target-scoped seed for `field`.
- `@Query() query`, `@Query("q") q`, `@Param() params`, `@Param("id") id`, `@Headers() headers` -> target-scoped seeds.
- `@Req() req` / `@Request() req` -> request object seed; `req.body`, `req.query`, `req.params`, and `req.headers` taint via field access.

Limit:

- DTO field sensitivity is deferred. `body.safeField` and `body.userField` are both tainted if `body` is request-bound.
- No class-based DTO discovery pass is required for MVP. Unlike Pydantic, NestJS DTOs do not have a reliable base-class signal.

### 5.2 Fastify

Detection:

- Import/require `fastify`.
- Receiver binding from `fastify()` or registered plugin route object.

Sources:

- Handler `(request, reply)` seeds `request`.
- `request.body`, `request.query`, `request.params`, and `request.headers` are tainted field accesses.
- Route object handlers (`fastify.route({ handler(request, reply) { ... } })`) count.

### 5.3 Express

Detection:

- Import/require `express`.
- Receiver binding from `express()` or `express.Router()`.
- Route/middleware registration on bound receivers.

Sources:

- Handler `(req, res)` and middleware `(req, res, next)` seed `req`.
- `req.body`, `req.query`, `req.params`, `req.headers`, and `req.cookies` are tainted field accesses.
- Avoid file-wide `req.*` source seeding. Route/middleware context is required.
- Exported canonical middleware/handler functions count even if registration occurs in another file: `module.exports = function(req, res, next) { ... }`, `export function handler(req, res, next) { ... }`, and `export default (req, res, next) => { ... }`. Non-exported helpers still need local route/middleware registration.

### 5.4 Koa

Detection:

- Import/require `koa` and optionally `@koa/router` / `koa-router`.
- Receiver binding from `new Koa()` and router constructors.

Sources:

- Middleware `(ctx, next)` seeds `ctx`.
- `ctx.request.body`, `ctx.query`, `ctx.params`, `ctx.headers`, and `ctx.cookies` are tainted field accesses.
- Koa must run after Express/Fastify/NestJS to avoid broad `ctx`/middleware heuristics taking precedence.

---

## 6. Sink Model

### 6.1 CWE-79 XSS

Structured sinks:

- React/TSX `dangerouslySetInnerHTML={{ __html: value }}` where the `__html` expression is tainted.
- Direct DOM sinks: `element.innerHTML = value`, `element.outerHTML = value`, `element.insertAdjacentHTML(position, value)`.
- Express/Koa/Fastify response APIs that send unsafe HTML strings:
  - `res.send(value)` / `reply.send(value)` are fixture-driven for MVP, not broad XSS sinks.
  - Literal or template arg containing `<` / an HTML-looking tag is HTML context; fire if the HTML expression is tainted.
  - Object literal arg (`res.send({ name })`) is JSON context; do not fire as XSS.
  - Bare identifier arg (`res.send(body)`) is ambiguous; MVP fails open and does not fire XSS unless fixture evidence adds a stronger HTML-context signal.
  - Template engines with explicit unsafe bypasses should be fixture-driven; do not broadly mark every `render` call as XSS.

Sanitizers/safe forms:

- React text interpolation is escaped by default: `<div>{tainted}</div>` is not a sink.
- Common encoders: `escape-html`, `he.encode`, `lodash.escape`, framework-provided escape helpers.
- Do not mark `dangerouslySetInnerHTML` safe because an encoder appears elsewhere in the function; require value coupling.

### 6.2 CWE-89 SQL injection

Structured sinks:

- `client.query(sql)`, `pool.query(sql)`, `connection.query(sql)`.
- `db.execute(sql)`, `connection.execute(sql)`, and similar mysql2 / postgres-js execute APIs.
- `sequelize.query(sql)`, `Sequelize.literal(sql)`.
- `knex.raw(sql)`.
- TypeORM raw query APIs: `repository.query(sql)`, `manager.query(sql)`.
- Mongoose JavaScript execution sink: `Model.$where(js_string)` / `query.$where(js_string)`.
- Prisma raw SQL APIs: `$queryRaw`, `$executeRaw`, `$queryRawUnsafe`, `$executeRawUnsafe`.

Safe forms:

- Parameter arrays/objects: `client.query("... WHERE id = $1", [id])`.
- Tagged template / ORM builder APIs should be treated as safe only when the API is known to parameterize simple value interpolations. Unknown template tags fail open to "not a sanitizer" rather than suppressing.
- Prisma tagged templates are modeled explicitly: simple value interpolation can suppress when Prism can prove parameterization, but tainted SQL fragments, `Prisma.raw(tainted)`, and `*Unsafe` string APIs fire.

### 6.3 CWE-918 SSRF

Structured sinks:

- `fetch(url)`, `nodeFetch(url)`, `undici.fetch(url)`.
- `axios.get(url)`, `axios.post(url, ...)`, `axios({ url })`, `axios.request({ url })`.
- `got(url)`, `got.get(url)`, `got.post(url)`.
- `superagent.get(url)`, `superagent.post(url)`, and chained `superagent(method, url)` forms when URL position is identifiable.
- `http.get(url)`, `https.get(url)`, `http.request(url)`, `https.request(url)`.

Sanitizers:

- Reuse CFG-aware paired-check architecture for URL allowlists:
  - `new URL(url).hostname` / `.host` checked against allowlist.
  - Reject private/loopback/link-local IPs when parsed through a single parser.
- Fail closed if URL parser usage cannot be coupled to the sink argument.

Parser-disagreement note:

- MVP does **not** detect semantic disagreement between JS URL parsers (`new URL`, `url.parse`, WHATWG URL, library-specific parsers). Document as Phase 3+ "parser-disagreement suspicious pattern": validation parser and request parser differ on the same flow path.
- Phase 3.5 candidate: flag flows where validation uses one URL parser and the request library uses a different parser on the same tainted URL. This is an advisory pattern, not a Phase 3 sanitizer proof.

### 6.4 CWE-502 Deserialization

Structured sinks:

- `node-serialize.unserialize(value)`.
- `serialize-javascript` unsafe deserialization patterns if fixture-backed.
- `js-yaml.load(value)` where unsafe schema/options are used; safe schema variants suppress.
- `v8.deserialize(value)` where attacker-controlled data reaches deserialization.
- Handoff-aligned JS RCE bucket: `eval(code)`, `new Function(code)`, `vm.runInNewContext(code, ...)`, `vm.runInThisContext(code)`, `vm.runInContext(code, ...)`, and `new vm.Script(code)`. These are technically code-execution APIs rather than deserializers, but eval-side handoff §2.4 groups them with CWE-502 for JS/TS fixture coverage. Model them explicitly rather than relying only on the broad flat `eval` / `Function` fallback.

Explicit non-sinks:

- `JSON.parse(value)` is not CWE-502 and must not be reintroduced as a broad `parse`/`load` fallback.

### 6.5 CWE-78 OS command injection

Structured sinks:

- `child_process.exec(command)`.
- `child_process.execSync(command)`.
- `spawn(command, args, { shell: true })` and `spawnSync(..., { shell: true })`.
- `execFile(file, args)` only when file or args are tainted and the semantic shape is risky; literal binary with tainted array args may be lower confidence unless shell expansion is enabled.

Safe/precision rules:

- Keep Phase 1's lesson: literal binary does not prove the whole call safe if tainted command text appears in a shell-interpreted argument.
- Prefer structured child_process import/require matching over broad flat `exec` substring matching.

### 6.6 CWE-22 Path traversal

Structured sinks:

- `fs.readFile`, `fs.readFileSync`, `fs.writeFile`, `fs.writeFileSync`, `fs.createReadStream`, `fs.createWriteStream`, `fs.unlink`, `fs.rm`, `fs.rename`.
- `fs.promises.readFile`, `fs.promises.writeFile`, `fs.promises.createReadStream` when present, `fs.promises.unlink`, `fs.promises.rm`, `fs.promises.rename`.
- Destructured `fs/promises` imports: `import { readFile } from "fs/promises"` and `const { readFile } = require("fs/promises")` should resolve to the corresponding path sink.
- Express `res.sendFile(path)` / `res.download(path)`.
- Koa/Fastify static file helpers only when fixture-backed.

Sanitizers:

- `path.resolve(base, input)` / `path.normalize(input)` plus `startsWith(base)` paired checks.
- `path.relative(base, candidate)` plus reject-on-escape checks: `rel.startsWith("..")`, `rel === ".."`, or `path.isAbsolute(rel)`.
- `fs.realpath` / `fs.promises.realpath` plus base-prefix check for symlink-aware validation.
- Direct `path.isAbsolute(input)` rejection can contribute to a sanitizer but does not suppress by itself; it must be paired with a base containment check.
- Reuse the CFG-aware paired-check architecture. Bare `normalize` or `resolve` without an allowlist/prefix guard does not suppress.

Supported guard shapes for MVP:

- Reject-on-non-prefix: `if (!resolved.startsWith(BASE)) return error;` suppresses sinks after the rejecting branch.
- Allow-on-prefix: `if (resolved.startsWith(BASE)) { sink(resolved); }` suppresses sinks contained in the allow branch.
- Reject-on-relative-escape: `if (rel.startsWith("..") || path.isAbsolute(rel)) return error;` suppresses sinks after the rejecting branch.
- Pure OR disjunctions of positive bad checks in a rejecting branch are supported. AND conjunctions, mixed nested boolean expressions, and guards whose branch direction cannot be classified fail closed and do not suppress.
- Variable coupling is required: the checked variable must be the same resolved/relative/realpath value that reaches the sink.

---

## 7. Flat Fallback Policy

Phase 3 should keep the existing flat sink list for broad Tier-1 behavior, but JS/TS structured sinks should take precedence when they can prove a call safe or unsafe.

Rules:

- Structured JS/TS sink matching runs before flat identifier emission for the same line.
- Safe structured calls suppress flat fallback only for that call's byte range.
- `SemanticallyExcluded` is not an authoritative negative unless the helper has proven a safe call shape.
- Same-line unrelated sinks must remain visible.
- Avoid adding broad flat entries like `parse`, `load`, `request`, or `send` for CWE-specific semantics; prefer explicit structured call paths.
- Phase 3 does not globally partition the existing `SINK_PATTERNS` array by language. Instead, JS/TS structured handlers get first refusal for modeled call ranges. Language-scoped flat registries are a possible Phase 3.5 cleanup if flat fallback noise becomes material.

---

## 8. Current Scope and Limitations

Intentional MVP limits:

- No field-sensitive NestJS DTO taint.
- No cross-function framework source inference beyond existing DFG/CPG behavior.
- No full JS/TS type tracking for receiver objects. Canonical receiver variables, direct constructors/factories, imports, requires, and route-registration shapes are in scope; arbitrary factory-returned app/router/client instances are deferred.
- No parser-disagreement vulnerability detection in MVP.
- No broad `JSON.parse` CWE-502 sink.
- No claim that every `res.send(tainted)` is XSS; object/JSON response shapes should not fire as HTML XSS.
- No broad file-wide `req.*` / `ctx.*` sources outside registered framework handler context.
- Express exported canonical middleware signatures are in scope, but arbitrary cross-file route-registration inference is not.

---

## 9. Implementation Planning Notes

Recommended branch: `claude/phase3-cwe-js-ts`.

Suggested commit structure for the implementation plan:

1. **Engine + JS/TS AST helpers.** Import/require maps, decorator extraction, route callback extraction, TSX attribute sink extraction, JS string/template literal helper.
2. **Framework detection and source seeds.** NestJS, Fastify, Express, Koa modules and framework tests.
3. **Structured sink registries.** `JS_CWE79_SINKS`, `JS_CWE89_SINKS`, `JS_CWE918_SINKS`, `JS_CWE502_SINKS`, `JS_CWE78_SINKS`, `JS_CWE22_SINKS`.
4. **Sanitizers and sink-time safe shapes.** SQL parametrization, URL allowlists, path prefix checks, HTML encoders, safe YAML schema.
5. **Fixtures, status, and eval pass.** `STATUS-prism-cwe-phase3.md`, suppression fixture suite, eval C3 validation notes.

Implementation planning can proceed now that eval confirmed the six-CWE / 10-fixture-subset alignment. Reconcile the implementation plan against eval-team C3 when it lands, but do not block engine and registry planning on exact fixture filenames.
