# Spec review — review-path queue items A–D (gpt-5.6-sol @ xhigh, read-only, via served a2a-bridge)

Date: 2026-08-21. Context id `fable-reviewpath-design-sol-20260821`. Reviewed `main` @ 47e21ae. Verdicts: A FIX, B FIX, C FIX, D REJECT. The binding deltas folded into the implementer briefs live beside the design doc (`2026-08-21-review-path-queue-items-design.md`). Raw review text follows.

I’ll treat this as a strict read-only review: no edits, builds, tests, cache writes, provider calls, or gate reruns. I’ll do one primary code-reading pass and one bounded cross-check, verify the exact checkout first, and distinguish design-supplied acceptance evidence from anything I independently observe.

I’m using the `lsp-nav` and `prism-nav` guidance because the review explicitly asks for all parallel consumers and resolution blast radius; I’ll only use read-only navigation mechanisms that comply with your cap.The type-resolved LSP tool is not callable in this session, so I’m falling back to literal-reference census plus the repo’s read-only structural index. The structural index is advisory until each returned site is checked in source; I’ll cite only source lines I actually read.Checkout identity matches `main` at `47e21ae`; the worktree has unrelated pre-existing changes only under `eval/` plus untracked `.superpowers/`/snapshot state, while the reviewed `src/` and `tests/` regions are clean. Item A’s cited production and serial loops are real, but its precision proof is already under pressure: the cache stores each argument node’s byte span, while DFG variable keys use the actual identifier/access-path line, so “argument-node start line” is not generally the same identity. I’m tracing that across the five focus languages before assigning severity.Items A–C now have concrete mechanisms pinned. Item D has exposed a deeper ordering issue: the existing Go same-directory rung can return an “all filtered” drop before any later dot-import rung would run, which is exactly the `observer_test` shape. I’m finishing the import-consumer and incremental-cache census before assigning final verdicts.The final cross-check found one more Item D integration risk worth resolving: the legacy review path initially parses diff files only, while navigation loads the whole repository. I’m tracing whether review mode later expands that file map; if it does not, dot-import resolution cannot see an unchanged target package in the owner’s primary code-review workflow.Reviewed `main` at `47e21aee9f9443efcb41086799ba30e369814c8c`. I made no edits and ran no tests, builds, or providers. The worktree has unrelated `eval/` changes, but none of the cited `src/` or test files were modified.

## Item A — FIX

### Grounding

The stated primary mechanism is correct. The indexed argument representation already retains exact byte spans; argument text is derived from each named child of the call’s argument-list node ([src/ast.rs:109-124](/Users/wesleyjinks/code/slicing/src/ast.rs:109), [src/ast.rs:335-385](/Users/wesleyjinks/code/slicing/src/ast.rs:335)). Production Step 5b uses `site.line` in both its Use and Def lookup ([src/cpg/build.rs:880-943](/Users/wesleyjinks/code/slicing/src/cpg/build.rs:880)); the serial twin repeats it ([src/cpg/build.rs:994-1054](/Users/wesleyjinks/code/slicing/src/cpg/build.rs:994)). The negative Python pin is exactly as described ([src/cpg/tests.rs:871-897](/Users/wesleyjinks/code/slicing/src/cpg/tests.rs:871)).

The cache bump is justified: both the `CallGraph` and DFG plus assembled CPG edges are persisted ([src/cpg_cache.rs:290-305](/Users/wesleyjinks/code/slicing/src/cpg_cache.rs:290)). No new repo-wide incremental fact is introduced; graph assembly reruns on the merged CG/DFG ([src/cpg/build.rs:240-248](/Users/wesleyjinks/code/slicing/src/cpg/build.rs:240)).

### Findings

- **WRONG — Argument-node start line is not a sound proxy for the access occurrence line.** A constructible case in every focus language is a transparent grouping:

  ```text
  g(
      (
          user
      )
  )
  ```

  The indexed argument is the named wrapper child, whose span begins on the `(` line, while DFG variable spans use the identifier or field-access node’s own line ([src/ast.rs:356-369](/Users/wesleyjinks/code/slicing/src/ast.rs:356), [src/ast.rs:4151-4204](/Users/wesleyjinks/code/slicing/src/ast.rs:4151), [src/data_flow.rs:393-420](/Users/wesleyjinks/code/slicing/src/data_flow.rs:393)). The proposed `arg_line` lookup misses; `site.line` also misses; the incorrect output remains no arg→param edge and `NotReached`. Equivalent misses include Python keyword/star arguments, JS/TS spread elements, Java casts, and Rust borrow/wrapper expressions. Several also fail `AccessPath::from_expr`, which only normalizes a bounded set of transparent forms ([src/access_path.rs:56-192](/Users/wesleyjinks/code/slicing/src/access_path.rs:56)).

- **WRONG — The `site.line` fallback can bind the wrong occurrence.** For example:

  ```javascript
  g(obj.value,
    (obj.value = "safe"),
    (
      obj.value
    ));
  ```

  For the third argument, its wrapper line has no `obj.value`; fallback selects the first argument’s pre-assignment occurrence on the call line and connects that occurrence to the third parameter. The wrong output is a DataFlow edge from the old value to the third parameter. This is possible because lookup identity is only function, line, access path, and access kind; byte identity is deliberately excluded ([src/data_flow.rs:29-40](/Users/wesleyjinks/code/slicing/src/data_flow.rs:29), [src/cpg/build.rs:920-941](/Users/wesleyjinks/code/slicing/src/cpg/build.rs:920)). The design’s claim that line scoping excludes unrelated occurrences is therefore false.

- **SMELL — The proposed API discards the stronger identity already available.** `CallArg` already stores `start_byte..end_byte`; reducing that to a start line recreates the ambiguity responsible for the current gap ([src/ast.rs:109-124](/Users/wesleyjinks/code/slicing/src/ast.rs:109)). Return the argument span, then select a unique direct access occurrence contained by that span. If that cannot be proven, drop.

- **SMELL — “Nested multi-line call as an argument” is underspecified.** For `g(h(user))`, Step 5b may correctly add `user → h.param`, but directly adding `user → g.param` would invent return flow, which this item expressly excludes. The test must assert both poles: the inner edge exists and the outer direct edge does not.

- **SMELL — The `>0` smoke acceptance is nondiscriminating.** A wrong fallback edge also makes the count positive. Assert exact source occurrence/span, destination parameter, and absence of competing edges.

### Second copies and minimum tests

Both Step-5b implementations must change together. The retained text-only wrapper also has a separate production consumer in seed selection and should remain behaviorally unchanged ([src/reasoning/seeds.rs:358-373](/Users/wesleyjinks/code/slicing/src/reasoning/seeds.rs:358)). Existing AST parity tests cover mostly same-line forms and only one Rust multiline fixture; they do not test occurrence-line identity ([src/ast.rs:7335-7443](/Users/wesleyjinks/code/slicing/src/ast.rs:7335)).

Minimum additions:

1. Direct multiline identifier positive in Python, Go, Rust, Java, JS, and TS.
2. Transparent grouping in all five language families; Rust borrow, Java cast, Python keyword/star, and JS/TS spread cases.
3. The mutation-between-arguments case above: either bind the contained later occurrence or drop, never bind the site-line occurrence.
4. Two same-path occurrences in one call, with endpoint byte spans asserted.
5. Nested call: inner descent present, outer direct arg edge absent.
6. Parallel/serial Step-5b equality for the new cases.

### Reviewer questions

- **(a)** Remove the `site.line` fallback. For same-line arguments, the actual contained occurrence naturally reports `site.line`; absence of a provable occurrence should remain a drop.
- **(b)** Yes. Parenthesized/grouped expressions affect every focus language; Python keyword/star arguments, JS/TS spread, Java casts, and Rust wrapper expressions add further mismatches.

## Item B — FIX

### Grounding

The mechanism is correctly identified. The active registry chains shell, path, JS/TS, and Python recognizers ([src/sanitizers/mod.rs:18-26](/Users/wesleyjinks/code/slicing/src/sanitizers/mod.rs:18)); shell is empty, path is Go-only, and the other tables carry the stated languages ([src/sanitizers/path.rs:14-32](/Users/wesleyjinks/code/slicing/src/sanitizers/path.rs:14), [src/sanitizers/js_ts.rs:12-41](/Users/wesleyjinks/code/slicing/src/sanitizers/js_ts.rs:12), [src/sanitizers/python.rs:12-64](/Users/wesleyjinks/code/slicing/src/sanitizers/python.rs:12)).

Both advisory/category paths currently iterate recognizers without a language predicate ([src/algorithms/taint.rs:10626-10678](/Users/wesleyjinks/code/slicing/src/algorithms/taint.rs:10626), [src/algorithms/taint.rs:10725-10747](/Users/wesleyjinks/code/slicing/src/algorithms/taint.rs:10725)). `sanitizer_call_site` has the intended predicate ([src/algorithms/taint.rs:10805-10818](/Users/wesleyjinks/code/slicing/src/algorithms/taint.rs:10805)). `cleansed_for` is consulted by generic sink suppression, while Go path traversal receives additional sink-time validation ([src/algorithms/taint.rs:5999-6016](/Users/wesleyjinks/code/slicing/src/algorithms/taint.rs:5999), [src/algorithms/taint.rs:6068-6102](/Users/wesleyjinks/code/slicing/src/algorithms/taint.rs:6068)).

### Findings

- **WRONG — None found.** Language filtering at both advisory entry paths removes false marks and cannot introduce a new suppression.

- **SMELL — The documentation update list misses two parallel declarations.** Besides the P10 paragraph, update `SanitizerRecognizer.languages`, which currently says only the verdict matcher consumes it ([src/frameworks/mod.rs:92-97](/Users/wesleyjinks/code/slicing/src/frameworks/mod.rs:92)), and the second-source comment above `sanitizer_supported` ([src/sanitizers/mod.rs:28-43](/Users/wesleyjinks/code/slicing/src/sanitizers/mod.rs:28)). The active-registry comment also omits JS/TS ([src/sanitizers/mod.rs:18-25](/Users/wesleyjinks/code/slicing/src/sanitizers/mod.rs:18)).

- **SMELL — The tests do not fully cover the fail-closed pole.** The Python positive protects only one table. An accidental predicate inversion could remove all JS/TS advisory matching while the proposed tests still pass. Add a JS/TS positive through the full cleansing/warning path and a Go paired-check positive that proves its existing advisory and sink-time behavior is unchanged. Existing tests cover Python positive, unsupported Rust, paired-check exclusion from the node matcher, and wrong-language rejection only at the node matcher ([src/algorithms/taint.rs:11611-11738](/Users/wesleyjinks/code/slicing/src/algorithms/taint.rs:11611)).

- **SMELL — One helper comment becomes more misleading after this change.** `function_body_cleansed_for` still calls the phase “Go-only” even though its callers support Python and JS/TS ([src/algorithms/taint.rs:10615-10625](/Users/wesleyjinks/code/slicing/src/algorithms/taint.rs:10615)).

No cache bump is needed: this changes runtime taint/advisory matching, not persisted graph state.

### Minimum tests

1. Go `escape` negative through the complete finding and `Cleansed` warning path.
2. JS `markupsafe.escape` negative.
3. Python `html.escape` positive unchanged.
4. JS/TS `DOMPurify.sanitize` or `escapeHtml` positive through advisory and suppression behavior.
5. Go paired `filepath.Clean` positive advisory plus existing sink-time safe/unsafe guard cases.
6. Derived support-set equality across every `Language` variant.

### Reviewer questions

- **(a)** No legitimate cross-language exception should be preserved. A cgo/wasm boundary is represented by code in the caller file’s language; sharing a foreign sanitizer’s spelling does not prove its semantics.
- **(b)** Yes, keep the paired-check family advisory-only in the node-scoped reasoning matcher. It remains Go-language eligible in advisory scanning, while Go path suppression continues through the sink-time AST/CFG validator ([src/algorithms/taint.rs:10783-10816](/Users/wesleyjinks/code/slicing/src/algorithms/taint.rs:10783), [src/reasoning/sanitizer_walk.rs:145-206](/Users/wesleyjinks/code/slicing/src/reasoning/sanitizer_walk.rs:145)).

## Item C — FIX

### Grounding

Both diagram payload locations and their empty-vector serialization behavior are correctly cited ([src/slice.rs:22-43](/Users/wesleyjinks/code/slicing/src/slice.rs:22), [src/slice.rs:297-321](/Users/wesleyjinks/code/slicing/src/slice.rs:297), [src/output/review_compact.rs:89-118](/Users/wesleyjinks/code/slicing/src/output/review_compact.rs:89)).

### Findings

- **WRONG — Multi-algorithm output has a bypass copy.** `to_compact_review_output` owns each `results[*].findings` clone, but top-level `all_findings` is cloned independently in `main`. If only the helper strips diagrams, then:

  ```text
  prism ... --algorithm taint,primitive --format review --review-no-diagrams
  ```

  can omit diagrams from `results` while retaining `all_findings[*].diagrams`. The wrong output still contains diagram payloads under the suppression flag ([src/main.rs:963-1005](/Users/wesleyjinks/code/slicing/src/main.rs:963), [src/output/review_compact.rs:138-182](/Users/wesleyjinks/code/slicing/src/output/review_compact.rs:138)).

- **SMELL — The stderr-note precedent is mis-grounded.** `review_min_severity` and `review_full_slices` are only read inside the two review branches and are silently ignored elsewhere; there is no matching note behavior ([src/main.rs:152-166](/Users/wesleyjinks/code/slicing/src/main.rs:152), [src/main.rs:970-1013](/Users/wesleyjinks/code/slicing/src/main.rs:970), [src/main.rs:1087-1105](/Users/wesleyjinks/code/slicing/src/main.rs:1087)). Choose either silent ignore for consistency or add one common validation/note path for all review-only flags.

- **SMELL — “JSON unaffected” conflicts with the proposed stderr note unless scoped precisely.** Specify that JSON stdout and exit status remain byte-identical while stderr gains the note, or choose silent ignore.

- **SMELL — The finalize branch must be resolved now, not left conditional.** `finalize_diagrams` is the sole production producer of diagram warnings; it renders both result-level and per-finding graphs and pushes all warnings there ([src/algorithms/mod.rs:207-260](/Users/wesleyjinks/code/slicing/src/algorithms/mod.rs:207)). `run_algorithm` always invokes it before output selection ([src/main.rs:1337-1344](/Users/wesleyjinks/code/slicing/src/main.rs:1337)). Therefore the flag cannot skip finalization while preserving warnings and `--strict-diagrams` semantics. This item saves output size, not rendering compute, unless a separately specified validation-without-Mermaid path is added.

No persisted graph or cache state changes.

### Minimum tests

1. Single review: result-level and finding-level diagram keys absent.
2. Multi review: recursively verify `results[*]`, `results[*].findings[*]`, and top-level `all_findings[*]`.
3. A real warning-producing fixture; warning remains in JSON and stderr.
4. `--strict-diagrams` retains its bug-warning exit code; bug kinds are defined separately from informational truncation warnings ([src/slice.rs:481-505](/Users/wesleyjinks/code/slicing/src/slice.rs:481)).
5. Unflagged review golden byte-identical.
6. JSON stdout byte-identical with the flag, plus an explicit assertion for the chosen stderr behavior.

### Reviewer question

Keep `diagram_warnings` in structured JSON and continue emitting them to stderr. They are machine-readable diagnostics and drive strict exit behavior; suppressing the visualization does not erase the renderer/algorithm defect they report.

## Item D — REJECT

### Grounding

Dot aliases are indeed recognized by the walker and then explicitly discarded from the alias map ([src/ast.rs:2344-2398](/Users/wesleyjinks/code/slicing/src/ast.rs:2344)). However, the claimed reusable “import-path→directory convention” is mischaracterized: there is no full-path mapping. `resolve_go_owner_identity` takes only the last import segment and requires a unique directory basename ([src/resolution.rs:239-288](/Users/wesleyjinks/code/slicing/src/resolution.rs:239)); generic qualified-call resolution likewise matches only the final segment/stem ([src/resolution.rs:1558-1593](/Users/wesleyjinks/code/slicing/src/resolution.rs:1558)).

### Findings

- **WRONG — The primary review path cannot see an unchanged imported target.** Navigation loads the full repository ([src/main.rs:585-599](/Users/wesleyjinks/code/slicing/src/main.rs:585)), but legacy `--format review` parses only files named by the diff and builds its CPG from that map ([src/main.rs:714-736](/Users/wesleyjinks/code/slicing/src/main.rs:714), [src/main.rs:828-879](/Users/wesleyjinks/code/slicing/src/main.rs:828)). For a diff containing only `observer_test.go`, unchanged `observer.go` is absent from `functions`; `New` returns `UnknownName` before any dot-import directory resolution. The wrong output is still no edge in the owner’s stated code-review workflow, even if `nav call-stats` and tier-a succeed. The design must either load the target package for review CPGs or explicitly narrow the item to navigation/eval.

- **WRONG — Reusing the current basename convention can mint a false Exact edge.** Suppose the repository contains `tools/observer/observer.go` with `func New`, while a caller dot-imports external `"example.net/observer"`. Because `observer` is the unique indexed basename, the proposed rung binds the external import to the local function. The wrong output is an Exact edge to `tools/observer.New`; this violates the Go precision floor. The current helper proves it does not compare the complete path or module root ([src/resolution.rs:277-285](/Users/wesleyjinks/code/slicing/src/resolution.rs:277)).

- **WRONG — The stated position is unreachable for the zap shape unless the early-drop branch is rewritten.** The current Go rung gathers same-directory candidates, filters `observer.New` because `observer_test != observer`, and immediately returns `GoSamePkgAllFiltered` when zero survive ([src/resolution.rs:2062-2089](/Users/wesleyjinks/code/slicing/src/resolution.rs:2062), [src/resolution.rs:2164-2220](/Users/wesleyjinks/code/slicing/src/resolution.rs:2164), [src/go_build_profile.rs:99-113](/Users/wesleyjinks/code/slicing/src/go_build_profile.rs:99)). A rung inserted after this block never runs. The spec must explicitly route the zero-survivor namespace branch through dot-import resolution before returning the old drop.

- **WRONG — Lexical shadowing is not covered.** Compiling Go can contain:

  ```go
  import . "mod/observer"

  func f() {
      New := func() {}
      New()
  }
  ```

  The proposed resolver examines only free functions from the repository function index and therefore emits Exact to `observer.New`; the correct target is the local function value. Current unqualified resolution has no lexical-binding check ([src/resolution.rs:2042-2059](/Users/wesleyjinks/code/slicing/src/resolution.rs:2042)). P5 already demonstrates the required mechanism: check `receiver_type_in_fn` at the exact value occurrence before accepting a repository function ([src/call_graph.rs:2954-2980](/Users/wesleyjinks/code/slicing/src/call_graph.rs:2954)).

- **WRONG — Per-import uniqueness is insufficient; uniqueness must be global across all dot imports.** With two dot imports whose packages each uniquely export `New`, the described “for each path” loop can emit two Exact edges. That is forbidden fanout. Although such a completed file is rejected by the Go compiler, incomplete review diffs and parseable intermediate states are routine inputs; the analyzer must aggregate all eligible targets across all dot imports and emit only when the total set is exactly one.

- **WRONG — A module-aware implementation would remain stale after `go.mod` changes under the stated cache plan.** `LoadedRepo.manifest_hashes` currently scans only `Cargo.toml` ([src/repo_loader.rs:149-177](/Users/wesleyjinks/code/slicing/src/repo_loader.rs:149), [src/repo_loader.rs:180-213](/Users/wesleyjinks/code/slicing/src/repo_loader.rs:180)). Cache topology includes only supplied manifest hashes, and identical source hashes produce a full cache hit ([src/cpg_cache.rs:212-223](/Users/wesleyjinks/code/slicing/src/cpg_cache.rs:212), [src/cpg_cache.rs:441-449](/Users/wesleyjinks/code/slicing/src/cpg_cache.rs:441)). After changing only `go.mod` from `module old.example/m` to `module new.example/m`, a formerly internal dot import can become external while the cached Exact edge remains. A one-time `CACHE_VERSION` bump does not protect later manifest edits.

- **SMELL — `go_build_profile` does not expose module roots.** It contains only per-file package clause, test-file status, architecture/OS, and build expression ([src/go_build_profile.rs:1-21](/Users/wesleyjinks/code/slicing/src/go_build_profile.rs:1)). Module mapping belongs in repository loading/topology, not this helper.

- **SMELL — Imported-package visibility needs a distinct helper.** The existing visibility function compares caller and candidate package clauses, which is intentionally wrong for an imported package ([src/go_build_profile.rs:94-131](/Users/wesleyjinks/code/slicing/src/go_build_profile.rs:94)). The new rule must identify exactly one ordinary target package clause, exclude `_test.go` declarations, and apply compatible/certain build constraints without requiring the caller’s package name to match.

- **SMELL — Drop telemetry is underspecified.** Adding `ResolutionKind::GoDotImport` and its `as_str` is sufficient for positive `call-stats`; the query already counts every resolution kind automatically ([src/resolution.rs:31-138](/Users/wesleyjinks/code/slicing/src/resolution.rs:31), [src/navigation/queries.rs:214-270](/Users/wesleyjinks/code/slicing/src/navigation/queries.rs:214)). Add explicit ambiguous/external/shadow-refusal telemetry or drop reasons; otherwise the failure pole remains hidden behind `UnknownName` or `GoSamePkgAllFiltered` ([src/resolution.rs:737-768](/Users/wesleyjinks/code/slicing/src/resolution.rs:737)).

### Second-copy and consumer census

The new per-file field must be handled by:

- `CallGraph::empty`, skeleton construction, full construction, direct-subset construction, `remove_files`, and `merge` ([src/call_graph.rs:609-630](/Users/wesleyjinks/code/slicing/src/call_graph.rs:609), [src/call_graph.rs:822-835](/Users/wesleyjinks/code/slicing/src/call_graph.rs:822), [src/call_graph.rs:951-958](/Users/wesleyjinks/code/slicing/src/call_graph.rs:951), [src/call_graph.rs:1418-1446](/Users/wesleyjinks/code/slicing/src/call_graph.rs:1418), [src/call_graph.rs:1559-1577](/Users/wesleyjinks/code/slicing/src/call_graph.rs:1559), [src/call_graph.rs:3583-3613](/Users/wesleyjinks/code/slicing/src/call_graph.rs:3583)).
- Cache serialization and the pinned version test ([src/cpg_cache.rs:290-305](/Users/wesleyjinks/code/slicing/src/cpg_cache.rs:290), [src/cpg_cache.rs:646-650](/Users/wesleyjinks/code/slicing/src/cpg_cache.rs:646)).
- Module-import evidence and repo unresolved-module counts, which currently consume only alias-map values ([src/navigation/module_graph.rs:318-340](/Users/wesleyjinks/code/slicing/src/navigation/module_graph.rs:318), [src/navigation/module_graph.rs:412-420](/Users/wesleyjinks/code/slicing/src/navigation/module_graph.rs:412)).

Consumers that should not mechanically receive dot paths:

- Qualified-call and qualified-owner resolution still require a real alias, not `"."`.
- Provenance suppression iterates alias names; a dot import supplies no alias and must not be inserted there ([src/algorithms/provenance_slice.rs:375-395](/Users/wesleyjinks/code/slicing/src/algorithms/provenance_slice.rs:375), [src/algorithms/provenance_slice.rs:532-542](/Users/wesleyjinks/code/slicing/src/algorithms/provenance_slice.rs:532)).

Real adjacent gaps that must be expressly included or deferred:

- P5 bare function values such as `Register(New)` stop at same-package resolution and never consult imports ([src/resolution.rs:291-343](/Users/wesleyjinks/code/slicing/src/resolution.rs:291)).
- P11 return/receiver typing treats bare owner names and bare constructor calls as current-package facts; dot-imported `T` and `New()` therefore remain unresolved or can be mis-owned ([src/resolution.rs:239-288](/Users/wesleyjinks/code/slicing/src/resolution.rs:239), [src/go_receiver_index.rs:223-237](/Users/wesleyjinks/code/slicing/src/go_receiver_index.rs:223), [src/go_receiver_index.rs:475-500](/Users/wesleyjinks/code/slicing/src/go_receiver_index.rs:475)).

### Minimum tests

1. Actual `--format review` diff containing only the caller while the target remains unchanged.
2. Root `go.mod` mapping: `module go.uber.org/zap` plus import `go.uber.org/zap/zaptest/observer`.
3. External import with the same unique directory basename: must drop.
4. Two internal directories with the same basename, exact full path selecting one.
5. Two dot-import paths each exporting `New`: global drop, never two Exact edges.
6. Parameter, local variable, and short-declaration shadowing of `New`.
7. Unexported function and target `_test.go` function: no edge.
8. Build-tag-exclusive definitions: Exact only when visibility is certain; otherwise drop.
9. Dot-import path edit under partial cache: old edge removed, new edge added.
10. `go.mod`-only edit: topology miss/full rebuild and stale edge removed.
11. Serialization, remove/merge, and full-versus-incremental equality.
12. Explicit P5/P11 non-goal tests if those consumers are deferred.
13. Positive `go_dot_import` kind plus ambiguous/external drop telemetry.
14. Non-Go byte identity.

### Reviewer questions

- **(a)** Do not include methods as direct candidates. Methods do not become unqualified package identifiers and require a receiver; composite literals are not call targets. Dot-imported types do affect P11 receiver typing, but that is a separate owner-resolution extension that must be included explicitly or deferred.
- **(b)** No. The present last-segment convention is insufficient and unsafe. Build an exact mapping from repository `go.mod` module directives to filesystem roots; account for nested modules, and drop on ambiguous `go.work`/`replace` cases unless those are modeled. Hash the relevant manifests into cache topology.
- **(c)** Yes: P5 bare function-value resolution, P11 bare type/constructor owner recovery, module-import evidence, and all CallGraph incremental/schema copies are affected. Alias-based qualified resolution and provenance alias suppression must remain separate.

## Ranked changes required before implementation

### Item A

1. Replace line-returning argument API with byte-span-backed, unique contained-occurrence resolution.
2. Remove `site.line` fallback; drop when identity is not provable.
3. Specify transparent-wrapper versus computed/nested-call behavior.
4. Add endpoint-identity and parallel/serial tests.

### Item B

1. Add JS/TS and Go paired-check positive tests.
2. Update all three stale language-consumer comments.
3. Implement the same language predicate in both advisory paths and derive the support gate.

### Item C

1. Strip top-level multi-run `all_findings` as well as per-result copies.
2. Require finalization to run under the flag; this iteration provides payload-size savings only.
3. Define one consistent non-review flag behavior.
4. Test single/multi output, warnings, and strict exit semantics.

### Item D

1. Define exact module-root mapping and fail closed; do not reuse basename matching.
2. Make the `GoSamePkgAllFiltered` zero-survivor branch consult the dot-import rung.
3. Add lexical-shadow and global-across-imports uniqueness gates.
4. Make review mode load the unchanged imported target, or narrow the item’s claimed scope.
5. Add `go.mod`/`go.work` topology invalidation and complete incremental/schema plumbing.
6. Specify imported-package build/test visibility and P5/P11 disposition.

**Overall verdict: FIX REQUIRED — Items A, B, and C are bounded fixes; Item D is REJECTED until its review-loading, identity, shadowing, and cache model are redesigned.**
