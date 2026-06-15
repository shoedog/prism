# Go Struct Embedding Method Promotion — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Resolve Go embedded-struct method calls (`w.Ping()` where `Wrap` embeds `Base`) by promoting the embedded type's concrete methods into the outer type's owner index, so the existing P6-lite seam resolves them as `Exact`.

**Architecture:** `CallGraph::build` consumes the existing (registry-independent) `GoTypeProvider` to read promoted concrete methods, then writes owner-index aliases `methods[(Wrap,Ping)] += Base::Ping` (the trait dual-key pattern) plus a `promoted_aliases` map for telemetry/incremental-replace. The unchanged `owner_lookup` at the resolver seam then hits the alias; the seam relabels the hit `EmbeddedPromotion`. Promotion is whole-program and recomputed (replace-not-merge) on incremental builds. Confidence is `Exact` (deterministic Go rule, single target — no fan-out).

**Tech Stack:** Rust, tree-sitter (Go grammar), `bincode` CPG cache, Python `uv` Tier-A harness.

**Spec:** `docs/superpowers/specs/2026-06-15-prism-phase-ip-go-embedding-design.md`. **Branch:** `phase-ip` (already checked out).

---

## File Structure

- **Modify `src/type_providers/go.rs`** — add the public `PromotedMethod` struct + `GoTypeProvider::promoted_struct_methods()` helper (transitive embedded-**struct** method promotion with depth; embedded interfaces skipped). Add an internal `#[cfg(test)] mod` test.
- **Modify `src/resolution.rs`** — add `ResolutionKind::EmbeddedPromotion` + its `as_str` arm; add the pure `normalize_go_struct_key` fn; add the `EmbeddedPromotion` label in the P6-lite seam.
- **Modify `src/call_graph.rs`** — add `CallGraph.promoted_aliases` + `embedding_gaps` fields (init in all 4 constructors); add `apply_go_embedding_promotion(&mut self, files)`; call it at the end of `build()`; Go-gate a `[…]`-strip in `recover_receiver`.
- **Modify `src/cpg/build.rs`** — call `apply_go_embedding_promotion(files)` after `merge` in `build_incremental` (replace-not-merge).
- **Modify `src/cpg_cache.rs`** — bump `CACHE_VERSION` 7→8 (new serialized `CallGraph` fields; bincode ignores `serde(default)`).
- **Modify `src/navigation/queries.rs`** — surface `embedding_gaps` in `call_stats` JSON.
- **Modify `eval/fixtures/go/embedded_method/expected.toml`** — flip `status` `known_fail`→`pass`.
- **Add tests** in `tests/integration/resolution_test.rs` (resolution behavior) and `tests/cli/call_stats_test.rs` (telemetry).

**Provider FunctionId identity (load-bearing):** the `FunctionId` built in `promoted_struct_methods` from a `GoMethod` (`{name, file, start_line, end_line}`) must equal the CallGraph function node so resolved edges materialize. `extract_method` (go.rs:399-400) computes `start_line/end_line = node.start_position().row + 1` / `…end…+1` from the `method_declaration`, identical to `node_line_range` used by `CallGraph::build` (call_graph.rs:250). They match.

---

## Task 1: `promoted_struct_methods` provider helper

**Files:**
- Modify: `src/type_providers/go.rs` (add `PromotedMethod` near the other type structs ~line 70; add the public method in the `impl GoTypeProvider` block near `collect_promoted_methods_from` ~line 550; add/extend `#[cfg(test)] mod tests` at end of file)

- [ ] **Step 1: Write the failing test**

Add at the end of `src/type_providers/go.rs`:

```rust
#[cfg(test)]
mod embedding_tests {
    use super::*;
    use crate::ast::ParsedFile;
    use std::collections::BTreeMap;

    fn provider(src: &str) -> GoTypeProvider {
        let mut files = BTreeMap::new();
        files.insert(
            "main.go".to_string(),
            ParsedFile::parse("main.go", src, Language::Go).unwrap(),
        );
        GoTypeProvider::from_parsed_files(&files)
    }

    #[test]
    fn promotes_concrete_method_from_embedded_struct() {
        let p = provider(
            "package main\ntype Base struct{}\nfunc (b Base) Ping() {}\ntype Wrap struct {\n\tBase\n}\n",
        );
        let got = p.promoted_struct_methods();
        let pings: Vec<_> = got
            .iter()
            .filter(|m| m.struct_name == "Wrap" && m.method == "Ping")
            .collect();
        assert_eq!(pings.len(), 1, "Wrap should promote one Ping");
        assert_eq!(pings[0].func_id.name, "Ping");
        assert_eq!(pings[0].func_id.file, "main.go");
        assert_eq!(pings[0].depth, 1);
    }

    #[test]
    fn promotes_transitively_with_increasing_depth() {
        let p = provider(
            "package main\ntype C struct{}\nfunc (c C) M() {}\ntype B struct{ C }\ntype A struct{ B }\n",
        );
        let got = p.promoted_struct_methods();
        let a_m: Vec<_> = got.iter().filter(|m| m.struct_name == "A" && m.method == "M").collect();
        assert_eq!(a_m.len(), 1);
        assert_eq!(a_m[0].depth, 2, "A embeds B embeds C: depth 2");
    }

    #[test]
    fn equal_depth_collision_returns_both_for_caller_to_drop() {
        // A embeds X and Y, both have M at depth 1 -> helper returns BOTH;
        // ambiguity resolution happens in CallGraph (Task 4).
        let p = provider(
            "package main\ntype X struct{}\nfunc (x X) M() {}\ntype Y struct{}\nfunc (y Y) M() {}\ntype A struct{\n\tX\n\tY\n}\n",
        );
        let got = p.promoted_struct_methods();
        let a_m: Vec<_> = got.iter().filter(|m| m.struct_name == "A" && m.method == "M").collect();
        assert_eq!(a_m.len(), 2, "both equal-depth M's returned");
        assert!(a_m.iter().all(|m| m.depth == 1));
    }

    #[test]
    fn embedded_interface_is_not_promoted() {
        // Embedding an interface is interface dispatch (deferred), not concrete promotion.
        let p = provider(
            "package main\ntype R interface { Read() }\ntype S struct {\n\tR\n}\n",
        );
        let got = p.promoted_struct_methods();
        assert!(
            got.iter().all(|m| m.struct_name != "S"),
            "interface methods must not be promoted as concrete"
        );
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --lib type_providers::go::embedding_tests 2>&1 | tail -20`
Expected: FAIL — `no method named promoted_struct_methods` / `cannot find type PromotedMethod`.

- [ ] **Step 3: Write minimal implementation**

In `src/type_providers/go.rs`, add the public struct after `GoMethod` (~line 70, before the `GoTypeData` section):

```rust
/// A concrete method promoted onto an outer struct via embedding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PromotedMethod {
    /// The outer struct that gains the method.
    pub struct_name: String,
    /// The promoted method's name.
    pub method: String,
    /// The defining method's identity (owner stays the defining type).
    pub func_id: FunctionId,
    /// Embedding depth (1 = directly embedded, 2 = embedded-of-embedded, …).
    pub depth: usize,
}
```

Add this method inside `impl GoTypeProvider` (place it right after `collect_promoted_methods_from`, ~line 549):

```rust
/// All concrete methods promoted onto each struct via **struct** embedding,
/// transitively, with depth. Embedded **interface** fields are skipped (their
/// methods have no concrete body — that is interface dispatch, deferred).
/// Returns every promotion including same-name duplicates from different embed
/// paths (the caller resolves direct-wins + equal-depth ambiguity).
pub fn promoted_struct_methods(&self) -> Vec<PromotedMethod> {
    let mut out = Vec::new();
    for struct_name in self.data.structs.keys() {
        let mut visited = BTreeSet::new();
        Self::collect_promotions(&self.data, struct_name, struct_name, 1, &mut visited, &mut out);
    }
    out
}

fn collect_promotions(
    data: &GoTypeData,
    outer: &str,
    current: &str,
    depth: usize,
    visited: &mut BTreeSet<String>,
    out: &mut Vec<PromotedMethod>,
) {
    let go_struct = match data.structs.get(current) {
        Some(s) => s,
        None => return,
    };
    for embedded_name in &go_struct.embedded {
        let bare = strip_generic(strip_pointer(embedded_name));
        if !visited.insert(bare.to_string()) {
            continue;
        }
        // Embedded interface -> skip (interface dispatch, deferred).
        if data.interfaces.contains_key(bare) {
            continue;
        }
        if let Some(methods) = data.methods.get(bare) {
            for m in methods {
                out.push(PromotedMethod {
                    struct_name: outer.to_string(),
                    method: m.name.clone(),
                    func_id: FunctionId {
                        name: m.name.clone(),
                        file: m.file.clone(),
                        start_line: m.start_line,
                        end_line: m.end_line,
                    },
                    depth,
                });
            }
        }
        // Recurse into the embedded struct (transitive promotion).
        Self::collect_promotions(data, outer, bare, depth + 1, visited, out);
    }
}
```

Add the `strip_generic` helper next to `strip_pointer` (~line 688):

```rust
/// Strip a Go generic type-argument suffix: `Wrap[T]` → `Wrap`.
fn strip_generic(s: &str) -> &str {
    s.split('[').next().unwrap_or(s).trim()
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --lib type_providers::go::embedding_tests 2>&1 | tail -20`
Expected: PASS (4 tests).

- [ ] **Step 5: Commit**

```bash
git add src/type_providers/go.rs
git commit -m "feat(go): promoted_struct_methods helper — transitive embedded-struct method promotion"
```

---

## Task 2: `ResolutionKind::EmbeddedPromotion`

**Files:**
- Modify: `src/resolution.rs:16-54` (enum + `as_str`)

- [ ] **Step 1: Write the failing test**

Add to the bottom of `src/resolution.rs` (inside the existing `#[cfg(test)] mod tests` if present, else add one):

```rust
#[cfg(test)]
mod kind_tests {
    use super::ResolutionKind;
    #[test]
    fn embedded_promotion_as_str() {
        assert_eq!(ResolutionKind::EmbeddedPromotion.as_str(), "embedded_promotion");
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --lib resolution::kind_tests 2>&1 | tail -20`
Expected: FAIL — `no variant named EmbeddedPromotion`.

- [ ] **Step 3: Write minimal implementation**

In `src/resolution.rs`, add the variant to the enum (after `StemMulti`, line 32):

```rust
    StemSingle,
    StemMulti,
    EmbeddedPromotion,
}
```

Add the arm to `as_str` (after the `StemMulti` arm, line 52):

```rust
            ResolutionKind::StemMulti => "stem_multi",
            ResolutionKind::EmbeddedPromotion => "embedded_promotion",
        }
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --lib resolution::kind_tests 2>&1 | tail -20`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/resolution.rs
git commit -m "feat(resolution): add EmbeddedPromotion resolution kind"
```

---

## Task 3: `normalize_go_struct_key`

**Files:**
- Modify: `src/resolution.rs` (add the pure fn after `owner_key`, ~line 84)

- [ ] **Step 1: Write the failing test**

Add to `src/resolution.rs` `#[cfg(test)] mod kind_tests` (or a new test module):

```rust
#[cfg(test)]
mod go_key_tests {
    use super::normalize_go_struct_key;
    #[test]
    fn strips_pointer_ref_and_generic_args() {
        assert_eq!(normalize_go_struct_key("Wrap"), "Wrap");
        assert_eq!(normalize_go_struct_key("*Wrap"), "Wrap");
        assert_eq!(normalize_go_struct_key("&Wrap"), "Wrap");
        assert_eq!(normalize_go_struct_key("Wrap[User]"), "Wrap");
        assert_eq!(normalize_go_struct_key("*Repo[T]"), "Repo");
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --lib resolution::go_key_tests 2>&1 | tail -20`
Expected: FAIL — `cannot find function normalize_go_struct_key`.

- [ ] **Step 3: Write minimal implementation**

In `src/resolution.rs`, after `owner_key` (line 84):

```rust
/// Bare-name key for a Go struct/receiver type: `owner_key` (strips refs/pointers
/// and `<…>`) then strips Go `[…]` generic type-arguments. Used for embedding
/// owner keys and the recovered Go receiver so they match. (Cross-package `pkg.`
/// normalization is deferred to the interface spec.)
pub fn normalize_go_struct_key(text: &str) -> String {
    let k = owner_key(text);
    k.split('[').next().unwrap_or(&k).trim().to_string()
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --lib resolution::go_key_tests 2>&1 | tail -20`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/resolution.rs
git commit -m "feat(resolution): normalize_go_struct_key (owner_key + Go generic strip)"
```

---

## Task 4: CallGraph promotion fields, `apply_go_embedding_promotion`, build() wiring, and the seam

This is the integration task: it makes the embedded-method call resolve. Several files change together; the resolution tests pass at the end.

**Files:**
- Modify: `src/call_graph.rs` — `CallGraph` struct (47-71), `empty()` (75-86), `build_skeleton` (94-204 init region), `build()` (206-…), `build_direct_subset` (777-…), `recover_receiver` (1260-1265)
- Modify: `src/resolution.rs` — the P6-lite seam (404-424)
- Test: `tests/integration/resolution_test.rs`

- [ ] **Step 1: Write the failing tests**

Append to `tests/integration/resolution_test.rs`:

```rust
#[test]
fn go_embedded_method_resolves_exact() {
    use prism::languages::Language::Go;
    let (cg, _) = build(&[(
        "main.go",
        "package main\ntype Base struct{}\nfunc (b Base) Ping() {}\ntype Wrap struct {\n\tBase\n}\nfunc run(w Wrap) {\n\tw.Ping()\n}\n",
        Go,
    )]);
    let site = site_in(&cg, "run", "Ping");
    let r = cg.resolve_call_site(&site);
    assert_eq!(r.len(), 1, "w.Ping() resolves to Base::Ping");
    assert_eq!(r[0].target.name, "Ping");
    assert_eq!(r[0].target.file, "main.go");
    assert_eq!(r[0].confidence, ResolutionConfidence::Exact);
    assert_eq!(r[0].kind, ResolutionKind::EmbeddedPromotion);
}

#[test]
fn go_direct_method_wins_over_promoted() {
    use prism::languages::Language::Go;
    let (cg, _) = build(&[(
        "main.go",
        "package main\ntype Base struct{}\nfunc (b Base) Ping() {}\ntype Wrap struct {\n\tBase\n}\nfunc (w Wrap) Ping() {}\nfunc run(w Wrap) {\n\tw.Ping()\n}\n",
        Go,
    )]);
    let site = site_in(&cg, "run", "Ping");
    let r = cg.resolve_call_site(&site);
    assert_eq!(r.len(), 1);
    // The direct Wrap.Ping is on a later line than Base.Ping; assert it's the direct one.
    assert_eq!(r[0].target.start_line, 6, "direct Wrap.Ping (line 6) wins");
    assert_ne!(r[0].kind, ResolutionKind::EmbeddedPromotion);
}

#[test]
fn go_equal_depth_embedding_ambiguity_drops() {
    use prism::languages::Language::Go;
    use prism::resolution::DropReason;
    let (cg, _) = build(&[(
        "main.go",
        "package main\ntype X struct{}\nfunc (x X) M() {}\ntype Y struct{}\nfunc (y Y) M() {}\ntype A struct {\n\tX\n\tY\n}\nfunc run(a A) {\n\ta.M()\n}\n",
        Go,
    )]);
    let site = site_in(&cg, "run", "M");
    let out = cg.resolve_call_site_full(&site);
    assert!(out.resolved.is_empty(), "equal-depth M is ambiguous -> not promoted");
    // Falls through to the existing receiver drop (no in-scope owner for A.M).
    assert!(matches!(out.drop, Some(DropReason::ExternalReceiver) | Some(DropReason::MultiOwnerCollision)));
}

#[test]
fn go_embedded_interface_field_not_promoted() {
    use prism::languages::Language::Go;
    let (cg, _) = build(&[(
        "main.go",
        "package main\ntype R interface { Read() }\ntype S struct {\n\tR\n}\nfunc run(s S) {\n\ts.Read()\n}\n",
        Go,
    )]);
    let site = site_in(&cg, "run", "Read");
    let out = cg.resolve_call_site_full(&site);
    assert!(out.resolved.is_empty(), "embedded interface method is not concrete-promoted");
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --test integration resolution_test::go_embedded 2>&1 | tail -25`
Expected: FAIL — `go_embedded_method_resolves_exact` resolves to 0 callees (current `ExternalReceiver` drop).

- [ ] **Step 3a: Add the CallGraph fields**

In `src/call_graph.rs`, add to the `CallGraph` struct (after `receiver_vars`, line 70):

```rust
    /// S3 (Go): receiver variable name per method FunctionId.
    #[serde(default)]
    pub receiver_vars: BTreeMap<FunctionId, String>,
    /// Phase-IP (Go embedding): promoted alias `(outer_struct_key, method)` →
    /// the embedded methods' FunctionIds. Keys are also the EmbeddedPromotion
    /// label set; the map carries fids for clean incremental replace.
    #[serde(default)]
    pub promoted_aliases: BTreeMap<(String, String), Vec<FunctionId>>,
    /// Phase-IP (Go embedding): gap telemetry, e.g. {"ambiguous": n}.
    #[serde(default)]
    pub embedding_gaps: BTreeMap<String, usize>,
}
```

In `empty()` (line 76-85), add both fields:

```rust
            receiver_vars: BTreeMap::new(),
            promoted_aliases: BTreeMap::new(),
            embedding_gaps: BTreeMap::new(),
        }
```

In `build_skeleton` (the returned struct literal, ~line 195-203) and `build_direct_subset` (its returned struct literal, ~line 190 region of that fn) add the two fields initialized empty:

```rust
            receiver_vars,
            promoted_aliases: BTreeMap::new(),
            embedding_gaps: BTreeMap::new(),
        }
```

(`build_skeleton` and `build_direct_subset` do **not** promote — promotion is whole-program, applied in `build()` and after merge in `build_incremental`.)

- [ ] **Step 3b: Add `apply_go_embedding_promotion`**

In `src/call_graph.rs`, add this method to `impl CallGraph` (place near `merge`, ~line 770):

```rust
/// Recompute Go embedding promotions over `files` and write owner-index aliases.
/// Idempotent: clears prior aliases first (supports incremental replace).
pub fn apply_go_embedding_promotion(&mut self, files: &BTreeMap<String, ParsedFile>) {
    // 1. Remove prior promoted aliases, preserving any direct methods on the key.
    let prior = std::mem::take(&mut self.promoted_aliases);
    for (key, fids) in &prior {
        if let Some(v) = self.methods.get_mut(key) {
            v.retain(|f| !fids.contains(f));
            if v.is_empty() {
                self.methods.remove(key);
            }
        }
    }
    self.embedding_gaps.clear();

    // Skip the provider build entirely if there are no Go files.
    if !files.values().any(|p| p.language == crate::languages::Language::Go) {
        return;
    }

    // 2. Group promotions by (normalized struct key, method).
    let provider = crate::type_providers::go::GoTypeProvider::from_parsed_files(files);
    let mut by_key: BTreeMap<(String, String), Vec<(usize, FunctionId)>> = BTreeMap::new();
    for pm in provider.promoted_struct_methods() {
        let key = (
            crate::resolution::normalize_go_struct_key(&pm.struct_name),
            pm.method,
        );
        by_key.entry(key).or_default().push((pm.depth, pm.func_id));
    }

    // 3. For each (struct, method): direct-wins, then uniquely-shallowest else ambiguous-drop.
    let mut ambiguous = 0usize;
    for ((owner, method), mut cands) in by_key {
        // direct-wins: a method the struct itself owns under this key.
        let has_direct = self
            .methods
            .get(&(owner.clone(), method.clone()))
            .map(|v| v.iter().any(|f| self.method_owners.get(f) == Some(&owner)))
            .unwrap_or(false);
        if has_direct {
            continue;
        }
        cands.sort_by_key(|(d, _)| *d);
        let min_depth = cands[0].0;
        let shallowest: Vec<&FunctionId> =
            cands.iter().filter(|(d, _)| *d == min_depth).map(|(_, f)| f).collect();
        if shallowest.len() > 1 {
            ambiguous += 1; // equal-depth selector ambiguity -> not promoted
            continue;
        }
        let fid = shallowest[0].clone();
        self.methods
            .entry((owner.clone(), method.clone()))
            .or_default()
            .push(fid.clone());
        self.promoted_aliases
            .entry((owner, method))
            .or_default()
            .push(fid);
    }
    if ambiguous > 0 {
        self.embedding_gaps.insert("ambiguous".to_string(), ambiguous);
    }
}
```

- [ ] **Step 3c: Call it at the end of `build()`**

In `src/call_graph.rs build()`, the function currently ends by returning a `CallGraph { … }` literal (after Phase 3, ~line 700). Bind it and apply before returning. Change the final `CallGraph { … }` expression to:

```rust
        let mut cg = CallGraph {
            functions,
            calls,
            callers,
            static_functions,
            imports,
            methods,
            method_owners,
            receiver_vars,
            promoted_aliases: BTreeMap::new(),
            embedding_gaps: BTreeMap::new(),
        };
        cg.apply_go_embedding_promotion(files);
        cg
```

(If `build()`'s tail is structured differently, the rule is: construct the `CallGraph` with the two new fields empty, then call `cg.apply_go_embedding_promotion(files)` on it, then return `cg`. `files` is the `&BTreeMap<String, ParsedFile>` parameter of `build`.)

- [ ] **Step 3d: Go-gate a generic strip in `recover_receiver`**

In `src/call_graph.rs recover_receiver` (line 1260-1265), the recovered receiver is `owner_key`-normalized. For Go, also strip generic args so a `Wrap[T]` receiver matches the bare promoted key. Replace the `.map(...)` closure:

```rust
    parsed
        .receiver_type_in_fn(func_node, q, line)
        .map(|(ty, how)| {
            let peeled = crate::resolution::peel_type(&ty);
            let key = if parsed.language == crate::languages::Language::Go {
                crate::resolution::normalize_go_struct_key(&peeled)
            } else {
                crate::resolution::owner_key(&peeled)
            };
            (key, how)
        })
```

- [ ] **Step 3e: Label the seam**

In `src/resolution.rs`, the P6-lite recovered-receiver block (404-424) calls `self.owner_lookup(recv_ty, name)`. On a hit, relabel an embedding alias. Replace the `Some(mut resolved) => { … }` arm (lines 412-421):

```rust
                    return match self.owner_lookup(recv_ty, name) {
                        Some(mut resolved) => {
                            let is_embed = self
                                .promoted_aliases
                                .contains_key(&(recv_ty.to_string(), name.to_string()));
                            for callee in &mut resolved {
                                if is_embed {
                                    callee.kind = ResolutionKind::EmbeddedPromotion;
                                } else if callee.kind == ResolutionKind::QualifiedOwner {
                                    callee.kind = recovered_kind;
                                }
                                // Trait-CHA hits keep TraitCha (dyn Trait receivers).
                            }
                            ResolutionOutcome::hit(resolved)
                        }
                        None => ResolutionOutcome::dropped(DropReason::ExternalReceiver),
                    };
```

(`recv_ty` here is the Go-normalized receiver from `recover_receiver` (3d), so it matches the promoted alias key inserted with `normalize_go_struct_key`.)

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --test integration resolution_test::go_embedded 2>&1 | tail -25`
Run: `cargo test --test integration resolution_test::go_direct_method_wins 2>&1 | tail -25`
Run: `cargo test --test integration resolution_test::go_equal_depth 2>&1 | tail -25`
Expected: PASS (4 tests). Then run the full Go resolution suite for no regressions:
Run: `cargo test --test integration resolution_test:: 2>&1 | tail -25`
Expected: all PASS.

- [ ] **Step 5: Commit**

```bash
git add src/call_graph.rs src/resolution.rs tests/integration/resolution_test.rs
git commit -m "feat(go): promote embedded-struct methods into the owner index (Exact, direct-wins, equal-depth drop)"
```

---

## Task 5: Replace-not-merge on incremental builds

**Files:**
- Modify: `src/cpg/build.rs:185-191` (`build_incremental`)
- Test: `tests/integration/resolution_test.rs`

- [ ] **Step 1: Write the failing test**

Append to `tests/integration/resolution_test.rs`:

```rust
#[test]
fn go_embedding_recomputed_on_incremental_rebuild() {
    use prism::cpg::CodePropertyGraph;
    use prism::data_flow::DataFlowGraph;
    use prism::languages::Language::Go;
    use std::collections::{BTreeMap, BTreeSet};

    // v1: Wrap embeds Base (has Ping). Full build promotes Wrap.Ping.
    let mut files = BTreeMap::new();
    files.insert(
        "main.go".to_string(),
        prism::ast::ParsedFile::parse(
            "main.go",
            "package main\ntype Base struct{}\nfunc (b Base) Ping() {}\ntype Wrap struct {\n\tBase\n}\n",
            Go,
        )
        .unwrap(),
    );
    let cg_v1 = prism::call_graph::CallGraph::build(&files);
    assert!(cg_v1.promoted_aliases.contains_key(&("Wrap".to_string(), "Ping".to_string())));

    // v2: edit main.go to remove the embedding. Incremental rebuild must DROP the alias.
    let mut files2 = BTreeMap::new();
    files2.insert(
        "main.go".to_string(),
        prism::ast::ParsedFile::parse(
            "main.go",
            "package main\ntype Base struct{}\nfunc (b Base) Ping() {}\ntype Wrap struct{}\n",
            Go,
        )
        .unwrap(),
    );
    let changed: BTreeSet<String> = ["main.go".to_string()].into_iter().collect();
    let dfg = DataFlowGraph::build(&files);
    let cpg = CodePropertyGraph::build_incremental(cg_v1, dfg, &changed, &files2, None);
    assert!(
        !cpg.call_graph().promoted_aliases.contains_key(&("Wrap".to_string(), "Ping".to_string())),
        "stale promoted alias must be cleared on incremental rebuild"
    );
}
```

(Note: if `CodePropertyGraph` does not expose `call_graph()`, use the public accessor the codebase already provides for the merged CallGraph; check `src/cpg.rs` / `src/cpg/types.rs` for the field/getter and adjust this assertion to read `promoted_aliases` off it.)

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test integration resolution_test::go_embedding_recomputed 2>&1 | tail -25`
Expected: FAIL — the stale `Wrap→Ping` alias survives (merge carried it; no recompute).

- [ ] **Step 3: Write minimal implementation**

In `src/cpg/build.rs build_incremental`, after the merge (line 187), before `assemble_graph` (line 191), add:

```rust
        // Step 3: Merge fresh into retained.
        cached_cg.merge(fresh_cg);
        cached_dfg.merge(fresh_dfg);

        // Step 3b (Phase-IP): Go embedding promotion is whole-program — recompute
        // (replace-not-merge) over ALL merged files so a removed/changed embedding
        // does not leave a stale promoted alias (remove_files prunes methods by
        // fid.file only).
        cached_cg.apply_go_embedding_promotion(files);

        // Step 4: Assemble the petgraph from the merged CG/DFG.
        Self::assemble_graph(cached_cg, cached_dfg, files, type_db)
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --test integration resolution_test::go_embedding_recomputed 2>&1 | tail -25`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/cpg/build.rs tests/integration/resolution_test.rs
git commit -m "fix(go): recompute embedding promotion on incremental rebuild (replace-not-merge)"
```

---

## Task 6: Bump `CACHE_VERSION` to 8

**Files:**
- Modify: `src/cpg_cache.rs:44-47`

- [ ] **Step 1: Write the failing test**

Add to `src/cpg_cache.rs` (inside the existing `#[cfg(test)] mod tests`):

```rust
    #[test]
    fn cache_version_is_8_for_embedding_fields() {
        // promoted_aliases/embedding_gaps are new serialized CallGraph fields;
        // bincode ignores serde(default), so the version bump is the format-safety
        // mechanism (not GIT_SHA alone).
        assert_eq!(super::CACHE_VERSION, 8);
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --lib cpg_cache::tests::cache_version_is_8 2>&1 | tail -20`
Expected: FAIL — `assertion left == right` (7 != 8).

- [ ] **Step 3: Write minimal implementation**

In `src/cpg_cache.rs`, update the version constant + doc (line 44-47):

```rust
/// - v6: EFT CpgEdge::Call/Return carry ResolutionConfidence.
/// - v7: + git_sha in cache key (resolver auto-invalidation).
/// - v8: Phase-IP CallGraph.promoted_aliases + embedding_gaps (Go embedding).
const CACHE_VERSION: u32 = 8; // bincode ignores serde(default) for new trailing fields.
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --lib cpg_cache::tests::cache_version_is_8 2>&1 | tail -20`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/cpg_cache.rs
git commit -m "chore(cache): bump CACHE_VERSION 7->8 for Go embedding CallGraph fields"
```

---

## Task 7: Surface embedding telemetry in `call-stats`

**Files:**
- Modify: `src/navigation/queries.rs:38-46` (`call_stats` JSON)
- Test: `tests/cli/call_stats_test.rs`

- [ ] **Step 1: Write the failing test**

Append to `tests/cli/call_stats_test.rs`:

```rust
#[test]
fn call_stats_reports_embedded_promotion_kind() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("main.go"),
        "package main\ntype Base struct{}\nfunc (b Base) Ping() {}\ntype Wrap struct {\n\tBase\n}\nfunc run(w Wrap) {\n\tw.Ping()\n}\n",
    )
    .unwrap();
    let out = Command::cargo_bin("prism")
        .unwrap()
        .args(["nav", "--no-cache", "call-stats", "--repo"])
        .arg(dir.path())
        .output()
        .unwrap();
    assert!(out.status.success(), "stderr: {}", String::from_utf8_lossy(&out.stderr));
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["kinds"]["embedded_promotion"], 1);
    assert!(v["embedding_gaps"].is_object());
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test cli call_stats_test::call_stats_reports_embedded_promotion 2>&1 | tail -25`
Expected: FAIL — `embedding_gaps` key absent (`null`), `kinds.embedded_promotion` absent.

The `kinds` histogram already includes `embedded_promotion` automatically (queries.rs:31 keys off `c.kind.as_str()`), so only `embedding_gaps` needs surfacing.

- [ ] **Step 3: Write minimal implementation**

In `src/navigation/queries.rs call_stats`, add `embedding_gaps` to the JSON (line 38-46):

```rust
    serde_json::json!({
        "total_call_sites": total,
        "kinds": kinds,
        "demoted_edges": demoted,
        "dropped_multi_owner": multi,
        "dropped_external_receiver": external,
        "dropped_import_external": import_ext,
        "unresolved_unknown_name": unknown,
        "embedding_gaps": cg.embedding_gaps,
    })
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --test cli call_stats_test::call_stats_reports_embedded_promotion 2>&1 | tail -25`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/navigation/queries.rs tests/cli/call_stats_test.rs
git commit -m "feat(nav): surface embedding_gaps in call-stats; EmbeddedPromotion kind histogrammed"
```

---

## Task 8: Flip the capability fixture

**Files:**
- Modify: `eval/fixtures/go/embedded_method/expected.toml`

- [ ] **Step 1: Verify the gap is now a flip candidate (pre-change)**

Run: `cargo build --release && cd eval && uv run tier-a --matrix-only --allow-stale-sut 2>&1 | grep -i embedded_method; cd ..`
Expected: `embedded_method` reports `flip_candidate` (status is still `known_fail` but the resolver now matches the expected caller). If it still reports `expected_gap`, Task 4 is incomplete — stop and fix.

- [ ] **Step 2: Flip the fixture status**

Edit `eval/fixtures/go/embedded_method/expected.toml` — replace the `status` line and rationale comment:

```toml
[case]
language = "go"
capability = "embedded_method"
# Phase-IP (2026-06-15): RESOLVED. `w.Ping()` where Wrap embeds Base now resolves —
# CallGraph::build promotes the embedded Base.Ping into Wrap's owner index
# (apply_go_embedding_promotion), so owner_lookup hits and the seam labels it
# EmbeddedPromotion (Exact). See docs/superpowers/specs/2026-06-15-prism-phase-ip-go-embedding-design.md.
status = "pass"
[seed]
symbol = "Ping"
file = "main.go"
line = 5
[[expect.callers]]
file = "main.go"
line = 12
[expect]
exact = true
```

- [ ] **Step 3: Verify the fixture now passes**

Run: `cargo build --release && cd eval && uv run tier-a --matrix-only --allow-stale-sut 2>&1 | grep -iE 'embedded_method|interface_dispatch|regression'; cd ..`
Expected: `embedded_method` → `ok`; `interface_dispatch` stays `expected_gap` (deferred); **no** `regression` lines anywhere.

- [ ] **Step 4: Commit**

```bash
git add eval/fixtures/go/embedded_method/expected.toml
git commit -m "test(eval): flip go/embedded_method known_fail -> pass (Phase-IP embedding)"
```

---

## Task 9: Full repo validation + final commit

**Files:** none (validation only)

- [ ] **Step 1: Format**

Run: `cargo fmt`
Then: `cargo fmt --check` — Expected: clean (exit 0).

- [ ] **Step 2: Full test suite (default + mcp features)**

Run: `cargo test 2>&1 | tail -30`
Expected: all PASS.
Run: `cargo test --features mcp 2>&1 | tail -15`
Expected: all PASS.

- [ ] **Step 3: Tier-A matrix + quick (accuracy harness)**

Run: `cargo build --release && cd eval && uv run tier-a --matrix-only --allow-stale-sut 2>&1 | tail -30; cd ..`
Expected: exit 0; `go/embedded_method` `ok`; no `regression`.
Run: `cd eval && uv run tier-a --quick --allow-stale-sut 2>&1 | tail -30; cd ..`
Expected: exit 0; no new regressions vs the committed baseline. Paste any regression/flip-candidate lines into the PR description (do **not** re-baseline — full-corpus runs are human-triggered, spec §11).

- [ ] **Step 4: Commit any fmt-only changes**

```bash
git add -A
git commit -m "chore: cargo fmt" || echo "nothing to format"
```

---

## Self-Review

**1. Spec coverage** (each spec section → task):
- §0/§4 embedding promotion (owner-index dual-key, transitive, direct-wins, equal-depth drop) → Tasks 1, 4.
- §2 consume registry-independent `GoTypeProvider` in `CallGraph::build` → Tasks 1, 4.
- §3 confidence Exact + `EmbeddedPromotion` kind → Tasks 2, 4.
- §5 receiver recovery + addressability → Task 4 (typed-param fixture; addressability is the existing recovery surface — value selectors of value-receiver methods covered; pointer-receiver-of-addressable is the same FunctionId so no extra code; non-addressable bases never reach the recovered-receiver seam).
- §6 `normalize_go_struct_key` (owner_key + generic strip; pkg. deferred) → Task 3, applied in Tasks 4 (recover_receiver + alias keys).
- §7 gap contract (ambiguity counter) → Tasks 4 (count) + 7 (surface). **Note:** the spec listed three gap counters; this plan ships the `"ambiguous"` counter (the meaningful one) and skips embedded-interface/generic counters as silent (generics are *resolved* via stripping, not gapped; embedded interfaces are skipped in Task 1). A one-line spec trim is warranted (see below).
- §8 failure modes → Tasks 1, 4 tests.
- §9 cache v8 + replace-not-merge across build/incremental; build_scoped best-effort → Tasks 5, 6 (build + incremental). `build_scoped` is **not** modified: it builds the CallGraph over its scoped subset, so embedding is scoped/best-effort there exactly as the spec's §9 states (nav builds over all files → metric safe). No task needed; documented here as intentional.
- §10 tests + matrix flip → Tasks 4, 7, 8, 9.
- §11 human-triggered caddy rerun → out of plan scope (noted in Task 9 Step 3).

**2. Placeholder scan:** none. Every code step shows complete code. The two "if the codebase differs, adjust" notes (Task 5 `call_graph()` accessor; Task 4 `build()` tail shape) are guardrails for exact-line drift, with the rule stated, not placeholders.

**3. Type consistency:** `PromotedMethod {struct_name, method, func_id, depth}` (Task 1) is consumed unchanged in Task 4. `promoted_aliases: BTreeMap<(String,String), Vec<FunctionId>>` and `embedding_gaps: BTreeMap<String,usize>` (Task 4 fields) are read in Task 5 (`promoted_aliases`), Task 7 (`embedding_gaps`). `normalize_go_struct_key` (Task 3) used in Task 4 (recover_receiver + alias keys) and is the same fn the seam's `recv_ty` already carries (Task 4 3d). `ResolutionKind::EmbeddedPromotion` (Task 2) set in Task 4 seam, read in Task 7 via `as_str`. Consistent.

**Spec trim to apply before execution:** in the embedding spec, narrow §7 to the `"ambiguous"` gap counter and state that generic structs are **resolved** via `normalize_go_struct_key` stripping `[…]` (not a gap), and embedded-interface fields are skipped in `promoted_struct_methods` (deferred to the interface spec, no counter). This matches Tasks 1/3/4/7.
