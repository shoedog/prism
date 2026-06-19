# Phase-3 Slice-1a: In-repo method-chain receiver typing (#5) Implementation Plan (rev 3)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Status:** REV 3 PLAN. Rev 3 folds the round-2 review: AST-based method-chain decomposition, `StdWrapperPeel` fail-closed dispatch, `MethodKind` borrow-safe matching, let-init method chains, external-return rejection, corrected Task 5 test filters, corrected test-module locations, wrapper-peel/depth-cap/multi-argument tests, and AGENTS.md Tier-A commands.

**Goal:** type the receiver of in-repo Rust method-chain calls (`a.b().c()`, `a.b(1).c(2).d()`, and `let x = a.b(); x.c()`) so the final call resolves by receiver identity (Exact) instead of the `r6_single_owner` name guess.

**Architecture:** `RustReceiverTyper` already performs bounded recursive receiver typing in `src/resolution_receiver.rs:80`, with `type_of_expr` at `src/resolution_receiver.rs:106`, `MAX_RECEIVER_TYPE_DEPTH` at `src/resolution_receiver.rs:18`, and cycle guards in `TypeVisit` at `src/resolution_receiver.rs:35`. Rev 3 adds an AST path for call receivers using `ReceiverTypeCtx.receiver_expr` (`src/resolution_receiver.rs:25`). A Rust method call is a `call_expression` whose `function` field is a `field_expression`; that field expression has `value` (receiver) and `field` (method name). This is verified in tree-sitter-rust 0.24.2 at `/Users/wesleyjinks/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tree-sitter-rust-0.24.2/grammar.js:1111` and `/Users/wesleyjinks/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tree-sitter-rust-0.24.2/grammar.js:1332`, and matches prism's existing extraction helpers at `src/languages/mod.rs:701` and `src/ast.rs:3522`. The chain typer recurses on the receiver node, dispatches the intermediate method on `methods_by_scope` (`src/call_graph.rs:168`), and propagates only a single in-repo inherent return from `return_types` (`src/call_graph.rs:185`). External/Bare/StdWrapperPeel/non-unique/trait/External-return cases return `None` and leave today's residue unchanged.

**Design-of-record:** `docs/superpowers/specs/2026-06-18-prism-rust-receiver-typing-phase3-design.md` rev 5, especially §4.1, §4.2, §4.3, §5, and §6.

**Recall-safety invariant:** Slice-1a only adds a proven `ReceiverTypeKey::InRepo` outcome where the receiver is `None` today. It does not materialize external outcomes, does not promote `Bare`, does not route through `StdWrapperPeel`, and does not remove existing edges.

**Memoization (spec §4.1) is deferred:** the 1a receiver walk is linear and depth-capped (`MAX_RECEIVER_TYPE_DEPTH`=4) with the `TypeVisit` cycle guard, so per-call-site memoization is not needed for correctness or cost in 1a; add it only if profiling later shows repeated sub-walks. Not an omission.

**Test locations:** receiver-typer unit tests go in `src/resolution_receiver/tests.rs`; call-graph post-pass unit tests go in the existing test module in `src/call_graph.rs`; integration tests go in `tests/integration/resolution_test.rs`.

---

## File structure

- **Modify `src/resolution_receiver.rs`**: add AST method-call decomposition, AST-aware `type_of_node`, node-based chain typing, borrow-safe single-Exact dispatch, and let-init call routing.
- **Modify `src/resolution_receiver/tests.rs`**: AST decomposition, dispatch, depth-cap, and local-cycle tests.
- **Modify `tests/integration/resolution_test.rs`**: positive chain tests and fail-closed negatives.
- **Modify `src/call_graph.rs`**: parallelize `rematerialize_rust_receiver_keys` at `src/call_graph.rs:1123` with deterministic sorted apply.
- **Create three `eval/fixtures/rust/*` directories**: `chain_in_repo_exact`, `external_chain_unchanged`, `inrepo_then_external_unchanged`.
- **Do not touch source during plan revision.** Implementation tasks below are for the later code pass.

---

## Task 1: AST method-call decomposition

Add a private AST helper that decomposes a Rust method-call receiver node into `(receiver_node, method_name, arg_count)`. This replaces the rev-2 string splitter. The helper must accept only a `call_expression` whose `function` is a `field_expression` with `value` and a `field_identifier`; it must count arguments from the `arguments` node using the same named-child rule used by `src/ast.rs:3561`.

**Files:**
- Modify: `src/resolution_receiver.rs` near `receiver_expr_text` at `src/resolution_receiver.rs:365`
- Modify: `src/resolution_receiver/tests.rs`

- [ ] **Step 1: Write the failing test**

Add this complete test to `src/resolution_receiver/tests.rs`:

```rust
#[test]
fn method_call_parts_decomposes_nested_arg_chain_from_ast() {
    let files = files(&[(
        "main.rs",
        "pub struct A;\n\
         pub fn drive(a: A) { a.b(1).c(2, 3).d(); }\n",
    )]);
    let parsed = files.get("main.rs").expect("parsed file");
    let func_node = parsed
        .all_functions()
        .into_iter()
        .find(|node| {
            parsed
                .language
                .function_name(node)
                .is_some_and(|name| parsed.node_text(&name) == "drive")
        })
        .expect("drive function");
    let all_lines: BTreeSet<usize> = (1..=3).collect();
    let (_, _, _, _, _, receiver_expr, _, _) = parsed
        .function_calls_with_qualifier_and_spans_on_lines(&func_node, &all_lines)
        .into_iter()
        .find(|(name, _, _, _, _, _, _, _)| name == "d")
        .expect("d call");
    let c_call = receiver_expr.expect("receiver for d is the c call");
    assert_eq!(c_call.kind(), "call_expression");

    let c = super::method_call_parts(parsed, c_call).expect("c method call");
    assert_eq!(c.method, "c");
    assert_eq!(c.arg_count, 2);
    assert_eq!(parsed.node_text(&c.receiver), "a.b(1)");

    let b = super::method_call_parts(parsed, c.receiver).expect("b method call");
    assert_eq!(b.method, "b");
    assert_eq!(b.arg_count, 1);
    assert_eq!(parsed.node_text(&b.receiver), "a");
}
```

- [ ] **Step 2: Run it to confirm it fails**

Run:

```bash
cargo test --lib method_call_parts_decomposes_nested_arg_chain_from_ast
```

Expected output contains:

```text
error[E0425]: cannot find function `method_call_parts` in module `super`
```

- [ ] **Step 3: Implement the AST helper**

In `src/resolution_receiver.rs`, add `MethodCallParts`, `method_call_parts`, and `rust_arg_count` near `src/resolution_receiver.rs:365`. This is the complete code to add:

```rust
#[derive(Clone, Copy)]
struct MethodCallParts<'a> {
    receiver: tree_sitter::Node<'a>,
    method: &'a str,
    arg_count: usize,
}

fn method_call_parts<'a>(
    parsed: &'a ParsedFile,
    call: tree_sitter::Node<'a>,
) -> Option<MethodCallParts<'a>> {
    if call.kind() != "call_expression" {
        return None;
    }
    let function = call.child_by_field_name("function")?;
    if function.kind() != "field_expression" {
        return None;
    }
    let receiver = function.child_by_field_name("value")?;
    let field = function.child_by_field_name("field")?;
    if field.kind() != "field_identifier" {
        return None;
    }
    let method = parsed.node_text(&field).trim();
    if !is_simple_ident(method) {
        return None;
    }
    let arguments = call.child_by_field_name("arguments")?;
    Some(MethodCallParts {
        receiver,
        method,
        arg_count: rust_arg_count(arguments),
    })
}

fn rust_arg_count(arguments: tree_sitter::Node<'_>) -> usize {
    let mut count = 0usize;
    let mut cursor = arguments.walk();
    for child in arguments.children(&mut cursor) {
        if child.is_named() {
            count += 1;
        }
    }
    count
}
```

- [ ] **Step 4: Run it to confirm it passes**

Run:

```bash
cargo test --lib method_call_parts_decomposes_nested_arg_chain_from_ast
```

Expected output contains:

```text
test method_call_parts_decomposes_nested_arg_chain_from_ast ... ok
test result: ok.
```

- [ ] **Step 5: Commit**

```bash
git add src/resolution_receiver.rs src/resolution_receiver/tests.rs
git commit -m "feat(rust-recv): decompose method chains from AST"
```

---

## Task 2: single-Exact intermediate dispatch

Add a helper that mirrors `combine_kind` at `src/resolution.rs:494`: filter by `has_self`, arity, and spread; require exactly one kept candidate; require `MethodKind::Inherent`; refuse `ReceiverRecovery::StdWrapperPeel` exactly like `src/resolution.rs:524`. `MethodKind` is `Clone`, not `Copy` (`src/call_graph.rs:100`), so match through a borrow.

**Files:**
- Modify: `src/resolution_receiver.rs`
- Modify: `src/resolution_receiver/tests.rs`

- [ ] **Step 1: Write the failing test**

Add this complete test to `src/resolution_receiver/tests.rs`:

```rust
#[test]
fn dispatch_method_single_exact_filters_kind_arity_self_and_wrapper_peel() {
    let files = files(&[(
        "lib.rs",
        "pub struct Inner;\n\
         impl Inner { pub fn step(&self, n: u8) -> Inner { Inner } pub fn assoc() -> Inner { Inner } }\n\
         pub struct Outer;\n\
         trait T { fn m(&self); }\n\
         impl Outer { fn m(&self) {} }\n\
         impl T for Outer { fn m(&self) {} }\n",
    )]);
    let cg = graph(&files);
    let inner = type_scope(&cg, "Inner");
    let outer = type_scope(&cg, "Outer");

    assert!(super::dispatch_method_single_exact(
        &cg,
        inner,
        "step",
        ReceiverRecovery::TypedParam,
        Some(1),
        false,
    )
    .is_some());
    assert_eq!(
        super::dispatch_method_single_exact(
            &cg,
            inner,
            "step",
            ReceiverRecovery::TypedParam,
            Some(0),
            false,
        ),
        None
    );
    assert_eq!(
        super::dispatch_method_single_exact(
            &cg,
            inner,
            "assoc",
            ReceiverRecovery::TypedParam,
            Some(0),
            false,
        ),
        None
    );
    assert_eq!(
        super::dispatch_method_single_exact(
            &cg,
            outer,
            "m",
            ReceiverRecovery::TypedParam,
            Some(0),
            false,
        ),
        None
    );
    assert_eq!(
        super::dispatch_method_single_exact(
            &cg,
            inner,
            "step",
            ReceiverRecovery::StdWrapperPeel,
            Some(1),
            false,
        ),
        None
    );
}
```

- [ ] **Step 2: Run it to confirm it fails**

Run:

```bash
cargo test --lib dispatch_method_single_exact_filters_kind_arity_self_and_wrapper_peel
```

Expected output contains:

```text
error[E0425]: cannot find function `dispatch_method_single_exact` in module `super`
```

- [ ] **Step 3: Implement the dispatcher**

Change the import at `src/resolution_receiver.rs:2` from:

```rust
use crate::call_graph::{CallGraph, FunctionId};
```

to this complete import:

```rust
use crate::call_graph::{CallGraph, FunctionId, MethodKind};
```

Then add this complete helper near `return_type_from_call` at `src/resolution_receiver.rs:235`:

```rust
fn dispatch_method_single_exact(
    cg: &CallGraph,
    scope: ScopeId,
    method: &str,
    recovery: ReceiverRecovery,
    arg_count: Option<usize>,
    arg_spread: bool,
) -> Option<FunctionId> {
    if recovery == ReceiverRecovery::StdWrapperPeel {
        return None;
    }
    let cands = cg.methods_by_scope.get(&(scope, method.to_string()))?;
    let kept: Vec<&FunctionId> = cands
        .iter()
        .filter(|fid| {
            let Some(fact) = cg.method_facts.get(*fid) else {
                return false;
            };
            fact.has_self
                && !matches!(
                    arg_count,
                    Some(n) if !arg_spread && fact.arity_excl_self != n
                )
        })
        .collect();
    match kept.as_slice() {
        [fid]
            if matches!(
                cg.method_facts.get(*fid),
                Some(fact) if matches!(&fact.kind, MethodKind::Inherent)
            ) =>
        {
            Some((*fid).clone())
        }
        _ => None,
    }
}
```

- [ ] **Step 4: Run it to confirm it passes**

Run:

```bash
cargo test --lib dispatch_method_single_exact_filters_kind_arity_self_and_wrapper_peel
```

Expected output contains:

```text
test dispatch_method_single_exact_filters_kind_arity_self_and_wrapper_peel ... ok
test result: ok.
```

- [ ] **Step 5: Commit**

```bash
git add src/resolution_receiver.rs src/resolution_receiver/tests.rs
git commit -m "feat(rust-recv): dispatch chain intermediates only through single Exact methods"
```

---

## Task 3: route AST method chains through the recursive typer

Wire `ReceiverTypeCtx.receiver_expr` into a node-based recursion. Leaf cases may still use `type_of_expr`, but `call_expression` must use `method_call_parts` and recurse on the receiver node. `InitExpr::Call` at `src/resolution_receiver.rs:176` must recover the initializer AST when possible so `let x = b.cfg(); x.run()` uses the same chain typer. External intermediate returns remain rejected because `return_types` stores only in-repo `TypeKey`s today (`src/receiver_index.rs:255`).

**Files:**
- Modify: `src/resolution_receiver.rs`
- Modify: `tests/integration/resolution_test.rs`

- [ ] **Step 1: Write the failing integration tests**

Add these complete tests to `tests/integration/resolution_test.rs`:

```rust
#[test]
fn rust_receiver_chain_builder_new_cfg_arg_resolves_exact() {
    let (cg, _) = build_rust_complete(&[(
        "lib.rs",
        "pub struct Builder;\n\
         impl Builder {\n\
             pub fn new() -> Builder { Builder }\n\
             pub fn cfg(&self, n: u8) -> Builder { Builder }\n\
             pub fn run(&self) {}\n\
         }\n\
         fn drive() { Builder::new().cfg(1).run(); }\n",
    )]);
    let site = site_in(&cg, "drive", "run");
    assert!(
        site.receiver_outcome.is_some(),
        "chain receiver should be typed: {site:?}"
    );
    let resolved = cg.resolve_call_site(&site);
    assert_eq!(resolved.len(), 1);
    assert_eq!(resolved[0].target.name, "run");
    assert_eq!(resolved[0].confidence, ResolutionConfidence::Exact);
}

#[test]
fn rust_receiver_chain_nested_arg_intermediate_resolves_exact() {
    let (cg, _) = build_rust_complete(&[(
        "lib.rs",
        "pub struct Builder;\n\
         impl Builder {\n\
             pub fn cfg(&self, n: u8) -> Builder { Builder }\n\
             pub fn tune(&self, a: u8, b: u8) -> Builder { Builder }\n\
             pub fn run(&self) {}\n\
         }\n\
         fn drive(b: Builder) { b.cfg(1).tune(2, 3).run(); }\n",
    )]);
    let site = site_in(&cg, "drive", "run");
    assert!(
        site.receiver_outcome.is_some(),
        "nested arg-bearing chain should be typed: {site:?}"
    );
    let resolved = cg.resolve_call_site(&site);
    assert_eq!(resolved.len(), 1);
    assert_eq!(resolved[0].target.name, "run");
    assert_eq!(resolved[0].confidence, ResolutionConfidence::Exact);
}

#[test]
fn rust_receiver_let_bound_method_init_chain_resolves_exact() {
    let (cg, _) = build_rust_complete(&[(
        "lib.rs",
        "pub struct Builder;\n\
         impl Builder { pub fn cfg(&self) -> Builder { Builder } pub fn run(&self) {} }\n\
         fn drive(b: Builder) { let x = b.cfg(); x.run(); }\n",
    )]);
    let site = site_in(&cg, "drive", "run");
    assert!(
        site.receiver_outcome.is_some(),
        "let-bound method-init chain should type: {site:?}"
    );
    let resolved = cg.resolve_call_site(&site);
    assert_eq!(resolved.len(), 1);
    assert_eq!(resolved[0].target.name, "run");
    assert_eq!(resolved[0].confidence, ResolutionConfidence::Exact);
}
```

- [ ] **Step 2: Run them to confirm they fail**

Run:

```bash
cargo test --test integration rust_receiver_chain_builder_new_cfg_arg_resolves_exact
cargo test --test integration rust_receiver_chain_nested_arg_intermediate_resolves_exact
cargo test --test integration rust_receiver_let_bound_method_init_chain_resolves_exact
```

Expected output contains:

```text
test rust_receiver_chain_builder_new_cfg_arg_resolves_exact ... FAILED
test rust_receiver_chain_nested_arg_intermediate_resolves_exact ... FAILED
test rust_receiver_let_bound_method_init_chain_resolves_exact ... FAILED
```

- [ ] **Step 3: Implement AST-aware recursion**

At `src/resolution_receiver.rs:41`, replace `RecursionCtx` with this complete definition:

```rust
struct RecursionCtx<'a, 'v> {
    cg: &'a CallGraph,
    graph: &'a ScopeGraph,
    parsed: &'a ParsedFile,
    generic_params: BTreeSet<String>,
    file: FileId,
    at_byte: usize,
    module_scope: ScopeId,
    caller: &'a FunctionId,
    visit: &'v mut TypeVisit,
    depth: usize,
}
```

At `src/resolution_receiver.rs:53`, replace the `descend` implementation with this complete implementation:

```rust
impl<'a, 'v> RecursionCtx<'a, 'v> {
    fn descend<T>(&mut self, f: impl FnOnce(&mut RecursionCtx<'_, '_>) -> T) -> T {
        let mut child = RecursionCtx {
            cg: self.cg,
            graph: self.graph,
            parsed: self.parsed,
            generic_params: self.generic_params.clone(),
            file: self.file,
            at_byte: self.at_byte,
            module_scope: self.module_scope,
            caller: self.caller,
            visit: self.visit,
            depth: self.depth + 1,
        };
        f(&mut child)
    }
}
```

At `src/resolution_receiver.rs:80`, replace `type_of_receiver` with this complete method:

```rust
    pub fn type_of_receiver(&self, ctx: ReceiverTypeCtx<'_>) -> Option<ReceiverOutcome> {
        if !matches!(ctx.parsed.language, crate::languages::Language::Rust) {
            return None;
        }
        let file = self.graph.file_paths.get(&ctx.caller.file).copied()?;
        let module_scope = module_scope_for_byte(self.graph, file, ctx.fn_node.start_byte())?;
        let mut visit = TypeVisit::default();
        let mut recursion = RecursionCtx {
            cg: self.cg,
            graph: self.graph,
            parsed: ctx.parsed,
            generic_params: enclosing_generic_type_params(ctx.parsed, ctx.fn_node),
            file,
            at_byte: ctx.call_start_byte,
            module_scope,
            caller: ctx.caller,
            visit: &mut visit,
            depth: 0,
        };
        if let Some(node) = ctx.receiver_expr {
            type_of_node(&mut recursion, node)
        } else {
            type_of_expr(&mut recursion, ctx.qualifier?)
        }
    }
```

At `src/resolution_receiver.rs:106`, `type_of_expr` stays **UNCHANGED** (it remains the string-leaf resolver). It is reproduced below **for context only — do NOT paste or redefine it**; add the NEW `type_of_node` immediately after it:

```rust
// UNCHANGED — shown for context only; do NOT re-add this fn.
fn type_of_expr(ctx: &mut RecursionCtx<'_, '_>, expr: &str) -> Option<ReceiverOutcome> {
    if ctx.depth > MAX_RECEIVER_TYPE_DEPTH {
        return None;
    }
    let expr = expr.trim();
    if matches!(expr, "self" | "Self") {
        return self_receiver_type(ctx);
    }
    if let Some((base, field)) = split_field_expr(expr) {
        return ctx.descend(|ctx| field_type_from_base(ctx, base, field));
    }
    if let Some(function) = split_call_expr(expr) {
        return ctx.descend(|ctx| return_type_from_call(ctx, function));
    }
    if is_simple_ident(expr) {
        return ctx.descend(|ctx| local_receiver_type(ctx, expr));
    }
    None
}

fn type_of_node<'a>(
    ctx: &mut RecursionCtx<'a, '_>,
    node: tree_sitter::Node<'a>,
) -> Option<ReceiverOutcome> {
    if ctx.depth > MAX_RECEIVER_TYPE_DEPTH {
        return None;
    }
    match node.kind() {
        "call_expression" => {
            if let Some(call) = method_call_parts(ctx.parsed, node) {
                return ctx.descend(|ctx| method_chain_type(ctx, call));
            }
            let function = node
                .child_by_field_name("function")
                .or_else(|| node.child_by_field_name("name"))?;
            let function_text = ctx.parsed.node_text(&function).trim().to_string();
            ctx.descend(|ctx| return_type_from_call(ctx, &function_text))
        }
        "field_expression" => {
            let base = node.child_by_field_name("value")?;
            let field = node.child_by_field_name("field")?;
            let field_text = ctx.parsed.node_text(&field).trim().to_string();
            if !is_simple_ident(&field_text) {
                return None;
            }
            ctx.descend(|ctx| field_type_from_node_base(ctx, base, &field_text))
        }
        _ => {
            let expr = ctx.parsed.node_text(&node).trim().to_string();
            type_of_expr(ctx, &expr)
        }
    }
}
```

At `src/resolution_receiver.rs:153`, replace `type_from_local_fact` with this complete function:

```rust
fn type_from_local_fact(
    ctx: &mut RecursionCtx<'_, '_>,
    fact: &LocalFact,
) -> Option<ReceiverOutcome> {
    if matches!(fact.kind, BindingKind::Param | BindingKind::Let) {
        if let Some(annotation) = fact.annotation.as_deref() {
            let recovery = match fact.kind {
                BindingKind::Param => ReceiverRecovery::TypedParam,
                BindingKind::Let => ReceiverRecovery::TypedLet,
                BindingKind::Pattern => unreachable!(),
            };
            return type_from_annotation(ctx, annotation, recovery);
        }
    }
    match fact.init.as_ref()? {
        InitExpr::Ctor(expr) => {
            let ty = ctor_type_syntax(expr)?;
            type_from_annotation(ctx, ty, ReceiverRecovery::ConstructorLocal)
        }
        InitExpr::Field(expr) => {
            let (base, field) = split_field_expr(expr)?;
            ctx.descend(|ctx| field_type_from_base(ctx, base, field))
        }
        InitExpr::Call(expr) => {
            if let Some(node) = call_init_node_at(ctx) {
                return ctx.descend(|ctx| type_of_node(ctx, node));
            }
            let function = split_call_expr(expr)?;
            ctx.descend(|ctx| return_type_from_call(ctx, function))
        }
        InitExpr::Other => None,
    }
}
```

At `src/resolution_receiver.rs:222`, keep `field_type_from_base` and add this complete node-base sibling after it:

```rust
fn field_type_from_node_base<'a>(
    ctx: &mut RecursionCtx<'a, '_>,
    base: tree_sitter::Node<'a>,
    field: &str,
) -> Option<ReceiverOutcome> {
    let base_ty = type_of_node(ctx, base)?;
    let ReceiverTypeKey::InRepo(owner_scope) = base_ty.key else {
        return None;
    };
    let key = certain_index_type(ctx.cg.field_types.get(&(owner_scope, field.to_string()))?)?;
    outcome_for_type_key(ctx.graph, key, ReceiverRecovery::FieldTyped)
}
```

At `src/resolution_receiver.rs:235`, add this complete `method_chain_type` after `return_type_from_call`:

```rust
fn method_chain_type<'a>(
    ctx: &mut RecursionCtx<'a, '_>,
    call: MethodCallParts<'a>,
) -> Option<ReceiverOutcome> {
    let recv_ty = type_of_node(ctx, call.receiver)?;
    let recovery = recv_ty.recovery;
    let ReceiverTypeKey::InRepo(scope) = recv_ty.key else {
        return None;
    };
    let fid = dispatch_method_single_exact(
        ctx.cg,
        scope,
        call.method,
        recovery,
        Some(call.arg_count),
        false,
    )?;
    if !ctx.visit.fns.insert(fid.clone()) {
        return None;
    }
    let result = ctx.cg.return_types.get(&fid).and_then(|entries| {
        let key = certain_index_type(entries)?;
        if !matches!(&key, TypeKey::InRepo(_)) {
            return None;
        }
        outcome_for_type_key(ctx.graph, key, ReceiverRecovery::ReturnTyped)
    });
    ctx.visit.fns.remove(&fid);
    result
}
```

Near `src/resolution_receiver.rs:365`, add this complete helper for let-initializer AST recovery:

```rust
fn call_init_node_at<'a>(ctx: &RecursionCtx<'a, '_>) -> Option<tree_sitter::Node<'a>> {
    let mut node = ctx
        .parsed
        .tree
        .root_node()
        .descendant_for_byte_range(ctx.at_byte, ctx.at_byte.saturating_add(1))?;
    loop {
        if node.kind() == "let_declaration" {
            let value = node.child_by_field_name("value")?;
            return (value.kind() == "call_expression").then_some(value);
        }
        if node.kind() == "function_item" {
            return None;
        }
        node = node.parent()?;
    }
}
```

- [ ] **Step 4: Run the positive tests**

Run:

```bash
cargo test --test integration rust_receiver_chain_builder_new_cfg_arg_resolves_exact
cargo test --test integration rust_receiver_chain_nested_arg_intermediate_resolves_exact
cargo test --test integration rust_receiver_let_bound_method_init_chain_resolves_exact
```

Expected output contains:

```text
test rust_receiver_chain_builder_new_cfg_arg_resolves_exact ... ok
test rust_receiver_chain_nested_arg_intermediate_resolves_exact ... ok
test rust_receiver_let_bound_method_init_chain_resolves_exact ... ok
test result: ok.
```

- [ ] **Step 5: Run the receiver suite**

Run:

```bash
cargo test --test integration rust_receiver_outcome
cargo test --lib rust_receiver_typer
```

Expected output contains:

```text
test result: ok.
```

- [ ] **Step 6: Commit**

```bash
git add src/resolution_receiver.rs tests/integration/resolution_test.rs
git commit -m "feat(rust-recv): type in-repo method-chain receivers from AST"
```

---

## Task 4: recall-safety negatives and recursion guards

Pin fail-closed behavior for external, in-repo-then-external, trait, wrapper-peeled, depth-cap, and local-cycle paths. The integration negatives use at least two same-name decoys when a residue edge would otherwise mask a wrong typed edge.

**Files:**
- Modify: `tests/integration/resolution_test.rs`
- Modify: `src/resolution_receiver/tests.rs`

- [ ] **Step 1: Add integration fail-closed tests**

Add these complete tests to `tests/integration/resolution_test.rs`:

```rust
#[test]
fn rust_receiver_chain_external_intermediate_unchanged() {
    let (cg, _) = build_rust_complete(&[(
        "lib.rs",
        "pub struct LocalA; impl LocalA { fn count(&self) {} }\n\
         pub struct LocalB; impl LocalB { fn count(&self) {} }\n\
         fn drive(v: Vec<u8>) { v.iter().count(); }\n",
    )]);
    let site = site_in(&cg, "drive", "count");
    assert!(
        site.receiver_outcome.is_none(),
        "external chain must not type: {site:?}"
    );
    assert!(cg.resolve_call_site(&site).is_empty());
}

#[test]
fn rust_receiver_chain_inrepo_then_external_unchanged() {
    let (cg, _) = build_rust_complete(&[(
        "lib.rs",
        "pub struct Foo; impl Foo { pub fn ext(&self) -> String { String::new() } }\n\
         pub struct LocalA; impl LocalA { fn m(&self) {} }\n\
         pub struct LocalB; impl LocalB { fn m(&self) {} }\n\
         fn a() -> Foo { Foo }\n\
         fn drive() { a().ext().m(); }\n",
    )]);
    let site = site_in(&cg, "drive", "m");
    assert!(
        site.receiver_outcome.is_none(),
        "external return mid-chain must fail closed: {site:?}"
    );
    assert!(cg.resolve_call_site(&site).is_empty());
}

#[test]
fn rust_receiver_chain_trait_intermediate_unchanged() {
    let (cg, _) = build_rust_complete(&[(
        "lib.rs",
        "pub struct Foo;\n\
         pub trait T { fn t(&self) -> Foo; }\n\
         impl T for Foo { fn t(&self) -> Foo { Foo } }\n\
         impl Foo { fn m(&self) {} }\n\
         fn drive(f: Foo) { f.t().m(); }\n",
    )]);
    let site = site_in(&cg, "drive", "m");
    assert!(
        site.receiver_outcome.is_none(),
        "trait intermediate must fail closed: {site:?}"
    );
}

#[test]
fn rust_receiver_chain_wrapper_peel_intermediate_unchanged() {
    let (cg, _) = build_rust_complete(&[(
        "lib.rs",
        "use std::sync::Arc;\n\
         pub struct Foo; pub struct Next; pub struct Other;\n\
         impl Foo { pub fn foo(&self) -> Next { Next } }\n\
         impl Next { fn m(&self) {} }\n\
         impl Other { fn m(&self) {} }\n\
         fn drive(arc: Arc<Foo>) { arc.foo().m(); }\n",
    )]);
    let site = site_in(&cg, "drive", "m");
    assert!(
        site.receiver_outcome.is_none(),
        "StdWrapperPeel intermediate must fail closed: {site:?}"
    );
    assert!(
        cg.resolve_call_site(&site).is_empty(),
        "wrong typed edge through Arc peel would resolve exactly"
    );
}
```

- [ ] **Step 2: Add depth-cap and local-cycle unit tests**

Add these complete tests to `src/resolution_receiver/tests.rs`:

```rust
#[test]
fn rust_receiver_chain_depth_cap_fails_closed() {
    let files = files(&[(
        "main.rs",
        "pub struct B;\n\
         impl B { pub fn a(&self) -> B { B } pub fn run(&self) {} }\n\
         pub fn drive(b: B) { b.a().a().a().a().a().run(); }\n",
    )]);
    let cg = graph(&files);
    assert_eq!(super::MAX_RECEIVER_TYPE_DEPTH, 4);
    assert!(
        ty(&files, &cg, "drive", "run").is_none(),
        "chain deeper than MAX_RECEIVER_TYPE_DEPTH must fail closed"
    );
}

#[test]
fn rust_receiver_local_fact_cycle_fails_closed() {
    let files = files(&[(
        "main.rs",
        "pub struct Inner; impl Inner { pub fn run(&self) {} }\n\
         pub struct Holder { pub inner: Inner }\n\
         pub fn drive() { let a = a.inner; a.run(); }\n",
    )]);
    let cg = graph(&files);
    assert!(
        ty(&files, &cg, "drive", "run").is_none(),
        "TypeVisit locals cycle guard must fail closed"
    );
}
```

- [ ] **Step 3: Run the new guards**

Run:

```bash
cargo test --test integration rust_receiver_chain_external_intermediate_unchanged
cargo test --test integration rust_receiver_chain_inrepo_then_external_unchanged
cargo test --test integration rust_receiver_chain_trait_intermediate_unchanged
cargo test --test integration rust_receiver_chain_wrapper_peel_intermediate_unchanged
cargo test --lib rust_receiver_chain_depth_cap_fails_closed
cargo test --lib rust_receiver_local_fact_cycle_fails_closed
```

Expected output contains:

```text
test rust_receiver_chain_external_intermediate_unchanged ... ok
test rust_receiver_chain_inrepo_then_external_unchanged ... ok
test rust_receiver_chain_trait_intermediate_unchanged ... ok
test rust_receiver_chain_wrapper_peel_intermediate_unchanged ... ok
test rust_receiver_chain_depth_cap_fails_closed ... ok
test rust_receiver_local_fact_cycle_fails_closed ... ok
test result: ok.
```

- [ ] **Step 4: Commit**

```bash
git add tests/integration/resolution_test.rs src/resolution_receiver/tests.rs
git commit -m "test(rust-recv): pin chain fail-closed and recursion guards"
```

---

## Task 5: parallelize the receiver-typing post-pass

`rematerialize_rust_receiver_keys` is serial at `src/call_graph.rs:1123`. Parallelize only the typing map phase, then sort by `(FunctionId, CallSite::cmp_key)` before the existing serial apply. `rayon::prelude::*` is already imported at `src/call_graph.rs:12`, and `CallSite::cmp_key` is available in the same module at `src/call_graph.rs:1983`.

**Files:**
- Modify: `src/call_graph.rs`

- [ ] **Step 1: Add the determinism test**

Add this complete test to the existing `src/call_graph.rs` test module:

```rust
#[test]
fn rematerialize_is_deterministic_across_independent_builds() {
    let src: &[(&str, &str)] = &[(
        "a.rs",
        "pub struct B;\n\
         impl B { pub fn new() -> B { B } pub fn c(&self) -> B { B } pub fn run(&self) {} }\n\
         fn d() { B::new().c().run(); }\n",
    )];
    let cg1 = build_complete(src);
    let cg2 = build_complete(src);
    let key = |cg: &CallGraph| {
        cg.calls
            .values()
            .flatten()
            .map(|site| {
                (
                    site.caller.clone(),
                    site.callee_name.clone(),
                    site.start_byte,
                    site.end_byte,
                    site.receiver_outcome.clone(),
                )
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(
        key(&cg1),
        key(&cg2),
        "receiver rematerialization must be deterministic across independent builds"
    );
}
```

- [ ] **Step 2: Run it on the serial baseline**

Run:

```bash
cargo test --lib rematerialize_is_deterministic_across_independent_builds
```

Expected output contains:

```text
test rematerialize_is_deterministic_across_independent_builds ... ok
test result: ok.
```

- [ ] **Step 3: Replace `rematerialize_rust_receiver_keys`**

At `src/call_graph.rs:1123`, replace the full function with this complete implementation:

```rust
    fn rematerialize_rust_receiver_keys(&mut self, files: &BTreeMap<String, ParsedFile>) {
        if self.scope_graph.is_none() {
            return;
        }

        // Explicit immutable reborrow so the rayon map cannot be read as touching
        // `&mut self`; the borrow ends when `updates` is collected, before the apply.
        let cg: &CallGraph = &*self;
        let mut updates: Vec<(
            FunctionId,
            CallSite,
            Option<crate::resolution_identity::ReceiverOutcome>,
        )> = cg
            .calls
            .par_iter()
            .flat_map(|(caller, sites)| {
                let mut caller_updates = Vec::new();
                let Some(parsed) = files.get(&caller.file) else {
                    return caller_updates;
                };
                if !matches!(parsed.language, crate::languages::Language::Rust) {
                    return caller_updates;
                }
                let Some(fn_node) = Self::function_node_for_id(parsed, caller) else {
                    return caller_updates;
                };
                let all_lines: BTreeSet<usize> = (caller.start_line..=caller.end_line).collect();
                let ast_calls =
                    parsed.function_calls_with_qualifier_and_spans_on_lines(&fn_node, &all_lines);
                let typer = crate::resolution_receiver::RustReceiverTyper::new(cg);
                for site in sites {
                    let Some((_, _, qualifier, start_byte, _, receiver_expr, _, _)) = ast_calls
                        .iter()
                        .find(|(callee_name, _, _, start_byte, end_byte, _, _, _)| {
                            callee_name == &site.callee_name
                                && *start_byte == site.start_byte
                                && *end_byte == site.end_byte
                        })
                    else {
                        continue;
                    };
                    if receiver_expr.is_none() && qualifier.is_none() {
                        continue;
                    }
                    let outcome =
                        typer.type_of_receiver(crate::resolution_receiver::ReceiverTypeCtx {
                            parsed,
                            caller,
                            fn_node,
                            receiver_expr: *receiver_expr,
                            qualifier: qualifier.as_deref(),
                            call_start_byte: *start_byte,
                        });
                    caller_updates.push((caller.clone(), site.clone(), outcome));
                }
                caller_updates
            })
            .collect();
        updates.sort_by(|(caller_a, site_a, _), (caller_b, site_b, _)| {
            caller_a
                .cmp(caller_b)
                .then_with(|| site_a.cmp_key().cmp(&site_b.cmp_key()))
        });

        for (caller, old_site, outcome) in updates {
            let mut updated = old_site.clone();
            updated.receiver_outcome = outcome;
            if let Some(sites) = self.calls.get_mut(&caller) {
                if sites.take(&old_site).is_some() {
                    sites.insert(updated.clone());
                }
            }
            if let Some(sites) = self.callers.get_mut(&old_site.callee_name) {
                for site in sites {
                    if site.caller == old_site.caller && site.cmp_key() == old_site.cmp_key() {
                        site.receiver_outcome = updated.receiver_outcome.clone();
                    }
                }
            }
        }
    }
```

- [ ] **Step 4: Run determinism and the lib suite**

Run:

```bash
cargo test --lib rematerialize_is_deterministic_across_independent_builds
cargo test --lib
```

Expected output contains:

```text
test rematerialize_is_deterministic_across_independent_builds ... ok
test result: ok.
```

- [ ] **Step 5: Measure build-time proxy**

Run on the branch:

```bash
cargo build --release
/usr/bin/time -p ./target/release/prism nav --no-cache call-stats --repo ~/code/bench-repos/tokio >/tmp/prism-slice1a-tokio-call-stats-1.json
/usr/bin/time -p ./target/release/prism nav --no-cache call-stats --repo ~/code/bench-repos/tokio >/tmp/prism-slice1a-tokio-call-stats-2.json
/usr/bin/time -p ./target/release/prism nav --no-cache call-stats --repo ~/code/bench-repos/tokio >/tmp/prism-slice1a-tokio-call-stats-3.json
```

Expected output contains three `real`, `user`, and `sys` lines from `/usr/bin/time`. The average `real` time must not show a significant cold-build regression versus a `main` binary measured with the same commands.

- [ ] **Step 6: Commit**

```bash
git add src/call_graph.rs
git commit -m "perf(rust-recv): parallelize receiver rematerialization"
```

---

## Task 6: capability fixtures and matrix

Add capability fixtures that make wrong edges observable. The fixture loader discovers `eval/fixtures/rust/*/expected.toml`; this task does not modify registration files.

**Files:**
- Create: `eval/fixtures/rust/chain_in_repo_exact/main.rs`
- Create: `eval/fixtures/rust/chain_in_repo_exact/expected.toml`
- Create: `eval/fixtures/rust/external_chain_unchanged/main.rs`
- Create: `eval/fixtures/rust/external_chain_unchanged/expected.toml`
- Create: `eval/fixtures/rust/inrepo_then_external_unchanged/main.rs`
- Create: `eval/fixtures/rust/inrepo_then_external_unchanged/expected.toml`

- [ ] **Step 1: Write `chain_in_repo_exact`**

Create `eval/fixtures/rust/chain_in_repo_exact/main.rs`:

```rust
pub struct Builder;
impl Builder {
    pub fn new() -> Builder { Builder }
    pub fn cfg(&self, n: u8) -> Builder { Builder }
    pub fn tune(&self, a: u8, b: u8) -> Builder { Builder }
    pub fn run(&self) {}
}
pub fn drive() {
    Builder::new().cfg(1).tune(2, 3).run();
}
```

Create `eval/fixtures/rust/chain_in_repo_exact/expected.toml`:

```toml
[case]
language = "rust"
capability = "chain_in_repo_exact"
status = "pass"
[seed]
symbol = "run"
file = "main.rs"
line = 6
[[expect.callers]]
file = "main.rs"
line = 9
[expect]
exact = true
```

- [ ] **Step 2: Write `external_chain_unchanged`**

Create `eval/fixtures/rust/external_chain_unchanged/main.rs`:

```rust
pub struct LocalA;
impl LocalA { pub fn count(&self) {} }
pub struct LocalB;
impl LocalB { pub fn count(&self) {} }

pub fn drive(v: Vec<u8>) {
    v.iter().count();
}
```

Create `eval/fixtures/rust/external_chain_unchanged/expected.toml`:

```toml
[case]
language = "rust"
capability = "external_chain_unchanged"
status = "pass"
[seed]
symbol = "count"
file = "main.rs"
line = 2
[expect]
callers = []
exact = true
```

- [ ] **Step 3: Write `inrepo_then_external_unchanged`**

Create `eval/fixtures/rust/inrepo_then_external_unchanged/main.rs`:

```rust
pub struct Foo;
impl Foo {
    pub fn ext(&self) -> String { String::new() }
}
pub struct LocalA;
impl LocalA { pub fn m(&self) {} }
pub struct LocalB;
impl LocalB { pub fn m(&self) {} }

pub fn a() -> Foo { Foo }
pub fn drive() {
    a().ext().m();
}
```

Create `eval/fixtures/rust/inrepo_then_external_unchanged/expected.toml`:

```toml
[case]
language = "rust"
capability = "inrepo_then_external_unchanged"
status = "pass"
[seed]
symbol = "m"
file = "main.rs"
line = 6
[expect]
callers = []
exact = true
```

- [ ] **Step 4: Run the matrix gate**

Run:

```bash
cargo build --release
cd eval && uv run tier-a --matrix-only --allow-stale-sut
```

Expected output contains:

```text
chain_in_repo_exact
external_chain_unchanged
inrepo_then_external_unchanged
```

Expected exit code: 0. Existing known non-rust expected gaps may remain labelled as expected gaps; no rust regression is acceptable.

- [ ] **Step 5: Commit**

```bash
git add eval/fixtures/rust/chain_in_repo_exact eval/fixtures/rust/external_chain_unchanged eval/fixtures/rust/inrepo_then_external_unchanged
git commit -m "test(rust-recv): add method-chain capability fixtures"
```

---

## Task 7: Tier-A gate and measurement report

Use the AGENTS.md commands with an immediate preceding release rebuild in the same worktree. Do not rebaseline. Paste regressions or flip candidates into the PR description.

**Files:** none.

- [ ] **Step 1: Capture branch call-stats**

Run:

```bash
cargo build --release
./target/release/prism nav --no-cache call-stats --repo ~/code/bench-repos/tokio > /tmp/prism-slice1a-tokio-branch.json
./target/release/prism nav --no-cache call-stats --repo /Users/wesleyjinks/code/slicing > /tmp/prism-slice1a-prism-branch.json
```

Expected output: no stdout because output is redirected; exit code 0.

- [ ] **Step 2: Capture main call-stats with a separate worktree**

Run:

```bash
git worktree add /tmp/prism-slice1a-main main
cd /tmp/prism-slice1a-main
cargo build --release
./target/release/prism nav --no-cache call-stats --repo ~/code/bench-repos/tokio > /tmp/prism-slice1a-tokio-main.json
./target/release/prism nav --no-cache call-stats --repo /Users/wesleyjinks/code/slicing > /tmp/prism-slice1a-prism-main.json
```

Expected output contains:

```text
Finished `release` profile
```

Expected exit code: 0.

- [ ] **Step 3: Run Tier-A matrix**

Run from the branch worktree:

```bash
cargo build --release
cd eval && uv run tier-a --matrix-only --allow-stale-sut
```

Expected exit code: 0.

- [ ] **Step 4: Run Tier-A quick**

Run from the branch worktree:

```bash
cargo build --release
cd eval && uv run tier-a --quick --allow-stale-sut
```

Expected exit code: 0. The quick run needs rust-analyzer available. If it reports regressions or flip candidates, copy the reported cases into the PR description and fix blockers before review.

- [ ] **Step 5: Generate the measurement summary for the PR description**

Run:

```bash
python3 - <<'PY'
import json
from pathlib import Path

pairs = {
    "prism": (
        Path("/tmp/prism-slice1a-prism-main.json"),
        Path("/tmp/prism-slice1a-prism-branch.json"),
    ),
    "tokio": (
        Path("/tmp/prism-slice1a-tokio-main.json"),
        Path("/tmp/prism-slice1a-tokio-branch.json"),
    ),
}

for corpus, (main_path, branch_path) in pairs.items():
    main = json.loads(main_path.read_text())
    branch = json.loads(branch_path.read_text())
    print(f"{corpus} r6_single_owner_rust: {main['r6_single_owner_rust']} -> {branch['r6_single_owner_rust']}")
    print(f"{corpus} dropped_external_receiver: {main['dropped_external_receiver']} -> {branch['dropped_external_receiver']}")
    print(f"{corpus} kind_exact main: {main.get('kind_exact', {})}")
    print(f"{corpus} kind_exact branch: {branch.get('kind_exact', {})}")
PY
```

**Confirm the buy is real in-repo recall, not an aggregate shift:** the `r6_single_owner_rust` *reduction* must be matched by a corresponding *increase* in in-repo Exact resolution kinds (`kind_exact` deltas — `typed_param`/`field_typed`/`return_typed`), and `dropped_external_receiver` must be **unchanged** (Slice-1a adds no external drops). If the r6 reduction instead shows up as more drops/unknown, the typer is losing edges, not recovering them — investigate before claiming the recall buy. Paste the per-corpus before→after lines into the PR description.

Expected output contains eight lines: four for prism and four for tokio. Copy that stdout, the Tier-A matrix/quick exit codes, and the build-time proxy times into the PR description.

- [ ] **Step 6: Final review**

Run the codex xhigh receiver-typing review on `git diff main..HEAD`. The review prompt must ask specifically about recall-safety, `StdWrapperPeel`, external returns, AST chain decomposition, arity filtering, depth/cycle fail-closed behavior, and deterministic rematerialization.

---

## Self-review checklist

- **AST chain decomposition:** Task 1 uses `call_expression` -> `function`/`arguments` and `field_expression` -> `value`/`field`, verified against tree-sitter-rust and existing prism helpers.
- **Single Exact intermediate:** Task 2 mirrors `combine_kind`'s has-self, arity, single-inherent, and `StdWrapperPeel` conditions.
- **In-repo recall only:** Task 3 propagates only `TypeKey::InRepo` returns and returns `None` for external/Bare/non-unique cases.
- **Let-init method chains:** Task 3 routes `InitExpr::Call` through the AST node when available.
- **Bounded recursion and cycle guards:** Task 4 pins `MAX_RECEIVER_TYPE_DEPTH` and `TypeVisit` fail-closed behavior.
- **Parallel post-pass:** Task 5 uses rayon map plus deterministic sorted serial apply.
- **Tier-A gate:** Task 7 uses `--matrix-only --allow-stale-sut` and `--quick --allow-stale-sut` only after immediate release builds.

## After Slice-1a

Slice-1b remains separate: widen in-repo field/let identity coverage in `resolve_type_path_to_type_scope` without promoting existing `Bare` outcomes. The **field-chain capability fixture** (e.g. `field_chain_exact`) belongs to Slice-1b, **not** 1a — its absence here is intentional, not an omission (1a's field support is the existing `field_type_from_base`; 1b widens the leaf resolver). External-return summaries, generic output re-entry, and wrapper-specific method modeling remain follow-on work.
