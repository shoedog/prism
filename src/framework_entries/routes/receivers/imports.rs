//! Source import paths and package member reads for Flask setup.
use super::{
    arguments, attribute, children, direct, import_parts, normalized, python_scope,
    refusals::{Capture, ImportRefusalReason},
    writes::{effective_name, private_name},
    ParsedFile, PythonBindings, PythonKind, PythonMember, PythonValue, Reason, SetupContext,
};
use crate::framework_entries::routes::python::ungroup;
use std::ops::Bound::{Included, Unbounded};
use tree_sitter::Node;

mod candidates;
mod lookup;
mod predicates;
mod reexports;

#[derive(Clone)]
struct Step<'a> {
    name: String,
    read: Option<Node<'a>>,
}

struct ImportPath<'a> {
    module: String,
    steps: Vec<Step<'a>>,
    statement: Node<'a>,
    span: Node<'a>,
    private: bool,
    alias: bool,
    /// The effective name the statement binds, certified at its binding scope.
    key: String,
}

enum Selection<'a> {
    Module(String),
    Member(PythonMember<'a>, String),
    Forwarded(PythonMember<'a>, String),
    Constructor(PythonKind),
    ObjectAttribute(PythonMember<'a>),
}

#[derive(Default)]
struct Proof<'a> {
    target: Option<Selection<'a>>,
    established: bool,
    reason: Option<ImportRefusalReason>,
}

impl<'a> Proof<'a> {
    fn selection(self) -> Option<Selection<'a>> {
        self.target
            .filter(|_| self.established && self.reason.is_none())
    }

    fn apply(&mut self, reason: Option<ImportRefusalReason>) {
        if let Some(reason) = reason {
            self.reason = Some(self.reason.map_or(reason, |earlier| earlier.min(reason)));
        }
    }
}

pub(super) fn import_items(node: Node<'_>) -> impl Iterator<Item = (Node<'_>, Option<Node<'_>>)> {
    let from = node.child_by_field_name("module_name");
    children(node)
        .into_iter()
        .filter(move |child| Some(*child) != from)
        .filter_map(import_parts)
}

pub(super) fn imported_as<'a>(
    file: &'a ParsedFile,
    path: Node<'_>,
    alias: Option<Node<'_>>,
) -> &'a str {
    let local = alias.or_else(|| path.named_child(0)).unwrap_or(path);
    file.node_text(&local)
}

pub(super) fn source_path(file: &ParsedFile, node: Node<'_>) -> String {
    match node.kind() {
        "dotted_name" => children(node)
            .into_iter()
            .map(|part| normalized(file.node_text(&part)))
            .collect::<Vec<_>>()
            .join("."),
        "relative_import" => children(node)
            .into_iter()
            .map(|part| source_path(file, part))
            .collect(),
        "import_prefix" => ".".repeat(
            node.children(&mut node.walk())
                .filter(|part| part.kind() == ".")
                .count(),
        ),
        _ => normalized(file.node_text(&node)).into_owned(),
    }
}

fn module_spelling(module: &str) -> bool {
    module.split('.').any(private_name)
}

fn absolute_module(origin: &str, module: String) -> Option<String> {
    let suffix = module.trim_start_matches('.');
    let relative = module.len() - suffix.len();
    if relative == 0 {
        return Some(module);
    }
    let (dir, _) = origin.rsplit_once('/')?;
    let mut package: Vec<_> = dir.split('/').collect();
    if relative > package.len() {
        return None;
    }
    package.truncate(package.len() + 1 - relative);
    if !suffix.is_empty() {
        package.push(suffix);
    }
    Some(package.join("."))
}

impl<'a> PythonBindings<'a> {
    pub(super) fn import_module(&self, file: &ParsedFile, node: Node<'_>) -> Option<String> {
        absolute_module(&file.path, source_path(file, node))
    }

    pub(super) fn module_file(&self, module: &str) -> Option<&'a ParsedFile> {
        let path = module.replace('.', "/");
        if !self.source_prefixes(&path) {
            return None;
        }
        let module = self.files.get(format!("{path}.py").as_str());
        let package = self.files.get(format!("{path}/__init__.py").as_str());
        match (module, package) {
            (Some(file), None) | (None, Some(file)) => Some(file),
            _ => None,
        }
    }

    fn source_prefixes(&self, path: &str) -> bool {
        path.match_indices('/').all(|(end, _)| {
            !self
                .files
                .contains_key(format!("{}.py", &path[..end]).as_str())
        })
    }

    pub(super) fn source_module(&self, module: &str) -> bool {
        if self.module_file(module).is_some() {
            return true;
        }
        let path = module.replace('.', "/");
        if !self.source_prefixes(&path) || self.files.contains_key(format!("{path}.py").as_str()) {
            return false;
        }
        let prefix = format!("{path}/");
        self.files
            .range::<str, _>((Included(prefix.as_str()), Unbounded))
            .next()
            .is_some_and(|(path, _)| path.starts_with(&prefix))
    }

    fn native_module(&self, module: &str) -> bool {
        matches!(module, "flask" | "flask.views")
            && !self.source_module("flask")
            && !self.files.contains_key("flask.py")
    }

    fn import_path(
        &self,
        file: &'a ParsedFile,
        statement: Node<'a>,
        name: &str,
    ) -> Option<ImportPath<'a>> {
        self.import_paths(file, statement, name).next()
    }

    fn import_paths<'b>(
        &'b self,
        file: &'a ParsedFile,
        statement: Node<'a>,
        name: &'b str,
    ) -> impl Iterator<Item = ImportPath<'a>> + 'b
    where
        'a: 'b,
    {
        let scope = python_scope(statement);
        import_items(statement)
            .filter(move |&(path, alias)| {
                matches!(
                    statement.kind(),
                    "import_statement" | "import_from_statement"
                ) && effective_name(file, scope, imported_as(file, path, alias)) == name
            })
            .filter_map(move |(path, alias)| self.import_item_path(file, statement, path, alias))
    }

    fn import_item_path(
        &self,
        file: &'a ParsedFile,
        statement: Node<'a>,
        path: Node<'a>,
        alias: Option<Node<'a>>,
    ) -> Option<ImportPath<'a>> {
        let local = normalized(imported_as(file, path, alias));
        let written = source_path(file, path);
        let from = statement.child_by_field_name("module_name");
        let (module, steps, span, private) = if let Some(from) = from {
            let from = source_path(file, from);
            let private = module_spelling(&from);
            let module = absolute_module(&file.path, from)?;
            let steps = vec![Step {
                name: written.to_string(),
                read: None,
            }];
            (module, steps, path, private)
        } else {
            let mut parts = written.split('.');
            let module = parts.next()?.to_owned();
            let steps = if alias.is_some() {
                parts
                    .map(|name| Step {
                        name: name.to_owned(),
                        read: None,
                    })
                    .collect()
            } else {
                Vec::new()
            };
            (module, steps, path, false)
        };
        let key = effective_name(
            file,
            python_scope(statement),
            imported_as(file, path, alias),
        );
        Some(ImportPath {
            module,
            steps,
            statement,
            span,
            private: private || module_spelling(&written) || private_name(&local),
            alias: false,
            key: key.into_owned(),
        })
    }

    pub(super) fn imported(
        &self,
        file: &'a ParsedFile,
        expression: Node<'a>,
        context: &SetupContext<'a>,
        depth: usize,
        capture: Capture<'a>,
    ) -> Option<PythonValue<'a>> {
        let path = self.trace_import_root(
            file,
            expression,
            context,
            depth,
            !matches!(capture, Capture::Silent),
        )?;
        let proof = self.prove_import(file, &path, depth);
        let span = match capture {
            Capture::Silent => None,
            Capture::Value => Some(path.span),
            Capture::Callee(call) => Some(call),
        };
        if let (Some(span), Some(reason)) = (span, proof.reason) {
            self.record_import_refusal(file, span, reason);
        }
        let selection = proof.selection().filter(|selection| {
            !matches!(capture, Capture::Callee(_)) || (!path.alias && matches!(
                selection, Selection::Member(member, _) if member.node.kind() == "function_definition"
            ))
        });
        Some(self.imported_value(selection, depth))
    }

    pub(super) fn imported_binding(
        &self,
        member: PythonMember<'a>,
        name: &str,
        depth: usize,
    ) -> PythonValue<'a> {
        let path = self.import_path(member.file, member.node, &normalized(name));
        let selection =
            path.and_then(|path| self.prove_import(member.file, &path, depth).selection());
        self.imported_value(selection, depth)
    }

    fn imported_value(&self, selection: Option<Selection<'a>>, depth: usize) -> PythonValue<'a> {
        let unknown = PythonValue::Unknown(Reason::ImportedUnproven);
        match selection {
            Some(Selection::Module(_)) => PythonValue::Module,
            Some(Selection::Constructor(kind)) => PythonValue::Constructor(kind),
            Some(Selection::ObjectAttribute(member)) => PythonValue::Opaque(member),
            Some(Selection::Member(member, name)) => {
                let value = self.bound(
                    member,
                    &name,
                    &SetupContext::default(),
                    depth.saturating_sub(1),
                );
                match &value {
                    PythonValue::Router(router) if router.identity.file == member.file.path => {
                        value
                    }
                    PythonValue::Definition(def) if def.file.path == member.file.path => value,
                    _ => unknown,
                }
            }
            Some(Selection::Forwarded(..)) | None => unknown,
        }
    }

    fn prove_import(&self, file: &'a ParsedFile, path: &ImportPath<'a>, depth: usize) -> Proof<'a> {
        let mut proof = Proof::default();
        let mut current = Some(Selection::Module(path.module.clone()));
        let mut established = true;
        for step in &path.steps {
            let Some(selection) = current.take() else {
                break;
            };
            let Selection::Module(module) = selection else {
                if established
                    && proof.reason.is_none()
                    && matches!(
                        self.imported_value(Some(selection), depth),
                        PythonValue::Router(_)
                            | PythonValue::Definition(_)
                            | PythonValue::Constructor(_)
                    )
                {
                    current = path
                        .steps
                        .last()
                        .and_then(|step| step.read)
                        .map(|node| Selection::ObjectAttribute(PythonMember { file, node }));
                }
                break;
            };
            let hop = self.module_member(file, &module, step, depth);
            proof.apply(hop.reason);
            established &= hop.established;
            current = hop.target;
        }
        // The name this statement binds must be one static binding at its binding scope.
        match self.writes[file.path.as_str()].binding_scope_of_write(path.statement, &path.key) {
            Some(scope) => {
                if let Err(clause) = self.certify(file, scope, &path.key) {
                    proof.apply(Some(clause.into()));
                }
            }
            None => proof.apply(Some(ImportRefusalReason::GlobalWrite)),
        }
        if path.private {
            proof.apply(Some(ImportRefusalReason::ClassPrivateName));
        }
        let scope = python_scope(path.statement);
        if scope.kind() == "class_definition" {
            proof.apply(Some(ImportRefusalReason::ClassBodyImport));
        }
        proof.target = current;
        proof.established = depth > 0 && established && direct(path.statement, scope);
        proof
    }

    fn module_member(
        &self,
        file: &'a ParsedFile,
        module: &str,
        step: &Step<'a>,
        depth: usize,
    ) -> Proof<'a> {
        let name = &step.name;
        let child = format!("{module}.{name}");
        let native = self.native_module(module);
        let child_exists = self.source_module(&child) || self.native_module(&child);
        let owner = self.module_file(module);
        let sites = owner.map_or(&[][..], |owner| {
            self.writes[owner.path.as_str()].module_sites(name)
        });
        let mut proof = Proof {
            reason: self.member_refusal(owner, module, name, child_exists, sites),
            ..Proof::default()
        };

        if native {
            let kind = match (module, name.as_str()) {
                ("flask", "Flask") => Some(PythonKind::FlaskApp),
                ("flask", "Blueprint") => Some(PythonKind::Blueprint),
                ("flask.views", "View") => Some(PythonKind::View),
                ("flask.views", "MethodView") => Some(PythonKind::MethodView),
                _ => None,
            };
            if let Some(kind) = kind {
                proof.target = Some(Selection::Constructor(kind));
                proof.established = true;
                return proof;
            }
        }

        if let (Some(owner), [node]) = (owner, sites) {
            if direct(*node, owner.tree.root_node()) {
                if let Some(alias) = self.initializer_module(owner, *node, name, &child, depth) {
                    proof.apply(alias.reason);
                    proof.target = alias.target;
                    proof.established = alias.established;
                    return proof;
                }
                if let Some(member) = declaration(owner, *node) {
                    proof.target = Some(Selection::Member(member, name.clone()));
                    proof.established = true;
                    return proof;
                }
                if !child_exists && node.kind() == "import_from_statement" {
                    proof.target = Some(Selection::Forwarded(
                        PythonMember {
                            file: owner,
                            node: *node,
                        },
                        name.clone(),
                    ));
                    proof.established = true;
                    return proof;
                }
            }
        }

        if child_exists {
            let named = (native && child == "flask.views")
                || step
                    .read
                    .is_none_or(|read| self.child_named_before(file, read, &child));
            if named || !sites.is_empty() || proof.reason.is_some() {
                proof.target = Some(Selection::Module(child));
                proof.established = sites.is_empty() && named;
            }
        }
        proof
    }

    fn initializer_module(
        &self,
        owner: &'a ParsedFile,
        node: Node<'a>,
        name: &str,
        child: &str,
        depth: usize,
    ) -> Option<Proof<'a>> {
        if depth == 0 || !owner.path.ends_with("/__init__.py") {
            return None;
        }
        if self.same_child_import(owner, node, name, child) && self.source_module(child) {
            return Some(Proof {
                target: Some(Selection::Module(child.to_owned())),
                established: true,
                reason: None,
            });
        }
        let path = if matches!(node.kind(), "import_statement" | "import_from_statement") {
            self.import_path(owner, node, name)?
        } else {
            self.trace_import(
                owner,
                assignment_alias(node)?,
                &SetupContext::default(),
                depth - 1,
            )?
        };
        let result = self.prove_import(owner, &path, depth - 1);
        // A terminal re-export has only the caller's role-specific refusal.
        matches!(&result.target, Some(Selection::Module(module))
            if self.source_module(module) || self.native_module(module))
        .then_some(result)
    }

    fn child_named_before(&self, file: &ParsedFile, capture: Node<'a>, child: &str) -> bool {
        let scope = python_scope(capture);
        self.writes[file.path.as_str()]
            .imports(scope)
            .iter()
            .copied()
            .take_while(|node| node.start_byte() <= capture.start_byte())
            .filter(|node| node.end_byte() <= capture.start_byte())
            .any(|node| {
                let parent = node
                    .child_by_field_name("module_name")
                    .and_then(|from| self.import_module(file, from));
                import_items(node).any(|(path, _)| {
                    let path = source_path(file, path);
                    if let Some(parent) = &parent {
                        child
                            .strip_prefix(parent)
                            .and_then(|name| name.strip_prefix('.'))
                            == Some(path.as_str())
                    } else {
                        path == child
                            || path
                                .strip_prefix(child)
                                .is_some_and(|rest| rest.starts_with('.'))
                    }
                })
            })
    }

    fn same_child_import(
        &self,
        owner: &'a ParsedFile,
        node: Node<'a>,
        name: &str,
        child: &str,
    ) -> bool {
        if !matches!(node.kind(), "import_statement" | "import_from_statement") {
            return false;
        }
        self.import_path(owner, node, name).is_some_and(|path| {
            let mut module = path.module;
            for step in path.steps {
                module.push('.');
                module.push_str(&step.name);
            }
            module == child
        })
    }

    pub(super) fn direct_callee(
        &self,
        file: &'a ParsedFile,
        node: Node<'a>,
        call: Node<'a>,
    ) -> PythonValue<'a> {
        let context = SetupContext::default();
        if let Some(value) = self.imported(file, node, &context, 24, Capture::Callee(call)) {
            return value;
        }
        let node = ungroup(node);
        let name = effective_name(file, python_scope(node), file.node_text(&node));
        if node.kind() == "identifier"
            && self
                .import_binding(file, node, &name, &context, false)
                .is_some_and(|write| write.kind() == "function_definition")
        {
            return self.value_at(&file.path, node, &context);
        }
        PythonValue::Unknown(Reason::ReceiverUnproven)
    }
}

fn assignment_alias(node: Node<'_>) -> Option<Node<'_>> {
    if node.kind() != "assignment" || node.child_by_field_name("left")?.kind() != "identifier" {
        return None;
    }
    let value = ungroup(node.child_by_field_name("right")?);
    matches!(value.kind(), "identifier" | "attribute").then_some(value)
}

fn declaration<'a>(file: &'a ParsedFile, node: Node<'a>) -> Option<PythonMember<'a>> {
    matches!(
        node.kind(),
        "assignment" | "function_definition" | "class_definition"
    )
    .then_some(PythonMember { file, node })
}
