# Phase-IP Go Interface Dispatch — FOUNDATION (PR-1) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

> **rev 2 (2026-06-15) — plan-review folded (codex gpt-5.5 xhigh 5-node + claude operator-subagent, both "needs changes").** All fixes are buildability/correctness/completeness; the architecture verified sound. Key folds: Task 4 satisfaction is now **full-interface** (not per-method) and **augments** `data.satisfaction` (keeps `subtypes_of`/`resolve_dispatch` green) with explicit `set_value`/`set_ptr` (set_ptr⊇set_value) + **promoted/embedded** methods; grammar fixes (`&T{}` via `unary_expression`, `channel_type` field `value`, array length preserved); `CpgContext::build(&files, None)` (two-arg); `site_in(...,"Go")` (callee is the bare method name, verified vs `go_embedded_method_resolves_exact`); CPG edge-inspection via the **public `ctx.cpg.graph`** API in `tests/ast/cpg_test.rs` (NOT `#[cfg(test)]` accessors — invisible cross-crate); the 5th constructor init at `call_graph.rs:~1013`; telemetry counters wired (`fanout`/`fallback_fired`/`CrossPackageBareName`); harness test under `eval/tests` importing `tier_a.*`, call-stats test under the **`cli`** binary; real CHA test via a manual `TypeDatabase`; concrete cache test mirroring `cache_v6_round_trips_edge_confidence`. **Spec §14 reconciled:** PR-1 = per-edge `resolution_kind`/`dispatch_kind` + probe-JSON/pending persistence + replay test; the fingerprinted manifest + precision gate move wholly to PR-2. Records: `docs/prism-query-layer/phase-ip-plan-review-{codex,claude}-2026-06-15.md`.

**Goal:** Resolve Go interface-method calls on P6-lite-typed receivers (`func run(r Runner){ r.Go() }`) to their in-repo implementers, minted `Exact`/`InterfaceDispatch`, via signature-confirmed structural satisfaction + RTA liveness; plus the Tier-A harness apparatus to attribute interface precision.

**Architecture:** `CallGraph::build` consumes the existing (already-wired for embedding) `GoTypeProvider` via a new sibling `apply_go_interface_dispatch`, precomputing a CallGraph-owned `interface_impls` map. On an `owner_lookup` **miss** at the P6-lite seam (`resolution.rs:438`), `resolve_call_site` consults `interface_impls` and mints N `Exact` callees. Mirrors the merged embedding mechanism (`apply_go_embedding_promotion`/`clear_promoted_embedding`/`promoted_aliases`). Spec: `docs/superpowers/specs/2026-06-14-prism-phase-ip-type-confirmed-dispatch-design.md` (rev 5).

**Tech Stack:** Rust (tree-sitter, petgraph, serde/bincode), Python (Tier-A harness: `eval/tier_a/`), `cargo test` + `uv run tier-a`.

**Scope:** PR-1 only. The PR-2 receiver-expansion (type-assertion / `var` / interface-slice receivers) is explicitly deferred to the spec's PR-2 work-list. Owner-locked decisions are fixed: Exact (earned), keep the empty-live fallback Exact, uncapped fan-out.

**Commit/branch:** Work on branch `phase-ip-interface` (already created; rev-5 spec committed at `d64c957`). Commit after each task. Do NOT push or open a PR until the owner asks. End every commit message with:
`Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>`

---

## File Structure

**Rust (engine):**
- `src/resolution.rs` — add `ResolutionKind::InterfaceDispatch`; add `iface_key`/`admission_key`; the interface consult at the seam (`:438`). (owner of the R1–R7 ladder.)
- `src/type_providers/go.rs` — add `GoDispatchGap`/`GoDispatchOverApprox` enums, `canon_type`/`canon_sig`, signature-confirmed satisfaction with receiver-kind sets, generics-at-decl gating, and the public `compute_interface_dispatch`.
- `src/call_graph.rs` — `interface_impls`/`interface_gaps`/`interface_overapprox` fields (`#[serde(default)]`); `apply_go_interface_dispatch` + `clear_interface_dispatch`; wire into `build`, `build_incremental`, `build_scoped`, `remove_files`.
- `src/live_types.rs` — extend `scan_go_node` to the `{T,*T}` admission alphabet.
- `src/cpg_cache.rs` — bump `CACHE_VERSION` 8→9.
- `src/cpg/build.rs` — filter Step-9 CHA index + seed to `type_db`-owned (C/C++).
- `src/navigation/queries.rs` — `call-stats` telemetry.

**Rust (tests):**
- `tests/integration/resolution_test.rs` — unit/resolution tests (helpers `build`, `site_in`, `cg.resolve_call_site`).
- `eval/fixtures/go/interface_dispatch/expected.toml` — flip `known_fail → pass`.
- New fixtures under `tests/fixtures/` or inline `build(&[...])` for barrier-precision + CHA mixed-repo.

**Python (harness):**
- `eval/tier_a/model.py` — `CallEdge.resolution_kind`.
- `eval/tier_a/sut.py` — extract `Resolution.kind`.
- `eval/tier_a/adjudication.py` — `Adjudication.dispatch_kind`.
- `eval/tier_a/cli.py` — interface-site manifest + precision-gate site filter.

---

## Task 1: `ResolutionKind::InterfaceDispatch`

**Files:**
- Modify: `src/resolution.rs:17-34` (enum), `src/resolution.rs:36-56` (`as_str`)
- Test: `tests/integration/resolution_test.rs`

- [ ] **Step 1: Write the failing test**

```rust
// tests/integration/resolution_test.rs
#[test]
fn interface_dispatch_kind_as_str() {
    assert_eq!(
        prism::resolution::ResolutionKind::InterfaceDispatch.as_str(),
        "interface_dispatch"
    );
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test integration resolution_test::interface_dispatch_kind_as_str`
Expected: FAIL — `no variant named InterfaceDispatch` (compile error).

- [ ] **Step 3: Implement**

Add the variant after `EmbeddedPromotion` in the enum (resolution.rs:33):
```rust
    EmbeddedPromotion,
    InterfaceDispatch,
```
Add the arm after the `EmbeddedPromotion` arm in `as_str` (resolution.rs:54):
```rust
            ResolutionKind::EmbeddedPromotion => "embedded_promotion",
            ResolutionKind::InterfaceDispatch => "interface_dispatch",
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --test integration resolution_test::interface_dispatch_kind_as_str`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/resolution.rs tests/integration/resolution_test.rs
git commit -m "feat(go-iface): add ResolutionKind::InterfaceDispatch

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

## Task 2: Go key contracts — `iface_key` + `admission_key`

Separate key contracts (spec §10 BLOCKER-2): `owner_key` (existing) stays for bare owners; `iface_key` produces the interface lookup key (strip `*`/`&`/`pkg.`, **gap on generics `[…]`**); `admission_key` is pointer-preserving (`T` vs `*T`).

**Files:**
- Modify: `src/resolution.rs` (add two `pub fn` after `owner_key`, ~:86)
- Test: `tests/integration/resolution_test.rs`

- [ ] **Step 1: Write the failing tests**

```rust
// tests/integration/resolution_test.rs
use prism::resolution::{admission_key, iface_key};

#[test]
fn iface_key_strips_pkg_and_pointer() {
    assert_eq!(iface_key("Runner").as_deref(), Some("Runner"));
    assert_eq!(iface_key("io.Reader").as_deref(), Some("Reader"));
    assert_eq!(iface_key("*Runner").as_deref(), Some("Runner"));
}

#[test]
fn iface_key_gaps_on_generic_instantiation() {
    assert_eq!(iface_key("Container[T]"), None);
    assert_eq!(iface_key("pkg.Map[string,int]"), None);
}

#[test]
fn admission_key_distinguishes_pointer() {
    assert_eq!(admission_key("Fast", false), "Fast");
    assert_eq!(admission_key("Fast", true), "*Fast");
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test --test integration resolution_test::iface_key`
Expected: FAIL — `cannot find function iface_key`.

- [ ] **Step 3: Implement**

Add to `src/resolution.rs` after `owner_key` (after line 86):
```rust
/// Interface lookup key (Go): strip `&`/`*` and a `pkg.` qualifier to the bare
/// interface name. Returns `None` for a generic instantiation (`Foo[T]`), which
/// is non-dispatchable (a recorded gap, never a key) — spec §6/§10.
pub fn iface_key(text: &str) -> Option<String> {
    let t = text.trim().trim_start_matches('&').trim_start_matches('*').trim();
    if t.contains('[') {
        return None; // generic instantiation -> gap, not a key
    }
    let bare = t.rsplit('.').next().unwrap_or(t).trim();
    if bare.is_empty() {
        None
    } else {
        Some(bare.to_string())
    }
}

/// Admission key (Go method-set asymmetry): a value-receiver satisfier admits as
/// `T`; a pointer-receiver-only satisfier admits as `*T` (spec §7). Bare `T` must
/// already be normalized (no `pkg.`).
pub fn admission_key(bare_type: &str, is_pointer: bool) -> String {
    if is_pointer {
        format!("*{bare_type}")
    } else {
        bare_type.to_string()
    }
}
```

- [ ] **Step 4: Run to verify it passes**

Run: `cargo test --test integration resolution_test::iface_key && cargo test --test integration resolution_test::admission_key`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/resolution.rs tests/integration/resolution_test.rs
git commit -m "feat(go-iface): iface_key + admission_key contracts (spec §10 BLOCKER-2)

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

## Task 3: `canon_type` / `canon_sig` + gap enums

Canonical, name-free, type-only signature strings so an interface method and a concrete method compare byte-for-byte (spec §6). An unknown node **fails closed** to a gap; generics/anonymous-interface gap. Defined inside go.rs (operates on tree-sitter nodes via `ParsedFile`).

**Files:**
- Modify: `src/type_providers/go.rs` (new enums + `canon_type`/`canon_sig` static fns)
- Test: `src/type_providers/go.rs` `#[cfg(test)] mod canon_tests`

- [ ] **Step 1: Write the failing tests**

Add at the bottom of `src/type_providers/go.rs`:
```rust
#[cfg(test)]
mod canon_tests {
    use super::*;
    use crate::ast::ParsedFile;
    use crate::languages::Language;

    // Parse a Go file and return the canon_sig of the first method named `m` on any
    // receiver, or the first interface method spec named `m`. Helper for byte-equality.
    fn sig_of(src: &str, method: &str) -> Result<String, GoDispatchGap> {
        let p = ParsedFile::parse("t.go", src, Language::Go).unwrap();
        GoTypeProvider::first_canon_sig_for_test(&p, method)
    }

    #[test]
    fn param_names_dropped_iface_eq_concrete() {
        let iface = "package p\ntype I interface { Do(a int, b string) error }\n";
        let conc = "package p\ntype T struct{}\nfunc (t T) Do(x int, y string) error { return nil }\n";
        assert_eq!(sig_of(iface, "Do").unwrap(), sig_of(conc, "Do").unwrap());
    }

    #[test]
    fn grouped_params_expand() {
        let grouped = "package p\ntype T struct{}\nfunc (t T) F(a, b int) {}\n";
        let expanded = "package p\ntype I interface { F(int, int) }\n";
        assert_eq!(sig_of(grouped, "F").unwrap(), sig_of(expanded, "F").unwrap());
    }

    #[test]
    fn channel_direction_distinguishes() {
        let send = "package p\ntype T struct{}\nfunc (t T) C(c chan<- int) {}\n";
        let bidi = "package p\ntype I interface { C(c chan int) }\n";
        assert_ne!(sig_of(send, "C").unwrap(), sig_of(bidi, "C").unwrap());
    }

    #[test]
    fn any_equals_empty_interface() {
        let a = "package p\ntype T struct{}\nfunc (t T) F(x any) {}\n";
        let b = "package p\ntype I interface { F(x interface{}) }\n";
        assert_eq!(sig_of(a, "F").unwrap(), sig_of(b, "F").unwrap());
    }

    #[test]
    fn variadic_canonicalizes() {
        let a = "package p\ntype T struct{}\nfunc (t T) F(xs ...int) {}\n";
        let b = "package p\ntype I interface { F(...int) }\n";
        assert_eq!(sig_of(a, "F").unwrap(), sig_of(b, "F").unwrap());
    }

    #[test]
    fn return_mismatch_differs() {
        let a = "package p\ntype T struct{}\nfunc (t T) F() error { return nil }\n";
        let b = "package p\ntype I interface { F() string }\n";
        assert_ne!(sig_of(a, "F").unwrap(), sig_of(b, "F").unwrap());
    }
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test --lib type_providers::go::canon_tests`
Expected: FAIL — `GoDispatchGap` / `first_canon_sig_for_test` not found.

- [ ] **Step 3: Implement the enums + canon functions**

Near the top of `src/type_providers/go.rs` (after imports), add:
```rust
/// Non-dispatchable, fail-closed — mints NO edge (spec §15).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GoDispatchGap {
    Generic,            // decl carries a type_parameter_list / interface type-set
    AnonymousInterface, // non-empty anonymous interface in a signature
    UnknownCanonType,   // unenumerated type node — fail closed
}

/// Admitted over-approximation — the Exact edge IS minted; a precision counter (spec §15).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GoDispatchOverApprox {
    CrossPackageBareName,         // io.Reader ≡ bufio.Reader under bare-name canon
    NonLocalConstructionFallback, // empty-live fallback fired: full satisfier set
}
```

Add the canon functions as static methods in `impl GoTypeProvider` (next to `extract_method_signature`, go.rs:365):
```rust
/// Canonical type string, recursive. Fails closed on unknown nodes (spec §6).
fn canon_type(node: &tree_sitter::Node, parsed: &ParsedFile) -> Result<String, GoDispatchGap> {
    match node.kind() {
        "type_identifier" | "qualified_type" => {
            // bare name; pkg.T -> T
            let txt = parsed.node_text(node);
            let bare = txt.trim().rsplit('.').next().unwrap_or(txt.trim()).trim();
            Ok(bare.to_string())
        }
        "pointer_type" => {
            let inner = node.named_child(0).ok_or(GoDispatchGap::UnknownCanonType)?;
            Ok(format!("*{}", Self::canon_type(&inner, parsed)?))
        }
        "slice_type" => {
            let inner = node.child_by_field_name("element").ok_or(GoDispatchGap::UnknownCanonType)?;
            Ok(format!("[]{}", Self::canon_type(&inner, parsed)?))
        }
        "array_type" => {
            let inner = node.child_by_field_name("element").ok_or(GoDispatchGap::UnknownCanonType)?;
            // Array length is part of Go type identity ([3]int != [4]int) — preserve the
            // literal length text (round-4 codex MINOR). Non-literal/const length kept as text.
            let len = node.child_by_field_name("length")
                .map(|n| parsed.node_text(&n).trim().to_string())
                .unwrap_or_default();
            Ok(format!("[{len}]{}", Self::canon_type(&inner, parsed)?))
        }
        "map_type" => {
            let k = node.child_by_field_name("key").ok_or(GoDispatchGap::UnknownCanonType)?;
            let v = node.child_by_field_name("value").ok_or(GoDispatchGap::UnknownCanonType)?;
            Ok(format!("map[{}]{}", Self::canon_type(&k, parsed)?, Self::canon_type(&v, parsed)?))
        }
        "channel_type" => {
            // direction-preserving: `chan<-` / `<-chan` / `chan`
            let txt = parsed.node_text(node);
            let dir = if txt.contains("<-chan") {
                "<-chan"
            } else if txt.contains("chan<-") {
                "chan<-"
            } else {
                "chan"
            };
            // element type is field `value` in tree-sitter-go 0.23.4 (round-4 claude MAJOR)
            let inner = node.child_by_field_name("value").ok_or(GoDispatchGap::UnknownCanonType)?;
            Ok(format!("{dir} {}", Self::canon_type(&inner, parsed)?))
        }
        "function_type" => {
            let params = node.child_by_field_name("parameters");
            let result = node.child_by_field_name("result");
            Ok(format!("func{}", Self::canon_sig(params.as_ref(), result.as_ref(), parsed)?))
        }
        "interface_type" => {
            // empty interface{} / any -> "any"; non-empty anonymous -> gap
            let txt = parsed.node_text(node).replace(char::is_whitespace, "");
            if txt == "interface{}" || txt == "any" {
                Ok("any".to_string())
            } else {
                Err(GoDispatchGap::AnonymousInterface)
            }
        }
        "generic_type" => Err(GoDispatchGap::Generic),
        "variadic_parameter_declaration" => {
            // handled in canon_sig; reaching here is a structural surprise
            Err(GoDispatchGap::UnknownCanonType)
        }
        _ => Err(GoDispatchGap::UnknownCanonType),
    }
}

/// Canonical `(params)(results)`; names dropped, grouped params expanded, variadic
/// as `...T`. Either side gapping fails the whole sig (spec §6).
fn canon_sig(
    params: Option<&tree_sitter::Node>,
    result: Option<&tree_sitter::Node>,
    parsed: &ParsedFile,
) -> Result<String, GoDispatchGap> {
    let ps = Self::canon_param_list(params, parsed)?;
    // result may be a single type node OR a parameter_list (multi/parenthesized).
    let rs = match result {
        None => Vec::new(),
        Some(r) if r.kind() == "parameter_list" => Self::canon_param_list(Some(r), parsed)?,
        Some(r) => vec![Self::canon_type(r, parsed)?],
    };
    Ok(format!("({})({})", ps.join(","), rs.join(",")))
}

/// Expand a parameter_list to canonical type strings: drop names, expand grouped
/// `(a, b int)` -> [int,int], variadic `...T` -> `...T`.
fn canon_param_list(
    list: Option<&tree_sitter::Node>,
    parsed: &ParsedFile,
) -> Result<Vec<String>, GoDispatchGap> {
    let mut out = Vec::new();
    let list = match list {
        Some(l) => l,
        None => return Ok(out),
    };
    let mut cursor = list.walk();
    for decl in list.named_children(&mut cursor) {
        match decl.kind() {
            "parameter_declaration" => {
                let ty = decl.child_by_field_name("type").ok_or(GoDispatchGap::UnknownCanonType)?;
                let canon = Self::canon_type(&ty, parsed)?;
                // grouped `(a, b int)`: count name children (>=1) -> repeat the type.
                let names = decl.children(&mut decl.walk())
                    .filter(|c| c.kind() == "identifier").count().max(1);
                for _ in 0..names {
                    out.push(canon.clone());
                }
            }
            "variadic_parameter_declaration" => {
                let ty = decl.child_by_field_name("type").ok_or(GoDispatchGap::UnknownCanonType)?;
                out.push(format!("...{}", Self::canon_type(&ty, parsed)?));
            }
            _ => {}
        }
    }
    Ok(out)
}

#[cfg(test)]
pub fn first_canon_sig_for_test(parsed: &ParsedFile, method: &str) -> Result<String, GoDispatchGap> {
    // Walk for a method_declaration or interface method_spec named `method`.
    fn find<'a>(n: tree_sitter::Node<'a>, parsed: &ParsedFile, method: &str) -> Option<tree_sitter::Node<'a>> {
        let is_match = matches!(n.kind(), "method_declaration" | "method_spec" | "method_elem")
            && n.child_by_field_name("name").map(|x| parsed.node_text(&x).trim() == method).unwrap_or(false);
        if is_match { return Some(n); }
        let mut c = n.walk();
        for ch in n.children(&mut c) {
            if let Some(f) = find(ch, parsed, method) { return Some(f); }
        }
        None
    }
    let node = find(parsed.tree.root_node(), parsed, method).expect("method not found");
    GoTypeProvider::canon_sig(
        node.child_by_field_name("parameters").as_ref(),
        node.child_by_field_name("result").as_ref(),
        parsed,
    )
}
```

> Implementer note: tree-sitter-go field/kind names (`element`, `key`, `value`, `parameters`, `result`, `method_spec` vs `method_elem`) vary by grammar version — verify against `build.rs`'s pinned grammar and the existing `extract_method_signature` (go.rs:365-387), which already enumerates the type-node kinds. Adjust kind/field strings to match; the tests above are the oracle.

- [ ] **Step 4: Run to verify it passes**

Run: `cargo test --lib type_providers::go::canon_tests`
Expected: PASS (all 6).

- [ ] **Step 5: Commit**

```bash
git add src/type_providers/go.rs
git commit -m "feat(go-iface): canon_type/canon_sig + GoDispatchGap/OverApprox enums (spec §6/§15)

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

## Task 4: Signature-confirmed satisfaction + receiver-kind sets + generics-at-decl gating

Upgrade satisfaction from name-only (go.rs:464) to **full-interface canonical-signature equality**,
tracking the **admission key** (`T`/`*T`) per satisfier, and gate any interface/method whose
**declaration** carries a `type_parameter_list` (spec §6 — a context-free `canon_type` cannot detect a
bare type-parameter use). Three corrections from the round-2 plan-review are load-bearing:

- **Full-interface, not per-method (codex BLOCKER#1):** `T` satisfies `I` iff `T`'s method set has EVERY
  method of `I` with equal `canon_sig`. Per-`(iface,method)` entries are populated **only from the
  full-interface satisfiers** — never admit a type that has just one method of a multi-method interface.
- **Augment, don't replace `data.satisfaction` (claude BLOCKER / codex BLOCKER#2):** `subtypes_of`
  (go.rs:700) and `DispatchProvider::resolve_dispatch` (go.rs:726) read the existing bare-name
  `data.satisfaction`. Keep populating it — now **derived from the signature-confirmed full satisfiers**
  (so those trait consumers improve to signature-confirmed and drop name-only false positives) — in
  addition to the new admission-keyed `sat_keys`. Do NOT empty or admission-key `data.satisfaction`.
- **`set_ptr(T) ⊇ set_value(T)` (codex MAJOR#10):** `*T`'s method set includes value-receiver methods;
  `T`'s does not include pointer-receiver methods. So a **value-receiver** method admits as **both `T`
  and `*T`**; a **pointer-receiver-only** method admits as **`*T` only**. **Promoted (embedded) methods**
  (codex MAJOR#11) join the set by their embedded receiver kind (reuse `promoted_struct_methods`).

**Files:**
- Modify: `src/type_providers/go.rs` (`GoInterface`/`GoMethod` add a `generic` flag at extraction; rewrite `compute_satisfaction`; add `sat_keys`/`dispatch_gaps` to `GoTypeData`; `#[cfg(test)]` accessors)
- Test: `src/type_providers/go.rs` `#[cfg(test)] mod satisfaction_tests`; regression `tests/ast/type_provider_test.rs` (unchanged, must stay green)

- [ ] **Step 1: Write the failing tests**

```rust
#[cfg(test)]
mod satisfaction_tests {
    use super::*;
    use crate::ast::ParsedFile;
    use crate::languages::Language;
    use std::collections::BTreeMap;

    fn provider(src: &str) -> GoTypeProvider {
        let mut files = BTreeMap::new();
        files.insert("t.go".to_string(), ParsedFile::parse("t.go", src, Language::Go).unwrap());
        GoTypeProvider::from_parsed_files(&files)
    }

    #[test]
    fn signature_mismatch_does_not_satisfy() {
        let p = provider(
            "package p\n\
             type I interface { Do() error }\n\
             type T struct{}\nfunc (t T) Do() string { return \"\" }\n",
        );
        assert!(p.satisfier_admission_keys_for_test("I", "Do").is_empty());
    }

    #[test]
    fn multi_method_partial_satisfier_not_admitted() {
        // codex#1: T has only Do(), interface needs Do()+Stop() -> NOT a satisfier.
        let p = provider(
            "package p\n\
             type I interface { Do(); Stop() }\n\
             type T struct{}\nfunc (t T) Do() {}\n",
        );
        assert!(p.satisfier_admission_keys_for_test("I", "Do").is_empty());
        assert!(p.satisfier_admission_keys_for_test("I", "Stop").is_empty());
    }

    #[test]
    fn value_receiver_admits_as_both_t_and_ptr_t() {
        // set_ptr ⊇ set_value: a value-receiver method satisfies via BOTH T and *T.
        let p = provider(
            "package p\n\
             type I interface { Do() }\n\
             type T struct{}\nfunc (t T) Do() {}\n",
        );
        assert_eq!(p.satisfier_admission_keys_for_test("I", "Do"),
                   vec!["*T".to_string(), "T".to_string()]); // sorted: '*' < 'T'
    }

    #[test]
    fn pointer_receiver_only_admits_as_pointer() {
        let p = provider(
            "package p\n\
             type I interface { Do() }\n\
             type T struct{}\nfunc (t *T) Do() {}\n",
        );
        assert_eq!(p.satisfier_admission_keys_for_test("I", "Do"), vec!["*T".to_string()]);
    }

    #[test]
    fn mixed_value_and_pointer_method_admits_as_pointer_only() {
        // I needs A()+B(); T has value A + pointer B -> only *T's set has both.
        let p = provider(
            "package p\n\
             type I interface { A(); B() }\n\
             type T struct{}\nfunc (t T) A() {}\nfunc (t *T) B() {}\n",
        );
        assert_eq!(p.satisfier_admission_keys_for_test("I", "A"), vec!["*T".to_string()]);
        assert_eq!(p.satisfier_admission_keys_for_test("I", "B"), vec!["*T".to_string()]);
    }

    #[test]
    fn embedded_method_satisfies() {
        // codex#11: Wrap embeds Base (Base has Do); Wrap satisfies I via the promoted Do.
        let p = provider(
            "package p\n\
             type I interface { Do() }\n\
             type Base struct{}\nfunc (b Base) Do() {}\n\
             type Wrap struct { Base }\n",
        );
        let sats = p.satisfier_admission_keys_for_test("I", "Do");
        // Wrap admits (value-embedded value method) as both; Base also satisfies.
        assert!(sats.contains(&"Wrap".to_string()) || sats.contains(&"*Wrap".to_string()),
                "promoted method must make Wrap satisfy I: {sats:?}");
    }

    #[test]
    fn generic_interface_is_non_dispatchable() {
        let p = provider(
            "package p\n\
             type I[X any] interface { Do(X) }\n\
             type T struct{}\nfunc (t T) Do(int) {}\n",
        );
        assert!(p.dispatch_gaps_for_test().contains(&GoDispatchGap::Generic));
        assert!(p.satisfier_admission_keys_for_test("I", "Do").is_empty());
    }

    #[test]
    fn anonymous_interface_method_is_gap() {
        // §13.3: a method whose param is a non-empty anonymous interface is non-dispatchable.
        let p = provider(
            "package p\n\
             type I interface { Do(x interface{ Run() }) }\n\
             type T struct{}\nfunc (t T) Do(x interface{ Run() }) {}\n",
        );
        assert!(p.dispatch_gaps_for_test().contains(&GoDispatchGap::AnonymousInterface));
        assert!(p.satisfier_admission_keys_for_test("I", "Do").is_empty());
    }
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test --lib type_providers::go::satisfaction_tests`
Expected: FAIL — accessors not found / model not yet full-interface + receiver-kind.

- [ ] **Step 3: Implement**

1. **Generic gate at the declaration.** Add `generic: bool` to `GoInterface` and `GoMethod`, set `true`
   when the enclosing `type_spec`/`method_declaration` has a `type_parameter_list` child (verify the
   field name against the grammar — `type_spec.type_parameters`). A generic interface pushes
   `GoDispatchGap::Generic` and is skipped; a generic concrete method is excluded from satisfaction.
2. **Canonical method sets.** `extract_method_signature`/`extract_func_signature` (go.rs:365/445) become
   thin wrappers delegating to `canon_sig`, storing `Result<String, GoDispatchGap>`; `GoMethod.signature`
   becomes that `Result`, `GoInterface.methods` becomes `BTreeMap<String, Result<String, GoDispatchGap>>`.
   (Embedding's `promoted_struct_methods` uses `m.name`, not `.signature`, so it is unaffected.)
3. **Per-type method sets with admission keys** (helper, conceptually):
```rust
// set_value(T) = value-receiver methods of T (+ value-embedded promoted methods)
// set_ptr(T)   = set_value(T) ∪ pointer-receiver methods (+ pointer-embedded promoted)
// Each entry: method_name -> (canon_sig, FunctionId). A gapped canon_sig excludes that method.
```
   Promoted methods come from `self.promoted_struct_methods()` keyed by the owner struct; map each to a
   `GoMethod` (its `func_id`) and place it in `set_value`/`set_ptr` by the embedded receiver kind.
4. **Full-interface satisfaction.** Rewrite `compute_satisfaction`:
```rust
fn compute_satisfaction(data: &mut GoTypeData) {
    // dispatch_gaps: push Generic for generic interfaces, AnonymousInterface/UnknownCanonType
    // for any gapped interface-method canon (skip those interfaces/methods).
    // For each NON-generic interface I with a fully-canonical method set M_I:
    //   for each concrete type T:
    //     if set_value(T) has every (m, sig) in M_I  -> T satisfies via "T" AND "*T"
    //     else if set_ptr(T) has every (m, sig)      -> T satisfies via "*T" only
    //   For each satisfying (T, admission_key):
    //     data.satisfaction[I].insert(T);                      // bare name, for trait consumers
    //     for m in M_I: data.sat_keys[(I, m)].push((admission_key, fid_of_T_method_m));
    //   Emit GoDispatchOverApprox::CrossPackageBareName at the point a qualified type was
    //   bare-named during canon (recorded in Task 13's telemetry; here just the gap/overapprox vecs).
}
```
   Store `sat_keys: BTreeMap<(String,String), Vec<(String, FunctionId)>>`,
   `dispatch_gaps: Vec<GoDispatchGap>`, and `dispatch_overapprox: Vec<GoDispatchOverApprox>` on
   `GoTypeData` (push `CrossPackageBareName` when canon bare-names a `qualified_type` — codex MAJOR#12,
   so the over-approx is emitted, not just declared). Build admission keys with
   `crate::resolution::admission_key(bare_T, is_pointer)`.
5. **Accessors:**
```rust
#[cfg(test)]
pub fn satisfier_admission_keys_for_test(&self, iface: &str, method: &str) -> Vec<String> {
    let mut v: Vec<String> = self.data.sat_keys
        .get(&(iface.to_string(), method.to_string()))
        .map(|s| s.iter().map(|(k, _)| k.clone()).collect())
        .unwrap_or_default();
    v.sort();
    v.dedup();
    v
}
#[cfg(test)]
pub fn dispatch_gaps_for_test(&self) -> Vec<GoDispatchGap> { self.data.dispatch_gaps.clone() }
```

- [ ] **Step 4: Run to verify it passes (incl. the existing-test regression)**

Run: `cargo test --lib type_providers::go::satisfaction_tests`
Expected: PASS (all 8).
Run: `cargo test --test ast type_provider_test::`
Expected: PASS — **`test_go_interface_satisfaction` must stay green** (proves `data.satisfaction` is still
populated, now signature-confirmed). If it fails, you replaced `data.satisfaction` instead of augmenting.

- [ ] **Step 5: Commit**

```bash
git add src/type_providers/go.rs
git commit -m "feat(go-iface): full-interface signature-confirmed satisfaction + set_ptr⊇set_value + promoted + generics gate (spec §6/§7)

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

## Task 5: Comprehensive Go liveness — admission-key alphabet

Extend `scan_go_node` (live_types.rs:158) to emit `{T, *T}` for `&T{}`/`new(T)` and `T` for `var x T` (spec §8). The traversal is already whole-tree (`scan_tree_recursive`, live_types.rs:275) — change only the node handler.

**Files:**
- Modify: `src/live_types.rs:158-173`
- Test: `src/live_types.rs` `#[cfg(test)] mod go_liveness_tests` (or extend an existing test module)

- [ ] **Step 1: Write the failing test**

```rust
#[cfg(test)]
mod go_liveness_tests {
    use super::*;
    use crate::ast::ParsedFile;
    use crate::languages::Language;
    use std::collections::BTreeSet;

    fn live(src: &str) -> BTreeSet<String> {
        let p = ParsedFile::parse("t.go", src, Language::Go).unwrap();
        let mut s = BTreeSet::new();
        scan_go(&p, &mut s);
        s
    }

    #[test]
    fn admission_alphabet() {
        let s = live(
            "package p\nfunc f() {\n\
             _ = T{}\n\
             _ = &U{}\n\
             _ = new(V)\n\
             var w W\n_ = w\n}\n",
        );
        assert!(s.contains("T"));         // value literal
        assert!(s.contains("U") && s.contains("*U")); // addressable -> both
        assert!(s.contains("V") && s.contains("*V")); // new(V) -> both
        assert!(s.contains("W"));         // var decl, concrete
    }
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test --lib live_types::go_liveness_tests`
Expected: FAIL — `*U`/`V`/`W` absent (only composite-literal value handled today).

- [ ] **Step 3: Implement**

Replace `scan_go_node` (live_types.rs:158-173):
```rust
fn scan_go_node(node: &tree_sitter::Node, parsed: &ParsedFile, live: &mut BTreeSet<String>) {
    match node.kind() {
        "composite_literal" => {
            // value literal T{} -> T. NOTE: in `&T{}` the `&` is on the PARENT
            // unary_expression (round-2 plan-review BLOCKER), handled in the next arm.
            if let Some(type_node) = node.child_by_field_name("type") {
                let base = parsed.node_text(&type_node).trim()
                    .trim_start_matches('*').split('.').last().unwrap_or("").to_string();
                if !base.is_empty() && base.starts_with(|c: char| c.is_ascii_uppercase()) {
                    live.insert(base);
                }
            }
        }
        "unary_expression" => {
            // &T{} -> addressable -> T AND *T. The `&` operator lives here, not in the literal.
            let is_addr = node.child_by_field_name("operator")
                .map(|op| parsed.node_text(&op).trim() == "&")
                .unwrap_or_else(|| parsed.node_text(node).trim_start().starts_with('&'));
            if is_addr {
                if let Some(operand) = node.child_by_field_name("operand") {
                    if operand.kind() == "composite_literal" {
                        if let Some(type_node) = operand.child_by_field_name("type") {
                            let base = parsed.node_text(&type_node).trim()
                                .split('.').last().unwrap_or("").to_string();
                            if !base.is_empty() && base.starts_with(|c: char| c.is_ascii_uppercase()) {
                                live.insert(base.clone());
                                live.insert(format!("*{base}"));
                            }
                        }
                    }
                }
            }
        }
        "call_expression" => {
            // new(T) builtin -> T and *T
            if let Some(func) = node.child_by_field_name("function") {
                if parsed.node_text(&func).trim() == "new" {
                    if let Some(args) = node.child_by_field_name("arguments") {
                        if let Some(arg) = args.named_child(0) {
                            let base = parsed.node_text(&arg).trim()
                                .split('.').last().unwrap_or("").to_string();
                            if !base.is_empty() && base.starts_with(|c: char| c.is_ascii_uppercase()) {
                                live.insert(base.clone());
                                live.insert(format!("*{base}"));
                            }
                        }
                    }
                }
            }
        }
        "var_declaration" => {
            // var x T -> T (concrete only)
            let mut cur = node.walk();
            for spec in node.named_children(&mut cur) {
                if spec.kind() == "var_spec" {
                    if let Some(ty) = spec.child_by_field_name("type") {
                        let base = parsed.node_text(&ty).trim()
                            .trim_start_matches('*')
                            .split('.').last().unwrap_or("").to_string();
                        if !base.is_empty() && base.starts_with(|c: char| c.is_ascii_uppercase()) {
                            live.insert(base);
                        }
                    }
                }
            }
        }
        _ => {}
    }
}
```

> Known limitations (RTA-safe — the kept-Exact fallback covers, spec §8): (1) the uppercase-first filter
> misses **unexported** in-repo types (lowercase) — they never go live, so their interface edges always
> take the wide fallback (still Exact, just not RTA-pruned); preferring a known-concrete-types set over
> casing is a PR-2 refinement (round-2 codex). (2) grouped `var ( x T; y U )` wrapped in `var_spec_list`
> is not recursed here; single `var x T` (the §8 case) works. Both only widen the fallback, never drop a
> satisfier.

- [ ] **Step 4: Run to verify it passes**

Run: `cargo test --lib live_types::go_liveness_tests`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/live_types.rs
git commit -m "feat(go-iface): scan_go admission-key alphabet for RTA liveness (spec §8)

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

## Task 6: `compute_interface_dispatch` (the public engine entry)

The sole public dispatch entry (spec §10 BLOCKER-3): given a live admission-key set, produce `interface_impls` + gaps + over-approx + fanout/fallback telemetry. RTA-intersect satisfier admission keys against `live`; if **no** admission key for `I` is live, fire the receiver-kind-aware fallback (full satisfier set, Exact) and record `NonLocalConstructionFallback`.

**Files:**
- Modify: `src/type_providers/go.rs` (`InterfaceDispatchTable` struct + `pub fn compute_interface_dispatch`)
- Test: `src/type_providers/go.rs` `#[cfg(test)] mod dispatch_tests`

- [ ] **Step 1: Write the failing tests**

```rust
#[cfg(test)]
mod dispatch_tests {
    use super::*;
    use crate::ast::ParsedFile;
    use crate::languages::Language;
    use std::collections::{BTreeMap, BTreeSet};

    fn provider(src: &str) -> GoTypeProvider {
        let mut files = BTreeMap::new();
        files.insert("t.go".to_string(), ParsedFile::parse("t.go", src, Language::Go).unwrap());
        GoTypeProvider::from_parsed_files(&files)
    }
    const SRC: &str = "package p\n\
        type I interface { Do() }\n\
        type Fast struct{}\nfunc (f Fast) Do() {}\n\
        type Slow struct{}\nfunc (s Slow) Do() {}\n";

    #[test]
    fn rta_prunes_to_live() {
        let p = provider(SRC);
        let live: BTreeSet<String> = ["Fast".to_string()].into_iter().collect();
        let t = p.compute_interface_dispatch(&live);
        let ids = t.impls.get(&("I".to_string(), "Do".to_string())).unwrap();
        assert_eq!(ids.len(), 1, "only live Fast");
        assert_eq!(ids[0].name, "Do");
        assert_eq!(ids[0].file, "t.go");
        assert!(!t.fallback_fired[&("I".to_string(), "Do".to_string())]);
    }

    #[test]
    fn empty_live_fires_fallback_full_set() {
        let p = provider(SRC);
        let live: BTreeSet<String> = BTreeSet::new();
        let t = p.compute_interface_dispatch(&live);
        let ids = t.impls.get(&("I".to_string(), "Do".to_string())).unwrap();
        assert_eq!(ids.len(), 2, "fallback -> all satisfiers");
        assert!(t.fallback_fired[&("I".to_string(), "Do".to_string())]);
        assert!(t.overapprox.contains(&GoDispatchOverApprox::NonLocalConstructionFallback));
    }
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test --lib type_providers::go::dispatch_tests`
Expected: FAIL — `compute_interface_dispatch` / `InterfaceDispatchTable` not found.

- [ ] **Step 3: Implement**

Add the struct (top of go.rs, near the gap enums):
```rust
pub struct InterfaceDispatchTable {
    pub impls: BTreeMap<(String, String), Vec<FunctionId>>,
    pub gaps: Vec<GoDispatchGap>,
    pub overapprox: Vec<GoDispatchOverApprox>,
    pub fanout: BTreeMap<(String, String), usize>,
    pub fallback_fired: BTreeMap<(String, String), bool>,
}
```
Add the public method in `impl GoTypeProvider`:
```rust
/// Compute named in-repo interface dispatch, RTA-pruned to `live` (admission keys),
/// receiver-kind-aware, with the empty-live fallback kept Exact (spec §5/§7/§8).
pub fn compute_interface_dispatch(&self, live: &BTreeSet<String>) -> InterfaceDispatchTable {
    let mut t = InterfaceDispatchTable {
        impls: BTreeMap::new(),
        gaps: self.data.dispatch_gaps.clone(),
        overapprox: self.data.dispatch_overapprox.clone(), // CrossPackageBareName seeded in Task 4
        fanout: BTreeMap::new(),
        fallback_fired: BTreeMap::new(),
    };
    // sat_keys: (iface, method) -> Vec<(admission_key, FunctionId)>  (Task 4)
    for ((iface, method), sats) in &self.data.sat_keys {
        if sats.is_empty() {
            continue;
        }
        let live_hits: Vec<&FunctionId> = sats.iter()
            .filter(|(k, _)| live.contains(k))
            .map(|(_, fid)| fid)
            .collect();
        let (chosen, fired): (Vec<FunctionId>, bool) = if live_hits.is_empty() {
            // receiver-kind-aware-empty fallback: full satisfier set, kept Exact.
            (sats.iter().map(|(_, fid)| fid.clone()).collect(), true)
        } else {
            (live_hits.into_iter().cloned().collect(), false)
        };
        // de-dup FunctionIds (a value+pointer satisfier may appear twice).
        let mut uniq: Vec<FunctionId> = Vec::new();
        for fid in chosen {
            if !uniq.contains(&fid) { uniq.push(fid); }
        }
        let key = (iface.clone(), method.clone());
        t.fanout.insert(key.clone(), uniq.len());
        t.fallback_fired.insert(key.clone(), fired);
        if fired {
            t.overapprox.push(GoDispatchOverApprox::NonLocalConstructionFallback);
        }
        if !uniq.is_empty() {
            t.impls.insert(key, uniq);
        }
    }
    t
}
```

- [ ] **Step 4: Run to verify it passes**

Run: `cargo test --lib type_providers::go::dispatch_tests`
Expected: PASS (both).

- [ ] **Step 5: Commit**

```bash
git add src/type_providers/go.rs
git commit -m "feat(go-iface): compute_interface_dispatch — RTA + kept-Exact fallback + telemetry (spec §5/§8/§10)

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

## Task 7: CallGraph wiring — fields, apply/clear, build paths, cache bump

Add CallGraph-owned maps + the sibling apply/clear (mirroring embedding), wire into all three build paths + `remove_files`, and bump `CACHE_VERSION` 8→9.

**Files:**
- Modify: `src/call_graph.rs:71-78` (fields), `:93-94`/`:213-214`/`:714-717` + `build_direct_subset` (constructors + build apply), `:768` (remove_files), `:775-791` (merge unaffected); `apply_go_interface_dispatch`/`clear_interface_dispatch` near `:798-865`.
- Modify: `src/cpg/build.rs` (`build_incremental` post-merge apply; `build_scoped` path is `CallGraph::build` already → covered).
- Modify: `src/cpg_cache.rs:49` (CACHE_VERSION).
- Test: `tests/integration/resolution_test.rs`

- [ ] **Step 1: Write the failing test**

```rust
// tests/integration/resolution_test.rs
#[test]
fn callgraph_exposes_interface_impls() {
    use prism::languages::Language::Go;
    let (cg, _) = build(&[(
        "main.go",
        "package main\n\
         type Runner interface { Go() }\n\
         type Fast struct{}\nfunc (f Fast) Go() {}\n\
         func use() { _ = Fast{} }\n\
         func run(r Runner) { r.Go() }\n",
        Go,
    )]);
    // Fast is constructed -> live -> interface_impls has (Runner, Go) -> [Fast.Go].
    let ids = cg.interface_impls.get(&("Runner".to_string(), "Go".to_string()))
        .expect("interface_impls populated");
    assert_eq!(ids.len(), 1);
    assert_eq!(ids[0].name, "Go");
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test --test integration resolution_test::callgraph_exposes_interface_impls`
Expected: FAIL — `no field interface_impls`.

- [ ] **Step 3: Implement**

1. Fields after `embedding_gaps` (call_graph.rs:78):
```rust
    #[serde(default)]
    pub interface_impls: BTreeMap<(String, String), Vec<FunctionId>>,
    #[serde(default)]
    pub interface_gaps: BTreeMap<String, usize>,
    #[serde(default)]
    pub interface_overapprox: BTreeMap<String, usize>,
```
2. Init `BTreeMap::new()` in all **five** constructors (round-2 claude): `empty`:93-94, `build_skeleton`:213-214, `build`:714-715, `build_direct_subset`, **and the constructor at `call_graph.rs:~1013`** (the `promoted_aliases` init sites are the exact set to mirror — grep `promoted_aliases: BTreeMap::new()` to find all five; missing one leaves the field uninitialized → compile error).
3. After `self.apply_go_embedding_promotion(files)` (call_graph.rs:717) add:
```rust
    self.apply_go_interface_dispatch(files);
```
4. Add the sibling methods (next to `apply_go_embedding_promotion`/`clear_promoted_embedding`):
```rust
fn clear_interface_dispatch(&mut self) {
    self.interface_impls.clear();
    self.interface_gaps.clear();
    self.interface_overapprox.clear();
}

pub fn apply_go_interface_dispatch(&mut self, files: &BTreeMap<String, ParsedFile>) {
    self.clear_interface_dispatch();
    if !files.values().any(|p| p.language == crate::languages::Language::Go) {
        return;
    }
    let live = crate::live_types::go_admission_live_set(files);
    let provider = crate::type_providers::go::GoTypeProvider::from_parsed_files(files);
    let table = provider.compute_interface_dispatch(&live);
    self.interface_impls = table.impls;
    for g in &table.gaps {
        *self.interface_gaps.entry(format!("{g:?}")).or_insert(0) += 1;
    }
    for o in &table.overapprox {
        *self.interface_overapprox.entry(format!("{o:?}")).or_insert(0) += 1;
    }
}
```
5. In `remove_files`, next to the `clear_promoted_embedding()` call (call_graph.rs:768):
```rust
    self.clear_interface_dispatch();
```
6. Add `go_admission_live_set` to `src/live_types.rs` (Go-scoped public wrapper):
```rust
/// Public Go-scoped live set over the admission-key alphabet (spec §8).
pub fn go_admission_live_set(files: &std::collections::BTreeMap<String, crate::ast::ParsedFile>)
    -> std::collections::BTreeSet<String>
{
    let mut live = std::collections::BTreeSet::new();
    for parsed in files.values() {
        if parsed.language == crate::languages::Language::Go {
            scan_go(parsed, &mut live);
        }
    }
    live
}
```
7. In `src/cpg/build.rs` `build_incremental`, after the CallGraph `merge` and the existing `apply_go_embedding_promotion(files)` (build.rs:192 region), add:
```rust
    cached_cg.apply_go_interface_dispatch(files);
```
8. Bump `src/cpg_cache.rs:49`: `const CACHE_VERSION: u32 = 9;` and update the test assertion that expects `8` (cpg_cache.rs:511) to `9`.

- [ ] **Step 4: Run to verify it passes**

Run: `cargo test --test integration resolution_test::callgraph_exposes_interface_impls && cargo test --lib cpg_cache`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/call_graph.rs src/live_types.rs src/cpg/build.rs src/cpg_cache.rs tests/integration/resolution_test.rs
git commit -m "feat(go-iface): CallGraph interface_impls + apply/clear siblings + build-path wiring; CACHE_VERSION 8->9 (spec §10/§11)

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

## Task 8: Resolution seam — interface consult + capability flip

Replace the terminal `None ⇒ ExternalReceiver` arm at the P6-lite branch (resolution.rs:438) with the `interface_impls` consult; `iface_key` is fallible (skip → drop) (spec §5).

**Files:**
- Modify: `src/resolution.rs:428-439`
- Test: `tests/integration/resolution_test.rs`; `eval/fixtures/go/interface_dispatch/expected.toml`

- [ ] **Step 1: Write the failing tests**

```rust
// tests/integration/resolution_test.rs
fn go_iface_src() -> &'static str {
    "package main\n\
     type Runner interface { Go() }\n\
     type Fast struct{}\nfunc (f Fast) Go() {}\n\
     type Slow struct{}\nfunc (s Slow) Go() {}\n\
     func use() { _ = Fast{}; _ = Slow{} }\n\
     func run(r Runner) { r.Go() }\n"
}

#[test]
fn interface_dispatch_resolves_multi_implementer_exact() {
    use prism::languages::Language::Go;
    let (cg, _) = build(&[("main.go", go_iface_src(), Go)]);
    let site = site_in(&cg, "run", "Go");
    let r = cg.resolve_call_site(&site);
    assert_eq!(r.len(), 2, "Fast + Slow (both live)");
    assert!(r.iter().all(|c| c.confidence == ResolutionConfidence::Exact));
    assert!(r.iter().all(|c| c.kind == ResolutionKind::InterfaceDispatch));
}
```

> Implementer note: the parser records `r.Go()` with `callee_name == "Go"` (the bare method name) and qualifier `r` — verified against `go_embedded_method_resolves_exact` which seeds `site_in(&cg, "run", "Ping")` for `w.Ping()`. The recovered receiver type `recv_ty` for `r` is `"Runner"` (P6-lite typed param).

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test --test integration resolution_test::interface_dispatch_resolves_multi_implementer_exact`
Expected: FAIL — currently drops `ExternalReceiver` (len 0).

- [ ] **Step 3: Implement**

Replace the `None` arm (resolution.rs:438) inside the P6-lite branch:
```rust
                    return match self.owner_lookup(recv_ty, name) {
                        Some(mut resolved) => {
                            for callee in &mut resolved {
                                if callee.kind == ResolutionKind::QualifiedOwner {
                                    callee.kind = recovered_kind;
                                }
                            }
                            ResolutionOutcome::hit(resolved)
                        }
                        None => match crate::resolution::iface_key(recv_ty) {
                            Some(k) => match self.interface_impls.get(&(k, name.to_string())) {
                                Some(ids) if !ids.is_empty() => ResolutionOutcome::hit(exact(
                                    ids.iter(),
                                    ResolutionKind::InterfaceDispatch,
                                )),
                                _ => ResolutionOutcome::dropped(DropReason::ExternalReceiver),
                            },
                            None => ResolutionOutcome::dropped(DropReason::ExternalReceiver),
                        },
                    };
```
> Note: `exact(ids.iter(), kind)` — `exact` (resolution.rs:169) takes `impl IntoIterator<Item = &FunctionId>`; `&Vec<FunctionId>` iterates `&FunctionId`. Good.

- [ ] **Step 4: Run to verify it passes**

Run: `cargo test --test integration resolution_test::interface_dispatch_resolves_multi_implementer_exact`
Expected: PASS.

- [ ] **Step 5: Flip the capability fixture + run the matrix gate**

Edit `eval/fixtures/go/interface_dispatch/expected.toml`: change `status = "known_fail"` → `status = "pass"`, and replace the rationale comment with a one-line Phase-IP note pointing at this spec. Then:
```bash
cargo build --release
cd eval && uv run tier-a --matrix-only --allow-stale-sut
```
Expected: `go/interface_dispatch` now `ok`; **no other flips** (paste any regression/flip-candidate into the eventual PR description, do not re-baseline).

- [ ] **Step 6: Commit**

```bash
git add src/resolution.rs tests/integration/resolution_test.rs eval/fixtures/go/interface_dispatch/expected.toml
git commit -m "feat(go-iface): interface consult at the P6-lite seam; flip go/interface_dispatch (spec §5/§13.6)

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

## Task 9: Interface basics — fallback + RTA + ExactOnly survival

Pin the §5 fallback semantics and the linchpin "keep fallback Exact" decision (spec §13.1).

**Files:**
- Test: `tests/integration/resolution_test.rs`

- [ ] **Step 1: Write the failing tests**

```rust
// tests/integration/resolution_test.rs
#[test]
fn interface_fallback_no_construction_full_set_exact() {
    use prism::languages::Language::Go;
    // constructs nothing -> empty live -> fallback -> full satisfier set, Exact.
    let (cg, _) = build(&[(
        "main.go",
        "package main\n\
         type Runner interface { Go() }\n\
         type Fast struct{}\nfunc (f Fast) Go() {}\n\
         type Slow struct{}\nfunc (s Slow) Go() {}\n\
         func run(r Runner) { r.Go() }\n",
        Go,
    )]);
    let site = site_in(&cg, "run", "Go");
    let r = cg.resolve_call_site(&site);
    assert_eq!(r.len(), 2);
    assert!(r.iter().all(|c| c.confidence == ResolutionConfidence::Exact));
}

#[test]
fn interface_rta_prunes_uninstantiated() {
    use prism::languages::Language::Go;
    // only Fast constructed -> Slow pruned.
    let (cg, _) = build(&[(
        "main.go",
        "package main\n\
         type Runner interface { Go() }\n\
         type Fast struct{}\nfunc (f Fast) Go() {}\n\
         type Slow struct{}\nfunc (s Slow) Go() {}\n\
         func use() { _ = Fast{} }\n\
         func run(r Runner) { r.Go() }\n",
        Go,
    )]);
    let site = site_in(&cg, "run", "Go");
    let r = cg.resolve_call_site(&site);
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].target.name, "Go");
}
```

- [ ] **Step 2: Run to verify it fails (then passes)**

Run: `cargo test --test integration resolution_test::interface_fallback_no_construction_full_set_exact && cargo test --test integration resolution_test::interface_rta_prunes_uninstantiated`
Expected: With Tasks 1-8 implemented these should already PASS (they pin behavior). If `interface_rta_prunes_uninstantiated` returns 2 instead of 1, the live-set wiring (Task 5/7) is not feeding `compute_interface_dispatch` — fix there. The fallback test failing means the seam isn't reached.

- [ ] **Step 3: ExactOnly survival (the linchpin) — public CPG graph walk**

The fallback edge's *Exact*-ness is the linchpin (spec §13.1). `#[cfg(test)]` accessors on a library type
are **not visible from the external integration-test crate** (round-2 both reviewers), so inspect the
**public** `ctx.cpg.graph` directly — `CpgContext.cpg` and `CodePropertyGraph.graph` are `pub` (verified
src/cpg/context.rs:41, src/cpg/build.rs:30), and `tests/ast/cpg_test.rs:573-577` already walks it. Put
this test in **`tests/ast/cpg_test.rs`** (the `ast` umbrella target), with a small file-local helper:
```rust
// tests/ast/cpg_test.rs
use petgraph::visit::EdgeRef;
use prism::cpg::{CodePropertyGraph, CpgContext, CpgEdge, CpgNode};
use prism::resolution::ResolutionConfidence;

fn exact_callee_names(ctx: &CpgContext, caller: &str) -> Vec<String> {
    let g: &CodePropertyGraph = &ctx.cpg;
    let mut out = Vec::new();
    for n in g.graph.node_indices() {
        if !matches!(&g.graph[n], CpgNode::Function { name, .. } if name == caller) { continue; }
        for e in g.graph.edges(n) {
            if matches!(e.weight(), CpgEdge::Call(ResolutionConfidence::Exact)) {
                if let CpgNode::Function { name, .. } = &g.graph[e.target()] {
                    out.push(name.clone());
                }
            }
        }
    }
    out
}

#[test]
fn interface_fallback_edge_is_cpg_exact() {
    use prism::languages::Language::Go;
    let mut files = std::collections::BTreeMap::new();
    files.insert("main.go".to_string(), prism::ast::ParsedFile::parse(
        "main.go",
        "package main\n\
         type Runner interface { Go() }\n\
         type Fast struct{}\nfunc (f Fast) Go() {}\n\
         func run(r Runner) { r.Go() }\n",
        Go,
    ).unwrap());
    let ctx = CpgContext::build(&files, None);
    // fallback fires (nothing constructed); the edge MUST be Exact to enter ExactOnly slices.
    assert!(exact_callee_names(&ctx, "run").iter().any(|n| n == "Go"),
        "fallback interface edge must be Exact to survive ExactOnly");
}
```
> Verify the import paths against the crate's re-exports (`prism::cpg::{CpgEdge, CpgNode}` may live under `prism::cpg::types` — grep the existing `use` lines in `tests/ast/cpg_test.rs`). If `--confidence exact` end-to-end coverage is also wanted, drive `prism nav callers --confidence exact` via `assert_cmd` on a tempdir repo; the CPG-edge assertion above is the sufficient oracle.

- [ ] **Step 4: Run to verify all pass**

Run: `cargo test --test integration resolution_test::interface_ && cargo test --test ast cpg_test::interface_fallback_edge_is_cpg_exact`
Expected: PASS (resolve_call_site interface_* tests + the ExactOnly-survival test).

- [ ] **Step 5: Commit**

```bash
git add tests/integration/resolution_test.rs tests/ast/cpg_test.rs
git commit -m "test(go-iface): fallback full-set Exact, RTA pruning, ExactOnly survival (spec §13.1)

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

## Task 10: Multi-implementer barrier precision (the gating fixture)

Both variants: live-intersection AND empty-live fallback fan-out into an `ExactOnly` barrier slice = exactly the satisfier set, no non-satisfier leakage (spec §13.7, the only PR-1 guard on uncapped-Exact).

**Files:**
- Test: `tests/ast/cpg_test.rs` (reuses the `exact_callee_names` helper added in Task 9; public graph API).

This is the **only PR-1 guard on the uncapped-Exact decision** while the §14 corpus gate is dormant —
so it covers **both** the live-intersection AND the empty-live fallback path (spec §13.7), and asserts
the §16 DataFlow fan-out tracks the call-edge fan-out with no non-satisfier leak.

- [ ] **Step 1: Write the failing test**

```rust
// tests/ast/cpg_test.rs  (exact_callee_names + imports from Task 9)
use petgraph::visit::EdgeRef;

// distinct functions reached from `caller` via an interprocedural DATA-FLOW edge (Step-5b).
fn dataflow_callee_funcs(ctx: &prism::cpg::CpgContext, caller: &str) -> std::collections::BTreeSet<String> {
    use prism::cpg::{CpgEdge, CpgNode};
    let g = &ctx.cpg.graph;
    let mut out = std::collections::BTreeSet::new();
    for n in g.node_indices() {
        if !matches!(&g[n], CpgNode::Function { name, .. } if name == caller) { continue; }
        for e in g.edges(n) {
            if e.weight().is_data_flow() {
                // map the data-flow target node back to its enclosing function name
                if let Some(f) = ctx.enclosing_function_name(e.target()) { out.insert(f); }
            }
        }
    }
    out
}

const SRC: &str =
    "package main\n\
     type Runner interface { Go() }\n\
     type Fast struct{}\nfunc (f Fast) Go() {}\n\
     type Slow struct{}\nfunc (s Slow) Go() {}\n\
     type Other struct{}\nfunc (o Other) Go(x int) {}\n"; // same name, WRONG sig -> not a satisfier

fn ctx_for(extra: &str) -> prism::cpg::CpgContext<'static> {
    use prism::languages::Language::Go;
    let src = format!("{SRC}{extra}");
    let mut files = std::collections::BTreeMap::new();
    files.insert("main.go".to_string(),
        prism::ast::ParsedFile::parse("main.go", &src, Go).unwrap());
    // NOTE: CpgContext borrows `files`; in the real test keep `files` alive (don't return a
    // dangling borrow — inline the body or thread `files` in). See implementer note below.
    prism::cpg::CpgContext::build(Box::leak(Box::new(files)), None)
}

#[test]
fn barrier_fanout_live_intersection_exact_no_leak() {
    // all three constructed; Other.Go has the wrong signature so it must not satisfy.
    let ctx = ctx_for("func use(){ _ = Fast{}; _ = Slow{}; _ = Other{} }\nfunc run(r Runner){ r.Go() }\n");
    let go: Vec<_> = exact_callee_names(&ctx, "run").into_iter().filter(|n| n == "Go").collect();
    assert_eq!(go.len(), 2, "exactly the 2 satisfiers (Fast,Slow); Other leaked?");
}

#[test]
fn barrier_fanout_empty_live_fallback_exact_no_leak() {
    // constructs NOTHING -> empty-live fallback -> full satisfier set, still exactly {Fast,Slow}.
    let ctx = ctx_for("func run(r Runner){ r.Go() }\n");
    let go: Vec<_> = exact_callee_names(&ctx, "run").into_iter().filter(|n| n == "Go").collect();
    assert_eq!(go.len(), 2, "fallback -> full satisfier set; Other (wrong sig) must not leak");
    // §16: Step-5b DataFlow fan-out tracks the call edges (no flow into the non-satisfier).
    let df = dataflow_callee_funcs(&ctx, "run");
    assert!(!df.contains("Other"), "non-satisfier received an interprocedural data-flow edge");
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test --test ast cpg_test::barrier_fanout_`
Expected: FAIL if `Other.Go(int)` leaks (wrong `canon_sig` must exclude it — Task 4) or if helpers are missing.

- [ ] **Step 3: Implement the helper / fix leakage**

`exact_callee_names` is from Task 9. Add `dataflow_callee_funcs` to the test file. For
`enclosing_function_name(NodeIndex) -> Option<String>`: if `CpgContext` exposes no public mapping,
implement it in the test by matching the target node's location against `CpgNode::Function` node ranges
via the public `ctx.cpg.graph` (find the `Function` node whose `[start_line,end_line]` contains the
target's line in the same file). No production change unless `Other` leaks — then the bug is in Task 4
`canon_sig` satisfaction (wrong-signature `Other.Go(int)` must not satisfy `Runner.Go()`).

> Implementer note: the `ctx_for`/`Box::leak` shim above is a test-only convenience to satisfy
> `CpgContext`'s borrow of `files`; prefer inlining `files` into each test (as Task 9 does) over leaking.
> Use whichever keeps `files` alive for `ctx`'s lifetime.

- [ ] **Step 4: Run to verify it passes**

Run: `cargo test --test ast cpg_test::barrier_fanout_`
Expected: PASS (both variants).

- [ ] **Step 5: Commit**

```bash
git add tests/ast/cpg_test.rs
git commit -m "test(go-iface): barrier precision both variants (live + fallback) + Step-5b DataFlow no-leak (spec §13.7/§16)

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

## Task 11: Replace-not-merge + non-Go regression + cache round-trip

Pin §13.8/§13.9/§13.10.

**Files:**
- Test: `tests/integration/resolution_test.rs`

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn removing_implementer_drops_interface_edge_no_phantom() {
    use prism::languages::Language::Go;
    use prism::cpg::CpgContext;
    let iface = ("iface.go".to_string(), prism::ast::ParsedFile::parse(
        "iface.go", "package main\ntype Runner interface { Go() }\nfunc run(r Runner){ r.Go() }\n", Go).unwrap());
    let impl_file = ("fast.go".to_string(), prism::ast::ParsedFile::parse(
        "fast.go", "package main\ntype Fast struct{}\nfunc (f Fast) Go() {}\nfunc use(){ _ = Fast{} }\n", Go).unwrap());
    let mut files: std::collections::BTreeMap<_, _> = [iface.clone(), impl_file].into_iter().collect();
    let cg_full = prism::call_graph::CallGraph::build(&files);
    assert!(cg_full.interface_impls.contains_key(&("Runner".to_string(), "Go".to_string())));
    // remove the only implementer file -> rebuild -> no stale entry.
    files.remove("fast.go");
    let cg_min = prism::call_graph::CallGraph::build(&files);
    assert!(!cg_min.interface_impls.contains_key(&("Runner".to_string(), "Go".to_string())),
        "no phantom implementer after removal");
    let _ = CpgContext::build(&files, None);
}

#[test]
fn non_go_repo_has_empty_interface_impls() {
    use prism::languages::Language::Rust;
    let (cg, _) = build(&[("a.rs", "pub struct A;\nimpl A { pub fn go(&self){} }\n", Rust)]);
    assert!(cg.interface_impls.is_empty());
}
```

- [ ] **Step 2: Run to verify it fails (then passes)**

Run: `cargo test --test integration resolution_test::removing_implementer_drops_interface_edge_no_phantom && cargo test --test integration resolution_test::non_go_repo_has_empty_interface_impls`
Expected: PASS with Task 7's `clear_interface_dispatch` + Go-gate. If the removal test fails, the clear/recompute path is wrong.

- [ ] **Step 3: Cache round-trip** — add to `tests/ast/cpg_cache_test.rs`, **mirroring the real template
`cache_v6_round_trips_edge_confidence` (cpg_cache_test.rs:20)** for the exact save/load helpers (there is
no `promoted_aliases` round-trip test to mirror — codex correction):

```rust
// tests/ast/cpg_cache_test.rs
#[test]
fn cache_v9_round_trips_interface_impls() {
    use prism::languages::Language::Go;
    let mut files = std::collections::BTreeMap::new();
    files.insert("main.go".to_string(), prism::ast::ParsedFile::parse(
        "main.go",
        "package main\ntype Runner interface { Go() }\n\
         type Fast struct{}\nfunc (f Fast) Go() {}\n\
         func use(){ _ = Fast{} }\nfunc run(r Runner){ r.Go() }\n",
        Go).unwrap());
    let cg = prism::call_graph::CallGraph::build(&files);
    assert!(cg.interface_impls.contains_key(&("Runner".to_string(), "Go".to_string())));
    // Round-trip via the SAME save/load path cache_v6_round_trips_edge_confidence uses (grep that test
    // for the helper — e.g. write to a tempdir cache then load), then assert the field survived:
    let restored = /* load_cache(...) as in the v6 template */ cg.clone();
    assert!(restored.interface_impls.contains_key(&("Runner".to_string(), "Go".to_string())),
        "interface_impls must survive the v9 cache round-trip");
}
```
The cross-version rejection needs no separate assertion beyond the bump: a pre-bump (v8) blob fails
`CACHE_VERSION` validation and loads as a **miss → rebuild** (cpg_cache.rs:253). If a forced-miss test is
wanted, mirror any existing cross-version assertion (grep `CACHE_VERSION` in `tests/`): a v8-header blob
must return a cache miss.

- [ ] **Step 4: Run to verify it passes**

Run: `cargo test --lib cpg_cache && cargo test --test integration resolution_test::non_go && cargo test --test integration resolution_test::removing_implementer`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add tests/integration/resolution_test.rs tests/ast/cpg_cache_test.rs
git commit -m "test(go-iface): replace-not-merge, non-Go regression, cache round-trip (spec §13.8-10)

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

## Task 12: Step-9 CHA — C++-only filter (spec §17)

CHA's override index + seed scan are currently language-blind (`build.rs:534`). Filter both to `type_db`-owned functions so a Go method sharing a name with a C++ virtual method cannot be minted.

**Files:**
- Modify: `src/cpg/build.rs:534-545` (candidate index) + `:547-555` (seed)
- Test: `tests/ast/cpg_test.rs` (real manual-`TypeDatabase` test; `TypeDatabase`/`RecordInfo` fields are pub — mirror `tests/integration/core_test.rs:722-725`).

- [ ] **Step 1: Write the failing test (real, manual TypeDatabase)**

```rust
// tests/ast/cpg_test.rs
#[test]
fn cha_does_not_mint_cross_language_edge() {
    use prism::languages::Language::{Cpp, Go};
    use prism::type_db::{RecordInfo, TypeDatabase};
    use std::collections::BTreeMap;

    // A Go function and a C++ class method share the name `Handle`. CHA (C++) must never
    // mint an Exact edge to the GO `Handle` (spec §17 — the index/seed were language-blind).
    let mut files = BTreeMap::new();
    files.insert("svc.go".to_string(), prism::ast::ParsedFile::parse(
        "svc.go", "package main\nfunc Handle() {}\n", Go).unwrap());
    files.insert("h.cpp".to_string(), prism::ast::ParsedFile::parse(
        "h.cpp",
        "struct Base { virtual void Handle(); };\n\
         struct D : Base { void Handle() override {} };\n\
         void drive(Base* b) { b->Handle(); }\n", Cpp).unwrap());

    // Manual tdb: Base has virtual method Handle, owned by the C++ file (RecordInfo.file).
    let mut tdb = TypeDatabase::default();
    let mut vmethods = BTreeMap::new();
    vmethods.insert("Handle".to_string(), "void()".to_string());
    tdb.records.insert("Base".to_string(), RecordInfo {
        name: "Base".to_string(),
        file: "h.cpp".to_string(),
        virtual_methods: vmethods,
        ..Default::default()
    });

    let ctx = prism::cpg::CpgContext::build(&files, Some(&tdb));

    // The Go `Handle` (in svc.go) must have NO incoming Exact CHA edge.
    use petgraph::visit::EdgeRef;
    use prism::cpg::{CpgEdge, CpgNode};
    use prism::resolution::ResolutionConfidence;
    let g = &ctx.cpg.graph;
    let go_handle = g.node_indices().find(|&n| matches!(&g[n],
        CpgNode::Function { name, file, .. } if name == "Handle" && file == "svc.go"))
        .expect("go Handle node");
    let inbound_exact = g.edges_directed(go_handle, petgraph::Direction::Incoming)
        .filter(|e| matches!(e.weight(), CpgEdge::Call(ResolutionConfidence::Exact)))
        .count();
    assert_eq!(inbound_exact, 0, "CHA minted a cross-language edge to the Go Handle");
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test --test ast cpg_test::cha_does_not_mint_cross_language_edge`
Expected: FAIL pre-fix — the language-blind `virtual_method_nodes` includes the Go `Handle`, so the C++
seed mints an Exact edge to it. (If CHA does not trigger in this minimal setup, strengthen the C++ caller
until Step 2 fails, then apply the fix — TDD.)

- [ ] **Step 3: Implement the filter (candidate index is load-bearing)**

The override **target** comes solely from `virtual_method_nodes` (build.rs:557-559), so filtering the
candidate index is the load-bearing fix (round-2 claude). Use the **`type_db`-owned** file set
(`RecordInfo.file`, type_db.rs:56) rather than an extension heuristic:
```rust
            let owned: std::collections::BTreeSet<&str> =
                tdb.records.values().map(|r| r.file.as_str()).collect();
            // ... candidate index:
                    for (&(ref file, ref name, ref _start_line), &idx) in &func_index {
                        if name == method_name && owned.contains(file.as_str()) {
                            virtual_method_nodes.entry(method_name.clone())
                                .or_default().push((record.name.clone(), idx));
                        }
                    }
```
Also gate the seed walk (build.rs:547-555) to skip non-owned caller functions (belt-and-suspenders).
This strictly *removes* spurious cross-language edges; pure-C++ behavior is unchanged (its funcs are
tdb-owned). No current corpus is mixed Go+C++, so corpus metrics are unaffected (spec §17).

- [ ] **Step 4: Run to verify it passes**

Run: `cargo test --test integration resolution_test::cha && cargo test --lib cpg::build`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/cpg/build.rs tests/integration/resolution_test.rs
git commit -m "fix(cha): restrict Step-9 virtual index + seed to C/C++ (spec §17)

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

## Task 13: Telemetry + harness attribution apparatus

`call-stats` reports the new counters; the Tier-A harness reads `Resolution.kind` per edge and adds per-site dispatch metadata (NOT a stratum — spec §14).

**Files:**
- Modify: `src/navigation/queries.rs` (`call-stats` JSON)
- Modify: `eval/tier_a/model.py` (`CallEdge.resolution_kind`), `eval/tier_a/sut.py` (extraction), `eval/tier_a/adjudication.py` (`dispatch_kind`)
- Test: `cargo test` (call-stats) + a Python unit test for sut extraction.

- [ ] **Step 1 (Rust): call-stats telemetry**

In `src/navigation/queries.rs` `call_stats`, compute a fan-out histogram from `interface_impls` and add
three fields next to `embedding_gaps`:
```rust
    let mut interface_fanout: std::collections::BTreeMap<usize, usize> = Default::default();
    for ids in cg.interface_impls.values() { *interface_fanout.entry(ids.len()).or_insert(0) += 1; }
    // ... in the call_stats object, beside "embedding_gaps":
    //   "interface_gaps": cg.interface_gaps,            // fatal GoDispatchGap counts (§15)
    //   "interface_overapprox": cg.interface_overapprox, // incl. NonLocalConstructionFallback + CrossPackageBareName (§15)
    //   "interface_fanout": interface_fanout,            // width -> count (codex#12: not discarded)
```
The fallback-fired count rides `interface_overapprox["NonLocalConstructionFallback"]`; `CrossPackageBareName`
is emitted in Task 4. The `InterfaceDispatch` resolution kind also rides the existing `kinds` histogram for free.
Add/extend the call-stats test — it lives under the **`cli`** test binary (codex#14), not `integration`:
`cargo test --test cli call_stats` (grep the existing `embedding_gaps` assertion there and mirror it). Expected: PASS.

- [ ] **Step 2 (Python): failing test for resolution_kind extraction**

```python
# eval/tests/test_sut_resolution_kind.py  (existing harness tests live under eval/tests/; package is `tier_a`)
from tier_a.model import FunctionDef, Location
from tier_a.sut import extract_callers

def test_extract_callers_reads_resolution_kind():
    seed = FunctionDef("Go", "method", None, Location("main.go", 9, 9), 9)
    ev = {"items": [{
        "location": {"file": "main.go", "start_line": 12, "end_line": 12},
        "symbol": {"Function": {"name": "run"}},
        "why": [
            {"CalledBy": {"caller": "run", "call_site_line": 12}},
            {"Resolution": {"kind": "interface_dispatch"}},
        ],
    }]}
    edges = extract_callers(seed, ev)
    assert edges[0].resolution_kind == "interface_dispatch"
```

- [ ] **Step 3 (Python): run to verify it fails**

Run: `cd eval && uv run pytest tests/test_sut_resolution_kind.py -q`
Expected: FAIL — `CallEdge` has no `resolution_kind`.

- [ ] **Step 4 (Python): implement**

1. `eval/tier_a/model.py:36-41` — add a trailing defaulted field:
```python
@dataclass(frozen=True)
class CallEdge:
    direction: str
    seed: FunctionDef
    other_def: Location | None
    other_name: str | None
    call_site: Location
    resolution_kind: str | None = None
```
2. `eval/tier_a/sut.py` — in `extract_callers` (sut.py:78-89) and `extract_callees` (sut.py:92-105), read the kind and pass it:
```python
        res = _why(it, "Resolution")
        rkind = res.get("kind") if res else None
        edges.append(CallEdge("caller", seed, other, name, site, rkind))
```
(and the analogous `CallEdge("callee", seed, other, calls.get("callee"), site, rkind)`).
3. `eval/tier_a/adjudication.py:26-44` — add a trailing defaulted field to `Adjudication`:
```python
    dispatch_kind: str | None = None
```
(`Adjudication(**json.loads(line))` and the 5-arg positional `CallEdge(...)` both stay backward-compatible.)

- [ ] **Step 5 (Python): run to verify it passes**

Run: `cd eval && uv run pytest tests/test_sut_resolution_kind.py -q`
Expected: PASS.

- [ ] **Step 6 (Python): persist resolution_kind in the probe JSON + replay (codex#7)**

So per-edge attribution survives `--report-only` replay (the run JSON currently stores only site triples).
In `eval/tier_a/cli.py`, where each probe's caller/callee sites are serialized into the run JSON (grep the
probe-storage block, ~cli.py:500-559), include each edge's `resolution_kind` (and `dispatch_kind` where a
pending record is written). Add a replay test under `eval/tests/`:
```python
# eval/tests/test_replay_keeps_resolution_kind.py
def test_run_json_roundtrips_resolution_kind(tmp_path):
    # Build a minimal probe dict with an edge carrying resolution_kind, write it to a run JSON,
    # load it back via the same (de)serialization cli.py uses, and assert resolution_kind survives.
    # (grep cli.py for the run-dict writer/reader helpers; mirror them.)
    ...
```

- [ ] **Step 7: Commit**

```bash
git add src/navigation/queries.rs eval/tier_a/model.py eval/tier_a/sut.py eval/tier_a/adjudication.py eval/tier_a/cli.py eval/tests/test_sut_resolution_kind.py eval/tests/test_replay_keeps_resolution_kind.py
git commit -m "feat(go-iface): call-stats telemetry + harness per-site resolution_kind/dispatch_kind + replay (spec §14)

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

> **Spec §14 reconciliation (this PR-1):** PR-1 ships per-edge `resolution_kind`/`dispatch_kind` extraction
> + **probe-JSON/pending persistence + replay** (above). The **fingerprinted interface-site manifest** and
> the **precision gate** move wholly to **PR-2** (they are unfalsifiable while the corpus gate is dormant —
> caddy-neutral — and the PR-2 receiver classes need a separate AST/drop-telemetry source, spec §14b). The
> spec §14 prose is updated to this boundary in the same commit as the plan.

---

## Final acceptance (after all tasks)

Run the full repo workflow + the light PR-1 acceptance (caddy-neutral by construction — spec §14e):
```bash
cargo fmt
cargo test                       # all unit + integration
cargo test --features mcp        # MCP adapter compiles + passes
cargo build --release
cd eval && uv run tier-a --matrix-only --allow-stale-sut   # go/interface_dispatch = ok, no other flips
cd eval && uv run tier-a --quick --allow-stale-sut         # needs rust-analyzer; no regression
```
**No 5-corpus rerun and no caddy re-baseline in PR-1** (those are PR-2's heavy ceremony). Paste any matrix regression/flip-candidate into the PR description rather than re-baselining.

Then dispatch a final whole-branch code review (codex+claude, the established dual-review) before opening the PR.

---

## Self-Review (plan author)

**Spec coverage:** §3 decisions → Tasks 1-8,12; §5 seam → Task 8; §6 canon → Tasks 3-4; §7 receiver kind → Task 4; §8 liveness → Task 5; §9 confidence → Tasks 6,9; §10 wiring/provider API/keys → Tasks 2,6,7,13; §11 cache/build paths → Task 7,11; §13 tests → Tasks 8-11,13; §14 attribution → Task 13 (per-edge plumbing + persistence + replay; manifest/gate deferred to PR-2 per the §14 reconciliation); §15 gap taxonomy → Tasks 3,6; §16 DataFlow fan-out → Task 10's `dataflow_callee_funcs` assertion (both variants; Step-5b re-resolves via the same ladder, no production change); §17 CHA → Task 12. PR-2 work-list → untouched (deferred). **No gaps.**

**Type consistency:** `InterfaceDispatchTable{impls,gaps,overapprox,fanout,fallback_fired}` (Task 6) matches its consumer (Task 7) and is seeded from `GoTypeData{sat_keys,dispatch_gaps,dispatch_overapprox}` (Task 4). `iface_key -> Option<String>` / `admission_key(bare,bool) -> String` (Task 2) used consistently in Tasks 6,8. `compute_interface_dispatch(&BTreeSet<String>)` (Task 6) fed by `go_admission_live_set` (Task 7). `set_ptr⊇set_value` (Task 4) consistent with the receiver-kind tests. `CallEdge.resolution_kind` (Task 13) matches `extract_callers/callees`. `GoDispatchGap`/`GoDispatchOverApprox` (Task 3) used in Tasks 4,6,13.

**Rev-2 status (plan-review folded):** the round-1 `#[cfg(test)]` accessors were replaced with **public `ctx.cpg.graph` walks** in `tests/ast/cpg_test.rs` (accessors are invisible to the external integration-test crate); the CHA test is now a **real manual-`TypeDatabase`** test (mirroring `core_test.rs:722`); the satisfaction model is **full-interface + augments `data.satisfaction` + `set_ptr⊇set_value` + promoted methods** (the round-1 BLOCKERs); tree-sitter-go field/kind names in Task 3 were verified vs 0.23.4 (channel `value`, array length, `method_elem`); `&T{}` liveness fixed via `unary_expression`; `CpgContext::build(_, None)`; `site_in(...,"Go")`; the 5th constructor init; telemetry/persistence wired; spec §14 reconciled. Remaining implementer lookups (each with the assertion as oracle): exact `CpgEdge`/`CpgNode` re-export paths (grep `tests/ast/cpg_test.rs` `use` lines), `enclosing_function_name` (Task 10), the cli.py run-dict (de)serialization helpers (Task 13 Step 6), and the cache save/load helpers (Task 11, mirror `cache_v6_round_trips_edge_confidence`).
