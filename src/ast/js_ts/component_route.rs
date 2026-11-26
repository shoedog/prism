//! Positive proof of the value invoked by a literal or JSX component site.

use crate::ast::js_binding::{JsBinding, JsBindingCache};
use crate::ast::js_binding_walk::Walk;
use crate::ast::{is_js_ts_function_like, ParsedFile};
use std::collections::BTreeSet;
use tree_sitter::Node;

use super::props_keys::{is_transparent_wrapper, strip_transparent_wrappers};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SiteKind {
    Literal,
    #[allow(dead_code)] // JSX source collection uses this route in S3.
    Jsx,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Route {
    Direct,
    LocalHookOperand,
    JsxRenderOperand,
    UnknownWrapper,
    UnprovedIdentity,
}

pub(crate) fn js_ts_component_route<'tree>(
    site_kind: SiteKind,
    lexical_file: &'tree ParsedFile,
    lexical_site: Node<'tree>,
    selected_file: &ParsedFile,
    selected_callable: Node<'tree>,
    export_proved: bool,
) -> Route {
    let field = match site_kind {
        SiteKind::Literal if lexical_site.kind() == "call_expression" => "function",
        SiteKind::Jsx
            if matches!(
                lexical_site.kind(),
                "jsx_opening_element" | "jsx_self_closing_element"
            ) =>
        {
            "name"
        }
        _ => return Route::UnprovedIdentity,
    };
    let Some(value) = lexical_site.child_by_field_name(field) else {
        return Route::UnprovedIdentity;
    };
    let mut proof = Proof {
        kind: site_kind,
        file: lexical_file,
        selected: (lexical_file.path == selected_file.path).then_some(selected_callable),
        export_proved,
        bindings: JsBindingCache::for_sites(),
        seen: BTreeSet::new(),
    };
    let route = proof.value(value);
    if route != Route::Direct {
        return route;
    }

    // A private-self identifier can name a wrapped operand without traversing
    // its result binding. Apply the enclosing-wrapper check there as well.
    if let Some(call) = operand_call(selected_callable) {
        let Some(decl) = call
            .parent()
            .filter(|parent| parent.kind() == "variable_declarator")
        else {
            return Route::UnknownWrapper;
        };
        if proof.selected.is_some() {
            return proof.wrapper(decl, call);
        }
        let mut target_proof = Proof {
            kind: site_kind,
            file: selected_file,
            selected: Some(selected_callable),
            export_proved,
            bindings: JsBindingCache::for_sites(),
            seen: BTreeSet::new(),
        };
        return target_proof.wrapper(decl, call);
    }
    Route::Direct
}

struct Proof<'a> {
    kind: SiteKind,
    file: &'a ParsedFile,
    selected: Option<Node<'a>>,
    export_proved: bool,
    bindings: JsBindingCache<'a>,
    seen: BTreeSet<usize>,
}

impl<'a> Proof<'a> {
    fn value(&mut self, expression: Node<'a>) -> Route {
        let Some(value) = strip_transparent_wrappers(expression) else {
            return Route::UnprovedIdentity;
        };
        match value.kind() {
            "identifier" => self.name(value),
            "member_expression" if self.export_proved => {
                let qualifier = value
                    .child_by_field_name("object")
                    .filter(|node| node.kind() == "identifier");
                let member = value
                    .child_by_field_name("property")
                    .filter(|node| node.kind() == "property_identifier");
                if member.is_some()
                    && qualifier.is_some_and(|node| {
                        self.file
                            .js_ts_namespace_binding(node, &mut self.bindings)
                            .is_some_and(|(binding, module)| {
                                binding == JsBinding::Import && module.is_some()
                            })
                    })
                {
                    Route::Direct
                } else {
                    Route::UnprovedIdentity
                }
            }
            _ => Route::UnprovedIdentity,
        }
    }

    fn name(&mut self, ident: Node<'a>) -> Route {
        let name = self.file.node_text(&ident);
        let Walk::Found(scope, None) =
            self.file
                .js_ts_binding_walk(ident, name, &mut self.bindings)
        else {
            return Route::UnprovedIdentity;
        };
        let (Some(declarations), false) =
            self.file
                .js_ts_scope_lookup(scope, name, &mut self.bindings)
        else {
            return Route::UnprovedIdentity;
        };
        let mut implementations = declarations
            .into_iter()
            .filter(|decl| decl.kind() != "function_signature");
        let Some(decl) = implementations.next() else {
            return Route::UnprovedIdentity;
        };
        if implementations.next().is_some() || !self.seen.insert(decl.id()) {
            return Route::UnprovedIdentity;
        }
        // For overloads, look up the unique runtime declaration as the explicit
        // scope member. Every existing recovery and write proof still applies.
        let binding =
            self.file
                .js_ts_scope_binding(scope, Some(decl), name, ident, &mut self.bindings, true);
        match decl.kind() {
            "import_statement"
                if binding == JsBinding::Import
                    && self.export_proved
                    && !self.file.js_ts_type_only_imports().contains_key(name) =>
            {
                Route::Direct
            }
            "function_declaration"
            | "generator_function_declaration"
            | "function_expression"
            | "generator_function"
                if matches!(binding, JsBinding::Callable(_)) && self.selected == Some(decl) =>
            {
                Route::Direct
            }
            "variable_declarator"
                if decl
                    .child_by_field_name("name")
                    .is_some_and(|node| node.kind() == "identifier")
                    && evaluated_after(self.file, ident, decl, scope) =>
            {
                let Some(value) = decl
                    .child_by_field_name("value")
                    .and_then(strip_transparent_wrappers)
                else {
                    return Route::UnprovedIdentity;
                };
                if value.kind() == "call_expression"
                    && self
                        .selected
                        .is_some_and(|selected| call_contains_operand(value, selected))
                {
                    return self.wrapper(decl, value);
                }
                if matches!(binding, JsBinding::Callable(_)) && self.selected == Some(value) {
                    Route::Direct
                } else if binding == JsBinding::Alias {
                    self.value(value)
                } else {
                    Route::UnprovedIdentity
                }
            }
            _ => Route::UnprovedIdentity,
        }
    }

    fn wrapper(&mut self, decl: Node<'a>, call: Node<'a>) -> Route {
        let Some(list) = decl.parent() else {
            return Route::UnknownWrapper;
        };
        let Some(name) = decl
            .child_by_field_name("name")
            .filter(|node| node.kind() == "identifier")
        else {
            return Route::UnknownWrapper;
        };
        let Some(operand) = first_argument(call) else {
            return Route::UnknownWrapper;
        };
        if self.selected != Some(operand)
            || call != decl.child_by_field_name("value").unwrap_or(decl)
        {
            return Route::UnknownWrapper;
        }
        let site = call.child_by_field_name("function").unwrap_or(call);
        let name = self.file.node_text(&name);
        // Reuse the initializer proof through the actual stable result
        // binding. This also checks the nearest React import at the call.
        let Walk::Found(scope, None) = self.file.js_ts_binding_walk(site, name, &mut self.bindings)
        else {
            return Route::UnknownWrapper;
        };
        let (Some(declarations), false) =
            self.file
                .js_ts_scope_lookup(scope, name, &mut self.bindings)
        else {
            return Route::UnknownWrapper;
        };
        if declarations.as_slice() != [decl] {
            return Route::UnknownWrapper;
        }
        let JsBinding::Callable(terminal) =
            self.file
                .js_ts_scope_binding(scope, Some(decl), name, site, &mut self.bindings, true)
        else {
            return Route::UnknownWrapper;
        };
        match self.kind {
            SiteKind::Literal if !terminal.wrapped => Route::LocalHookOperand,
            SiteKind::Jsx if terminal.wrapped && at_program_scope(list) => Route::JsxRenderOperand,
            _ => Route::UnknownWrapper,
        }
    }
}

fn evaluated_after(file: &ParsedFile, mut site: Node<'_>, decl: Node<'_>, scope: Node<'_>) -> bool {
    if site.start_byte() >= decl.end_byte() {
        return true;
    }
    while site != scope {
        let Some(parent) = site.parent() else {
            break;
        };
        if parent != scope
            && is_js_ts_function_like(parent.kind())
            && !immediately_invoked(file, parent)
            && ["body", "parameters", "parameter"]
                .iter()
                .any(|field| parent.child_by_field_name(field) == Some(site))
        {
            return true;
        }
        site = parent;
    }
    false
}

fn immediately_invoked(file: &ParsedFile, mut callable: Node<'_>) -> bool {
    while let Some(parent) = callable.parent() {
        if is_transparent_wrapper(parent.kind())
            && strip_transparent_wrappers(parent) == strip_transparent_wrappers(callable)
        {
            callable = parent;
            continue;
        }
        if parent.kind() == "member_expression"
            && parent.child_by_field_name("object") == Some(callable)
            && parent
                .child_by_field_name("property")
                .is_some_and(|property| matches!(file.node_text(&property), "call" | "apply"))
        {
            callable = parent;
            continue;
        }
        return match parent.kind() {
            "call_expression" => parent.child_by_field_name("function") == Some(callable),
            "new_expression" => parent.child_by_field_name("constructor") == Some(callable),
            _ => false,
        };
    }
    false
}

fn at_program_scope(mut declaration: Node<'_>) -> bool {
    if let Some(parent) = declaration
        .parent()
        .filter(|node| node.kind() == "export_statement")
    {
        declaration = parent;
    }
    declaration
        .parent()
        .is_some_and(|node| node.kind() == "program")
}

fn first_argument(call: Node<'_>) -> Option<Node<'_>> {
    let args = call.child_by_field_name("arguments")?;
    let mut cursor = args.walk();
    let first = args
        .named_children(&mut cursor)
        .find(|node| !node.is_extra());
    first
}

fn call_contains_operand(call: Node<'_>, selected: Node<'_>) -> bool {
    let mut nested = operand_call(selected);
    while let Some(node) = nested {
        if node == call {
            return true;
        }
        nested = operand_call(node);
    }
    false
}

fn operand_call(mut operand: Node<'_>) -> Option<Node<'_>> {
    let mut parent = operand.parent()?;
    while is_transparent_wrapper(parent.kind())
        && strip_transparent_wrappers(parent) == strip_transparent_wrappers(operand)
    {
        operand = parent;
        parent = parent.parent()?;
    }
    let call = parent.parent()?;
    (parent.kind() == "arguments"
        && call.kind() == "call_expression"
        && call.child_by_field_name("arguments") == Some(parent))
    .then_some(call)
}

#[cfg(test)]
pub(super) fn wrapper_census_counts(attempts: &[(SiteKind, bool, Route)]) -> (usize, usize) {
    attempts
        .iter()
        .filter(|&&(kind, exact, _)| kind == SiteKind::Jsx && exact)
        .fold((0, 0), |(wrapped, exact), &(_, _, route)| {
            (
                wrapped + usize::from(route == Route::UnknownWrapper),
                exact + 1,
            )
        })
}
