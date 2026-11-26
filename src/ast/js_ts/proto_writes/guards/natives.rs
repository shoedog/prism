//! File-local writes to names or properties used as native operations.
use super::*;
use std::collections::BTreeMap;

// Unknown computed names match any property at that position. A write to a
// name or an ancestor property invalidates the operation behind that name.
type Path = Vec<Option<String>>;
type Aliases = BTreeMap<usize, BTreeSet<Path>>;

#[derive(Default)]
pub(in super::super) struct NativeWrites {
    paths: BTreeSet<Path>,
    pub(super) exhausted: bool,
}

impl NativeWrites {
    pub(in super::super) fn collect<'a>(
        file: &'a ParsedFile,
        analysis: &Analysis<'a>,
        mut charge: impl FnMut(usize) -> bool,
    ) -> Self {
        let mut census = Census {
            file,
            analysis,
            charge: &mut charge,
        };
        census.collect().unwrap_or_else(|| Self {
            exhausted: true,
            ..Self::default()
        })
    }

    pub(super) fn written(&self, path: &[&str]) -> bool {
        for len in 0..=path.len() {
            // A path with a dynamic component is rare. The native paths
            // admitted by guard rules have at most three components.
            for mask in 0..(1 << len) {
                let target: Path = path[..len]
                    .iter()
                    .enumerate()
                    .map(|(i, name)| ((mask & (1 << i)) == 0).then(|| (*name).to_owned()))
                    .collect();
                if self.paths.contains(&target) {
                    return true;
                }
            }
        }
        false
    }
}

struct Census<'a, 'b, F> {
    file: &'a ParsedFile,
    analysis: &'b Analysis<'a>,
    charge: &'b mut F,
}

impl<'a, F: FnMut(usize) -> bool> Census<'a, '_, F> {
    fn collect(&mut self) -> Option<NativeWrites> {
        let mut nodes = vec![self.file.tree.root_node()];
        let mut stores = Vec::new();
        let mut calls = Vec::new();
        let mut definitions = Vec::new();
        while let Some(node) = nodes.pop() {
            if !(self.charge)(node.named_child_count() + 1) {
                return None;
            }
            match node.kind() {
                "assignment_expression" | "augmented_assignment_expression" => {
                    if let Some(left) = field(node, "left") {
                        stores.push(left);
                        if let Some(value) = field(node, "right") {
                            definitions.push((left, value));
                        }
                    }
                }
                "variable_declarator" => {
                    definitions.extend(field(node, "name").zip(field(node, "value")));
                }
                "update_expression" => stores.extend(field(node, "argument")),
                "unary_expression" if operator(node, self.file) == "delete" => {
                    stores.extend(field(node, "argument"));
                }
                "for_in_statement" if field(node, "kind").is_none() => {
                    stores.extend(field(node, "left"));
                }
                "call_expression" => calls.push(node),
                _ => {}
            }
            nodes.extend(children(node));
        }

        let mut aliases = Aliases::new();
        // Each iteration records new may-aliases. Reassignments never erase
        // a witnessed alias, even in a different function or branch.
        loop {
            let mut changed = false;
            for &(left, right) in &definitions {
                if !(self.charge)(1) {
                    return None;
                }
                let left = peel(left);
                let Some(binding) = identifier(left).then(|| self.binding(left)).flatten() else {
                    continue;
                };
                let values = self.paths(right, &aliases, 0)?;
                if !values.is_empty() {
                    let entry = aliases.entry(binding).or_default();
                    let old_len = entry.len();
                    entry.extend(values);
                    changed |= entry.len() != old_len;
                }
            }
            if !changed {
                break;
            }
        }

        let mut writes = NativeWrites::default();
        while let Some(target) = stores.pop().map(peel) {
            if !(self.charge)(1) {
                return None;
            }
            if identifier(target) {
                if self.lookup(target).is_none() {
                    writes
                        .paths
                        .insert(vec![Some(self.name(target).to_owned())]);
                }
            } else if matches!(target.kind(), "member_expression" | "subscript_expression") {
                writes.paths.extend(self.paths(target, &aliases, 0)?);
            } else {
                stores.extend(pattern_parts(target));
            }
        }
        for call in calls {
            self.reflection(call, &aliases, &mut writes)?;
        }
        Some(writes)
    }

    fn name(&self, node: Node<'a>) -> &str {
        self.analysis.lexical.canonical(self.file.node_text(&node))
    }

    fn lookup(&self, node: Node<'a>) -> Option<&Option<AstSpan>> {
        let scope = std::iter::successors(Some(node), Node::parent)
            .take(128)
            .find(|n| is_scope(*n))?;
        self.analysis.lexical.lookup(scope, self.name(node))
    }

    fn binding(&self, node: Node<'a>) -> Option<usize> {
        Some(self.lookup(node)?.as_ref()?.start_byte)
    }

    fn key(&self, node: Node<'a>) -> Option<String> {
        let node = peel(node);
        let text = self.file.node_text(&node);
        match node.kind() {
            "property_identifier" | "private_property_identifier" => {
                Some(self.name(node).to_owned())
            }
            "string" => {
                let value = text.get(1..text.len().checked_sub(1)?)?;
                if value.contains('\\') {
                    ParsedFile::js_ts_decode_identifier_text(value)
                } else {
                    Some(value.to_owned())
                }
            }
            _ => None,
        }
    }

    fn paths(&mut self, node: Node<'a>, aliases: &Aliases, depth: usize) -> Option<Vec<Path>> {
        if !(self.charge)(1) || depth == 64 {
            return None;
        }
        let node = peel(node);
        if identifier(node) {
            return Some(match self.lookup(node) {
                None => match self.name(node) {
                    "globalThis" | "global" | "window" | "self" => vec![Vec::new()],
                    name => vec![vec![Some(name.to_owned())]],
                },
                Some(Some(span)) => {
                    let paths = aliases.get(&span.start_byte);
                    if !(self.charge)(paths.map_or(0, BTreeSet::len)) {
                        return None;
                    }
                    paths.into_iter().flatten().cloned().collect()
                }
                Some(None) => Vec::new(),
            });
        }
        let result = match node.kind() {
            "member_expression" | "subscript_expression" => {
                let base = field(node, "object")?;
                let key = field(node, "property").or_else(|| field(node, "index"))?;
                let key = self.key(key);
                let mut paths = self.paths(base, aliases, depth + 1)?;
                for path in &mut paths {
                    if path.len() < 4 {
                        path.push(key.clone());
                    }
                }
                paths
            }
            "this"
                if !self.analysis.lexical.module_syntax
                    && !std::iter::successors(node.parent(), Node::parent).any(|owner| {
                        is_owner(owner) && !matches!(owner.kind(), "arrow_function" | "program")
                    }) =>
            {
                vec![Vec::new()]
            }
            "ternary_expression" => {
                let mut yes = self.paths(field(node, "consequence")?, aliases, depth + 1)?;
                yes.extend(self.paths(field(node, "alternative")?, aliases, depth + 1)?);
                yes
            }
            "binary_expression" if matches!(operator(node, self.file), "||" | "&&" | "??") => {
                let mut left = self.paths(field(node, "left")?, aliases, depth + 1)?;
                left.extend(self.paths(field(node, "right")?, aliases, depth + 1)?);
                left
            }
            "sequence_expression" => self.paths(children(node).last()?, aliases, depth + 1)?,
            "assignment_expression" => self.paths(field(node, "right")?, aliases, depth + 1)?,
            _ => Vec::new(),
        };
        Some(result)
    }

    fn reflection(
        &mut self,
        call: Node<'a>,
        aliases: &Aliases,
        writes: &mut NativeWrites,
    ) -> Option<()> {
        if !(self.charge)(1) {
            return None;
        }
        let Some(callee) = field(call, "function").map(peel) else {
            return Some(());
        };
        let Some(property) = field(callee, "property").or_else(|| field(callee, "index")) else {
            return Some(());
        };
        let Some(method) = self.key(property) else {
            return Some(());
        };
        if !matches!(
            method.as_str(),
            "set"
                | "deleteProperty"
                | "defineProperty"
                | "defineProperties"
                | "assign"
                | "setPrototypeOf"
        ) {
            return Some(());
        }
        let mut callees = self.paths(callee, aliases, 0)?;
        if !callees.iter_mut().any(|path| {
            path.pop();
            matches!(path.as_slice(), [Some(name)] if name == "Object" || name == "Reflect")
        }) {
            return Some(());
        }
        let Some(args) = field(call, "arguments") else {
            return Some(());
        };
        let mut args = children(args);
        let Some(target) = args.next() else {
            return Some(());
        };
        let targets = self.paths(target, aliases, 0)?;
        let key = match method.as_str() {
            "set" | "deleteProperty" | "defineProperty" => {
                args.next().and_then(|key| self.key(key))
            }
            _ => None,
        };
        for mut path in targets {
            path.push(key.clone());
            writes.paths.insert(path);
        }
        Some(())
    }
}
