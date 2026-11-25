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
        (repo_path(file)
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
    !file.contains(['\\', ':', '#'])
        && !file.chars().any(char::is_control)
        && file.split('/').all(|part| !matches!(part, "" | "." | ".."))
}

fn identifier(name: &str, javascript: bool) -> bool {
    let mut chars = name.chars();
    let symbol = |ch| ch == '_' || (javascript && ch == '$');
    chars
        .next()
        .is_some_and(|ch| ch.is_alphabetic() || symbol(ch))
        && chars.all(|ch| ch.is_alphanumeric() || symbol(ch))
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
