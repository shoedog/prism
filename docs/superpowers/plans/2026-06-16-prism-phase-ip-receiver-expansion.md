# Phase-IP receiver-expansion (PR-2) — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended)
> or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax
> for tracking.
>
> **rev 2 (2026-06-16) — dual plan-review folded** (codex gpt-5.5 xhigh + claude/Opus xhigh, both
> prism-wired, via a2a-bridge). Verdict was *"fix before building."* All 3 blockers (B5 cache-test compile
> failure; Slice-D wrong-seam/under-spec; B7 fixture line), the C4 false-recovery path, and the §12/§4
> coverage gaps are resolved here; minor snippet/doc items folded. Review record: `/tmp/pr2-plan-review-2026-06-16.md`.

**Goal:** Expand Go interface-dispatch **receiver type recovery** so type-assertion (`x.(Module).M()`)
and `var`-declared (`var r Runner`) receivers reach PR-1's (unchanged) `interface_impls` dispatch engine,
behind a swappable `ReceiverClassifier` seam.

**Architecture:** *Recover-and-route* (spec §2). A new `ReceiverClassifier` seam recovers a receiver's
**syntactic** static type and tags the fact; the *existing* `owner_lookup → interface_impls → drop` ladder
(`resolution.rs:453-492`) routes it — concrete → `owner_lookup`, interface → `interface_impls`, miss → drop.
**No engine change, no `GoTypeProvider` at extraction, no interface-set predicate at extraction.** Slice A
extracts today's `recover_receiver` into the seam (byte-identical `legacy` parity) and makes the one
upstream API change: surfacing the receiver node into the classifier. Slices B/C add the two forms purely as
classifier arms + an additive `ReceiverRecovery` variant each. Slice D adds a structural in-scope manifest
(emitted on `&CpgContext`) + a precision **report** (never gating in PR-2). Slice E (owner-gated) re-
adjudicates caddy + re-baselines. Slice F reserves `SliceElem`.

**Tech Stack:** Rust (tree-sitter-go 0.23.4, rayon, serde/bincode, petgraph); Python tier-A harness
(`eval/tier_a/`, `uv`/pytest); a2a-bridge for per-task implementation + review.

---

## Execution Handoff & Orientation (cold-start)

**Branch:** `phase-ip-receiver-expansion`, stacked directly on merged `main` (`5cd1ac9`). PR-1 (#96) is
**MERGED**. Do all PR-2 work here. **Do not push or open a PR until the owner asks.**

**Untracked eval artifacts** (`docs/eval/tier-a/2026-06-15-*`, `eval/snapshots/*`) are leftover run
outputs — **leave untracked, never commit**.

**The contract:** `docs/superpowers/specs/2026-06-16-prism-phase-ip-receiver-expansion-design.md` (rev 2,
owner-approved). This plan implements its §10 slices A–F; §13 decisions are locked.

**Verified code-seam facts** (read at plan time + re-verified during the dual review against the current
tip — these supersede the spec/review's looser text):

| Fact | Location (verified) |
|------|---------------------|
| `recover_receiver` (the seam to subsume) + its gate | `src/call_graph.rs:1384-1412` (gate `:1392-1405`) |
| `recover_self_receiver_qualifier` reassigns the qualifier **before** recovery | `src/call_graph.rs:367-372` and `:1017-1022` |
| Two extraction call sites (build / build_direct_subset) | `src/call_graph.rs:373` and `:1023` |
| Extraction loop is **rayon `.par_iter()`** | `src/call_graph.rs:338-339` → classifier trait must be `Sync` |
| Call extractor (returns `(name,line,qualifier,start,end)`) | `src/ast.rs:3515-3559`; manual fallback `:3563` |
| `call_function_qualifier` already returns the **node** (Go selector `operand`) | `src/languages/mod.rs:701-742` (Go `:733`) |
| Legacy scan `receiver_type_in_fn` | `src/ast.rs:313-380` (calls `walk_receiver_bindings` rooted at the fn node, `:368-375`) |
| Binding walk (where the `var` arm hooks) | `src/ast.rs:3816-3897`; `constructor_type:3899`; `first_constructor_type_child:3934`; `node_binds_name:3944` (recurses the **whole subtree**); `simple_binding_text:3957` |
| `ReceiverRecovery` enum (today: 2 variants) | `src/resolution.rs:158-162` |
| R6 relabel + interface consult (the router) | `src/resolution.rs:453-492` (relabel `_ => TypedParam` `:455-460`; consult `:479-489`) |
| `owner_key` (strips `&*` + `::`, **NOT** Go `pkg.`) / `iface_key` (strips `*` + `pkg.`) / `peel_type` | `src/resolution.rs:79 / 93 / 123` |
| `CallSite` struct (byte spans + receiver fields) + ordering | `src/call_graph.rs:24-44`, `cmp_key 1347-1358` |
| `CACHE_VERSION = 9` + the test that pins it to 9 | `src/cpg_cache.rs:50`; **`fn cache_version_is_9_for_interface_fields` asserts `9` at `:511-512`** |
| GIT_SHA self-invalidation (per-commit built; dirty needs `--no-cache`) | `src/cpg_cache.rs:298-307` |
| `build` callers (production) | `src/cpg/build.rs:136` (build), `:182` (build_direct_subset) |
| Resolution-test harness (`build`/`site_in`, interface tests) | `tests/integration/resolution_test.rs:7-212` |
| tier-A fixture format + **auto-discovery** (`glob("*/expected.toml")`), keyed by **call-site line** | `eval/fixtures/go/interface_dispatch/`, `eval/tier_a/matrix.py` `_run_matrix_inner` |
| `call_stats` emitter pattern (on `&CallGraph`) | `src/navigation/queries.rs:12`; wired `src/main.rs:447` |
| **`NavigationIndex` holds only `cpg: CodePropertyGraph`** (no files, no provider) | `src/navigation/mod.rs:16-17` |
| **`CpgContext<'a>` holds `cpg` + `files: &BTreeMap<String,ParsedFile>`** (Slice-D emitter target) | `src/cpg/context.rs:39-43`; constructor `CpgContext::build(&files, None)` (`resolution_test.rs:119`) |
| Go provider interface internals are **private** (`GoTypeData.interfaces`, `GoInterface.methods`) | `src/type_providers/go.rs:67-75,126-130` → needs a public helper |
| Harness edge+resolution_kind extraction / fingerprint / **line-keyed adjudication** | `eval/tier_a/sut.py:78-109`, `adjudication.py:80` (`fingerprint`), `:26-45,80-126` (line-keyed + optional fingerprint) |

**Corrections baked into this plan (vs. the spec's / draft's illustrative text):**
1. **`ReceiverCtx` must also carry `recv_var` + `file_imports`** — the legacy gate (`call_graph.rs:1401-1402`)
   tests `is_recv`/`is_import`; the spec §2 sketch omitted them.
2. **No Go type-assertion precedent exists.** `taint.rs:~5567` handles the *TypeScript* `type_assertion`
   node, not Go's `type_assertion_expression`. The `child_by_field_name` idiom is precedented; the Go node
   is first handled in Slice B → grammar-pinning tests matter.
3. **`build` takes no config today** → the config is **additive** (`build` keeps its signature, delegates to
   `build_with_receiver_config`, default `Expanded`) so the ~20 existing `CallGraph::build` callers don't break.
4. **The engine is untouched by B/C.** The R6 relabel's `_ => TypedParam` arm absorbs the new variants for
   concrete `owner_lookup` hits; interface hits are already form-agnostic (`InterfaceDispatch`).
5. **Cache-test compile failure (blocker):** the `CACHE_VERSION` bump (Slice B) must also update
   `cache_version_is_9_for_interface_fields` (`cpg_cache.rs:511-512`) or `cargo test` fails.
6. **Slice-D emitter runs on `&CpgContext`, not `call_stats(&CallGraph)`** — `NavigationIndex` drops the
   files + provider that the §8a denominator needs. The "method ∈ some interface" set is captured **at build**
   onto `CallGraph` (a public `GoTypeProvider` helper feeds it), since the provider isn't retained post-build.
7. **`owner_key` does not strip Go `pkg.`** — an interface `pkg.Module` still routes (via `iface_key`), but a
   *concrete* `x.(pkg.T).M()` will **not** owner-resolve. Cross-package precise concrete keys are deferred (D2);
   the plan claims only same-package concrete + any interface assertion.

**Execution routing (owner-approved mods — handoff §"Execution approach"):**
- **Slice A: IN-SESSION via TDD** (may delegate to an opus/sonnet subagent). Integration-heavy refactor.
- **Slices B, C: a2a-bridge per slice**, model **sonnet[1m]** (config
  `~/code/a2a-bridge/examples/a2a-bridge.slicing-implement-s2xhigh.toml`, `--base-ref
  phase-ip-receiver-expansion`). Review each hand-off diff, cherry-pick onto the branch.
- **Slice D: bridge implements; operator runs `cargo test` (D-Rust) + `uv run pytest` (D-Python)** via a
  sonnet subagent — the bridge container can't run pytest. D is **two separately-committable** pieces.
- **Slice E: OWNER-GATED.** Not executed without the owner.
- **Frame every bridge task "as written OR BETTER (named axis: cleaner / tighter / more-consistent-with-
  codebase-idioms / better-tested / more-sound) + a no-new-scope guard"** (NOT "verbatim"). **New-scope ideas
  → `docs/superpowers/specs/2026-06-16-prism-phase-ip-receiver-expansion-deferred.md`** with a judgement call
  (do-now / dismiss / defer) — never silently expand scope.
- **Bake the file:line anchors above into each bridge task body.**
- **Bridge review leg is LIVE** (`a2a-bridge models` probe 2026-06-16: codex gpt-5.5 + claude `default`≈Opus,
  both up to xhigh) → use the **dual codex+claude** per-task review.
- **TDD per task** (failing test → confirm fail → implement → confirm pass → commit). **One commit per
  slice** (Slice D is the sanctioned exception — see its section). Commit messages end with
  `Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>`.

---

## File Structure

**New / modified source (Rust):**
- `src/resolution.rs` — **(modify)** add the seam vocabulary (`RecoveredReceiver`, `ReceiverCtx<'a>`,
  `trait ReceiverClassifier: Sync`, `LegacyClassifier`, `ExpandedClassifier`, `ReceiverRecoveryMode`,
  `ReceiverRecoveryConfig`, `legacy_recover`/`recover_simple_ident`); (B) `recover_type_assertion`; extend
  `ReceiverRecovery` with `TypeAssertion` (B), `VarDecl` (C), `SliceElem` (F).
- `src/call_graph.rs` — **(modify)** delete the free `recover_receiver`; add `build_with_receiver_config` +
  `build_direct_subset_with_receiver_config`; construct the classifier once before the par loop; build a
  `ReceiverCtx` at the two call sites; (D) add `pub interface_method_names: BTreeSet<String>`, populated at build.
- `src/ast.rs` — **(modify)** surface the receiver node from
  `function_calls_with_qualifier_and_spans_on_lines` (+ manual fallback); (C) add `recover_var: bool` to
  `receiver_type_in_fn` + `walk_receiver_bindings` and a Go `var_spec` arm.
- `src/cpg_cache.rs` — **(modify, Slice B)** bump `CACHE_VERSION` 9 → 10 + update the pinning test + a history line.
- `src/type_providers/go.rs` — **(modify, Slice D)** add `pub fn interface_method_names(&self) -> BTreeSet<String>`.
- `src/navigation/queries.rs` — **(modify, Slice D)** add `interface_dispatch_manifest(ctx: &CpgContext) -> serde_json::Value`.
- `src/main.rs` — **(modify, Slice D)** wire a `prism interface-manifest --repo <dir> [--format json]` subcommand.

**New tests (Rust):** `tests/integration/resolution_test.rs` — parity (A); type-assertion + grammar-pin (B);
var-decl + bail + per-form-gating config tests (C); manifest emitter test (D); `#[ignore]`d `SliceElem` (F).

**New fixtures (tier-A, auto-discovered by the matrix glob):**
- `eval/fixtures/go/interface_dispatch_assert/{main.go,expected.toml}` (B)
- `eval/fixtures/go/interface_dispatch_var/{main.go,expected.toml}` (C)

**New / modified harness (Python) + tests (Slice D):** `eval/tier_a/manifest.py` (new) + `eval/tests/test_manifest.py` (new).

**New doc (scope discipline):** `docs/superpowers/specs/2026-06-16-prism-phase-ip-receiver-expansion-deferred.md` (created in Slice A).

---

## Shared seam design (introduced in Slice A; referenced by all slices)

All in `src/resolution.rs`. `tree_sitter::Node<'a>` and shared refs are `Copy`, so `ReceiverCtx` is `Copy`.

```rust
/// S3 receiver-recovery: a syntactically-recovered static receiver type plus the
/// fact that recovered it. Routing happens downstream in `resolve_call_site`
/// (spec §2 recover-and-route).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecoveredReceiver {
    pub static_type: String,
    pub recovery: ReceiverRecovery,
}

/// Inputs a `ReceiverClassifier` needs. Borrows from the ParsedFile/tree of the
/// enclosing function. Carries `recv_var` + `file_imports` because the legacy gate
/// tests `is_recv`/`is_import` (call_graph.rs:1401-1402). Recover-and-route needs
/// NO GoTypeProvider here.
#[derive(Clone, Copy)]
pub struct ReceiverCtx<'a> {
    /// Receiver/selector-operand node (e.g. the `type_assertion_expression` in
    /// `x.(Module).M()`). `None` on the manual-fallback path / unqualified calls.
    pub receiver_expr: Option<tree_sitter::Node<'a>>,
    pub qualifier: Option<&'a str>,
    pub fn_node: tree_sitter::Node<'a>,
    pub call_line: usize,
    pub parsed: &'a crate::ast::ParsedFile,
    pub recv_var: Option<&'a str>,
    pub file_imports: Option<&'a std::collections::BTreeMap<String, String>>,
}

/// Swappable receiver-recovery strategy (strangler seam). `Sync` because the CPG
/// build extracts call sites with rayon (`call_graph.rs:339`).
pub trait ReceiverClassifier: Sync {
    fn classify(&self, ctx: ReceiverCtx<'_>) -> Option<RecoveredReceiver>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReceiverRecoveryMode { Legacy, Expanded }

/// Build-time config. Default = `Expanded` (all implemented forms on). `Legacy` is the
/// granular fall-back / parity-test mode. Per-form booleans allow `type_assertion_only`
/// / `var_local_only` selection (spec §13.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReceiverRecoveryConfig {
    pub mode: ReceiverRecoveryMode,
    pub type_assertion: bool,
    pub var_local: bool,
}
impl Default for ReceiverRecoveryConfig {
    fn default() -> Self {
        Self { mode: ReceiverRecoveryMode::Expanded, type_assertion: true, var_local: true }
    }
}
impl ReceiverRecoveryConfig {
    pub fn legacy() -> Self {
        Self { mode: ReceiverRecoveryMode::Legacy, type_assertion: false, var_local: false }
    }
    pub fn classifier(&self) -> Box<dyn ReceiverClassifier> {
        match self.mode {
            ReceiverRecoveryMode::Legacy => Box::new(LegacyClassifier),
            ReceiverRecoveryMode::Expanded => Box::new(ExpandedClassifier {
                type_assertion: self.type_assertion,
                var_local: self.var_local,
            }),
        }
    }
}
```

The legacy recovery, extracted **verbatim** from `call_graph::recover_receiver`. Slice C generalizes this to
`recover_simple_ident(ctx, recover_var)`; in Slice A it is exactly:

```rust
/// PR-1 P6-lite recovery, extracted verbatim from the former `call_graph::recover_receiver`.
pub fn legacy_recover(ctx: &ReceiverCtx<'_>) -> Option<RecoveredReceiver> {
    use crate::languages::Language;
    if !matches!(ctx.parsed.language, Language::Rust | Language::Go) {
        return None;
    }
    let q = ctx.qualifier?;
    let simple = !q.is_empty() && q.chars().all(|c| c.is_alphanumeric() || c == '_');
    let is_kw = matches!(q, "self" | "this" | "cls");
    let is_recv = ctx.recv_var == Some(q);
    let is_import = ctx.file_imports.map(|m| m.contains_key(q)).unwrap_or(false);
    if !(simple && !is_kw && !is_recv && !is_import) {
        return None;
    }
    ctx.parsed
        .receiver_type_in_fn(&ctx.fn_node, q, ctx.call_line)
        .map(|(ty, how)| RecoveredReceiver {
            static_type: owner_key(&peel_type(&ty)),
            recovery: how,
        })
}

pub struct LegacyClassifier;
impl ReceiverClassifier for LegacyClassifier {
    fn classify(&self, ctx: ReceiverCtx<'_>) -> Option<RecoveredReceiver> { legacy_recover(&ctx) }
}

pub struct ExpandedClassifier { pub type_assertion: bool, pub var_local: bool }
impl ReceiverClassifier for ExpandedClassifier {
    fn classify(&self, ctx: ReceiverCtx<'_>) -> Option<RecoveredReceiver> {
        // Slice A: identical to legacy (no forms yet). Slices B/C extend this body.
        legacy_recover(&ctx)
    }
}
```

> **Recover-and-route ordering:** for `x.(Module).M()` the qualifier text is `x.(Module)` → `legacy_recover`
> fails its simple-ident gate → the type-assertion arm (B) fires on `receiver_expr`. For `var r Runner;
> r.M()` the qualifier `r` passes the gate but `receiver_type_in_fn` finds no param/binding match → `None` →
> the var arm (C) fires. Both recovered types are normalized with `owner_key(peel_type(..))` — **identical to
> legacy** — so the existing R6 router sends an interface to `interface_impls` and a same-package concrete to
> `owner_lookup` with no new code.

---

## Slice A — `ReceiverClassifier` seam + `legacy` parity + extraction-API change (IN-SESSION, TDD)

**MANDATORY FIRST. Pure refactor — byte-identical resolution.** Lands alone before any form.

**Files:** modify `src/resolution.rs`, `src/call_graph.rs`, `src/ast.rs`; test `tests/integration/resolution_test.rs`; create the deferred doc.

- [ ] **A1. Write the failing parity test.** Add a `build_cfg` helper next to `build` in
  `tests/integration/resolution_test.rs`, then:

```rust
fn build_cfg(
    sources: &[(&str, &str, prism::languages::Language)],
    cfg: &prism::resolution::ReceiverRecoveryConfig,
) -> (CallGraph, BTreeMap<String, prism::ast::ParsedFile>) {
    let mut files = BTreeMap::new();
    for (path, src, lang) in sources {
        files.insert(path.to_string(), prism::ast::ParsedFile::parse(path, src, *lang).unwrap());
    }
    (CallGraph::build_with_receiver_config(&files, cfg), files)
}

#[test]
fn slice_a_legacy_parity_p6_typed_param() {
    use prism::languages::Language::Go;
    use prism::resolution::{ReceiverRecovery, ReceiverRecoveryConfig};
    let legacy = build_cfg(&[("main.go", go_iface_src(), Go)], &ReceiverRecoveryConfig::legacy());
    let expanded = build_cfg(&[("main.go", go_iface_src(), Go)], &ReceiverRecoveryConfig::default());
    for (cg, _) in [&legacy, &expanded] {
        let site = site_in(cg, "run", "Go");
        assert_eq!(site.receiver_type.as_deref(), Some("Runner"));
        assert_eq!(site.receiver_recovery, Some(ReceiverRecovery::TypedParam));
        let r = cg.resolve_call_site(&site);
        assert_eq!(r.len(), 2);
        assert!(r.iter().all(|c| c.kind == ResolutionKind::InterfaceDispatch));
    }
}
```

- [ ] **A2. Confirm fail-to-compile.** `cargo test --test integration resolution_test::slice_a_legacy_parity_p6_typed_param`
  → FAIL (`build_with_receiver_config` / `ReceiverRecoveryConfig` undefined).

- [ ] **A3. Add the seam vocabulary to `src/resolution.rs`** — the full "Shared seam design" block above.
  Keep `ReceiverRecovery` at 2 variants.

- [ ] **A4. Surface the receiver node from the extractor** (`src/ast.rs:3515`). Add the lifetime + a 6th
  tuple element; reuse the qualifier node:

```rust
pub fn function_calls_with_qualifier_and_spans_on_lines<'a>(
    &'a self,
    func_node: &Node<'a>,
    lines: &BTreeSet<usize>,
) -> Vec<(String, usize, Option<String>, usize, usize, Option<Node<'a>>)> {
    // query path push block (was ast.rs:3543-3550):
    if let Some(name_node) = self.language.call_function_name(&capture.node) {
        let name = self.node_text(&name_node).to_string();
        let qualifier_node = self.language.call_function_qualifier(&capture.node);
        let qualifier = qualifier_node.map(|q| self.node_text(&q).to_string());
        calls.push((name, line, qualifier,
                    capture.node.start_byte(), capture.node.end_byte(), qualifier_node));
    }
}
```
  Update the manual fallback `collect_calls_manual_with_qualifier_and_spans` (`ast.rs:3563`) to the 6-tuple;
  it may push `None` for the node (type-assertion recovery is Go-only and Go uses the query path — document
  this asymmetry in a comment). Fix any other extractor callers the compiler surfaces.

- [ ] **A5. Rewire `src/call_graph.rs`.** Delete the free `recover_receiver` (`:1384-1412`). Add the config
  split (`build` delegates; classifier built once, `Sync`, captured by the par closure):

```rust
pub fn build(files: &BTreeMap<String, ParsedFile>) -> Self {
    Self::build_with_receiver_config(files, &crate::resolution::ReceiverRecoveryConfig::default())
}
pub fn build_with_receiver_config(
    files: &BTreeMap<String, ParsedFile>,
    receiver_config: &crate::resolution::ReceiverRecoveryConfig,
) -> Self {
    let classifier = receiver_config.classifier();
    let classifier: &dyn crate::resolution::ReceiverClassifier = classifier.as_ref();
    // ...existing body; the par_iter closure captures `classifier` (Sync Copy ref).
}
```
  Same `_with_receiver_config` split for `build_direct_subset` (`:918`). At each call site (`:366-391`,
  `:1016-1045`) the loop destructures the 6-tuple and **preserves the existing qualifier reassignment**:

```rust
for (callee_name, line, qualifier, start_byte, end_byte, receiver_expr) in call_sites {
    let qualifier = Self::recover_self_receiver_qualifier(parsed, &callee_name, line, qualifier);
    let recovered = classifier.classify(crate::resolution::ReceiverCtx {
        receiver_expr,
        qualifier: qualifier.as_deref(),
        fn_node: func_node,            // owned Node from all_functions() (Copy)
        call_line: line,
        parsed,
        recv_var: recv_var.as_deref(),
        file_imports: file_imports_ref,
    });
    // ...
    receiver_type: recovered.as_ref().map(|r| r.static_type.clone()),
    receiver_recovery: recovered.as_ref().map(|r| r.recovery),
}
```

- [ ] **A6. Confirm pass.** `cargo test --test integration resolution_test::` → PASS, including the
  pre-existing `interface_dispatch_*` / `iface_key_*` / `r1_*` tests (the real parity gate).

- [ ] **A7. Full guard + matrix.** `cargo fmt && cargo build && cargo test && cargo test --features mcp`;
  `cargo build --release && (cd eval && uv run tier-a --matrix-only --allow-stale-sut)` — **no flips**.

- [ ] **A8. Create the deferred-scope doc** `docs/superpowers/specs/2026-06-16-prism-phase-ip-receiver-expansion-deferred.md`,
  seeded with the recorded deferrals (package-level `var`, cross-package concrete `pkg.T` keys, optional
  `--receiver-recovery` CLI flag — see "Deferred decisions" below), each with priority/why/impact/fix-sketch.

- [ ] **A9. Commit (one commit, Slice A):** `feat(phase-ip): PR-2 Slice A — ReceiverClassifier seam + legacy
  parity + receiver-node extraction` + trailer.

---

## Slice B — `TypeAssertion` form (a2a-bridge, sonnet[1m], TDD)

`x.(Module).M()` (the 57 caddy sites). Adds `ReceiverRecovery::TypeAssertion`, the classifier arm, the cache
bump (+ test/history fix), tests, and a tier-A fixture. **Engine untouched.**

**Bridge task framing:** *"Implement Slice B as written **or better** (named axis: cleaner / tighter / more-
consistent / better-tested / more-sound), without expanding scope, changing the public contract, or
contradicting the plan/spec/other slices; note any deviation + rationale; **new-scope ideas →
receiver-expansion-deferred.md with a do-now/dismiss/defer call**. Pin the tree-sitter-go 0.23.4 grammar via
tests — there is NO existing Go type-assertion precedent (`taint.rs:~5567` is the TypeScript node)."* Bake in
the anchors from the orientation table.

**Files:** modify `src/resolution.rs`, `src/cpg_cache.rs`, `tests/integration/resolution_test.rs`; create `eval/fixtures/go/interface_dispatch_assert/{main.go,expected.toml}`.

- [ ] **B1. Failing tests** (`tests/integration/resolution_test.rs`):

```rust
fn go_assert_src() -> &'static str {
    "package main\n\
     type Runner interface { Go() }\n\
     type Fast struct{}\nfunc (f Fast) Go() {}\n\
     func use() { _ = Fast{} }\n\
     func run(x any) { x.(Runner).Go() }\n"
}

#[test]
fn type_assertion_interface_receiver_dispatches_exact() {
    use prism::languages::Language::Go;
    use prism::resolution::ReceiverRecovery;
    let (cg, _) = build(&[("main.go", go_assert_src(), Go)]);
    let site = site_in(&cg, "run", "Go");
    assert_eq!(site.receiver_type.as_deref(), Some("Runner"));
    assert_eq!(site.receiver_recovery, Some(ReceiverRecovery::TypeAssertion));
    let r = cg.resolve_call_site(&site);
    assert_eq!(r.len(), 1);                              // assert BEFORE .all() (no vacuous pass)
    assert!(r.iter().all(|c| c.kind == ResolutionKind::InterfaceDispatch));
}

#[test]
fn type_assertion_concrete_pointer_receiver_owner_resolves() {
    use prism::languages::Language::Go;
    let (cg, _) = build(&[(
        "main.go",
        "package main\n\
         type Fast struct{}\nfunc (f Fast) Go() {}\n\
         func run(x any) { x.(*Fast).Go() }\n",
        Go,
    )]);
    let site = site_in(&cg, "run", "Go");
    assert_eq!(site.receiver_type.as_deref(), Some("Fast"));   // owner_key peels the '*'
    let r = cg.resolve_call_site(&site);
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].target.name, "Go");
}

#[test]
fn type_assertion_comma_ok_is_not_a_call_receiver() {
    use prism::languages::Language::Go;
    use prism::resolution::ReceiverRecovery;
    let (cg, _) = build(&[(
        "main.go",
        "package main\n\
         type Runner interface { Go() }\n\
         func run(x any) { v, ok := x.(Runner); _ = ok; _ = v }\n",
        Go,
    )]);
    if let Some((_, sites)) = cg.calls.iter().find(|(fid, _)| fid.name == "run") {
        assert!(sites.iter().all(|c| c.receiver_recovery != Some(ReceiverRecovery::TypeAssertion)));
    }
}

#[test]
fn type_assertion_grammar_pin_normalization() {
    // Pin Module / pkg.Module / *T / (T) recovery against tree-sitter-go 0.23.4.
    use prism::languages::Language::Go;
    use prism::resolution::ReceiverRecovery;
    let cases = [
        ("x.(Runner).Go()", "Runner"),
        ("x.(pkg.Runner).Go()", "pkg.Runner"),  // owner_key keeps pkg.; iface_key strips at route time
        ("x.(*Fast).Go()", "Fast"),
        ("x.((Runner)).Go()", "Runner"),         // parenthesized_type unwrapped
    ];
    for (call, want) in cases {
        let src = format!("package main\nfunc run(x any) {{ {call} }}\n");
        let (cg, _) = build(&[("main.go", Box::leak(src.into_boxed_str()), Go)]);
        let site = site_in(&cg, "run", "Go");
        assert_eq!(site.receiver_type.as_deref(), Some(want), "call {call}");
        assert_eq!(site.receiver_recovery, Some(ReceiverRecovery::TypeAssertion), "call {call}");
    }
}
```

- [ ] **B2. Confirm fail.** `cargo test --test integration resolution_test::type_assertion_` → FAIL.

- [ ] **B3. Add the variant** `src/resolution.rs` (additive, trailing): `ReceiverRecovery::TypeAssertion`.

- [ ] **B4. Add `recover_type_assertion` + wire into `ExpandedClassifier`** (`src/resolution.rs`):
```rust
fn recover_type_assertion(ctx: &ReceiverCtx<'_>) -> Option<RecoveredReceiver> {
    use crate::languages::Language;
    if ctx.parsed.language != Language::Go { return None; }
    let node = ctx.receiver_expr?;
    if node.kind() != "type_assertion_expression" { return None; }
    let mut ty = node.child_by_field_name("type")?;
    while ty.kind() == "parenthesized_type" { ty = ty.named_child(0)?; }
    // Same normalization as the legacy path → consistent routing: an interface
    // (`Runner`/`pkg.Module`) routes via iface_key→interface_impls; a same-package
    // concrete (`*Fast`) owner_lookup-resolves. Cross-package concrete `pkg.T` does
    // NOT owner-resolve (owner_key keeps `pkg.`) — deferred (D2).
    let static_type = owner_key(&peel_type(ctx.parsed.node_text(&ty)));
    if static_type.is_empty() { return None; }
    Some(RecoveredReceiver { static_type, recovery: ReceiverRecovery::TypeAssertion })
}

// ExpandedClassifier::classify:
fn classify(&self, ctx: ReceiverCtx<'_>) -> Option<RecoveredReceiver> {
    if let Some(r) = legacy_recover(&ctx) { return Some(r); }
    if self.type_assertion {
        if let Some(r) = recover_type_assertion(&ctx) { return Some(r); }
    }
    None
}
```

- [ ] **B5. Bump the cache version + fix its pinning test** (the blocker):
  - `src/cpg_cache.rs:50`: `const CACHE_VERSION: u32 = 10;` and add a `// v10: PR-2 ReceiverRecovery
    variants (TypeAssertion/VarDecl/SliceElem) + interface_method_names` history line near `:40-50`.
  - `src/cpg_cache.rs:511-512`: update the test — rename `cache_version_is_9_for_interface_fields` →
    `cache_version_is_10_for_phase_ip_pr2` and `assert_eq!(super::CACHE_VERSION, 10);`.
  - `CACHE_VERSION` stays 10 for C/D/F: built binaries self-invalidate per commit via `GIT_SHA`
    (`cpg_cache.rs:300`); dirty dev iteration uses `--no-cache` (`:298-300`). One bump for all of PR-2.

- [ ] **B6. Confirm pass.** `cargo test --test integration resolution_test::type_assertion_` + `cargo test --test ast cpg_cache_test::` → PASS.

- [ ] **B7. tier-A fixture** `eval/fixtures/go/interface_dispatch_assert/main.go` (the `go_assert_src` shape
  written to disk) + `expected.toml`. **The matrix keys callers by call-site line; the asserted call is line
  14, the `Fast.Go` def is line 9:**
```toml
[case]
language = "go"
capability = "interface_dispatch_assert"
status = "pass"
[seed]
symbol = "Go"
file = "main.go"
line = 9
[[expect.callers]]
file = "main.go"
line = 14
[expect]
exact = true
```
  (`main.go`: `package main`=1, blank=2, `type Runner interface {`=3, `Go()`=4, `}`=5, blank=6, `type Fast
  struct{}`=7, blank=8, `func (f Fast) Go() {}`=9, blank=10, `func use() { _ = Fast{} }`=11, blank=12,
  `func run(x any) {`=13, `x.(Runner).Go()`=14, `}`=15.)

- [ ] **B8. Verify guard + matrix.** As A7; the new `go/interface_dispatch_assert` is **ok**, no other flips.

- [ ] **B9. Commit (one commit, Slice B):** `feat(phase-ip): PR-2 Slice B — type-assertion receiver recovery (x.(Module).M())` + trailer.

---

## Slice C — `VarDecl` form (a2a-bridge, sonnet[1m], TDD)

`var r Runner` / `var r Runner = f()`. Recovers the declared type, routed per recover-and-route. Reuses the
binding bail discipline via a `recover_var` flag (keeps `legacy` byte-identical when off). **Intra-function
scope only** (package-level `var` is deferred — see "Deferred decisions").

**Bridge task framing:** same "as written OR BETTER + no-new-scope + deferred-doc" wording. Pin `var_spec`
grammar fields against tree-sitter-go 0.23.4 via tests (`var_spec.name` is `multiple:true`).

**Files:** modify `src/resolution.rs`, `src/ast.rs`, `tests/integration/resolution_test.rs`; create `eval/fixtures/go/interface_dispatch_var/{main.go,expected.toml}`.

- [ ] **C1. Failing tests** (`tests/integration/resolution_test.rs`):
```rust
#[test]
fn var_local_interface_receiver_dispatches_exact() {
    use prism::languages::Language::Go;
    use prism::resolution::ReceiverRecovery;
    let (cg, _) = build(&[(
        "main.go",
        "package main\n\
         type Runner interface { Go() }\n\
         type Fast struct{}\nfunc (f Fast) Go() {}\n\
         func use() { _ = Fast{} }\n\
         func run() { var r Runner; r.Go() }\n",
        Go,
    )]);
    let site = site_in(&cg, "run", "Go");
    assert_eq!(site.receiver_type.as_deref(), Some("Runner"));
    assert_eq!(site.receiver_recovery, Some(ReceiverRecovery::VarDecl));
    let r = cg.resolve_call_site(&site);
    assert_eq!(r.len(), 1);                              // assert BEFORE .all()
    assert!(r.iter().all(|c| c.kind == ResolutionKind::InterfaceDispatch));
}

#[test]
fn var_local_concrete_receiver_owner_resolves() {
    use prism::languages::Language::Go;
    let (cg, _) = build(&[(
        "main.go",
        "package main\n\
         type Fast struct{}\nfunc (f Fast) Go() {}\n\
         func run() { var r Fast; r.Go() }\n",
        Go,
    )]);
    let site = site_in(&cg, "run", "Go");
    assert_eq!(site.receiver_type.as_deref(), Some("Fast"));
    assert_eq!(cg.resolve_call_site(&site).len(), 1);
}

#[test]
fn var_local_shadowed_binding_bails() {
    use prism::languages::Language::Go;
    let (cg, _) = build(&[(
        "main.go",
        "package main\n\
         type Runner interface { Go() }\n\
         func run(x Runner) { var r Runner; r = x; r.Go() }\n",
        Go,
    )]);
    assert_eq!(site_in(&cg, "run", "Go").receiver_type, None);  // >1 binding → bail
}

#[test]
fn var_local_false_name_in_initializer_not_recovered() {
    // The receiver `f` appears only in the initializer, never as the bound name.
    use prism::languages::Language::Go;
    let (cg, _) = build(&[(
        "main.go",
        "package main\n\
         type Runner interface { Go() }\n\
         func run() { var r Runner = f(); f.Go() }\n",
        Go,
    )]);
    // `f.Go()` must NOT recover `f` as Runner.
    if let Some((_, sites)) = cg.calls.iter().find(|(fid, _)| fid.name == "run") {
        for s in sites.iter().filter(|s| s.qualifier.as_deref() == Some("f")) {
            assert_eq!(s.receiver_type, None);
        }
    }
}

#[test]
fn var_local_off_in_legacy_mode() {
    use prism::languages::Language::Go;
    use prism::resolution::ReceiverRecoveryConfig;
    let (cg, _) = build_cfg(&[(
        "main.go",
        "package main\n\
         type Runner interface { Go() }\n\
         func run() { var r Runner; r.Go() }\n",
        Go,
    )], &ReceiverRecoveryConfig::legacy());
    assert_eq!(site_in(&cg, "run", "Go").receiver_type, None);
}

// §12 per-form gating: the ExpandedClassifier booleans must be exercised independently.
#[test]
fn config_var_local_only_gates_type_assertion_off() {
    use prism::languages::Language::Go;
    use prism::resolution::{ReceiverRecovery, ReceiverRecoveryConfig, ReceiverRecoveryMode};
    let cfg = ReceiverRecoveryConfig { mode: ReceiverRecoveryMode::Expanded, type_assertion: false, var_local: true };
    let (cg, _) = build_cfg(&[(
        "main.go",
        "package main\n\
         type Runner interface { Go() }\n\
         func a(x any) { x.(Runner).Go() }\n\
         func b() { var r Runner; r.Go() }\n",
        Go,
    )], &cfg);
    assert_eq!(site_in(&cg, "a", "Go").receiver_type, None);                    // type-assertion OFF
    assert_eq!(site_in(&cg, "b", "Go").receiver_recovery, Some(ReceiverRecovery::VarDecl)); // var ON
}

#[test]
fn config_type_assertion_only_gates_var_local_off() {
    use prism::languages::Language::Go;
    use prism::resolution::{ReceiverRecovery, ReceiverRecoveryConfig, ReceiverRecoveryMode};
    let cfg = ReceiverRecoveryConfig { mode: ReceiverRecoveryMode::Expanded, type_assertion: true, var_local: false };
    let (cg, _) = build_cfg(&[(
        "main.go",
        "package main\n\
         type Runner interface { Go() }\n\
         func a(x any) { x.(Runner).Go() }\n\
         func b() { var r Runner; r.Go() }\n",
        Go,
    )], &cfg);
    assert_eq!(site_in(&cg, "a", "Go").receiver_recovery, Some(ReceiverRecovery::TypeAssertion)); // assert ON
    assert_eq!(site_in(&cg, "b", "Go").receiver_type, None);                    // var OFF
}
```

- [ ] **C2. Confirm fail.** `cargo test --test integration resolution_test::var_local_ resolution_test::config_` → FAIL.

- [ ] **C3. Add the variant** `src/resolution.rs`: append `VarDecl` (after `TypeAssertion`).

- [ ] **C4. Thread `recover_var` into the legacy scan + add the `var_spec` arm** (`src/ast.rs`). Add
  `recover_var: bool` to `receiver_type_in_fn` (`:313`) and `walk_receiver_bindings` (`:3816`, pass through
  the recursion). Add the Go `var_spec` arm — **match only `name`-field children** (NOT `node_binds_name`
  over the whole node, which would match names in the type/initializer):
```rust
(Language::Go, "var_spec") if recover_var => {
    // var_spec.name is multiple:true; match the bound name(s) only.
    let mut cur = node.walk();
    let matched = node
        .children_by_field_name("name", &mut cur)
        .any(|n| self.simple_binding_text(&n).as_deref() == Some(receiver));
    if matched {
        *bindings += 1;
        if let Some(ty) = node.child_by_field_name("type") {
            *found = Some((self.node_text(&ty).to_string(), ReceiverRecovery::VarDecl));
        } else if let Some(value) = node.child_by_field_name("value") {
            // single-constructor initializer only; multi-value expr_list → None (safe)
            *found = self.constructor_type(&value)
                .or_else(|| self.first_constructor_type_child(&value))
                .map(|ty| (ty, ReceiverRecovery::VarDecl));
        } else {
            *found = None;
        }
    }
}
```
  Update `receiver_type_in_fn`'s `walk_receiver_bindings(...)` call to pass `recover_var`.

- [ ] **C5. Make the classifier var-aware** (`src/resolution.rs`). Generalize `legacy_recover` to a private
  `recover_simple_ident(ctx, recover_var)` (gate + `receiver_type_in_fn(.., recover_var)` + peel/key);
  `legacy_recover(ctx) = recover_simple_ident(ctx, false)`. `ExpandedClassifier::classify`:
```rust
fn classify(&self, ctx: ReceiverCtx<'_>) -> Option<RecoveredReceiver> {
    if let Some(r) = recover_simple_ident(&ctx, self.var_local) { return Some(r); }
    if self.type_assertion {
        if let Some(r) = recover_type_assertion(&ctx) { return Some(r); }
    }
    None
}
```

- [ ] **C6. Confirm pass + parity intact.** `cargo test --test integration resolution_test::` → PASS (incl.
  `slice_a_legacy_parity_*`, `var_local_off_in_legacy_mode`, the two `config_*` tests).

- [ ] **C7. tier-A fixture** `eval/fixtures/go/interface_dispatch_var/main.go`:
```go
package main

type Runner interface {
	Go()
}

type Fast struct{}

func (f Fast) Go() {}

func use() { _ = Fast{} }

func run() {
	var r Runner
	r.Go()
}
```
  + `expected.toml` (call-site-line convention: seed `Fast.Go`=line 9, caller `r.Go()`=line 15):
```toml
[case]
language = "go"
capability = "interface_dispatch_var"
status = "pass"
[seed]
symbol = "Go"
file = "main.go"
line = 9
[[expect.callers]]
file = "main.go"
line = 15
[expect]
exact = true
```

- [ ] **C8. Verify guard + matrix.** As A7; the new `go/interface_dispatch_var` is **ok**, no other flips.

- [ ] **C9. Commit (one commit, Slice C):** `feat(phase-ip): PR-2 Slice C — var-declared local receiver recovery (var r Runner)` + trailer.

---

## Slice D — in-scope manifest + precision gate **report** (bridge implements; operator runs cargo test + pytest)

**Two separately-committable pieces** (the review's BLOCKER #2 restructure): **D-Rust** (the structural
manifest emitter — the spec §14b "separate AST source") and **D-Python** (the report harness). The gate is a
**report, never gating** in PR-2; `corrected_fp` is *provisional* until Slice E.

### D-Rust (commit 1) — manifest emitter on `&CpgContext`

The emitter needs the ParsedFiles + the interface set. `NavigationIndex` drops both, but `CpgContext` carries
`files`, and the "method ∈ some interface" set is captured **at build** onto `CallGraph` (the provider is live
there). So: a new `CallGraph` field + a public provider helper + an emitter on `&CpgContext` + a subcommand.

- [ ] **D-R1. (Operator)** Re-read `src/navigation/queries.rs:12` (call_stats), `src/main.rs` CLI wiring,
  `src/cpg/context.rs`, and `src/type_providers/go.rs` interface internals; finalize the exact anchors and
  write the bridge task body with them + the "as written OR BETTER" framing.
- [ ] **D-R2. (Bridge, TDD)** Add the interface-method-name set:
  - `src/type_providers/go.rs`: `pub fn interface_method_names(&self) -> std::collections::BTreeSet<String>`
    (union of `GoInterface.methods` keys over `GoTypeData.interfaces`). *Red test:* the helper returns
    `{"Go"}` for a one-interface fixture.
  - `src/call_graph.rs`: add `pub interface_method_names: BTreeSet<String>` to `CallGraph`; populate it where
    the provider is live during build (alongside `interface_impls`); init it in every `CallGraph { .. }`
    literal. *Red test:* after `build`, `cg.interface_method_names` contains `"Go"` for the iface fixture.
- [ ] **D-R3. (Bridge, TDD)** Add `interface_dispatch_manifest(ctx: &CpgContext) -> serde_json::Value` to
  `src/navigation/queries.rs`. For each `(caller, sites)` in `ctx.cpg.call_graph.calls`, for each `site`
  whose `receiver_recovery ∈ {TypedParam, ConstructorLocal, TypeAssertion, VarDecl}` **AND** whose
  `callee_name ∈ ctx.cpg.call_graph.interface_method_names`, emit a record:
  ```json
  {"file": "...", "start_byte": 0, "end_byte": 0, "line": 0,
   "receiver_class": "type_assertion", "method": "Go", "fanout": 2}
  ```
  (`fanout` = implementer count from `interface_impls` for the routed interface, else 0/absent for concrete.)
  Output `{"corpus": null, "sites": [...]}`. *Red test* (`tests/integration/resolution_test.rs` or a new
  `manifest_test`): build a `CpgContext` over `go_assert_src()`, call the emitter, assert one
  `type_assertion` record with the right method + byte span; assert a non-interface method call is **excluded**.
- [ ] **D-R4. (Bridge, TDD)** Wire `prism interface-manifest --repo <dir> [--format json]` in `src/main.rs`:
  build files → `CpgContext::build(&files, None)` (force-fresh; eval runs `--no-cache`) → print
  `interface_dispatch_manifest(&ctx)`. *Red test:* a CLI test (`tests/cli/`) runs it on a Go fixture dir and
  asserts a JSON `sites` array.
- [ ] **D-R5. `slice_candidate` (§5 manifest-only class) — scoped sub-step.** Add an AST pass over
  `ctx.files` recognizing `for _, r := range xs { r.M() }` receiver sites (unrecovered) and emit them with
  `receiver_class = "slice_candidate"`. *Red test:* a range fixture yields one `slice_candidate` record. **If
  this balloons, record it in the deferred doc and ship D-Rust without it** — the recovered classes (the main
  denominator) do not depend on it.
- [ ] **D-R6. Verify.** (Operator, via sonnet subagent) `cargo fmt && cargo build && cargo test && cargo test
  --features mcp`; run `prism interface-manifest` on a Go fixture and eyeball the JSON.
- [ ] **D-R7. Commit:** `feat(phase-ip): PR-2 Slice D-Rust — interface-dispatch in-scope manifest emitter` + trailer.

### D-Python (commit 2) — `eval/tier_a/manifest.py` + the gate report

- [ ] **D-P1. (Bridge, TDD)** `eval/tier_a/manifest.py`:
  - `@dataclass ManifestSite(file, start_byte, end_byte, line, receiver_class, method, fanout)`;
    `load_manifest(path) -> list[ManifestSite]`.
  - `byte_key(s) -> str` = `f"{s.file}:{s.start_byte}:{s.end_byte}"` (primary key; `file:line` display only).
  - `stratify(sites) -> dict[str, list[ManifestSite]]` by `receiver_class` (incl. `slice_candidate`).
  - `gate_report(sites, prism_only_keys, adjudications, corpus, direction) -> list[dict]` per
    `receiver_class`: `{corpus, direction, receiver_class, raw_fp, corrected_fp, pending, ambiguous,
    fanout_width}`. **FP rule:** `raw_fp` = prism-only sites; `corrected_fp` = prism-only after adjudication
    **excluding** `ambiguous`/`oracle_artifact`; `pending` = unadjudicated; `fanout_width` = mean `fanout`.
  - **Adjudication join:** `adjudications.jsonl` is **line-keyed with optional fingerprint**
    (`adjudication.py:26-45`); join each byte-span site to a verdict by `(file, line)` + `fingerprint`
    (`adjudication.fingerprint` over the call line ±1) for drift re-anchoring.
- [ ] **D-P2. (Bridge, TDD)** `eval/tests/test_manifest.py`: byte-span keying; stratification (incl.
  `slice_candidate`); the gate-report FP rule with synthetic adjudications (distinct `raw_fp` vs `corrected_fp`
  vs `pending` vs `ambiguous`); the line+fingerprint join. Each test red-first.
- [ ] **D-P3. Verify.** (Operator, via sonnet subagent) `cd eval && uv run pytest` → green.
- [ ] **D-P4. Commit:** `feat(phase-ip): PR-2 Slice D-Python — in-scope manifest reader + precision gate report` + trailer.

### Slice F (folded as commit 3) — reserved `SliceElem`

- [ ] **F1. (Bridge or in-session)** Append `ReceiverRecovery::SliceElem` (trailing; classifier stays
  `None`). Add an `#[ignore]`d pin:
```rust
#[test]
#[ignore = "SliceElem is reserved (spec §5/§10); classifier returns None until a future slice"]
fn slice_elem_variant_reserved() {
    let _ = prism::resolution::ReceiverRecovery::SliceElem;  // compiles → variant exists
}
```
  `CACHE_VERSION` stays 10 (GIT_SHA covers built; dirty uses `--no-cache`).
- [ ] **F2. Commit:** `feat(phase-ip): PR-2 Slice F — reserve ReceiverRecovery::SliceElem (sketch only)` + trailer.

---

## Slice E — caddy re-adjudication + 5-corpus re-baseline (OWNER-GATED — not executed here)

Once D resolves the 57 caddy sites: (1) re-adjudicate via the dual-adjudicator protocol (codex + claude),
record Cohen's κ, re-anchor stale verdicts by fingerprint; (2) re-baseline caddy — full `uv run tier-a
--corpus all` + a deliberate anchor update in `docs/eval/tier-a/` with the adjudication record. **Multi-corpus
runs are human-triggered.** The operator coordinates an **opus 4.8 xtra-high + codex gpt-5.5 high**
adjudication session via a2a-bridge when the owner triggers it.

---

## Deferred decisions (recorded now; seeded into the deferred doc in A8)

- **Package-level `var r Runner`** (spec §4): `walk_receiver_bindings` is rooted at the enclosing function
  (`ast.rs:368-375`), so a file-scope `var` is never in scope. **Deferred** — *priority Low; impact ~zero
  (caddy's 57 sites are all type-assertion); fix-sketch: a file-root declaration scan with shadowing
  semantics + a test.*
- **Cross-package concrete `x.(pkg.T).M()` keys** (D2): `owner_key` keeps `pkg.`, so concrete cross-package
  assertions don't owner-resolve. **Deferred** — *priority Low; impact narrow (interface `pkg.Module` already
  routes via iface_key); fix-sketch: a Go-aware bare-name normalizer + collision handling.*
- **`--receiver-recovery` CLI flag** (revertability ergonomics): build-time `ReceiverRecoveryConfig` already
  satisfies spec §10's "revertable to legacy." **Optional** — *a runtime flag would disable a form without a
  rebuild after the §8b report; add only if the report motivates it.*

---

## Acceptance (after the engine slices A–D)

- [ ] `cargo fmt --check`; `cargo test`; `cargo test --features mcp`; `cargo build --release` — all green.
- [ ] `cd eval && uv run tier-a --matrix-only --allow-stale-sut` — `go/interface_dispatch_assert` and
  `go/interface_dispatch_var` are **ok**; **no other flips** (pre-existing `target-c-method` flip_candidate is
  unrelated, per PR-1).
- [ ] `cd eval && uv run tier-a --quick --allow-stale-sut` — capability matrix all ok; `baseline_invalid`
  (exit 2) is the **expected** stale-baseline signal. **Do NOT re-baseline outside Slice E.**
- [ ] `cd eval && uv run pytest` — harness tests green (incl. new `test_manifest.py`).
- [ ] **Doc hygiene:** update `CLAUDE.md:178-186` — drop "Go embedding promotion, Go interface satisfaction"
  from the deferred-gaps list (shipped #95/#96) and note PR-2 widens receiver recovery (type-assertion +
  var-local) behind the `ReceiverClassifier` seam.
- [ ] **Whole-branch dual review** vs `main`: generate `git diff main..HEAD` and instruct the reviewers to
  review the **entire** branch diff. codex **gpt-5.5 xhigh** + claude/Opus **xhigh** via a2a-bridge. Fix
  blockers/criticals + low-risk findings; defer large/risky findings into the deferred doc with
  priority/why/impact/fix-sketch.
- [ ] **Report to the owner** — the owner opens/merges the PR (PR-1 norm). Slice E (caddy re-baseline) is
  owner-gated and follows.

---

## Self-Review (author, rev 2)

**Spec coverage:** §1 scope → B/C/F; §2 seam + recover-and-route + extraction-API → A; §2 wire/cache → B5
(variants additive; cache 9→10 + test fixed); §3 type-assertion (grammar pinned, comma-ok excluded;
`Module`/`pkg.Module`/`*T`/`(T)` normalized; concrete-`pkg.T` deferred) → B; §4 var-local (declared type,
bail rule reused, no interface predicate; package-level deferred) → C; §5 SliceElem sketch + manifest-only
`slice_candidate` → F + D-R5; §6 dispatch unchanged → engine untouched; §7 confidence unchanged; §8a manifest
(byte-span keys + denominator predicate, on `&CpgContext`) → D-Rust; §8b gate report (FP rule, report-not-
gating) → D-Python; §9 caddy re-baseline → Slice E (owner-gated); §10 slices → A–F; §11 alignment
(syntactic, no-build) → preserved; §12 tests → A1/B1/C1 (+ the two per-form config tests) + D tests; §13
decisions → encoded; §14 open → cache bump done (B5), gate response post-first-report.

**Dual-review fold:** BLOCKER B5 cache-test (B5 fixes the assertion+name+history); BLOCKER Slice-D wrong-seam
(restructured onto `&CpgContext` + `interface_method_names` + provider helper + new subcommand, split into
D-Rust/D-Python, TDD red-first); BLOCKER B7 line (→14, C7→15, call-site-line convention stated); MAJOR C4
false-recovery (name-field children only); MAJOR §4 package-level (deferred + recorded); MAJOR §12 (two
per-form gating tests added); MAJOR pkg.T (claim corrected + deferred); MAJOR weak tests (`r.len()` before
`.all()`, grammar-pin body); MAJOR D TDD (red-first steps); MINOR A5 qualifier reassignment (in snippet);
MINOR D5/F (concrete patch + ignored test); MINOR CLAUDE.md (acceptance doc step); NIT revertability
(recorded, optional flag deferred).

**Placeholder scan:** A/B/C/F carry complete reference code; D names exact signatures, JSON/record schemas,
join keys, subcommand, and TDD steps. No `TODO`/"similar to"/uncoded steps remain.

**Type/name consistency:** `RecoveredReceiver`, `ReceiverCtx{receiver_expr,qualifier,fn_node,call_line,parsed,
recv_var,file_imports}`, `ReceiverClassifier::classify(&self, ReceiverCtx)->Option<RecoveredReceiver>`,
`ReceiverRecoveryConfig{mode,type_assertion,var_local}`, `build_with_receiver_config`,
`recover_simple_ident(ctx,recover_var)`, `recover_type_assertion`, `interface_method_names`,
`interface_dispatch_manifest(&CpgContext)`, `ReceiverRecovery::{TypedParam,ConstructorLocal,TypeAssertion,
VarDecl,SliceElem}` — used identically across A→F.
