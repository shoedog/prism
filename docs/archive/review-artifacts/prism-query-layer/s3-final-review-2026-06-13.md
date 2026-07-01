# Full-Branch Code Review — S3 (codex gpt-5.5 xhigh, correctness + architecture)

**Process:** `code-review-codex` workflow (both lenses codex xhigh + synth, codex-only variant `examples/a2a-bridge.slicing-review-codex.toml` while the bridge claude model-override defect is open) over the full `git diff main..s3-precision` (14 commits) with prism MCP. **Triage (owner fix-vs-defer policy):**

- **FIXED (4 MAJOR)** in commit on s3-precision: MAJOR1 R1 module-prefix narrowing (`foo::Engine::start` no longer resolves `bar::Engine::start` as Exact — `owner_lookup_in_modules`); MAJOR2 R3 import-qualified excludes methods (a `pkg.f()` can no longer forge an edge to a class method `f`); MAJOR3 Python self/cls skip gated on `info.owner.is_some()` (free function with a `self` param keeps all args); MAJOR4 `peel_type` strips Rust lifetimes + `*const`/`*mut` (`&'a Sender` recovers). Each pinned by a regression test.
- **DEFERRED (1 MAJOR + 2 MINOR):** MAJOR5 durable cache-key (resolver/build identity) — the main→S3 invalidation is already covered by the Task-4 v3→v4 bump; the durable auto-invalidation across future resolver changes is non-trivial (resolver-version semantics + warm-cache test) and is deferred doc item 4. MINOR1 self-receiver line-substring matching (robustness; rare) and MINOR2 callees DropReason detail (telemetry; call-stats already exposes the global split) — both added to the deferred doc.

## Raw review

MAJOR — src/resolution.rs:220-237 — R1 owner lookup ignores module path for `mod::T::m`. `foo::Engine::start()` can resolve every `(Engine, start)` definition, including `bar::Engine::start()`, as `Exact`. Fix by applying preceding module segments as a file/package narrowing filter or by indexing module-qualified owner keys; add same-bare-type/different-module tests.

MAJOR — src/resolution.rs:304-322 — R3 import-qualified calls can bind to methods. `import util; util.f()` may resolve to `class C: def f(...)` inside `util.py`, creating a false package-function edge to a receiver method. Filter R3 candidates to free functions only; keep class/static owner calls on R3b. Add an imported-module-has-only-method fixture.

MAJOR — src/cpg/build.rs:341-356 — Python `self`/`cls` parameter skipping is applied to free functions. `def helper(self, x)` called as `helper(a, b)` binds `a -> x` instead of `a -> self` and loses `b -> x`. Gate the skip on actual method ownership, e.g. `info.owner.is_some()`, and add a CPG dataflow test for free functions whose first parameter is named `self` or `cls`.

MAJOR — src/resolution.rs:78-101 — P6-lite receiver typing misses common Rust lifetime/pointer forms. `fn run(tx: &'a Sender) { tx.send(); }` peels to `"'a Sender"` instead of `Sender`, so a recoverable typed receiver can fall through to R6 and be dropped in collisions. Strip Rust lifetime tokens and pointer qualifiers after `&`/`*`, including `&'a`, `&'a mut`, `*const`, and `*mut`; add typed-param tests.

MAJOR — src/cpg_cache.rs:161 — Nav/CPG cache invalidation is still tied to package/format version, not resolver behavior or build identity. Warm caches can keep serving pre-S3 edges across binary revisions, while eval only avoids this with `--no-cache`. Add a resolver/build identity such as `GIT_SHA` or a resolver behavior version to cache metadata/key, and test warm-cache invalidation.

MINOR — src/call_graph.rs:947-963 — Rust self-receiver recovery uses raw line substring matching. A bare `helper(); // self.helper` or multiple calls on one line can be misclassified as `self.helper()`. Remove the line-text fallback or make it span/AST-node based; add comment/string and same-line regression tests.

MINOR — src/navigation/queries.rs:296 — Callees navigation drops `ResolutionOutcome.drop` detail. Unresolved callee evidence cannot distinguish unknown names, import-external drops, external receivers, or multi-owner collisions. Carry `DropReason` into unresolved callee evidence and expose it in warning/scoring metadata.

Disagreement resolution: Claude’s scoped-CPG finding is not included because the diff explicitly documents that scope computation is intentionally recall-biased; Codex’s concrete resolver/CPG failure paths are the actionable correctness issues.

Overall verdict: ship after fixing the 5 MAJOR issues; the MINORs can follow if release pressure is high.