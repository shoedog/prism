use super::*;

pub(super) fn declaration_scope(
    parsed: &ParsedFile,
    func_node: &Node<'_>,
    node: Node<'_>,
    kind: DeclarationKind,
    function_scope: ScopeSpan,
) -> ScopeSpan {
    if kind == DeclarationKind::JavaScriptVar {
        return function_scope;
    }
    if kind == DeclarationKind::Pattern {
        if let Some(scope) = rust_scope::pattern_declaration_scope(parsed, node) {
            return scope;
        }
    }
    if kind == DeclarationKind::CaptureCopy {
        if let Some(scope) = cpp::capture_body_scope(node) {
            return scope_span(scope);
        }
    }
    if let Some(scope) = rust_scope::closure_parameter_scope(parsed, node) {
        return scope;
    }
    let mut current = node.parent();
    while let Some(parent) = current {
        if parent.id() == func_node.id() {
            break;
        }
        if binding_scope_rule_for(parsed, parent.kind()).creates_scope {
            if let Some(owner) = cpp::condition_scope_owner(parsed.language, parent) {
                return scope_span(owner);
            }
            if parsed.language == Language::Lua {
                return lua::lexical_scope(parsed.language, parent);
            }
            return c::scope_span(parsed, parent);
        }
        current = parent.parent();
    }
    function_scope
}
