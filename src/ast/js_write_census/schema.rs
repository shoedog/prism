//! Grammar schema access for the census certificate (#660 S0, DESIGN.md 4.2 and 18.2).
//!
//! Mirrors `census/grammar.py`: slot expansion through supertypes, the write-relevant slot set,
//! the named-leaf set and the complete schema signature. The input is the generated
//! `node-types.json` (`Language::node_types_json`), parsed with its own structs because the
//! certificate needs the `named` flag and the `multiple`/`required` bits.

use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Deserialize)]
pub(crate) struct TypeRef {
    #[serde(rename = "type")]
    pub kind: String,
    pub named: bool,
}

#[derive(Deserialize)]
pub(crate) struct Members {
    #[serde(default)]
    pub multiple: bool,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub types: Vec<TypeRef>,
}

#[derive(Deserialize)]
pub(crate) struct NodeType {
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(default)]
    pub named: bool,
    #[serde(default)]
    pub subtypes: Option<Vec<TypeRef>>,
    #[serde(default)]
    pub fields: BTreeMap<String, Members>,
    #[serde(default)]
    pub children: Option<Members>,
}

pub(crate) type Slots = BTreeMap<(String, String), BTreeSet<String>>;

pub(crate) const NAME_LEAVES: [&str; 3] = [
    "identifier",
    "shorthand_property_identifier_pattern",
    "undefined",
];
const PATTERNS: [&str; 6] = [
    "array_pattern",
    "object_pattern",
    "assignment_pattern",
    "object_assignment_pattern",
    "rest_pattern",
    "pair_pattern",
];
const TARGETS: [&str; 2] = ["member_expression", "subscript_expression"];
pub(crate) const WRAPPERS: [&str; 5] = [
    "parenthesized_expression",
    "non_null_expression",
    "as_expression",
    "satisfies_expression",
    "type_assertion",
];

fn is_relevant(kind: &str) -> bool {
    NAME_LEAVES.contains(&kind)
        || PATTERNS.contains(&kind)
        || TARGETS.contains(&kind)
        || WRAPPERS.contains(&kind)
}

pub(crate) fn parse(json: &str) -> Result<Vec<NodeType>, serde_json::Error> {
    serde_json::from_str(json)
}

fn expand(
    kinds: &[&str],
    supers: &BTreeMap<&str, Vec<&str>>,
    seen: &mut BTreeSet<String>,
) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for k in kinds {
        if let Some(sub) = supers.get(k) {
            if !seen.insert((*k).to_string()) {
                continue;
            }
            out.extend(expand(sub, supers, seen));
        } else {
            out.insert((*k).to_string());
        }
    }
    out
}

/// `(kind, slot) -> expanded named child kinds`; a children slot is named `<children>`.
pub(crate) fn slots(d: &[NodeType]) -> Slots {
    let supers: BTreeMap<&str, Vec<&str>> = d
        .iter()
        .filter(|x| x.named)
        .filter_map(|x| {
            x.subtypes
                .as_ref()
                .map(|s| (x.kind.as_str(), s.iter().map(|t| t.kind.as_str()).collect()))
        })
        .collect();
    fn named(m: &Members) -> Vec<&str> {
        m.types
            .iter()
            .filter(|t| t.named)
            .map(|t| t.kind.as_str())
            .collect()
    }
    let mut res = Slots::new();
    for x in d.iter().filter(|x| x.named && x.subtypes.is_none()) {
        for (f, m) in &x.fields {
            res.insert(
                (x.kind.clone(), f.clone()),
                expand(&named(m), &supers, &mut BTreeSet::new()),
            );
        }
        if let Some(ch) = &x.children {
            res.insert(
                (x.kind.clone(), "<children>".to_string()),
                expand(&named(ch), &supers, &mut BTreeSet::new()),
            );
        }
    }
    res
}

/// Named kinds with no fields, no children and no subtypes.
pub(crate) fn leaves(d: &[NodeType]) -> BTreeSet<String> {
    d.iter()
        .filter(|x| x.named && x.subtypes.is_none() && x.fields.is_empty() && x.children.is_none())
        .map(|x| x.kind.clone())
        .collect()
}

/// Write-relevant slots: those whose expanded alternatives meet the relevant kinds (plus
/// `type_identifier` in a `name` slot, where TS class names bind a value).
pub(crate) fn relevant(slot_map: &Slots) -> BTreeMap<(String, String), BTreeSet<String>> {
    let mut out = BTreeMap::new();
    for (key, kinds) in slot_map {
        let mut hit: BTreeSet<String> = kinds.iter().filter(|k| is_relevant(k)).cloned().collect();
        if key.1 == "name" && kinds.contains("type_identifier") {
            hit.insert("type_identifier".to_string());
        }
        if !hit.is_empty() {
            out.insert(key.clone(), hit);
        }
    }
    out
}

/// Every kind, supertype member, slot and accepted alternative, one line each.
pub(crate) fn signature(d: &[NodeType]) -> BTreeSet<String> {
    let flag = |named: bool| if named { "named" } else { "anon" };
    let cap = |b: bool| if b { "True" } else { "False" };
    let mut lines = BTreeSet::new();
    for x in d {
        let k = &x.kind;
        lines.insert(format!("KIND\t{k}\t{}", flag(x.named)));
        for s in x.subtypes.iter().flatten() {
            lines.insert(format!("SUPER\t{k}\t{}\t{}", s.kind, flag(s.named)));
        }
        let mut all: Vec<(&str, &Members)> =
            x.fields.iter().map(|(f, m)| (f.as_str(), m)).collect();
        if let Some(ch) = &x.children {
            all.push(("<children>", ch));
        }
        for (f, m) in all {
            lines.insert(format!(
                "SLOT\t{k}\t{f}\tmultiple={}\trequired={}",
                cap(m.multiple),
                cap(m.required)
            ));
            for t in &m.types {
                lines.insert(format!("ALT\t{k}\t{f}\t{}\t{}", t.kind, flag(t.named)));
            }
        }
    }
    lines
}
