//! Source-aware compatibility view for Go 0.25's ambiguous indexed callees.
//! Only call position changed from a callee plus type_arguments to an index.
//! Standalone F[T] values retain their existing index reading. No raw tree or
//! resolver confidence is rewritten: the ordinary resolution gates still apply.
use super::ParsedFile;
use crate::languages::Language;
use std::collections::BTreeMap;
use tree_sitter::Node;

#[derive(Clone, Default)]
pub(super) struct Facts {
    values: BTreeMap<String, Vec<Binding>>,
    types: BTreeMap<String, Vec<Binding>>,
    generics: BTreeMap<String, ()>,
}

#[derive(Clone)]
struct Binding {
    lo: usize,
    hi: usize,
    depth: usize,
}

impl Binding {
    fn new(scope: Node<'_>, lo: usize) -> Self {
        let mut depth = 0;
        let mut parent = scope.parent();
        while let Some(node) = parent {
            depth += 1;
            parent = node.parent();
        }
        Self {
            lo,
            hi: scope.end_byte(),
            depth,
        }
    }
}

impl Facts {
    fn new(p: &ParsedFile) -> Self {
        fn scope(n: Node<'_>) -> Node<'_> {
            let mut parent = n.parent();
            while let Some(s) = parent {
                if matches!(
                    s.kind(),
                    "source_file"
                        | "block"
                        | "function_declaration"
                        | "method_declaration"
                        | "func_literal"
                        | "if_statement"
                        | "for_statement"
                        | "expression_case"
                        | "type_case"
                        | "communication_case"
                        | "default_case"
                        | "expression_switch_statement"
                        | "type_switch_statement"
                ) {
                    return s;
                }
                parent = s.parent();
            }
            n
        }
        fn walk(p: &ParsedFile, n: Node<'_>, facts: &mut Facts) {
            if n.kind() == "function_declaration"
                && n.child_by_field_name("type_parameters").is_some()
            {
                if let Some(name) = n.child_by_field_name("name") {
                    facts.generics.insert(p.node_text(&name).to_owned(), ());
                }
            }
            let parameter = matches!(
                n.kind(),
                "parameter_declaration" | "variadic_parameter_declaration"
            );
            // Only parameters/results/receivers directly owned by a callable
            // bind values. Function types and interface signatures do not.
            let callable_parameter = parameter
                && n.parent()
                    .and_then(|list| list.parent())
                    .is_some_and(|owner| {
                        matches!(
                            owner.kind(),
                            "function_declaration" | "method_declaration" | "func_literal"
                        )
                    });
            let field = match n.kind() {
                "parameter_declaration" | "variadic_parameter_declaration"
                    if callable_parameter =>
                {
                    Some("name")
                }
                "var_spec" | "const_spec" => Some("name"),
                "short_var_declaration" | "range_clause" => Some("left"),
                _ => None,
            };
            if let Some(field) = field {
                let scope = scope(n);
                let lo = if parameter
                    || n.parent()
                        .and_then(|s| s.parent())
                        .is_some_and(|s| s.kind() == "source_file")
                {
                    scope.start_byte()
                } else {
                    n.end_byte()
                };
                let mut cursor = n.walk();
                for child in n.children_by_field_name(field, &mut cursor) {
                    let mut cursor = child.walk();
                    let names = if child.kind() == "expression_list" {
                        child.named_children(&mut cursor).collect::<Vec<_>>()
                    } else {
                        vec![child]
                    };
                    for name in names.into_iter().filter(|n| n.kind() == "identifier") {
                        facts
                            .values
                            .entry(p.node_text(&name).to_owned())
                            .or_default()
                            .push(Binding::new(scope, lo));
                    }
                }
            }
            let type_scope = match n.kind() {
                "type_parameter_declaration" => {
                    n.parent().and_then(|list| list.parent()).filter(|owner| {
                        matches!(
                            owner.kind(),
                            "function_declaration" | "type_spec" | "type_alias"
                        )
                    })
                }
                "type_spec" | "type_alias" => Some(scope(n)),
                _ => None,
            };
            if let Some(scope) = type_scope {
                let mut cursor = n.walk();
                for name in n.children_by_field_name("name", &mut cursor) {
                    facts
                        .types
                        .entry(p.node_text(&name).to_owned())
                        .or_default()
                        .push(Binding::new(scope, n.start_byte()));
                }
            }
            let mut cursor = n.walk();
            for child in n.named_children(&mut cursor) {
                walk(p, child, facts);
            }
        }
        let mut facts = Self::default();
        walk(p, p.tree.root_node(), &mut facts);
        facts
    }
    fn is_value(&self, name: &str, at: usize) -> bool {
        let nearest = |bindings: &BTreeMap<String, Vec<Binding>>| {
            bindings
                .get(name)
                .into_iter()
                .flatten()
                .filter(|b| b.lo <= at && at < b.hi)
                .map(|b| b.depth)
                .max()
        };
        // Go shares one namespace: an inner type hides an outer value, and
        // an inner value still vetoes a type argument. Ties stay conservative.
        nearest(&self.values).is_some_and(|value| nearest(&self.types).is_none_or(|ty| value >= ty))
    }
}

impl ParsedFile {
    /// The semantic callee expression, flattening a legacy-compatible generic call wrapper.
    /// The source-less Language helpers remain available for raw syntax users.
    pub fn call_function_node<'t>(&self, call: &Node<'t>) -> Option<Node<'t>> {
        let function = call.child_by_field_name("function")?;
        Some(self.go_generic_call_operand(function).unwrap_or(function))
    }

    pub fn call_function_name<'t>(&self, call: &Node<'t>) -> Option<Node<'t>> {
        if self.language == Language::Go {
            if let Some(function) = self.call_function_node(call) {
                return Some(if function.kind() == "selector_expression" {
                    function.child_by_field_name("field").unwrap_or(function)
                } else {
                    function
                });
            }
        }
        self.language.call_function_name(call)
    }

    pub fn call_function_qualifier<'t>(&self, call: &Node<'t>) -> Option<Node<'t>> {
        if self.language == Language::Go {
            return self
                .call_function_node(call)
                .filter(|n| n.kind() == "selector_expression")
                .and_then(|n| n.child_by_field_name("operand"));
        }
        self.language.call_function_qualifier(call)
    }

    fn go_generic_call_operand<'t>(&self, n: Node<'t>) -> Option<Node<'t>> {
        if self.language != Language::Go
            || n.kind() != "index_expression"
            || !n.parent().is_some_and(|p| {
                p.kind() == "call_expression" && p.child_by_field_name("function") == Some(n)
            })
        {
            return None;
        }
        let base = n.child_by_field_name("operand")?;
        if !matches!(base.kind(), "identifier" | "selector_expression") {
            return None;
        }
        let facts = self.go_call_facts.get_or_init(|| Facts::new(self));
        // A known value (including a slice/map of funcs) cannot be instantiated.
        if base.kind() == "identifier" && facts.is_value(self.node_text(&base), n.start_byte()) {
            return None;
        }
        let index = n.child_by_field_name("index")?;
        // A declared generic function also proves the reading, but its bracket
        // must still have the type syntax accepted by the pre-bump grammar.
        let generic =
            base.kind() == "identifier" && facts.generics.contains_key(self.node_text(&base));
        self.go_type_argument_shape(index, facts, generic)
            .then_some(base)
    }

    fn go_type_argument_shape(&self, n: Node<'_>, facts: &Facts, generic: bool) -> bool {
        match n.kind() {
            // Go has no separate type namespace. A visible value rules out the
            // type reading. Unbound names may be package types in another file;
            // the old grammar accepted precisely this identifier type syntax.
            "identifier" => generic || !facts.is_value(self.node_text(&n), n.start_byte()),
            "selector_expression" => n.child_by_field_name("operand").is_some_and(|base| {
                base.kind() == "identifier"
                    && !facts.is_value(self.node_text(&base), n.start_byte())
            }),
            "unary_expression" => {
                n.child_by_field_name("operator")
                    .is_some_and(|op| self.node_text(&op) == "*")
                    && n.child_by_field_name("operand")
                        .is_some_and(|inner| self.go_type_argument_shape(inner, facts, generic))
            }
            "parenthesized_expression" => n
                .named_child(0)
                .is_some_and(|inner| self.go_type_argument_shape(inner, facts, generic)),
            _ => false,
        }
    }
}
