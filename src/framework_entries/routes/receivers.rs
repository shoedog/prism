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

use super::python::{arguments, attribute, children, walk};
use super::Framework;
use crate::ast::ParsedFile;
use std::collections::BTreeMap;
use tree_sitter::Node;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PythonKind {
    FlaskApp,
    Blueprint,
    View,
    MethodView,
}

#[derive(Clone, Copy)]
pub(crate) struct PythonMember<'a> {
    pub file: &'a ParsedFile,
    pub node: Node<'a>,
}

pub(crate) struct PythonImport<'a> {
    pub module: &'a str,
    pub member: &'a str,
}

#[derive(Clone)]
pub(crate) struct PythonRouter<'a> {
    pub kind: PythonKind,
    pub identity: RouterId,
    pub allocation: PythonMember<'a>,
}

pub(crate) enum PythonValue<'a> {
    Constructor(PythonKind),
    Module(String),
    Definition(PythonMember<'a>),
    Router(PythonRouter<'a>),
    Opaque(PythonMember<'a>),
    Unknown(Reason),
}

#[derive(Default, Clone)]
pub(crate) struct SetupContext<'a> {
    pub factory: Option<PythonMember<'a>>,
    pub frames: Vec<Invocation>,
}

pub(crate) struct PythonBindings<'a> {
    pub files: BTreeMap<&'a str, &'a ParsedFile>,
    nodes: BTreeMap<&'a str, Vec<Node<'a>>>,
}

pub(crate) fn index_python(files: &BTreeMap<String, ParsedFile>) -> PythonBindings<'_> {
    let files: BTreeMap<_, _> = files
        .iter()
        .filter(|(_, file)| file.language == Language::Python)
        .map(|(path, file)| (path.as_str(), file))
        .collect();
    let nodes = files
        .iter()
        .map(|(&path, file)| (path, walk(file.tree.root_node())))
        .collect();
    PythonBindings { files, nodes }
}

pub(crate) fn python_scope(node: Node<'_>) -> Node<'_> {
    let mut child = node;
    while let Some(parent) = child.parent() {
        if matches!(
            parent.kind(),
            "function_definition" | "class_definition" | "lambda"
        ) && parent.child_by_field_name("body") == Some(child)
        {
            return parent;
        }
        child = parent;
    }
    child
}

fn direct(node: Node<'_>, scope: Node<'_>) -> bool {
    let statement = node
        .parent()
        .filter(|p| matches!(p.kind(), "expression_statement" | "decorated_definition"))
        .unwrap_or(node);
    statement.parent() == scope.child_by_field_name("body").or(Some(scope))
}

fn targets(node: Node<'_>) -> Vec<Node<'_>> {
    match node.kind() {
        "identifier" => vec![node],
        "pattern_list"
        | "tuple_pattern"
        | "list_pattern"
        | "list_splat_pattern"
        | "dictionary_splat_pattern"
        | "as_pattern_target" => children(node).into_iter().flat_map(targets).collect(),
        "default_parameter" | "typed_default_parameter" | "typed_parameter" => {
            node.named_child(0).map_or_else(Vec::new, targets)
        }
        _ => vec![],
    }
}

fn import_parts(import: Node<'_>) -> Option<(Node<'_>, Option<Node<'_>>)> {
    match import.kind() {
        "dotted_name" => Some((import, None)),
        "aliased_import" => Some((
            import.child_by_field_name("name")?,
            import.child_by_field_name("alias"),
        )),
        _ => None,
    }
}

fn writes(file: &ParsedFile, node: Node<'_>, name: &str) -> bool {
    let matches = |child: Node<'_>| file.node_text(&child) == name;
    match node.kind() {
        "function_definition" | "class_definition" => {
            node.child_by_field_name("name").is_some_and(matches)
        }
        "assignment"
        | "augmented_assignment"
        | "named_expression"
        | "for_statement"
        | "for_in_clause"
        | "as_pattern" => node
            .child_by_field_name("left")
            .or_else(|| node.child_by_field_name("name"))
            .or_else(|| node.child_by_field_name("alias"))
            .is_some_and(|target| targets(target).into_iter().any(matches)),
        "import_from_statement" | "import_statement" => children(node)
            .into_iter()
            .filter(|child| Some(*child) != node.child_by_field_name("module_name"))
            .any(|child| {
                child.kind() == "wildcard_import"
                    || import_parts(child).is_some_and(|(path, alias)| {
                        file.node_text(&alias.unwrap_or(path)).split('.').next() == Some(name)
                    })
            }),
        "delete_statement" | "global_statement" | "nonlocal_statement" => {
            children(node).into_iter().flat_map(targets).any(matches)
        }
        _ => false,
    }
}

impl<'a> PythonBindings<'a> {
    pub(crate) fn nodes(&self, file: &str) -> &[Node<'a>] {
        self.nodes.get(file).map_or(&[], Vec::as_slice)
    }

    fn module_file(&self, module: &str) -> Option<&'a ParsedFile> {
        let path = module.replace('.', "/");
        if path.match_indices('/').any(|(end, _)| {
            self.files
                .contains_key(format!("{}.py", &path[..end]).as_str())
        }) {
            return None;
        }
        let module = self.files.get(format!("{path}.py").as_str());
        let package = self.files.get(format!("{path}/__init__.py").as_str());
        if module.is_some() && package.is_some() {
            return None;
        }
        module.or(package).copied()
    }

    fn absolute_module(&self, origin: &str, module: &str) -> Option<String> {
        let relative = module.len() - module.trim_start_matches('.').len();
        if relative == 0 {
            return Some(module.into());
        }
        let (dir, _) = origin.rsplit_once('/')?;
        let mut package: Vec<_> = dir.split('/').collect();
        if relative > package.len() {
            return None;
        }
        package.truncate(package.len() + 1 - relative);
        package.extend(
            module[relative..]
                .split('.')
                .filter(|part| !part.is_empty()),
        );
        Some(package.join("."))
    }

    pub(crate) fn reaching(
        &self,
        file: &ParsedFile,
        scope: Node<'a>,
        name: &str,
        before: usize,
    ) -> Option<Node<'a>> {
        self.nodes(&file.path).iter().copied().rev().find(|node| {
            node.end_byte() <= before && python_scope(*node) == scope && writes(file, *node, name)
        })
    }

    pub(crate) fn module_member(
        &self,
        origin: &str,
        import: &PythonImport<'_>,
    ) -> Option<PythonMember<'a>> {
        let file = self.module_file(&self.absolute_module(origin, import.module)?)?;
        let root = file.tree.root_node();
        let node = self.reaching(file, root, import.member, root.end_byte())?;
        (direct(node, root)
            && matches!(
                node.kind(),
                "assignment" | "function_definition" | "class_definition"
            ))
        .then_some(PythonMember { file, node })
    }

    fn imported(
        &self,
        file: &ParsedFile,
        import: &PythonImport<'_>,
        depth: usize,
    ) -> PythonValue<'a> {
        let unknown = PythonValue::Unknown(Reason::ImportedUnproven);
        let Some(module) = self.absolute_module(&file.path, import.module) else {
            return unknown;
        };
        let package_path = module.replace('.', "/");
        let local = self
            .files
            .contains_key(format!("{package_path}.py").as_str())
            || self
                .files
                .contains_key(format!("{package_path}/__init__.py").as_str());
        if !local {
            let kind = match (module.as_str(), import.member) {
                ("flask", "Flask") => Some(PythonKind::FlaskApp),
                ("flask", "Blueprint") => Some(PythonKind::Blueprint),
                ("flask.views", "View") => Some(PythonKind::View),
                ("flask.views", "MethodView") => Some(PythonKind::MethodView),
                _ => None,
            };
            if let Some(kind) = kind {
                return PythonValue::Constructor(kind);
            }
        }
        if let Some(member) = self.module_member(&file.path, import) {
            let value = self.bound(member, import.member, &SetupContext::default(), depth);
            return match &value {
                PythonValue::Router(router) if router.identity.file == member.file.path => value,
                PythonValue::Definition(def) if def.file.path == member.file.path => value,
                _ => unknown,
            };
        }
        let submodule = format!("{module}.{}", import.member);
        if self.module_file(&submodule).is_some() || submodule == "flask.views" {
            PythonValue::Module(submodule)
        } else {
            unknown
        }
    }

    fn named(
        &self,
        file: &'a ParsedFile,
        use_node: Node<'a>,
        name: &str,
        context: &SetupContext<'a>,
        depth: usize,
    ) -> PythonValue<'a> {
        let unknown = PythonValue::Unknown(Reason::ReceiverUnproven);
        let scope = python_scope(use_node);
        if depth == 0 || scope.kind() == "lambda" {
            return unknown;
        }
        if let Some(node) = self.reaching(file, scope, name, use_node.start_byte()) {
            return self.bound(PythonMember { file, node }, name, context, depth - 1);
        }
        if scope.kind() == "module" {
            return unknown;
        }
        if scope.child_by_field_name("parameters").is_some_and(|p| {
            children(p)
                .into_iter()
                .flat_map(targets)
                .any(|id| file.node_text(&id) == name)
        }) || self.reaching(file, scope, name, scope.end_byte()).is_some()
        {
            return unknown;
        }
        let root = file.tree.root_node();
        let mut outer = python_scope(scope);
        if outer.kind() == "class_definition" {
            outer = python_scope(outer);
        }
        if outer != root {
            return unknown;
        }
        let before = context
            .frames
            .iter()
            .find_map(|frame| match frame {
                Invocation::Call {
                    file: path,
                    start_byte,
                    ..
                } if *path == file.path => Some(*start_byte),
                _ => None,
            })
            .unwrap_or(root.end_byte());
        let first = self.reaching(file, root, name, before);
        if context.frames.is_empty() && first != self.reaching(file, root, name, scope.start_byte())
        {
            return unknown;
        }
        first.map_or(unknown, |node| {
            self.bound(PythonMember { file, node }, name, context, depth - 1)
        })
    }

    fn bound(
        &self,
        member: PythonMember<'a>,
        name: &str,
        context: &SetupContext<'a>,
        depth: usize,
    ) -> PythonValue<'a> {
        let PythonMember { file, node } = member;
        let unknown = PythonValue::Unknown(Reason::ReceiverUnproven);
        let scope = python_scope(node);
        if depth == 0 || !direct(node, scope) {
            return unknown;
        }
        match node.kind() {
            "function_definition" | "class_definition" => PythonValue::Definition(member),
            "import_statement" | "import_from_statement" => {
                let module = node.child_by_field_name("module_name");
                for (path, alias) in children(node)
                    .into_iter()
                    .filter(|child| Some(*child) != module)
                    .filter_map(import_parts)
                {
                    let path = file.node_text(&path);
                    let bound = alias.map_or_else(
                        || path.split('.').next().unwrap_or(path),
                        |alias| file.node_text(&alias),
                    );
                    if bound == name {
                        return module.map_or_else(
                            || {
                                PythonValue::Module(
                                    if alias.is_some() { path } else { bound }.into(),
                                )
                            },
                            |module| {
                                self.imported(
                                    file,
                                    &PythonImport {
                                        module: file.node_text(&module),
                                        member: path,
                                    },
                                    depth - 1,
                                )
                            },
                        );
                    }
                }
                PythonValue::Unknown(Reason::ImportedUnproven)
            }
            "assignment"
                if node
                    .child_by_field_name("left")
                    .is_some_and(|left| left.kind() == "identifier") =>
            {
                let Some(value) = node.child_by_field_name("right") else {
                    return unknown;
                };
                if matches!(value.kind(), "identifier" | "attribute")
                    && !self.unwritten_alias(file, node, scope, name)
                {
                    return unknown;
                }
                self.evaluate(file, value, name, context, depth - 1)
            }
            _ => unknown,
        }
    }

    fn evaluate(
        &self,
        file: &'a ParsedFile,
        value: Node<'a>,
        binding: &str,
        context: &SetupContext<'a>,
        depth: usize,
    ) -> PythonValue<'a> {
        let unknown = PythonValue::Unknown(Reason::ReceiverUnproven);
        if depth == 0 || value.has_error() {
            return unknown;
        }
        match value.kind() {
            "identifier" => self.named(file, value, file.node_text(&value), context, depth),
            "parenthesized_expression" => value.named_child(0).map_or(unknown, |value| {
                self.evaluate(file, value, binding, context, depth - 1)
            }),
            "attribute" => {
                let Some((object, name)) = attribute(value) else {
                    return unknown;
                };
                if let PythonValue::Module(module) =
                    self.evaluate(file, object, "", context, depth - 1)
                {
                    self.imported(
                        file,
                        &PythonImport {
                            module: &module,
                            member: file.node_text(&name),
                        },
                        depth - 1,
                    )
                } else {
                    PythonValue::Opaque(PythonMember { file, node: value })
                }
            }
            "call" => {
                let Some(function) = value.child_by_field_name("function") else {
                    return unknown;
                };
                match self.evaluate(file, function, "", context, depth - 1) {
                    PythonValue::Constructor(
                        kind @ (PythonKind::FlaskApp | PythonKind::Blueprint),
                    ) => {
                        let scope = python_scope(value);
                        let invocations = if scope.kind() == "module" {
                            vec![]
                        } else if !context.frames.is_empty()
                            && context.factory.is_some_and(|factory| {
                                factory.node == scope && factory.file.path == file.path
                            })
                        {
                            context.frames.clone()
                        } else {
                            return unknown;
                        };
                        PythonValue::Router(PythonRouter {
                            kind,
                            allocation: PythonMember { file, node: value },
                            identity: RouterId {
                                file: file.path.clone(),
                                binding: binding.into(),
                                construction_byte: value.start_byte(),
                                invocations,
                            },
                        })
                    }
                    PythonValue::Definition(member)
                        if !arguments(value).unpacked() && arguments(value).nodes().is_empty() =>
                    {
                        let mut frames = context.frames.clone();
                        frames.push(Invocation::Call {
                            file: file.path.clone(),
                            start_byte: value.start_byte(),
                            end_byte: value.end_byte(),
                        });
                        self.factory(member, frames, depth - 1).map_or(
                            PythonValue::Opaque(PythonMember { file, node: value }),
                            PythonValue::Router,
                        )
                    }
                    _ => PythonValue::Opaque(PythonMember { file, node: value }),
                }
            }
            _ => PythonValue::Opaque(PythonMember { file, node: value }),
        }
    }

    pub(crate) fn value_at(
        &self,
        file: &str,
        value: Node<'a>,
        context: &SetupContext<'a>,
    ) -> PythonValue<'a> {
        let Some(&file) = self.files.get(file) else {
            return PythonValue::Unknown(Reason::ReceiverUnproven);
        };
        self.evaluate(file, value, file.node_text(&value), context, 24)
    }

    fn factory(
        &self,
        member: PythonMember<'a>,
        frames: Vec<Invocation>,
        depth: usize,
    ) -> Option<PythonRouter<'a>> {
        let PythonMember { file, node } = member;
        if depth == 0
            || frames.is_empty()
            || node.kind() != "function_definition"
            || node
                .parent()
                .is_some_and(|p| p.kind() == "decorated_definition")
            || node.child(0)?.kind() == "async"
            || !children(node.child_by_field_name("parameters")?).is_empty()
        {
            return None;
        }
        let body = node.child_by_field_name("body")?;
        let control: Vec<_> = walk(body)
            .into_iter()
            .filter(|n| {
                matches!(n.kind(), "return_statement" | "yield") && python_scope(*n) == node
            })
            .collect();
        let [ret] = control.as_slice() else {
            return None;
        };
        if ret.kind() != "return_statement"
            || ret.parent() != Some(body)
            || children(body).last() != Some(ret)
        {
            return None;
        }
        let context = SetupContext {
            factory: Some(member),
            frames,
        };
        match self.evaluate(file, ret.named_child(0)?, "", &context, depth - 1) {
            PythonValue::Router(router)
                if router.identity.invocations.starts_with(&context.frames)
                    && !router.identity.binding.is_empty() =>
            {
                Some(router)
            }
            _ => None,
        }
    }

    pub(crate) fn factory_return(
        &self,
        file: &str,
        factory: Node<'a>,
        frames: &[Invocation],
    ) -> Option<RouterId> {
        let member = PythonMember {
            file: self.files.get(file)?,
            node: factory,
        };
        self.factory(member, frames.to_vec(), 24)
            .map(|router| router.identity)
    }

    fn unwritten_alias(
        &self,
        file: &ParsedFile,
        definition: Node<'a>,
        scope: Node<'a>,
        name: &str,
    ) -> bool {
        self.nodes(&file.path)
            .iter()
            .copied()
            .filter(|node| {
                node.start_byte() >= definition.end_byte() && python_scope(*node) == scope
            })
            .all(|node| {
                if writes(file, node, name) {
                    return false;
                }
                if node.kind() != "identifier" || file.node_text(&node) != name {
                    return true;
                }
                let Some(parent) = node.parent() else {
                    return false;
                };
                match parent.kind() {
                    "return_statement" | "assignment" => true,
                    "attribute" => {
                        parent.child_by_field_name("object") != Some(node)
                            || parent.parent().is_some_and(|call| {
                                matches!(call.kind(), "call" | "subscript")
                                    || call.child_by_field_name("right") == Some(parent)
                            })
                    }
                    "keyword_argument" => {
                        parent.child_by_field_name("name") == Some(node)
                            || parent.child_by_field_name("name").is_some_and(|key| {
                                matches!(file.node_text(&key), "view_func" | "blueprint")
                            })
                    }
                    "argument_list" => parent
                        .parent()
                        .and_then(|call| call.child_by_field_name("function"))
                        .and_then(attribute)
                        .is_some_and(|(_, method)| {
                            matches!(
                                file.node_text(&method),
                                "register_blueprint" | "add_url_rule"
                            )
                        }),
                    _ => false,
                }
            })
    }

    pub(crate) fn router_hint(
        &self,
        file: &ParsedFile,
        object: Node<'a>,
        context: &SetupContext<'a>,
    ) -> bool {
        self.nodes(&file.path).iter().any(|node| {
            node.kind() == "assignment"
                && node.start_byte() < object.start_byte()
                && node
                    .child_by_field_name("left")
                    .is_some_and(|left| file.node_text(&left) == file.node_text(&object))
                && node
                    .child_by_field_name("right")
                    .and_then(|rhs| rhs.child_by_field_name("function"))
                    .is_some_and(|callee| {
                        matches!(
                            self.value_at(&file.path, callee, context),
                            PythonValue::Constructor(PythonKind::FlaskApp | PythonKind::Blueprint)
                        )
                    })
        })
    }

    pub(crate) fn roots(
        &self,
        framework: Framework,
        entry: Option<&RootSelector>,
    ) -> Vec<RootEvidence> {
        let mut roots = Vec::new();
        if framework != Framework::Flask {
            return roots;
        }
        for (&path, file) in &self.files {
            let module = path
                .trim_end_matches(".py")
                .trim_end_matches("/__init__")
                .replace('/', ".");
            if self
                .module_file(&module)
                .is_none_or(|owner| owner.path != path)
            {
                continue;
            }
            let root = file.tree.root_node();
            let names: std::collections::BTreeSet<_> = self
                .nodes(path)
                .iter()
                .filter(|n| python_scope(**n) == root && n.kind() == "identifier")
                .map(|n| file.node_text(n))
                .collect();
            for name in names {
                let Some(node) = self.reaching(file, root, name, root.end_byte()) else {
                    continue;
                };
                let selector = |factory| EntryKind::Python {
                    module: module.clone(),
                    binding: name.into(),
                    factory,
                };
                let value = self.bound(
                    PythonMember { file, node },
                    name,
                    &SetupContext::default(),
                    24,
                );
                let value = match value {
                    PythonValue::Definition(member) if entry.is_some_and(|entry| entry.kind == selector(true)
                        || matches!(&entry.kind, EntryKind::Identity(identity) if matches!(identity.invocations.first(), Some(Invocation::Entry { entry }) if EntryKind::parse(entry) == Some(selector(true))))) => {
                        let frame = Invocation::Entry { entry: format!("{module}:{name}()") };
                        self.factory(member, vec![frame], 24).map(|router| (router, true))
                    }
                    PythonValue::Router(router) => Some((router, false)),
                    _ => None,
                };
                if let Some((router, factory)) =
                    value.filter(|(router, _)| router.kind == PythonKind::FlaskApp)
                {
                    roots.push(RootEvidence {
                        identity: router.identity,
                        selectors: vec![selector(factory)],
                    });
                }
            }
        }
        roots
    }
}
