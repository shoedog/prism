//! Classify body reads that can run after the holder invocation returns.
use super::*;

#[derive(Default)]
pub(super) struct ReadTiming {
    suspensions: BTreeMap<usize, Vec<usize>>,
    deferred: Mutex<BTreeMap<(usize, usize), bool>>,
}

impl ReadTiming {
    pub(super) fn index(&mut self, node: Node<'_>, callables: &[&str]) {
        let mut offset = match node.kind() {
            "await_expression" => node.end_byte(),
            "for_in_statement" if has_token(node, "await") => {
                // The iterable is evaluated before the first async iteration.
                let Some(iterable) = node.child_by_field_name("right") else {
                    return;
                };
                iterable.end_byte()
            }
            _ => return,
        };
        let mut child = node;
        while let Some(parent) = child.parent() {
            if invocation_boundary(callables, parent, child) {
                self.suspensions
                    .entry(parent.id())
                    .or_default()
                    .push(offset);
                return;
            }
            if parent.kind() == "switch_case" && parent.child_by_field_name("value") == Some(child)
            {
                // Later case labels are evaluated before the default body.
                let mut earlier = std::iter::successors(parent.prev_named_sibling(), |case| {
                    case.prev_named_sibling()
                });
                if let Some(default) = earlier.find(|case| case.kind() == "switch_default") {
                    offset = offset.min(default.start_byte());
                }
            }
            child = parent;
        }
    }

    pub(super) fn deferred(&self, parsed: &ParsedFile, owner: Node<'_>, read: Node<'_>) -> bool {
        *self
            .deferred
            .lock()
            .expect("argument-follow read timing")
            .entry((owner.id(), read.id()))
            .or_insert_with(|| self.classify(parsed, owner, read))
    }

    fn classify(&self, parsed: &ParsedFile, owner: Node<'_>, read: Node<'_>) -> bool {
        if matches!(
            owner.kind(),
            "generator_function" | "generator_function_declaration"
        ) || owner.kind() == "method_definition" && has_token(owner, "*")
        {
            return true;
        }
        let callables = parsed.language.callable_boundary_node_types();
        let mut before = read.start_byte();
        let mut child = read;
        while let Some(parent) = child.parent().filter(|parent| *parent != owner) {
            if invocation_boundary(&callables, parent, child) {
                return true;
            }
            // Back edges and computed class keys can run later text first.
            let can_run_later_text = match parent.kind() {
                "for_statement" => parent.child_by_field_name("initializer") != Some(child),
                "for_in_statement" => parent.child_by_field_name("right") != Some(child),
                "while_statement" | "do_statement" | "class_body" => true,
                _ => false,
            };
            if can_run_later_text {
                before = parent.end_byte();
            }
            child = parent;
        }
        self.suspensions
            .get(&owner.id())
            .is_some_and(|offsets| offsets.iter().any(|&offset| offset <= before))
    }
}

fn invocation_boundary(callables: &[&str], parent: Node<'_>, child: Node<'_>) -> bool {
    callables.contains(&parent.kind())
        && (parent.kind() != "method_definition"
            || parent.child_by_field_name("parameters") == Some(child)
            || parent.child_by_field_name("body") == Some(child))
}

fn has_token(node: Node<'_>, token: &str) -> bool {
    let mut cursor = node.walk();
    let found = node
        .children(&mut cursor)
        .any(|child| !child.is_named() && child.kind() == token);
    found
}
