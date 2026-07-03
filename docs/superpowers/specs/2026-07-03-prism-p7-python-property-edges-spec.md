> **Status: SHIPPED — PR [#155](https://github.com/shoedog/prism/pull/155), merged 2026-07-03 (main c9e0243).** As-shipped deltas vs this brief (all review-driven, precision-tightening): tier-1 own-class hit file/span-filtered; tier-1 requires a genuine instance method (staticmethod/classmethod excluded, first POSITIONAL param literally `self`, `method_owners` entry — keyword-only/splat-led signatures route to tier-3); nested def/lambda/class scopes fenced in the outer walk; `del`/`for`-target/`with`-alias excluded as store/delete context; `property_access_store_skips` counter added. httpx: 57 callers @0.6 for `text`, 1361 recorded / 54 store skips. Cache landed at CPG 34 / sidecar 4. Brief incl. folded codex spec-review corrections: `cls.attr` EXCLUDED (class access returns the descriptor, not the getter), augmented assignment mandated skip+count, concrete decorator node path, receiver-recovery tier dropped (purely syntactic S2). Follows the P5 dedicated-table pattern + P3 cap doctrine, nav-only.

# Task P7 — Python `@property` access edges (nav-only, capped)

You work in the git worktree `/private/tmp/prism-p7-property-edges` on branch `p7-python-property-edges` (based on main @ 900adf6). The repo is prism. Follow TDD.

## Problem (adjudicated `prism_fn` class)

Resolution is call-syntax-only, so a Python attribute access that fires a `@property` getter mints no edge: adjudicated cases include httpx `response.text` (getter `httpx/_models.py:642`, access site `httpx/_main.py:181`), flask `response.max_cookie_size` (`src/flask/wrappers.py:247`), black `leaf.prev_sibling`. The gopls/pyright oracles count these accesses as incoming calls; prism shows nothing. The durable adjudication taxonomy explicitly classifies these as prism recall gaps.

## Architecture (settled by precedent — follow it)

This is the **P5 dedicated-table pattern** combined with the **P3 candidate-cap doctrine**:
- Property accesses are NOT calls — do NOT create synthetic `CallSite`s (a synthetic CallSite would resolve through the ladder and could mint a wrong-kind/Exact edge; this exact hole was caught in P5's spec review). Instead: a serialized table on `CallGraph` (mirror `go_registrations`' plumbing end-to-end) of `PropertyAccessRecord { enclosing: FunctionId, getter: FunctionId, site: {file, line, start_byte, end_byte} }`.
- Surfaced ONLY in `NavigationIndex::build_resolved_call_edges` (src/navigation/mod.rs:389 — beside the P5 registration merge; the P5 plumbing anchors, all spec-review-confirmed mirrorable: table serialized on CallGraph call_graph.rs:406, cleared/recomputed call_graph.rs:1147, re-applied after cache merge cpg/build.rs:306, counted explicitly in call-stats queries.rs:283) as NameOnly edges with new `ResolutionKind::PropertyAccess` (as_str `property_access`): `nav_callers(getter)` shows the access site; `nav_callees(enclosing)` shows the getter. **Nav-only per the consumer-visibility doctrine** (docs/analysis plan status block): property edges never feed Step-5b DataFlow, echo/membrane findings, or any non-nav consumer — with receivers mostly unknown, these are name-class candidates.

## Scope (three slices, in order; TDD each)

**S1 — property index.** During Python extraction (the per-function walk with `caller_id` in scope is at src/call_graph.rs:821 — spec-review confirmed; `unwrap_decorated` src/ast.rs:353 unwraps decorated_definition but does NOT expose decorator names). Add a small helper reading decorator expressions BEFORE unwrapping: tree-sitter python shape is `decorated_definition -> decorator* -> definition`, each `decorator` = `@` + expression — accept expression text exactly `property`, `cached_property`, or `functools.cached_property` (count cached separately); REJECT anything ending in `.setter`/`.deleter` and everything else. Record `(class, method_name) -> getter FunctionId`. Ensure an `@x.setter`-decorated method never pollutes the getter index (its decorator expression is `x.setter`, so the exact-match rule handles it — pin with a test).

**S2 — access-site extraction.** During the Python per-function walk (where `caller_id` is in scope — the P5 pattern; never line-based enclosing lookup): for each attribute node `recv.attr` that is NOT the function part of a call (i.e., `x.attr`, not `x.attr(...)` — verify how the AST distinguishes; a call's function child is the attribute node itself) and NOT an assignment TARGET — tree-sitter python: plain assignment AND augmented assignment both put the target under the `left` field (grammar: assignment/augmented_assignment); skip any attribute node that is (a descendant of) a `left` child of either. **`x.attr += 1` is MANDATED skip + count** (spec-review: fail closed, not implementer's choice). Loads only. Where `attr` exists in the S1 index:
   - Receiver narrowing, strongest first: (1) `self.attr` ONLY, inside an INSTANCE method of a class whose own class (or its same-file single base, mirroring the existing inherited-self limits) defines the property → record against that class's getter. **`cls.attr` is EXCLUDED (spec-review MAJOR): a normal `@property` fires on instance access; class access returns the descriptor object — the method-call resolver's cls-like-self shortcut (resolution.rs:1354) must NOT carry over here.** (2) receiver-type recovery is NOT persisted for attribute loads (no CallSite exists) — SKIP this tier entirely; purely syntactic S2 (spec-review adjudication). (3) unknown receiver (incl. cls and everything else) → ALL classes defining property `attr`, **capped: ≤3 distinct getter targets** (P3 fanout doctrine, confirmed) — above the cap, skip and count (`property_access_fanout_skips`).
   - Never cross-language; never for attr names not in the index (zero cost for normal attribute traffic).

**S3 — nav surfacing + telemetry.** Merge into `build_resolved_call_edges` beside the P5 registration loop (deterministic BTreeSet order). call-stats: iterate the table explicitly (the P5 lesson — table edges do NOT flow through resolver outcomes) → count under `kinds`/`kind_nameonly`/`demoted_edges` as `property_access`; explicit counters: recorded, fanout skips, cached_property recorded.

**Cache.** Bump CPG `CACHE_VERSION` 33→34 and sidecar 3→4 (+ the two version-pin tests). NOTE: a parallel JS branch (P4) bumps the same constants; whichever merges second rebases and increments FROM the landed value.

## Fixtures (matrix; TOML key `[expect] resolution_kind = "property_access"`, `exact = false`, subset mode where needed)
- `eval/fixtures/python/property_access/`: one class with `@property def text`, a function doing `r.text` (unknown receiver, single owner) → seed the getter, expect the access site as caller.
- `eval/fixtures/python/property_self_access/`: `self.text` inside the same class → attributed.
- `eval/fixtures/python/property_fanout/`: 4 classes each with `@property def text` + unknown-receiver access → `callers = []` (cap).
- `eval/fixtures/python/property_setter_guard/`: class with `@property` + `@text.setter`; a STORE `r.text = v` → getter NOT attributed as called from the store site (and the setter never indexed).

## Tests (TDD)
S1: property + cached_property + setter-exclusion + non-decorated same-name method NOT indexed. S2: load vs store vs call-of-attr (x.text() where text is a property returning a callable — the CALL should not double-record; decide + document), self-narrowing, cap. S3: nav callers/callees assertions incl. score 0.6 + kind reason (the P3 F4 pattern, tests/navigation/callees_test.rs); call-stats counters. Non-Python guard (a JS attribute access never records). Full `cargo test` + `cargo fmt`; files under 600 lines.

## Done-checks (run and paste into your report)
```
cargo build --release
cd eval && uv run tier-a --matrix-only --allow-stale-sut    # new fixtures ok; 0 regressions
cd eval && uv run tier-a --quick --allow-stale-sut          # exact_tier UNCHANGED vs base run (P6a gate; run base first); property edges appear in candidate tier at most
./target/release/prism nav --no-cache callers --repo <httpx checkout from eval/corpora.toml> --symbol text --file httpx/_models.py --format json | head -40   # adjudicated case: access-site callers appear at 0.6/property_access (verify the right seed line; use nav nodes-at if the symbol is ambiguous)
./target/release/prism nav call-stats --repo <httpx checkout>   # paste property counters
```

## Commit style
Small logical commits per slice. End each commit message with:
Co-Authored-By: Claude Sonnet 5 <noreply@anthropic.com>
