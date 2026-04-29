# STATUS: Prism CWE Coverage Phase 3 - JavaScript / TypeScript

**Date:** 2026-04-28
**From:** Prism agent
**To:** agent-eval team
**Re:** Phase 3 JS/TS CWE coverage implementation
**Status:** Implementation branch in progress; awaiting eval-team C3 fixture validation.

## TL;DR

Phase 3 adds JavaScript, TypeScript, and TSX coverage for CWE-79, CWE-89, CWE-918, CWE-502, CWE-78, and CWE-22 across NestJS, Fastify, Express, and Koa.

The implementation reuses the Phase 1/2.5 guardrails: target-scoped framework seeds, per-arg sink taint, per-call flat suppression ranges, explicit structured safe forms, and conservative sanitizer recognition.

The initial in-tree JS/TS suppression suite reports 6/6 sanitized fixtures suppressed and 6/6 unsanitized fixtures detected.

## What Shipped

| Layer | What |
|---|---|
| Framework detection | Registered JS/TS framework detectors for NestJS, Fastify, Express, and Koa in specific-to-general order. |
| Sources | NestJS parameter decorators (`@Body`, `@Query`, `@Param`, `@Headers`, `@Req`, `@Request`), Fastify `(request, reply)`, Express `(req, res, next)`, and Koa `(ctx, next)` canonical handler params. Assignment targets derived from request/DTO access are target-scoped. |
| Sinks | JS/TS structured registries for XSS, SQLi, SSRF, deserialization/RCE bucket, OS command injection, and path traversal. |
| Eval gap coverage | Added explicit coverage for Mongoose `$where`, Prisma unsafe raw SQL, superagent SSRF, `fs/promises` path sinks, and JS dynamic-code RCE bucket sinks (`eval` / `new Function` / `vm.*`). |
| TSX XSS | `dangerouslySetInnerHTML={{ __html: value }}` is modeled as an attribute-position sink; React text interpolation is not a sink. |
| Safe forms | `JSON.parse` remains non-CWE-502, YAML safe schema suppresses unsafe-load findings only when the second argument is an exact safe schema constant or a single non-spread/non-computed `schema` option with an exact safe constant, parametrized SQL calls suppress, literal-binary `execFile` suppresses the broad flat `execFile` fallback unless the binary is a shell/interpreter wrapper, shell options are enabled or uninspectable, shell options use spread/computed keys or pre-call mutation, or an interpreter argv is uninspectable/eval-shaped, URL allowlist and path-prefix checks suppress fixture-shaped SSRF/path flows only through sink-scoped variable-coupled guards with trusted, non-tainted allow/base proof. |
| Fixtures | `tests/fixtures/sanitizer-suite-js-ts/{sanitized,unsanitized}/` with 6+6 Phase 3 smoke fixtures covering all six CWE families. |

## Acceptance Status

| Criterion | Status |
|---|---|
| One vulnerable positive per CWE family | Pinned by `algo_taint_sink_js_ts` and `integration_cwe_phase3_suppression`. |
| Framework detection without per-run config | Pinned by `frameworks_js_ts`. |
| Framework sources are target-scoped | Pinned by request/DTO assignment-target tests and fixtures. |
| Sanitized suppression rate >=80% | `integration_cwe_phase3_suppression` currently reports 6/6 suppressed. |
| Unsanitized mirrors detected | `integration_cwe_phase3_suppression` currently reports 6/6 detected. |
| Phase 1/2 regressions preserved | Phase 1 and Phase 2 suppression suites remain green. |
| Eval C3 fixture pass | Pending eval-team C3 fixture validation. |

## Intentional Limits

- NestJS DTO taint is coarse variable-scoped, not field-sensitive.
- JS/TS framework detection is import/require gated but still canonical-shape based; arbitrary app/router factories and cross-file route registration are deferred.
- Inline callback source propagation is supported through target-scoped request-data assignments and direct request-param seeds; deeper anonymous-callback interprocedural propagation remains deferred.
- URL allowlist and path-prefix suppression are sink-time, variable-coupled, and guard-direction-aware for the canonical reject-on-fail / allow-on-pass shapes. URL guards require allowlist-shaped trusted receiver names backed by a literal or literal-bound collection; negative names such as `disallowedHosts` / `unsafeHosts`, request-derived collections, and collections mutated directly or through simple aliases with non-literal values fail closed. Path-prefix guards require a trusted literal or literal-bound base prefix that preserves a non-root path boundary (for example, `/uploads/`, not `/uploads` or `/`). Full parser-equivalence proof and parser-disagreement bypass detection remain Phase 3.5+.
- SQL parameterization suppression covers fixture-shaped bind/parameters calls and safe Prisma tagged templates. Tainted SQL fragments inside template tags remain a future precision item.
- `execFile("literal", taintedArgs)` is treated as a safe structured shape for Phase 3 unless shell expansion is explicitly enabled or uninspectable, the literal binary is a shell, or the literal binary is a known interpreter whose argv is uninspectable or contains eval/command flags.
- The in-tree fixture suite is a smoke suite (6+6). Eval-team C3 remains the broader acceptance suite for 10 CVE-shaped fixtures across the four frameworks.

## Phase 3.1 Follow-Up Queue

- Tighten Express/Fastify/Koa framework source detection from import-presence + canonical handler signature to constructor/receiver-aware registration where practical.
- Port the FastAPI-style AST decorator extraction pattern to NestJS decorators instead of relying on text-substring route decorator detection.
- Make JS/TS SSRF receiver narrowing import-aware and model factory-bound clients such as `axios.create(...).get(url)`.
- Revisit stricter guard false-positive cases after C3 validation: non-canonical URL allowlist names (`validHosts`, `APPROVED_DOMAINS`), trusted URL allowlist collections from env/config/imported constants, trusted path bases from env/config/imported constants, and safe interpreter argv shapes with spreads/conditionals.
- Tighten JS/TS alias-edge synthesis to request-data accessors (`body`, `query`, `params`, `headers`, `cookies`, `files`, `url`, etc.) instead of any property access on the request object; this avoids over-tainting server-controlled values such as `req.method`.
- Confirm multi-hop JS/TS alias propagation remains covered by standard DFG or extend alias synthesis beyond direct aliases if real fixtures expose alias-of-alias false negatives.

## Validation Commands

```bash
cargo fmt --check
cargo clippy --all-targets
cargo test --test frameworks_js_ts
cargo test --test algo_taint_sink_js_ts -- --nocapture
cargo test --test integration_cwe_phase3_suppression -- --nocapture
cargo test --test integration_cwe_phase2_suppression -- --nocapture
cargo test --test integration_cwe_phase1_suppression -- --nocapture
cargo test
```
