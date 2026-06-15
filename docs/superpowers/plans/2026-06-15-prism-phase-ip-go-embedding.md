# Go Struct Embedding Method Promotion — Implementation Plan (rev 2)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Resolve Go embedded-struct method calls (`w.Ping()` where `Wrap` embeds `Base`) by promoting the embedded type's concrete methods into the outer type's owner index, so the existing P6-lite seam resolves them as `Exact`.

**Architecture:** `CallGraph::build` consumes the existing (registry-independent) `GoTypeProvider` to read promoted concrete methods, then writes owner-index aliases `methods[(Wrap,Ping)] += Base::Ping` (the trait dual-key pattern) plus a `promoted_aliases` map (for telemetry + incremental replace). The `EmbeddedPromotion` relabel happens centrally in `owner_lookup`, so it is correct on every path (P6-lite, self/receiver-var, qualifier). Promotion is whole-program and recomputed (replace-not-merge) on incremental builds. Keys use the existing `owner_key` (bare name); generic + cross-package structs consistently **gap** (no false match). Confidence is `Exact` (deterministic Go rule, single target).

**Tech Stack:** Rust, tree-sitter (Go), `bincode` CPG cache, Python `uv` Tier-A harness. **Spec:** `docs/superpowers/specs/2026-06-15-prism-phase-ip-go-embedding-design.md`. **Branch:** `phase-ip`.

**rev 2 (plan-review fold, codex+claude 2026-06-15):** path-local `visited` (diamond ambiguity, codex); field-shadow filtering (codex); relabel in `owner_lookup` not the seam (codex); cross-file stale-alias test (codex); **generics gap via `owner_key`** — dropped the rev-1 `normalize_go_struct_key` task (codex/claude MAJOR3, lean resolution); Task-4 line `6`→`7` + `cpg.call_graph` field (both); `embedding_gaps["ambiguous"]==1` assertion (codex); `set -o pipefail`/subshell validation (codex); Accuracy-Harness gate before resolution/CPG/nav commits (codex, CLAUDE.md). build_scoped stays best-effort (claude; out-of-scope target nodes can't materialize regardless, nav builds full).

---

## File Structure

- **`src/type_providers/go.rs`** — public `PromotedMethod` + `GoTypeProvider::promoted_struct_methods()` (transitive embedded-**struct** promotion, **path-local** cycle detection, embedded-interface skip, **field-shadow** skip, depth). Internal `#[cfg(test)] mod`.
- **`src/resolution.rs`** — `ResolutionKind::EmbeddedPromotion` + `as_str`; relabel inside `owner_lookup`.
- **`src/call_graph.rs`** — `promoted_aliases` + `embedding_gaps` fields (init in all 4 constructors); `apply_go_embedding_promotion`; call at end of `build()`.
- **`src/cpg/build.rs`** — `apply_go_embedding_promotion(files)` after `merge` in `build_incremental`.
- **`src/cpg_cache.rs`** — `CACHE_VERSION` 7→8.
- **`src/navigation/queries.rs`** — `embedding_gaps` in `call_stats`.
- **`eval/fixtures/go/embedded_method/expected.toml`** — `known_fail`→`pass`.
- **Tests:** `tests/integration/resolution_test.rs`, `tests/cli/call_stats_test.rs`.

**No `normalize_go_struct_key`** (rev-1 had it): the existing `owner_key` (resolution.rs:75) is used for the alias struct key, and `recover_receiver` already `owner_key`s the receiver — so non-generic, non-`pkg.` structs match and generic/`pkg.` structs gap, with **one** key function. **No seam edit, no `recover_receiver` edit** (rev-1 had both): the relabel lives in `owner_lookup`, the chokepoint every hit path goes through.

**FunctionId identity:** `extract_method` builds `start_line/end_line = row+1` (go.rs:399-400) = `node_line_range` (call_graph.rs:250), so a `PromotedMethod.func_id` equals the CallGraph function node — edges materialize.

---

## Task 1: `promoted_struct_methods` provider helper

**Files:** Modify `src/type_providers/go.rs` (add `PromotedMethod` ~line 70; method after `collect_promoted_methods_from` ~line 549; internal test mod at end).

- [ ] **Step 1: Write the failing test** — append to `src/type_providers/go.rs`:

```rust
#[cfg(test)]
mod embedding_tests {
    use super::*;
    use std::collections::BTreeMap;

    fn provider(src: &str) -> GoTypeProvider {
        let mut files = BTreeMap::new();
        files.insert(
            "main.go".to_string(),
            crate::ast::ParsedFile::parse("main.go", src, Language::Go).unwrap(),
        );
        GoTypeProvider::from_parsed_files(&files)
    }

    fn ms<'a>(v: &'a [PromotedMethod], s: &str, m: &str) -> Vec<&'a PromotedMethod> {
        v.iter().filter(|p| p.struct_name == s && p.method == m).collect()
    }

    #[test]
    fn promotes_concrete_method_from_embedded_struct() {
        let v = provider("package main\ntype Base struct{}\nfunc (b Base) Ping() {}\ntype Wrap struct {\n\tBase\n}\n").promoted_struct_methods();
        let p = ms(&v, "Wrap", "Ping");
        assert_eq!(p.len(), 1);
        assert_eq!(p[0].func_id.name, "Ping");
        assert_eq!(p[0].func_id.file, "main.go");
        assert_eq!(p[0].depth, 1);
    }

    #[test]
    fn promotes_transitively_with_depth() {
        let v = provider("package main\ntype C struct{}\nfunc (c C) M() {}\ntype B struct{ C }\ntype A struct{ B }\n").promoted_struct_methods();
        let p = ms(&v, "A", "M");
        assert_eq!(p.len(), 1);
        assert_eq!(p[0].depth, 2);
    }

    #[test]
    fn diamond_returns_both_equal_depth_paths() {
        // A embeds B,C; B,C embed D; D.M reachable via two depth-2 paths.
        // Path-local visited must NOT suppress the second path (so CallGraph sees the ambiguity).
        let v = provider("package main\ntype D struct{}\nfunc (d D) M() {}\ntype B struct{ D }\ntype C struct{ D }\ntype A struct{\n\tB\n\tC\n}\n").promoted_struct_methods();
        let p = ms(&v, "A", "M");
        assert_eq!(p.len(), 2, "both depth-2 paths to D.M returned");
        assert!(p.iter().all(|m| m.depth == 2));
    }

    #[test]
    fn embedded_interface_not_promoted() {
        let v = provider("package main\ntype R interface { Read() }\ntype S struct {\n\tR\n}\n").promoted_struct_methods();
        assert!(ms(&v, "S", "Read").is_empty());
    }

    #[test]
    fn direct_field_shadows_promoted_method() {
        // Wrap has a field named Ping -> the embedded Base.Ping is shadowed, not promoted.
        let v = provider("package main\ntype Base struct{}\nfunc (b Base) Ping() {}\ntype Wrap struct {\n\tBase\n\tPing int\n}\n").promoted_struct_methods();
        assert!(ms(&v, "Wrap", "Ping").is_empty(), "a direct field named Ping shadows the promoted method");
    }

    #[test]
    fn intermediate_embedded_field_shadows_deeper_method() {
        // A embeds B; B has field M AND embeds D; D has method M. Go selector rules:
        // B's field M (depth 1) shadows D.M (depth 2) -> NOT promoted to A.
        let v = provider("package main\ntype D struct{}\nfunc (d D) M() {}\ntype B struct {\n\tD\n\tM int\n}\ntype A struct{ B }\n").promoted_struct_methods();
        assert!(ms(&v, "A", "M").is_empty(), "intermediate field M shadows the deeper promoted method");
    }
}
```

- [ ] **Step 2: Run to verify it fails** — `cargo test --lib type_providers::go::embedding_tests`
Expected: FAIL (`cannot find type PromotedMethod` / `no method promoted_struct_methods`).

- [ ] **Step 3: Implement.** Add `PromotedMethod` after `GoMethod` (~line 70):

```rust
/// A concrete method promoted onto an outer struct via embedding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PromotedMethod {
    pub struct_name: String,
    pub method: String,
    pub func_id: FunctionId,
    pub depth: usize,
}
```

Add inside `impl GoTypeProvider` (after `collect_promoted_methods_from`, ~line 549):

```rust
/// All concrete methods promoted onto each struct via transitive **struct**
/// embedding, with depth. One walk per struct collects (a) method candidates at
/// depth>=1 and (b) the shallowest depth of every field name (incl. the outer
/// struct's own fields at depth 0). A method at depth `d` is emitted only if NO
/// same-name field exists at depth `<= d` — the Go selector rule, which lets an
/// intermediate embedded struct's field shadow a deeper promoted method. Embedded
/// **interface** fields are skipped (no concrete body — interface dispatch,
/// deferred). Duplicates from different embed paths are kept (the caller resolves
/// direct-method-wins + equal-depth ambiguity). **Path-local** cycle detection so
/// diamond paths are preserved.
pub fn promoted_struct_methods(&self) -> Vec<PromotedMethod> {
    let mut out = Vec::new();
    for struct_name in self.data.structs.keys() {
        let mut field_depth: BTreeMap<String, usize> = BTreeMap::new();
        let mut cands: Vec<PromotedMethod> = Vec::new();
        let mut path: BTreeSet<String> = BTreeSet::new();
        path.insert(struct_name.clone());
        Self::walk_embedding(
            &self.data,
            struct_name,
            struct_name,
            0,
            &mut path,
            &mut field_depth,
            &mut cands,
        );
        for pm in cands {
            // Shadowed if a same-name field sits at a shallower-or-equal depth.
            match field_depth.get(&pm.method) {
                Some(fd) if *fd <= pm.depth => {}
                _ => out.push(pm),
            }
        }
    }
    out
}

#[allow(clippy::too_many_arguments)]
fn walk_embedding(
    data: &GoTypeData,
    outer: &str,
    current: &str,
    depth: usize, // depth of `current` within `outer` (0 = the outer struct itself)
    path: &mut BTreeSet<String>,
    field_depth: &mut BTreeMap<String, usize>,
    cands: &mut Vec<PromotedMethod>,
) {
    let go_struct = match data.structs.get(current) {
        Some(s) => s,
        None => return,
    };
    // Record this struct's own (non-embedded) field names at `depth` (keep the min).
    for (fname, _) in &go_struct.fields {
        if go_struct.embedded.contains(fname) {
            continue; // an embedded-as-field entry, not a real shadowing field
        }
        field_depth
            .entry(fname.clone())
            .and_modify(|d| {
                if depth < *d {
                    *d = depth;
                }
            })
            .or_insert(depth);
    }
    // Methods of `current` promote to `outer` (depth>=1; the outer struct's own
    // depth-0 methods are NOT promoted — direct-method-wins is the caller's job).
    if depth >= 1 {
        if let Some(methods) = data.methods.get(current) {
            for m in methods {
                cands.push(PromotedMethod {
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
    }
    for embedded_name in &go_struct.embedded {
        let bare = strip_pointer(embedded_name);
        if data.interfaces.contains_key(bare) {
            continue; // embedded interface -> interface dispatch, deferred
        }
        if !path.insert(bare.to_string()) {
            continue; // cycle along THIS path only
        }
        Self::walk_embedding(data, outer, bare, depth + 1, path, field_depth, cands);
        path.remove(bare); // restore for sibling paths (path-local)
    }
}
```

- [ ] **Step 4: Run to verify it passes** — `cargo test --lib type_providers::go::embedding_tests`
Expected: PASS (6 tests).

- [ ] **Step 5: Commit**

```bash
git add src/type_providers/go.rs
git commit -m "feat(go): promoted_struct_methods — transitive embedded-struct promotion (path-local, field-shadow, interface-skip)"
```

(Task 1 touches only `type_providers/go.rs` — not call resolution/CPG/nav — so no Accuracy-Harness gate yet.)

---

## Task 2: `ResolutionKind::EmbeddedPromotion`

**Files:** Modify `src/resolution.rs:16-54`.

- [ ] **Step 1: Failing test** — append to `src/resolution.rs`:

```rust
#[cfg(test)]
mod embedding_kind_tests {
    use super::ResolutionKind;
    #[test]
    fn embedded_promotion_as_str() {
        assert_eq!(ResolutionKind::EmbeddedPromotion.as_str(), "embedded_promotion");
    }
}
```

- [ ] **Step 2: Verify fails** — `cargo test --lib resolution::embedding_kind_tests`
Expected: FAIL (`no variant EmbeddedPromotion`).

- [ ] **Step 3: Implement.** Add the variant after `StemMulti` (line 32) and the arm after the `StemMulti` arm (line 52):

```rust
    StemMulti,
    EmbeddedPromotion,
}
```
```rust
            ResolutionKind::StemMulti => "stem_multi",
            ResolutionKind::EmbeddedPromotion => "embedded_promotion",
        }
```

- [ ] **Step 4: Verify passes** — `cargo test --lib resolution::embedding_kind_tests`
Expected: PASS. The only exhaustive `match` over `ResolutionKind` is `as_str` (verified, no other ripple).

- [ ] **Step 5: Commit**

```bash
git add src/resolution.rs
git commit -m "feat(resolution): add EmbeddedPromotion resolution kind"
```

(Pure enum addition, no resolution-behavior change → no harness gate.)

---

## Task 3: CallGraph promotion + `owner_lookup` relabel (the integration)

**Files:** `src/call_graph.rs` (struct 47-71, `empty` 75-86, `build_skeleton` ~195, `build` ~693, `build_direct_subset` ~909, add `apply_go_embedding_promotion` near `merge` ~770), `src/resolution.rs` (`owner_lookup` 199-204), `tests/integration/resolution_test.rs`.

- [ ] **Step 1: Failing tests** — append to `tests/integration/resolution_test.rs`:

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
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].target.name, "Ping");
    assert_eq!(r[0].target.file, "main.go");
    assert_eq!(r[0].confidence, ResolutionConfidence::Exact);
    assert_eq!(r[0].kind, ResolutionKind::EmbeddedPromotion);
}

#[test]
fn go_embedded_transitive_resolves() {
    use prism::languages::Language::Go;
    let (cg, _) = build(&[(
        "main.go",
        "package main\ntype C struct{}\nfunc (c C) M() {}\ntype B struct{ C }\ntype A struct{ B }\nfunc run(a A) {\n\ta.M()\n}\n",
        Go,
    )]);
    let r = cg.resolve_call_site(&site_in(&cg, "run", "M"));
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].kind, ResolutionKind::EmbeddedPromotion);
}

#[test]
fn go_embedded_pointer_receiver_addressable_resolves() {
    use prism::languages::Language::Go;
    let (cg, _) = build(&[(
        "main.go",
        "package main\ntype Base struct{}\nfunc (b *Base) Ping() {}\ntype Wrap struct {\n\tBase\n}\nfunc run(w Wrap) {\n\tw.Ping()\n}\n",
        Go,
    )]);
    let r = cg.resolve_call_site(&site_in(&cg, "run", "Ping"));
    assert_eq!(r.len(), 1, "addressable value receiver can call a pointer-receiver promoted method");
    assert_eq!(r[0].kind, ResolutionKind::EmbeddedPromotion);
}

#[test]
fn go_embedded_method_labeled_on_receiver_var_path() {
    use prism::languages::Language::Go;
    // The call is via the method receiver `w` (self/receiver-var path, not P6-lite param).
    let (cg, _) = build(&[(
        "main.go",
        "package main\ntype Base struct{}\nfunc (b Base) Ping() {}\ntype Wrap struct {\n\tBase\n}\nfunc (w Wrap) Run() {\n\tw.Ping()\n}\n",
        Go,
    )]);
    let r = cg.resolve_call_site(&site_in(&cg, "Run", "Ping"));
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].kind, ResolutionKind::EmbeddedPromotion, "relabel must apply on the receiver-var path too");
}

#[test]
fn go_direct_method_wins_over_promoted() {
    use prism::languages::Language::Go;
    let (cg, _) = build(&[(
        "main.go",
        "package main\ntype Base struct{}\nfunc (b Base) Ping() {}\ntype Wrap struct {\n\tBase\n}\nfunc (w Wrap) Ping() {}\nfunc run(w Wrap) {\n\tw.Ping()\n}\n",
        Go,
    )]);
    let r = cg.resolve_call_site(&site_in(&cg, "run", "Ping"));
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].target.start_line, 7, "direct Wrap.Ping (line 7) wins");
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
    let out = cg.resolve_call_site_full(&site_in(&cg, "run", "M"));
    assert!(out.resolved.is_empty(), "equal-depth M is ambiguous -> not promoted");
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
    assert!(cg.resolve_call_site_full(&site_in(&cg, "run", "Read")).resolved.is_empty());
}
```

- [ ] **Step 2: Verify fails** — `cargo test --test integration resolution_test::go_embedded`
Expected: FAIL (`go_embedded_method_resolves_exact` resolves to 0 — current `ExternalReceiver` drop).

- [ ] **Step 3a: CallGraph fields.** In `src/call_graph.rs` after `receiver_vars` (line 70):

```rust
    #[serde(default)]
    pub receiver_vars: BTreeMap<FunctionId, String>,
    /// Phase-IP (Go embedding): promoted alias `(owner_key, method)` → embedded
    /// methods' FunctionIds. Key set is the EmbeddedPromotion label set; carries
    /// fids for clean incremental replace.
    #[serde(default)]
    pub promoted_aliases: BTreeMap<(String, String), Vec<FunctionId>>,
    /// Phase-IP (Go embedding): gap telemetry, e.g. {"ambiguous": n}.
    #[serde(default)]
    pub embedding_gaps: BTreeMap<String, usize>,
}
```

Initialize both (empty) in **all four** constructors: `empty()` (after `receiver_vars: BTreeMap::new(),`), `build_skeleton`’s returned literal, `build`’s returned literal (see 3c), and `build_direct_subset`’s returned literal. Each gets:

```rust
            promoted_aliases: BTreeMap::new(),
            embedding_gaps: BTreeMap::new(),
```

- [ ] **Step 3b: `apply_go_embedding_promotion`.** Add to `impl CallGraph` near `merge` (~line 770):

```rust
/// Recompute Go embedding promotions over `files` and write owner-index aliases.
/// Idempotent: clears prior aliases first (incremental replace).
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
    if !files.values().any(|p| p.language == crate::languages::Language::Go) {
        return;
    }
    // 2. Group promotions by (owner_key(struct), method).
    let provider = crate::type_providers::go::GoTypeProvider::from_parsed_files(files);
    let mut by_key: BTreeMap<(String, String), Vec<(usize, FunctionId)>> = BTreeMap::new();
    for pm in provider.promoted_struct_methods() {
        let key = (crate::resolution::owner_key(&pm.struct_name), pm.method);
        by_key.entry(key).or_default().push((pm.depth, pm.func_id));
    }
    // 3. Direct-method-wins, then uniquely-shallowest else ambiguous-drop.
    let mut ambiguous = 0usize;
    for ((owner, method), mut cands) in by_key {
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
        let shallow: Vec<FunctionId> =
            cands.iter().filter(|(d, _)| *d == min_depth).map(|(_, f)| f.clone()).collect();
        if shallow.len() > 1 {
            ambiguous += 1;
            continue;
        }
        let fid = shallow.into_iter().next().unwrap();
        self.methods.entry((owner.clone(), method.clone())).or_default().push(fid.clone());
        self.promoted_aliases.entry((owner, method)).or_default().push(fid);
    }
    if ambiguous > 0 {
        self.embedding_gaps.insert("ambiguous".to_string(), ambiguous);
    }
}
```

- [ ] **Step 3c: Call from `build()`.** `CallGraph::build` ends with a `CallGraph { … }` literal (~line 693). Bind it and apply (the `files` param is in scope):

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

- [ ] **Step 3d: Relabel in `owner_lookup`.** In `src/resolution.rs`, `owner_lookup` (199-204) delegates to `owner_lookup_in_modules`. Wrap the result so every hit path (P6-lite, self/receiver-var, qualifier) gets the label:

```rust
    fn owner_lookup(&self, owner: &str, name: &str) -> Option<Vec<ResolvedCallee<'_>>> {
        let mut resolved = self.owner_lookup_in_modules(owner, name, &[])?;
        if self
            .promoted_aliases
            .contains_key(&(owner.to_string(), name.to_string()))
        {
            for c in &mut resolved {
                c.kind = ResolutionKind::EmbeddedPromotion;
            }
        }
        Some(resolved)
    }
```

(No seam edit and no `recover_receiver` edit: the alias key is `owner_key(struct)` and `recover_receiver` already `owner_key`s the receiver, so they match for non-generic/non-`pkg.` structs; the P6-lite seam’s `if kind == QualifiedOwner` branch leaves an already-`EmbeddedPromotion` kind untouched.)

- [ ] **Step 4: Verify passes** — `cargo test --test integration resolution_test::go_embedded` then the direct/equal-depth tests, then the whole resolution suite:

```bash
cargo test --test integration resolution_test::go_embedded
cargo test --test integration resolution_test::go_direct_method_wins_over_promoted
cargo test --test integration resolution_test::go_equal_depth_embedding_ambiguity_drops
cargo test --test integration resolution_test::
```
Expected: all PASS (no regressions in the existing resolution tests).

- [ ] **Step 5: Accuracy-Harness gate (CLAUDE.md — this touches call resolution) then commit**

```bash
cargo build --release && (cd eval && uv run tier-a --matrix-only --allow-stale-sut)
```
Expected: exit 0; `go/embedded_method` reports `flip_candidate` (resolves now; fixture still `known_fail` until Task 7); **no** `regression` lines. Then:

```bash
git add src/call_graph.rs src/resolution.rs tests/integration/resolution_test.rs
git commit -m "feat(go): promote embedded-struct methods into the owner index (Exact, direct-wins, equal-depth drop, owner_lookup-labeled)"
```

---

## Task 4: Replace-not-merge on incremental builds (cross-file)

**Files:** `src/cpg/build.rs:185-191`; `tests/integration/resolution_test.rs`.

- [ ] **Step 1: Failing test (cross-file — the real hazard).** Append:

```rust
#[test]
fn go_embedding_dropped_on_incremental_when_embedding_file_changes() {
    use prism::cpg::CodePropertyGraph;
    use prism::data_flow::DataFlowGraph;
    use prism::languages::Language::Go;
    use std::collections::{BTreeMap, BTreeSet};

    // Base.Ping in base.go (UNCHANGED); Wrap embeds Base in wrap.go.
    let parse = |p: &str, s: &str| (p.to_string(), prism::ast::ParsedFile::parse(p, s, Go).unwrap());
    let mut v1 = BTreeMap::new();
    v1.extend([
        parse("base.go", "package p\ntype Base struct{}\nfunc (b Base) Ping() {}\n"),
        parse("wrap.go", "package p\ntype Wrap struct {\n\tBase\n}\n"),
    ]);
    let cg_v1 = prism::call_graph::CallGraph::build(&v1);
    assert!(cg_v1.promoted_aliases.contains_key(&("Wrap".to_string(), "Ping".to_string())));

    // v2: wrap.go removes the embedding (base.go's fid file is UNCHANGED -> remove_files won't prune it).
    let mut v2 = BTreeMap::new();
    v2.extend([
        parse("base.go", "package p\ntype Base struct{}\nfunc (b Base) Ping() {}\n"),
        parse("wrap.go", "package p\ntype Wrap struct{}\n"),
    ]);
    let changed: BTreeSet<String> = ["wrap.go".to_string()].into_iter().collect();
    let dfg = DataFlowGraph::build(&v2);
    let cpg = CodePropertyGraph::build_incremental(cg_v1, dfg, &changed, &v2, None);
    assert!(
        !cpg.call_graph.promoted_aliases.contains_key(&("Wrap".to_string(), "Ping".to_string())),
        "stale promoted alias must be cleared even though Base.Ping's file is unchanged"
    );
}
```

- [ ] **Step 2: Verify fails** — `cargo test --test integration resolution_test::go_embedding_dropped_on_incremental`
Expected: FAIL — the stale `Wrap→Ping` alias survives (`merge` carried `methods`; `remove_files` prunes by `fid.file`=base.go which didn't change).

- [ ] **Step 3: Implement.** In `src/cpg/build.rs build_incremental`, after the merge (line 187), before `assemble_graph` (line 191):

```rust
        cached_cg.merge(fresh_cg);
        cached_dfg.merge(fresh_dfg);

        // Phase-IP: Go embedding promotion is whole-program — recompute (replace-
        // not-merge) over ALL merged files so a removed/changed embedding cannot
        // leave a stale alias (remove_files prunes methods by fid.file only).
        cached_cg.apply_go_embedding_promotion(files);

        Self::assemble_graph(cached_cg, cached_dfg, files, type_db)
```

- [ ] **Step 4: Verify passes** — `cargo test --test integration resolution_test::go_embedding_dropped_on_incremental`
Expected: PASS.

- [ ] **Step 5: Harness gate (touches CPG) + commit**

```bash
cargo build --release && (cd eval && uv run tier-a --matrix-only --allow-stale-sut)
```
Expected: exit 0; no `regression`. Then:

```bash
git add src/cpg/build.rs tests/integration/resolution_test.rs
git commit -m "fix(go): recompute embedding promotion on incremental rebuild (replace-not-merge)"
```

---

## Task 5: Bump `CACHE_VERSION` to 8

**Files:** `src/cpg_cache.rs:44-47`.

- [ ] **Step 1: Failing test** — add to `src/cpg_cache.rs` `#[cfg(test)] mod tests`:

```rust
    #[test]
    fn cache_version_is_8_for_embedding_fields() {
        assert_eq!(super::CACHE_VERSION, 8);
    }
```

- [ ] **Step 2: Verify fails** — `cargo test --lib cpg_cache::tests::cache_version_is_8`
Expected: FAIL (7 != 8).

- [ ] **Step 3: Implement** (lines 44-47):

```rust
/// - v6: EFT CpgEdge::Call/Return carry ResolutionConfidence.
/// - v7: + git_sha in cache key (resolver auto-invalidation).
/// - v8: Phase-IP CallGraph.promoted_aliases + embedding_gaps (Go embedding).
const CACHE_VERSION: u32 = 8; // bincode ignores serde(default) for new trailing fields.
```

- [ ] **Step 4: Verify passes** — `cargo test --lib cpg_cache::tests::cache_version_is_8`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/cpg_cache.rs
git commit -m "chore(cache): bump CACHE_VERSION 7->8 for Go embedding CallGraph fields"
```

---

## Task 6: `call-stats` embedding telemetry + ambiguity assertion

**Files:** `src/navigation/queries.rs:38-46`; `tests/cli/call_stats_test.rs`.

- [ ] **Step 1: Failing test** — append to `tests/cli/call_stats_test.rs`:

```rust
#[test]
fn call_stats_reports_embedded_promotion_and_ambiguity() {
    let dir = tempfile::tempdir().unwrap();
    // One resolved promotion (Wrap.Ping) + one equal-depth ambiguity (A.M via X,Y).
    std::fs::write(
        dir.path().join("main.go"),
        "package main\ntype Base struct{}\nfunc (b Base) Ping() {}\ntype Wrap struct {\n\tBase\n}\nfunc run(w Wrap) {\n\tw.Ping()\n}\ntype X struct{}\nfunc (x X) M() {}\ntype Y struct{}\nfunc (y Y) M() {}\ntype A struct {\n\tX\n\tY\n}\n",
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
    assert_eq!(v["embedding_gaps"]["ambiguous"], 1);
}
```

- [ ] **Step 2: Verify fails** — `cargo test --test cli call_stats_test::call_stats_reports_embedded_promotion_and_ambiguity`
Expected: FAIL — `embedding_gaps` absent (`null`). (`kinds.embedded_promotion` is already populated automatically since `call_stats` keys off `c.kind.as_str()`, queries.rs:31.)

- [ ] **Step 3: Implement.** Add `embedding_gaps` to the `call_stats` JSON (queries.rs:38-46):

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

- [ ] **Step 4: Verify passes** — `cargo test --test cli call_stats_test::call_stats_reports_embedded_promotion_and_ambiguity`
Expected: PASS.

- [ ] **Step 5: Harness gate (touches navigation) + commit**

```bash
cargo build --release && (cd eval && uv run tier-a --matrix-only --allow-stale-sut)
```
Expected: exit 0; no `regression`. Then:

```bash
git add src/navigation/queries.rs tests/cli/call_stats_test.rs
git commit -m "feat(nav): surface embedding_gaps in call-stats; assert EmbeddedPromotion + ambiguous counter"
```

---

## Task 7: Flip the capability fixture

**Files:** `eval/fixtures/go/embedded_method/expected.toml`.

- [ ] **Step 1: Confirm flip-candidate (pre-change)**

```bash
cargo build --release && (cd eval && uv run tier-a --matrix-only --allow-stale-sut) | grep -i embedded_method
```
Expected line contains `flip_candidate`. (The `grep` is for display only here — the build/run ran in the subshell with its own exit; if unsure, run the subshell alone and read the full output.) If it still says `expected_gap`, Task 3 is incomplete — stop.

- [ ] **Step 2: Flip the fixture.** Edit `eval/fixtures/go/embedded_method/expected.toml`:

```toml
[case]
language = "go"
capability = "embedded_method"
# Phase-IP (2026-06-15): RESOLVED. CallGraph::build promotes the embedded Base.Ping
# into Wrap's owner index (apply_go_embedding_promotion); owner_lookup hits and
# labels it EmbeddedPromotion (Exact). See the Go embedding spec.
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

- [ ] **Step 3: Verify pass + no regression**

```bash
cargo build --release && (cd eval && uv run tier-a --matrix-only --allow-stale-sut)
```
Read the full output: `embedded_method` → `ok`; `interface_dispatch` still `expected_gap`; **no** `regression`.

- [ ] **Step 4: Commit**

```bash
git add eval/fixtures/go/embedded_method/expected.toml
git commit -m "test(eval): flip go/embedded_method known_fail -> pass (Phase-IP embedding)"
```

---

## Task 8: Full repo validation

**Files:** none.

- [ ] **Step 1: Format** — `cargo fmt` then `cargo fmt --check` (expect exit 0).

- [ ] **Step 2: Full suite (pipefail so failures aren't masked)**

```bash
set -o pipefail
cargo test 2>&1 | tail -40
cargo test --features mcp 2>&1 | tail -20
```
Expected: both exit 0 (pipefail propagates a test failure through `tail`).

- [ ] **Step 3: Tier-A matrix + quick**

```bash
cargo build --release && (cd eval && uv run tier-a --matrix-only --allow-stale-sut)
cargo build --release && (cd eval && uv run tier-a --quick --allow-stale-sut)
```
Expected: exit 0; `go/embedded_method` `ok`; no `regression`. Paste any regression/flip-candidate lines into the PR (do **not** re-baseline — full-corpus runs are human-triggered, spec §11).

- [ ] **Step 4: Commit any fmt-only changes**

```bash
git add -A && git commit -m "chore: cargo fmt" || echo "nothing to format"
```

---

## Self-Review

**Spec coverage:** §4 promotion → Tasks 1,3. §5 receiver/addressability → Task 3 (`go_embedded_pointer_receiver_addressable_resolves`). §6 keys (owner_key; generics/pkg gap) → Task 3 (alias key via `owner_key`; no normalize). §7 gap (ambiguity counter; interface skip; generics gap) → Tasks 1 (skip/shadow), 3 (ambiguous), 6 (surface). §8 failure modes → Tasks 1,3 tests. §9 cache v8 + replace-not-merge (build/incremental; scoped best-effort, not modified) → Tasks 4,5. §10 tests + matrix → Tasks 3,4,6,7,8. §11 human-triggered → Task 8 note.

**Placeholder scan:** none — every step has complete code/commands.

**Type consistency:** `PromotedMethod{struct_name,method,func_id,depth}` (Task 1) consumed in Task 3. `promoted_aliases: BTreeMap<(String,String),Vec<FunctionId>>` + `embedding_gaps: BTreeMap<String,usize>` (Task 3) read in Task 4 (`promoted_aliases`), Task 6 (`embedding_gaps`). `EmbeddedPromotion` (Task 2) set in `owner_lookup` (Task 3), read in Task 6 via `as_str`. `owner_key` (existing) used for the alias key; no `normalize_go_struct_key`.

**Spec edits applied alongside this rev2:** §3/§6/§7/§8/§12 reverted to **generics gap** (drop the normalize/[…]-strip claim); §9 strike the build_scoped "Requires threading full files" clause (best-effort is the chosen reading); §10 the key-normalization test targets `*Wrap` + a generic-receiver **gap** assertion (not `pkg.Wrap`, which is deferred).
