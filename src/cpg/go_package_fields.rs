//! Package-wide func-typed struct fields for the Go receiver recall refusal
//! (#639 S2 fold 1).
//!
//! `p.f = x; p.M()` is not a flow into `p` when `M` is a func-typed *field*:
//! the call reads `p.M`. Syntax alone cannot tell a field from a method, and
//! the field is often declared in another file of the package, so the check
//! runs over the assembled repository (cold, full-hit and partial-hit builds
//! alike) against the per-file edges' recorded selectors.
//!
//! Package identity: the directory plus the `package` clause name. An external
//! test package (`foo_test`) is therefore its own package and does not see the
//! fields of the internal `foo`; internal `_test.go` files (`package foo`)
//! share it. A declaration in a `_test.go` file (field or type) is visible only
//! to call sites in `_test.go` files of the package (#677): a non-test file is
//! compiled without them. A field refuses its name when its type may be callable: `func(..)`,
//! a qualified `pkg.T`, a package-local named type that resolves (through
//! aliases, to depth 8) to a func type, or one whose declaration cannot be
//! classified (undeclared, a type parameter, a cycle). Basic, slice, map, chan,
//! array, pointer, struct and interface types do not refuse. Embedded fields
//! count under their type's name, but an embedded type's own fields are not
//! followed (a recorded gap). Every `field_declaration` of the package files
//! counts, wherever it sits: named top-level types, anonymous struct types and
//! function-local type declarations (fail closed: a local table-struct field
//! named like a method also refuses that method's receiver edges).
use crate::ast::ParsedFile;
use crate::data_flow::DataFlowGraph;
use crate::languages::Language;
use std::collections::{BTreeMap, BTreeSet};
use tree_sitter::Node;

type PackageKey = (String, String);

#[derive(Clone, Debug)]
enum Ty {
    Callable,
    Plain,
    Local(String),
    Unknown,
}

#[derive(Default)]
struct Package {
    /// Type declarations with the `_test.go` flag of their file.
    types: BTreeMap<String, Vec<(bool, Ty)>>,
    /// Fields with the `_test.go` flag of their file.
    fields: Vec<(bool, String, Ty)>,
}

fn is_test_file(path: &str) -> bool {
    path.ends_with("_test.go")
}

const BUILTIN: &[&str] = &[
    "bool",
    "byte",
    "complex64",
    "complex128",
    "error",
    "float32",
    "float64",
    "int",
    "int8",
    "int16",
    "int32",
    "int64",
    "rune",
    "string",
    "uint",
    "uint8",
    "uint16",
    "uint32",
    "uint64",
    "uintptr",
    "any",
    "comparable",
];

fn package_key(file: &str, parsed: &ParsedFile) -> Option<PackageKey> {
    let root = parsed.tree.root_node();
    let mut cursor = root.walk();
    let clause = root
        .named_children(&mut cursor)
        .find(|n| n.kind() == "package_clause")?;
    let mut inner = clause.walk();
    let name = clause
        .named_children(&mut inner)
        .find(|n| n.kind() == "package_identifier")?;
    let dir = file.rsplit_once('/').map_or("", |(d, _)| d);
    Some((dir.to_owned(), parsed.node_text(&name).to_owned()))
}

fn classify(parsed: &ParsedFile, node: Node<'_>) -> Ty {
    match node.kind() {
        "function_type" | "qualified_type" => Ty::Callable,
        "pointer_type"
        | "slice_type"
        | "array_type"
        | "implicit_length_array_type"
        | "map_type"
        | "channel_type"
        | "struct_type"
        | "interface_type" => Ty::Plain,
        "type_identifier" => Ty::Local(parsed.node_text(&node).to_owned()),
        "generic_type" => node
            .child_by_field_name("type")
            .map_or(Ty::Unknown, |t| classify(parsed, t)),
        "parenthesized_type" => node
            .named_child(0)
            .map_or(Ty::Unknown, |t| classify(parsed, t)),
        _ => Ty::Unknown,
    }
}

fn scan(parsed: &ParsedFile, node: Node<'_>, test: bool, pkg: &mut Package) {
    match node.kind() {
        "type_spec" | "type_alias" => {
            if let (Some(name), Some(ty)) = (
                node.child_by_field_name("name"),
                node.child_by_field_name("type"),
            ) {
                pkg.types
                    .entry(parsed.node_text(&name).to_owned())
                    .or_default()
                    .push((test, classify(parsed, ty)));
            }
        }
        "field_declaration" => {
            if let Some(ty) = node.child_by_field_name("type") {
                let class = classify(parsed, ty);
                let mut cursor = node.walk();
                let named: Vec<_> = node.children_by_field_name("name", &mut cursor).collect();
                if named.is_empty() {
                    // Embedded: the field is named after the type's last identifier.
                    let mut t = ty;
                    while let Some(inner) = t.child_by_field_name("name").or_else(|| {
                        matches!(t.kind(), "pointer_type" | "generic_type")
                            .then(|| t.child_by_field_name("type").or_else(|| t.named_child(0)))
                            .flatten()
                    }) {
                        t = inner;
                    }
                    pkg.fields
                        .push((test, parsed.node_text(&t).to_owned(), class));
                } else {
                    for n in named {
                        pkg.fields
                            .push((test, parsed.node_text(&n).to_owned(), class.clone()));
                    }
                }
            }
        }
        _ => {}
    }
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        scan(parsed, child, test, pkg);
    }
}

fn callable_possible(ty: &Ty, types: &BTreeMap<String, Vec<Ty>>, depth: usize) -> bool {
    match ty {
        Ty::Callable | Ty::Unknown => true,
        Ty::Plain => false,
        Ty::Local(name) => match types.get(name) {
            Some(_) if depth >= 8 => true,
            Some(decls) => decls.iter().any(|d| callable_possible(d, types, depth + 1)),
            None => !BUILTIN.contains(&name.as_str()),
        },
    }
}

/// The refusal predicate `(file, selector) -> refused` for the receiver edges
/// the graph recorded, or `None` when it recorded none.
///
/// `package_files` is the file map the package scan reads: a scoped build
/// assembles only its in-scope files but must refuse exactly what a full build
/// refuses, so it passes the whole repository's map here (`None`: `files`).
pub(crate) fn refusals<'a>(
    dfg: &DataFlowGraph,
    files: &'a BTreeMap<String, ParsedFile>,
    package_files: Option<&'a BTreeMap<String, ParsedFile>>,
) -> Option<impl Fn(&str, &str) -> bool + 'a> {
    let ledger_files = dfg.receiver_recall_files();
    if ledger_files.is_empty() {
        return None;
    }
    let mut of_file = BTreeMap::<String, PackageKey>::new();
    for file in &ledger_files {
        if let Some(key) = files
            .get(file)
            .filter(|p| p.language == Language::Go)
            .and_then(|p| package_key(file, p))
        {
            of_file.insert(file.clone(), key);
        }
    }
    let needed: BTreeSet<&PackageKey> = of_file.values().collect();
    let mut packages = BTreeMap::<PackageKey, Package>::new();
    for (path, parsed) in package_files.unwrap_or(files) {
        if parsed.language != Language::Go {
            continue;
        }
        let Some(key) = package_key(path, parsed).filter(|k| needed.contains(k)) else {
            continue;
        };
        scan(
            parsed,
            parsed.tree.root_node(),
            is_test_file(path),
            packages.entry(key).or_default(),
        );
    }
    // Per package: the callable-possible names a `_test.go` call site sees
    // (every declaration) and a non-test call site sees (#677: a non-test file
    // compiles without the package's `_test.go` files, so neither their fields
    // nor their type declarations are visible to it).
    let callable: BTreeMap<PackageKey, [BTreeSet<String>; 2]> = packages
        .into_iter()
        .map(|(key, pkg)| {
            let names = |test_view: bool| {
                let types: BTreeMap<String, Vec<Ty>> = pkg
                    .types
                    .iter()
                    .filter_map(|(n, ds)| {
                        let ds: Vec<Ty> = ds
                            .iter()
                            .filter(|(t, _)| test_view || !t)
                            .map(|(_, ty)| ty.clone())
                            .collect();
                        (!ds.is_empty()).then(|| (n.clone(), ds))
                    })
                    .collect();
                pkg.fields
                    .iter()
                    .filter(|(t, _, ty)| (test_view || !t) && callable_possible(ty, &types, 0))
                    .map(|(_, name, _)| name.clone())
                    .collect::<BTreeSet<_>>()
            };
            (key, [names(false), names(true)])
        })
        .collect();
    Some(move |file: &str, selector: &str| {
        of_file
            .get(file)
            .and_then(|key| callable.get(key))
            .is_some_and(|views| views[usize::from(is_test_file(file))].contains(selector))
    })
}
