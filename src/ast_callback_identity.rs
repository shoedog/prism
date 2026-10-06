//! js-param-defs PR-B ("callback identity"): DFG-only owner identity for
//! anonymous JS/TS/TSX callables, own-scope containment for their passes, and
//! the byte-precise binder fence (E3) shared by every JS/TS DFG reference walk.
//! SPEC: docs/superpowers/plans/2026-10-06-js-param-defs/SPEC-prB.md.
//!
//! The synthetic identity lives ONLY in `VarLocation.function` of DFG rows.
//! `FunctionInfo.name`, `Language::function_name`, the call graph, resolution,
//! seeds and navigation symbols never see it, so it cannot become a callee, a
//! caller, an owner-index key, a `free_single` target or a symbol seed.

use super::*;

/// Prefix of every synthetic owner name. `<` cannot start an ECMAScript
/// identifier, and every name `function_name` can return for JS/TS is an
/// identifier, a property name, a string literal (with its quotes) or a
/// computed key (with its bracket), so no real owner can spell it.
pub const SYNTHETIC_OWNER_PREFIX: &str = "<cb@";

/// Whether a DFG owner name is a PR-B synthetic callable identity.
pub fn is_synthetic_owner(name: &str) -> bool {
    name.starts_with(SYNTHETIC_OWNER_PREFIX)
}

impl ParsedFile {
    /// Owner name of a callable's DFG pass: the inferred/declared name when
    /// one exists (name inference always wins), else the synthetic
    /// `<cb@LINE:COL>` identity for an anonymous JS/TS/TSX arrow or function
    /// expression, else `None` (the callable keeps getting no pass).
    pub fn dfg_owner_name(&self, func_node: &Node<'_>) -> Option<String> {
        if let Some(name) = self.language.function_name(func_node) {
            return Some(self.node_text(&name).to_string());
        }
        self.js_ts_synthetic_callable_name(func_node)
    }

    /// `<cb@LINE:COL>` (1-based line, 1-based byte column of the callable's
    /// first byte) for an unnamed JS/TS/TSX `arrow_function` or
    /// `function_expression`. Two distinct callables never share a first byte.
    pub fn js_ts_synthetic_callable_name(&self, func_node: &Node<'_>) -> Option<String> {
        if !matches!(
            self.language,
            Language::JavaScript | Language::TypeScript | Language::Tsx
        ) || !matches!(func_node.kind(), "arrow_function" | "function_expression")
            || self.language.function_name(func_node).is_some()
        {
            return None;
        }
        let start = func_node.start_position();
        Some(format!(
            "{SYNTHETIC_OWNER_PREFIX}{}:{}>",
            start.row + 1,
            start.column + 1
        ))
    }

    /// Own-scope containment for a synthetic pass: the byte span lies inside
    /// `func_node` and its innermost enclosing callable boundary IS `func_node`.
    /// Spans inside nested callables belong to those callables' own passes;
    /// spans outside the callable (same line, E1) belong to no synthetic pass.
    pub(crate) fn js_ts_span_in_own_scope(
        &self,
        func_node: &Node<'_>,
        start_byte: usize,
        end_byte: usize,
    ) -> bool {
        if start_byte < func_node.start_byte() || func_node.end_byte() < end_byte {
            return false;
        }
        let Some(leaf) = self
            .tree
            .root_node()
            .descendant_for_byte_range(start_byte, end_byte)
        else {
            return false;
        };
        let boundaries = self.language.callable_boundary_node_types();
        let mut current = Some(leaf);
        while let Some(node) = current {
            if node.id() == func_node.id() {
                return true;
            }
            if boundaries.contains(&node.kind()) {
                return false;
            }
            current = node.parent();
        }
        false
    }

    /// E3 binder fence (JS/TS/TSX only). A reference `reference` to `name`,
    /// found while walking `owner` for a Def whose first byte is `def_byte`,
    /// is fenced when an ancestor scope strictly between the reference and
    /// the owner introduces its own binding of `name` and does not contain
    /// the Def. Binders: nested callable formals and `var`/function-scope
    /// declarations, function/class-expression self names, catch
    /// parameters, `let`/`const` for/for-in/for-of heads and block-level
    /// lexical/function/class declarations. Escaped binding spellings may
    /// bind any name and fence (fail closed).
    pub(crate) fn js_ts_reference_fenced(
        &self,
        reference: &Node<'_>,
        owner: &Node<'_>,
        name: &str,
        def_byte: usize,
    ) -> bool {
        if !matches!(
            self.language,
            Language::JavaScript | Language::TypeScript | Language::Tsx
        ) {
            return false;
        }
        let boundaries = self.language.callable_boundary_node_types();
        let mut current = reference.parent();
        while let Some(scope) = current {
            if scope.id() == owner.id() {
                return false;
            }
            let contains_def = scope.start_byte() <= def_byte && def_byte < scope.end_byte();
            if !contains_def && self.js_ts_scope_binds(scope, &boundaries, name) {
                return true;
            }
            current = scope.parent();
        }
        false
    }

    fn js_ts_scope_binds(&self, scope: Node<'_>, boundaries: &[&str], name: &str) -> bool {
        if boundaries.contains(&scope.kind()) {
            if let Some(params) = self
                .find_parameters_node(&scope)
                .or_else(|| scope.child_by_field_name("parameter"))
            {
                if self.js_ts_binding_pattern_may_bind(params, name) {
                    return true;
                }
            }
            if matches!(scope.kind(), "function_expression" | "generator_function")
                && scope
                    .child_by_field_name("name")
                    .is_some_and(|n| self.js_ts_binding_pattern_may_bind(n, name))
            {
                return true;
            }
            // Function-scope (`var`, hoisted declarations) of the nested callable.
            return scope
                .child_by_field_name("body")
                .is_some_and(|body| self.js_ts_function_scope_binds(body, boundaries, name));
        }
        match scope.kind() {
            "class" => scope
                .child_by_field_name("name")
                .is_some_and(|n| self.js_ts_binding_pattern_may_bind(n, name)),
            "catch_clause" => scope
                .child_by_field_name("parameter")
                .is_some_and(|p| self.js_ts_binding_pattern_may_bind(p, name)),
            "for_in_statement" => {
                scope
                    .child_by_field_name("kind")
                    .is_some_and(|k| matches!(self.node_text(&k), "let" | "const"))
                    && scope
                        .child_by_field_name("left")
                        .is_some_and(|l| self.js_ts_binding_pattern_may_bind(l, name))
            }
            "for_statement" => scope
                .child_by_field_name("initializer")
                .filter(|init| init.kind() == "lexical_declaration")
                .is_some_and(|init| self.js_ts_declaration_binds(init, name)),
            "statement_block" | "switch_body" => {
                let mut cursor = scope.walk();
                let found = scope
                    .named_children(&mut cursor)
                    .any(|stmt| self.js_ts_block_statement_binds(stmt, name));
                found
            }
            _ => false,
        }
    }

    fn js_ts_block_statement_binds(&self, stmt: Node<'_>, name: &str) -> bool {
        match stmt.kind() {
            "lexical_declaration" => self.js_ts_declaration_binds(stmt, name),
            "function_declaration"
            | "generator_function_declaration"
            | "class_declaration"
            | "abstract_class_declaration" => stmt
                .child_by_field_name("name")
                .is_some_and(|n| self.js_ts_binding_pattern_may_bind(n, name)),
            "switch_case" | "switch_default" => {
                let mut cursor = stmt.walk();
                let found = stmt
                    .named_children(&mut cursor)
                    .any(|s| self.js_ts_block_statement_binds(s, name));
                found
            }
            _ => false,
        }
    }

    fn js_ts_declaration_binds(&self, decl: Node<'_>, name: &str) -> bool {
        let mut cursor = decl.walk();
        let found = decl.named_children(&mut cursor).any(|d| {
            d.kind() == "variable_declarator"
                && d.child_by_field_name("name")
                    .is_some_and(|n| self.js_ts_binding_pattern_may_bind(n, name))
        });
        found
    }

    /// Whether a nested callable's function scope binds `name`: `var`
    /// declarators anywhere in its body outside further nested callables, and
    /// its body-level lexical/function/class declarations.
    fn js_ts_function_scope_binds(&self, body: Node<'_>, boundaries: &[&str], name: &str) -> bool {
        let mut stack = vec![body];
        while let Some(node) = stack.pop() {
            if node.id() != body.id() && boundaries.contains(&node.kind()) {
                continue;
            }
            if node.kind() == "variable_declaration" && self.js_ts_declaration_binds(node, name) {
                return true;
            }
            // `for (var k in o)` declares a function-scoped `k` without a
            // `variable_declaration` node.
            if node.kind() == "for_in_statement"
                && node
                    .child_by_field_name("kind")
                    .is_some_and(|k| self.node_text(&k) == "var")
                && node
                    .child_by_field_name("left")
                    .is_some_and(|l| self.js_ts_binding_pattern_may_bind(l, name))
            {
                return true;
            }
            if node.id() == body.id() && node.kind() == "statement_block" {
                let mut cursor = node.walk();
                if node
                    .named_children(&mut cursor)
                    .any(|s| self.js_ts_block_statement_binds(s, name))
                {
                    return true;
                }
            }
            let mut cursor = node.walk();
            stack.extend(node.named_children(&mut cursor));
        }
        false
    }

    /// PR-B reference-position classifier (E7 must not grow): an rvalue span
    /// whose exact node is a property/attribute/label name is not a variable
    /// reference (pair keys, JSX attribute names, member and callee property
    /// names, method names). Shorthand `{x}` stays a reference. Applied to
    /// synthetic passes only; legacy passes keep base parity (SPEC-prB D9).
    pub(crate) fn js_ts_span_is_non_reference(&self, start_byte: usize, end_byte: usize) -> bool {
        self.tree
            .root_node()
            .descendant_for_byte_range(start_byte, end_byte)
            .filter(|n| n.start_byte() == start_byte && n.end_byte() == end_byte)
            .is_some_and(|n| {
                matches!(
                    n.kind(),
                    "property_identifier"
                        | "private_property_identifier"
                        | "statement_identifier"
                        | "type_identifier"
                )
            })
    }

    /// PR-B (E4): an rvalue span on a write-only or binding occurrence
    /// (`return x = f() || x` collects the LHS `x` too) must not become the
    /// line's preferred Use when the same line also reads `x`.
    pub(crate) fn js_ts_span_is_not_a_read(&self, start_byte: usize, end_byte: usize) -> bool {
        self.tree
            .root_node()
            .descendant_for_byte_range(start_byte, end_byte)
            .filter(|n| n.start_byte() == start_byte && n.end_byte() == end_byte)
            .filter(|n| n.kind() == "identifier")
            .is_some_and(|n| self.js_ts_identifier_is_not_a_read(n))
    }

    /// PR-B D11: the alias a synthetic pass may derive from one own-scope
    /// lvalue identifier: `a = b` / `var a = b` → `b`; `const {k} = o` /
    /// `const {k: a} = o` → `o.k`. Anything else (augmented assignment, array
    /// or nested patterns, non-identifier RHS) establishes no alias.
    pub(crate) fn js_ts_own_alias_target(
        &self,
        start_byte: usize,
        end_byte: usize,
    ) -> Option<String> {
        let ident = self
            .tree
            .root_node()
            .descendant_for_byte_range(start_byte, end_byte)
            .filter(|n| {
                n.start_byte() == start_byte
                    && n.end_byte() == end_byte
                    && matches!(
                        n.kind(),
                        "identifier" | "shorthand_property_identifier_pattern"
                    )
            })?;
        let parent = ident.parent()?;
        let plain_rhs = |value: Option<Node<'_>>| {
            value
                .filter(|v| v.kind() == "identifier")
                .map(|v| self.node_text(&v).to_string())
        };
        match parent.kind() {
            "variable_declarator"
                if parent.child_by_field_name("name").map(|n| n.id()) == Some(ident.id()) =>
            {
                plain_rhs(parent.child_by_field_name("value"))
            }
            "assignment_expression"
                if parent.child_by_field_name("left").map(|n| n.id()) == Some(ident.id()) =>
            {
                plain_rhs(parent.child_by_field_name("right"))
            }
            "object_pattern" | "pair_pattern" => {
                let (key, pattern) = if parent.kind() == "pair_pattern" {
                    if parent.child_by_field_name("value").map(|n| n.id()) != Some(ident.id()) {
                        return None;
                    }
                    let key = parent.child_by_field_name("key")?;
                    if key.kind() != "property_identifier" {
                        return None;
                    }
                    (self.node_text(&key).to_string(), parent.parent()?)
                } else {
                    (self.node_text(&ident).to_string(), parent)
                };
                if pattern.kind() != "object_pattern" {
                    return None;
                }
                let holder = pattern.parent()?;
                let value = match holder.kind() {
                    "variable_declarator"
                        if holder.child_by_field_name("name").map(|n| n.id())
                            == Some(pattern.id()) =>
                    {
                        holder.child_by_field_name("value")
                    }
                    _ => None,
                };
                plain_rhs(value).map(|object| format!("{object}.{key}"))
            }
            _ => None,
        }
    }

    /// PR-B (E4 must not grow in new passes): an identifier occurrence that
    /// only introduces a binding or is only written is not a read.
    pub(crate) fn js_ts_identifier_is_not_a_read(&self, ident: Node<'_>) -> bool {
        if self.language.is_in_erased_type_context(ident) {
            return true;
        }
        let mut child = ident;
        let mut parent = ident.parent();
        while let Some(p) = parent {
            match p.kind() {
                // Destructuring wrappers: keep climbing to the binding site.
                "object_pattern"
                | "array_pattern"
                | "rest_pattern"
                | "shorthand_property_identifier_pattern" => {}
                "pair_pattern" => {
                    if p.child_by_field_name("value").map(|v| v.id()) != Some(child.id()) {
                        return false;
                    }
                }
                "assignment_pattern" | "object_assignment_pattern" => {
                    if p.child_by_field_name("left").map(|v| v.id()) != Some(child.id()) {
                        return false;
                    }
                }
                "variable_declarator" => {
                    return p.child_by_field_name("name").map(|v| v.id()) == Some(child.id());
                }
                "assignment_expression" => {
                    return p.child_by_field_name("left").map(|v| v.id()) == Some(child.id());
                }
                "for_in_statement" => {
                    return p.child_by_field_name("left").map(|v| v.id()) == Some(child.id());
                }
                "catch_clause" => {
                    return p.child_by_field_name("parameter").map(|v| v.id()) == Some(child.id());
                }
                "formal_parameters" => return true,
                "required_parameter" | "optional_parameter" => {
                    return p.child_by_field_name("pattern").map(|v| v.id()) == Some(child.id());
                }
                "arrow_function" => {
                    return p.child_by_field_name("parameter").map(|v| v.id()) == Some(child.id());
                }
                "function_expression"
                | "function_declaration"
                | "generator_function"
                | "generator_function_declaration"
                | "class"
                | "class_declaration"
                | "abstract_class_declaration"
                | "import_specifier"
                | "namespace_import"
                | "import_clause" => {
                    return p.child_by_field_name("name").map(|v| v.id()) == Some(child.id())
                        || matches!(
                            p.kind(),
                            "import_specifier" | "namespace_import" | "import_clause"
                        );
                }
                _ => return false,
            }
            child = p;
            parent = p.parent();
        }
        false
    }

    /// PR-B Def-scope fence for synthetic passes (E10 must not grow): the
    /// byte region a lexical Def binds in. `let`/`const` declarators bind in
    /// their enclosing block / `for` head; `var`, assignments and formals are
    /// function-scoped (`None` = the whole owner).
    pub(crate) fn js_ts_lexical_def_scope(
        &self,
        def_start: usize,
        def_end: usize,
    ) -> Option<(usize, usize)> {
        let node = self
            .tree
            .root_node()
            .descendant_for_byte_range(def_start, def_end)?;
        let mut current = node.parent();
        let mut declarator_seen = false;
        while let Some(n) = current {
            match n.kind() {
                "variable_declarator" => declarator_seen = true,
                "object_pattern"
                | "array_pattern"
                | "pair_pattern"
                | "assignment_pattern"
                | "object_assignment_pattern"
                | "rest_pattern"
                | "shorthand_property_identifier_pattern" => {}
                "lexical_declaration" if declarator_seen => {
                    let scope = n.parent()?;
                    return Some((scope.start_byte(), scope.end_byte()));
                }
                // `for (const k of …)` / `for (let k in …)`: the loop owns k.
                "for_in_statement" => {
                    let lexical = n
                        .child_by_field_name("kind")
                        .is_some_and(|k| matches!(self.node_text(&k), "let" | "const"));
                    return lexical.then(|| (n.start_byte(), n.end_byte()));
                }
                _ => return None,
            }
            current = n.parent();
        }
        None
    }

    /// PR-B (synthetic passes): `this.x` in a nested non-arrow function, a
    /// method or a class body names a different receiver than the owner's
    /// `this`, so an owner Def of `this.x` never reaches it.
    pub(crate) fn js_ts_this_rebound_between(
        &self,
        reference: &Node<'_>,
        owner: &Node<'_>,
    ) -> bool {
        let mut current = reference.parent();
        while let Some(scope) = current {
            if scope.id() == owner.id() {
                return false;
            }
            if matches!(
                scope.kind(),
                "function_expression"
                    | "function_declaration"
                    | "generator_function"
                    | "generator_function_declaration"
                    | "method_definition"
                    | "class_body"
            ) {
                return true;
            }
            current = scope.parent();
        }
        false
    }

    /// `find_path_references_scoped` plus the E3 binder fence for JS/TS/TSX.
    /// Other languages delegate unchanged (byte-identical).
    pub fn find_path_references_fenced(
        &self,
        func_node: &Node<'_>,
        path: &crate::access_path::AccessPath,
        def_line: usize,
        def_byte: usize,
    ) -> BTreeSet<usize> {
        self.find_path_references_fenced_in(func_node, path, def_line, def_byte, None, false)
    }

    /// `find_path_references_fenced` restricted to references inside
    /// `within` (a synthetic pass's lexical Def scope). `reads_only` (synthetic
    /// passes, E4 must not grow) skips identifiers that only bind or are only
    /// written (declaration names, formals, plain `=` targets, for-in/of heads,
    /// catch parameters) and identifiers in erased type positions.
    pub(crate) fn find_path_references_fenced_in(
        &self,
        func_node: &Node<'_>,
        path: &crate::access_path::AccessPath,
        def_line: usize,
        def_byte: usize,
        within: Option<(usize, usize)>,
        reads_only: bool,
    ) -> BTreeSet<usize> {
        if !matches!(
            self.language,
            Language::JavaScript | Language::TypeScript | Language::Tsx
        ) {
            return self.find_path_references_scoped(func_node, path, def_line);
        }
        let mut lines = BTreeSet::new();
        let scope_root = within
            .and_then(|(s, e)| self.tree.root_node().descendant_for_byte_range(s, e))
            .filter(|n| within == Some((n.start_byte(), n.end_byte())))
            .unwrap_or(*func_node);
        if path.is_simple() {
            self.collect_fenced_simple_refs(
                scope_root, func_node, &path.base, def_line, def_byte, reads_only, &mut lines,
            );
        } else {
            self.collect_fenced_path_refs(
                scope_root, func_node, path, def_line, def_byte, reads_only, &mut lines,
            );
        }
        lines
    }

    #[allow(clippy::too_many_arguments)]
    fn collect_fenced_simple_refs(
        &self,
        node: Node<'_>,
        owner: &Node<'_>,
        name: &str,
        def_line: usize,
        def_byte: usize,
        reads_only: bool,
        out: &mut BTreeSet<usize>,
    ) {
        use crate::queries::{get_query, QueryKind};
        use tree_sitter::StreamingIterator;
        let Some(query) = get_query(self.language, QueryKind::Identifiers) else {
            out.extend(self.find_variable_references_scoped(owner, name, def_line));
            return;
        };
        let ident_idx = query
            .capture_index_for_name("ident")
            .expect("Identifiers query must have @ident capture");
        let mut cursor = tree_sitter::QueryCursor::new();
        cursor.set_byte_range(node.byte_range());
        let mut matches = cursor.matches(query, self.tree.root_node(), self.source.as_bytes());
        while let Some(m) = matches.next() {
            for capture in m.captures {
                if capture.index == ident_idx
                    && self.node_text(&capture.node) == name
                    && !self.is_shadowed_at(&capture.node, owner, name, def_line)
                    && !self.js_ts_reference_fenced(&capture.node, owner, name, def_byte)
                    && !(reads_only && self.js_ts_identifier_is_not_a_read(capture.node))
                {
                    out.insert(capture.node.start_position().row + 1);
                }
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn collect_fenced_path_refs(
        &self,
        node: Node<'_>,
        owner: &Node<'_>,
        path: &crate::access_path::AccessPath,
        def_line: usize,
        def_byte: usize,
        reads_only: bool,
        out: &mut BTreeSet<usize>,
    ) {
        let line = node.start_position().row + 1;
        if Self::is_field_access_node(node.kind()) {
            let node_path = self
                .bounded_member_path(node)
                .map(|member| member.path)
                .unwrap_or_else(|| {
                    crate::access_path::AccessPath::from_expr(self.node_text(&node))
                });
            if node_path == *path && line > def_line {
                let this_rebound = reads_only
                    && path.base == "this"
                    && self.js_ts_this_rebound_between(&node, owner);
                if !this_rebound && !self.js_ts_reference_fenced(&node, owner, &path.base, def_byte)
                {
                    out.insert(line);
                }
                return;
            }
        }
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.collect_fenced_path_refs(child, owner, path, def_line, def_byte, reads_only, out);
        }
    }
}
