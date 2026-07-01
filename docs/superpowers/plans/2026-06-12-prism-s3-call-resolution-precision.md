# S3 Call-Resolution Precision Floor — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Rebuild prism's shared call resolver around a method-owner index so collision-method caller claims disappear (tokio C-method P=0.00 → gated ≤20 FPs) and qualified `Type::fn` calls resolve (callee recall 0.70 class closed), per spec rev 2.1.

**Architecture:** One new fact (`FunctionInfo.owner` + Go `receiver_var`) extracted per language; a `(owner, name)` method index in `CallGraph` Phase 1; the resolver becomes an ordered R1–R7 ladder returning `(FunctionId, ResolutionConfidence)`; P6-lite receiver-type recovery runs at **extraction time** (Phase 2 has the `ParsedFile`s) and stores `CallSite.receiver_type`; CPG includes Exact+NameOnly edges and excludes drops; navigation maps confidence to scores (1.0/0.6) with `Reason::Resolution` and `Collision` warnings.

**Tech Stack:** Rust, tree-sitter, petgraph, serde/bincode (CPG cache), the Tier-A eval harness (`eval/`, uv Python) for acceptance.

**Spec:** `docs/superpowers/specs/2026-06-12-prism-s3-call-resolution-precision-design.md` (rev 2.1, owner-approved). Spec review record: `docs/archive/review-artifacts/prism-query-layer/s3-spec-review-2026-06-12.md`.

**Plan rev 2** — codex xhigh dual-lens plan review folded (record:
`docs/archive/review-artifacts/prism-query-layer/s3-plan-review-2026-06-12.md`; all 5 BLOCKER + 8 MAJOR +
4 MINOR findings fixed in-plan: incremental `build_direct_subset` recovery parity,
`ResolutionOutcome`/`DropReason` classification, `ReceiverRecovery` carried on
`CallSite`, call-line-ordered shadow bail, both traversal branches per-site, 5b
param path via `FunctionInfo.param_names`, nav migration enumerated with
`NavCallEdge`, ego warnings, module-graph reason struct, telemetry split, fixture
empty-callers schema, JS class-field arrows).

**Branch:** create `s3-precision` from `main` before Task 1 (`git checkout -b s3-precision`).

---

## File structure (what changes where)

| File | Responsibility in S3 |
|---|---|
| `src/languages/mod.rs` | NEW `method_owner()` + `rust_impl_trait()` + `go_receiver_var()` node accessors; Lua `function_name` keying fix |
| `src/ast.rs` | `FunctionInfo.owner`/`receiver_var` fields; `build_function_table` populates them; NEW `receiver_type_in_fn()` (P6-lite scan, Rust+Go); NEW `function_node_spanning()` helper |
| `src/resolution.rs` | **NEW module**: `ResolutionConfidence`, `ResolutionKind`, `ResolvedCallee`, `ReceiverRecovery`, `DropReason`, `ResolutionOutcome`, `owner_key()`, `peel_type()`, and `impl CallGraph { resolve_call_site, resolve_call_site_full }` (the R1–R7 ladder; `_full` carries the drop classification) |
| `src/call_graph.rs` | `CallSite.receiver_type` + `receiver_recovery` fields; `CallGraph.methods` index + `method_owners`/`receiver_vars` side maps; maintenance in `empty/build/build_skeleton/build_direct_subset/remove_files/merge`; Phase 2 P6-lite recovery in `build` **and `build_direct_subset`** (incremental updates must match full builds); 4 traversal helpers switch to `resolve_call_site` |
| `src/cpg/build.rs` | Step 5/5b consume `resolve_call_site` (Exact+NameOnly in, drops out); Step 5b Python `self`/`cls` param skip |
| `src/cpg/context.rs` | `compute_scope` pinned to recall-biased `resolve_callees` (name-only) — comment contract only, no behavior change |
| `src/cpg_cache.rs` | `CACHE_VERSION` 3 → 4 |
| `src/navigation/call_resolve.rs` | Shrinks to an adapter over `resolve_call_site` (stem logic moves to ladder R7) |
| `src/navigation/queries.rs` | Scores Exact=1.0 / NameOnly=0.6 × hop decay; `Reason::Resolution`; `Collision` warning on callers/ego |
| `src/navigation/module_graph.rs` | Per-file-pair max-score aggregation |
| `src/navigation/types.rs` | `Reason::Resolution { kind }` variant |
| `src/main.rs` | NEW `nav call-stats` subcommand (R6 telemetry for the PR) |
| `tests/ast/owner_test.rs` | NEW — owner/receiver extraction per language (+ `mod owner_test;` in `tests/ast/main.rs`) |
| `tests/integration/resolution_test.rs` | NEW — R1–R7 ladder tests (+ `mod resolution_test;` in `tests/integration/main.rs`) |
| `tests/navigation/{callers,callees,module_graph}_test.rs` | score/Reason/warning assertions added |
| `eval/fixtures/{rust,go}/...` | matrix statuses reconciled; missing R6-policy fixtures added |
| `CLAUDE.md` | nav resolution-coverage paragraph updated post-acceptance |

Conventions that bind every task: 1-indexed lines; BTreeMap/BTreeSet for determinism; files stay under ~600 lines (that's why `src/resolution.rs` is a new module); every new test file needs its `mod <stem>;` line or `tests/integration/umbrella_completeness_test.rs` fails; run `cargo fmt` before every commit.

**Harness duty (CLAUDE.md):** after any commit touching `src/call_graph.rs`, `src/navigation/`, `src/cpg/`, `src/ast.rs`: `cargo build --release && cd eval && uv run tier-a --matrix-only --allow-stale-sut`. Expected during mid-plan tasks: pre-existing statuses hold (the flip lands in Task 13). If a capability regresses ok→fail, stop and fix before committing.

---

### Task 1: Rust owner extraction (`method_owner`, `FunctionInfo.owner`)

**Files:**
- Modify: `src/languages/mod.rs` (new methods near `function_name`, ~line 870)
- Modify: `src/ast.rs:49-57` (FunctionInfo), `src/ast.rs:245-261` (build_function_table)
- Create: `tests/ast/owner_test.rs`; Modify: `tests/ast/main.rs` (add `mod owner_test;`)

- [ ] **Step 1: Write failing tests**

```rust
// tests/ast/owner_test.rs
use prism::ast::ParsedFile;
use prism::languages::Language;

fn parse(src: &str, lang: Language, path: &str) -> ParsedFile {
    ParsedFile::parse(path, src, lang).unwrap()
}

fn owners(pf: &ParsedFile) -> Vec<(Option<String>, Option<String>)> {
    pf.functions()
        .iter()
        .map(|f| (f.name.clone(), f.owner.clone()))
        .collect()
}

#[test]
fn rust_inherent_impl_method_has_owner() {
    let pf = parse(
        "struct Foo;\nimpl Foo {\n    fn m(&self) {}\n}\nfn free() {}\n",
        Language::Rust,
        "a.rs",
    );
    let o = owners(&pf);
    assert!(o.contains(&(Some("m".into()), Some("Foo".into()))));
    assert!(o.contains(&(Some("free".into()), None)));
}

#[test]
fn rust_generic_impl_owner_strips_generics() {
    let pf = parse(
        "impl<T> Wrapper<T> {\n    fn get(&self) {}\n}\n",
        Language::Rust,
        "a.rs",
    );
    assert!(owners(&pf).contains(&(Some("get".into()), Some("Wrapper".into()))));
}

#[test]
fn rust_trait_impl_owner_is_type_not_trait() {
    let pf = parse(
        "impl Display for Foo {\n    fn fmt(&self) {}\n}\n",
        Language::Rust,
        "a.rs",
    );
    assert!(owners(&pf).contains(&(Some("fmt".into()), Some("Foo".into()))));
}

#[test]
fn rust_trait_default_method_owner_is_trait() {
    let pf = parse(
        "trait Greet {\n    fn hello(&self) {}\n}\n",
        Language::Rust,
        "a.rs",
    );
    assert!(owners(&pf).contains(&(Some("hello".into()), Some("Greet".into()))));
}

#[test]
fn rust_nested_fn_inside_method_is_not_a_method() {
    let pf = parse(
        "impl Foo {\n    fn m(&self) {\n        fn helper() {}\n        helper();\n    }\n}\n",
        Language::Rust,
        "a.rs",
    );
    assert!(owners(&pf).contains(&(Some("helper".into()), None)));
}
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test --test ast owner_test:: 2>&1 | tail -5`
Expected: compile error — `FunctionInfo` has no field `owner`.

- [ ] **Step 3: Implement**

`src/ast.rs` — extend the struct and table builder:

```rust
pub struct FunctionInfo {
    pub name: Option<String>,
    pub kind_id: u16,
    pub start_byte: usize,
    pub end_byte: usize,
    pub start_line: usize, // 1-indexed, inclusive
    pub end_line: usize,   // 1-indexed, inclusive
    pub param_names: Vec<String>,
    /// S3: owning type for methods (bare key, generics stripped). None = free fn.
    pub owner: Option<String>,
    /// S3 (Go only): receiver variable name (`t` in `func (t *T) m()`).
    pub receiver_var: Option<String>,
}
```

In `build_function_table`, after `param_names`:

```rust
                owner: self
                    .language
                    .method_owner(&node)
                    .map(|n| crate::resolution::owner_key(self.node_text(&n))),
                receiver_var: self
                    .language
                    .go_receiver_var(&node)
                    .map(|n| self.node_text(&n).to_string()),
```

(`owner_key` lands in Task 5's module; for THIS task define it temporarily in `src/ast.rs` as a free fn and move it in Task 5 — or create `src/resolution.rs` now with just `owner_key`/`peel_type` and `pub mod resolution;` in `src/lib.rs`. Do the latter; it avoids churn.)

```rust
// src/resolution.rs (new file, started here, grown in Task 5)
//! S3 call-resolution: confidence types, owner-key normalization, and the
//! R1–R7 resolution ladder (impl on CallGraph lives here to keep
//! call_graph.rs under the size cap).

/// Normalize an owner type's source text to its bare index key:
/// strip refs/pointers, smart-pointer wrappers are NOT peeled here (that is
/// receiver peeling, `peel_type`), strip generic args, strip `dyn `/`impl `.
pub fn owner_key(text: &str) -> String {
    let t = text.trim();
    let t = t.trim_start_matches("&mut ").trim_start_matches('&');
    let t = t.trim_start_matches('*');
    let t = t.trim_start_matches("dyn ").trim_start_matches("impl ");
    let t = t.split('<').next().unwrap_or(t);
    // C++ out-of-line `ns::Foo` declarator prefix → last segment
    let t = t.rsplit("::").next().unwrap_or(t);
    t.trim().to_string()
}
```

`src/languages/mod.rs` — new accessors (house style: return `Node`, caller extracts text):

```rust
    /// S3: the type that owns a method definition — the enclosing
    /// impl/trait/class type name node. None for free functions and for
    /// languages without methods (C/Bash/Terraform). Direct-member rule:
    /// a nested function inside a method returns None.
    pub fn method_owner<'a>(&self, func_node: &Node<'a>) -> Option<Node<'a>> {
        match self {
            Language::Rust => {
                let mut anc = func_node.parent();
                while let Some(n) = anc {
                    match n.kind() {
                        "impl_item" => return n.child_by_field_name("type"),
                        "trait_item" => return n.child_by_field_name("name"),
                        // crossing another function boundary ⇒ nested fn, not a method
                        "function_item" | "closure_expression" => return None,
                        _ => {}
                    }
                    anc = n.parent();
                }
                None
            }
            _ => None, // other languages: Tasks 2–3
        }
    }

    /// S3 (Rust): for `impl Trait for Type`, the trait name node (dual-key).
    pub fn rust_impl_trait<'a>(&self, func_node: &Node<'a>) -> Option<Node<'a>> {
        if !matches!(self, Language::Rust) {
            return None;
        }
        let mut anc = func_node.parent();
        while let Some(n) = anc {
            match n.kind() {
                // `impl_item` has `trait` field only in `impl Trait for Type` form
                "impl_item" => return n.child_by_field_name("trait"),
                "function_item" | "closure_expression" => return None,
                _ => {}
            }
            anc = n.parent();
        }
        None
    }

    /// S3 (Go): receiver variable name node (`t` in `func (t *T) m()`).
    pub fn go_receiver_var<'a>(&self, _func_node: &Node<'a>) -> Option<Node<'a>> {
        None // Go: Task 2
    }
```

- [ ] **Step 4: Run tests**

Run: `cargo test --test ast owner_test:: 2>&1 | tail -5`
Expected: 5 passed. Verify the trait-impl `type` vs `trait` field names against tree-sitter-rust if a test fails (`impl_item` fields: `trait`, `type`, `body` — check with a quick `parsed.tree.root_node().to_sexp()` print in the test if needed).

- [ ] **Step 5: Run full suite + fmt; commit**

Run: `cargo fmt && cargo test 2>&1 | tail -3`
Expected: all pass (new fields default-constructed nowhere else — `FunctionInfo` is only built in `build_function_table`; fix any struct-literal compile errors in `src/ast.rs` tests by adding the two fields).

```bash
git add -A && git commit -m "feat(s3): FunctionInfo.owner + Rust method_owner extraction"
```

---

### Task 2: Go / Python / JS / TS / Java owner extraction

**Files:**
- Modify: `src/languages/mod.rs` (`method_owner`, `go_receiver_var`)
- Modify: `tests/ast/owner_test.rs`

- [ ] **Step 1: Write failing tests**

```rust
#[test]
fn go_method_owner_and_receiver_var() {
    let pf = parse(
        "package p\n\ntype T struct{}\n\nfunc (t *T) M() {}\n\nfunc Free() {}\n",
        Language::Go,
        "a.go",
    );
    let f = pf.functions().iter().find(|f| f.name.as_deref() == Some("M")).unwrap();
    assert_eq!(f.owner.as_deref(), Some("T")); // '*' stripped by owner_key
    assert_eq!(f.receiver_var.as_deref(), Some("t"));
    let free = pf.functions().iter().find(|f| f.name.as_deref() == Some("Free")).unwrap();
    assert_eq!(free.owner, None);
}

#[test]
fn python_direct_member_only() {
    let pf = parse(
        "class C:\n    def m(self):\n        def nested():\n            pass\n\n@deco\ndef free():\n    pass\n",
        Language::Python,
        "a.py",
    );
    let o = owners(&pf);
    assert!(o.contains(&(Some("m".into()), Some("C".into()))));
    assert!(o.contains(&(Some("nested".into()), None)));
    assert!(o.contains(&(Some("free".into()), None)));
}

#[test]
fn js_class_method_owner() {
    let pf = parse(
        "class Widget {\n  render() {}\n  handler = () => {};\n}\nfunction free() {}\n",
        Language::JavaScript,
        "a.js",
    );
    let o = owners(&pf);
    assert!(o.contains(&(Some("render".into()), Some("Widget".into()))));
    // class-field arrow method (plan-review MINOR): owner via field_definition → class_body
    assert!(o.contains(&(Some("handler".into()), Some("Widget".into()))));
    assert!(o.contains(&(Some("free".into()), None)));
}

#[test]
fn java_every_method_has_owner() {
    let pf = parse(
        "class App {\n    void run() {}\n    static void main(String[] a) {}\n}\n",
        Language::Java,
        "A.java",
    );
    for f in pf.functions() {
        assert!(f.owner.as_deref() == Some("App"), "{:?}", f.name);
    }
}
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test --test ast owner_test:: 2>&1 | tail -5` — new tests FAIL (owner None).

- [ ] **Step 3: Implement**

Extend `method_owner`'s match (the ancestor-walk pattern; each language stops at its own function-node kinds so nested defs return None):

```rust
            Language::Go => {
                if func_node.kind() != "method_declaration" {
                    return None;
                }
                let recv = func_node.child_by_field_name("receiver")?;
                let mut c = recv.walk();
                for ch in recv.children(&mut c) {
                    if ch.kind() == "parameter_declaration" {
                        return ch.child_by_field_name("type");
                    }
                }
                None
            }
            Language::Python => {
                // function_definition (possibly wrapped in decorated_definition)
                // whose enclosing block's parent is class_definition = direct member.
                let mut n = *func_node;
                if let Some(p) = n.parent() {
                    if p.kind() == "decorated_definition" {
                        n = p;
                    }
                }
                let block = n.parent()?;
                if block.kind() != "block" {
                    return None;
                }
                let cls = block.parent()?;
                if cls.kind() == "class_definition" {
                    return cls.child_by_field_name("name");
                }
                None
            }
            Language::JavaScript | Language::TypeScript | Language::Tsx => {
                // method_definition → class_body, OR class-field arrow:
                // arrow_function → field_definition/public_field_definition → class_body
                let mut body = func_node.parent()?;
                if matches!(body.kind(), "field_definition" | "public_field_definition") {
                    body = body.parent()?;
                }
                if body.kind() != "class_body" {
                    return None;
                }
                let cls = body.parent()?;
                if matches!(cls.kind(), "class_declaration" | "class") {
                    return cls.child_by_field_name("name");
                }
                None
            }
            Language::Java => {
                let body = func_node.parent()?;
                if body.kind() != "class_body" {
                    return None;
                }
                let cls = body.parent()?;
                if matches!(cls.kind(), "class_declaration" | "enum_declaration") {
                    return cls.child_by_field_name("name");
                }
                None
            }
```

If the class-field arrow test fails on `name == None` (anonymous arrow): extend the JS/TS arm of `function_name` to return the enclosing `field_definition`'s `property` name node when the function node's parent is a field definition — the same pattern as the Lua keying fix in Task 3.

And `go_receiver_var`:

```rust
    pub fn go_receiver_var<'a>(&self, func_node: &Node<'a>) -> Option<Node<'a>> {
        if !matches!(self, Language::Go) || func_node.kind() != "method_declaration" {
            return None;
        }
        let recv = func_node.child_by_field_name("receiver")?;
        let mut c = recv.walk();
        for ch in recv.children(&mut c) {
            if ch.kind() == "parameter_declaration" {
                return ch.child_by_field_name("name");
            }
        }
        None
    }
```

Note: Go owner text is `*T` for pointer receivers — `owner_key`'s `*` strip handles it. JS method nodes are `method_definition` (already in the Functions query); TS class methods same.

- [ ] **Step 4: Run tests; fix node-kind mismatches against grammar reality (print `to_sexp()` when stuck)**

Run: `cargo test --test ast owner_test:: 2>&1 | tail -5` — all pass.

- [ ] **Step 5: fmt + full suite + commit**

```bash
cargo fmt && cargo test 2>&1 | tail -3
git add -A && git commit -m "feat(s3): owner extraction for Go/Python/JS/TS/Java + Go receiver_var"
```

---

### Task 3: C++ and Lua owner extraction (incl. Lua name-keying change)

**Files:**
- Modify: `src/languages/mod.rs`
- Modify: `tests/ast/owner_test.rs`

- [ ] **Step 1: Write failing tests**

```rust
#[test]
fn cpp_in_class_and_out_of_line_owners() {
    let pf = parse(
        "class Foo {\n  void bar();\n  void inline_m() {}\n};\nvoid Foo::bar() {}\nvoid free_fn() {}\nnamespace ns { void nf() {} }\n",
        Language::Cpp,
        "a.cpp",
    );
    let o = owners(&pf);
    assert!(o.contains(&(Some("inline_m".into()), Some("Foo".into()))));
    assert!(o.contains(&(Some("bar".into()), Some("Foo".into())))); // out-of-line
    assert!(o.contains(&(Some("free_fn".into()), None)));
    // namespace prefix indexes uniformly as the prefix key (spec §2.1 C++ row)
    assert!(o.contains(&(Some("nf".into()), None)) || o.contains(&(Some("nf".into()), Some("ns".into()))));
}

#[test]
fn lua_table_methods_key_by_bare_name_with_owner() {
    let pf = parse(
        "local M = {}\nfunction M.f() end\nfunction M:g() end\nfunction free() end\n",
        Language::Lua,
        "a.lua",
    );
    let o = owners(&pf);
    // KEYING CHANGE (spec §2.1): name is now "f", not "M.f"
    assert!(o.contains(&(Some("f".into()), Some("M".into()))));
    assert!(o.contains(&(Some("g".into()), Some("M".into()))));
    assert!(o.contains(&(Some("free".into()), None)));
}
```

- [ ] **Step 2: Run to verify failure** — `cargo test --test ast owner_test:: 2>&1 | tail -5`

- [ ] **Step 3: Implement**

C++ in `method_owner`:

```rust
            Language::Cpp => {
                // In-class definition: ancestor field_declaration_list → class_specifier
                let mut anc = func_node.parent();
                while let Some(n) = anc {
                    match n.kind() {
                        "field_declaration_list" => {
                            let cls = n.parent()?;
                            if matches!(cls.kind(), "class_specifier" | "struct_specifier") {
                                return cls.child_by_field_name("name");
                            }
                            return None;
                        }
                        "function_definition" => break,
                        _ => {}
                    }
                    anc = n.parent();
                }
                // Out-of-line: declarator is qualified_identifier `Foo::bar`
                let decl = func_node.child_by_field_name("declarator")?;
                let mut d = decl;
                loop {
                    if d.kind() == "qualified_identifier" {
                        return d.child_by_field_name("scope");
                    }
                    d = d.child_by_field_name("declarator")?;
                }
            }
```

Lua: two changes. (a) owner via the name node's table part; (b) **`function_name` keying change** — when the Lua name node is a `dot_index_expression`/`method_index_expression`, return the field/method child (mirrors what `call_function_name` already does at `languages/mod.rs:655-666`). Find the Lua arm of `function_name` and apply; then:

```rust
            Language::Lua => {
                // function M.f() / function M:g() — owner = table identifier
                let name = func_node.child_by_field_name("name")?;
                if matches!(name.kind(), "dot_index_expression" | "method_index_expression") {
                    return name
                        .child_by_field_name("table")
                        .or_else(|| name.child_by_field_name("object"));
                }
                None
            }
```

**Expect collateral drift:** Lua tests elsewhere that asserted `"M.f"`-style names (grep: `rg -n '"[A-Za-z_]+\.[a-z_]+"' tests/lang/lua/ tests/`) need updating to bare names — that is the spec-sanctioned keying change; update those assertions in this commit and say so in the commit message.

- [ ] **Step 4: Run** `cargo test --test ast owner_test:: && cargo test --test lang_lua 2>&1 | tail -5` — fix Lua assertion drift.

- [ ] **Step 5: fmt + full suite + commit**

```bash
cargo fmt && cargo test 2>&1 | tail -3
git add -A && git commit -m "feat(s3): C++/Lua owner extraction; Lua method names keyed bare (spec 2.1)"
```

---

### Task 4: `CallGraph` methods index, side maps, `CallSite.receiver_type`, cache v4

**Files:**
- Modify: `src/call_graph.rs` (struct fields; `empty/build/build_skeleton/build_direct_subset/remove_files/merge`)
- Modify: `src/cpg_cache.rs:44` (`CACHE_VERSION` 3 → 4)
- Modify: `tests/integration/call_graph_test.rs` (index tests), `tests/ast/cpg_cache_test.rs` (round-trip covers new fields automatically; version-bump assert if one exists)

- [ ] **Step 1: Write failing test**

```rust
// tests/integration/call_graph_test.rs (append)
#[test]
fn methods_index_and_side_maps_populated() {
    let mut files = std::collections::BTreeMap::new();
    files.insert(
        "a.rs".to_string(),
        prism::ast::ParsedFile::parse(
            "a.rs",
            "struct Foo;\nimpl Foo {\n    fn m(&self) {}\n}\nimpl Clone for Foo {\n    fn clone(&self) -> Foo { Foo }\n}\nfn free() {}\n",
            prism::languages::Language::Rust,
        )
        .unwrap(),
    );
    let cg = prism::call_graph::CallGraph::build(&files);
    // (owner, name) index
    assert!(cg.methods.contains_key(&("Foo".to_string(), "m".to_string())));
    // dual-key: trait impl indexed under the trait too
    assert!(cg.methods.contains_key(&("Clone".to_string(), "clone".to_string())));
    assert!(cg.methods.contains_key(&("Foo".to_string(), "clone".to_string())));
    // side map: owner by FunctionId
    let m_fid = &cg.functions["m"][0];
    assert_eq!(cg.method_owners.get(m_fid).map(|s| s.as_str()), Some("Foo"));
    // free fn absent
    let free_fid = &cg.functions["free"][0];
    assert!(!cg.method_owners.contains_key(free_fid));
}
```

- [ ] **Step 2: Run** `cargo test --test integration call_graph_test::methods_index 2>&1 | tail -5` — compile FAIL (no field `methods`).

- [ ] **Step 3: Implement**

`src/call_graph.rs` struct additions (all serde-derived already on `CallGraph`):

```rust
pub struct CallSite {
    pub caller: FunctionId,
    pub callee_name: String,
    pub line: usize,
    pub qualifier: Option<String>,
    /// S3 P6-lite: receiver type recovered syntactically at extraction time
    /// (typed param / constructor local, peeled). None = unrecovered.
    #[serde(default)]
    pub receiver_type: Option<String>,
    /// S3 P6-lite: which syntactic fact recovered `receiver_type`
    /// (telemetry + ResolutionKind split). Excluded from cmp_key —
    /// derived from the same scan as receiver_type.
    #[serde(default)]
    pub receiver_recovery: Option<crate::resolution::ReceiverRecovery>,
}

pub struct CallGraph {
    pub functions: BTreeMap<String, Vec<FunctionId>>,
    pub calls: BTreeMap<FunctionId, BTreeSet<CallSite>>,
    pub callers: BTreeMap<String, Vec<CallSite>>,
    pub static_functions: BTreeSet<(String, String)>,
    pub imports: BTreeMap<String, BTreeMap<String, String>>,
    /// S3: (owner_key, method_name) → definitions. Trait impls dual-keyed
    /// under both the impl type and the trait name.
    #[serde(default)]
    pub methods: BTreeMap<(String, String), Vec<FunctionId>>,
    /// S3: owning type per method FunctionId (primary owner, not the trait).
    #[serde(default)]
    pub method_owners: BTreeMap<FunctionId, String>,
    /// S3 (Go): receiver variable name per method FunctionId.
    #[serde(default)]
    pub receiver_vars: BTreeMap<FunctionId, String>,
}
```

Add `receiver_type: None, receiver_recovery: None` to every existing `CallSite { ... }` literal (build, build_skeleton, build_direct_subset, Phase 3 sites at lines ~321/350/428/516/534 — Phase-3 synthesized sites are *verified receiver-less* per the spec invariant, so `qualifier: None, receiver_type: None, receiver_recovery: None` is correct there). Extend `CallSite::cmp_key` with `self.receiver_type.as_deref()` as the final tuple element (`receiver_recovery` is derived data — excluded).

Phase-1 population (in `build`'s per-file closure; same loop has `func_node` + `parsed`):

```rust
                        let owner = parsed
                            .language
                            .method_owner(&func_node)
                            .map(|n| crate::resolution::owner_key(parsed.node_text(&n)));
                        let trait_key = parsed
                            .language
                            .rust_impl_trait(&func_node)
                            .map(|n| crate::resolution::owner_key(parsed.node_text(&n)));
                        let recv_var = parsed
                            .language
                            .go_receiver_var(&func_node)
                            .map(|n| parsed.node_text(&n).to_string());
```

…collect `(owner, trait_key, recv_var)` alongside each `(name, func_id)` in `FileFunctions`, then in the serial flatten:

```rust
                if let Some(o) = owner {
                    methods.entry((o.clone(), name.clone())).or_default().push(func_id.clone());
                    method_owners.insert(func_id.clone(), o);
                }
                if let Some(t) = trait_key {
                    methods.entry((t, name.clone())).or_default().push(func_id.clone());
                }
                if let Some(rv) = recv_var {
                    receiver_vars.insert(func_id.clone(), rv);
                }
```

Mirror the same population (serially) in `build_skeleton` and `build_direct_subset`. `empty()` gets the three new empty maps. `remove_files`: retain `methods` values by `!exclude.contains(&fid.file)` then drop empty keys; retain `method_owners`/`receiver_vars` by key file. `merge`: extend all three.

`src/cpg_cache.rs:44`: `const CACHE_VERSION: u32 = 4;` and append history line `/// - v4: CallGraph methods/owner indexes + CallSite.receiver_type (S3)`.

- [ ] **Step 4: Run** `cargo test --test integration call_graph_test:: && cargo test --test ast cpg_cache_test:: 2>&1 | tail -4` — pass.

- [ ] **Step 5: fmt + full suite + harness + commit**

```bash
cargo fmt && cargo test 2>&1 | tail -3
cargo build --release && (cd eval && uv run tier-a --matrix-only --allow-stale-sut)
git add -A && git commit -m "feat(s3): CallGraph (owner,name) method index + side maps; cache v4"
```

---

### Task 5: Resolution types + ladder core — R1 (qualified owner), R2 (self/receiver), static linkage

**Files:**
- Modify: `src/resolution.rs` (the ladder), `src/lib.rs` (`pub mod resolution;` if not already)
- Create: `tests/integration/resolution_test.rs`; Modify: `tests/integration/main.rs` (`mod resolution_test;`)

- [ ] **Step 1: Write failing tests**

```rust
// tests/integration/resolution_test.rs
use prism::call_graph::{CallGraph, CallSite, FunctionId};
use prism::resolution::{ResolutionConfidence, ResolutionKind};
use std::collections::BTreeMap;

fn build(sources: &[(&str, &str, prism::languages::Language)]) -> (CallGraph, BTreeMap<String, prism::ast::ParsedFile>) {
    let mut files = BTreeMap::new();
    for (path, src, lang) in sources {
        files.insert(path.to_string(), prism::ast::ParsedFile::parse(path, src, *lang).unwrap());
    }
    (CallGraph::build(&files), files)
}

fn site_in(cg: &CallGraph, caller_name: &str, callee: &str) -> CallSite {
    cg.calls
        .iter()
        .find(|(fid, _)| fid.name == caller_name)
        .and_then(|(_, sites)| sites.iter().find(|s| s.callee_name == callee))
        .unwrap_or_else(|| panic!("no site {caller_name}->{callee}"))
        .clone()
}

#[test]
fn r1_type_qualified_call_resolves_to_owner_method_exact() {
    use prism::languages::Language::Rust;
    let (cg, _) = build(&[
        ("engine.rs", "pub struct Engine;\nimpl Engine {\n    pub fn start() {}\n}\n", Rust),
        ("main.rs", "fn main() {\n    Engine::start();\n}\n", Rust),
    ]);
    let site = site_in(&cg, "main", "Engine::start");
    let r = cg.resolve_call_site(&site);
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].target.file, "engine.rs");
    assert_eq!(r[0].confidence, ResolutionConfidence::Exact);
    assert_eq!(r[0].kind, ResolutionKind::QualifiedOwner);
}

#[test]
fn r1_trait_qualified_multi_impl_demotes_to_name_only() {
    use prism::languages::Language::Rust;
    let (cg, _) = build(&[
        ("a.rs", "impl Render for A {\n    fn draw(&self) {}\n}\n", Rust),
        ("b.rs", "impl Render for B {\n    fn draw(&self) {}\n}\n", Rust),
        ("main.rs", "fn go(x: &dyn Render) {\n    Render::draw(x);\n}\n", Rust),
    ]);
    let site = site_in(&cg, "go", "Render::draw");
    let r = cg.resolve_call_site(&site);
    assert_eq!(r.len(), 2);
    assert!(r.iter().all(|c| c.confidence == ResolutionConfidence::NameOnly));
    assert!(r.iter().all(|c| c.kind == ResolutionKind::TraitCha));
}

#[test]
fn r2_self_method_call_resolves_via_enclosing_owner_cross_file() {
    use prism::languages::Language::Rust;
    let (cg, _) = build(&[
        ("a.rs", "impl Foo {\n    fn entry(&self) {\n        self.helper();\n    }\n}\n", Rust),
        ("b.rs", "impl Foo {\n    fn helper(&self) {}\n}\nimpl Bar {\n    fn helper(&self) {}\n}\n", Rust),
    ]);
    let site = site_in(&cg, "entry", "helper");
    let r = cg.resolve_call_site(&site);
    assert_eq!(r.len(), 1, "must hit Foo::helper only, not Bar::helper");
    assert_eq!(r[0].kind, ResolutionKind::SelfReceiver);
    assert_eq!(r[0].confidence, ResolutionConfidence::Exact);
}

#[test]
fn r2_go_receiver_var_call() {
    use prism::languages::Language::Go;
    let (cg, _) = build(&[(
        "a.go",
        "package p\ntype T struct{}\nfunc (t *T) A() {\n    t.B()\n}\nfunc (t *T) B() {}\ntype U struct{}\nfunc (u *U) B() {}\n",
        Go,
    )]);
    let site = site_in(&cg, "A", "B");
    let r = cg.resolve_call_site(&site);
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].kind, ResolutionKind::SelfReceiver);
}
```

- [ ] **Step 2: Run** `cargo test --test integration resolution_test:: 2>&1 | tail -5` — compile FAIL.

- [ ] **Step 3: Implement the ladder core in `src/resolution.rs`**

```rust
use crate::call_graph::{CallGraph, CallSite, FunctionId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ResolutionConfidence {
    Exact,
    NameOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolutionKind {
    StaticLinkage,
    QualifiedOwner,
    SelfReceiver,
    ImportQualified,
    QualifierOwner,
    LocalDef,
    ImplicitThis,
    FreeSingle,
    FreeMulti,
    TypedParam,
    ConstructorLocal,
    TraitCha,
    R6SingleOwner,
    StemSingle,
    StemMulti,
}

impl ResolutionKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            ResolutionKind::StaticLinkage => "static_linkage",
            ResolutionKind::QualifiedOwner => "qualified_owner",
            ResolutionKind::SelfReceiver => "self_receiver",
            ResolutionKind::ImportQualified => "import_qualified",
            ResolutionKind::QualifierOwner => "qualifier_owner",
            ResolutionKind::LocalDef => "local_def",
            ResolutionKind::ImplicitThis => "implicit_this",
            ResolutionKind::FreeSingle => "free_single",
            ResolutionKind::FreeMulti => "free_multi",
            ResolutionKind::TypedParam => "typed_param",
            ResolutionKind::ConstructorLocal => "constructor_local",
            ResolutionKind::TraitCha => "trait_cha",
            ResolutionKind::R6SingleOwner => "r6_single_owner",
            ResolutionKind::StemSingle => "stem_single",
            ResolutionKind::StemMulti => "stem_multi",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedCallee<'a> {
    pub target: &'a FunctionId,
    pub confidence: ResolutionConfidence,
    pub kind: ResolutionKind,
}

/// Which syntactic fact recovered a receiver type (stored on CallSite).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ReceiverRecovery {
    TypedParam,
    ConstructorLocal,
}

/// Why a call site resolved to nothing — the classification API that
/// Collision warnings (Task 11) and call-stats telemetry (Task 12) consume.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DropReason {
    /// R6: method name defined on multiple owner types, receiver unknown.
    MultiOwnerCollision,
    /// P6-lite: receiver type recovered but (T, m) has no entry — provably
    /// external (Vec::truncate class) or wrong-name.
    ExternalReceiver,
    /// R3: qualifier is an import that narrows to no in-repo candidate.
    ImportExternal,
    /// Name not defined in-repo at all (ordinary unresolved call).
    UnknownName,
}

pub struct ResolutionOutcome<'a> {
    pub resolved: Vec<ResolvedCallee<'a>>,
    /// Some(..) iff `resolved` is empty for a classified reason.
    pub drop: Option<DropReason>,
}

impl<'a> ResolutionOutcome<'a> {
    fn hit(resolved: Vec<ResolvedCallee<'a>>) -> Self {
        Self { resolved, drop: None }
    }
    fn dropped(reason: DropReason) -> Self {
        Self { resolved: Vec::new(), drop: Some(reason) }
    }
}
```

**Ladder body convention (applies to every snippet in Tasks 5–8):** the ladder is
implemented ONCE as `resolve_call_site_full(&self, site) -> ResolutionOutcome`;
`resolve_call_site` is `self.resolve_call_site_full(site).resolved`. Each
`return Vec::new()` in the Task 5–8 snippets translates mechanically:
R3 empty-narrowing → `ResolutionOutcome::dropped(DropReason::ImportExternal)`;
P6 recovered-type index-miss → `dropped(ExternalReceiver)`; R6 multi-owner →
`dropped(MultiOwnerCollision)`; name absent from `functions` (and `methods`) →
`dropped(UnknownName)`; non-empty paths → `ResolutionOutcome::hit(..)`.

```rust

fn exact<'a>(ids: impl IntoIterator<Item = &'a FunctionId>, kind: ResolutionKind) -> Vec<ResolvedCallee<'a>> {
    ids.into_iter()
        .map(|target| ResolvedCallee { target, confidence: ResolutionConfidence::Exact, kind })
        .collect()
}

fn demoted<'a>(ids: impl IntoIterator<Item = &'a FunctionId>, kind: ResolutionKind) -> Vec<ResolvedCallee<'a>> {
    ids.into_iter()
        .map(|target| ResolvedCallee { target, confidence: ResolutionConfidence::NameOnly, kind })
        .collect()
}

fn is_simple_ident(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_alphanumeric() || c == '_')
}

impl CallGraph {
    /// Owner-index lookup that knows whether the key is a multi-impl trait key.
    fn owner_lookup(&self, owner: &str, name: &str) -> Option<Vec<ResolvedCallee<'_>>> {
        let ids = self.methods.get(&(owner.to_string(), name.to_string()))?;
        // A key hit with >1 candidates whose primary owners differ is trait-CHA
        // fan-out (dual-key) — demote (spec §2.1). Same-owner duplicates (split
        // impl blocks) stay Exact.
        let primary_owners: std::collections::BTreeSet<&str> = ids
            .iter()
            .filter_map(|fid| self.method_owners.get(fid).map(|s| s.as_str()))
            .collect();
        Some(if ids.len() > 1 && primary_owners.len() > 1 {
            demoted(ids.iter(), ResolutionKind::TraitCha)
        } else {
            exact(ids.iter(), ResolutionKind::QualifiedOwner)
        })
    }

    /// S3 R1–R7 ladder. The single resolution entry point for edge creation
    /// (CPG Step 5/5b), navigation, and call-graph traversal helpers.
    /// `resolve_callees` (name-only) remains for recall-biased scope/Phase-3 use.
    pub fn resolve_call_site(&self, site: &CallSite) -> Vec<ResolvedCallee<'_>> {
        let name = site.callee_name.as_str();
        let caller = &site.caller;

        // ---- ::-path shapes (Rust/C++; name carries the full path) ----
        if name.contains("::") {
            let mut segs: Vec<&str> = name.split("::").collect();
            let fn_name = segs.pop().unwrap_or(name);
            while matches!(segs.first(), Some(&"crate") | Some(&"super")) {
                segs.remove(0);
            }
            // `self::f()` — current-module free fn: local-file preference.
            if segs.as_slice() == ["self"] {
                if let Some(ids) = self.functions.get(fn_name) {
                    let local: Vec<&FunctionId> =
                        ids.iter().filter(|f| f.file == caller.file).collect();
                    if !local.is_empty() {
                        return exact(local, ResolutionKind::LocalDef);
                    }
                }
                return Vec::new();
            }
            if let Some(&head) = segs.last() {
                // `Self::f()` → enclosing owner (R2)
                if head == "Self" {
                    if let Some(owner) = self.method_owners.get(caller) {
                        if let Some(mut r) = self.owner_lookup(owner, fn_name) {
                            for c in &mut r {
                                if c.kind == ResolutionKind::QualifiedOwner {
                                    c.kind = ResolutionKind::SelfReceiver;
                                }
                            }
                            return r;
                        }
                    }
                    return Vec::new();
                }
                // R1: `T::m` / `mod::T::m` — owner-index lookup on the last segment.
                if let Some(r) = self.owner_lookup(head, fn_name) {
                    return r;
                }
                // R7: stem fallback (module-qualified free fn), moved from nav.
                if let Some(ids) = self.functions.get(fn_name) {
                    let matched: Vec<&FunctionId> = ids
                        .iter()
                        .filter(|fid| crate::resolution::file_stem(&fid.file) == head)
                        .collect();
                    return match matched.len() {
                        0 => Vec::new(),
                        1 => exact(matched, ResolutionKind::StemSingle),
                        _ => demoted(matched, ResolutionKind::StemMulti),
                    };
                }
            }
            return Vec::new();
        }

        // ---- static linkage (C/C++) — preserved pre-ladder behavior ----
        if self
            .static_functions
            .contains(&(caller.file.clone(), name.to_string()))
        {
            if let Some(ids) = self.functions.get(name) {
                return exact(
                    ids.iter().filter(|f| f.file == caller.file),
                    ResolutionKind::StaticLinkage,
                );
            }
            return Vec::new();
        }

        match site.qualifier.as_deref() {
            // ---- R2: explicit self/this/cls receiver, or Go receiver var ----
            Some(q)
                if q == "self"
                    || q == "this"
                    || q == "cls"
                    || self.receiver_vars.get(caller).map(String::as_str) == Some(q) =>
            {
                if let Some(owner) = self.method_owners.get(caller) {
                    if let Some(mut r) = self.owner_lookup(owner, name) {
                        for c in &mut r {
                            if c.kind == ResolutionKind::QualifiedOwner {
                                c.kind = ResolutionKind::SelfReceiver;
                            }
                        }
                        return r;
                    }
                }
                Vec::new() // method on own type not found in-repo: unresolved
            }
            Some(_) | None => Vec::new(), // R3/R3b/R4–R6: Tasks 6–7
        }
    }
}

/// File stem matching `resolve_callees_qualified`'s idiom (`a.b.rs` → `a`).
pub fn file_stem(path: &str) -> &str {
    path.rsplit('/')
        .next()
        .unwrap_or(path)
        .rsplit('.')
        .last()
        .unwrap_or(path)
}
```

- [ ] **Step 4: Run** `cargo test --test integration resolution_test:: 2>&1 | tail -5` — 4 pass. (If `Render::draw(x)` extraction doesn't produce a site — UFCS — check what `call_function_name` returns for `scoped_identifier`; the site's callee_name must be `"Render::draw"`. Adjust the test source to a shape extraction sees if needed, e.g. via `x.draw()` is Task 7 — keep the trait-qualified form here.)

- [ ] **Step 5: fmt + full suite + commit**

```bash
cargo fmt && cargo test 2>&1 | tail -3
git add -A && git commit -m "feat(s3): resolution ladder core — R1 qualified-owner, R2 self/receiver, R7 stem, trait-CHA demotion"
```

---

### Task 6: R3 (import narrowing, no fall-through, Go package paths), R3b, R4, R4b, R5

**Files:**
- Modify: `src/resolution.rs` (replace the `Some(_) | None => Vec::new()` arms)
- Modify: `tests/integration/resolution_test.rs`

- [ ] **Step 1: Write failing tests**

```rust
#[test]
fn r3_import_qualified_with_no_repo_candidate_is_unresolved() {
    use prism::languages::Language::Go;
    // `zap` is imported but resolves to no in-repo file ⇒ provably external ⇒ NO edge
    // (the ~148-FP caddy class, spec §2.2 R3).
    let (cg, _) = build(&[
        ("notify/notify.go", "package notify\nfunc Error(err error) error { return err }\n", Go),
        ("main.go", "package main\nimport \"go.uber.org/zap\"\nfunc main() {\n    zap.Error(nil)\n}\n", Go),
    ]);
    let site = site_in(&cg, "main", "Error");
    assert!(cg.resolve_call_site(&site).is_empty());
}

#[test]
fn r3_go_import_matches_package_directory_not_file_stem() {
    use prism::languages::Language::Go;
    let (cg, _) = build(&[
        ("modules/caddyhttp/errors.go", "package caddyhttp\nfunc Error(c int) error { return nil }\n", Go),
        ("main.go", "package main\nimport \"github.com/x/y/modules/caddyhttp\"\nfunc main() {\n    caddyhttp.Error(1)\n}\n", Go),
    ]);
    let site = site_in(&cg, "main", "Error");
    let r = cg.resolve_call_site(&site);
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].target.file, "modules/caddyhttp/errors.go");
    assert_eq!(r[0].kind, ResolutionKind::ImportQualified);
}

#[test]
fn r3b_qualifier_as_owner_resolves_statics() {
    use prism::languages::Language::Python;
    let (cg, _) = build(&[
        ("c.py", "class Config:\n    def load(self):\n        pass\n", Python),
        ("m.py", "def main():\n    Config.load(cfg)\n", Python),
    ]);
    let site = site_in(&cg, "main", "load");
    let r = cg.resolve_call_site(&site);
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].kind, ResolutionKind::QualifierOwner);
}

#[test]
fn r4_local_free_definition_wins_alone() {
    use prism::languages::Language::Rust;
    let (cg, _) = build(&[
        ("a.rs", "fn slice() {}\nfn run() {\n    slice();\n}\n", Rust),
        ("b.rs", "fn slice() {}\n", Rust),
        ("c.rs", "fn slice() {}\n", Rust),
    ]);
    let site = site_in(&cg, "run", "slice");
    let r = cg.resolve_call_site(&site);
    assert_eq!(r.len(), 1, "local-def preference: a.rs only");
    assert_eq!(r[0].target.file, "a.rs");
    assert_eq!(r[0].kind, ResolutionKind::LocalDef);
}

#[test]
fn r4b_java_sibling_call_survives() {
    use prism::languages::Language::Java;
    // Java has NO free functions — unqualified f() is implicit-this (spec B1 fix).
    let (cg, _) = build(&[(
        "App.java",
        "class App {\n    void run() {\n        helper();\n    }\n    void helper() {}\n}\n",
        Java,
    )]);
    let site = site_in(&cg, "run", "helper");
    let r = cg.resolve_call_site(&site);
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].kind, ResolutionKind::ImplicitThis);
}

#[test]
fn r5_unqualified_never_binds_to_methods() {
    use prism::languages::Language::Rust;
    let (cg, _) = build(&[
        ("a.rs", "fn run() {\n    process();\n}\n", Rust),
        ("b.rs", "impl Worker {\n    fn process(&self) {}\n}\n", Rust),
    ]);
    let site = site_in(&cg, "run", "process");
    assert!(cg.resolve_call_site(&site).is_empty(), "method requires a receiver");
}

#[test]
fn r5_cross_file_free_multi_kept_demoted() {
    use prism::languages::Language::Rust;
    let (cg, _) = build(&[
        ("a.rs", "fn run() {\n    helper();\n}\n", Rust),
        ("b.rs", "fn helper() {}\n", Rust),
        ("c.rs", "fn helper() {}\n", Rust),
    ]);
    let site = site_in(&cg, "run", "helper");
    let r = cg.resolve_call_site(&site);
    assert_eq!(r.len(), 2);
    assert!(r.iter().all(|c| c.confidence == ResolutionConfidence::NameOnly));
    assert!(r.iter().all(|c| c.kind == ResolutionKind::FreeMulti));
}
```

- [ ] **Step 2: Run** — new tests FAIL (empty/wrong results).

- [ ] **Step 3: Implement — replace the placeholder arms**

```rust
            Some(q) => {
                // ---- R3: imported-module qualifier ----
                if let Some(file_imports) = self.imports.get(&caller.file) {
                    if let Some(module_path) = file_imports.get(q) {
                        let ids = self.functions.get(name).map(|v| v.as_slice()).unwrap_or(&[]);
                        let module_last = module_path.rsplit('/').next().unwrap_or(module_path);
                        let module_stem = module_last.rsplit('.').last().unwrap_or(module_last);
                        let matched: Vec<&FunctionId> = ids
                            .iter()
                            .filter(|fid| {
                                // (a) existing stem rule (Python `utils` → utils.py)
                                let stem_hit = file_stem(&fid.file) == module_stem;
                                // (b) Go package rule: file's parent dir == last
                                //     import-path segment (spec §2.2 R3)
                                let dir_hit = fid
                                    .file
                                    .rsplit('/')
                                    .nth(1)
                                    .map(|d| d == module_last)
                                    .unwrap_or(false);
                                stem_hit || dir_hit
                            })
                            .collect();
                        // NO fall-through (spec): import matched ⇒ external if empty.
                        return exact(matched, ResolutionKind::ImportQualified);
                    }
                }
                // ---- R3b: qualifier text is itself an owner key ----
                if is_simple_ident(q) {
                    if let Some(mut r) = self.owner_lookup(q, name) {
                        for c in &mut r {
                            if c.kind == ResolutionKind::QualifiedOwner {
                                c.kind = ResolutionKind::QualifierOwner;
                            }
                        }
                        return r;
                    }
                }
                // ---- R6: unknown receiver — Task 7 ----
                Vec::new()
            }
            None => {
                let ids = match self.functions.get(name) {
                    Some(v) => v,
                    None => return Vec::new(),
                };
                // Free candidates = not in the method_owners side map.
                let free: Vec<&FunctionId> = ids
                    .iter()
                    .filter(|fid| !self.method_owners.contains_key(*fid))
                    .collect();
                // ---- R4: local free definition wins alone ----
                let local: Vec<&FunctionId> =
                    free.iter().copied().filter(|f| f.file == caller.file).collect();
                if !local.is_empty() {
                    return exact(local, ResolutionKind::LocalDef);
                }
                // ---- R4b: implicit-this languages (Java/C++) ----
                if let Some(owner) = self.method_owners.get(caller) {
                    if caller.file.ends_with(".java")
                        || caller.file.ends_with(".cpp")
                        || caller.file.ends_with(".cc")
                        || caller.file.ends_with(".cxx")
                        || caller.file.ends_with(".hpp")
                        || caller.file.ends_with(".h")
                    {
                        if let Some(mut r) = self.owner_lookup(owner, name) {
                            for c in &mut r {
                                if c.kind == ResolutionKind::QualifiedOwner {
                                    c.kind = ResolutionKind::ImplicitThis;
                                }
                            }
                            return r;
                        }
                    }
                }
                // ---- R5: cross-file free functions only ----
                let nonstatic: Vec<&FunctionId> = free
                    .into_iter()
                    .filter(|fid| {
                        !self
                            .static_functions
                            .contains(&(fid.file.clone(), name.to_string()))
                    })
                    .collect();
                match nonstatic.len() {
                    0 => Vec::new(),
                    1 => exact(nonstatic, ResolutionKind::FreeSingle),
                    _ => demoted(nonstatic, ResolutionKind::FreeMulti),
                }
            }
```

Note on R3: Go import map values — verify what `collect_go_imports` stores (`src/ast.rs`, search `collect_go_imports`): if it stores the full quoted path (`github.com/x/y/modules/caddyhttp`), `module_last` is `caddyhttp` and the dir rule works. If it stores only the last segment already, both rules degenerate to the same string — fine.

- [ ] **Step 4: Run** `cargo test --test integration resolution_test:: 2>&1 | tail -5` — all pass.

- [ ] **Step 5: fmt + full suite + commit**

```bash
cargo fmt && cargo test 2>&1 | tail -3
git add -A && git commit -m "feat(s3): ladder R3 (no fall-through, Go pkg paths), R3b, R4, R4b implicit-this, R5 free-only"
```

---

### Task 7: R6 residue policy — single-owner demote, multi-owner drop

**Files:**
- Modify: `src/resolution.rs` (the `// ---- R6` placeholder)
- Modify: `tests/integration/resolution_test.rs`

- [ ] **Step 1: Write failing tests**

```rust
#[test]
fn r6_multi_owner_unknown_receiver_drops() {
    use prism::languages::Language::Rust;
    // The tokio `poll` class: x.poll() with poll on 2+ owner types ⇒ unresolved.
    let (cg, _) = build(&[
        ("a.rs", "impl A {\n    fn poll(&self) {}\n}\n", Rust),
        ("b.rs", "impl B {\n    fn poll(&self) {}\n}\n", Rust),
        ("m.rs", "fn drive(x: UnknownToIndex) {\n    x.poll();\n}\n", Rust),
    ]);
    let site = site_in(&cg, "drive", "poll");
    assert!(cg.resolve_call_site(&site).is_empty());
}

#[test]
fn r6_single_owner_unknown_receiver_kept_demoted() {
    use prism::languages::Language::Rust;
    let (cg, _) = build(&[
        ("a.rs", "impl OnlyOwner {\n    fn frobnicate(&self) {}\n}\n", Rust),
        ("m.rs", "fn run(x: UnknownToIndex) {\n    x.frobnicate();\n}\n", Rust),
    ]);
    let site = site_in(&cg, "run", "frobnicate");
    let r = cg.resolve_call_site(&site);
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].confidence, ResolutionConfidence::NameOnly);
    assert_eq!(r[0].kind, ResolutionKind::R6SingleOwner);
}

#[test]
fn r6_caller_file_single_owner_preferred_over_repo_multi() {
    use prism::languages::Language::Rust;
    let (cg, _) = build(&[
        ("m.rs", "impl Local {\n    fn step(&self) {}\n}\nfn run(x: UnknownToIndex) {\n    x.step();\n}\n", Rust),
        ("far.rs", "impl Far {\n    fn step(&self) {}\n}\n", Rust),
    ]);
    let site = site_in(&cg, "run", "step");
    let r = cg.resolve_call_site(&site);
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].target.file, "m.rs");
    assert_eq!(r[0].confidence, ResolutionConfidence::NameOnly);
}

#[test]
fn r6_never_binds_receiver_call_to_free_function() {
    use prism::languages::Language::Go;
    // The caddy `t.Error` class: receiver call must not hit free `Error`.
    let (cg, _) = build(&[
        ("notify/n.go", "package notify\nfunc Error(e error) error { return e }\n", Go),
        ("x.go", "package x\nfunc run(t Untyped) {\n    t.Error(nil)\n}\n", Go),
    ]);
    let site = site_in(&cg, "run", "Error");
    assert!(cg.resolve_call_site(&site).is_empty());
}
```

- [ ] **Step 2: Run** — FAIL (R6 arm returns empty for single-owner too / or binds free fns if R5 logic leaked).

- [ ] **Step 3: Implement — replace the R6 placeholder**

```rust
                // ---- R6 residue (P2): method candidates only, never free fns ----
                // (P6-lite recovered-receiver handling lands in Task 8 ahead of this.)
                let method_ids: Vec<&FunctionId> = self
                    .functions
                    .get(name)
                    .map(|v| {
                        v.iter()
                            .filter(|fid| self.method_owners.contains_key(*fid))
                            .collect()
                    })
                    .unwrap_or_default();
                if method_ids.is_empty() {
                    return Vec::new();
                }
                // Caller's-own-file preference: exactly one owner defining it there.
                let local: Vec<&FunctionId> = method_ids
                    .iter()
                    .copied()
                    .filter(|f| f.file == caller.file)
                    .collect();
                let local_owners: std::collections::BTreeSet<&str> = local
                    .iter()
                    .filter_map(|f| self.method_owners.get(*f).map(String::as_str))
                    .collect();
                if local_owners.len() == 1 {
                    return demoted(local, ResolutionKind::R6SingleOwner);
                }
                let owners: std::collections::BTreeSet<&str> = method_ids
                    .iter()
                    .filter_map(|f| self.method_owners.get(*f).map(String::as_str))
                    .collect();
                if owners.len() == 1 {
                    return demoted(method_ids, ResolutionKind::R6SingleOwner);
                }
                Vec::new() // multi-owner ⇒ prefer unresolved over wrong
```

- [ ] **Step 4: Run** `cargo test --test integration resolution_test:: 2>&1 | tail -5` — all pass.

- [ ] **Step 5: fmt + full suite + commit**

```bash
cargo fmt && cargo test 2>&1 | tail -3
git add -A && git commit -m "feat(s3): R6 residue policy — single-owner demote, multi-owner drop, methods-only"
```

---

### Task 8: P6-lite receiver-type recovery (Rust + Go, extraction-time)

**Files:**
- Modify: `src/ast.rs` (new `receiver_type_in_fn` + helper), `src/resolution.rs` (`peel_type`, R6 step-1 consumption), `src/call_graph.rs` (Phase 2 populates `CallSite.receiver_type`)
- Modify: `tests/integration/resolution_test.rs`

- [ ] **Step 1: Write failing tests**

```rust
#[test]
fn p6_typed_param_recovers_exact_among_collisions() {
    use prism::languages::Language::Rust;
    let (cg, _) = build(&[
        ("a.rs", "impl Sender {\n    fn send(&self) {}\n}\n", Rust),
        ("b.rs", "impl Pipe {\n    fn send(&self) {}\n}\n", Rust),
        ("m.rs", "fn run(tx: &Sender) {\n    tx.send();\n}\n", Rust),
    ]);
    let site = site_in(&cg, "run", "send");
    let r = cg.resolve_call_site(&site);
    assert_eq!(r.len(), 1, "typed param defeats the multi-owner drop");
    assert_eq!(r[0].target.file, "a.rs");
    assert_eq!(r[0].confidence, ResolutionConfidence::Exact);
    assert_eq!(r[0].kind, ResolutionKind::TypedParam);
}

#[test]
fn p6_peel_list_handles_pin_mut_self_shape() {
    assert_eq!(prism::resolution::peel_type("Pin<&mut Self>"), "Self");
    assert_eq!(prism::resolution::peel_type("Arc<Mutex<Conn>>"), "Mutex"); // outermost wrapper peeled, Mutex<Conn> → Mutex
    assert_eq!(prism::resolution::peel_type("&mut Foo"), "Foo");
    assert_eq!(prism::resolution::peel_type("Box<dyn Render>"), "Render");
    assert_eq!(prism::resolution::peel_type("*T"), "T");
}

#[test]
fn p6_constructor_local_recovers() {
    use prism::languages::Language::Rust;
    let (cg, _) = build(&[
        ("a.rs", "impl Engine {\n    pub fn new() -> Engine { Engine }\n    pub fn start(&self) {}\n}\n", Rust),
        ("b.rs", "impl Other {\n    fn start(&self) {}\n}\n", Rust),
        ("m.rs", "fn main() {\n    let e = Engine::new();\n    e.start();\n}\n", Rust),
    ]);
    let site = site_in(&cg, "main", "start");
    let r = cg.resolve_call_site(&site);
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].target.file, "a.rs");
    assert_eq!(r[0].kind, ResolutionKind::ConstructorLocal);
}

#[test]
fn p6_shadowing_bails_to_residue() {
    use prism::languages::Language::Rust;
    let (cg, _) = build(&[
        ("a.rs", "impl A {\n    fn go(&self) {}\n}\n", Rust),
        ("b.rs", "impl B {\n    fn go(&self) {}\n}\n", Rust),
        ("m.rs", "fn main() {\n    let x = A::new();\n    let x = mystery();\n    x.go();\n}\n", Rust),
    ]);
    let site = site_in(&cg, "main", "go");
    assert!(cg.resolve_call_site(&site).is_empty(), "rebinding ⇒ bail ⇒ multi-owner drop");
}

#[test]
fn p6_external_recovered_type_drops_stdlib_binding() {
    use prism::languages::Language::Rust;
    // The Vec::truncate→AccessPath::truncate class: receiver provably Vec ⇒ drop,
    // even though `truncate` has exactly one in-repo owner.
    let (cg, _) = build(&[
        ("ap.rs", "impl AccessPath {\n    fn truncate(&mut self) {}\n}\n", Rust),
        ("m.rs", "fn run(items: &mut Vec<String>) {\n    items.truncate(5);\n}\n", Rust),
    ]);
    let site = site_in(&cg, "run", "truncate");
    assert!(cg.resolve_call_site(&site).is_empty());
}
```

- [ ] **Step 2: Run** — FAIL (no recovery; first test resolves empty via multi-owner drop, external test resolves demoted).

- [ ] **Step 3: Implement**

`src/resolution.rs` — `peel_type` (closed list, spec §2.3):

```rust
/// Closed-list syntactic peel (spec §2.3): refs/pointers and std wrappers,
/// recursively; then generic args; then dyn/impl. NEVER Deref-semantic.
pub fn peel_type(text: &str) -> String {
    let mut t = text.trim();
    loop {
        let before = t;
        t = t.trim_start_matches("&mut ").trim_start_matches('&').trim_start_matches('*').trim();
        for w in ["Box", "Arc", "Rc", "Pin"] {
            if let Some(inner) = t.strip_prefix(w) {
                if let Some(inner) = inner.trim().strip_prefix('<') {
                    if let Some(inner) = inner.strip_suffix('>') {
                        t = inner.trim();
                    }
                }
            }
        }
        if t == before {
            break;
        }
    }
    let t = t.trim_start_matches("dyn ").trim_start_matches("impl ");
    t.split('<').next().unwrap_or(t).trim().to_string()
}
```

`src/ast.rs` — recovery scan (Rust + Go), called per call site at Phase-2 time:

```rust
    /// S3 P6-lite: syntactically-provable receiver type for `receiver` at a call
    /// on `call_line`. Typed params + constructor locals only; only bindings at
    /// or before `call_line` count (a rebinding AFTER the call must not cancel
    /// recovery); >1 binding before the call ⇒ shadow bail (None). Rust + Go.
    /// Returns the raw (unpeeled) type text + which fact recovered it.
    pub fn receiver_type_in_fn(
        &self,
        func_node: &Node<'_>,
        receiver: &str,
        call_line: usize,
    ) -> Option<(String, crate::resolution::ReceiverRecovery)> {
        use crate::languages::Language;
        if !matches!(self.language, Language::Rust | Language::Go) {
            return None;
        }
        use crate::resolution::ReceiverRecovery;
        let src = self.source.as_bytes();
        let mut found: Option<(String, ReceiverRecovery)> = None;
        let mut bindings = 0usize;
        // walk_bindings/param scan below: wrap every binding-site check with
        //   if node.start_position().row + 1 > call_line { skip }
        // (params have no line guard — they always precede the call).

        // (1) typed parameters
        if let Some(params) = func_node.child_by_field_name("parameters") {
            let mut c = params.walk();
            for p in params.children(&mut c) {
                match self.language {
                    Language::Rust if p.kind() == "parameter" => {
                        let (Some(pat), Some(ty)) =
                            (p.child_by_field_name("pattern"), p.child_by_field_name("type"))
                        else {
                            continue;
                        };
                        if pat.utf8_text(src).ok() == Some(receiver) {
                            found = Some((self.node_text(&ty).to_string(), ReceiverRecovery::TypedParam));
                            bindings += 1;
                        }
                    }
                    Language::Go if p.kind() == "parameter_declaration" => {
                        let Some(ty) = p.child_by_field_name("type") else { continue };
                        let mut pc = p.walk();
                        for ch in p.children(&mut pc) {
                            if ch.kind() == "identifier"
                                && ch.utf8_text(src).ok() == Some(receiver)
                            {
                                found = Some(self.node_text(&ty).to_string());
                                bindings += 1;
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        // (2) body bindings: let/:= ; count ALL bindings of `receiver` (shadow bail)
        fn walk_bindings(
            pf: &ParsedFile,
            node: Node<'_>,
            receiver: &str,
            call_line: usize,
            found: &mut Option<(String, ReceiverRecovery)>,
            bindings: &mut usize,
        ) {
            let src = pf.source.as_bytes();
            if node.start_position().row + 1 > call_line {
                return; // bindings after the call cannot type this receiver (M6)
            }
            match node.kind() {
                "let_declaration" => {
                    if let Some(pat) = node.child_by_field_name("pattern") {
                        if pat.utf8_text(src).ok() == Some(receiver) {
                            *bindings += 1;
                            if let Some(ty) = node.child_by_field_name("type") {
                                *found = Some((
                                    pf.node_text(&ty).to_string(),
                                    ReceiverRecovery::ConstructorLocal,
                                ));
                            } else if let Some(val) = node.child_by_field_name("value") {
                                *found = constructor_type(pf, &val)
                                    .map(|t| (t, ReceiverRecovery::ConstructorLocal));
                            }
                        }
                    }
                }
                "short_var_declaration" => {
                    // Go: x := Type{...} / x := NewType(...)
                    if let (Some(left), Some(right)) =
                        (node.child_by_field_name("left"), node.child_by_field_name("right"))
                    {
                        if pf.node_text(&left).trim() == receiver {
                            *bindings += 1;
                            let mut rc = right.walk();
                            for ch in right.children(&mut rc) {
                                if let Some(t) = constructor_type(pf, &ch) {
                                    *found = Some((t, ReceiverRecovery::ConstructorLocal));
                                }
                            }
                        }
                    }
                }
                "assignment_expression" => {
                    if let Some(left) = node.child_by_field_name("left") {
                        if pf.node_text(&left) == receiver {
                            *bindings += 1; // reassignment ⇒ bail via count
                            *found = None;
                        }
                    }
                }
                _ => {}
            }
            let mut c = node.walk();
            for ch in node.children(&mut c) {
                walk_bindings(pf, ch, receiver, call_line, found, bindings);
            }
        }

        /// `Type::new(..)` / `Type::default(..)` / `Type{..}` / `NewType(..)` → "Type"
        fn constructor_type(pf: &ParsedFile, val: &Node<'_>) -> Option<String> {
            match val.kind() {
                "call_expression" => {
                    let f = val.child_by_field_name("function")?;
                    let text = pf.node_text(&f);
                    if let Some((ty, _last)) = text.rsplit_once("::") {
                        return Some(ty.rsplit("::").next().unwrap_or(ty).to_string());
                    }
                    // Go convention: NewFoo(...) → Foo (validated against the
                    // owner index by the caller; bare guess otherwise rejected)
                    text.strip_prefix("New").map(|s| s.to_string())
                }
                "struct_expression" | "composite_literal" => {
                    let t = val
                        .child_by_field_name("name")
                        .or_else(|| val.child_by_field_name("type"))?;
                    Some(pf.node_text(&t).to_string())
                }
                _ => None,
            }
        }

        walk_bindings(self, *func_node, receiver, call_line, &mut found, &mut bindings);
        if bindings > 1 {
            return None; // shadow/rebind bail (count = bindings at/before call_line)
        }
        found
    }
```

`src/call_graph.rs` Phase 2 — in **`build` AND `build_direct_subset`** (incremental
cache updates splice in `build_direct_subset` output; the two MUST recover
identically or full and incremental builds diverge — plan-review BLOCKER 1).
`build_skeleton` stays receiver-blind (scope-only). Extract the shared block as a
free fn `recover_receiver(parsed, func_node, recv_var, file_imports, q, line)` so
both builders call one implementation. After constructing each site, when the
qualifier is a simple identifier that is NOT self/this/cls, NOT the caller's
receiver var, and NOT an import alias:

```rust
                    for (callee_name, line, qualifier) in call_sites {
                        let recovered = qualifier.as_deref().and_then(|q| {
                            let simple = !q.is_empty()
                                && q.chars().all(|c| c.is_alphanumeric() || c == '_');
                            let is_kw = matches!(q, "self" | "this" | "cls");
                            let is_recv = recv_var.as_deref() == Some(q);
                            let is_import = file_imports_ref
                                .map(|m| m.contains_key(q))
                                .unwrap_or(false);
                            if simple && !is_kw && !is_recv && !is_import {
                                parsed
                                    .receiver_type_in_fn(&func_node, q, line)
                                    .map(|(t, how)| (crate::resolution::peel_type(&t), how))
                            } else {
                                None
                            }
                        });
                        let site = CallSite {
                            caller: caller_id.clone(),
                            callee_name,
                            line,
                            qualifier,
                            receiver_type: recovered.as_ref().map(|(t, _)| t.clone()),
                            receiver_recovery: recovered.as_ref().map(|(_, how)| *how),
                        };
```

Add an integration test pinning build/incremental equivalence: build a 2-file repo
fully, then `remove_files` one file + `merge(build_direct_subset(..))` and assert
the affected sites carry identical `receiver_type` values both ways.

(`recv_var` = `parsed.language.go_receiver_var(&func_node).map(|n| parsed.node_text(&n).to_string())` computed once per function above the loop; `file_imports_ref` = the imports map entry for this file, captured before the par_iter from the already-built `imports`. Adjust the closure to take `&imports`.)

`src/resolution.rs` — R6 step 1 (insert at the TOP of the R6 arm, before `method_ids`):

```rust
                // ---- R6 step 1: P6-lite recovered receiver ----
                if let Some(recv_ty) = site.receiver_type.as_deref() {
                    let recovered_kind = match site.receiver_recovery {
                        Some(crate::resolution::ReceiverRecovery::ConstructorLocal) => {
                            ResolutionKind::ConstructorLocal
                        }
                        _ => ResolutionKind::TypedParam,
                    };
                    return match self.owner_lookup(recv_ty, name) {
                        Some(mut r) => {
                            for c in &mut r {
                                if c.kind == ResolutionKind::QualifiedOwner {
                                    c.kind = recovered_kind;
                                }
                                // trait-CHA hits keep TraitCha (dyn Trait receivers)
                            }
                            ResolutionOutcome::hit(r)
                        }
                        // Recovered type with no (T, m) entry ⇒ provably external
                        // or wrong-name ⇒ drop (kills the Vec::truncate class).
                        None => ResolutionOutcome::dropped(DropReason::ExternalReceiver),
                    };
                }
```

`CallSite.receiver_recovery` (Task 4) carries the param-vs-local split, so
`p6_constructor_local_recovers` asserts `ResolutionKind::ConstructorLocal` exactly
as written in Step 1.

- [ ] **Step 4: Run** `cargo test --test integration resolution_test:: 2>&1 | tail -6` — all pass (NewFoo-guess validation: `owner_lookup(recv_ty, ..)` returning None drops — so a wrong `New`-strip guess is safe).

- [ ] **Step 5: fmt + full suite + harness + commit**

```bash
cargo fmt && cargo test 2>&1 | tail -3
cargo build --release && (cd eval && uv run tier-a --matrix-only --allow-stale-sut)
git add -A && git commit -m "feat(s3): P6-lite receiver recovery — typed params, constructor locals, peel list, shadow bail, external drop"
```

---

### Task 9: Thread sites through the 4 traversal helpers; pin the scope path

**Files:**
- Modify: `src/call_graph.rs:857,883,925,992` (helpers), `src/cpg/context.rs:351-358` (comment contract)
- Modify: `tests/integration/call_graph_test.rs`

- [ ] **Step 1: Write failing test**

```rust
#[test]
fn traversal_helpers_respect_ladder_not_bare_names() {
    use prism::languages::Language::Rust;
    let mut files = std::collections::BTreeMap::new();
    for (p, s) in [
        ("a.rs", "impl A {\n    fn poll(&self) {}\n}\n"),
        ("b.rs", "impl B {\n    fn poll(&self) {}\n}\n"),
        ("m.rs", "fn drive(x: Unknown) {\n    x.poll();\n}\n"),
    ] {
        files.insert(p.to_string(), prism::ast::ParsedFile::parse(p, s, Rust).unwrap());
    }
    let cg = prism::call_graph::CallGraph::build(&files);
    // callees_of must NOT fan `drive` out to both polls (multi-owner drop)
    let callees = cg.callees_of("drive", "m.rs", 2);
    assert!(callees.is_empty(), "got {callees:?}");
    // resolve_callers must NOT report drive as a caller of A::poll
    let callers = cg.resolve_callers("poll", "a.rs");
    assert!(callers.is_empty(), "got {callers:?}");
}
```

- [ ] **Step 2: Run** — FAIL (both fan out today).

- [ ] **Step 3: Implement**

In each helper, replace name-only resolution with the ladder over the site in hand:

- `callers_of_in_file` (`:857`): **both branches** must resolve per-site (plan-review MAJOR: the no-target branch otherwise stays name-indexed and collision-prone). Replace the `if let Some(tf) = target_file { ... }` block with:

```rust
                    let resolved = self.resolve_call_site(site);
                    let hit = match target_file {
                        Some(tf) => resolved.iter().any(|c| c.target.file == tf),
                        None => !resolved.is_empty(),
                    };
                    if !hit {
                        continue;
                    }
```
- `resolve_callers` (`:883`): same substitution inside the filter.
- `callees_of` (`:925`): `let callee_ids = self.resolve_call_site(site); for c in callee_ids { queue.push_back((c.target.clone(), depth + 1)); }`
- `dfs_cycles` (`:992`): same shape.

`src/cpg/context.rs:356` — leave `resolve_callees` (name-only) in `compute_scope`, adding the contract comment:

```rust
                // S3 contract: scope computation deliberately uses the
                // recall-biased name-only resolver — scope is a superset
                // heuristic, not a truth claim (spec §3.4). Edge creation
                // (cpg/build.rs Step 5) uses the precision ladder.
```

- [ ] **Step 4: Run** `cargo test --test integration 2>&1 | tail -4` — pass, including pre-existing call_graph/scoped_cpg tests. Pre-existing tests that pinned fan-out behavior will fail — inspect each: if the test certified a *false* edge (cross-receiver method bind), update the assertion and note it in the commit message; if it certified a true edge that the ladder now misses, STOP — that is a regression to fix, not re-bless.

- [ ] **Step 5: fmt + full suite + harness + commit**

```bash
cargo fmt && cargo test 2>&1 | tail -3
cargo build --release && (cd eval && uv run tier-a --matrix-only --allow-stale-sut)
git add -A && git commit -m "feat(s3): traversal helpers resolve per-site via ladder; scope path pinned recall-biased"
```

---

### Task 10: CPG Step 5/5b on the ladder; 5b receiver-binding rule

**Files:**
- Modify: `src/cpg/build.rs:305-345` (Step 5), `:327-401` (Step 5b)
- Modify: `tests/ast/cpg_test.rs`

- [ ] **Step 1: Write failing test**

```rust
// tests/ast/cpg_test.rs (append)
#[test]
fn cpg_call_edges_exclude_multi_owner_drops_include_demoted() {
    use prism::languages::Language::Rust;
    let mut files = std::collections::BTreeMap::new();
    for (p, s) in [
        ("a.rs", "impl A {\n    fn poll(&self) {}\n}\nimpl OnlyOwner {\n    fn frob(&self) {}\n}\n"),
        ("b.rs", "impl B {\n    fn poll(&self) {}\n}\n"),
        ("m.rs", "fn drive(x: Unknown, y: Unknown2) {\n    x.poll();\n    y.frob();\n}\n"),
    ] {
        files.insert(p.to_string(), prism::ast::ParsedFile::parse(p, s, Rust).unwrap());
    }
    let cpg = prism::cpg::CodePropertyGraph::build(&files, None);
    let drive_callees = cpg.callees_of("drive", "m.rs", 1);
    let names: Vec<&str> = drive_callees.iter().map(|(f, _)| f.name.as_str()).collect();
    assert!(!names.contains(&"poll"), "multi-owner dropped: {names:?}");
    assert!(names.contains(&"frob"), "single-owner NameOnly included: {names:?}");
}
```

(Adapt the build call to the actual `CodePropertyGraph` constructor used in existing `cpg_test.rs` — copy its setup helper.)

- [ ] **Step 2: Run** — FAIL (poll edges exist).

- [ ] **Step 3: Implement**

Step 5 (`src/cpg/build.rs:311-316`):

```rust
            for site in sites {
                // S3: Exact + NameOnly included; drops excluded (spec §1).
                for resolved in cg.resolve_call_site(site) {
                    let callee_id = resolved.target;
                    let callee_key = (callee_id.file.clone(), callee_id.name.clone());
                    if let Some(&callee_idx) = func_index.get(&callee_key) {
                        graph.add_edge(caller_idx, callee_idx, CpgEdge::Call);
                        graph.add_edge(callee_idx, caller_idx, CpgEdge::Return);
                    }
                }
            }
```

Step 5b: same substitution for its `resolve_callees_qualified` call (`:330-334`); the rest of 5b is unchanged except the **receiver-binding rule**. 5b gets callee params from the callee's `FunctionInfo.param_names` (the "first name match wins — pinned-until-S2" lookup at `cpg/build.rs:349-357` — there is NO `callee_node` in scope; plan-review BLOCKER 3). Apply the skip on that existing path:

```rust
                    // S3 (spec §3.3): the receiver never binds to a parameter;
                    // Python declares self/cls explicitly — skip it so explicit
                    // args align with the remaining params.
                    let is_python = callee_parsed.language == crate::languages::Language::Python;
                    let param_names: &[String] = match info.param_names.first().map(String::as_str) {
                        Some("self") | Some("cls") if is_python => &info.param_names[1..],
                        _ => &info.param_names[..],
                    };
```

(`info` = the matched `FunctionInfo`; adapt the binding to however `:349-357` names it. Add a focused Python test in `tests/ast/cpg_test.rs`: `obj.method(x)` with `def method(self, a)` produces an arg-edge to `a`, not `self`.)

After this lands, grep for remaining `resolve_callees_qualified` callers: `rg -n "resolve_callees_qualified" src/` — should be only its own definition + `navigation/call_resolve.rs` (Task 11 removes that). Delete the method once callers reach zero (Task 11 Step 5).

- [ ] **Step 4: Run** `cargo test --test ast cpg_test:: 2>&1 | tail -4` — pass; full `cargo test` — triage drift exactly as in Task 9 Step 4 (false-edge assertions re-blessed with a note; true-edge losses are STOP-regressions).

- [ ] **Step 5: fmt + harness + commit**

```bash
cargo fmt && cargo test 2>&1 | tail -3
cargo build --release && (cd eval && uv run tier-a --matrix-only --allow-stale-sut)
git add -A && git commit -m "feat(s3): CPG Step 5/5b on the resolution ladder; Python self/cls receiver-binding rule"
```

---

### Task 11: Navigation — scores, `Reason::Resolution`, `Collision` warning, module-graph max-score

**Files:**
- Modify: `src/navigation/types.rs` (Reason variant), `src/navigation/call_resolve.rs` (adapter), `src/navigation/queries.rs` (callers/callees/direct_*), `src/navigation/module_graph.rs`
- Modify: `tests/navigation/callers_test.rs`, `tests/navigation/callees_test.rs`, `tests/navigation/module_graph_test.rs`

- [ ] **Step 1: Write failing tests** (use the existing test-session helpers in `tests/navigation/` — copy the setup pattern from `callers_test.rs`)

```rust
#[test]
fn demoted_callee_scores_0_6_with_resolution_reason() {
    // fixture: x.frobnicate() single-owner cross-file (R6SingleOwner)
    // assert: item.score == 0.6, item.why contains Resolution { kind: "r6_single_owner" }
}

#[test]
fn exact_callee_scores_1_0() { /* Engine::start fixture; score == 1.0, kind "qualified_owner" */ }

#[test]
fn callers_query_emits_collision_warning_for_dropped_sites() {
    // fixture: A::poll / B::poll / x.poll() — callers of A::poll returns no x-site item
    // but Evidence.warnings contains WarningKind::Collision with "1" in the message.
}
```

Write these as real tests against the session helper (the existing files show the exact construction — `NavigationSession` over a temp repo dir); assert on the serialized Evidence.

- [ ] **Step 2: Run** `cargo test --test navigation 2>&1 | tail -5` — FAIL.

- [ ] **Step 3: Implement**

`types.rs` — add to `Reason`:

```rust
    Resolution {
        kind: &'static str,
    },
```

`call_resolve.rs` — **migrate, don't rename-and-strand** (plan-review BLOCKER 5): `resolve_callees_nav` is DELETED and every caller migrated in this task — enumerate with `rg -n "resolve_callees_nav" src/ tests/` (expected: `queries.rs:144`, `queries.rs:231`, `module_graph.rs` `collect_module_edges`, plus `tests/navigation/scoped_calls_test.rs` and any sibling tests — update each test to the new API). The replacement adapter returns full metadata (plan-review MAJOR: helper return types must carry confidence/kind):

```rust
use crate::resolution::{DropReason, ResolvedCallee};

/// One resolved nav call edge with everything queries need for
/// score + Reason::Resolution (no metadata discarded between layers).
pub struct NavCallEdge<'a> {
    pub target: &'a FunctionId,
    pub call_site_line: usize,
    pub qualifier: Option<String>,
    pub confidence: crate::resolution::ResolutionConfidence,
    pub kind: crate::resolution::ResolutionKind,
}

pub fn resolve_site_nav<'a>(cg: &'a CallGraph, site: &'a CallSite) -> Vec<NavCallEdge<'a>> {
    cg.resolve_call_site(site)
        .into_iter()
        .map(|r: ResolvedCallee<'a>| NavCallEdge {
            target: r.target,
            call_site_line: site.line,
            qualifier: site.qualifier.clone(),
            confidence: r.confidence,
            kind: r.kind,
        })
        .collect()
}

/// Collision-dropped same-name receiver sites for a seed name
/// (callers/ego warnings; counts ONLY MultiOwnerCollision — not external
/// imports, external receivers, or unknown names).
pub fn collision_dropped_sites(cg: &CallGraph, seed_name: &str) -> usize {
    scoped_caller_sites(cg, seed_name)
        .into_iter()
        .filter(|site| {
            cg.resolve_call_site_full(site).drop == Some(DropReason::MultiOwnerCollision)
        })
        .count()
}
```

Keep `scoped_caller_sites` unchanged. Both `queries.rs` loops (`:144`, `:231`) already iterate `site` — pass it to `resolve_site_nav` instead of `(name, file, qualifier)` triples; `direct_callers`/`direct_callees` change return type from `Vec<(FunctionId, usize)>` to `Vec<NavCallEdge>` so confidence/kind survive to item construction. Score mapping where items are constructed (`:190`, `:305` and the direct-item constructors):

```rust
fn confidence_score(c: crate::resolution::ResolutionConfidence) -> f32 {
    match c {
        crate::resolution::ResolutionConfidence::Exact => 1.0,
        crate::resolution::ResolutionConfidence::NameOnly => 0.6, // spec §3.4: avoids hop-decay collision at 0.5
    }
}
// item score: confidence_score(resolved.confidence) / (1.0 + hop as f32)
// item why: push Reason::Resolution { kind: resolved.kind.as_str() } alongside Calls/CalledBy
```

Collision warning in **both** the callers query and `ego_graph` (spec §2.5 names callers/ego; plan-review MAJOR) — after assembling items/edges for the seed:

```rust
    let dropped = crate::navigation::call_resolve::collision_dropped_sites(cg, &target.name);
    if dropped > 0 {
        warnings.push(Warning {
            kind: WarningKind::Collision,
            message: format!(
                "{dropped} same-name receiver call site(s) with unknown receiver type across multiple owner types; not attributed as callers"
            ),
            location: None,
        });
    }
```

(In `ego_graph`, emit it when the seed's incoming Call edges are being collected — same helper, same message.)

`module_graph.rs` `collect_module_edges`: the tuple reasons can't carry score/kind (plan-review MAJOR) — replace with a struct:

```rust
pub struct ModuleCallReason {
    pub callee: String,
    pub call_site_line: usize,
    pub qualifier: Option<String>,
    pub score: f32,                 // confidence_score(confidence), no hop decay here
    pub kind: &'static str,         // ResolutionKind::as_str()
}
```

Aggregate per `(source, target)` file pair with `f32::max` over reason scores; emit the max as the item score (replaces the constant at `:119`) and push `Reason::Resolution { kind }` alongside each `Reason::Calls`.

- [ ] **Step 4: Run** `cargo test --test navigation 2>&1 | tail -4`; then `cargo test --test cli nav_compat_test:: 2>&1 | tail -4`. nav_compat pins output shapes — score changes WILL drift it; re-bless per the golden discipline (every drifted line traces to a demotion/drop/new-Exact). Full `cargo test`.

- [ ] **Step 5: Delete dead code + fmt + harness + commit**

**`resolve_callees` currently delegates to `resolve_callees_qualified(None)`** (plan-review BLOCKER 4) — before deleting the latter, re-home the recall-biased body: move the name-lookup + static-linkage filtering (the current `resolve_callees_qualified` minus the qualifier block) INTO `resolve_callees` directly, with the doc comment "recall-biased name+static resolver — scope computation and Phase-3 indirect resolution ONLY; edge creation uses resolve_call_site". Then `rg -n "resolve_callees_qualified" src/ tests/` → migrate any remaining test callers to `resolve_call_site` or `resolve_callees` as semantically appropriate → delete the definition.

```bash
cargo fmt && cargo test 2>&1 | tail -3
cargo build --release && (cd eval && uv run tier-a --matrix-only --allow-stale-sut)
git add -A && git commit -m "feat(s3): nav confidence scores (1.0/0.6), Reason::Resolution, Collision warnings, module max-score"
```

---

### Task 12: `prism nav call-stats` telemetry subcommand

**Files:**
- Modify: `src/main.rs` (NavQuery enum + dispatch), `src/navigation/mod.rs` or `queries.rs` (stats fn)
- Modify: `tests/cli/nav_compat_test.rs` or new `tests/cli/call_stats_test.rs` (+ `mod` line in `tests/cli/main.rs`)

- [ ] **Step 1: Write failing test**

```rust
// tests/cli/call_stats_test.rs
use assert_cmd::Command;

#[test]
fn call_stats_reports_kind_counts_and_drops() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.rs"), "impl A {\n    fn poll(&self) {}\n}\n").unwrap();
    std::fs::write(dir.path().join("b.rs"), "impl B {\n    fn poll(&self) {}\n}\n").unwrap();
    std::fs::write(dir.path().join("m.rs"), "fn drive(x: U) {\n    x.poll();\n}\n").unwrap();
    let out = Command::cargo_bin("prism")
        .unwrap()
        .args(["nav", "--no-cache", "call-stats", "--repo"])
        .arg(dir.path())
        .output()
        .unwrap();
    assert!(out.status.success());
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["dropped_multi_owner"], 1);
    assert!(v["kinds"].is_object());
}
```

- [ ] **Step 2: Run** — FAIL (unknown subcommand).

- [ ] **Step 3: Implement** — follow the existing `NavQuery` clap pattern in `src/main.rs:230-301`: add a `CallStats { repo: PathBuf }` variant; handler loads the repo (same loader as repo-map), then:

```rust
pub fn call_stats(cg: &CallGraph) -> serde_json::Value {
    use crate::resolution::{DropReason, ResolutionConfidence};
    use std::collections::BTreeMap;
    let mut kinds: BTreeMap<&'static str, usize> = BTreeMap::new();
    let mut demoted = 0usize;
    let mut total = 0usize;
    let (mut multi, mut external, mut import_ext, mut unknown) = (0usize, 0usize, 0usize, 0usize);
    for sites in cg.calls.values() {
        for site in sites {
            total += 1;
            let out = cg.resolve_call_site_full(site);
            match out.drop {
                Some(DropReason::MultiOwnerCollision) => multi += 1,
                Some(DropReason::ExternalReceiver) => external += 1,
                Some(DropReason::ImportExternal) => import_ext += 1,
                Some(DropReason::UnknownName) => unknown += 1,
                None => {}
            }
            for c in &out.resolved {
                *kinds.entry(c.kind.as_str()).or_default() += 1;
                if c.confidence == ResolutionConfidence::NameOnly {
                    demoted += 1;
                }
            }
        }
    }
    serde_json::json!({
        "total_call_sites": total,
        "kinds": kinds,
        "demoted_edges": demoted,
        "dropped_multi_owner": multi,
        "dropped_external_receiver": external,
        "dropped_import_external": import_ext,
        "unresolved_unknown_name": unknown,
    })
}
```

(The split per `DropReason` is plan-review-mandated: a single drop counter would
conflate collision drops with ordinary unresolved calls and misreport the §5.4
telemetry.)

- [ ] **Step 4: Run** `cargo test --test cli call_stats_test:: 2>&1 | tail -4` — pass.

- [ ] **Step 5: fmt + full suite + commit**

```bash
cargo fmt && cargo test 2>&1 | tail -3
git add -A && git commit -m "feat(s3): nav call-stats telemetry (resolution kind counts, demotions, drops)"
```

---

### Task 13: Capability matrix reconciliation + R6 fixtures

**Files:**
- Modify: `eval/fixtures/rust/type_method_qualified/expected.toml` (+ any other flips)
- Create (only if the listed shapes are missing — check `eval/fixtures/{rust,go}/` first; several exist): collision/demote/recovery fixtures
- No prism source changes.

- [ ] **Step 1: Run the matrix to discover flips**

```bash
cargo build --release && cd eval && uv run tier-a --matrix-only --allow-stale-sut; cd ..
```

Expected: exit 1 with flip-candidates listed — `type_method_qualified` (and possibly `method_cross_file_type_ne_stem`, `trait_static_dispatch`, `common_name_collision`, `receiver_method_cross_file_stem_eq`) now behave differently than their pinned status.

- [ ] **Step 2: Update statuses** — for each `known_fail → pass` flip, edit the fixture's `expected.toml` `status` field to `"pass"`. For any `ok → fail` flip: **STOP — regression**; fix code, do not edit the status.

- [ ] **Step 3: Add missing R6-policy fixtures** (mirror the `expected.toml` schema shown in `eval/fixtures/rust/type_method_qualified/`; one dir per capability, status `"pass"` with the new resolver):
  - `rust/r6_multi_owner_drop` — two owners of `poll`, unknown receiver; the empty-expectation form is `[expect]` with `callers = []` and `exact = true` (an omitted callers key fails the loader; `[[expect.callers]]` without file/line is invalid — plan-review M14; confirm against `eval/tests/test_matrix.py` before authoring).
  - `rust/r6_single_owner_demote` — one owner, unknown receiver; callers include the site.
  - `rust/p6_typed_param_recovery` — collision + typed param; callers attribute ONLY the typed site.
  - `rust/p6_shadow_bail` — rebinding; no attribution.
  - `go/r6_receiver_var_exact` — `t.M()` inside a method; resolves to own type among collisions.
  - `go/import_package_path` — import-qualified call to a package whose dir ≠ file stem.

- [ ] **Step 4: Re-run matrix** — `cd eval && uv run tier-a --matrix-only --allow-stale-sut` → exit 0, new fixtures pass, statuses reconciled.

- [ ] **Step 5: Commit**

```bash
git add eval/fixtures && git commit -m "feat(s3): matrix flips reconciled (type_method_qualified pass) + R6/P6 policy fixtures"
```

---

### Task 14: Golden re-bless sweep + full verification

**Files:** whatever the suite surfaces (tests/cli, tests/lang, tests/algo goldens).

- [ ] **Step 1:** `cargo fmt --check && cargo test 2>&1 | tail -20` — enumerate every remaining failure.
- [ ] **Step 2:** For each failure, classify per the spec §5.3 discipline: (a) removed false edge / new qualified resolution / Lua keying → update assertion, record one line per file in the commit body; (b) anything else → fix code. No unexplained drift.
- [ ] **Step 3:** `cargo test --features mcp 2>&1 | tail -3` (MCP build must stay green).
- [ ] **Step 4:** `cargo build --release && cd eval && uv run tier-a --quick --allow-stale-sut` (needs rust-analyzer; minutes). Expected: prism-corpus strata move in the gated directions (C-method caller FPs down, U-callee recall up); paste the summary into the PR draft.
- [ ] **Step 5: Commit**

```bash
git add -A && git commit -m "test(s3): golden re-bless — each drift traced to removed false edge / new qualified resolution / Lua keying"
```

---

### Task 15: Docs + PR

- [ ] **Step 1:** Update `CLAUDE.md`'s "Navigation call resolution covers…" paragraph (lines ~166-172): rewrite to name the R1–R7 ladder, owner index, P6-lite recovery, confidence scores, and the remaining gaps (field receivers → S3.1 candidate; full dispatch → Phase-IP).
- [ ] **Step 2:** Run the telemetry: `./target/release/prism nav call-stats --repo . | tee /tmp/s3-call-stats.json` — include in the PR body (spec §5.4 gate: residue counts named).
- [ ] **Step 3:** Open the PR: branch `s3-precision`, body = before/after matrix summary + `--quick` strata movements + call-stats JSON + the golden-drift trace list + the §5.4 gate checklist with measured values. End with the standard Claude Code footer.
- [ ] **Step 4:** Request the second-opinion review per workflow preference (a2a-bridge codex review of the FULL branch diff vs main — instruct "review the entire branch diff `git diff main..HEAD` in depth").
- [ ] **Step 5:** **Human-triggered acceptance:** full 5-corpus rerun (`uv run tier-a --corpus all --date 2026-06-XX`) — owner runs/authorizes; gates per spec §5.4; `docs/eval/tier-a/baseline.md` updated deliberately after acceptance.

---

## Self-review (performed at plan-write time)

- **Spec coverage:** §2.1 owners (T1–T3), OwnerKey (T1 `owner_key`), dual-key + trait demotion (T4/T5), §2.2 CallTarget shapes (T5 ::-parsing; dot-qualifier classes T6/T7; the contract is realized in the ladder's input handling rather than a separate enum — the `CallSite{qualifier, receiver_type}` + name-with-`::` carries every §2.2 shape), R1–R7 (T5–T8), §2.3 P6-lite+peel+bail+external-drop (T8), residue (T7), §2.5 visibility (T11), §3.2 four helpers (T9), §3.3 CPG rule + 5b binding (T10), §3.4 scope pin (T9) + scores/Reason/max-score (T11), §3.5 cache v4 + mutators (T4), §4 same-file-overload exclusion (inherent — func_index untouched), §5 fixtures (T13), gates/telemetry (T12/T15), Lua keying (T3). **Gap check:** spec's `Reason::Resolution` kind list includes `stem_single/stem_multi` — covered; `import_qualified` exact — covered.
- **Placeholder scan (rev 2):** the TypedParam/ConstructorLocal collapse was rejected by plan review — `CallSite.receiver_recovery` now carries the split end-to-end. Task 11 Step 1 tests are summarized in comments where the existing session-helper shape must be copied from neighboring files — the assertions to write are stated concretely.
- **Type consistency:** `resolve_call_site(&self, site: &CallSite) -> Vec<ResolvedCallee>` used identically in T5–T12; `ResolutionKind::as_str` snake_case values match spec §3.4's list (plus `static_linkage`, an existing behavior given a name); `confidence_score` 1.0/0.6 matches spec; `CACHE_VERSION = 4` referenced once.
- **Order safety:** every task leaves `cargo test` green (T5–T7 build the ladder behind new tests while old resolver paths still serve consumers until T9–T11 switch them; the brief dual-path window is intentional and tested on both sides).
