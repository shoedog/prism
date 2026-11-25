//! Kill events of the I1 route walk (lane 10, #847): replacement of W's root or of a static
//! selector prefix of its storage. Capture-to-commit and key-blind delete are a later job.
use super::python::{kids, leaves, peel_down, text};
use super::StaticPath;
use tree_sitter::Node;

/// Kill events along a route. `atom` is a simple statement or a header expression.
pub(crate) trait Transfer {
    fn kills(&self, atom: Node<'_>, src: &str) -> bool;
    /// A loop target bound on a successful entry.
    fn kills_target(&self, target: Node<'_>, src: &str) -> bool;
}

/// Replacement of W's root or of a static selector prefix of its storage.
pub(crate) struct Rebind {
    root: String,
    fields: Vec<String>,
}

impl Rebind {
    pub(crate) fn of(path: &StaticPath) -> Option<Rebind> {
        Some(Rebind {
            root: path.root.clone()?,
            fields: path.fields.clone(),
        })
    }

    /// A target replacing the root, or an attribute chain naming a prefix of W's selectors.
    fn hits(&self, target: Node, src: &str) -> bool {
        let mut n = peel_down(target);
        let mut fields = Vec::new();
        while n.kind() == "attribute" {
            fields.extend(n.child_by_field_name("attribute").map(|a| text(a, src)));
            let Some(o) = n.child_by_field_name("object") else {
                return false;
            };
            n = peel_down(o);
        }
        fields.reverse();
        n.kind() == "identifier"
            && text(n, src) == self.root
            && fields.len() <= self.fields.len()
            && fields.iter().zip(&self.fields).all(|(a, b)| a == b)
    }

    fn any_leaf(&self, target: Node, src: &str) -> bool {
        let (mut found, mut nested) = (Vec::new(), false);
        leaves(target, &mut found, &mut nested);
        found.into_iter().any(|l| self.hits(l, src))
    }

    fn imports(&self, n: Node, src: &str) -> bool {
        n.children_by_field_name("name", &mut n.walk()).any(|c| {
            let bound = match c.kind() {
                "aliased_import" => c.child_by_field_name("alias"),
                _ => kids(c).into_iter().find(|k| k.kind() == "identifier"),
            };
            bound.is_some_and(|b| text(b, src) == self.root)
        })
    }
}

impl Transfer for Rebind {
    fn kills(&self, atom: Node<'_>, src: &str) -> bool {
        if matches!(atom.kind(), "function_definition" | "class_definition") {
            return self.def_named(atom, src);
        }
        if atom.kind() == "decorated_definition" {
            let def = atom.child_by_field_name("definition");
            return def.is_some_and(|d| self.def_named(d, src));
        }
        let mut stack = vec![(atom, false)];
        while let Some((n, cond)) = stack.pop() {
            let named: Vec<_> = kids(n).into_iter().filter(|c| c.is_named()).collect();
            match n.kind() {
                "lambda" | "function_definition" | "class_definition" => continue,
                "assignment" => {
                    let left = n.child_by_field_name("left");
                    let value = n.child_by_field_name("right").is_some();
                    if value && !cond && left.is_some_and(|l| self.any_leaf(l, src)) {
                        return true;
                    }
                }
                "augmented_assignment" => {
                    let left = n.child_by_field_name("left");
                    if !cond && left.is_some_and(|l| self.hits(l, src)) {
                        return true;
                    }
                }
                "named_expression" => {
                    let name = n.child_by_field_name("name");
                    if !cond && name.is_some_and(|b| text(b, src) == self.root) {
                        return true;
                    }
                }
                "delete_statement" => {
                    if !cond && named.iter().any(|t| self.any_leaf(*t, src)) {
                        return true;
                    }
                }
                "import_statement" | "import_from_statement" => {
                    if !cond && self.imports(n, src) {
                        return true;
                    }
                }
                _ => {}
            }
            for (i, c) in named.into_iter().enumerate() {
                stack.push((c, cond || Self::deferred(n, i)));
            }
        }
        false
    }

    fn kills_target(&self, target: Node<'_>, src: &str) -> bool {
        self.any_leaf(target, src)
    }
}

impl Rebind {
    fn def_named(&self, def: Node, src: &str) -> bool {
        let name = def.child_by_field_name("name");
        name.is_some_and(|n| text(n, src) == self.root)
    }

    /// True for the named child that may not execute: conditional-expression arms, the right
    /// operand of `and`/`or`, later operands of a comparison chain and comprehension parts.
    fn deferred(parent: Node, index: usize) -> bool {
        match parent.kind() {
            "conditional_expression" => index != 1,
            "boolean_operator" => index == 1,
            "comparison_operator" => index >= 2,
            "list_comprehension"
            | "set_comprehension"
            | "dictionary_comprehension"
            | "generator_expression" => true,
            _ => false,
        }
    }
}
