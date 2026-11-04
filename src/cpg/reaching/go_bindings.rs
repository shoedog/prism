//! Go lexical introductions exist independently of DataFlow def endpoints.
//! This inventory only assigns ownership; it never produces a value or edge.
use super::*;

pub(super) fn collect(
    parsed: &ParsedFile,
    root: Node<'_>,
    defs: &[DefSite],
) -> (Vec<Declaration>, Vec<Option<usize>>) {
    let mut inventory = Inventory {
        parsed,
        root,
        declarations: Vec::new(),
        occurrences: BTreeMap::new(),
    };
    inventory.walk(root);
    let attached = defs
        .iter()
        .map(|def| {
            // An alias twin names the source at another identifier's bytes.
            // Only a genuine def at this declaration occurrence can attach.
            (!def.alias_derived)
                .then(|| inventory.occurrences.get(&def.start_byte).copied())
                .flatten()
                .filter(|index| inventory.declarations[*index].name == def.path.base)
        })
        .collect();
    (inventory.declarations, attached)
}

/// These introducing fields are fully represented by the inventory below;
/// they must not trigger the grammar heuristic's flat-name fallback.
pub(super) fn classified(kind: &str) -> bool {
    matches!(
        kind,
        "short_var_declaration"
            | "var_spec"
            | "const_spec"
            | "range_clause"
            | "receive_statement"
            | "type_switch_statement"
            | "func_literal"
            | "parameter_declaration"
            | "variadic_parameter_declaration"
            | "type_parameter_declaration"
            | "type_spec"
            | "type_alias"
    )
}

struct Inventory<'a> {
    parsed: &'a ParsedFile,
    root: Node<'a>,
    declarations: Vec<Declaration>,
    occurrences: BTreeMap<usize, usize>,
}

impl Inventory<'_> {
    fn add(
        &mut self,
        name: Node<'_>,
        scope: ScopeSpan,
        visible_from: usize,
        reuse: bool,
        attach: bool,
    ) {
        let text = self.parsed.node_text(&name);
        if text == "_" || !matches!(name.kind(), "identifier" | "type_identifier") {
            return;
        }
        let reused = reuse
            .then(|| {
                self.declarations
                    .iter()
                    .enumerate()
                    .find_map(|(index, declaration)| {
                        (declaration.name == text
                            && declaration.binding.scope == scope
                            && declaration.is_visible_at(name.start_byte()))
                        .then_some(index)
                    })
            })
            .flatten();
        let index = reused.unwrap_or_else(|| {
            let index = self.declarations.len();
            self.declarations.push(Declaration {
                binding: Binding {
                    id: BindingId::Declaration(index),
                    scope,
                    declaration_line: Some(name.start_position().row + 1),
                },
                name: text.to_owned(),
                visible_from,
                additional_visibility: Vec::new(),
            });
            index
        });
        if attach {
            self.occurrences.insert(name.start_byte(), index);
        }
    }

    fn names(
        &mut self,
        node: Node<'_>,
        scope: ScopeSpan,
        visible_from: usize,
        reuse: bool,
        attach: bool,
    ) {
        if matches!(node.kind(), "identifier" | "type_identifier") {
            self.add(node, scope, visible_from, reuse, attach);
        } else if node.kind() == "expression_list" {
            let mut cursor = node.walk();
            for name in node.named_children(&mut cursor) {
                self.add(name, scope, visible_from, reuse, attach);
            }
        }
    }

    fn lexical_scope(&self, node: Node<'_>) -> ScopeSpan {
        let mut parent = node.parent();
        while let Some(candidate) = parent {
            if binding_scope_rule(Language::Go, candidate.kind()).creates_scope {
                return scope_span(candidate);
            }
            if candidate.id() == self.root.id() {
                break;
            }
            parent = candidate.parent();
        }
        function_scope(self.parsed, &self.root)
    }

    fn signature(&mut self, owner: Node<'_>) {
        let scope = function_scope(self.parsed, &owner);
        for field in ["receiver", "parameters", "result"] {
            if let Some(parameters) = owner.child_by_field_name(field) {
                if parameters.kind() != "parameter_list" {
                    continue;
                }
                let mut cursor = parameters.walk();
                for parameter in parameters.named_children(&mut cursor) {
                    self.named_fields(parameter, scope, scope.start_byte);
                    if field == "receiver" {
                        if let Some(ty) = parameter.child_by_field_name("type") {
                            self.receiver_parameters(ty, scope);
                        }
                    }
                }
            }
        }
        self.type_parameters(owner);
    }

    fn receiver_parameters(&mut self, node: Node<'_>, scope: ScopeSpan) {
        if node.kind() == "generic_type" {
            if let Some(arguments) = node.child_by_field_name("type_arguments") {
                // Receiver arguments declare type parameter names; ordinary
                // instantiations elsewhere merely use types.
                fn names(inventory: &mut Inventory<'_>, node: Node<'_>, scope: ScopeSpan) {
                    if node.kind() == "type_identifier" {
                        inventory.add(node, scope, scope.start_byte, false, true);
                        return;
                    }
                    let mut cursor = node.walk();
                    for child in node.named_children(&mut cursor) {
                        names(inventory, child, scope);
                    }
                }
                names(self, arguments, scope);
            }
        } else if node.kind() == "pointer_type" {
            let mut cursor = node.walk();
            for child in node.named_children(&mut cursor) {
                self.receiver_parameters(child, scope);
            }
        }
    }

    fn named_fields(&mut self, node: Node<'_>, scope: ScopeSpan, visible_from: usize) {
        let mut cursor = node.walk();
        for name in node.children_by_field_name("name", &mut cursor) {
            self.add(name, scope, visible_from, false, true);
        }
    }

    fn type_parameters(&mut self, owner: Node<'_>) {
        if let Some(parameters) = owner.child_by_field_name("type_parameters") {
            let mut cursor = parameters.walk();
            for parameter in parameters.named_children(&mut cursor) {
                self.named_fields(parameter, scope_span(owner), owner.start_byte());
            }
        }
    }

    fn walk(&mut self, node: Node<'_>) {
        match node.kind() {
            "function_declaration" | "method_declaration" | "func_literal" => self.signature(node),
            "short_var_declaration" | "receive_statement" | "range_clause" => {
                let mut cursor = node.walk();
                let declares = node.children(&mut cursor).any(|child| child.kind() == ":=");
                if declares {
                    if let Some(left) = node.child_by_field_name("left") {
                        self.names(left, self.lexical_scope(node), node.end_byte(), true, true);
                    }
                }
            }
            "var_spec" | "const_spec" => {
                // In a group, each spec becomes visible after its own RHS,
                // including a const spec with an omitted expression.
                self.named_fields(node, self.lexical_scope(node), node.end_byte());
            }
            "type_spec" | "type_alias" => {
                // A local type is in scope from its TypeSpec, including its
                // own recursive type expression. Type names share the block.
                self.named_fields(node, self.lexical_scope(node), node.start_byte());
                self.type_parameters(node);
            }
            "type_switch_statement" => {
                if let Some(alias) = node.child_by_field_name("alias") {
                    let mut cursor = node.walk();
                    for clause in node.named_children(&mut cursor) {
                        if matches!(clause.kind(), "type_case" | "default_case") {
                            // The header declares a distinct variable in each
                            // clause. It is not visible in the guard or sibling.
                            self.names(
                                alias,
                                scope_span(clause),
                                clause.start_byte(),
                                false,
                                false,
                            );
                        }
                    }
                }
            }
            _ => {}
        }
        let mut cursor = node.walk();
        for child in node.named_children(&mut cursor) {
            self.walk(child);
        }
    }
}
