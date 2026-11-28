//! Object-pattern formals, independent of positional parameter slots.

use super::props_keys::{definite_key, strip_transparent_wrappers};
use crate::ast::ParsedFile;
use crate::parameter_slots::is_inert_default_value;
use std::collections::{BTreeMap, BTreeSet};
use std::ops::Range;
use tree_sitter::Node;

#[derive(Debug)]
pub(crate) struct PatternParam {
    #[allow(dead_code)] // Used when matching runtime argument positions in S2b.
    pub(crate) position: usize,
    pub(crate) alias_skipped: bool,
    pub(crate) levels: Vec<PatternLevel>,
    pub(crate) elements: Vec<PatternElement>,
}

#[derive(Debug)]
pub(crate) struct PatternLevel {
    pub(crate) path: Option<Vec<String>>,
    #[allow(dead_code)] // Retains source order and the level's syntax identity.
    pub(crate) bytes: Range<usize>,
    pub(crate) consumed: BTreeSet<String>,
    pub(crate) unknown_sibling: bool,
    pub(crate) unknown_entries: usize,
}

impl PatternLevel {
    /// The first remaining segment is the property removed by object rest.
    #[allow(dead_code)] // Consumed by pattern-target discovery in S2b.
    pub(crate) fn accepts_rest_leaf(&self, leaf: &[String]) -> bool {
        !self.unknown_sibling
            && self.path.as_ref().is_some_and(|parent| {
                leaf.strip_prefix(parent.as_slice())
                    .and_then(|below| below.first())
                    .is_some_and(|key| !self.consumed.contains(key))
            })
    }
}

#[derive(Debug)]
pub(crate) struct PatternElement {
    pub(crate) name: String,
    pub(crate) bytes: Range<usize>,
    #[allow(dead_code)] // Consumed by pattern-target discovery in S2b.
    pub(crate) path: Option<Vec<String>>,
    #[allow(dead_code)]
    pub(crate) rest_level: Option<usize>,
    #[allow(dead_code)]
    pub(crate) defaulted: bool,
    pub(crate) admitted: bool,
}

impl ParsedFile {
    pub(crate) fn js_ts_pattern_params(&self, callable: &Node<'_>) -> Vec<PatternParam> {
        if !self.language.is_js_family() {
            return Vec::new();
        }
        let Some(list) = self.find_parameters_node(callable) else {
            return Vec::new();
        };
        if list.kind() != "formal_parameters" || list.has_error() || list.is_missing() {
            return Vec::new();
        }

        let mut names = Vec::new();
        self.js_ts_pattern_bound_names_with_decoder(list, &mut names, |name| Some(name.to_owned()));
        // The accessor reports syntax, even on an early-error list. The producer
        // separately preserves its owner guard on those lists.
        let mut counts = BTreeMap::new();
        let mut undecodable = false;
        for mut name in names {
            if name.contains('\\') {
                let Some(decoded) = Self::js_ts_decoded_identifier(&name) else {
                    undecodable = true;
                    continue;
                };
                name = decoded;
            }
            *counts.entry(name).or_insert(0) += 1;
        }
        let mut cursor = list.walk();
        list.named_children(&mut cursor)
            .filter(|entry| !entry.is_extra())
            .filter_map(formal_binding)
            .filter(|(binding, _)| binding.kind() != "this")
            .enumerate()
            .filter_map(|(position, (binding, default))| {
                if binding.kind() != "object_pattern" {
                    return None;
                }
                let alias_skipped = self.js_ts_param_pattern_alias_target(binding).is_some();
                let mut pattern = PatternParam {
                    position,
                    alias_skipped,
                    levels: Vec::new(),
                    elements: Vec::new(),
                };
                let walk = PatternWalk {
                    parsed: self,
                    counts: &counts,
                    undecodable,
                };
                walk.object(
                    binding,
                    Some(Vec::new()),
                    DefaultState::default().with(default),
                    &mut pattern,
                );
                Some(pattern)
            })
            .collect()
    }
}

pub(crate) fn formal_binding(entry: Node<'_>) -> Option<(Node<'_>, Option<Node<'_>>)> {
    match entry.kind() {
        "required_parameter" | "optional_parameter" => Some((
            entry.child_by_field_name("pattern")?,
            entry.child_by_field_name("value"),
        )),
        "assignment_pattern" => Some((
            entry.child_by_field_name("left")?,
            entry.child_by_field_name("right"),
        )),
        _ => Some((entry, None)),
    }
}

#[derive(Clone, Copy, Default)]
struct DefaultState {
    present: bool,
    refused: bool,
}

impl DefaultState {
    fn with(self, value: Option<Node<'_>>) -> Self {
        Self {
            present: self.present || value.is_some(),
            refused: self.refused
                || value.is_some_and(|value| {
                    !strip_transparent_wrappers(value).is_some_and(is_inert_default_value)
                }),
        }
    }
}

struct PatternWalk<'a> {
    parsed: &'a ParsedFile,
    counts: &'a BTreeMap<String, usize>,
    undecodable: bool,
}

impl PatternWalk<'_> {
    fn object(
        &self,
        node: Node<'_>,
        path: Option<Vec<String>>,
        defaults: DefaultState,
        pattern: &mut PatternParam,
    ) {
        let level = pattern.levels.len();
        pattern.levels.push(PatternLevel {
            path: path.clone(),
            bytes: node.byte_range(),
            consumed: BTreeSet::new(),
            unknown_sibling: false,
            unknown_entries: 0,
        });

        let mut cursor = node.walk();
        for child in node
            .named_children(&mut cursor)
            .filter(|child| !child.is_extra())
        {
            let (key, value) = match child.kind() {
                "pair_pattern" => (
                    child.child_by_field_name("key"),
                    child.child_by_field_name("value"),
                ),
                "object_assignment_pattern" => (child.child_by_field_name("left"), Some(child)),
                "shorthand_property_identifier_pattern" => (Some(child), Some(child)),
                "rest_pattern" => {
                    let mut rest = child.walk();
                    if let Some(local) = child.named_children(&mut rest).find(|n| !n.is_extra()) {
                        self.binding(local, path.clone(), Some(level), defaults, pattern);
                    }
                    continue;
                }
                _ => (None, None),
            };
            let key = key.and_then(|key| definite_key(self.parsed, key));
            let info = &mut pattern.levels[level];
            let child_path = if let Some(key) = key {
                info.consumed.insert(key.clone());
                path.clone().map(|mut prefix| {
                    prefix.push(key);
                    prefix
                })
            } else {
                info.unknown_sibling = true;
                info.unknown_entries += usize::from(path.is_some());
                None
            };
            if let Some(value) = value {
                self.binding(value, child_path, None, defaults, pattern);
            }
        }
    }

    fn binding(
        &self,
        node: Node<'_>,
        path: Option<Vec<String>>,
        rest_level: Option<usize>,
        defaults: DefaultState,
        pattern: &mut PatternParam,
    ) {
        match node.kind() {
            "object_pattern" => self.object(node, path, defaults, pattern),
            "assignment_pattern" | "object_assignment_pattern" => {
                if let Some(left) = node.child_by_field_name("left") {
                    let defaults = defaults.with(node.child_by_field_name("right"));
                    self.binding(left, path, rest_level, defaults, pattern);
                }
            }
            "identifier" | "shorthand_property_identifier_pattern" | "undefined"
                if !pattern.alias_skipped =>
            {
                let name = self.parsed.node_text(&node);
                pattern.elements.push(PatternElement {
                    name: name.to_owned(),
                    bytes: node.byte_range(),
                    path,
                    rest_level,
                    defaulted: defaults.present,
                    admitted: !defaults.refused
                        && !self.undecodable
                        && !name.contains('\\')
                        && self.counts.get(name) == Some(&1),
                });
            }
            // Array subtrees compete in BoundNames, but supply no elements.
            _ => {}
        }
    }
}
