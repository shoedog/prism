# Prism JSX/TSX & React 18/19 Support Plan

## Status: Draft Spec — April 2026

## Purpose

This document is a combined implementation plan and Claude Code handoff spec for adding JSX/TSX parsing, JSX semantic analysis, and React 18/19 hook awareness to Prism. It covers six implementation layers spanning from a critical parsing fix through research-grade React render cycle modeling.

Each layer is self-contained — later layers depend on earlier ones, but any prefix of the layers produces a useful, shippable improvement.

-----

## Current State Assessment

### What Works Today

**JSX (`.jsx`)** routes to `Language::JavaScript` via `from_extension`. The `tree-sitter-javascript` grammar includes JSX support by default, so `.jsx` files parse correctly. However, the algorithms traverse JSX node types without extracting semantic meaning — identifiers inside `jsx_expression` nodes (`{variable}`) are picked up by recursive traversal, but component instantiation isn’t modeled as a function call.

**TypeScript (`.ts`)** works correctly using `LANGUAGE_TYPESCRIPT` from `tree-sitter-typescript`.

### What’s Broken

**TSX (`.tsx`)** is broken. `languages/mod.rs` line 45 uses `LANGUAGE_TYPESCRIPT` for all TypeScript files, but the `tree-sitter-typescript` crate (v0.23.2) exports two distinct grammars: `LANGUAGE_TYPESCRIPT` (no JSX) and `LANGUAGE_TSX` (with JSX). Any `.tsx` file containing JSX syntax produces parse errors. The `error_rate()` metric flags this, but slicing results are garbage. This is a data-corrupting bug for any React TypeScript codebase.

### Cross-File Behavior

Prism’s call graph resolves by function name string matching, not by language variant. A `Button` component defined in `Button.jsx` and rendered in `App.tsx` connects correctly as long as both files parse. The JSX node types (`jsx_element`, `jsx_self_closing_element`, `jsx_expression`, etc.) are identical across the JavaScript and TSX grammars since both inherit from the same upstream grammar rules.

### Zero JSX Awareness

A grep across all source files confirms zero references to any JSX-specific node type (`jsx_element`, `jsx_self_closing_element`, `jsx_expression`, `jsx_attribute`, `jsx_fragment`, `jsx_opening_element`, `jsx_closing_element`, `jsx_text`). All JSX handling is currently implicit via recursive traversal.

-----

## Layer 1: TSX Parsing Fix

**Effort:** ~30 lines | **Priority:** P0 — Blocking | **Depends on:** Nothing

### Problem

`.tsx` files are parsed with `LANGUAGE_TYPESCRIPT`, which cannot parse JSX syntax. This is a silent data corruption bug.

### Solution: New `Tsx` Enum Variant (Option A)

Add `Language::Tsx` to the enum. Route `.tsx` → `Tsx`. Use `LANGUAGE_TSX` for parsing. Share all node-type methods with `TypeScript` via match arm grouping (`Self::TypeScript | Self::Tsx =>`).

Option B (a runtime `is_tsx: bool` flag on `ParsedFile`) was considered but rejected — it obscures the distinction and complicates pattern matching throughout the codebase.

### Changes Required

**`src/languages/mod.rs`:**

1. Add `Tsx` to the `Language` enum.
1. In `from_extension`: change `"ts" | "tsx" => Some(Self::TypeScript)` to `"ts" => Some(Self::TypeScript)` and add `"tsx" => Some(Self::Tsx)`.
1. In `tree_sitter_language`: add `Self::Tsx => tree_sitter_typescript::LANGUAGE_TSX.into()`.
1. In every match arm that currently says `Self::TypeScript =>` or `Self::JavaScript | Self::TypeScript =>`, extend to include `Self::Tsx`. The full list of methods requiring this change:
- `function_node_types` — add `Self::Tsx` alongside `Self::TypeScript`
- `is_identifier_node` — already uses a flat match on kind strings, no change needed
- `is_assignment_node` — extend `Self::JavaScript | Self::TypeScript` to `Self::JavaScript | Self::TypeScript | Self::Tsx`
- `is_declaration_node` — same pattern
- `assignment_target` — same pattern
- `assignment_value` — same pattern
- `declaration_name` — same pattern
- `declaration_value` — same pattern
- `is_control_flow_node` — already uses flat match, no change needed
- `control_flow_condition` — already universal, no change needed
- `is_call_node` — already uses flat match, no change needed
- `call_function_name` — already universal, no change needed
- `call_arguments` — already universal, no change needed
- `is_scope_block` — already uses flat match, no change needed
- `is_return_node` — already uses flat match, no change needed
- `is_statement_node` — already uses flat match, no change needed
- `is_loop_node` — already uses flat match, no change needed
- `is_terminator` — already uses flat match, no change needed
- `switch_has_fallthrough` — add `Self::Tsx` alongside `Self::TypeScript`
- `function_name` — already universal, no change needed

**`src/main.rs`:** If there’s any match on `Language` for CLI dispatch or output formatting, add the `Tsx` arm.

**`src/lib.rs`:** No change needed — `Language` is re-exported from `languages`.

**Serialization:** The `Tsx` variant needs `serde::Serialize`/`Deserialize` — the derive macro handles this automatically.

### Validation

1. Parse a `.tsx` file containing JSX and verify `error_rate() == 0.0`.
1. Confirm that existing `.ts` files (no JSX) still parse identically.
1. Run the full test suite — no existing tests should break since none use `.tsx`.

### Test Fixture

Add a `make_tsx_test()` helper to `tests/common/mod.rs`:

```typescript
// test.tsx
import React, { useState, useEffect } from 'react';

interface Props {
    userId: string;
    onLoad: (data: UserData) => void;
}

const UserProfile: React.FC<Props> = ({ userId, onLoad }) => {
    const [user, setUser] = useState<UserData | null>(null);
    const [loading, setLoading] = useState(true);

    useEffect(() => {
        fetchUser(userId).then(data => {
            setUser(data);
            setLoading(false);
            onLoad(data);
        });
    }, [userId, onLoad]);

    if (loading) return <Spinner size="large" />;

    return (
        <div className="profile">
            <Avatar src={user.avatar} alt={user.name} />
            <h1>{user.name}</h1>
            <ContactList contacts={user.contacts} />
        </div>
    );
};
```

-----

## Layer 2: JSX Call Graph Integration

**Effort:** ~40 lines | **Priority:** P1 — High Value | **Depends on:** Layer 1

### Problem

JSX component instantiation (`<MyComponent />`, `<MyComponent>...</MyComponent>`) is semantically a function call, but tree-sitter represents it as `jsx_self_closing_element` or `jsx_opening_element`, not `call_expression`. The call graph misses all component usage relationships.

### JSX Node Types (tree-sitter grammar)

|JSX Syntax        |tree-sitter Node Type     |Relevant Children                                           |
|------------------|--------------------------|------------------------------------------------------------|
|`<Comp />`        |`jsx_self_closing_element`|Tag name identifier, `jsx_attribute` children               |
|`<Comp>`          |`jsx_opening_element`     |Tag name identifier, `jsx_attribute` children               |
|`</Comp>`         |`jsx_closing_element`     |Tag name identifier                                         |
|`<Comp>...</Comp>`|`jsx_element`             |`jsx_opening_element` + children + `jsx_closing_element`    |
|`<>...</>`        |`jsx_fragment`            |Children only, no tag name                                  |
|`{expr}`          |`jsx_expression`          |Arbitrary expression child                                  |
|`prop={val}`      |`jsx_attribute`           |Name identifier + value (string literal or `jsx_expression`)|
|`literal text`    |`jsx_text`                |Raw string content                                          |

### Solution

**`src/languages/mod.rs` — `is_call_node`:**

Add `jsx_self_closing_element` and `jsx_opening_element` to the flat match. These are the two node types that represent a component being *invoked* (closing elements and fragments don’t invoke anything).

```rust
pub fn is_call_node(&self, kind: &str) -> bool {
    matches!(
        kind,
        "call_expression"
            | "call"
            | "method_invocation"
            | "object_creation_expression"
            | "new_expression"
            | "function_call"
            | "jsx_self_closing_element"    // <Component />
            | "jsx_opening_element"        // <Component>
    )
}
```

**`src/languages/mod.rs` — `call_function_name`:**

Add logic to extract the tag name from JSX elements. The tag name is the first named child (an `identifier` for user components like `MyComponent`, or a `member_expression` for `Foo.Bar`, or a `jsx_namespace_name` for `ns:tag`). HTML intrinsics (`div`, `span`, etc.) also have identifier tag names but are lowercase — filtering those out is a Layer 3 concern.

```rust
pub fn call_function_name<'a>(&self, node: &Node<'a>) -> Option<Node<'a>> {
    // JSX elements: tag name is the first named child
    if node.kind() == "jsx_self_closing_element"
        || node.kind() == "jsx_opening_element"
    {
        // First named child is the tag name (identifier or member_expression)
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "identifier"
                || child.kind() == "member_expression"
                || child.kind() == "jsx_namespace_name"
            {
                // For member_expression (e.g., <Foo.Bar />), extract the property
                if child.kind() == "member_expression" {
                    if let Some(prop) = child.child_by_field_name("property") {
                        return Some(prop);
                    }
                }
                return Some(child);
            }
        }
        return None;
    }

    // ... existing call_function_name logic unchanged ...
}
```

### Filtering HTML Intrinsics

JSX elements with lowercase tag names (`<div>`, `<span>`, `<input>`) are HTML intrinsics, not component calls. Two options:

**Option A (simple):** Filter in `call_graph.rs` — skip any call site where `callee_name` starts with a lowercase ASCII character and matches a known HTML element set. Simple, pragmatic.

**Option B (deferred):** Don’t filter. HTML intrinsics will appear in the call graph as unresolved callees (no matching function definition). This is harmless for slicing — they add noise to the graph but don’t produce incorrect edges. Filter later when it becomes annoying.

Recommend Option B for now. The call graph already handles unresolved callees gracefully.

### Validation

1. Parse the Layer 1 test fixture and verify the call graph contains edges from `UserProfile` → `Spinner`, `Avatar`, `ContactList`.
1. Verify `fetchUser` still appears as a regular `call_expression` call site.
1. Verify `<div>`, `<h1>` appear as (unresolved) call sites without causing errors.

-----

## Layer 3: JSX Data Flow Enhancement

**Effort:** ~80 lines | **Priority:** P1 — High Value | **Depends on:** Layer 2

### What Already Works

Because Prism’s AST traversal functions (`collect_rvalue_paths`, `collect_identifiers_at_row`, `collect_all_identifiers`, `collect_rvalues`) recurse through all children regardless of node type, identifiers inside JSX expressions are already detected as variable uses. Specifically:

- `{user.name}` → `user` is found as an identifier use on that line
- `{loading ? <Spinner /> : <Content />}` → `loading` is found as an identifier use
- `onClick={() => handleClick(item)}` → `handleClick` and `item` are found

### What Needs Work

**Props as data flow edges.** When you write `<Component name={value} />`, the `jsx_attribute` node contains a name (`name`) and a value (`jsx_expression` containing `value`). The `value` identifier is already detected as a use. However, the *connection* between this use and the `name` parameter inside `Component`’s function body is not modeled — this is the same limitation as any dynamically-resolved function call today.

To model this properly:

1. For each `jsx_self_closing_element` / `jsx_opening_element`, collect the list of `jsx_attribute` children.
1. Each attribute has a name and a value. The value’s identifiers are already captured as uses.
1. If the component’s function definition is known (via the call graph), match attribute names to function parameter destructuring patterns.

This is the prop-to-parameter connection and is genuinely hard to do in full generality (spread props, default values, `React.forwardRef`, HOCs). A pragmatic approach:

**Simple case:** The component function uses destructured props:

```tsx
function Button({ label, onClick }) { ... }
// <Button label={text} onClick={handler} />
```

Here, `label` in the JSX attribute maps to `label` in the destructuring pattern. Prism’s existing destructuring alias analysis (`extract_destructuring_aliases` in `ast.rs`) already handles `const { label } = props`, so the machinery exists.

**Recommendation:** Defer full prop-to-parameter data flow to a future layer. The immediate value of Layer 2 (call graph edges for component usage) is high on its own. Document this as a known gap.

### JSX Children as Data Flow

`{expression}` children inside JSX elements are `jsx_expression` nodes. The identifiers within them are already captured. However, the *children* relationship (this expression is a child of this component) could be modeled as an implicit `children` prop data flow edge.

Again, defer. The existing traversal picks up the variable uses, which is the critical thing for slicing.

### New: `jsx_expression` as Statement-Level Construct

Consider whether `jsx_expression` should be added to `is_statement_node`. Currently it isn’t, which means CFG construction may skip over expressions embedded in JSX. In practice this doesn’t matter much since JSX expressions are typically simple reads, not assignments or control flow — but for completeness, adding it prevents any CFG gaps.

-----

## Layer 4: React Hook Call Detection

**Effort:** ~100 lines (new module) | **Priority:** P2 — Moderate Value | **Depends on:** Layer 1

### Overview

React hooks are syntactically just `call_expression` nodes with known names. Prism already detects these. This layer adds *semantic tagging* — annotating known hook calls with metadata so downstream algorithms can reason about them.

### Hook Taxonomy

|Hook              |Category   |Key Semantic                          |Data Flow Implication                              |
|------------------|-----------|--------------------------------------|---------------------------------------------------|
|`useState`        |State      |Returns `[value, setter]` tuple       |Setter call triggers re-render; value is reactive  |
|`useReducer`      |State      |Returns `[state, dispatch]` tuple     |Dispatch call triggers re-render; state is reactive|
|`useEffect`       |Effect     |Callback deferred to post-render      |Callback is a separate execution context           |
|`useLayoutEffect` |Effect     |Like useEffect but synchronous        |Same as useEffect for static analysis              |
|`useMemo`         |Memoization|Returns cached value                  |Value recomputed only when deps change             |
|`useCallback`     |Memoization|Returns cached function               |Function identity stable across renders            |
|`useRef`          |Ref        |Returns mutable ref object            |`.current` mutations don’t trigger re-render       |
|`useContext`      |Context    |Reads from context provider           |Value is reactive to provider changes              |
|`useId`           |Utility    |Returns stable unique ID              |Pure, no data flow implications                    |
|`useTransition`   |Concurrent |Returns `[isPending, startTransition]`|Marks state updates as non-urgent                  |
|`useDeferredValue`|Concurrent |Returns deferred version of value     |Value may lag behind source                        |
|`use` (React 19)  |Resource   |Unwraps promise or context            |Suspends component until resolved                  |

### Implementation: `src/react_hooks.rs`

Create a new module that provides hook metadata extraction:

```rust
/// Detected React hook call with semantic metadata.
pub struct HookCall {
    pub file: String,
    pub function: String,    // Enclosing component/hook function
    pub line: usize,
    pub hook_type: HookType,
    pub callback: Option<CallbackInfo>,    // For useEffect/useMemo/useCallback
    pub deps: Option<DepsInfo>,            // Dependency array info
}

pub enum HookType {
    UseState,
    UseReducer,
    UseEffect,
    UseLayoutEffect,
    UseMemo,
    UseCallback,
    UseRef,
    UseContext,
    UseId,
    UseTransition,
    UseDeferredValue,
    Use,              // React 19
    Custom(String),   // useXxx pattern
}

pub struct CallbackInfo {
    pub start_line: usize,
    pub end_line: usize,
    pub captured_identifiers: Vec<(String, usize)>,  // (name, line)
}

pub struct DepsInfo {
    pub line: usize,
    pub identifiers: Vec<(String, usize)>,  // (name, line)
    pub is_empty: bool,                      // [] = mount-only
    pub is_missing: bool,                    // No second arg = every render
}
```

### Detection Logic

1. Walk all `call_expression` nodes within component functions.
1. Match the function name against the hook taxonomy. Also match `useXxx` pattern for custom hooks.
1. For hooks with callbacks (useEffect, useMemo, useCallback): extract the first argument (should be `arrow_function` or `function_expression`). Record its line range and collect all identifiers within it.
1. For hooks with dependency arrays: extract the second argument (should be `array`). Collect all identifiers within it. Flag if empty (`[]`) or missing.

### Integration Points

- **Call graph:** Hook calls are already detected as `call_expression`. No change needed.
- **Data flow:** The callback’s captured identifiers and the dependency array identifiers feed into the existing def-use chain analysis. See Layer 5.
- **Algorithms:** Hook metadata can enrich existing slicing. For example, `echo_slice` could trace a state setter back to its `useState` declaration, and `absence_slice` could flag missing dependency array entries.

-----

## Layer 5: Dependency Array Analysis

**Effort:** ~150 lines | **Priority:** P2 — High Value for React Code Review | **Depends on:** Layer 4

### Problem Statement

The most common React hook bug category is **stale closures** — when a `useEffect`/`useMemo`/`useCallback` callback captures a variable that isn’t listed in the dependency array. The callback continues to reference a stale value from a previous render.

ESLint’s `exhaustive-deps` rule catches this via scope-walking, but is limited by ESLint’s static analysis capabilities — it can’t handle aliasing or indirect references. Prism’s data flow graph provides a fundamentally more powerful analysis substrate.

### Analysis Algorithm

For each hook call with a dependency array (from Layer 4):

1. **Extract callback body identifiers:** All identifiers referenced within the callback’s AST subtree that are defined outside the callback (i.e., in the enclosing component function scope).
1. **Extract dependency array identifiers:** All identifiers listed in the `[deps]` array.
1. **Classify each callback identifier:**
- **Stable:** React guarantees referential stability (e.g., `setState` setter from `useState`, `dispatch` from `useReducer`, `ref` from `useRef`). These don’t need to be in the dependency array.
- **External:** Defined outside the component (module scope, imports). These are stable.
- **Reactive:** Defined inside the component and may change between renders (state values, props, computed values). These *must* be in the dependency array.
1. **Compare:**
- **Missing dependency:** Reactive identifier used in callback but not in deps array → potential stale closure.
- **Unnecessary dependency:** Identifier in deps array but not used in callback → unnecessary re-execution (performance issue, not a bug).
- **Correct:** All reactive identifiers are in deps, no extras.

### Stable Value Detection

The trickiest part is identifying stable values. Prism can do this with data flow tracing:

- **`useState` setter:** The second element of `useState`’s return value. Trace the destructuring: `const [value, setValue] = useState(...)` → `setValue` is stable.
- **`useReducer` dispatch:** Same pattern: `const [state, dispatch] = useReducer(...)` → `dispatch` is stable.
- **`useRef` ref:** The return value of `useRef(...)` is stable. But `ref.current` is *not* — it’s mutable and reading it in an effect is fine (not a dep), but relying on its value for effect timing is a bug.
- **`useCallback` result:** The return value is stable *by identity*, but its *value* depends on its own deps. This creates a transitive dependency analysis problem.

### Output Format

```rust
pub struct DepsAnalysis {
    pub hook_line: usize,
    pub hook_type: HookType,
    pub missing: Vec<MissingDep>,
    pub unnecessary: Vec<UnnecessaryDep>,
}

pub struct MissingDep {
    pub identifier: String,
    pub used_at_line: usize,    // Where it's used in the callback
    pub defined_at_line: usize, // Where it's defined in the component
    pub is_stable: bool,        // If true, this is a false positive
    pub severity: DepSeverity,
}

pub enum DepSeverity {
    /// State/prop value missing — will cause stale closure
    StaleClosure,
    /// Function missing — may cause stale closure if function captures state
    PotentialStaleClosure,
    /// Ref value accessed — usually intentional, low severity
    RefAccess,
}
```

### Academic Grounding

This analysis is essentially what the `exhaustive-deps` ESLint rule does, but with two advantages from Prism’s architecture:

1. **Data flow graphs** provide actual def-use chains, not just scope-based identifier collection. This means Prism can trace through aliases (`const fn = someCallback; useEffect(() => { fn(); }, [])` — ESLint may miss that `fn` is a reactive alias).
1. **Access path tracking** (Prism’s `AccessPath` system) can distinguish `user.name` from `user.id` in dependency arrays, providing more precise analysis than ESLint’s identifier-level matching.

### Relationship to React Compiler

Meta’s React Compiler (v1.0, stable Oct 2025) solves a related problem — automatic memoization — using a CFG-based HIR with SSA, type inference, and effect analysis. Its “reactive analysis” phase classifies values as static vs. reactive, which is exactly the classification needed here. The key difference: the React Compiler operates at build time to *insert* memoization, while Prism operates at review time to *flag* potential issues. The analysis is the same; the action is different.

The React Compiler’s Mutability & Aliasing Model (June 2025) tracks five effect types (read, store, capture, mutate, freeze) and groups instructions into reactive scopes. Prism’s simpler def-use model is sufficient for dependency array validation but could be enhanced with mutation tracking for more precise analysis.

-----

## Layer 6: Deferred Execution Modeling

**Effort:** ~200 lines | **Priority:** P3 — Stretch | **Depends on:** Layer 4

### Problem Statement

`useEffect` callbacks don’t execute in the component’s synchronous control flow. They’re deferred to after the browser paints (or after layout, for `useLayoutEffect`). In Prism’s CFG, this means the callback body should not have control flow edges from the surrounding function body — it’s a separate execution context with an implicit scheduling relationship.

### Current Behavior

Prism’s `all_functions` collects arrow functions and function expressions. A `useEffect(() => { ... }, [deps])` callback is an `arrow_function` node, so it’s already collected as a separate function. Its identifiers are tracked in the data flow graph within their own function scope. This is *mostly correct* — the callback is analyzed as its own function, with its own defs and uses.

The gap: the *invocation* of this callback is not modeled. The call graph doesn’t show “React runtime → effect callback” as an edge, because there’s no `call_expression` for it. The effect callback is passed as an argument to `useEffect`, not called directly.

### Solution: Synthetic Call Graph Edges

Model `useEffect` (and `useLayoutEffect`, `useMemo`, `useCallback`) callbacks as functions with synthetic callers:

1. When a hook call is detected (Layer 4), and it has a callback argument:
1. Find the corresponding function node in `all_functions` (match by line range).
1. Add a synthetic edge in the call graph: `<react_runtime>` → callback function.
1. For `useEffect`/`useLayoutEffect`: mark the edge as *deferred* (not part of the component’s synchronous control flow).
1. For `useMemo`/`useCallback`: mark the edge as *synchronous* (executes during render, but only when deps change).

### Data Flow Implications

The dependency array creates an explicit data dependency: the callback *observes* the dependency array values. Model this as:

- For each identifier in the dependency array, create a data flow edge from the identifier’s definition to a synthetic “dependency observation” use at the hook call line.
- For each identifier captured by the callback but NOT in the dependency array (a Layer 5 finding), note that this is a *stale capture* — the value is frozen at the render cycle when the callback was created.

### CFG Treatment

In the CFG for the enclosing component function:

- The `useEffect(callback, deps)` call is a statement node (it’s a `call_expression`).
- The callback body is *not* part of the component’s CFG — it’s a separate function’s CFG.
- The `deps` array is evaluated synchronously as part of the component’s CFG.

This is already how Prism handles it (arrow functions get their own function scope). The enhancement is making the scheduling relationship explicit in the call graph metadata.

### Cleanup Function Modeling

`useEffect` callbacks can return a cleanup function:

```tsx
useEffect(() => {
    const subscription = source.subscribe();
    return () => subscription.unsubscribe();  // cleanup
}, [source]);
```

The cleanup function is another deferred execution context — it runs before the next effect execution or on unmount. Model it as:

1. Detect `return_statement` inside the effect callback that returns a function.
1. Add a synthetic call graph edge: `<react_cleanup>` → cleanup function.
1. Mark cleanup as running *before* the next effect callback invocation.

Missing cleanup functions are a common React bug (memory leaks, event listener accumulation). This modeling enables `absence_slice` to detect when an effect creates a subscription/listener but doesn’t return a cleanup.

-----

## Implementation Roadmap

### Phase 1: Parsing Foundation (Layers 1-2)

**Estimated effort:** 2-3 hours Claude Code session
**Deliverables:**

- `Language::Tsx` variant with `LANGUAGE_TSX` parsing
- JSX elements in `is_call_node` and `call_function_name`
- `make_tsx_test()` fixture in test suite
- All 26 algorithms verified against TSX fixture

**Success criteria:**

- `.tsx` files parse with `error_rate() == 0.0`
- Call graph contains component-to-component edges
- No existing tests broken

### Phase 2: React Hook Metadata (Layers 3-4)

**Estimated effort:** 4-6 hours Claude Code session
**Deliverables:**

- `src/react_hooks.rs` module with hook detection and metadata extraction
- JSX data flow documentation (known gaps)
- Test fixtures covering all major hook types

**Success criteria:**

- Hook calls detected and tagged with correct `HookType`
- Callback bodies and dependency arrays correctly extracted
- Custom hooks (`useXxx` pattern) detected

### Phase 3: Dependency Array Analysis (Layer 5)

**Estimated effort:** 8-12 hours across sessions
**Deliverables:**

- `DepsAnalysis` output from dependency array validation
- Stable value detection for `useState` setter, `useReducer` dispatch, `useRef`
- CLI output format for deps analysis results
- Test cases covering stale closure, unnecessary dep, and correct dep scenarios

**Success criteria:**

- Known stale closure patterns detected with zero false negatives
- Stable values correctly classified (low false positive rate)
- Analysis runs in < 100ms per component

### Phase 4: Deferred Execution Modeling (Layer 6)

**Estimated effort:** 6-8 hours
**Deliverables:**

- Synthetic call graph edges for effect callbacks
- Cleanup function detection and modeling
- Documentation of CFG treatment for deferred execution

**Success criteria:**

- Effect callbacks appear in call graph with deferred scheduling metadata
- Missing cleanup functions detectable via absence slice
- No regression in existing algorithm behavior

-----

## Research Context

### Formal Semantics

Two academic papers provide the theoretical foundation:

**Madsen, Lhoták, Tip (ECOOP 2020)** — “A Semantics for the Essence of React.” Small-step operational semantics for React covering mounting, unmounting, and reconciliation. Explicitly scoped out hooks. Proved that well-behavedness is preserved by key React operations. Stated long-term goal of automatic tools for program understanding and bug finding.

**Lee, Ahn (OOPSLA 2025, Seoul National University)** — “React-tRace: A Semantics for Understanding React Hooks.” Extends the ECOOP 2020 work to formalize useState and useEffect semantics. Key contributions:

- Models infinite re-rendering as the most catastrophic hook bug category, arising from a Check-Effect decision cycle (setter called inside effect modifies state that re-triggers the effect).
- Formalizes the property that a setter called during rendering immediately re-evaluates the component body.
- Provides a definitional interpreter (OCaml) that tracks render cycles, state updates, and effect executions.
- Validates against a conformance test suite comparing interpreter output to actual React behavior.

### Production Implementations

**React Compiler v1.0 (Meta, stable Oct 2025)** — Build-time compiler that automatically memoizes React components. Architecture: CFG-based HIR → SSA → type inference → effect analysis → reactive analysis → scope discovery → code generation. The “reactive analysis” phase classifies values as static vs. reactive, which is the same classification Layer 5 needs. The Mutability & Aliasing Model (June 2025) tracks five effect types (read, store, capture, mutate, freeze).

**`eslint-plugin-react-hooks` `exhaustive-deps` rule** — The closest existing tool to Layer 5. Walks the AST scope tree to find variables referenced inside hook callbacks that are defined outside, then compares against the dependency array. Limitations: can’t handle aliasing or indirect references due to ESLint’s static analysis capabilities. Prism’s data flow graphs provide a more powerful analysis substrate.

### Why Not Full Render Cycle Modeling?

React-tRace and the React Compiler model the full render/re-render/effect cycle, state batching, and reconciliation. Prism doesn’t need this for code review slicing. The bugs caught by the AI reviewer in React code are: stale closures, missing cleanup functions, prop mutation, incorrect dependency arrays, and unnecessary re-renders. All of these are addressable at Layers 1-6 without modeling the full React runtime.

Full render cycle modeling (a hypothetical Layer 7) would be needed for: detecting infinite render loops, analyzing component composition performance, and reasoning about concurrent mode scheduling. These are valuable but are research-grade problems with diminishing returns for code review.

-----

## File Manifest

|File                             |Action   |Layer|Description                                     |
|---------------------------------|---------|-----|------------------------------------------------|
|`src/languages/mod.rs`           |Modify   |1-2  |Add `Tsx` variant, JSX call detection           |
|`src/main.rs`                    |Modify   |1    |Handle `Tsx` in any CLI dispatch                |
|`src/react_hooks.rs`             |Create   |4-6  |Hook metadata, deps analysis, deferred execution|
|`src/lib.rs`                     |Modify   |4    |Export `react_hooks` module                     |
|`src/call_graph.rs`              |Modify   |6    |Synthetic edges for effect callbacks            |
|`src/data_flow.rs`               |Modify   |5    |Stable value classification                     |
|`tests/common/mod.rs`            |Modify   |1    |`make_tsx_test()` fixture                       |
|`tests/lang/tsx/`                |Create   |1-3  |TSX-specific algorithm tests                    |
|`tests/lang/tsx/tsx_test.rs`     |Create   |1    |Basic TSX algorithm coverage                    |
|`tests/lang/tsx/hooks_test.rs`   |Create   |4-5  |Hook detection and deps analysis tests          |
|`tests/lang/tsx/jsx_call_test.rs`|Create   |2    |JSX call graph integration tests                |
|`Cargo.toml`                     |No change|—    |`tree-sitter-typescript` already includes TSX   |

-----

## Open Questions

1. **HTML intrinsic filtering:** Should `<div>`, `<span>`, etc. be filtered from the call graph in Layer 2, or deferred? Current recommendation: defer (Option B).
1. **Custom hook dependency propagation:** If `useCustomHook(dep)` internally calls `useEffect` with its own deps, should Prism trace through? This requires interprocedural hook analysis. Defer to post-Layer 6.
1. **React.memo / forwardRef / HOC wrapping:** Components wrapped in these have modified call signatures. How should the call graph model `React.memo(Component)` vs `Component`? Likely: treat the wrapper call as a transparent alias.
1. **Server Components / “use server” / “use client”:** These directives create execution boundary annotations. Could map to Prism’s `membrane_slice` boundary concept. Not in scope for this plan but worth noting for future work.
1. **JSX spread props:** `<Component {...props} />` makes prop-to-parameter mapping impossible without type information. Accept as a known gap.