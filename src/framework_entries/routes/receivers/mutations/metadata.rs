use super::*;
use crate::framework_entries::routes::receivers::{
    direct, python_scope, PythonMember, SetupContext,
};
use std::collections::BTreeMap;

#[derive(Clone, Copy)]
pub(crate) struct MetadataWrite<'a> {
    pub file: &'a ParsedFile,
    pub node: Node<'a>,
    pub field: &'static str,
    pub value: Option<Node<'a>>,
    object: Node<'a>,
}

#[derive(Default)]
pub(super) struct MetadataWrites<'a> {
    definitions: BTreeMap<Allocation<'a>, Vec<MetadataWrite<'a>>>,
    unresolved: BTreeMap<&'a str, Vec<MetadataWrite<'a>>>,
}

fn definition(member: PythonMember<'_>) -> Allocation<'_> {
    (member.file.path.as_str(), member.node.start_byte())
}

impl<'a> MetadataWrites<'a> {
    pub(super) fn record(
        &mut self,
        file: &'a ParsedFile,
        node: Node<'a>,
        object: Node<'a>,
        field: &str,
        value: Option<Node<'a>>,
        owners: &[PythonValue<'a>],
    ) {
        let field = match field {
            "methods" => "methods",
            "required_methods" => "required_methods",
            "provide_automatic_options" => "provide_automatic_options",
            "__name__" => "__name__",
            _ => return,
        };
        let write = MetadataWrite {
            file,
            node,
            field,
            value,
            object,
        };
        let mut definitions = BTreeSet::new();
        for owner in owners {
            if let PythonValue::Definition(member) = owner {
                definitions.insert(definition(*member));
            }
        }
        if owners
            .first()
            .is_none_or(|owner| matches!(owner, PythonValue::Unknown(_) | PythonValue::Opaque(_)))
        {
            self.unresolved
                .entry(file.path.as_str())
                .or_default()
                .push(write);
        }
        for target in definitions {
            self.definitions.entry(target).or_default().push(write);
        }
    }
}

impl<'a> PythonBindings<'a> {
    pub(crate) fn view_metadata_writes(
        &self,
        member: PythonMember<'a>,
        file: &'a ParsedFile,
        origin: Node<'a>,
        context: &SetupContext<'a>,
    ) -> Vec<MetadataWrite<'a>> {
        let writes = self.mutations.as_ref().expect("indexed Python bindings");
        let fields = &writes.metadata;
        let target = definition(member);
        let mut candidates = fields.definitions.get(&target).cloned().unwrap_or_default();
        for source in [
            Some(file),
            (member.file.path != file.path).then_some(member.file),
        ]
        .into_iter()
        .flatten()
        {
            if let Some(unresolved) = fields.unresolved.get(source.path.as_str()) {
                candidates.extend(unresolved.iter().copied());
            }
        }
        candidates.sort_by_key(|write| {
            (
                write.file.path.as_str(),
                write.node.start_byte(),
                write.field,
            )
        });
        candidates.dedup_by_key(|write| {
            (
                write.file.path.as_str(),
                write.node,
                write.field,
                write.object,
            )
        });

        let mut field_files = BTreeMap::<_, BTreeSet<_>>::new();
        for write in &candidates {
            field_files
                .entry(write.field)
                .or_default()
                .insert(write.file.path.as_str());
        }
        let registration_scope = python_scope(origin);
        let module_before = if registration_scope.kind() == "module" {
            origin.start_byte()
        } else if context.frames.is_empty() {
            // A record_once callback can run before the end of its module.
            registration_scope.start_byte()
        } else {
            self.module_before(file, context)
        };
        let mut ordered = Vec::new();
        for mut write in candidates {
            let source = write.file;
            let scope = python_scope(write.node);
            let same_file = source.path == file.path;
            let order = if !direct(write.node, scope) || field_files[write.field].len() > 1 {
                None
            } else if same_file
                && scope.kind() == "module"
                && write.node.end_byte() <= module_before
            {
                Some(0)
            } else if same_file
                && scope == registration_scope
                && write.node.end_byte() <= origin.start_byte()
            {
                Some(1)
            } else if !same_file
                && source.path == member.file.path
                && scope.kind() == "module"
                && writes
                    .reach
                    .member_precedes(&source.path, &file.path, module_before)
            {
                Some(0)
            } else {
                None
            };
            let proved = matches!(self.value_at(&source.path, write.object, context),
                PythonValue::Definition(owner) if definition(owner) == target);
            if order.is_none() || !proved {
                write.value = None;
            }
            ordered.push((order.unwrap_or(2), write));
        }
        ordered.sort_by_key(|(order, write)| (*order, write.node.start_byte()));
        ordered.into_iter().map(|(_, write)| write).collect()
    }
}
