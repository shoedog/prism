//! Python attribute loads, properties and self-instance methods.

use crate::ast::{byte_range_eq, ParsedFile, PythonAttributeLoadCandidate, PythonAttributeLoadScan, PythonPropertyKind};
use crate::languages::Language;
use std::collections::BTreeSet;
use tree_sitter::Node;

impl ParsedFile {
    /// Python constructor/local-annotation lookup uses enclosing function scopes.
    /// A signature annotation is evaluated outside its own function body.
    pub(crate) fn python_receiver_owner_shadowed(
        &self,
        function: Node<'_>,
        name: &str,
        include_current: bool,
    ) -> bool {
        let mut current = if include_current {
            Some(function)
        } else {
            function.parent()
        };
        while let Some(node) = current {
            if matches!(node.kind(), "function_definition" | "lambda")
                && self
                    .function_local_value_bindings(&node)
                    .is_none_or(|names| names.contains(name))
            {
                return true;
            }
            current = node.parent();
        }
        false
    }

    /// P7 S1: which `@property`-family decorator (if any, exact-match only)
    /// decorates a Python function node from `all_functions()`. Reads
    /// decorator expressions on the raw `decorated_definition` wrapper BEFORE
    /// `unwrap_decorated` strips it (mirrors the existing
    /// `type_providers::python::has_decorator` / flask decorator-reading
    /// convention: iterate `decorated_definition`'s `decorator` children and
    /// read each one's source text directly — no compiled query exists for
    /// `decorator` nodes).
    ///
    /// Exact full-expression match only: `property`, `cached_property`, or
    /// `functools.cached_property`. This is deliberately not a last-segment
    /// match (unlike `has_decorator`'s dataclass check) — `@x.setter` /
    /// `@x.deleter` (whatever `x` is) and any other decorator never match,
    /// so the setter/deleter exclusion falls out of the exact-match rule
    /// itself rather than needing a separate reject list.
    pub(crate) fn python_property_kind(&self, func_node: &Node<'_>) -> Option<PythonPropertyKind> {
        if self.language != Language::Python || func_node.kind() != "decorated_definition" {
            return None;
        }
        let mut cursor = func_node.walk();
        for child in func_node.children(&mut cursor) {
            if child.kind() != "decorator" {
                continue;
            }
            let text = self.node_text(&child).trim();
            let Some(expr) = text.strip_prefix('@') else {
                continue;
            };
            match expr.trim() {
                "property" => return Some(PythonPropertyKind::Property),
                "cached_property" | "functools.cached_property" => {
                    return Some(PythonPropertyKind::CachedProperty)
                }
                _ => {}
            }
        }
        None
    }

    /// P7 (F2, codex MAJOR 2; re-fixed per codex re-review): whether
    /// `func_node` (as returned by `all_functions()`) is a genuine Python
    /// instance method whose first POSITIONAL parameter is literally named
    /// `self` — gates tier-1 same-class property narrowing for `self.attr`
    /// in `call_graph.rs::self_property_owner_getters`. (Tier-1 eligibility
    /// ALSO requires `method_owners.contains_key(&caller_id)`, checked in
    /// `call_graph.rs` before this gate is even consulted — a `self`
    /// receiver in a non-method has no owner and must route straight to
    /// tier-3, never tier-1-then-drop.)
    ///
    /// `method_owner` (`languages/mod.rs`) marks ANY class-contained
    /// function as owned by its class, including `@staticmethod`/
    /// `@classmethod`-decorated ones — a `@staticmethod def f(self)` has
    /// `self` as an ordinary parameter of unknown type, not a receiver, so
    /// it must not get same-class narrowing. This check is scoped to the P7
    /// property-access path only; it does NOT change `method_owner`'s
    /// semantics or method-CALL resolution's existing behavior.
    ///
    /// The enclosing function MAY legitimately carry other decorators,
    /// including `@property` itself: a getter reading `self.other_prop` is
    /// still a genuine instance method.
    pub(crate) fn python_is_self_instance_method(&self, func_node: &Node<'_>) -> bool {
        if self.language != Language::Python {
            return false;
        }
        if func_node.kind() == "decorated_definition" {
            let mut cursor = func_node.walk();
            for child in func_node.children(&mut cursor) {
                if child.kind() != "decorator" {
                    continue;
                }
                let text = self.node_text(&child).trim();
                let Some(expr) = text.strip_prefix('@') else {
                    continue;
                };
                if matches!(expr.trim(), "staticmethod" | "classmethod") {
                    return false;
                }
            }
        }
        self.python_first_positional_param_is_self(func_node)
    }

    /// P7 (F2 re-fix, codex MAJOR re-review): whether the FIRST POSITIONAL
    /// parameter of a Python function node is literally named `self`.
    ///
    /// Unlike the generic `function_parameter_names` extractor (which
    /// flattens every parameter into a bare name list with no
    /// separator/splat context — the exact gap the re-review flagged),
    /// this walks the `parameters` node's named children directly, in
    /// declaration order:
    /// - A bare `*` (`keyword_separator`), `*args` (`list_splat_pattern`),
    ///   or `**kwargs` (`dictionary_splat_pattern`) encountered before any
    ///   plain parameter FAILS the gate outright — there is no positional
    ///   receiver at all (a keyword-only or splat-only signature).
    /// - A bare `/` (`positional_separator`) is transparent: a `self`
    ///   preceding it is still the first positional parameter
    ///   (`def m(self, /)`).
    /// - The first plain parameter encountered — `identifier`,
    ///   `typed_parameter` (`self: T`), `default_parameter`/
    ///   `typed_default_parameter` (`self=...`) — must be named `self` to
    ///   pass; anything else (including a `tuple_pattern`, which can never
    ///   resolve to a plain name) fails.
    ///
    /// Scoped to the P7 property-access path only via
    /// `python_is_self_instance_method` — never consulted by method-CALL
    /// resolution or `method_owner`.
    fn python_first_positional_param_is_self(&self, func_node: &Node<'_>) -> bool {
        let Some(params) = self.find_parameters_node(func_node) else {
            return false;
        };
        let mut cursor = params.walk();
        for child in params.named_children(&mut cursor) {
            match child.kind() {
                "keyword_separator" | "list_splat_pattern" | "dictionary_splat_pattern" => {
                    return false;
                }
                "positional_separator" => continue,
                _ => {
                    return self.extract_param_name(&child).as_deref() == Some("self");
                }
            }
        }
        false
    }

    /// P7 S2: candidate `@property`/`@cached_property` LOAD access sites
    /// (`recv.attr`) within a function body, restricted to `attr_names`
    /// (S1's indexed property names — the "zero cost for normal attribute
    /// traffic" gate: an empty/non-matching `attr_names` short-circuits
    /// before any receiver/store analysis). Excludes:
    /// - a call's function child (`x.attr(...)` — `x.attr` is being CALLED,
    ///   not read; call_graph.rs documents why this must not double-record
    ///   against the ordinary call-resolution path for the same site).
    /// - any attribute node that is (a descendant of) the `left` field of an
    ///   `assignment` or `augmented_assignment` (`x.attr = v` / `x.attr += v`
    ///   — a STORE, never a load; both grammars put the target under `left`).
    /// - (F4) `del`-statement targets, `for`/comprehension targets, and
    ///   `with ... as` alias targets — see `collect_python_attribute_loads`.
    /// - (F3, codex MAJOR 3) anything inside a nested `function_definition`/
    ///   `decorated_definition`/`class_definition`: the walk fences at those
    ///   boundaries so a nested def's body is scanned exactly once, by its
    ///   OWN entry in `all_functions()` — never misattributed to the
    ///   enclosing function. Nested `lambda` bodies are fenced too, but
    ///   (unlike nested defs) nothing else ever scans them: lambdas never
    ///   get their own `FunctionId` from `all_functions()`, so any
    ///   `self.attr`/property access inside a lambda body is an accepted,
    ///   deliberate recall gap (never recorded against anything).
    ///
    /// (F5) Also reports a `store_skips` count: every store/delete-context
    /// attribute access above whose name is S1-indexed, even though it
    /// never becomes a candidate — telemetry for
    /// `CallGraph::property_access_store_skips`.
    pub(crate) fn python_attribute_load_candidates(
        &self,
        func_node: &Node<'_>,
        attr_names: &BTreeSet<String>,
    ) -> PythonAttributeLoadScan {
        if self.language != Language::Python || attr_names.is_empty() {
            return PythonAttributeLoadScan::default();
        }
        let def_node = if func_node.kind() == "decorated_definition" {
            func_node
                .child_by_field_name("definition")
                .unwrap_or(*func_node)
        } else {
            *func_node
        };
        let Some(body) = def_node.child_by_field_name("body") else {
            return PythonAttributeLoadScan::default();
        };
        let mut candidates = Vec::new();
        let mut store_skips = 0usize;
        self.collect_python_attribute_loads(
            body,
            attr_names,
            false,
            &mut candidates,
            &mut store_skips,
        );
        PythonAttributeLoadScan {
            candidates,
            store_skips,
        }
    }

    fn collect_python_attribute_loads(
        &self,
        node: Node<'_>,
        attr_names: &BTreeSet<String>,
        in_store_target: bool,
        out: &mut Vec<PythonAttributeLoadCandidate>,
        store_skips: &mut usize,
    ) {
        // F3 (codex MAJOR 3): fence nested callable/class scopes. The walk
        // starts at a function's OWN body (never at the function node
        // itself — see `python_attribute_load_candidates`), so any of these
        // kinds encountered here is necessarily a NESTED one. The nested
        // def/class gets its own separate scan via its own `all_functions()`
        // entry; recursing into it here would misattribute its accesses to
        // the outer function AND double-scan it. Lambda bodies are fenced
        // too but are never scanned by anything else (lambdas have no
        // `FunctionId`) — an accepted, deliberate recall gap.
        if matches!(
            node.kind(),
            "function_definition" | "decorated_definition" | "lambda" | "class_definition"
        ) {
            return;
        }

        // (F4) `assignment`/`augmented_assignment` LHS and `for`/
        // `for_in_clause` (incl. comprehensions) loop targets share the same
        // shape: the `left` field is a store, every other child keeps the
        // ambient load/store context (the right-hand/iterable side is a
        // LOAD — e.g. `for x in r.text:` — `r.text` IS a load).
        if matches!(
            node.kind(),
            "assignment" | "augmented_assignment" | "for_statement" | "for_in_clause"
        ) {
            self.collect_left_field_as_store(node, attr_names, in_store_target, out, store_skips);
            return;
        }

        // (F4) `del` targets and `with ... as TARGET:` alias targets are
        // always store/delete context, never a load, regardless of the
        // ambient context or internal shape (single attribute, tuple,
        // expression_list, ...). `as_pattern_target` also covers
        // except/match `as` bindings, which is harmless here (same rule).
        if matches!(node.kind(), "delete_statement" | "as_pattern_target") {
            self.collect_all_children_as_store(node, attr_names, out, store_skips);
            return;
        }

        if node.kind() == "attribute" {
            if let Some(attr_field) = node.child_by_field_name("attribute") {
                let name = self.node_text(&attr_field);
                if attr_names.contains(name) {
                    if in_store_target {
                        // F5: a store/delete-context access of an indexed
                        // property name is never a candidate, but IS counted.
                        *store_skips += 1;
                    } else {
                        let is_call_function = node
                            .parent()
                            .filter(|p| p.kind() == "call")
                            .and_then(|p| p.child_by_field_name("function"))
                            .is_some_and(|f| byte_range_eq(&f, &node));
                        if !is_call_function {
                            let receiver_identifier = node
                                .child_by_field_name("object")
                                .filter(|o| o.kind() == "identifier")
                                .map(|o| self.node_text(&o).to_string());
                            out.push(PythonAttributeLoadCandidate {
                                attr_name: name.to_string(),
                                receiver_identifier,
                                line: node.start_position().row + 1,
                                start_byte: node.start_byte(),
                                end_byte: node.end_byte(),
                            });
                        }
                    }
                }
            }
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.collect_python_attribute_loads(
                child,
                attr_names,
                in_store_target,
                out,
                store_skips,
            );
        }
    }

    /// (F4) Shared shape for `assignment`/`augmented_assignment` and
    /// `for`/`for_in_clause`: recurse into `node`'s `left` field as a store
    /// target, and every other child with the ambient `in_store_target`
    /// flag unchanged (so the right-hand/iterable side stays LOAD context
    /// unless it was already nested inside an outer store target).
    fn collect_left_field_as_store(
        &self,
        node: Node<'_>,
        attr_names: &BTreeSet<String>,
        in_store_target: bool,
        out: &mut Vec<PythonAttributeLoadCandidate>,
        store_skips: &mut usize,
    ) {
        if let Some(left) = node.child_by_field_name("left") {
            self.collect_python_attribute_loads(left, attr_names, true, out, store_skips);
        }
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            let is_left = node
                .child_by_field_name("left")
                .is_some_and(|l| byte_range_eq(&l, &child));
            if is_left {
                continue; // already walked above with in_store_target = true.
            }
            self.collect_python_attribute_loads(
                child,
                attr_names,
                in_store_target,
                out,
                store_skips,
            );
        }
    }

    /// (F4) Shared shape for `delete_statement`/`as_pattern_target`: every
    /// child is unconditionally a store/delete target, regardless of the
    /// ambient context.
    fn collect_all_children_as_store(
        &self,
        node: Node<'_>,
        attr_names: &BTreeSet<String>,
        out: &mut Vec<PythonAttributeLoadCandidate>,
        store_skips: &mut usize,
    ) {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.collect_python_attribute_loads(child, attr_names, true, out, store_skips);
        }
    }
}
