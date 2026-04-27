# Phase 2 — CWE Coverage: Python (Flask, Django, DRF, FastAPI)

**Status:** Implemented on `claude/phase2-cwe-python` (2026-04-27). Eval-team has signed off on C2 coverage needs; this spec records Prism-side engine decisions and implementation constraints.
**Parent:** Eval-team C2 plan (`~/code/agent-eval/analysis/c2-python-expansion-plan.md`, committed `be710f9`).
**Source-of-truth references:**
- Eval-side handoff: `~/code/agent-eval/analysis/prism-cwe-coverage-handoff.md` §2.1–§2.4 (Python sink lists), §3.1–§3.4 (sanitizer recognition), §4 (framework-aware source/sink inference), §10 Q4 (Phase 2 phasing)
- Phase 1 design spec: `docs/superpowers/specs/2026-04-25-phase1-cwe-go-design.md` (architectural template; reuse FrameworkSpec / SourcePattern / SinkPattern / SanitizerRecognizer)
- Phase 1 STATUS: `STATUS-prism-cwe-phase1.md` (updated `dfee78c`); per-arg DFG closed via PR #74 (`fbd962a`); per-call cleanser scoping via `9a77d3a`; PowerShell wrappers via `fc9dbe9`

**Scope:** Python source/sink/sanitizer coverage for **CWE-79** (XSS), **CWE-89** (SQLi), **CWE-918** (SSRF), **CWE-502** (deserialization) across **Flask + Django + DRF + FastAPI**. Targets the 10 fixtures enumerated in C2.

**Review callout:** §4 records the four architectural decisions and rejected alternatives. The chosen lean **prefers long-term-correct choices** over MVP shortcuts where the cost gap is reasonable, on the principle that *deferred debt is paid in multiples* (per the Phase 1.5 dual-layer rollback experience). Prism-side review should focus on the engine-generalization requirements in §3 and the flat-fallback / variable-scoped-source guardrails in §4–§8.

---

## 1. Problem statement

Phase 1 shipped Go-only CWE-78 + CWE-22 with framework detection (net/http, gin, gorilla/mux), sanitizer recognition (`filepath.Clean+strings.HasPrefix`, `filepath.Rel+strings.HasPrefix`), and per-arg DFG (PR #74). Phase 2 extends to Python with four new CWE families and four new frameworks. The Python work is structurally similar to Phase 1 but introduces engine concerns that didn't arise for Go:

1. **Pydantic-typed parameters as sources** — FastAPI's `def handler(item: Item)` taints the entire `Item` model from the request body. Whether to track field-level taint (`item.filter_field` tainted distinctly from `item.public_id`) is an architectural question with downstream effects on precision and test expectations.
2. **Anti-sanitizer primitives** — `markupsafe.Markup(value)` promotes a value to "trusted" for Jinja2 template rendering. If the value is tainted, this is a CWE-79 *escalation*, not a sink in the conventional sense. Phase 1 has no anti-sanitizer concept.
3. **Implicit sanitization via Jinja2 autoescape** — Flask's `render_template()` and `render_template_string()` autoescape by default, which means a naive "all template renders are CWE-79 sinks" policy would over-fire massively. The engine needs to model autoescape policy.
4. **Decorator-based source detection** — FastAPI uses `@app.get("/items/{id}")` to bind path parameters; DRF uses `@api_view(['GET'])` and class-based `APIView`/`ViewSet` to mark handler scope. Phase 1's framework detection is import + corroborating-signal based; Phase 2 extends with decorator-AST awareness.

Eval-team's C2 plan documents the WHAT (10 CVE-shaped fixtures, source/sink/sanitizer lists). This spec documents the HOW — engine architecture decisions Prism needs to make before implementation.

---

## 2. Goals + acceptance criteria

Mirrors Phase 1's ACK §1 acceptance pattern.

| # | Criterion | Measure |
|---|---|---|
| 1 | Taint fires on ≥1 vulnerable example for each of CWE-79, CWE-89, CWE-918, CWE-502 | In-tree positive tests in `tests/algo/taxonomy/taint_sink_python_test.rs` and `tests/algo/taxonomy/sanitizers_python_test.rs` |
| 2 | ≥80% sanitizer suppression rate on Python sanitizer fixture suite | `tests/integration/cwe_phase2_suppression_test.rs` modeled on `cwe_phase1_suppression_test.rs`; 10+10 sanitized/unsanitized fixtures in `tests/fixtures/sanitizer-suite-python/` |
| 3 | Framework detection without per-run config for Flask, Django, DRF, FastAPI | `tests/frameworks/python/{flask,django,drf,fastapi,registry}_test.rs` |
| 4 | D2 coexistence — no Prism-side dedup | Same as Phase 1 (no new dedup logic added) |
| 5 | Python precision guards for the new engine seams | Tests prove handler-param taint is variable-scoped, autoescaped renders do not leak through flat fallback, and same-line unrelated sinks still fire |
| 6 | No Tier-1 regression — existing Phase 1 tests pass | Full `cargo test` green; sanitized rate 10/10 unchanged for Go fixtures |

Stretch: validate against eval-team's 10 C2 fixtures (`~/code/agent-eval/cache/prism-cwe-fixtures/cwe-{79,89,918,502}/`) when their curation handoff lands. Their RE-prism-cwe-phase2-status reply is the eval-side gate.

---

## 3. Engine architecture deltas from Phase 1

Most of Phase 1's data model transfers, but Phase 2 is **not registry-only**. Several pieces in the current taint engine are still Go-gated (`detect_framework_sources`, structured sink matching, flat-range suppression, and `apply_cleansers`). Phase 2 must first generalize those seams, then add Python registries.

Reusable Phase 1 pieces:

- `FrameworkSpec { name, detect, sources, sinks, sanitizers }` — reusable with Python-specific detection helpers.
- `SinkPattern { call_path, category, tainted_arg_indices, semantic_check }` — reusable for fixed positional-arg sinks.
- Per-arg DFG (`arg_is_tainted_in_path`) — reusable after call collection / call-path extraction is language-neutral.
- Cleanser-suppression byte-range scoping (`9a77d3a`) — reusable, but must be generalized from Go-only structured sink ranges to Python structured sink ranges too.

Required engine generalization before Python coverage:

- **Variable-scoped taint seeds.** Current taint seeds are `(file, line)` only. That is insufficient for handler parameters because `def handler(item: Item, db: Session)` should taint `item`, not every variable on the signature line. Introduce a seed shape like `TaintSeed { file, line, target: Option<AccessPath>, origin }`. Existing diff/IPc sources can remain line-only (`target: None`); framework sources should emit target-scoped seeds (`item`, `request`, etc.).
- **Framework source detection beyond call-result sources.** `SourcePattern { call_path, origin, taints_arg }` is call-centric. Python frameworks need parameter/annotation/decorator sources (`request`, FastAPI `Path()/Query()` params, Pydantic body models). Add a source variant such as `SourceKind::CallResult | SourceKind::TaintedArg | SourceKind::HandlerParam`.
- **Language-neutral call utilities.** Replace Go-only `collect_go_calls` / `go_call_path_text` usage with helpers that work for Go and Python. Existing Go helpers can become wrappers if that keeps the patch small.
- **Language-neutral structured sink loop.** `line_matches_structured_sink`, `go_sink_outcome`, and `cleansed_structured_sink_call_ranges` currently return `NoMatch` for non-Go files. Phase 2 needs a shared structured-sink dispatcher over cross-cutting Python sinks plus active framework sinks.
- **Custom Python sink taint selectors.** Fixed `tainted_arg_indices` is enough for most SQLi/SSRF/deserialization sinks, but not for `render_template_string("{{ x | safe }}", x=name, y=other)` because the dangerous value is selected by a template variable name / Python keyword arg. Add an optional custom taint predicate or arg selector for Python sinks that cannot be represented by positional indices.
- **Python flat-fallback policy.** Some Python sink names are already in `SINK_PATTERNS` (`render`, `render_template_string`, `execute`, `load(s)`, `mark_safe`, `Markup`). Structured Python logic must be allowed to suppress only the matching safe call range when it proves a call safe (e.g. autoescaped render), without suppressing unrelated same-line sinks. Do not reintroduce PR #73-style line-wide suppression.
- **Python sanitizer application.** `apply_cleansers` currently returns for non-Go files. It must run Python sanitizer recognizers for `Xss`, `Sqli`, `Ssrf`, and `Deserialization`, with sink-time helpers for paired checks where category-wide `cleansed_for` would be too broad.
- **Python literal helpers.** `CallSite::literal_arg` currently recognizes Go string node kinds. Add language-aware string literal extraction for Python string / interpolated string nodes before relying on inline Jinja2 template scans or SQL placeholder checks.

New Python-specific pieces:

- `src/frameworks/python/{mod,flask,django,drf,fastapi}.rs` — analogous to `src/frameworks/{nethttp,gin,gorilla_mux}.rs`, registered in `ALL_FRAMEWORKS` after deciding disambiguation order.
- New helper(s) for decorator-AST traversal: `python_decorator_call_paths(node)` returns the list of decorator call_paths on a function. Used by FastAPI and DRF route detection.
- Python-specific per-CWE sink lists: `PY_CWE79_SINKS`, `PY_CWE89_SINKS`, `PY_CWE918_SINKS`, `PY_CWE502_SINKS`.
- Python sanitizer recognizers: `python_html_escape`, `python_sql_parametrize`, `python_url_allowlist`, `python_yaml_safe_load`. Categories already exist in `SanitizerCategory` (`Xss`, `Sqli`, `Ssrf`, `Deserialization`) but naming should stay consistent with the Rust enum (`Sqli`, not `SqlInjection`).

Tree-sitter Python grammar is already integrated (`tree-sitter-python` in `Cargo.toml` per CLAUDE.md). No new grammar work.

---

## 4. Architectural decisions

Each question has options with explicit pros/cons/uncertainties/costs. The lean is the Phase 2 implementation assumption unless a reviewer reopens it.

### 4.1 Pydantic-typed parameter source taint — coarse vs field-sensitive

**The question:** When `def handler(item: Item)` is a FastAPI route handler and `Item` is a Pydantic `BaseModel`, what's tainted?

```python
class Item(BaseModel):
    filter_field: str
    public_id: int
    nested: Metadata

@app.post("/items")
def create_item(item: Item):
    session.execute(text(f"WHERE x = {item.filter_field}"))
```

#### Option A — Coarse taint (whole model tainted)

Whole `item` is a tainted source. `item.filter_field`, `item.public_id`, `item.nested.x` are treated as user-controlled because their base object is request-bound — but the source seed must be **variable-scoped** to `item`, not merely line-scoped to the handler signature.

Implementation requirement: Option A depends on §3's `TaintSeed { target: Some(AccessPath::simple("item")) }` shape or equivalent. A line-only source at `def create_item(item: Item, db: Session)` is unsafe because it can taint unrelated same-line parameters like `db`. Also, today's DFG intentionally skips base parameter defs when a parameter is only used through field access, so relying on "existing field-resolution" without a scoped seed would miss the canonical `item.filter_field` path.

**Pros:**
- Smaller than field-sensitive Pydantic enumeration. FastAPI detection emits the model parameter base (`item`) as the tainted seed; sink-side arg matching treats descendant field access on that base as tainted.
- Semantically correct for the common case: a request-bound Pydantic model is fully user-controlled (Pydantic validates structure but doesn't sanitize content). Field-by-field analysis doesn't change correctness for canonical request shapes.
- Aligns with how FastAPI handlers are typically used — Pydantic models are request-DTOs; treating any field as untainted is a developer trust assertion the engine can't verify.

**Cons:**
- Over-fires on mixed-trust models. A `class Item(BaseModel): user_input: str; server_set: int` where `server_set` is populated server-side but the model arrives from request would over-fire on `server_set`. Rare but possible.
- Coarse taint propagates to field accesses syntactically — `item.public_id` becomes a tainted use-site even though the field was server-set. False positives in mixed-trust shapes.
- Requires variable-scoped source plumbing in the taint engine. This is smaller than field-sensitive Pydantic, but it is still a structural engine change.

**Uncertainty:**
- How common are mixed-trust Pydantic models in practice? Most real FastAPI endpoints have Pydantic models that are 100% user-controlled (the entire point is request validation). Mixed-trust shapes do exist (e.g., audit fields, server-stamped IDs) but are typically excluded from the request-body model and added server-side after.
- Eval-team's C2 fixtures (#6 FastAPI SQLi, #8 manga-image-translator pickle) — both fully-user-controlled models by inspection. Coarse won't visibly under-perform.

**Cost:**
- Upfront: ~1 day. Add target-scoped taint seed plumbing; add FastAPI detection of Pydantic-bound params via type annotation; emit `item` (base name) as the tainted seed; update arg-taint matching so `item.field` is covered by a tainted base seed.
- Long-term: LOW *if* we keep the seed shape access-path-aware now. Field-sensitive Pydantic later becomes additive (`item.field` seeds) rather than a rewrite of line-only source plumbing.

**Recommendation lean:** Option A for MVP, but only with variable-scoped source seeds. Field-sensitivity at the SOURCE is precision improvement, not C2 correctness, but line-only handler sources are too broad and should not ship.

#### Option B — Field-sensitive taint (per-field tainted via AccessPath)

Each field of a request-bound Pydantic model is tainted as a distinct AccessPath. `item.filter_field` tainted; `item.public_id` tainted; `item.nested.x` tainted (recursively for nested models).

**Pros:**
- Precision: enables future "this field is server-set, not request-derived" annotations to selectively un-taint.
- Aligns architecturally with Phase 1's per-arg DFG (`tainted_arg_indices` is per-arg precision at sink-eval time; per-field source taint is the symmetric precision at source-eval time).
- The existing `access_path.rs` infrastructure structurally supports this — `AccessPath { base, fields }` with field chains; DataFlowGraph indexes by `(file, function, AccessPath)`.

**Cons:**
- Larger upfront cost. Requires (1) Pydantic class-AST traversal to enumerate fields, (2) per-field source emission (one source location per field), (3) field-aware SourcePattern shape, (4) integration of AccessPath into `taint_forward_cfg` if not already present.
- Doesn't change correctness for any C2 fixture. All 10 fixtures have models that are fully user-controlled.
- Phase 1's `arg_node_taints_match` (taint.rs:899) currently matches on `var_name() == name` (base only — field-insensitive at the matching point). Even with field-sensitive sources, the match side would need updating to consult AccessPath, otherwise we'd lose the field info at sink-eval.

**Uncertainty:**
- Does the existing `access_path.rs` integrate cleanly with `taint_forward_cfg`'s reachability machinery? The DataFlowGraph index suggests yes; needs verification with a small probe.
- Pydantic class-AST traversal is non-trivial — class body, type annotations, defaults. Tree-sitter Python parses class bodies, but extracting field names + types requires careful AST walking.

**Cost:**
- Upfront: 2-3 days. Pydantic class enumeration + per-field source emission + access-path matching at sink eval. Plus tests for per-field taint propagation.
- Long-term: ZERO additional refactor for "field-sensitive precision" later (it's already there). MEDIUM debt if the field enumeration logic needs to handle exotic Pydantic features (Field aliases, discriminated unions, computed fields).

**Recommendation lean:** Option A for MVP, with §10 noting field-sensitive sources as a Phase 2.5 hardening item. The cost gap (~1 day vs 2-3 days) is meaningful and the correctness delta on C2 fixtures is zero. **However**, if eval-team or downstream consumers are likely to need field-sensitive Pydantic taint within the next 1-2 phases, Option B is worth doing now to avoid a Phase 2 → Phase 2.5 refactor.

**Decision:** Option A for Phase 2. Reopen only if eval produces a discriminator that needs field-level source precision.

---

### 4.2 Anti-sanitizer encoding — `markupsafe.Markup`

**The question:** `markupsafe.Markup(value)` promotes a value to "trusted" HTML in Jinja2 templates. If `value` is tainted, this is a CWE-79 escalation. How do we model it?

```python
@app.route("/profile")
def profile():
    name = request.args.get("name")
    return render_template_string("Hello {{ greeting }}", greeting=Markup(name))
    # Markup(tainted) bypasses autoescape — XSS
```

Per eval-team's handoff §3.1, `markupsafe.Markup` is listed as *anti-sanitizer*. C2 fixture suite doesn't currently exercise it, but eval-team flagged it as a Phase 2 concern.

#### Option A — Markup as a regular CWE-79 sink

Treat `markupsafe.Markup` as a `SinkPattern` in `PY_CWE79_SINKS`. `Markup(taintedVar)` fires the sink; `Markup("<b>literal HTML</b>")` doesn't fire (per-arg DFG: literal arg has no identifier).

**Pros:**
- Trivial. Add one entry to the CWE-79 sink list. Per-arg DFG (already shipped via PR #74) handles "Markup of literal" vs "Markup of tainted" automatically.
- No new engine machinery. Reuses everything.

**Cons:**
- Conceptually fuzzy: `Markup` isn't a "sink" in the usual sense (data doesn't *exit* the program through it; it just gets a trust upgrade). But functionally it's where the harm crystallizes.
- If a value is Markup'd then passed to `render_template`, the engine fires CWE-79 at the `Markup(...)` line, not at the render. Acceptable — same line ID gets reported either way; the `render_template` call doesn't need to be a separate sink for this case.

**Uncertainty:**
- What about `Markup("<b>%s</b>") % tainted` or `Markup(template_str.format(tainted))`? Per-arg DFG handles these via complex-expression conservative recursion (descends into descendants). If `tainted` is an identifier inside `Markup(...)`'s arg subtree, the sink fires. ✓
- What about `safe_html = Markup("<b>literal</b>"); render_template(... safe_html=safe_html)`? Here Markup is being legitimately used; per-arg DFG won't fire (no tainted identifier inside Markup's arg). ✓

**Cost:**
- Upfront: 1 hour. One sink entry + one positive test + one negative test.
- Long-term: LOW. The per-arg DFG semantics are already in place and known-correct.

**Recommendation lean:** Option A.

#### Option B — Anti-sanitizer category that REMOVES from `cleansed_for`

Introduce `AntiSanitizer { call_path, category, ... }` that, when called on a path, *removes* the matching category from `path.cleansed_for`. So a previously-cleansed value becomes uncleansed if Markup-ified.

**Pros:**
- Semantically pure model: "Markup makes things less safe."
- Composable with future anti-sanitizer patterns (e.g., `urllib.parse.unquote` as an unsafer for path validation).

**Cons:**
- New engine machinery (Anti-cleansers, removal from `cleansed_for`).
- Doesn't actually buy correctness for the canonical Markup-of-tainted case — Option A handles it via per-arg DFG.
- The model conflates two distinct concerns: (1) "Markup makes the value MORE dangerous" (escalation) vs (2) "Markup undoes html-escape" (cleanser-removal). They're different; Option A handles (1) directly.

**Uncertainty:**
- Are there real Python patterns where Markup-after-html.escape needs to be detected? Probably rare — html.escape returns a str, Markup-of-escaped-str is a no-op.

**Cost:**
- Upfront: 2-3 days. New AntiSanitizer type + integration into apply_cleansers + tests.
- Long-term: MEDIUM. Adds a concept that future framework specs might or might not need.

**Recommendation lean:** Option A. Anti-sanitizer machinery is overengineered for the Markup case. Per-arg DFG already gives us the semantic "Markup-of-tainted-fires-sink" for free.

**Decision:** Option A for Phase 2. Reopen anti-sanitizer machinery only if a second concrete pattern appears that cannot be modeled as a sink.

---

### 4.3 Implicit autoescape (Jinja2) — when does `render_template*` cleanse?

**The question:** Flask's `render_template()` and `render_template_string()` autoescape by default. A naive "every `render_template_string` with a tainted context variable is a CWE-79 sink" policy over-fires on every Flask app. How do we model autoescape?

```python
# Case 1: autoescape ON (default), tainted var safely rendered
@app.route("/profile")
def profile_safe():
    name = request.args.get("name")
    return render_template_string("Hello {{ greeting }}", greeting=name)
    # No XSS — Jinja2 autoescape escapes `name`

# Case 2: explicit `| safe` filter — autoescape bypassed
@app.route("/profile")
def profile_unsafe():
    name = request.args.get("name")
    return render_template_string("Hello {{ greeting | safe }}", greeting=name)
    # XSS — `| safe` disables autoescape for this variable

# Case 3: mark_safe directly
@app.route("/profile")
def profile_mark_safe():
    name = request.args.get("name")
    return render_template_string("Hello {{ greeting }}", greeting=mark_safe(name))
    # XSS — mark_safe promotes to trusted
```

C2 fixture #2 (flask-appbuilder) explicitly relies on autoescape behavior — `fixed.py` removes `| safe`; `vulnerable.py` keeps it.

#### Option A — Render as conditional sink: only fires when `| safe` substring or `autoescape=False` detected

`render_template_string` is in `PY_CWE79_SINKS` with a `semantic_check` that:
1. Inline templates (string literal arg): scan the template string for `| safe`, `| Markup`, or `{% autoescape false %}`. If found, fire. Otherwise don't.
2. External templates (string variable arg or path constant): treat conservatively — don't fire unless context variable was explicitly Markup'd or mark_safe'd.

**Pros:**
- Matches actual Jinja2 behavior. No false-positive flood.
- Inline template-string scanning is feasible (the template is a string literal in Python source; tree-sitter exposes it).
- Aligns with C2 fixture #2's expected behavior — `fixed.py` (no `| safe`) suppresses; `vulnerable.py` (`| safe`) fires.

**Cons:**
- External template files (e.g., `templates/profile.html`) are not parseable from Python source alone. Requires either (a) loading the template file from `templates/` directory, or (b) treating external templates conservatively (don't fire — risk of FN).
- The `semantic_check` becomes non-trivial: need a mini Jinja2 parser to identify `| safe` filters in template strings.

**Uncertainty:**
- How accurate is "scan for `| safe` substring" as a heuristic? False positives if `| safe` appears in a comment or string literal inside the template. False negatives if `Markup(...)` is used inside the template via `{{ Markup(x) }}`. A simple substring check is a 95%-good heuristic; full Jinja2 parsing is 99%-good but much more complex.
- Eval-team's C2 fixture #2 uses the canonical inline `render_template_string("{{ provider | safe }}", ...)` shape — substring-scan handles this case directly.

**Cost:**
- Upfront: 1 day. Inline-template substring scan + `semantic_check` for `render_template_string` + test cases.
- Long-term: External template file analysis is a Phase 2.5+ item — substring scan is "good enough" for ~95% of CVE shapes (which use inline templates for compactness anyway).

**Recommendation lean:** Option A.

#### Option B — Render is always a sink; rely on explicit `html.escape` cleanser

Treat all `render_template*` calls with tainted context as CWE-79 sinks. `html.escape(name)` upstream of the render becomes a path-level sanitizer that suppresses.

**Pros:**
- Simpler engine — no template-string parsing.
- Forces developers to use explicit `html.escape` (security via consistency).

**Cons:**
- Doesn't match Jinja2 semantics — autoescape IS the cleanser in Flask. Forcing developers to add `html.escape` everywhere is wrong-by-design.
- Massive false-positive flood on real Flask apps. Suppression rate would be near zero on any realistic codebase.
- C2 fixture #2's `fixed.py` would *still fire* under this option (no explicit `html.escape`). That contradicts eval-team's expected suppression.

**Uncertainty:**
- Could autoescape be inferred from `app = Flask(__name__)` (which sets autoescape on by default) plus `render_template*`? Maybe — but inference is brittle.

**Cost:**
- Upfront: 0.5 day.
- Long-term: HIGH. Will need to be reverted to Option A as soon as real Flask code is tested.

**Recommendation lean:** Strongly avoid Option B. The mismatch with Jinja2 semantics means it can't ship.

#### Option C — Render is conditional, with separate explicit-mark-unsafe sinks

`render_template*` is not a generic "tainted context == XSS" sink. A render call is implicitly safe under default autoescape. The explicit unsafe primitives are what trigger findings: `mark_safe`, `Markup`, `format_html` only when its format string arg is tainted, `Template().render` with `autoescape=False`, and `render_template_string` only when the inline template disables escaping for a tainted value.

Implementation requirement: this option depends on the §3 flat-fallback policy. Existing flat patterns already contain `render`, `render_template_string`, `mark_safe`, and `Markup`; structured Python logic must prevent safe autoescaped renders from leaking through the flat matcher. Suppression must be byte-range/per-call scoped, not line-wide, so unrelated same-line sinks still fire.

**Pros:**
- Matches Jinja2's actual policy: autoescape is the default; opt-in unsafe is the user's choice.
- Sink list is concrete primitives, not "every render is conditionally a sink."
- Inline `{{ x | safe }}` becomes a sink at the `render_template_string("{{ x | safe }}", x=...)` call site, detectable via string-arg scan plus keyword-arg mapping.

**Cons:**
- Misses external template files using `| safe`. Same FN class as Option A's external-template caveat.
- A `render_template_string("{{ x }}", x=mark_safe(tainted))` shape is detected as a `mark_safe` sink at the line where mark_safe is called — but if mark_safe is not on the same line as render, the FlowEdge tracking still gets it (Phase 1's tainted-binary case is the precedent).
- Requires a custom Python sink taint selector. `tainted_arg_indices` alone would over-fire on `render_template_string("{{ x | safe }}", x=safe_literal, y=tainted)` because it cannot know that only keyword `x` is rendered unsafe.

**Uncertainty:**
- Same as Option A: external template handling.
- How much Jinja2 parsing is required for MVP? A targeted extractor for `{{ name | safe }}` and `{% autoescape false %}` is enough for C2. Full Jinja2 AST parsing remains out of scope.

**Cost:**
- Upfront: ~1 day. Inline-template string extraction, unsafe-variable extraction, keyword-arg mapping, and per-call flat suppression for safe renders.
- Long-term: LOW. External-template handling deferred uniformly with Option A.

**Recommendation lean:** Option C is the cleanest. It models reality (autoescape ON, opt-in unsafe primitives are the real sinks) without inventing conditional-sink machinery for `render_template*`.

The sink list becomes: `mark_safe`, `markupsafe.Markup`, `format_html` only when arg0 is tainted, `Template().render` with `autoescape=False` argument, `render_template_string` only when the inline template contains `| safe` or `{% autoescape false %}` and the corresponding unsafe context value is tainted.

**Decision:** Option C for Phase 2. Render is implicitly safe under default autoescape; explicit unsafe primitives and unsafe inline-template constructs are sinks.

---

### 4.4 Decorator-based source detection — FastAPI / DRF

**The question:** Phase 1's framework detection is import + corroborating-signal based (e.g., gin: `import "github.com/gin-gonic/gin"` + `*gin.Context` parameter). Python's FastAPI / DRF lean heavily on decorators (`@app.get("/items")`, `@api_view(['GET'])`). Does Phase 2 need decorator-AST awareness?

#### Option A — Decorator-AST awareness for FastAPI; class-hierarchy + decorator combo for DRF

New helpers:
- `python_decorator_call_paths(func_node)` returns the list of decorator call_paths (e.g., `["app.get"]` for `@app.get("/items")`).
- FastAPI detection: a function with a route-decorator (`@app.get/post/put/delete/patch/head/options`) is a route handler only when the decorator receiver is bound to `FastAPI()` or `APIRouter()` in the file/scope (`app = FastAPI()`, `router = APIRouter()`). Avoid treating arbitrary `app.get` helpers as routes.
- DRF detection: `@api_view([...])` decorator OR class inherits from `APIView`/`ViewSet` (resolved via class-hierarchy walk).
- FastAPI param taint:
  - Path/query/header/body/form/file params with `Annotated[..., Query()]` / `Path()` / etc. — tainted via the annotation.
  - Pydantic-typed params (resolved via class-hierarchy: param's type inherits from `BaseModel`) — tainted (per §4.1's coarse decision).
  - `request: Request` — tainted via type annotation.

**Pros:**
- Required for accurate FastAPI source resolution. Without it, we can't distinguish handler functions from helper functions.
- Decorator-AST traversal is straightforward in tree-sitter Python (`decorator` node is exposed).
- Reuses Phase 1's `FrameworkSpec.detect` pattern; just extends the set of corroborating signals.

**Cons:**
- New helper code (~50-100 lines).
- Class-hierarchy resolution for DRF requires walking type chains. Tree-sitter doesn't resolve types; we'd need a simple "is this class inheriting from APIView/ViewSet by NAME?" check — string-based, not type-system-aware. Acceptable for MVP.

**Uncertainty:**
- DRF's `ViewSet` actions (e.g., `def list(self, request)`) are framework-routed without explicit `@action` decorator. Detection by method name + class hierarchy.
- FastAPI also has `@app.api_route(["GET", "POST"])` for multi-method routes — the call_path list grows.
- FastAPI routers are common (`router = APIRouter(); @router.get(...)`). Include `APIRouter` receiver binding in MVP unless eval confirms no router fixture uses it.

**Cost:**
- Upfront: 1 day. Decorator-AST helper + FastAPI detection + DRF detection (class-hierarchy + `@api_view`) + tests.
- Long-term: LOW. The helper is reusable for any Python framework using decorators (AIOHTTP, Sanic, Litestar future-proofing).

**Recommendation lean:** Option A. This is the only viable path for FastAPI/DRF; no real shortcut exists.

#### Option B — Skip decorator detection; rely on import + parameter-type signal only

FastAPI: import `from fastapi import` plus `request: Request` parameter (Phase 1 pattern). DRF: import `from rest_framework import` plus `APIView` class definition.

**Pros:**
- Reuses Phase 1's detection pattern exactly.
- ~Zero new code for detection (sources still need a per-CWE registry, but framework detection is import-based).

**Cons:**
- Misses Pydantic-typed source detection. `def handler(item: Item)` has no `Request` parameter — wouldn't be detected as a handler. C2 fixture #6 (FastAPI Pydantic SQLi) WOULD NOT FIRE under this option.
- DRF's `@api_view`-decorated function-based views without class inheritance also wouldn't be detected.

**Uncertainty:**
- N/A — the missed cases are concrete.

**Cost:**
- Upfront: 0.25 day.
- Long-term: HIGH. Multiple C2 fixtures would not fire. Mandatory rework.

**Recommendation lean:** Strongly avoid Option B. Doesn't ship.

**Decision:** Option A. Decorator-AST awareness is in scope.

---

## 5. Per-framework specs

Using §4's decisions (coarse variable-scoped Pydantic + decorator awareness), §5.1–§5.4 will enumerate `FrameworkSpec` for Flask, Django, DRF, FastAPI during implementation planning. Sources/sinks/sanitizers per framework are listed in eval-team's handoff §2 and C2 plan; this spec translates the architectural requirements into Rust SourcePattern / SinkPattern entries.

Detection signals (preview):

- **Flask:** `from flask import Flask` (or any submodule) + `app = Flask(__name__)` corroborating signal.
- **Django:** `from django.` import + view function signature (`def view(request, ...)`) or class-based view.
- **DRF:** `from rest_framework import` + `@api_view` decorator OR class inheriting from `APIView`/`ViewSet` (decorator-AST aware per §4.4).
- **FastAPI:** `from fastapi import` + `app = FastAPI()` or route decorator (`@app.get/post/...`) on a function (decorator-AST aware per §4.4).

FastAPI `request: Request` handling: `request` itself is the variable-scoped source seed. Method calls on that object (`request.body()`, `request.json()`, etc.) are treated as tainted at sink evaluation via per-arg DFG conservative recursion, not via separate `CallResult` source patterns.

Disambiguation order (when multiple signals match): `[fastapi, drf, flask, django]`. Rationale: more specific frameworks first (FastAPI > Django; DRF disambiguation against plain Django via class-hierarchy).

---

## 6. Per-CWE sink registry

Per eval-team's handoff §2.1–§2.4, the Python sink lists by CWE are:

- **CWE-79 (XSS):** `mark_safe`, `markupsafe.Markup`, `format_html` only when the format-string arg is tainted, `Template().render` (with autoescape=False), `render_template_string` (conditional per §4.3).
- **CWE-89 (SQLi):** `cursor.execute`, `cursor.executemany`, `session.execute(text(raw_sql))`, `Model.objects.raw`.
- **CWE-918 (SSRF):** `requests.{get,post,put,delete,patch,head,options,request}`, `urllib.request.urlopen`, `urllib.request.Request`, `httpx.{get,post,put,delete,patch,head,options}`, `aiohttp.ClientSession.{get,post,...}`, `urllib3.PoolManager.request`.
- **CWE-502 (Deserialization):** `pickle.{loads,load}`, `cPickle.loads`, `yaml.load` (without SafeLoader), `jsonpickle.decode`, `marshal.loads`, `dill.loads`.

`tainted_arg_indices` per sink:

- Most CWE-89 / CWE-918 / CWE-502 sinks: `&[0]` (first arg is the dangerous payload).
- `format_html(format_string, *args, **kwargs)`: `&[0]` only. Django escapes subsequent args before interpolation; a tainted value in arg1+ should not fire solely because of `format_html`.
- `render_template_string(template, **context)`: custom taint selector. If the inline template has `{{ name | safe }}`, only the context value bound to `name` is sink-relevant. If `{% autoescape false %}` applies globally, all context values are sink-relevant.
- `cursor.execute(query, params)`: `&[0]` (query string); the `params` arg is an explicit cleanser when present (parametrized form). Detection: if arg[1] is a tuple/dict/list and arg[0] is a string literal with `%s`/`?`/`:name` placeholders → safe; otherwise fire.
- `session.execute(text(raw_sql), bindparams=...)`: `&[0]` (the `text()`-wrapped raw_sql); detection via `text()` call shape + presence/absence of `.bindparams()` chain.
- `Template().render(context_dict)`: depends on §4.3 decision.

Flat fallback interaction:

- Python structured sinks must run before flat sink emission for lines containing structured Python calls.
- If structured Python proves a call safe (e.g. `render_template_string("{{ x }}", x=tainted)` with default autoescape), suppress flat identifier matches only inside that call's byte range.
- If structured Python rejects a call as "not modeled" rather than "safe", do **not** suppress flat fallback. This preserves PR #73's P1/P2 lesson.

Full SinkPattern enumeration is deferred to the implementation plan, but the arg-selection and flat-fallback constraints above are required.

---

## 7. Sanitizer registry

Per eval-team's handoff §3.1–§3.4:

- **HTML escape (CWE-79 cleanser):** `html.escape`, `markupsafe.escape`, `bleach.clean`, `bleach.linkify`. Implicit: Jinja2 autoescape (per §4.3 decision).
- **SQL parametrize (CWE-89 cleanser):** literal SQL string + separate params arg shape; SQLAlchemy `text(":param").bindparams()` / `.params()`.
- **URL allowlist (CWE-918 cleanser):** `urlparse(url).hostname in ALLOWED_HOSTS` paired check; `ipaddress.ip_address(...).is_private`/`is_loopback`/`is_link_local` paired check.
- **YAML safe variant (CWE-502 cleanser):** `yaml.safe_load` (replaces `yaml.load` — but this is detection-side: `yaml.safe_load` is just NOT in the sink list, no cleanser needed).

Anti-sanitizer decision (§4.2): `markupsafe.Markup` is a CWE-79 sink for Phase 2. Do not add anti-cleanser machinery unless a second concrete pattern requires it.

CFG-aware paired-check direction (Go work merged via PR #75) — the current helper is Go-PathTraversal-specific. Phase 2 should either generalize that helper to accept language/category-specific binding and guard classifiers, or add a separate Python URL-allowlist sink-time helper using the same architecture. Do not rely on category-wide `cleansed_for` alone for URL allowlists; direction and variable coupling matter.

Full SanitizerRecognizer enumeration deferred.

---

## 8. Test plan (high-level)

Mirrors Phase 1's test structure:

### 8.1 In-tree positive/negative tests

`tests/algo/taxonomy/taint_sink_go_test.rs`, `tests/algo/taxonomy/taint_sink_python_test.rs`, and `tests/algo/taxonomy/sanitizers_python_test.rs`:

- ≥3 positive tests per CWE family per framework where applicable (12 cells × 1-3 tests per cell = ~30 tests).
- ≥1 negative test per CWE family per framework (no-fire shape: fully sanitized or unrelated).
- Regression tests for the §4 architectural decisions:
  - Coarse Pydantic taint (§4.1 Option A): test that `def handler(item: Item)` followed by `f"... {item.field}"` flowing into SQL fires. Add a discriminator where the same signature has `db: Session` and only `db` reaches the sink; it must not fire from a line-wide source.
  - Markup-of-tainted (§4.2): `mark_safe(taintedVar)` in a Flask handler fires; `mark_safe("<b>literal</b>")` doesn't fire.
  - Inline `| safe` autoescape (§4.3): `render_template_string("{{ x | safe }}", x=tainted)` fires; `render_template_string("{{ x }}", x=tainted)` doesn't fire despite the flat `render_template_string` pattern.
  - Keyword coupling for inline templates: `render_template_string("{{ x | safe }}", x="literal", y=tainted)` does not fire; `x=tainted` fires.
  - Same-line flat fallback guard: safe autoescaped render on the same line as an unrelated real sink does not suppress the unrelated sink.
  - `format_html` arg policy: tainted arg0 fires; tainted arg1 with literal format string does not fire.
  - Decorator-based detection (§4.4): a function without route decorator is NOT a handler (no source taint); same function with `@app.get("/items")` IS a handler; arbitrary `app.get` where `app` is not bound to `FastAPI()` does not count.

### 8.2 Sanitizer suppression integration test

`tests/integration/cwe_phase2_suppression_test.rs` modeled on `cwe_phase1_suppression_test.rs`. Targets:

- Sanitized fixture suite: `tests/fixtures/sanitizer-suite-python/sanitized/{01..10}_*.py` — 10 fixtures with HTML-escape / SQL-parametrize / URL-allowlist / YAML-safe-variant cleansers. Suppression rate ≥80% (target 10/10).
- Unsanitized fixture suite: `tests/fixtures/sanitizer-suite-python/unsanitized/{01..10}_*.py` — 10 fixtures with no cleanser; all should fire. Leakage detection rate 100% (10/10).

### 8.3 Framework detection tests

`tests/frameworks/python/{flask,django,drf,fastapi,registry}_test.rs` mirroring Phase 1's pattern. Positive/negative/disambiguation tests per framework.

### 8.4 Phase 1 regression

`cargo test` full suite must pass. Phase 1 sanitizer suppression rate must remain 10/10 (Go fixtures unchanged).

---

## 9. Out of scope for Phase 2

- **CWE-22 (path traversal) in Python** — eval-team's C2 plan §"Open questions" Q7 explicitly notes "no CWE-22 in this set (CWE-22 is C1/Phase 1 territory)." Revisit in Phase 2.5 if fixtures surface.
- **CWE-78 (OS command injection) in Python** — Phase 1 covered Go; Python's `subprocess.run`/`os.system`/`shell=True` patterns are different enough to warrant their own design pass. Defer.
- **`jsonpickle.decode`, `marshal.loads`, `dill.loads`** — listed in eval-team's CWE-502 sink set, but C2 fixtures don't exercise them. Add if real fixtures surface.
- **Field-sensitive Pydantic taint (Option B in §4.1)** — Phase 2.5 hardening if §4.1 lands as Option A and downstream fixtures need precision.
- **External Jinja2 template file analysis (§4.3 caveat)** — Phase 2.5 if external-template CVEs surface. Inline-template substring scan is the MVP.
- **Async-aware taint propagation** — `await request.body()` is treated as a synchronous source; `asyncio` task boundary tracking is a Phase 3+ concern.
- **Cross-function sanitizer validation** — already out of Phase 1 scope; remains out for Phase 2.
- **Tree-walk caching** — pure perf; same Phase 1.5+ queue item.

---

## 10. Remaining review checks

1. **Variable-scoped source plumbing:** implementation plan must add this before FastAPI Pydantic sources; line-only handler sources are not acceptable.
2. **Python flat fallback:** implementation plan must explicitly handle safe structured calls that otherwise match `SINK_PATTERNS`, especially `render_template_string`.
3. **Inline Jinja2 selector:** implementation plan must include keyword-arg coupling for `{{ x | safe }}` rather than treating every context kwarg as sink-relevant.
4. **External Jinja2 template handling:** Phase 2.5 deferred unless eval produces a C2 fixture that requires external template file analysis.
5. **`format_html` result-cleansing:** C2 fixture #1 is covered because `format_html("literal template", tainted_value)` is the final render-shaped operation and arg0 is literal. Explicitly marking the result as `cleansed_for(Xss)` is deferred to Phase 2.5 unless a downstream-propagation fixture appears.
6. **CWE-78 in Python:** explicitly out of Phase 2 scope per §9.
7. **C2 plan gaps:** confirm no additional eval fixtures require field-sensitive Pydantic, external-template scanning, or Python subprocess coverage.

---

## 11. Cross-references

- **C2 plan:** `~/code/agent-eval/analysis/c2-python-expansion-plan.md`
- **Eval-side handoff:** `~/code/agent-eval/analysis/prism-cwe-coverage-handoff.md`
- **Phase 1 design spec:** `docs/superpowers/specs/2026-04-25-phase1-cwe-go-design.md`
- **Phase 1.5 per-arg DFG:** `docs/superpowers/specs/2026-04-26-phase15-per-arg-dfg.md` + PR #74 (`fbd962a`)
- **CFG-aware paired-check:** PR #75, `claude/phase15-cfg-aware-paired-check`
- **Phase 1 STATUS:** `STATUS-prism-cwe-phase1.md` (updated `dfee78c`)

---

*Implemented on `claude/phase2-cwe-python`; see `STATUS-prism-cwe-phase2.md` for validation status.*
