//! Whole-list certificate for reads in pattern-formal initializers.

use crate::ast::js_ts::formal_binding;
use crate::ast::js_ts::props_keys::strip_transparent_wrappers;
use crate::ast::ParsedFile;
use crate::data_flow::s1_evidence::{Admission, Refusal};
use crate::parameter_slots::is_inert_default_value;
use std::collections::{BTreeMap, BTreeSet};
use tree_sitter::Node;

#[derive(Clone, Copy)]
struct BareRead<'tree> {
    node: Node<'tree>,
    reader: bool,
}

#[derive(Clone, Copy)]
enum Step<'tree> {
    Read(BareRead<'tree>),
    Bind(Node<'tree>),
}

pub(super) struct Signature(Result<BTreeMap<usize, Admission>, Refusal>);

impl Signature {
    pub(super) fn certify(
        parsed: &ParsedFile,
        owner: Node<'_>,
        eligible: &BTreeSet<usize>,
    ) -> Self {
        Self(Self::check(parsed, owner, eligible))
    }

    pub(super) fn admission(&self, operand: Node<'_>) -> Admission {
        match &self.0 {
            Ok(reads) => reads
                .get(&operand.id())
                .copied()
                .unwrap_or(Admission::Refused(Refusal::Shape)),
            Err(reason) => Admission::Refused(*reason),
        }
    }

    fn check(
        parsed: &ParsedFile,
        owner: Node<'_>,
        eligible: &BTreeSet<usize>,
    ) -> Result<BTreeMap<usize, Admission>, Refusal> {
        let entries = syntax(owner).ok_or(Refusal::Shape)?;
        let mut binders = BTreeMap::new();
        for (position, steps) in entries.iter().enumerate() {
            for step in steps {
                if let Step::Bind(node) = step {
                    let name = decoded(parsed, *node).ok_or(Refusal::Shape)?;
                    if binders
                        .insert(name, (position, node.start_byte()))
                        .is_some()
                    {
                        return Err(Refusal::Shape);
                    }
                }
            }
        }

        let allowed_binder = |read: BareRead<'_>| {
            let (position, byte) = *binders.get(&decoded(parsed, read.node)?)?;
            (!read.reader || eligible.contains(&byte)).then_some((position, byte))
        };
        for step in entries.iter().flatten() {
            if let Step::Read(read) = *step {
                allowed_binder(read).ok_or(Refusal::Binding)?;
            }
        }

        let mut initialized = BTreeSet::new();
        let mut admitted = BTreeMap::new();
        for (position, steps) in entries.iter().enumerate() {
            for step in steps {
                match *step {
                    Step::Bind(node) => {
                        initialized.insert(node.start_byte());
                    }
                    Step::Read(read) => {
                        let (binder_position, byte) =
                            allowed_binder(read).ok_or(Refusal::Binding)?;
                        if !initialized.contains(&byte)
                            || (read.reader && binder_position >= position)
                        {
                            return Err(Refusal::Order);
                        }
                        let admission = if read.reader {
                            Admission::Reader
                        } else {
                            Admission::Bare
                        };
                        admitted.insert(read.node.id(), admission);
                    }
                }
            }
        }
        Ok(admitted)
    }
}

fn syntax(owner: Node<'_>) -> Option<Vec<Vec<Step<'_>>>> {
    let list = owner.child_by_field_name("parameters")?;
    if list.kind() != "formal_parameters" || list.has_error() || list.is_missing() {
        return None;
    }
    let mut cursor = list.walk();
    let entries: Vec<_> = list
        .named_children(&mut cursor)
        .filter(|entry| !entry.is_extra())
        .collect();
    let mut runtime = Vec::new();
    for entry in entries {
        let (binding, default) = formal_binding(entry)?;
        if matches!(entry.kind(), "required_parameter" | "optional_parameter") {
            let mut cursor = entry.walk();
            if entry.named_children(&mut cursor).any(|child| {
                child != binding
                    && Some(child) != default
                    && !child.is_extra()
                    && child.kind() != "type_annotation"
            }) {
                return None;
            }
        }
        if binding.kind() != "this" {
            runtime.push((binding, default));
        }
    }
    runtime
        .iter()
        .enumerate()
        .map(|(index, &(binding, default))| {
            let mut steps = Vec::new();
            let final_formal = index + 1 == runtime.len();
            if binding.kind() == "object_pattern" {
                if let Some(value) = default {
                    if !is_inert_default_value(strip_transparent_wrappers(value)?) {
                        return None;
                    }
                }
                object_steps(binding, &mut steps)?;
            } else {
                simple_steps(binding, default, final_formal, &mut steps)?;
            }
            Some(steps)
        })
        .collect()
}

fn object_steps<'tree>(object: Node<'tree>, steps: &mut Vec<Step<'tree>>) -> Option<()> {
    let mut cursor = object.walk();
    for child in object
        .named_children(&mut cursor)
        .filter(|child| !child.is_extra())
    {
        let leaf = match child.kind() {
            "shorthand_property_identifier_pattern" | "object_assignment_pattern" => child,
            "pair_pattern" => {
                let key = child.child_by_field_name("key")?;
                match key.kind() {
                    "identifier" | "property_identifier" | "string" | "number" => {}
                    "computed_property_name" => {
                        steps.push(Step::Read(BareRead {
                            node: bare(only_named_child(key)?)?,
                            reader: false,
                        }));
                    }
                    _ => return None,
                }
                child.child_by_field_name("value")?
            }
            _ => return None,
        };
        if matches!(
            leaf.kind(),
            "object_assignment_pattern" | "assignment_pattern"
        ) {
            let binding = leaf.child_by_field_name("left")?;
            let default = leaf.child_by_field_name("right")?;
            simple_steps(binding, Some(default), false, steps)?;
        } else {
            simple_steps(leaf, None, false, steps)?;
        }
    }
    Some(())
}

fn simple_steps<'tree>(
    binding: Node<'tree>,
    default: Option<Node<'tree>>,
    final_formal: bool,
    steps: &mut Vec<Step<'tree>>,
) -> Option<()> {
    if !matches!(
        binding.kind(),
        "identifier" | "undefined" | "shorthand_property_identifier_pattern"
    ) {
        return None;
    }
    if let Some(value) = default {
        let value = strip_transparent_wrappers(value)?;
        if !is_inert_default_value(value) {
            let read = if final_formal && value.kind() == "arrow_function" {
                let mut cursor = value.walk();
                if value
                    .children(&mut cursor)
                    .any(|child| child.kind() == "async")
                {
                    return None;
                }
                let formals = value.child_by_field_name("parameters")?;
                let mut cursor = formals.walk();
                if formals
                    .named_children(&mut cursor)
                    .any(|child| !child.is_extra())
                {
                    return None;
                }
                BareRead {
                    node: bare(value.child_by_field_name("body")?)?,
                    reader: true,
                }
            } else {
                BareRead {
                    node: bare(value)?,
                    reader: false,
                }
            };
            steps.push(Step::Read(read));
        }
    }
    steps.push(Step::Bind(binding));
    Some(())
}

fn only_named_child(node: Node<'_>) -> Option<Node<'_>> {
    let mut cursor = node.walk();
    let mut children = node
        .named_children(&mut cursor)
        .filter(|child| !child.is_extra());
    let value = children.next()?;
    children.next().is_none().then_some(value)
}

fn bare(node: Node<'_>) -> Option<Node<'_>> {
    let node = strip_transparent_wrappers(node)?;
    matches!(node.kind(), "identifier" | "undefined").then_some(node)
}

fn decoded(parsed: &ParsedFile, node: Node<'_>) -> Option<String> {
    let spelling = parsed.node_text(&node);
    if spelling.contains('\\') {
        ParsedFile::js_ts_decode_identifier_text(spelling)
    } else {
        Some(spelling.to_owned())
    }
}
