# Deferred — Go dot-import resolution (roadmap follow-up #4b) — REJECTED at spec review 2026-08-21

**Priority:** Important (measured Go recall gap: 4 zap `observer.New` sites adjudicated `prism_fn`, tier-a
baseline 2026-07-04) — but NOT a bounded fix; needs a redesign.

**Why deferred:** the first design (`2026-08-21-review-path-queue-items-design.md` Item D) was REJECTED by the
sol spec review (`...-spec-review-sol.md`) with six WRONG findings, each verified against `main` @ 47e21ae:
1. Reusing the last-segment/basename import→dir convention (`resolution.rs:239-288`, `:1558-1593`) can mint a
   FALSE Exact: an external `example.net/observer` dot-import binds to a local `tools/observer.New` when that
   basename is unique in the repo (violates the Go precision floor).
2. The proposed rung position is unreachable for the zap shape: the same-directory rung returns
   `GoSamePkgAllFiltered` as soon as zero clause-compatible survivors remain (`resolution.rs:2062-2089`,
   `:2164-2220`), before any later rung runs — the zero-survivor branch itself must consult dot-imports.
3. Lexical shadowing (`New := func(){}; New()`) is not checked by unqualified resolution; P5's
   `receiver_type_in_fn`-at-occurrence check (`call_graph.rs:2954-2980`) is the pattern to reuse.
4. Uniqueness must be GLOBAL across all dot imports of the file (two dot-imported packages each exporting
   `New` must drop, never two Exacts) — incomplete review diffs are routine inputs, compiler guarantees don't hold.
5. `go.mod`/`go.work` module directives are not part of the cache topology (`repo_loader.rs:149-213` hashes
   only `Cargo.toml`; `cpg_cache.rs:212-223`, `:441-449`) — a module-aware rung would go stale on manifest-only
   edits; one CACHE_VERSION bump is not enough.
6. Legacy `--format review` builds its CPG from diff-named files only (`main.rs:714-736`, `:828-879`), so an
   unchanged imported target package is invisible to the owner's primary review workflow — the item must either
   load the target package for review CPGs or be scoped to navigation/eval explicitly. (Controller note: this
   claim affects ALL cross-file review resolution and should be re-verified before the redesign.)

**Impact if left:** bare-name calls through `. "pkg"` stay unresolved (recall gap, Go only, fails safe — no
false edges). Also blocks P5 bare function-value (`Register(New)`) and P11 bare type/constructor owner recovery
for dot-imported names (`resolution.rs:291-343`, `go_receiver_index.rs:223-237`, `:475-500`).

**Fix sketch (redesign inputs):** exact `go.mod` module-root → filesystem mapping (nested modules; drop on
ambiguous `go.work`/`replace`), manifests hashed into cache topology; per-file `go_dot_imports` list plumbed
through CallGraph empty/skeleton/full/subset/remove_files/merge + cache serialization + module-import evidence;
candidates = exported free funcs of the ONE resolved package dir's ordinary (non-`_test`) package clause with
certain build-constraint visibility; route the zero-survivor same-package branch into it; lexical-shadow gate;
global uniqueness across all dot imports; `ResolutionKind::GoDotImport` + explicit ambiguous/external/shadow drop
telemetry; explicit P5/P11 disposition; tests 1–14 in the sol review (incl. review-mode diff-only fixture,
external-basename drop, go.mod-only edit invalidation, full-vs-incremental equality, non-Go byte identity).
