//! Prove that uses of an entry object leave the selected path intact.
use super::*;

mod literal;
mod mapped;

pub(super) use literal::plain_access;
pub(super) use mapped::arguments_instability;

#[derive(Default)]
pub(super) struct References<'a> {
    names: BTreeMap<String, Vec<Node<'a>>>,
    literals: Mutex<literal::PlainObjects<'a>>,
}

impl<'a> FileBindings<'a> {
    pub(super) fn references(&self, parsed: &'a ParsedFile) -> &References<'a> {
        self.references.get_or_init(|| References::index(parsed))
    }
}

impl<'a> References<'a> {
    fn index(parsed: &'a ParsedFile) -> Self {
        let mut refs = Self::default();
        let mut cursor = parsed.tree.walk();
        loop {
            let node = cursor.node();
            if let Some(name) = param_slot::name(parsed, node) {
                refs.names.entry(name).or_default().push(node);
            }
            if cursor.goto_first_child() {
                continue;
            }
            while !cursor.goto_next_sibling() {
                if !cursor.goto_parent() {
                    return refs;
                }
            }
        }
    }

    fn in_scope(&self, name: &str, scope: Node<'a>) -> &[Node<'a>] {
        let nodes = self.names.get(name).map(Vec::as_slice).unwrap_or_default();
        let start = nodes.partition_point(|node| node.start_byte() < scope.start_byte());
        let end = nodes.partition_point(|node| node.start_byte() < scope.end_byte());
        &nodes[start..end]
    }
}

pub(super) struct PathRead<'a, 'path> {
    pub(super) source: Node<'a>,
    pub(super) scope: Node<'a>,
    pub(super) observation: Node<'a>,
    pub(super) name: &'path str,
    pub(super) selected: &'path [Option<String>],
}

pub(super) fn path_unstable<'a>(
    parsed: &'a ParsedFile,
    refs: &References<'a>,
    read: PathRead<'a, '_>,
    plain_access: &DisjointProof<'a, '_>,
    cache: &mut JsBindingCache<'a>,
) -> bool {
    let before = Before::new(parsed, read.observation);
    refs.in_scope(read.name, read.scope)
        .iter()
        .copied()
        .any(|occurrence| {
            if occurrence == read.source
                || !before.may_include(parsed, occurrence)
                || parsed.language.is_in_erased_type_context(occurrence)
            {
                return false;
            }
            match parsed.js_ts_binding_walk(occurrence, read.name, cache) {
                param_slot::Walk::Found(found, _) if found == read.scope => !PathUse::new(
                    parsed,
                    occurrence,
                    read.selected,
                )
                .preserves(parsed, 0, plain_access),
                param_slot::Walk::Found(found, _) => found.kind() == "with_statement",
                param_slot::Walk::Unchecked(_) => true,
                param_slot::Walk::Unbound => false,
            }
        })
}

pub(super) fn selected_keys<'a>(
    selected: &'a [Selection<'_>],
) -> impl Iterator<Item = Option<String>> + 'a {
    selected.iter().filter_map(|step| match step {
        Selection::Object(key) => Some(Some(key.clone())),
        Selection::Array(index) => Some(Some(index.to_string())),
        Selection::Computed(_) | Selection::Rest => Some(None),
        Selection::Default(_) => None,
    })
}

fn execution_owner(parsed: &ParsedFile, mut node: Node<'_>) -> Option<usize> {
    let boundaries = parsed.language.callable_boundary_node_types();
    while let Some(parent) = node.parent() {
        if boundaries.contains(&parent.kind())
            || matches!(parent.kind(), "class_static_block" | "program")
        {
            return Some(parent.id());
        }
        node = parent;
    }
    None
}

struct Before {
    end: usize,
    owner: Option<usize>,
}

impl Before {
    fn new(parsed: &ParsedFile, node: Node<'_>) -> Self {
        let owner = execution_owner(parsed, node);
        let mut end = node.end_byte();
        let mut parent = node.parent();
        while let Some(enclosing) = parent.filter(|parent| Some(parent.id()) != owner) {
            // A back edge, a later case label, or a computed class key can
            // execute text after the observation before the read takes place.
            if matches!(
                enclosing.kind(),
                "for_statement"
                    | "for_in_statement"
                    | "while_statement"
                    | "do_statement"
                    | "switch_statement"
                    | "class_body"
            ) {
                end = enclosing.end_byte();
            }
            parent = enclosing.parent();
        }
        Self { end, owner }
    }

    fn may_include(&self, parsed: &ParsedFile, occurrence: Node<'_>) -> bool {
        occurrence.start_byte() < self.end || execution_owner(parsed, occurrence) != self.owner
    }
}

fn member_key(parsed: &ParsedFile, member: Node<'_>) -> Option<String> {
    if let Some(property) = member.child_by_field_name("property") {
        return param_slot::property_key(parsed, property);
    }
    let index = member
        .child_by_field_name("index")
        .and_then(param_slot::peel)?;
    if matches!(index.kind(), "number" | "string") {
        param_slot::property_key(parsed, index)
    } else {
        None
    }
}

fn member(node: Node<'_>) -> bool {
    matches!(node.kind(), "member_expression" | "subscript_expression")
}

fn unwrap_use(mut node: Node<'_>) -> Node<'_> {
    while let Some(parent) = node
        .parent()
        .filter(|parent| param_slot::peel(*parent) == Some(node))
    {
        node = parent;
    }
    node
}

/// Identify actual lvalue leaves, including targets wrapped in assignment patterns.
fn write_operation<'a>(parsed: &ParsedFile, mut node: Node<'a>) -> Option<Node<'a>> {
    while let Some(parent) = node.parent() {
        let field = match parent.kind() {
            "array_pattern" | "object_pattern" | "rest_pattern" | "parenthesized_pattern" => {
                node = parent;
                continue;
            }
            "pair_pattern" => "value",
            "assignment_pattern" | "object_assignment_pattern" => "left",
            _ => return (param_slot::write_target(parsed, parent) == Some(node)).then_some(parent),
        };
        if parent.child_by_field_name(field) != Some(node) {
            break;
        }
        node = parent;
    }
    None
}

pub(super) type DisjointProof<'a, 'proof> = dyn Fn(&PathUse<'a>, usize) -> bool + 'proof;

pub(super) struct PathUse<'a> {
    node: Node<'a>,
    depth: usize,
    keys: Vec<Option<String>>,
    selected_depth: usize,
    disjoint: Option<usize>,
    prototype_access: bool,
}

impl<'a> PathUse<'a> {
    fn new(parsed: &ParsedFile, root: Node<'a>, selected: &[Option<String>]) -> Self {
        let mut node = unwrap_use(root);
        let mut depth = 0;
        let mut keys = Vec::new();
        let mut disjoint = None;
        let mut prototype_access = false;
        while let Some(parent) = node
            .parent()
            .filter(|parent| member(*parent) && parent.child_by_field_name("object") == Some(node))
        {
            let key = member_key(parsed, parent);
            if depth < selected.len() && disjoint.is_none() {
                prototype_access |= key.as_deref() == Some("__proto__");
                if matches!((&key, &selected[depth]), (Some(key), Some(want)) if key != want) {
                    disjoint = Some(depth + 1);
                }
            }
            depth += 1;
            keys.push(key);
            node = unwrap_use(parent);
        }
        Self {
            node,
            depth,
            keys,
            selected_depth: selected.len(),
            disjoint,
            prototype_access,
        }
    }

    fn preserves(
        &self,
        parsed: &ParsedFile,
        copied_depth: usize,
        plain_access: &DisjointProof<'a, '_>,
    ) -> bool {
        // A method call can mutate its receiver even when its own key is disjoint.
        let receiver_escapes = self.depth > copied_depth
            && self.depth <= self.selected_depth
            && self.disjoint.is_none_or(|depth| self.depth <= depth)
            && self.node.parent().is_some_and(|parent| {
                parent.kind() == "call_expression"
                    && parent.child_by_field_name("function") == Some(self.node)
            });
        if self.prototype_access || receiver_escapes {
            return false;
        }
        if self.disjoint.is_some() {
            return plain_access(self, 0);
        }
        if self
            .keys
            .iter()
            .take(self.selected_depth)
            .any(Option::is_none)
        {
            // An unknown key can expose a selected ancestor or invoke an
            // accessor; its syntax cannot establish a harmless read.
            return false;
        }
        if let Some(write) = write_operation(parsed, self.node) {
            return (self.depth > 0
                && self.depth <= copied_depth
                && write.kind() == "assignment_expression")
                || self.depth > self.selected_depth;
        }
        // Reading the selected value or a descendant cannot replace its slot.
        self.depth >= self.selected_depth || harmless_use(parsed, self.node)
    }
}

fn harmless_use(parsed: &ParsedFile, node: Node<'_>) -> bool {
    if parsed.js_ts_identifier_is_not_a_read(node) {
        return true;
    }
    let Some(parent) = node.parent() else {
        return false;
    };
    let operator = parent.child_by_field_name("operator");
    match parent.kind() {
        "expression_statement" => true,
        "unary_expression" => {
            operator.is_some_and(|op| matches!(op.kind(), "typeof" | "void" | "!"))
        }
        "binary_expression" => operator.is_some_and(|op| matches!(op.kind(), "===" | "!==")),
        _ => false,
    }
}
