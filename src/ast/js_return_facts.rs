//! Slice V: the returned-value fact of one JS/TS function, computed from syntax. The fact says,
//! for each object key or array slot (or for the whole value), which one callable the function
//! returns there, as a callable value in the function's own file. It is a fact about the
//! function only: nothing here looks at a call site or follows a second returned-value step.
//! A selection that is not provably one callable at every value-returning statement is simply
//! absent, and the resolver then leaves the site as it was.
use super::js_binding::{JsBinding, JsBindingCache, JsTerminal};
use super::{is_js_ts_function_like, ParsedFile};
use crate::ast::JsFollow;
use crate::js_returns::JsReturnFacts;
use std::collections::BTreeMap;
use tree_sitter::Node;

/// What one `return` expression hands back, per selector. Absent selectors are not callable
/// (or not provable) at this statement.
#[derive(Default)]
struct Selection {
    whole: Option<JsFollow>,
    keys: BTreeMap<String, JsFollow>,
    slots: BTreeMap<usize, JsFollow>,
}

impl Selection {
    /// Both statements must select the same callable identity for a selector to survive.
    fn meet(mut self, other: Selection) -> Selection {
        self.whole = self.whole.filter(|w| other.whole.as_ref() == Some(w));
        self.keys.retain(|k, v| other.keys.get(k) == Some(v));
        self.slots.retain(|i, v| other.slots.get(i) == Some(v));
        self
    }
}

/// Whether `expr` ends a `return`'s value as nothing (`null`, `undefined`).
fn is_nullish(expr: Node<'_>) -> bool {
    matches!(expr.kind(), "null" | "undefined")
}

impl ParsedFile {
    /// The returned-value fact of the function `func`, or `None` when it has none: a parse error
    /// inside it, an `async` or generator function, no value-returning statement, or any
    /// value-returning statement that is not an object literal, an array literal, a unique
    /// unwritten `const` of one of those, or a callable value.
    pub(crate) fn js_ts_return_facts<'a>(
        &'a self,
        func: Node<'a>,
        cache: &mut JsBindingCache<'a>,
    ) -> Option<JsReturnFacts> {
        if func.has_error() || !is_js_ts_function_like(func.kind()) || is_async_or_generator(func) {
            return None;
        }
        let mut acc: Option<Selection> = None;
        for expr in self.js_ts_return_exprs(func) {
            let expr = self.js_ts_unwrap_value(expr)?;
            if is_nullish(expr) {
                continue;
            }
            let sel = self.js_ts_return_selection(func, expr, cache)?;
            acc = Some(match acc {
                None => sel,
                Some(prev) => prev.meet(sel),
            });
        }
        let sel = acc?;
        let facts = JsReturnFacts {
            whole: sel.whole,
            keys: sel.keys,
            slots: sel.slots,
        };
        (!facts.is_empty()).then_some(facts)
    }

    /// The value expressions of `func`'s value-returning statements: every `return <expr>;`
    /// outside nested functions and classes, or an arrow's expression body.
    fn js_ts_return_exprs<'a>(&self, func: Node<'a>) -> Vec<Node<'a>> {
        let Some(body) = func.child_by_field_name("body") else {
            return Vec::new();
        };
        if body.kind() != "statement_block" {
            return vec![body];
        }
        let mut out = Vec::new();
        let mut stack = vec![body];
        while let Some(n) = stack.pop() {
            let mut cursor = n.walk();
            for c in n.named_children(&mut cursor) {
                match c.kind() {
                    k if is_js_ts_function_like(k) => {}
                    "class_declaration"
                    | "class"
                    | "abstract_class_declaration"
                    | "class_static_block" => {}
                    "return_statement" => {
                        let mut inner = c.walk();
                        let value = c.named_children(&mut inner).find(|v| v.kind() != "comment");
                        out.extend(value);
                    }
                    _ => stack.push(c),
                }
            }
        }
        out
    }

    /// What the unwrapped return expression `expr` selects, or `None` when it is not a shape the
    /// contract admits (the function then has no fact at all).
    fn js_ts_return_selection<'a>(
        &'a self,
        func: Node<'a>,
        expr: Node<'a>,
        cache: &mut JsBindingCache<'a>,
    ) -> Option<Selection> {
        let literal = match expr.kind() {
            "object" | "array" => Some(expr),
            "identifier" => self.js_ts_const_literal(func, expr, cache),
            _ => None,
        };
        if let Some(literal) = literal {
            return Some(if literal.kind() == "object" {
                Selection {
                    keys: self.js_ts_object_keys(literal, cache),
                    ..Selection::default()
                }
            } else {
                Selection {
                    slots: self.js_ts_array_slots(literal, cache),
                    ..Selection::default()
                }
            });
        }
        let whole = self.js_ts_callable_value(expr, cache)?;
        Some(Selection {
            whole: Some(whole),
            ..Selection::default()
        })
    }

    /// The object or array literal that the identifier `ident` is bound to, one step: a unique,
    /// unwritten `const` declared inside `func` whose unwrapped value is that literal.
    fn js_ts_const_literal<'a>(
        &'a self,
        func: Node<'a>,
        ident: Node<'a>,
        cache: &mut JsBindingCache<'a>,
    ) -> Option<Node<'a>> {
        let name = self.node_text(&ident).to_string();
        // `Alias` is what the binder gives a clean, unwritten declarator whose value may hold
        // a function; a written binding is `MayCall` and a refused scope is `Refused`.
        if self.js_ts_site_binding(ident, &name, cache) != JsBinding::Alias {
            return None;
        }
        let (_, decl) = self.js_ts_alias_declarator(ident, &name, cache)?;
        if decl.start_byte() < func.start_byte() || decl.end_byte() > func.end_byte() {
            return None;
        }
        decl.child_by_field_name("name")
            .filter(|p| p.kind() == "identifier")?;
        let value = self.js_ts_unwrap_value(decl.child_by_field_name("value")?)?;
        matches!(value.kind(), "object" | "array").then_some(value)
    }

    /// The callable each key of an object literal selects. A computed, string or numeric key,
    /// or an accessor, anywhere in the literal refuses it; a key is dropped when it occurs
    /// twice or when a spread element follows it (a spread before it is fine: the later key
    /// wins).
    fn js_ts_object_keys<'a>(
        &'a self,
        obj: Node<'a>,
        cache: &mut JsBindingCache<'a>,
    ) -> BTreeMap<String, JsFollow> {
        // `(position among the non-comment members, key, member)`.
        let mut entries: Vec<(usize, String, Node<'a>)> = Vec::new();
        let mut last_spread: Option<usize> = None;
        let mut cursor = obj.walk();
        for (index, entry) in obj
            .named_children(&mut cursor)
            .filter(|e| e.kind() != "comment")
            .enumerate()
        {
            match entry.kind() {
                "spread_element" => last_spread = Some(index),
                "shorthand_property_identifier" => {
                    entries.push((index, self.node_text(&entry).to_string(), entry))
                }
                "pair" | "method_definition" => {
                    let method = entry.kind() == "method_definition";
                    let key = entry
                        .child_by_field_name(if method { "name" } else { "key" })
                        .filter(|k| k.kind() == "property_identifier");
                    match key {
                        Some(k) if !(method && has_accessor_token(entry)) => {
                            entries.push((index, self.node_text(&k).to_string(), entry))
                        }
                        _ => return BTreeMap::new(),
                    }
                }
                _ => return BTreeMap::new(),
            }
        }
        let mut out = BTreeMap::new();
        for (position, key, entry) in &entries {
            let unique = entries.iter().filter(|(_, k, _)| k == key).count() == 1;
            let spread_after = last_spread.is_some_and(|s| s > *position);
            if !unique || spread_after {
                continue;
            }
            let value = match entry.kind() {
                "shorthand_property_identifier" => self.js_ts_callable_value(*entry, cache),
                "method_definition" => self.js_ts_inline_callable(*entry),
                _ => entry
                    .child_by_field_name("value")
                    .and_then(|v| self.js_ts_unwrap_value(v))
                    .and_then(|v| self.js_ts_callable_value(v, cache)),
            };
            out.extend(value.map(|v| (key.clone(), v)));
        }
        out
    }

    /// The callable each slot of an array literal selects. A hole is not an element; nothing at
    /// or after a spread element is a known slot.
    fn js_ts_array_slots<'a>(
        &'a self,
        arr: Node<'a>,
        cache: &mut JsBindingCache<'a>,
    ) -> BTreeMap<usize, JsFollow> {
        let mut out = BTreeMap::new();
        let mut index = 0;
        let mut cursor = arr.walk();
        for child in arr.children(&mut cursor) {
            match child.kind() {
                "[" | "]" | "comment" => {}
                "," => {
                    index += 1;
                }
                "spread_element" => break,
                _ => {
                    let value = self
                        .js_ts_unwrap_value(child)
                        .and_then(|v| self.js_ts_callable_value(v, cache));
                    out.extend(value.map(|v| (index, v)));
                }
            }
        }
        out
    }

    /// A callable value in this file: an inline function that has an identity, or an identifier
    /// whose binding at its position is a proven callable, a program-scope import local, or what
    /// slice L's alias follow reaches. Anything else is not a callable value.
    fn js_ts_callable_value<'a>(
        &'a self,
        expr: Node<'a>,
        cache: &mut JsBindingCache<'a>,
    ) -> Option<JsFollow> {
        match expr.kind() {
            "arrow_function" | "function_expression" => self.js_ts_inline_callable(expr),
            "identifier" | "shorthand_property_identifier" => {
                let name = self.node_text(&expr).to_string();
                self.js_ts_terminal_of(expr, &name, cache)
            }
            _ => None,
        }
    }

    /// What the identifier `ident` (spelled `name`) denotes as a callable value: its own proven
    /// callable, a program-scope import local, or the end of a followed alias chain.
    pub(super) fn js_ts_terminal_of<'a>(
        &'a self,
        ident: Node<'a>,
        name: &str,
        cache: &mut JsBindingCache<'a>,
    ) -> Option<JsFollow> {
        match self.js_ts_site_binding(ident, name, cache) {
            JsBinding::Callable(t) => Some(JsFollow::Callable(t)),
            JsBinding::Import if self.js_ts_program_import(ident, name, cache) => {
                Some(JsFollow::ImportLocal(name.to_string()))
            }
            JsBinding::Alias => self.js_ts_follow_alias(ident, name, cache),
            _ => None,
        }
    }

    /// An inline function (arrow, function expression, method) the call graph names: by what it
    /// is assigned to, its property key, or its method name. A bare inline function has no
    /// identity and is not a target.
    fn js_ts_inline_callable(&self, func: Node<'_>) -> Option<JsFollow> {
        let name = self.language.function_name(&func)?;
        let (start_line, end_line) = self.node_line_range(&func);
        Some(JsFollow::Callable(JsTerminal {
            local: self.node_text(&name).to_string(),
            start_line,
            end_line,
            wrapped: false,
        }))
    }
}

/// An `async` function, a generator, or an accessor-less method with either marker: its call
/// does not yield the value its `return` statements name.
fn is_async_or_generator(func: Node<'_>) -> bool {
    if matches!(
        func.kind(),
        "generator_function" | "generator_function_declaration"
    ) {
        return true;
    }
    let mut cursor = func.walk();
    let found = func
        .children(&mut cursor)
        .take_while(|c| Some(*c) != func.child_by_field_name("body"))
        .any(|c| matches!(c.kind(), "async" | "*"));
    found
}

/// A `get` or `set` marker on an object method.
fn has_accessor_token(method: Node<'_>) -> bool {
    let mut cursor = method.walk();
    let found = method
        .children(&mut cursor)
        .take_while(|c| Some(*c) != method.child_by_field_name("name"))
        .any(|c| matches!(c.kind(), "get" | "set"));
    found
}

#[cfg(test)]
#[path = "js_return_facts_tests.rs"]
mod tests;
