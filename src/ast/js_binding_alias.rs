//! Slice L (owner direction 2026-10-13): the nearest binding decides, and an alias is followed
//! to what it names. A JS/TS `const` alias whose value is an identifier, a plain namespace
//! member, or a flat object-pattern key over a namespace import is followed through a bounded,
//! cycle-safe chain of unique, unwritten `const` links to one callable, one import local, or one
//! member of a namespace import's module. Anything else keeps the base outcome (`Alias`).
//!
//! The follow is a separate query over `js_ts_site_binding`'s answer. `js_ts_classify` and
//! therefore D4's `js_ts_local_export_target` still see `Alias` for every link (an exported
//! alias stays unproven; following aliases across files is a later slice), and the follow never
//! classifies a declaration itself: each link re-enters `js_ts_site_binding` at a strictly
//! different declaration, so the visited set and the link bound terminate it.
use super::js_binding::{JsBinding, JsBindingCache, JsTerminal};
use super::js_binding_values::js_ts_has_default;
use super::js_binding_walk::Walk;
use super::ParsedFile;
use std::collections::BTreeSet;
use tree_sitter::Node;

/// The most alias declarations one chain may traverse.
const MAX_LINKS: usize = 8;

/// What a followed `const` alias chain ends at.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum JsFollow {
    /// A proven callable in this file (its `wrapped` flag included).
    Callable(JsTerminal),
    /// The program-scope import binding `local`; the import rungs decide the target.
    ImportLocal(String),
    /// Member `member` of the namespace import of `module_path`.
    NamespaceMember { module_path: String, member: String },
}

/// One alias declaration's value.
enum Link<'a> {
    /// `const a = X`: continue at the identifier `X`.
    Next(Node<'a>),
    /// `const a = Q.m` or `const { m: a } = Q`: member `m` of the qualifier identifier `Q`.
    Member(Node<'a>, String),
}

impl ParsedFile {
    /// Follows the `const` alias chain that the identifier `ident` (spelled `name`) starts, when
    /// `js_ts_site_binding` says it is an alias. `None` leaves the outcome what base gives.
    pub(crate) fn js_ts_follow_alias<'a>(
        &'a self,
        ident: Node<'a>,
        name: &str,
        cache: &mut JsBindingCache<'a>,
    ) -> Option<JsFollow> {
        let mut visited = BTreeSet::new();
        let (mut at, mut spelling) = (ident, name.to_string());
        let mut links = 0;
        loop {
            match self.js_ts_site_binding(at, &spelling, cache) {
                JsBinding::Alias => {
                    if links == MAX_LINKS {
                        return None;
                    }
                    let (scope, decl) = self.js_ts_alias_declarator(at, &spelling, cache)?;
                    if !visited.insert((scope.id(), decl.id())) {
                        return None;
                    }
                    links += 1;
                    match self.js_ts_alias_link(decl, &spelling)? {
                        Link::Next(next) => {
                            spelling = self.node_text(&next).to_string();
                            at = next;
                        }
                        Link::Member(qualifier, member) => {
                            let (binding, module) =
                                self.js_ts_namespace_binding(qualifier, cache)?;
                            return match (binding, module) {
                                (JsBinding::Import, Some(module_path)) => {
                                    Some(JsFollow::NamespaceMember {
                                        module_path,
                                        member,
                                    })
                                }
                                _ => None,
                            };
                        }
                    }
                }
                JsBinding::Callable(t) if links > 0 => return Some(JsFollow::Callable(t)),
                JsBinding::Import
                    if links > 0 && self.js_ts_program_import(at, &spelling, cache) =>
                {
                    return Some(JsFollow::ImportLocal(spelling));
                }
                _ => return None,
            }
        }
    }

    /// The single `const` declarator that `name` at `at` is bound by, with its scope.
    fn js_ts_alias_declarator<'a>(
        &'a self,
        at: Node<'a>,
        name: &str,
        cache: &mut JsBindingCache<'a>,
    ) -> Option<(Node<'a>, Node<'a>)> {
        let Walk::Found(scope, None) = self.js_ts_binding_walk(at, name, cache) else {
            return None;
        };
        let (decls, annex_b) = self.js_ts_scope_lookup(scope, name, cache);
        let decls = decls.filter(|_| !annex_b)?;
        let [decl] = decls.as_slice() else {
            return None;
        };
        let is_const = decl.kind() == "variable_declarator"
            && decl.parent().is_some_and(|list| {
                list.kind() == "lexical_declaration"
                    && list.child(0).is_some_and(|k| k.kind() == "const")
            });
        is_const.then_some((scope, *decl))
    }

    /// `name` at `at` is bound by a single import declaration of the program scope: the only
    /// scope whose imports the file-level import rungs describe.
    fn js_ts_program_import<'a>(
        &'a self,
        at: Node<'a>,
        name: &str,
        cache: &mut JsBindingCache<'a>,
    ) -> bool {
        let Walk::Found(scope, None) = self.js_ts_binding_walk(at, name, cache) else {
            return false;
        };
        let (decls, _) = self.js_ts_scope_lookup(scope, name, cache);
        scope.kind() == "program"
            && decls.is_some_and(|ds| {
                matches!(ds.as_slice(), [d] if matches!(d.kind(), "import_statement" | "import_alias"))
            })
    }

    /// The link a `const` declarator of `name` forms, if its shape is one of the three the
    /// contract follows.
    fn js_ts_alias_link<'a>(&'a self, decl: Node<'a>, name: &str) -> Option<Link<'a>> {
        let pattern = decl.child_by_field_name("name")?;
        let value = self.js_ts_unwrap_value(decl.child_by_field_name("value")?)?;
        match (pattern.kind(), value.kind()) {
            ("identifier", "identifier") => Some(Link::Next(value)),
            ("identifier", "member_expression") => {
                let qualifier = value
                    .child_by_field_name("object")
                    .filter(|o| o.kind() == "identifier")?;
                let property = value
                    .child_by_field_name("property")
                    .filter(|p| p.kind() == "property_identifier")?;
                let mut cursor = value.walk();
                let optional = value
                    .children(&mut cursor)
                    .any(|c| c.kind() == "optional_chain");
                (!optional).then(|| Link::Member(qualifier, self.node_text(&property).to_string()))
            }
            ("object_pattern", "identifier") => {
                // Whole-pattern admission: every entry (comments aside) must be a shorthand
                // identifier or a pair with a plain property-identifier key and an identifier
                // value. A rest element, nested pattern, computed/string/numeric key or default
                // anywhere means the declaration may be invalid or bind something unmodeled.
                if js_ts_has_default(pattern) {
                    return None;
                }
                let mut keys = Vec::new();
                let mut cursor = pattern.walk();
                for entry in pattern.named_children(&mut cursor) {
                    let (bound, key) = match entry.kind() {
                        "comment" => continue,
                        "shorthand_property_identifier_pattern" => {
                            let text = self.node_text(&entry);
                            (text, text)
                        }
                        "pair_pattern" => {
                            let key = entry
                                .child_by_field_name("key")
                                .filter(|k| k.kind() == "property_identifier")?;
                            let bound = entry
                                .child_by_field_name("value")
                                .filter(|v| v.kind() == "identifier")?;
                            (self.node_text(&bound), self.node_text(&key))
                        }
                        _ => return None,
                    };
                    if bound == name {
                        keys.push(key.to_string());
                    }
                }
                match keys.as_mut_slice() {
                    [key] => Some(Link::Member(value, std::mem::take(key))),
                    _ => None,
                }
            }
            _ => None,
        }
    }

    /// B3's unwrapping minus assignment chains: parentheses and TypeScript assertions.
    fn js_ts_unwrap_value<'a>(&self, mut value: Node<'a>) -> Option<Node<'a>> {
        loop {
            let inner = match value.kind() {
                "parenthesized_expression"
                | "as_expression"
                | "satisfies_expression"
                | "non_null_expression" => first_expression(value, false),
                "type_assertion" => first_expression(value, true),
                _ => return Some(value),
            };
            value = inner?;
        }
    }
}

/// The first (or, for an angle-bracket assertion, last) named child that is not a comment.
fn first_expression(node: Node<'_>, last: bool) -> Option<Node<'_>> {
    let mut cursor = node.walk();
    let mut named = node
        .named_children(&mut cursor)
        .filter(|c| c.kind() != "comment");
    if last {
        named.last()
    } else {
        named.next()
    }
}

#[cfg(test)]
#[path = "js_binding_alias_pattern_tests.rs"]
mod pattern_tests;
#[cfg(test)]
#[path = "js_binding_alias_tests.rs"]
mod tests;
