use super::{EntryError, Reason};
use crate::languages::Language;
use serde::{Deserialize, Serialize};

/// Allocations at the same source byte are distinct across evidenced invocations.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RouterId {
    pub file: String,
    pub binding: String,
    pub construction_byte: usize,
    pub invocations: Vec<Invocation>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(untagged, deny_unknown_fields)]
pub enum Invocation {
    Entry {
        entry: String,
    },
    Call {
        file: String,
        start_byte: usize,
        end_byte: usize,
    },
}

impl RouterId {
    fn valid(&self) -> bool {
        repo_path(&self.file)
            && !self.binding.trim().is_empty()
            && self
                .invocations
                .iter()
                .enumerate()
                .all(|(index, frame)| match frame {
                    Invocation::Entry { entry } => {
                        index == 0 && python_entry(entry).is_some_and(|(_, _, factory)| factory)
                    }
                    Invocation::Call {
                        file,
                        start_byte,
                        end_byte,
                    } => repo_path(file) && start_byte < end_byte,
                })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EntryKind {
    Python {
        module: String,
        binding: String,
        factory: bool,
    },
    JavaScript {
        file: String,
        export: Option<String>,
    },
    Identity(RouterId),
}

impl EntryKind {
    pub fn parse(entry: &str) -> Option<Self> {
        if entry.trim_start().starts_with('{') {
            let identity: RouterId = serde_json::from_str(entry).ok()?;
            return identity.valid().then_some(Self::Identity(identity));
        }
        if entry.contains(':') {
            let (module, binding, factory) = python_entry(entry)?;
            return Some(Self::Python {
                module: module.into(),
                binding: binding.into(),
                factory,
            });
        }
        let (file, export) = entry
            .split_once('#')
            .map_or((entry, None), |(file, name)| (file, Some(name)));
        let file = file.strip_prefix("./").unwrap_or(file);
        // Keep shorthand unambiguous. A serialized identity can spell files
        // whose names contain shorthand delimiters.
        (repo_path(file)
            && !file.contains('\\')
            && file.split('/').all(|part| !matches!(part, "" | "." | ".."))
            && Language::from_path(file).is_some_and(Language::is_js_family)
            && export.is_none_or(|name| identifier(name, true)))
        .then(|| Self::JavaScript {
            file: file.into(),
            export: export.map(str::to_owned),
        })
    }
}

fn python_entry(entry: &str) -> Option<(&str, &str, bool)> {
    let (module, binding) = entry.split_once(':')?;
    let (binding, factory) = binding
        .strip_suffix("()")
        .map_or((binding, false), |name| (name, true));
    (module.split('.').all(|name| identifier(name, false)) && identifier(binding, false))
        .then_some((module, binding, factory))
}

fn repo_path(file: &str) -> bool {
    if file.is_empty()
        || std::path::Path::new(file).is_absolute()
        || file
            .chars()
            .any(|ch| ch.is_control() || matches!(ch, '\u{2028}' | '\u{2029}'))
    {
        return false;
    }
    let mut depth = 0;
    for part in file.split('/') {
        match part {
            "" | "." => {}
            ".." if depth == 0 => return false,
            ".." => depth -= 1,
            _ => depth += 1,
        }
    }
    true
}

fn identifier(name: &str, javascript: bool) -> bool {
    if name.is_empty() {
        return false;
    }
    // The Python grammar supplies XID_Start/XID_Continue. ECMAScript uses
    // ID_Start/ID_Continue instead, plus $, _ and continuation joiners. Map
    // the ID-only characters to a letter before passing through the lexer.
    let mut probe = String::with_capacity(name.len() + 1);
    for (index, ch) in name.chars().enumerate() {
        let js_id_only = matches!(ch,
            '$' | '\u{037a}' | '\u{309b}'..='\u{309c}' | '\u{fc5e}'..='\u{fc63}'
            | '\u{fdfa}'..='\u{fdfb}' | '\u{fe70}' | '\u{fe72}' | '\u{fe74}' | '\u{fe76}'
            | '\u{fe78}' | '\u{fe7a}' | '\u{fe7c}' | '\u{fe7e}'
        ) || (index == 0
            && matches!(ch, '\u{0e33}' | '\u{0eb3}' | '\u{ff9e}'..='\u{ff9f}'))
            || (index != 0 && matches!(ch, '\u{200c}' | '\u{200d}'));
        probe.push(if javascript && js_id_only { '_' } else { ch });
    }
    // Appending one identifier character makes the check lexical, including
    // reserved export names in JS. Scope and binding legality need evidence.
    probe.push('_');
    let mut parser = tree_sitter::Parser::new();
    if parser
        .set_language(&Language::Python.tree_sitter_language())
        .is_err()
    {
        return false;
    }
    parser.parse(&probe, None).is_some_and(|tree| {
        let root = tree.root_node();
        !root.has_error()
            && root.named_child_count() == 1
            && root.named_child(0).is_some_and(|statement| {
                statement.kind() == "expression_statement"
                    && statement.named_child(0).is_some_and(|node| {
                        node.kind() == "identifier" && node.byte_range() == (0..probe.len())
                    })
            })
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RootSelector {
    entry: String,
    kind: EntryKind,
}

/// The adapter supplies only statically evidenced roots and their selectable names.
pub struct RootEvidence {
    pub identity: RouterId,
    pub selectors: Vec<EntryKind>,
}

impl RootSelector {
    pub fn parse(entry: &str) -> Result<Self, EntryError> {
        let kind = EntryKind::parse(entry).ok_or_else(|| unproven(entry))?;
        Ok(Self {
            entry: entry.into(),
            kind,
        })
    }

    pub fn resolve<'a>(&self, roots: &'a [RootEvidence]) -> Result<&'a RouterId, EntryError> {
        let mut matches = roots
            .iter()
            .filter(|root| root.identity.valid())
            .filter(|root| match &self.kind {
                EntryKind::Identity(identity) => &root.identity == identity,
                selector => root.selectors.contains(selector),
            })
            .map(|root| &root.identity);
        let first = matches.next().ok_or_else(|| unproven(&self.entry))?;
        if matches.all(|other| other == first) {
            Ok(first)
        } else {
            Err(unproven(&self.entry))
        }
    }
}

fn unproven(entry: &str) -> EntryError {
    EntryError {
        reason: Reason::EntryUnproven,
        entry: entry.into(),
    }
}
