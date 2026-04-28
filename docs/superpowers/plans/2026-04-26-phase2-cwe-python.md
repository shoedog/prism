# Phase 2 - Python CWE Coverage Implementation Plan

> **Status:** Implementation plan approved; implemented on `claude/phase2-cwe-python`. Pairs with `docs/superpowers/specs/2026-04-26-phase2-cwe-python.md` committed on main as `261b437`.
>
> **For agentic workers:** this is larger than Phase 1.5 and should be executed as a branch-and-PR series, not directly on main. The execution rhythm is: branch off main -> land preparatory engine generalization -> land Python frameworks/sources -> land Python sinks -> land Python sanitizers/fixtures -> final docs/status -> push + PR.

**Goal:** Add Python CWE-79, CWE-89, CWE-918, and CWE-502 coverage across Flask, Django, DRF, and FastAPI while preserving Phase 1 / Phase 1.5 correctness guardrails: per-arg sink taint, per-call flat suppression, variable-scoped sources, and sink-time validation for paired checks.

**Branch:** `claude/phase2-cwe-python`

**Source-of-truth references:**
- Spec: `docs/superpowers/specs/2026-04-26-phase2-cwe-python.md`.
- Phase 1 Go design: `docs/superpowers/specs/2026-04-25-phase1-cwe-go-design.md`.
- Phase 1.5 per-arg DFG: `docs/superpowers/specs/2026-04-26-phase15-per-arg-dfg.md`.
- CFG-aware paired check: PR #75 / `claude/phase15-cfg-aware-paired-check`, now merged to `main`.
- Eval C2 plan: `~/code/agent-eval/analysis/c2-python-expansion-plan.md` at `be710f9`.

---

## Commit Structure

### Commit 1 - Engine generalization for Python

Purpose: remove Go-only assumptions from source detection, structured sink dispatch, flat fallback suppression, and sanitizer application. No Python framework coverage should be expected to pass before this commit.

Files likely touched:

| File | Change |
|---|---|
| `src/algorithms/taint.rs` | Add target-scoped taint seeds; language-neutral call collection/path extraction; language-neutral structured sink dispatch; Python-ready flat suppression; Python sanitizer execution hook. |
| `src/frameworks/mod.rs` | Add source resolver / source kind extension for handler-param sources; add optional custom sink taint selector if not kept local to `taint.rs`. |
| `src/data_flow.rs` or `src/cpg.rs` | Add helper to produce FlowPaths from target-scoped seeds, including synthetic parameter-source paths when the DFG skipped a base param used only via field access. |
| `src/ast.rs` | Add any generic Python-friendly helpers needed for function params, annotations, decorators, and string literal extraction if they do not fit in `taint.rs`. |
| `tests/algo/taxonomy/taint_sink_go_test.rs` | Rename existing `taint_sink_lang_test.rs` to keep Go coverage isolated before Python tests land. |
| `tests/algo/taxonomy/taint_sink_python_test.rs` | New Python sink/source regression tests. |
| `Cargo.toml` / `tests/integration/coverage_test.rs` | Register the Go/Python split test files if required by the repo's test harness. |

Key implementation requirements:

- Add a seed shape equivalent to:

```rust
struct TaintSeed {
    file: String,
    line: usize,
    target: Option<AccessPath>,
    origin: Origin,
}
```

- Existing diff and IPC sources remain line-scoped with `target: None`.
- Framework handler-param sources use `target: Some(AccessPath::simple("param"))`.
- A target-scoped seed must taint only references whose access-path base matches the seed target. It must not taint unrelated variables on the same line.
- If a handler parameter is used only via field access (`item.filter_field`), the DFG may not have a base `item` parameter def. Synthesize a source `VarLocation` at the handler line and create FlowPath edges to same-function uses whose access-path base is `item`, subject to CFG reachability.
- Generalize `collect_go_calls` / `go_call_path_text` into language-neutral helpers. Go behavior must remain unchanged.
- Generalize `go_sink_outcome` into a structured sink dispatcher that can include Go built-ins, Python built-ins, and active framework sinks.
- Generalize `cleansed_structured_sink_call_ranges` so Python structured calls can suppress flat fallback only inside their own byte range.
- Keep PR #73 guardrail: a semantically excluded structured call is not an authoritative negative unless the Python-specific logic proved the call safe.
- Extend `apply_cleansers` beyond Go. Python recognizers should run for `Xss`, `Sqli`, `Ssrf`, and `Deserialization`; sink-time helpers remain authoritative where category-wide `cleansed_for` is too broad.

Diagnostic tests for Commit 1:

- Target-scoped source: a synthetic handler signature with `item` and `db` on the same line taints `item.field` but does not taint `db`.
- Base-param skipped by field isolation: use a fixture where `item` is only ever field-accessed, never used bare, so the test exercises the `data_flow.rs` parameter-skip case. Example: `def create_item(item: Item): cursor.execute(f"SELECT FROM x WHERE f = {item.filter_field}")`.
- Base-param normal path companion: include one fixture where `item` is used bare as well as through `item.field`, so both synthetic-source and ordinary base-param paths are covered.
- Same-line flat fallback: safe structured call suppression does not hide an unrelated flat sink on the same line.
- Go regression: existing Phase 1 sanitizer suite remains 10/10.

### Commit 2 - Python framework detection and source seeds

Purpose: add framework detection and variable-scoped source generation for Flask, Django, DRF, and FastAPI.

Files likely touched:

| File | Change |
|---|---|
| `src/frameworks/mod.rs` | Register Python framework modules in `ALL_FRAMEWORKS`, preserving Go behavior. |
| `src/frameworks/python/mod.rs` | Python framework module root. |
| `src/frameworks/python/flask.rs` | Flask detection and source resolver. |
| `src/frameworks/python/django.rs` | Django detection and source resolver. |
| `src/frameworks/python/drf.rs` | DRF detection and source resolver. |
| `src/frameworks/python/fastapi.rs` | FastAPI detection and source resolver. |
| `tests/frameworks/python/{flask,django,drf,fastapi,registry}_test.rs` | Framework detection, disambiguation, and negative tests. |
| `Cargo.toml` / `tests/integration/coverage_test.rs` | Add test entries if this repo still requires explicit registration. |

Framework detection requirements:

- Flask: import `flask` plus `Flask(__name__)` corroborating signal.
- Django: import `django` plus view-shaped `def view(request, ...)` or class-based view signal.
- DRF: import `rest_framework` plus `@api_view` or class inheritance by name from `APIView` / `ViewSet`.
- FastAPI: import `fastapi` plus `FastAPI()` / `APIRouter()` receiver binding and route decorators.
- FastAPI route decorators must be receiver-aware. `@app.get` counts only when `app = FastAPI()` or equivalent is bound. `@router.get` counts when `router = APIRouter()`.
- Include `@app.api_route(...)` and common HTTP method decorators (`get`, `post`, `put`, `delete`, `patch`, `head`, `options`).

Source seed requirements:

- Flask request accessors produce source seeds for `request.args`, `request.form`, `request.json`, `request.data`, and equivalent call/result shapes used by C2.
- Django / DRF request objects are handler-param seeds; request attribute/method uses are tainted via target-scoped source propagation.
- FastAPI scalar route/query/header/body params are variable-scoped seeds when the function is route-decorated.
- FastAPI Pydantic body params are coarse variable-scoped seeds (`item`, not `item.field`) per the spec's Option A.
- FastAPI `request: Request` is a variable-scoped seed. `request.body()`, `request.json()`, etc. are tainted through per-arg DFG conservative recursion at sink evaluation, not separate call-result source patterns.

Diagnostic tests for Commit 2:

- `def helper(item: Item)` without a FastAPI route decorator does not create a source.
- `@app.post(...); def handler(item: Item)` creates a source for `item`.
- `def handler(item: Item, db: Session)` does not taint `db` from the handler line.
- `request: Request` reaches `pickle.loads(await request.body())`.
- `@app.get` on an arbitrary non-FastAPI object does not count.
- `router = APIRouter(); @router.post(...)` counts.

### Commit 3 - Python structured sink registries and XSS render semantics

Purpose: add Python sink registries for CWE-79, CWE-89, CWE-918, and CWE-502, with correct XSS autoescape behavior and flat fallback interaction.

Files likely touched:

| File | Change |
|---|---|
| `src/algorithms/taint.rs` | Add `PY_CWE79_SINKS`, `PY_CWE89_SINKS`, `PY_CWE918_SINKS`, `PY_CWE502_SINKS`; add Python semantic checks and custom taint selectors. |
| `src/frameworks/python/*.rs` | Add framework-specific sinks where needed. |
| `tests/algo/taxonomy/taint_sink_python_test.rs` | Python positive/negative sink tests. |

Sink requirements:

- CWE-79:
  - `mark_safe(tainted)` fires.
  - `markupsafe.Markup(tainted)` fires.
  - `format_html(format_string, *args, **kwargs)` fires only when arg0 is tainted. Tainted arg1+ with literal arg0 does not fire because Django escapes those args.
  - `render_template_string` is sink-relevant only when the inline template disables escaping for a tainted value.
  - `Template().render` fires when the template object has `autoescape=False` and receives tainted context.
- CWE-89:
  - `cursor.execute`, `cursor.executemany`, SQLAlchemy `session.execute(text(raw_sql))`, and `Model.objects.raw`.
  - Parametrized forms are handled in Commit 4 as sanitizers / safe semantic shapes.
- CWE-918:
  - `requests.{get,post,put,delete,patch,head,options,request}` with URL at arg0.
  - `urllib.request.urlopen` and `urllib.request.Request` with URL at arg0.
  - `httpx.{get,post,put,delete,patch,head,options}` with URL at arg0.
  - `aiohttp.ClientSession.{get,post,...}`.
  - `urllib3.PoolManager.request(method, url, ...)` with URL at arg1.
- CWE-502:
  - `pickle.{loads,load}`, `cPickle.{loads,load}`, `cloudpickle.{loads,load}`, `yaml.load` without SafeLoader.
  - `jsonpickle.decode`, `marshal.{loads,load}`, `dill.{loads,load}`.

Inline Jinja2 requirements:

- Extract literal Python strings from the template arg. Update `CallSite::literal_arg` or add a Python-aware helper.
- For `{{ x | safe }}`, only keyword `x` is sink-relevant.
- For `{% autoescape false %}`, all context values are sink-relevant.
- `render_template_string("{{ x }}", x=tainted)` must not fire and must not leak through the flat `render_template_string` pattern.
- `render_template_string("{{ x | safe }}", x="literal", y=tainted)` must not fire.
- `render_template_string("{{ x | safe }}", x=tainted)` must fire.
- External template file scanning is out of scope; do not fire solely because `render_template("profile.html", x=tainted)` appears.

Flat fallback requirements:

- Structured Python sinks run before flat identifier emission for the same line.
- If a structured Python call is proved safe, suppress flat matches only inside that call byte range.
- If structured Python says "not modeled", do not suppress flat fallback.
- Preserve same-line unrelated sink behavior.

Diagnostic tests for Commit 3:

- At least one vulnerable positive per CWE family.
- XSS autoescape negative and `| safe` positive.
- Jinja keyword coupling negative.
- Same-line safe render plus unrelated real sink still reports the unrelated sink.
- `format_html` arg0 positive and arg1 negative.

### Commit 4 - Python sanitizers and suppression fixtures

Purpose: add Python sanitizer recognition, sink-time checks where category-wide suppression is unsafe, and the Phase 2 sanitizer integration suite.

Files likely touched:

| File | Change |
|---|---|
| `src/sanitizers/mod.rs` | Register Python sanitizer recognizers or delegate by language. |
| `src/sanitizers/python.rs` or submodules | Python sanitizer recognizers. |
| `src/algorithms/taint.rs` | Python sink-time checks for URL allowlists and SQL parametrization if needed. |
| `tests/algo/taxonomy/sanitizers_test.rs` or `sanitizers_python_test.rs` | Unit tests for sanitizer shapes. |
| `tests/integration/cwe_phase2_suppression_test.rs` | 10 sanitized + 10 unsanitized Python fixtures. |
| `tests/fixtures/sanitizer-suite-python/{sanitized,unsanitized}/` | Fixture suite. |
| `Cargo.toml` / `tests/integration/coverage_test.rs` | Add test entries if required. |

Sanitizer requirements:

- CWE-79:
  - `html.escape`, `markupsafe.escape`, `bleach.clean`, `bleach.linkify`.
  - Jinja2 default autoescape is modeled as safe-render semantics, not as category-wide `cleansed_for`.
  - `format_html("literal", tainted)` is safe at that call site, but do not mark its result as `cleansed_for(Xss)` in Phase 2. Result-cleansing is deferred to Phase 2.5 unless a downstream-propagation fixture appears.
- CWE-89:
  - `cursor.execute("... WHERE x = %s", (value,))` and equivalent param forms suppress.
  - SQLAlchemy `session.execute` semantic check walks arg0's call chain. If arg0 is `text("... :name ...").bindparams(...)` or `text("... :name ...").params(...)` and the literal passed to `text(...)` uses named placeholders, treat it as parametrized and do not fire. Otherwise the raw SQL sink fires.
  - Raw f-string / string concatenation query in arg0 fires.
- CWE-918:
  - `urlparse(url).hostname in ALLOWED_HOSTS` paired check.
  - `ipaddress.ip_address(...).is_private` / `is_loopback` / `is_link_local` paired checks.
  - Generalize the CFG-aware paired-check mechanism from PR #75 using language/category-specific binding collectors and guard classifier callbacks, then plug in Python URL-allowlist classifiers. Do not fork a near-duplicate helper unless the generic shape proves materially worse.
  - Do not use category-wide textual co-occurrence alone; variable coupling and branch direction matter.
- CWE-502:
  - `yaml.safe_load` is not a sink. No cleanser marker is needed for `yaml.safe_load`.
  - `yaml.load(..., Loader=yaml.SafeLoader)` is safe; unsafe loaders or omitted Loader fire.

Suppression fixture suite:

- 10 sanitized fixtures covering HTML escape / autoescape-safe render, SQL parametrization, URL allowlist, and YAML safe variant.
- 10 unsanitized fixtures mirroring the same families.
- Acceptance: sanitized suppression rate >=80%, target 10/10; unsanitized leakage detection 10/10.

Diagnostic tests for Commit 4:

- SQL parametrized positive suppression and unparametrized f-string positive finding.
- URL allowlist safe branch suppresses; inverted guard fires.
- Unrelated URL allowlist variable does not suppress.
- YAML `safe_load` no finding; `yaml.load` finding.
- Go Phase 1 suppression remains 10/10.

### Commit 5 - Status/docs and eval fixture pass

Purpose: finalize docs, status, and optional eval-fixture validation once implementation is green.

Files likely touched:

| File | Change |
|---|---|
| `STATUS-prism-cwe-phase2.md` | New Phase 2 status file modeled on Phase 1 status; eval C2 validation cadence expects this path. |
| `docs/superpowers/specs/2026-04-26-phase2-cwe-python.md` | Update any implementation-discovered nuance. |
| `docs/superpowers/plans/2026-04-26-phase2-cwe-python.md` | Mark status approved/implemented after review. |

Validation requirements:

- `cargo fmt --check`
- `cargo clippy --all-targets`
- `cargo test --test algo_taxonomy_sanitizers -- --nocapture` or Python split equivalent
- `cargo test --test integration_cwe_phase1_suppression -- --nocapture`
- `cargo test --test integration_cwe_phase2_suppression -- --nocapture`
- `cargo test`
- If available locally, run Prism against eval-team's 10 C2 fixtures and record expected fires/suppressions.
- Sanity-check expected test growth: Phase 2 should add roughly 50-60 tests, with a final full-suite count around 1,510-1,520 from the current 1,455 baseline. Treat large deviations as a prompt to check whether a test family was skipped or overcounted.

---

## Pre-flight

- [ ] Confirm branch and base:

```bash
cd /Users/wesleyjinks/code/slicing
git switch main
git pull --ff-only
git log -1 --oneline
git log --oneline | grep -q 261b437 || { echo "spec commit missing"; exit 1; }
git switch -c claude/phase2-cwe-python
```

- [ ] Confirm baseline:

```bash
cargo fmt --check
cargo clippy --all-targets
cargo test
cargo test --test integration_cwe_phase1_suppression -- --nocapture
```

- [ ] Read required spec sections:

```bash
sed -n '/^## 3\./,/^## 5\./p' docs/superpowers/specs/2026-04-26-phase2-cwe-python.md
sed -n '/^## 6\./,/^## 9\./p' docs/superpowers/specs/2026-04-26-phase2-cwe-python.md
```

- [ ] Inspect current Go-specific seams:

```bash
rg -n "collect_go_calls|go_call_path_text|go_sink_outcome|cleansed_structured_sink_call_ranges|apply_cleansers|detect_framework_sources" src/algorithms/taint.rs
```

---

## Review Gates

- Gate 1 after Commit 1: reviewer checks engine generalization before Python registries build on it.
- Gate 2 after Commit 3: reviewer checks XSS render semantics and flat fallback behavior before sanitizer fixtures depend on it.
- Gate 3 before PR push: full verification plus eval C2 fixture pass if fixtures are available.

---

## Phase 2.5 Follow-On Queue

Eval-team C2 validation accepted Phase 2 and surfaced the next Phase 2.5 priorities:

1. **Django `def view(request)` source broadening.** Complete in PR #80. Standalone Django function views with a `request` parameter are modeled as handler source contexts where practical, even without same-file `urlpatterns` / `path()` corroboration.
2. **`format_html` result-cleansing propagation.** Complete in PR #81. Assigned results from literal-format `format_html(...)` are treated as XSS-cleansed at downstream XSS sinks, with tainted format strings still firing.
3. **O2 - Gate Flask `request.*` sources.** Phase 2.5 branch scopes Flask `request.*` source seeding to registered route handlers and target-scoped assignment results, without regressing C2 fixtures.
4. **O4 - aiohttp SSRF sinks.** Add `aiohttp.ClientSession.{get,post,...}` coverage only if eval fixtures or real usage require it.

Already completed: O3 bare `loads` tightening, explicit `cloudpickle` CWE-502 coverage, and `json.loads` negative regression coverage landed in PR #77. O1 AST-based FastAPI receiver/decorator detection landed in PR #78. Multi-line `render_template_string(... | safe ...)` detection landed in PR #79. Django function-view broadening landed in PR #80. `format_html` result-cleansing landed in PR #81.

---

## Explicit Non-goals

- No Python CWE-78 in Phase 2.
- No Python CWE-22 in Phase 2.
- No external Jinja2 template file parsing.
- No field-sensitive Pydantic enumeration.
- No cross-function `format_html` result-cleansing propagation; Phase 2.5 covers the intraprocedural assigned-result shape.
- No cross-function sanitizer validation beyond existing DFG/CPG behavior.
- No tree-walk caching unless profiling shows a real regression.
