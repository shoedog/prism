# STATUS: Prism CWE Coverage Phase 2 - Python

**Date:** 2026-04-27
**From:** Prism agent
**To:** agent-eval team
**Re:** Phase 2 Python CWE coverage spec and plan
**Status:** SHIPPED. Awaiting eval-team C2 validation.

## TL;DR

Phase 2 adds Python coverage for CWE-79, CWE-89, CWE-918, and CWE-502 across Flask, Django, DRF, and FastAPI. The implementation reuses the Phase 1/1.5 guardrails: per-arg sink taint, target-scoped framework sources, per-call flat suppression ranges, and CFG-aware paired checks where sink-time proof is needed.

The in-tree Python suppression suite reports 10/10 sanitized fixtures suppressed and 10/10 unsanitized fixtures detected.

## What Shipped

| Layer | What |
|---|---|
| Engine generalization | Language-neutral call collection/sink dispatch, target-scoped taint seeds, synthetic handler-param flow paths, Python-aware flat-range suppression, and Python sanitizer execution. |
| Framework detection | Flask, Django, DRF, and FastAPI modules registered ahead of Go frameworks. FastAPI route decorators are receiver-aware for `FastAPI()` and `APIRouter()` bindings. |
| Sources | Flask request accessors, Django/DRF `request` handler params, FastAPI scalar/request/Pydantic handler params. |
| Sinks | Python CWE-79 (`mark_safe`, `Markup`, `format_html`, unsafe `render_template_string`), CWE-89 (`execute`, `executemany`, `raw`), CWE-918 (`requests`, `httpx`, `urllib`, `urllib3`), CWE-502 (`pickle`, `cloudpickle`, `yaml.load`, `jsonpickle`, `marshal`, `dill`). |
| Sanitizers/safe sinks | HTML escaping recognizers, Jinja2 default autoescape semantics, DB-API parametrized SQL, SQLAlchemy `text(...).bindparams/params`, CFG-aware URL hostname allowlist, `yaml.safe_load`, and `yaml.load(..., Loader=SafeLoader)`. |
| Fixtures | `tests/fixtures/sanitizer-suite-python/{sanitized,unsanitized}/` with 10+10 Python fixtures. |

## Acceptance Status

| Criterion | Status |
|---|---|
| At least one vulnerable example per CWE family | Pinned by `algo_taint_sink_python` and `algo_taxonomy_sanitizers_python`. |
| Sanitized suppression rate >=80% | `integration_cwe_phase2_suppression` currently reports 10/10 suppressed. |
| Unsanitized mirrors detected | `integration_cwe_phase2_suppression` currently reports 10/10 detected. |
| Framework detection without per-run config | Pinned by `frameworks_python_fastapi`, `frameworks_python_flask`, and `frameworks_python_drf_django`. |
| Phase 1/1.5 regressions preserved | Go sink and sanitizer suites remain green. |

## Intentional Limits

- Pydantic is coarse variable-scoped, not field-sensitive.
- External Jinja2 template file parsing is out of scope; inline `render_template_string` semantics are modeled.
- `format_html("literal", tainted)` is safe at the call site; result-cleansing propagation remains Phase 2.5.
- Python source==sink fallback remains Go-only because the Go fallback intentionally skips per-arg DFG for inline framework-source calls. Python coverage relies on target-scoped seeds or normal DFG paths to avoid broad literal-arg false positives.
- Cross-function sanitizer proof beyond existing DFG/CPG behavior remains out of scope.
- FastAPI receiver/decorator detection was upgraded in the Phase 2.5 O1 cleanup to AST-scoped assignment/decorator traversal for canonical, type-annotated, and tuple receiver bindings. Other Python framework detection remains intentionally lean unless eval or real fixtures surface gaps.
- Flask-style `request.*` source seeding is broad across Python files rather than gated to confirmed Flask handler context. Downstream reachability bounds the impact; framework/handler-scoped gating is deferred.
- CWE-502 broad bare `loads` / `load` matching was removed in the Phase 2.5 O3 cleanup. Unsafe deserializers are modeled by explicit qualified call paths; imported aliases such as `from pickle import loads` are not resolved yet.
- CWE-918 does not yet model `aiohttp.ClientSession.{get,post,...}` sinks; add when C2/eval fixtures or real usage require aiohttp coverage.

## Validation Commands

```bash
cargo fmt --check
cargo clippy --all-targets
cargo test --test algo_taint_sink_python -- --nocapture
cargo test --test algo_taxonomy_sanitizers_python -- --nocapture
cargo test --test integration_cwe_phase2_suppression -- --nocapture
cargo test --test integration_cwe_phase1_suppression -- --nocapture
cargo test
```
