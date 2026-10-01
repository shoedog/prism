//! S1b-3 (SPEC §3.1 M1, B3, §3.8 (4)(11)): call-value classification, the closed "provably
//! holds no function" class (NoFn), and the destructuring-default test.
use super::js_binding::JsBinding;
use super::ParsedFile;
use tree_sitter::Node;

impl ParsedFile {
    /// B3's admitted React wrapper (S1's predicate, wrapped) and M1: any other call with a
    /// direct function argument may-calls (base behavior); any other call value is an alias
    /// (owner 2026-09-29). `local_route` (SPEC §3.7, §3.8 (9)): a call-site binding lookup
    /// also admits `useCallback`, a plain (unwrapped) callable; D4 never does.
    pub(super) fn js_ts_classify_call(
        &self,
        decl: Node<'_>,
        call: Node<'_>,
        using: bool,
        local_route: bool,
    ) -> JsBinding {
        let list = decl.parent().filter(|_| !using);
        let admitted = list.and_then(|l| self.js_ts_wrapped_export(l, decl, local_route).ok());
        if let Some((local, start_line, end_line)) = admitted {
            // `wrapped` is true only for forwardRef/memo (bindable from JSX sites only); a
            // `useCallback` admission, possible only on the local route, stays unwrapped.
            let wrapped = list.is_some_and(|l| self.js_ts_wrapped_export(l, decl, false).is_ok());
            return JsBinding::Callable(super::js_binding::JsTerminal {
                local,
                start_line,
                end_line,
                wrapped,
            });
        }
        js_ts_call_may_call(call)
    }
}

/// M1: a call with a direct function argument may return a callable (base behavior, E5); any
/// other call may return one by value flow (`alias`, owner 2026-09-29).
pub(super) fn js_ts_call_may_call(call: Node<'_>) -> JsBinding {
    let Some(args) = call.child_by_field_name("arguments") else {
        return JsBinding::Alias;
    };
    let mut cursor = args.walk();
    let function_argument = args
        .named_children(&mut cursor)
        .any(|a| matches!(a.kind(), "arrow_function" | "function_expression"));
    if function_argument {
        JsBinding::MayCall
    } else {
        JsBinding::Alias
    }
}

/// A destructuring pattern holding a default (`{ a = d }`, `{ k: a = d }`, `[a = d]`) anywhere
/// (owner 2026-09-29, the conservative cut, sol r2 W1).
pub(super) fn js_ts_has_default(pattern: Node<'_>) -> bool {
    let mut stack = vec![pattern];
    while let Some(n) = stack.pop() {
        if matches!(n.kind(), "assignment_pattern" | "object_assignment_pattern") {
            return true;
        }
        let mut cursor = n.walk();
        stack.extend(n.named_children(&mut cursor));
    }
    false
}

/// The closed "provably holds no function" value class (owner 2026-09-29, SPEC §3.8 (11)): a
/// literal, or an object or array literal whose every member value is itself in the class (a
/// shorthand property, a spread, a method or any other expression is not).
pub(super) fn js_ts_holds_no_function(value: Node<'_>) -> bool {
    let mut cursor = value.walk();
    match value.kind() {
        "number" | "string" | "template_string" | "true" | "false" | "null" | "undefined"
        | "regex" => true,
        "array" => value
            .named_children(&mut cursor)
            .all(|e| e.kind() == "comment" || js_ts_holds_no_function(e)),
        "object" => value.named_children(&mut cursor).all(|m| {
            m.kind() == "comment"
                || (m.kind() == "pair"
                    && m.child_by_field_name("value")
                        .is_some_and(js_ts_holds_no_function))
        }),
        _ => false,
    }
}

#[cfg(test)]
#[path = "js_binding_values_tests.rs"]
mod tests;
