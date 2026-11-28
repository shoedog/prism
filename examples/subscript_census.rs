//! Read-only grammar and intraprocedural subscript census.
use anyhow::{ensure, Context, Result};
use prism::{ast::ParsedFile, languages::Language};
use serde_json::{json, Value};
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    fs::{self, File},
    io::{BufWriter, Write},
    path::Path,
};
use tree_sitter::Node;

type N<'t> = Node<'t>;

fn children(node: N<'_>) -> impl Iterator<Item = N<'_>> {
    let mut cursor = node.walk();
    node.children(&mut cursor)
        .filter(|n| !n.is_extra())
        .collect::<Vec<_>>()
        .into_iter()
}

fn named(node: N<'_>) -> impl Iterator<Item = N<'_>> {
    children(node).filter(|n| n.is_named())
}

fn field<'t>(node: N<'t>, name: &str) -> Option<N<'t>> {
    node.child_by_field_name(name)
}

fn in_field(node: N<'_>, parent: N<'_>, name: &str) -> bool {
    field(parent, name) == Some(node)
}

fn token(node: N<'_>, text: &str) -> bool {
    children(node).any(|n| n.kind() == text)
}

fn line(node: N<'_>) -> usize {
    node.start_position().row + 1
}

fn js(lang: Language) -> bool {
    matches!(
        lang,
        Language::JavaScript | Language::TypeScript | Language::Tsx
    )
}

fn sub(kind: &str) -> bool {
    matches!(
        kind,
        "subscript" | "subscript_expression" | "index_expression"
    )
}

fn member(kind: &str) -> bool {
    matches!(
        kind,
        "attribute" | "member_expression" | "selector_expression"
    )
}

fn operand(node: N<'_>) -> Option<N<'_>> {
    field(node, "object")
        .or_else(|| field(node, "operand"))
        .or_else(|| field(node, "value"))
}

fn property(node: N<'_>) -> Option<N<'_>> {
    field(node, "property")
        .or_else(|| field(node, "attribute"))
        .or_else(|| field(node, "field"))
}

fn wrapper(node: N<'_>) -> Option<N<'_>> {
    match node.kind() {
        "parenthesized_expression"
        | "as_expression"
        | "satisfies_expression"
        | "non_null_expression" => named(node).next(),
        "type_assertion" => named(node).last(),
        _ => None,
    }
}

fn unwrap(mut node: N<'_>) -> N<'_> {
    while let Some(inner) = wrapper(node) {
        node = inner;
    }
    node
}

fn identifier(text: &str, dollar: bool) -> bool {
    let mut bytes = text.bytes();
    let initial = |b: u8| b.is_ascii_alphabetic() || b == b'_' || (dollar && b == b'$');
    bytes.next().is_some_and(initial) && bytes.all(|b| initial(b) || b.is_ascii_digit())
}

#[derive(Clone)]
struct Key {
    kind: &'static str,
    value: Option<String>,
}

impl Key {
    fn dynamic(kind: &'static str) -> Self {
        Self { kind, value: None }
    }

    fn string(text: String, lang: Language) -> Self {
        let ident = identifier(&text, js(lang));
        let value = (!js(lang) || (text.is_ascii() && text != "__proto__")).then_some(text);
        let kind = if ident { "str_ident" } else { "str_other" };
        Self { kind, value }
    }
}

fn string_key(p: &ParsedFile, node: N<'_>) -> Key {
    let text = p.node_text(&node);
    let lang = p.language;
    if node.kind() == "template_string" {
        return Key::dynamic("str_interp");
    }
    if node.kind() == "concatenated_string" {
        let mut decoded = String::new();
        let mut failure = None;
        for part in named(node) {
            let key = string_key(p, part);
            if let Some(value) = key.value {
                decoded.push_str(&value);
            } else if failure != Some("str_interp") {
                failure = Some(key.kind);
            }
        }
        return failure.map_or_else(|| Key::string(decoded, lang), Key::dynamic);
    }
    if node.kind() == "raw_string_literal" {
        return text
            .strip_prefix('`')
            .and_then(|s| s.strip_suffix('`'))
            .map_or_else(
                || Key::dynamic("other"),
                |s| Key::string(s.replace('\r', ""), lang),
            );
    }
    let Some(quote) = text.find(['"', '\'']) else {
        return Key::dynamic("other");
    };
    let prefix = text[..quote].to_ascii_lowercase();
    if prefix.contains('f') {
        return Key::dynamic("str_interp");
    }
    if prefix.contains('b') {
        return Key::dynamic("str_other");
    }
    let suffix = &text[quote..];
    let delim = if suffix.starts_with("\"\"\"") || suffix.starts_with("'''") {
        3
    } else {
        1
    };
    let Some(value) = suffix.get(delim..suffix.len().saturating_sub(delim)) else {
        return Key::dynamic("other");
    };
    if value.contains('\\') && !prefix.contains('r') {
        return Key::dynamic("str_escape");
    }
    let value = if lang == Language::Python {
        value.replace("\r\n", "\n").replace('\r', "\n")
    } else {
        value.to_owned()
    };
    Key::string(value, lang)
}

fn index_node(node: N<'_>) -> Option<N<'_>> {
    field(node, "index").or_else(|| field(node, "subscript"))
}

fn index_key(p: &ParsedFile, node: N<'_>, scope: usize, scopes: &[Scope<'_>]) -> Key {
    if node.kind() == "subscript"
        && (token(node, ",")
            || node
                .children_by_field_name("subscript", &mut node.walk())
                .count()
                != 1)
    {
        return Key::dynamic("other");
    }
    index_node(node).map_or_else(
        || Key::dynamic("other"),
        |key| expression_key(p, key, scope, scopes),
    )
}

fn expression_key(p: &ParsedFile, node: N<'_>, scope: usize, scopes: &[Scope<'_>]) -> Key {
    let node = unwrap(node);
    match node.kind() {
        "string"
        | "concatenated_string"
        | "template_string"
        | "interpreted_string_literal"
        | "raw_string_literal" => string_key(p, node),
        "integer" | "int_literal" => Key::dynamic("int"),
        "number" => {
            let compact = p.node_text(&node).replace('_', "");
            let unsigned = compact.trim_end_matches('n');
            let hex = unsigned.starts_with("0x") || unsigned.starts_with("0X");
            let integer = if hex {
                unsigned[2..].bytes().all(|b| b.is_ascii_hexdigit())
            } else {
                !unsigned.contains(['.', 'e', 'E'])
            };
            Key::dynamic(if integer { "int" } else { "other" })
        }
        "identifier" => {
            let name = p.node_text(&node);
            let is_const = p.language == Language::Go
                && resolve(scopes, scope, name, node.start_byte())
                    .is_some_and(|binding| scopes[binding].names[name].kind == DeclKind::Const);
            Key::dynamic(if is_const { "go_const" } else { "name" })
        }
        _ => Key::dynamic("other"),
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum Part {
    Field(String),
    String(String),
    Dynamic(char),
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct Access {
    name: String,
    binding: Option<usize>,
    parts: Vec<Part>,
}

impl Access {
    fn display(&self) -> String {
        let mut text = self.name.clone();
        for part in &self.parts {
            match part {
                Part::Field(name) => {
                    text.push('.');
                    text.push_str(name);
                }
                Part::String(key) => text.push_str(&format!("[s:{}]", json!(key))),
                Part::Dynamic(kind) => text.push_str(&format!("[{kind}]")),
            }
        }
        text
    }

    fn static_path(&self) -> bool {
        self.parts
            .iter()
            .all(|part| !matches!(part, Part::Dynamic(_)))
    }

    fn overlaps(&self, other: &Self) -> bool {
        self.name == other.name
            && self.binding == other.binding
            && self.parts.iter().zip(&other.parts).all(|(a, b)| {
                a == b || matches!(a, Part::Dynamic(_)) || matches!(b, Part::Dynamic(_))
            })
    }
}

fn component(key: &Key, lang: Language, normalize: bool) -> Part {
    match &key.value {
        Some(value) if normalize && js(lang) && key.kind == "str_ident" => {
            Part::Field(value.clone())
        }
        Some(value) => Part::String(value.clone()),
        None => Part::Dynamic(match key.kind {
            "int" => 'i',
            "name" | "go_const" => 'v',
            _ => 'x',
        }),
    }
}

fn path(
    p: &ParsedFile,
    node: N<'_>,
    scope: usize,
    scopes: &[Scope<'_>],
    normalize: bool,
) -> Option<Access> {
    let node = unwrap(node);
    let kind = node.kind();
    if matches!(kind, "identifier" | "this") {
        let name = p.node_text(&node);
        if normalize && js(p.language) && (name.starts_with('$') || name.contains('\\')) {
            return None;
        }
        return Some(Access {
            name: name.to_owned(),
            binding: if kind == "this" {
                Some(scopes[scope].owner_scope)
            } else {
                resolve(scopes, scope, name, node.start_byte())
            },
            parts: Vec::new(),
        });
    }
    if !sub(kind) && !member(kind) {
        return None;
    }
    if normalize && children(node).count() < 2 {
        return None;
    }
    if normalize
        && node
            .children(&mut node.walk())
            .any(|n| n.kind() == "comment")
    {
        return None;
    }
    let mut base = path(p, operand(node)?, scope, scopes, normalize)?;
    let part = if sub(kind) {
        component(&index_key(p, node, scope, scopes), p.language, normalize)
    } else {
        let name = p.node_text(&property(node)?);
        if normalize && js(p.language) && name.contains('\\') {
            return None;
        }
        Part::Field(name.to_owned())
    };
    base.parts.push(part);
    Some(base)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Role {
    Write,
    AugWrite,
    Update,
    Delete,
    ForHead,
    PatternLeaf,
    Read,
    Receiver,
    Unknown,
}

impl Role {
    fn name(self) -> &'static str {
        match self {
            Self::Write => "write",
            Self::AugWrite => "aug_write",
            Self::Update => "update",
            Self::Delete => "delete",
            Self::ForHead => "for_head",
            Self::PatternLeaf => "pattern_leaf",
            Self::Read => "read",
            Self::Receiver => "receiver",
            Self::Unknown => "unknown",
        }
    }

    fn writes(self) -> bool {
        matches!(
            self,
            Self::Write | Self::AugWrite | Self::Update | Self::ForHead | Self::PatternLeaf
        )
    }

    fn reads(self) -> bool {
        matches!(
            self,
            Self::Read | Self::Receiver | Self::AugWrite | Self::Update
        )
    }
}

fn target_wrapper(node: N<'_>, parent: N<'_>, lang: Language) -> bool {
    match parent.kind() {
        "pattern_list" | "tuple_pattern" | "list_pattern" | "list_splat_pattern"
        | "array_pattern" | "object_pattern" | "rest_pattern" | "as_pattern_target" => true,
        "pair_pattern" => in_field(node, parent, "value"),
        "assignment_pattern" | "object_assignment_pattern" => in_field(node, parent, "left"),
        "tuple" | "list" | "list_splat" => lang == Language::Python,
        _ => false,
    }
}

fn receiver(node: N<'_>, parent: N<'_>) -> bool {
    let kind = parent.kind();
    if sub(kind) || member(kind) || kind == "slice_expression" {
        return operand(parent) == Some(node);
    }
    matches!(
        kind,
        "call" | "call_expression" | "new_expression" | "instantiation_expression"
    ) && (in_field(node, parent, "function") || in_field(node, parent, "constructor"))
}

fn role(node: N<'_>, lang: Language) -> (Role, usize) {
    let mut current = node;
    let mut pattern = false;
    let known = node
        .parent()
        .is_some_and(|parent| known_parent(lang, parent.kind()));
    if sub(node.kind()) && !known {
        return (Role::Unknown, node.end_byte());
    }
    while let Some(parent) = current.parent() {
        let left = in_field(current, parent, "left");
        let end = parent.end_byte();
        let result = match parent.kind() {
            "assignment" | "assignment_expression" | "assignment_statement" if left => {
                if parent.kind() == "assignment" && field(parent, "right").is_none() {
                    (Role::Unknown, end)
                } else if parent.kind() == "assignment_statement" && !token(parent, "=") {
                    (Role::AugWrite, end)
                } else if pattern {
                    (Role::PatternLeaf, end)
                } else {
                    (Role::Write, end)
                }
            }
            "augmented_assignment" | "augmented_assignment_expression" if left => {
                (Role::AugWrite, end)
            }
            "update_expression" | "inc_statement" | "dec_statement" => (Role::Update, end),
            "delete_statement" => (Role::Delete, end),
            "unary_expression" if token(parent, "delete") => (Role::Delete, end),
            "for_statement" | "for_in_clause" | "for_in_statement" | "range_clause" if left => (
                Role::ForHead,
                field(parent, "right").map_or(current.end_byte(), |n| n.end_byte()),
            ),
            "receive_statement" if left => (Role::Write, end),
            _ if receiver(current, parent) => (Role::Receiver, node.end_byte()),
            _ if wrapper(parent) == Some(current) => {
                current = parent;
                continue;
            }
            _ if target_wrapper(current, parent, lang) || parent.kind() == "expression_list" => {
                pattern |= lang != Language::Go;
                current = parent;
                continue;
            }
            _ => (Role::Read, node.end_byte()),
        };
        return result;
    }
    (Role::Unknown, node.end_byte())
}

// Immediate parents expanded from the pinned node-types supertypes.
#[rustfmt::skip]
const PYTHON_PARENTS: &[&str] = &[
    "argument_list", "as_pattern", "assert_statement", "assignment", "attribute",
    "augmented_assignment", "await", "binary_operator", "boolean_operator", "call", "chevron",
    "comparison_operator", "conditional_expression", "decorator", "default_parameter",
    "delete_statement", "dictionary_splat", "dictionary_splat_pattern", "elif_clause",
    "except_clause", "except_group_clause", "exec_statement", "expression_list",
    "expression_statement", "for_in_clause", "for_statement", "format_expression",
    "generator_expression", "if_clause", "if_statement", "interpolation", "keyword_argument",
    "lambda", "list", "list_comprehension", "list_pattern", "list_splat", "list_splat_pattern",
    "match_statement", "named_expression", "not_operator", "pair", "parenthesized_expression",
    "pattern_list", "print_statement", "raise_statement", "return_statement", "set",
    "set_comprehension", "slice", "subscript", "tuple", "tuple_pattern", "type",
    "typed_default_parameter", "unary_operator", "while_statement", "with_item", "yield",
];

#[rustfmt::skip]
const JS_FAMILY_PARENTS: &[&str] = &[
    "arguments", "array", "array_pattern", "arrow_function", "assignment_expression",
    "assignment_pattern", "augmented_assignment_expression", "await_expression",
    "binary_expression", "call_expression", "computed_property_name", "export_statement",
    "expression_statement", "for_in_statement", "for_statement", "member_expression",
    "new_expression", "object_assignment_pattern", "pair", "pair_pattern",
    "parenthesized_expression", "rest_pattern", "return_statement", "sequence_expression",
    "spread_element", "subscript_expression", "switch_case", "template_substitution",
    "ternary_expression", "throw_statement", "unary_expression", "update_expression",
    "variable_declarator", "yield_expression",
];

#[rustfmt::skip]
const JS_PARENTS: &[&str] = &[
    "class_heritage", "field_definition", "formal_parameters", "jsx_expression",
];

#[rustfmt::skip]
const TS_PARENTS: &[&str] = &[
    "as_expression", "enum_assignment", "extends_clause", "instantiation_expression",
    "non_null_expression", "optional_parameter", "public_field_definition",
    "required_parameter", "satisfies_expression", "type_assertion", "type_query",
];

#[rustfmt::skip]
const TSX_PARENTS: &[&str] = &[
    "as_expression", "enum_assignment", "extends_clause", "instantiation_expression",
    "jsx_expression", "non_null_expression", "optional_parameter", "public_field_definition",
    "required_parameter", "satisfies_expression", "type_query",
];

#[rustfmt::skip]
const GO_PARENTS: &[&str] = &[
    "argument_list", "array_type", "binary_expression", "call_expression", "dec_statement",
    "defer_statement", "expression_list", "expression_statement", "expression_switch_statement",
    "for_clause", "for_statement", "go_statement", "if_statement", "inc_statement",
    "index_expression", "literal_element", "parenthesized_expression", "range_clause",
    "receive_statement", "selector_expression", "send_statement", "slice_expression",
    "type_assertion_expression", "type_conversion_expression", "type_switch_statement",
    "unary_expression", "variadic_argument",
];

fn known_parent(lang: Language, kind: &str) -> bool {
    let kinds = match lang {
        Language::Python => PYTHON_PARENTS,
        Language::JavaScript => JS_PARENTS,
        Language::TypeScript => TS_PARENTS,
        Language::Tsx => TSX_PARENTS,
        Language::Go => GO_PARENTS,
        _ => return false,
    };
    kinds.binary_search(&kind).is_ok() || js(lang) && JS_FAMILY_PARENTS.binary_search(&kind).is_ok()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DeclKind {
    Formal,
    Local,
    Var,
    Const,
    Short,
    Other,
    Directive,
    Write,
}

#[derive(Clone, Copy)]
struct Declaration {
    kind: DeclKind,
    active: usize,
}

struct Scope<'t> {
    parent: Option<usize>,
    owner_scope: usize,
    names: HashMap<&'t str, Declaration>,
}

fn resolve(scopes: &[Scope<'_>], mut scope: usize, name: &str, byte: usize) -> Option<usize> {
    loop {
        if scopes[scope]
            .names
            .get(name)
            .is_some_and(|decl| decl.active <= byte)
        {
            return Some(scope);
        }
        scope = scopes[scope].parent?;
    }
}

#[derive(Clone, Copy)]
struct BindingSite<'t> {
    kind: DeclKind,
    anchor: N<'t>,
    single: bool,
}

fn parameter_owner(mut node: N<'_>) -> Option<N<'_>> {
    while let Some(parent) = node.parent() {
        if matches!(
            parent.kind(),
            "parameters" | "lambda_parameters" | "formal_parameters" | "parameter_list"
        ) {
            return parent.parent();
        }
        if parent.kind() == "arrow_function" && in_field(node, parent, "parameter") {
            return Some(parent);
        }
        if callable(parent.kind()) || matches!(parent.kind(), "block" | "statement_block") {
            return None;
        }
        node = parent;
    }
    None
}

fn binding<'t>(ident: N<'t>, lang: Language) -> Option<BindingSite<'t>> {
    let mut node = ident;
    while let Some(parent) = node.parent() {
        let kind = parent.kind();
        let left = in_field(node, parent, "left");
        let name = in_field(node, parent, "name");
        let site = |kind, single| {
            Some(BindingSite {
                kind,
                anchor: parent,
                single,
            })
        };
        match kind {
            "parameters" | "formal_parameters" | "lambda_parameters" => {
                return site(DeclKind::Formal, node == ident);
            }
            "required_parameter" | "optional_parameter" if in_field(node, parent, "pattern") => {
                return site(DeclKind::Formal, node == ident);
            }
            "typed_parameter" if parent.named_child(0) == Some(node) => {
                return site(
                    DeclKind::Formal,
                    node == ident || node.kind() == "list_splat_pattern",
                );
            }
            "typed_default_parameter" | "default_parameter" if name => {
                return site(DeclKind::Formal, node == ident);
            }
            "parameter_declaration" | "variadic_parameter_declaration"
                if parent
                    .children_by_field_name("name", &mut parent.walk())
                    .any(|n| n == node) =>
            {
                return site(DeclKind::Formal, true);
            }
            "arrow_function" if in_field(node, parent, "parameter") => {
                return site(DeclKind::Formal, true);
            }
            "variable_declarator" if name => {
                let kind = if parent
                    .parent()
                    .is_some_and(|p| p.kind() == "variable_declaration")
                {
                    DeclKind::Var
                } else {
                    DeclKind::Local
                };
                return site(kind, node == ident && field(parent, "value").is_some());
            }
            "var_spec" | "const_spec"
                if parent
                    .children_by_field_name("name", &mut parent.walk())
                    .any(|n| n == node) =>
            {
                return site(
                    if kind == "const_spec" {
                        DeclKind::Const
                    } else {
                        DeclKind::Local
                    },
                    true,
                );
            }
            "short_var_declaration" | "range_clause" | "receive_statement"
                if left && token(parent, ":=") =>
            {
                return site(DeclKind::Short, true);
            }
            "assignment" if left && field(parent, "right").is_some() => {
                return site(
                    DeclKind::Local,
                    node == ident
                        && token(parent, "=")
                        && field(parent, "right").is_some_and(|n| n.kind() != "assignment"),
                );
            }
            "augmented_assignment" | "assignment" if left => return site(DeclKind::Write, false),
            "assignment_expression"
            | "augmented_assignment_expression"
            | "assignment_statement"
            | "range_clause"
            | "receive_statement"
                if left =>
            {
                return site(DeclKind::Write, false);
            }
            "update_expression" | "inc_statement" | "dec_statement" | "delete_statement" => {
                return site(DeclKind::Write, false);
            }
            "for_statement" | "for_in_clause" if left => return site(DeclKind::Other, false),
            "for_in_statement" if left => {
                let kind = if token(parent, "var") {
                    DeclKind::Var
                } else if token(parent, "let") || token(parent, "const") {
                    DeclKind::Local
                } else {
                    DeclKind::Write
                };
                return site(kind, false);
            }
            "named_expression" if name => return site(DeclKind::Other, false),
            "function_definition"
            | "function_declaration"
            | "generator_function_declaration"
            | "function_expression"
            | "generator_function"
            | "class_definition"
            | "class_declaration"
            | "class"
            | "enum_declaration"
            | "internal_module"
                if name =>
            {
                return site(DeclKind::Other, false);
            }
            "catch_clause" if in_field(node, parent, "parameter") => {
                return site(DeclKind::Other, false);
            }
            "as_pattern" if node.kind() == "as_pattern_target" => {
                return site(DeclKind::Other, false);
            }
            "except_clause" | "except_group_clause"
                if in_field(node, parent, "alias")
                    || node.prev_sibling().is_some_and(|n| n.kind() == "as") =>
            {
                return site(DeclKind::Other, false);
            }
            "type_switch_statement" if in_field(node, parent, "alias") => {
                return site(DeclKind::Other, false);
            }
            "global_statement" | "nonlocal_statement" => return site(DeclKind::Directive, false),
            "import_statement"
            | "import_from_statement"
            | "import_clause"
            | "import_specifier"
            | "namespace_import"
            | "type_alias_statement"
            | "case_pattern" => return site(DeclKind::Other, false),
            "dotted_name" | "aliased_import" | "expression_list" | "splat_pattern" => {}
            "assignment_pattern" if left && parameter_owner(parent).is_some() => {
                return site(DeclKind::Formal, node == ident);
            }
            _ if target_wrapper(node, parent, lang) || wrapper(parent) == Some(node) => {}
            _ => return None,
        }
        node = parent;
    }
    None
}

fn callable(kind: &str) -> bool {
    matches!(
        kind,
        "function_definition"
            | "lambda"
            | "function_declaration"
            | "method_declaration"
            | "method_definition"
            | "function_expression"
            | "arrow_function"
            | "generator_function"
            | "generator_function_declaration"
            | "func_literal"
    )
}

fn is_loop(kind: &str) -> bool {
    matches!(
        kind,
        "for_statement" | "for_in_statement" | "while_statement" | "do_statement" | "for_in_clause"
    )
}

fn comprehension(kind: &str) -> bool {
    matches!(
        kind,
        "list_comprehension"
            | "set_comprehension"
            | "dictionary_comprehension"
            | "generator_expression"
    )
}

fn is_case(kind: &str) -> bool {
    matches!(
        kind,
        "switch_case"
            | "switch_default"
            | "expression_case"
            | "type_case"
            | "communication_case"
            | "default_case"
            | "case_clause"
    )
}

fn exit_statement(kind: &str) -> bool {
    matches!(
        kind,
        "return_statement"
            | "raise_statement"
            | "throw_statement"
            | "break_statement"
            | "continue_statement"
    )
}

#[derive(Clone, Debug, Default)]
struct Route {
    arms: Vec<Arm>,
    loops: Vec<(usize, usize)>,
    field_initializer: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Arm {
    group: usize,
    branch: usize,
    label: &'static str,
}

fn route_for_child(parent: N<'_>, child: N<'_>, mut route: Route) -> Route {
    let kind = parent.kind();
    if is_loop(kind) && child.kind() != "else_clause" {
        route.loops.push((parent.start_byte(), parent.end_byte()));
    }
    if comprehension(kind) && child.kind() != "for_in_clause" {
        if let Some(clause) = named(parent).find(|n| n.kind() == "for_in_clause") {
            route.loops.push((clause.start_byte(), parent.end_byte()));
        }
    }
    let branch = match kind {
        "if_statement"
            if in_field(child, parent, "consequence")
                || matches!(child.kind(), "elif_clause" | "else_clause") =>
        {
            Some("if")
        }
        "if_statement" if in_field(child, parent, "alternative") => Some("if"),
        "conditional_expression" if named(parent).nth(1) != Some(child) => Some("if"),
        "ternary_expression" if !in_field(child, parent, "condition") => Some("if"),
        "try_statement"
            if in_field(child, parent, "body")
                || matches!(
                    child.kind(),
                    "except_clause" | "except_group_clause" | "catch_clause"
                ) =>
        {
            Some("try")
        }
        "switch_body"
        | "expression_switch_statement"
        | "type_switch_statement"
        | "select_statement"
            if is_case(child.kind()) =>
        {
            Some("switch")
        }
        "block"
            if child.kind() == "case_clause"
                && parent
                    .parent()
                    .is_some_and(|n| n.kind() == "match_statement") =>
        {
            Some("match")
        }
        "binary_expression" | "boolean_operator"
            if in_field(child, parent, "right")
                && children(parent)
                    .any(|n| matches!(n.kind(), "&&" | "||" | "??" | "and" | "or")) =>
        {
            Some("short")
        }
        _ => None,
    };
    if let Some(label) = branch {
        route.arms.push(Arm {
            group: parent.start_byte(),
            branch: child.start_byte(),
            label,
        });
    }
    if matches!(kind, "field_definition" | "public_field_definition")
        && in_field(child, parent, "value")
        && !token(parent, "static")
    {
        route.field_initializer = true;
    }
    route
}

#[derive(Clone)]
struct ContextAt {
    scope: usize,
    route: Route,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct Time {
    byte: usize,
    phase: u8,
}

#[derive(Clone)]
struct Site {
    start: usize,
    end: usize,
    line: usize,
    time: Time,
    route: Route,
}

impl Site {
    fn new(node: N<'_>, ctx: &ContextAt, byte: usize, phase: u8) -> Self {
        Self {
            start: node.start_byte(),
            end: node.end_byte(),
            line: line(node),
            route: ctx.route.clone(),
            time: Time { byte, phase },
        }
    }
}

#[derive(Default)]
struct NameFacts {
    occurrences: usize,
    candidates: usize,
    bare: bool,
    address: bool,
    directive: bool,
}

struct Owner<'t> {
    node: N<'t>,
    parent: Option<usize>,
    scope: usize,
    label: String,
    nested: bool,
    facts: HashMap<&'t str, NameFacts>,
    nodes: Vec<(N<'t>, ContextAt)>,
    refused: bool,
}

struct Inventory<'t> {
    parsed: &'t ParsedFile,
    owners: Vec<Owner<'t>>,
    scopes: Vec<Scope<'t>>,
    occupied: HashSet<String>,
}

impl<'t> Inventory<'t> {
    fn new(parsed: &'t ParsedFile) -> Self {
        Self {
            parsed,
            owners: Vec::new(),
            scopes: Vec::new(),
            occupied: HashSet::new(),
        }
    }

    fn scope(&mut self, parent: Option<usize>, owner_scope: Option<usize>) -> usize {
        let index = self.scopes.len();
        self.scopes.push(Scope {
            parent,
            owner_scope: owner_scope.unwrap_or(index),
            names: HashMap::new(),
        });
        index
    }

    fn owner(
        &mut self,
        node: N<'t>,
        parent: Option<usize>,
        scope: Option<usize>,
    ) -> (usize, usize) {
        let index = self.owners.len();
        let scope = self.scope(scope, None);
        let label = if let Some(parent) = parent {
            self.owners[parent].nested = true;
            let name = field(node, "name")
                .or_else(|| {
                    node.parent().and_then(|n| {
                        if matches!(n.kind(), "variable_declarator" | "pair") {
                            field(n, "name").or_else(|| field(n, "key"))
                        } else {
                            None
                        }
                    })
                })
                .map_or("<anonymous>", |n| self.parsed.node_text(&n));
            let label = format!("{name}@{}", line(node));
            if self.occupied.insert(label.clone()) {
                label
            } else {
                format!("{label}:{}", node.start_position().column + 1)
            }
        } else {
            "<module>".to_owned()
        };
        self.owners.push(Owner {
            node,
            parent,
            scope,
            label,
            nested: false,
            facts: HashMap::new(),
            nodes: Vec::new(),
            refused: false,
        });
        (index, scope)
    }

    fn walk(&mut self, node: N<'t>, mut owner: usize, mut ctx: ContextAt) {
        let lang = self.parsed.language;
        let kind = node.kind();
        if callable(kind) {
            (owner, ctx.scope) = self.owner(node, Some(owner), Some(ctx.scope));
            ctx.route = Route::default();
        } else if lang != Language::Python
            && node != self.owners[owner].node
            && matches!(
                kind,
                "block"
                    | "statement_block"
                    | "class_static_block"
                    | "class_body"
                    | "catch_clause"
                    | "for_statement"
                    | "for_in_statement"
                    | "if_statement"
                    | "switch_body"
                    | "expression_switch_statement"
                    | "type_switch_statement"
            )
            && field(self.owners[owner].node, "body") != Some(node)
            || lang == Language::Go && is_case(kind)
        {
            ctx.scope = self.scope(Some(ctx.scope), Some(self.owners[owner].scope));
        }
        if matches!(
            kind,
            "identifier"
                | "shorthand_property_identifier"
                | "shorthand_property_identifier_pattern"
        ) {
            self.observe_name(node, owner, &ctx);
        }
        if js(lang)
            && (kind == "with_statement"
                || kind == "call_expression"
                    && field(node, "function").is_some_and(|f| self.parsed.node_text(&f) == "eval"))
        {
            self.owners[owner].refused = true;
        }
        if sub(kind)
            || member(kind)
            || kind == "identifier"
            || kind == "this"
            || exit_statement(kind)
            || matches!(
                kind,
                "call_expression" | "dictionary" | "object" | "composite_literal"
            )
        {
            self.owners[owner].nodes.push((node, ctx.clone()));
        }
        for child in named(node) {
            // A declaration name belongs to the enclosing scope, not its callable body.
            if callable(kind)
                && in_field(child, node, "name")
                && matches!(
                    kind,
                    "function_definition"
                        | "function_declaration"
                        | "generator_function_declaration"
                )
            {
                let parent = self.owners[owner]
                    .parent
                    .expect("callable has an enclosing owner");
                let outer = ContextAt {
                    scope: self.scopes[ctx.scope].parent.unwrap(),
                    route: Route::default(),
                };
                self.walk(child, parent, outer);
                continue;
            }
            let child_ctx = ContextAt {
                scope: ctx.scope,
                route: route_for_child(node, child, ctx.route.clone()),
            };
            self.walk(child, owner, child_ctx);
        }
    }

    fn observe_name(&mut self, ident: N<'t>, owner: usize, ctx: &ContextAt) {
        let p = self.parsed;
        let lang = p.language;
        let name = p.node_text(&ident);
        if name == "_" && lang == Language::Go {
            return;
        }
        let is_bind = binding(ident, lang);
        if js(lang) && (name.contains('\\') || (name == "arguments" && is_bind.is_none())) {
            self.owners[owner].refused = true;
        }
        let mut parent = ident.parent();
        let mut bare = is_bind.is_none();
        let mut address = false;
        let mut current = ident;
        while let Some(node) = parent {
            if wrapper(node) != Some(current) {
                bare &= !(receiver(current, node)
                    && (sub(node.kind())
                        || member(node.kind())
                        || node.kind() == "slice_expression"));
                let mut path_end = node;
                while receiver(current, path_end)
                    && (member(path_end.kind()) || sub(path_end.kind()))
                {
                    current = path_end;
                    let Some(up) = path_end.parent() else { break };
                    path_end = up;
                }
                address = path_end.kind() == "slice_expression" && receiver(current, path_end)
                    || path_end.kind() == "unary_expression" && token(path_end, "&");
                break;
            }
            current = node;
            parent = node.parent();
        }
        let mut candidate = false;
        if let Some(site) = is_bind {
            let scope = match site.kind {
                _ if lang == Language::Python => self.owners[owner].scope,
                DeclKind::Var | DeclKind::Formal => self.owners[owner].scope,
                _ => ctx.scope,
            };
            let declaring = !matches!(site.kind, DeclKind::Directive | DeclKind::Write);
            let active = if lang == Language::Go && site.kind != DeclKind::Formal && scope != 0 {
                site.anchor.end_byte()
            } else {
                0
            };
            if declaring {
                self.scopes[scope].names.entry(name).or_insert(Declaration {
                    kind: site.kind,
                    active,
                });
            } else if lang == Language::Python && site.kind == DeclKind::Write {
                self.scopes[scope].names.entry(name).or_insert(Declaration {
                    kind: site.kind,
                    active: 0,
                });
            }
            let owner_node = self.owners[owner].node;
            candidate = site.single
                && match lang {
                    Language::Python => match site.kind {
                        DeclKind::Formal => parameter_owner(ident) == Some(owner_node),
                        DeclKind::Local => {
                            site.anchor.kind() == "assignment"
                                && site
                                    .anchor
                                    .parent()
                                    .is_some_and(|n| n.kind() == "expression_statement")
                                && ctx.route.loops.is_empty()
                                && !has_ancestor_until(ident, owner_node, comprehension)
                        }
                        _ => false,
                    },
                    Language::Go => {
                        owner != 0
                            && matches!(
                                site.kind,
                                DeclKind::Formal | DeclKind::Short | DeclKind::Local
                            )
                    }
                    _ => match site.kind {
                        DeclKind::Formal => parameter_owner(ident) == Some(owner_node),
                        DeclKind::Var => !has_ancestor_until(ident, owner_node, |kind| {
                            matches!(kind, "class_static_block" | "class_body")
                        }),
                        DeclKind::Local => {
                            scope == self.owners[owner].scope
                                && site.anchor.parent().and_then(|n| n.parent())
                                    == field(owner_node, "body")
                                        .or_else(|| (owner == 0).then_some(owner_node))
                        }
                        _ => false,
                    },
                };
        }
        let mut active_owner = Some(owner);
        while let Some(index) = active_owner {
            let facts = self.owners[index].facts.entry(name).or_default();
            facts.occurrences += usize::from(is_bind.is_some());
            facts.candidates += usize::from(candidate && index == owner);
            facts.address |= address;
            facts.bare |= bare;
            facts.directive |= is_bind.is_some_and(|site| site.kind == DeclKind::Directive);
            active_owner = self.owners[index].parent;
        }
    }
}

fn has_ancestor_until(mut node: N<'_>, limit: N<'_>, check: impl Fn(&str) -> bool) -> bool {
    while let Some(parent) = node.parent() {
        if parent == limit {
            break;
        }
        if check(parent.kind()) {
            return true;
        }
        node = parent;
    }
    false
}

fn route_compatible(w: &Route, r: &Route) -> bool {
    !w.arms.iter().any(|a| {
        r.arms
            .iter()
            .any(|b| a.group == b.group && a.branch != b.branch)
    })
}

fn common_loop(w: &Route, r: &Route) -> Option<(usize, usize)> {
    w.loops.iter().rev().copied().find(|l| r.loops.contains(l))
}

fn required(site: &Site, w: &Site, r: &Site) -> bool {
    site.route
        .arms
        .iter()
        .all(|a| w.route.arms.contains(a) || r.route.arms.contains(a))
        && site
            .route
            .loops
            .iter()
            .all(|l| w.route.loops.contains(l) || r.route.loops.contains(l))
}

fn between_rows<T>(
    sites: &[T],
    w: &Site,
    r: &Site,
    time: impl Fn(&T) -> Time,
    test: impl Fn(&T) -> bool,
) -> bool {
    let after = |bound| sites.partition_point(|s| time(s) <= bound);
    let before = |bound| sites.partition_point(|s| time(s) < bound);
    if w.time < r.time {
        sites[after(w.time)..before(r.time)].iter().any(test)
    } else if let Some((start, end)) = common_loop(&w.route, &r.route) {
        let tail = before(Time {
            byte: end,
            phase: 2,
        });
        let head = before(Time {
            byte: start,
            phase: 0,
        });
        sites[after(w.time).min(tail)..tail].iter().any(&test)
            || sites[head..before(r.time).max(head)].iter().any(test)
    } else {
        false
    }
}

fn between(sites: &[Site], w: &Site, r: &Site, test: impl Fn(&Site) -> bool) -> bool {
    between_rows(sites, w, r, |s| s.time, test)
}

#[derive(Clone)]
struct Occurrence {
    path: Access,
    key: Key,
    site: Site,
    role: Role,
    dot: bool,
}

#[derive(Clone)]
struct Alias {
    target: Access,
    source: Access,
    site: Site,
}

#[derive(Default)]
struct Events {
    writes: HashMap<Access, Vec<Occurrence>>,
    reads: HashMap<Access, Vec<Occurrence>>,
    barriers: HashMap<Access, Vec<Site>>,
    exits: Vec<Site>,
    aliases: HashMap<Access, Vec<Alias>>,
    roots: HashMap<Access, Vec<(Access, Site)>>,
    whole_reads: Vec<(Access, Site, &'static str)>,
}

fn base(path: &Access) -> Access {
    let mut result = path.clone();
    result.parts.pop();
    result
}

fn root(path: &Access) -> Access {
    Access {
        name: path.name.clone(),
        binding: path.binding,
        parts: Vec::new(),
    }
}

impl Events {
    fn add_barrier(&mut self, path: Access, site: Site) {
        self.roots
            .entry(root(&path))
            .or_default()
            .push((path.clone(), site.clone()));
        if path.static_path() {
            self.barriers.entry(path).or_default().push(site);
        }
    }

    fn add_occurrence(&mut self, occ: Occurrence, read: Site) {
        let base = base(&occ.path);
        if occ.role.reads() {
            let mut use_occ = occ.clone();
            use_occ.site = read;
            self.reads.entry(base.clone()).or_default().push(use_occ);
        }
        if occ.role.writes() || occ.role == Role::Delete {
            self.add_barrier(occ.path.clone(), occ.site.clone());
        }
        if occ.role.writes() {
            self.writes.entry(base).or_default().push(occ);
        }
    }

    fn sort(&mut self) {
        self.exits.sort_by_key(|s| s.time);
        for sites in self.barriers.values_mut() {
            sites.sort_by_key(|s| s.time);
        }
        for sites in self.writes.values_mut().chain(self.reads.values_mut()) {
            sites.sort_by_key(|occ| occ.site.time);
        }
    }

    fn feasible(&self, w: &Site, r: &Site, path: &Access) -> bool {
        if w.start == r.start && w.end == r.end
            || !route_compatible(&w.route, &r.route)
            || w.time >= r.time && common_loop(&w.route, &r.route).is_none()
        {
            return false;
        }
        let on_route = |s: &Site| required(s, w, r);
        if between(&self.exits, w, r, on_route) {
            return false;
        }
        let mut prefix = path.clone();
        loop {
            if self
                .barriers
                .get(&prefix)
                .is_some_and(|sites| between(sites, w, r, on_route))
            {
                return false;
            }
            if prefix.parts.pop().is_none() {
                return true;
            }
        }
    }

    fn alias_ok(&self, w: &Occurrence, r: &Occurrence) -> bool {
        let original = root(&w.path);
        let clean = |alias: &Alias, reverse: bool| {
            let shadow = if reverse {
                &alias.source
            } else {
                &alias.target
            };
            if !route_compatible(&alias.site.route, &w.site.route)
                || !route_compatible(&alias.site.route, &r.site.route)
            {
                return true;
            }
            let mut alias_path = shadow.clone();
            alias_path.parts.clone_from(&w.path.parts);
            self.roots.get(shadow).is_none_or(|writes| {
                writes.iter().all(|(path, site)| {
                    path.parts.is_empty()
                        || !alias_path.overlaps(path)
                        || alias.site.time > site.time
                        || !between(std::slice::from_ref(site), &w.site, &r.site, |s| {
                            required(s, &w.site, &r.site)
                        })
                        || self.barriers.get(shadow).is_some_and(|bindings| {
                            between(bindings, &alias.site, site, |s| {
                                required(s, &alias.site, site)
                            })
                        })
                })
            })
        };
        self.aliases.get(&original).is_none_or(|aliases| {
            aliases.iter().all(|alias| {
                if alias.target == original {
                    clean(alias, true)
                } else {
                    clean(alias, false)
                }
            })
        })
    }
}

fn whole_operand<'t>(node: N<'t>, p: &ParsedFile) -> Option<&'static str> {
    let mut current = node;
    while let Some(parent) = current.parent() {
        match parent.kind() {
            "return_statement" => return Some("return"),
            "assignment"
            | "assignment_expression"
            | "assignment_statement"
            | "short_var_declaration"
                if in_field(current, parent, "right")
                    && (parent.kind() != "assignment_statement" || token(parent, "=")) =>
            {
                return Some("assign_rhs");
            }
            "variable_declarator" | "var_spec" if in_field(current, parent, "value") => {
                return Some("assign_rhs");
            }
            "argument_list" | "arguments" => {
                let call = parent.parent()?;
                let callee = field(call, "function").map(|n| p.node_text(&n));
                if matches!(
                    callee,
                    Some("len" | "cap" | "id" | "Object.is" | "delete" | "clear")
                ) || matches!(call.kind(), "type_conversion_expression")
                {
                    return None;
                }
                return Some("arg");
            }
            "expression_list" if p.language == Language::Go => {}
            "keyword_argument" if in_field(current, parent, "value") => {}
            _ if wrapper(parent) == Some(current) => {}
            _ => return None,
        }
        current = parent;
    }
    None
}

fn single_expression(node: N<'_>) -> Option<N<'_>> {
    if node.kind() == "expression_list" {
        let mut expressions = named(node);
        let first = expressions.next()?;
        return expressions.next().is_none().then_some(first);
    }
    Some(node)
}

fn direct_assignment(node: N<'_>) -> Option<(N<'_>, N<'_>)> {
    let (left, right) = match node.kind() {
        "assignment" | "assignment_expression" | "short_var_declaration" => {
            (field(node, "left")?, field(node, "right")?)
        }
        "assignment_statement" if token(node, "=") => (field(node, "left")?, field(node, "right")?),
        "variable_declarator" | "var_spec" => (field(node, "name")?, field(node, "value")?),
        _ => return None,
    };
    if node.kind() == "var_spec"
        && node
            .children_by_field_name("name", &mut node.walk())
            .count()
            != 1
    {
        return None;
    }
    Some((single_expression(left)?, single_expression(right)?))
}

fn alias_assignment<'t>(ident: N<'t>, site: BindingSite<'t>) -> Option<N<'t>> {
    let pair = if site.kind == DeclKind::Formal && site.anchor.kind() == "assignment_pattern" {
        (field(site.anchor, "left")?, field(site.anchor, "right")?)
    } else {
        direct_assignment(site.anchor)?
    };
    (pair.0 == ident && pair.1.kind() == "identifier").then_some(pair.1)
}

fn relation(w: &Key, r: &Key) -> &'static str {
    match (&w.value, &r.value) {
        (Some(w), Some(r)) if w == r => "same",
        (Some(_), Some(_)) => "different",
        (None, Some(_)) => "w_dynamic",
        (Some(_), None) => "r_dynamic",
        (None, None) => "both_dynamic",
    }
}

struct Rows<'a> {
    writer: &'a mut dyn Write,
    corpus: &'a str,
    lang: &'a str,
    file: &'a str,
    owner: &'a str,
}

impl Rows<'_> {
    fn emit(&mut self, kind: &str, mut row: Value) -> Result<()> {
        row["kind"] = json!(kind);
        row["corpus"] = json!(self.corpus);
        row["lang"] = json!(self.lang);
        row["file"] = json!(self.file);
        row["owner"] = json!(self.owner);
        serde_json::to_writer(&mut self.writer, &row)?;
        self.writer.write_all(b"\n")?;
        Ok(())
    }
}

fn arm_text(route: &Route) -> String {
    route
        .arms
        .iter()
        .map(|a| format!("{}#{}:arm{}", a.label, a.group, a.branch))
        .collect::<Vec<_>>()
        .join("/")
}

fn go_delete_target(
    p: &ParsedFile,
    node: N<'_>,
    scope: usize,
    scopes: &[Scope<'_>],
) -> Option<Access> {
    let callee = field(node, "function")?;
    if p.node_text(&callee) != "delete"
        || resolve(scopes, scope, "delete", callee.start_byte()).is_some()
    {
        return None;
    }
    let args: Vec<_> = named(field(node, "arguments")?).collect();
    let [map, index] = args.as_slice() else {
        return None;
    };
    let mut target = path(p, *map, scope, scopes, true)?;
    let key = expression_key(p, *index, scope, scopes);
    target.parts.push(component(&key, p.language, true));
    Some(target)
}

fn index_events<'t>(
    p: &'t ParsedFile,
    scopes: &[Scope<'t>],
    owner: &Owner<'t>,
    rows: &mut Rows<'_>,
) -> Result<(Events, usize, usize)> {
    let mut events = Events::default();
    let mut n_sub = 0;
    let mut go_delete = 0;
    for (node, ctx) in &owner.nodes {
        let node = *node;
        let kind = node.kind();
        let read = Site::new(node, ctx, node.end_byte(), 0);
        if exit_statement(kind) {
            events.exits.push(Site::new(node, ctx, node.end_byte(), 2));
            continue;
        }
        if p.language == Language::Go && kind == "call_expression" {
            if let Some(target) = go_delete_target(p, node, ctx.scope, scopes) {
                events.add_barrier(target, Site::new(node, ctx, node.end_byte(), 1));
                go_delete += 1;
            }
        }
        let is_sub = sub(kind);
        let is_member = member(kind);
        let is_name = matches!(kind, "identifier" | "this");
        if !is_sub && !is_member && !is_name {
            continue;
        }
        let (role, at) = role(node, p.language);
        let key =
            if is_sub {
                let key = index_key(p, node, ctx.scope, scopes);
                let previous = operand(node).and_then(|b| path(p, b, ctx.scope, scopes, false));
                let admitted = path(p, node, ctx.scope, scopes, true).is_some();
                n_sub += 1;
                rows.emit("sub", json!({
                "line": read.line, "start": read.start, "end": read.end, "role": role.name(),
                "base_chain": previous.as_ref().map(Access::display),
                "index_kind": key.kind, "key": key.value, "admitted_chain": admitted,
                "loop_id": ctx.route.loops.last().map(|l| l.0), "arm": arm_text(&ctx.route),
                "in_nested_callable": owner.parent.is_some_and(|parent| parent != 0),
                "is_test_file": false,
            }))?;
                Some(key)
            } else if is_member {
                property(node).map(|key| Key::string(p.node_text(&key).to_owned(), p.language))
            } else {
                None
            };
        let Some(mut access) = path(p, node, ctx.scope, scopes, true) else {
            continue;
        };
        if is_name {
            if let Some(binding) = binding(node, p.language) {
                let at = if role == Role::ForHead {
                    at
                } else {
                    binding.anchor.end_byte()
                };
                if p.language == Language::Go {
                    access.binding = resolve(scopes, ctx.scope, &access.name, at);
                }
                let site = Site::new(node, ctx, at, 1);
                if let Some(source) = alias_assignment(node, binding)
                    .and_then(|n| path(p, n, ctx.scope, scopes, true))
                    .filter(|source| *source != access)
                {
                    let alias = Alias {
                        target: access.clone(),
                        source: source.clone(),
                        site: site.clone(),
                    };
                    events
                        .aliases
                        .entry(source)
                        .or_default()
                        .push(alias.clone());
                    events
                        .aliases
                        .entry(access.clone())
                        .or_default()
                        .push(alias);
                }
                if !matches!(binding.kind, DeclKind::Directive | DeclKind::Formal) {
                    events.add_barrier(access, site);
                }
                continue;
            }
        }
        if let Some(consumer) = whole_operand(node, p) {
            events
                .whole_reads
                .push((access.clone(), read.clone(), consumer));
        }
        if let Some(key) = key {
            let dot = !is_sub;
            let site = Site::new(node, ctx, at, 1);
            if !dot || js(p.language) && key.kind == "str_ident" && key.value.is_some() {
                events.add_occurrence(
                    Occurrence {
                        path: access,
                        key,
                        site,
                        role,
                        dot,
                    },
                    read,
                );
            } else if role.writes() || role == Role::Delete {
                events.add_barrier(access, site);
            }
        }
    }
    events.sort();
    Ok((events, n_sub, go_delete))
}

fn emit_pairs(
    p: &ParsedFile,
    owner: &Owner<'_>,
    scopes: &[Scope<'_>],
    events: &Events,
    rows: &mut Rows<'_>,
) -> Result<()> {
    let mut groups: Vec<_> = events.writes.iter().collect();
    groups.sort_by_key(|(_, writes)| writes[0].site.start);
    for (base, writes) in groups {
        let Some(reads) = events.reads.get(base) else {
            continue;
        };
        let facts = owner.facts.get(base.name.as_str());
        let definite = if js(p.language) && base.name == "this" {
            !owner.refused
        } else {
            !owner.refused
                && facts.is_some_and(|f| {
                    f.occurrences == 1
                        && f.candidates == 1
                        && !f.directive
                        && (p.language != Language::Go || !f.address)
                })
                && base
                    .binding
                    .is_some_and(|scope| scopes[scope].owner_scope == owner.scope)
        };
        for w in writes {
            let earliest = w.site.route.loops.first().map_or(w.site.time.byte, |l| l.0);
            for r in &reads[reads.partition_point(|r| r.site.time.byte < earliest)..] {
                let relation = relation(&w.key, &r.key);
                if w.dot && r.dot
                    || (w.dot || r.dot) && relation != "same"
                    || !events.feasible(&w.site, &r.site, &w.path)
                {
                    continue;
                }
                let different_kill = js(p.language)
                    && relation == "same"
                    && between_rows(
                        writes,
                        &w.site,
                        &r.site,
                        |other| other.site.time,
                        |other| {
                            other.key.value.is_some()
                                && other.key.value != w.key.value
                                && required(&other.site, &w.site, &r.site)
                        },
                    );
                rows.emit(
                    "pair",
                    json!({
                        "w_start": w.site.start, "w_end": w.site.end, "w_line": w.site.line,
                        "r_start": r.site.start, "r_end": r.site.end, "r_line": r.site.line,
                        "key_relation": relation, "w_via_dot": w.dot, "r_via_dot": r.dot,
                        "w_role": w.role.name(), "base_text": base.display(),
                        "base_definite": definite && !w.site.route.field_initializer,
                        "alias_c_ok": events.alias_ok(w, r),
                        "alias_a_ok": facts.is_none_or(|f| !f.bare),
                        "different_key_kill_between": different_kill,
                    }),
                )?;
            }
        }
    }
    Ok(())
}

fn emit_whole(events: &Events, rows: &mut Rows<'_>) -> Result<()> {
    let mut by_prefix: HashMap<Access, Vec<&Occurrence>> = HashMap::new();
    for (base, writes) in &events.writes {
        let mut prefix = base.clone();
        loop {
            by_prefix
                .entry(prefix.clone())
                .or_default()
                .extend(writes.iter().filter(|w| !w.dot));
            if prefix.parts.pop().is_none() {
                break;
            }
        }
    }
    for (container, read, consumer) in &events.whole_reads {
        let Some(writes) = by_prefix.get(container) else {
            continue;
        };
        if let Some(w) = writes
            .iter()
            .filter(|w| events.feasible(&w.site, read, &w.path))
            .max_by_key(|w| (w.site.time < read.time, w.site.time, w.site.start))
        {
            rows.emit(
                "whole",
                json!({
                    "w_start": w.site.start, "r_start": read.start, "r_end": read.end,
                    "consumer": consumer, "base_text": container.display(),
                }),
            )?;
        }
    }
    Ok(())
}

fn initializer_target(node: N<'_>) -> Option<(N<'_>, N<'_>)> {
    let mut current = node;
    while let Some(parent) = current.parent() {
        if wrapper(parent) == Some(current) || parent.kind() == "expression_list" {
            current = parent;
        } else {
            let (lhs, rhs) = direct_assignment(parent)?;
            return (lhs.kind() == "identifier" && unwrap(rhs) == node).then_some((lhs, parent));
        }
    }
    None
}

fn emit_literals<'t>(
    p: &'t ParsedFile,
    owner: &Owner<'t>,
    scopes: &[Scope<'t>],
    events: &Events,
    rows: &mut Rows<'_>,
) -> Result<()> {
    for (node, ctx) in &owner.nodes {
        let literal = match node.kind() {
            "dictionary" | "object" => *node,
            "composite_literal" if field(*node, "type").is_some_and(|n| n.kind() == "map_type") => {
                field(*node, "body").unwrap_or(*node)
            }
            _ => continue,
        };
        let children: Vec<_> = named(literal).collect();
        if children.iter().any(|n| {
            n.start_position().row == literal.start_position().row
                || !matches!(
                    n.kind(),
                    "pair" | "keyed_element" | "shorthand_property_identifier"
                )
                || field(*n, "key").is_some_and(|k| k.kind() == "computed_property_name")
        }) {
            continue;
        }
        let mut same_line = HashMap::new();
        for entry in &children {
            for row in entry.start_position().row..=entry.end_position().row {
                *same_line.entry(row).or_insert(0) += 1;
            }
        }
        let mut entries = Vec::new();
        for entry in children {
            let shorthand = entry.kind() == "shorthand_property_identifier";
            let (mut key, value) = if shorthand {
                (entry, entry)
            } else if let (Some(key), Some(value)) = (field(entry, "key"), field(entry, "value")) {
                (key, value)
            } else {
                continue;
            };
            let row = key.start_position().row;
            if row == literal.end_position().row
                || value.end_position().row != row
                || entry.end_position().row != row
                || same_line[&row] != 1
            {
                continue;
            }
            if key.kind() == "literal_element" {
                let Some(child) = named(key).next() else {
                    continue;
                };
                key = child;
            }
            let key = if shorthand || js(p.language) && key.kind() == "property_identifier" {
                Key::string(p.node_text(&key).to_owned(), p.language)
            } else {
                expression_key(p, key, ctx.scope, scopes)
            };
            if let Some(key) = key.value {
                entries.push((key, row + 1));
            }
        }
        if entries.is_empty() {
            continue;
        }
        let target = initializer_target(*node);
        let name = target.map(|(n, _)| p.node_text(&n));
        let mut later = Vec::new();
        if let Some((lhs, statement)) = target {
            let at = statement.end_byte();
            let old = Site::new(lhs, ctx, at, 1);
            let name = p.node_text(&lhs);
            let base = Access {
                name: name.to_owned(),
                parts: Vec::new(),
                binding: resolve(scopes, ctx.scope, name, at),
            };
            if let Some(reads) = events.reads.get(&base) {
                for read in reads {
                    if read.site.time > old.time
                        && read
                            .key
                            .value
                            .as_ref()
                            .is_some_and(|name| entries.iter().any(|e| &e.0 == name))
                        && events.feasible(&old, &read.site, &read.path)
                    {
                        later.extend(read.key.value.iter().cloned());
                    }
                }
            }
        }
        later.sort();
        later.dedup();
        rows.emit("lit", json!({
            "line": line(*node), "name": name,
            "entries": entries.into_iter().map(|(key, line)| json!({"key":key,"line":line})).collect::<Vec<_>>(),
            "later_reads": later,
        }))?;
    }
    Ok(())
}

fn lang_name(lang: Language) -> &'static str {
    match lang {
        Language::Python => "python",
        Language::JavaScript => "javascript",
        Language::TypeScript => "typescript",
        Language::Tsx => "tsx",
        Language::Go => "go",
        _ => unreachable!("unsupported language"),
    }
}

fn language(path: &Path) -> Option<Language> {
    match path.extension()?.to_str()? {
        "py" => Some(Language::Python),
        "js" | "jsx" | "mjs" | "cjs" => Some(Language::JavaScript),
        "ts" => Some(Language::TypeScript),
        "tsx" => Some(Language::Tsx),
        "go" => Some(Language::Go),
        _ => None,
    }
}

// Equivalent segment checks for the copied sizing probe TESTPATH regex.
// (^|/)(tests?|__tests__|testing|e2e|fixtures|__mocks__|mocks?|testdata|node_modules|vendor|dist|build|migrations|benchmarks?|examples?|docs?)/|(^|/)(test_[^/]*\.py|[^/]*_test\.(py|go)|conftest\.py|[^/]*\.(test|spec)\.[jt]sx?|[^/]*\.d\.ts)$
fn test_directory(name: &str) -> bool {
    matches!(
        name,
        "test"
            | "tests"
            | "__tests__"
            | "testing"
            | "e2e"
            | "fixtures"
            | "__mocks__"
            | "mock"
            | "mocks"
            | "testdata"
            | "node_modules"
            | "vendor"
            | "dist"
            | "build"
            | "migrations"
            | "benchmark"
            | "benchmarks"
            | "example"
            | "examples"
            | "doc"
            | "docs"
    )
}

fn test_file(name: &str) -> bool {
    name.starts_with("test_") && name.ends_with(".py")
        || name.ends_with("_test.py")
        || name.ends_with("_test.go")
        || name == "conftest.py"
        || name.ends_with(".d.ts")
        || name.rsplit_once('.').is_some_and(|(stem, ext)| {
            matches!(ext, "js" | "jsx" | "ts" | "tsx")
                && (stem.ends_with(".test") || stem.ends_with(".spec"))
        })
}

fn source_files(
    root: &Path,
    directory: &Path,
    files: &mut BTreeMap<String, Language>,
) -> Result<()> {
    for entry in
        fs::read_dir(directory).with_context(|| format!("reading {}", directory.display()))?
    {
        let entry = entry?;
        let ty = entry.file_type()?;
        let name = entry.file_name();
        let name = name.to_str().context("source tree has a non-UTF-8 path")?;
        if ty.is_dir() && name != ".git" && !test_directory(name) {
            source_files(root, &entry.path(), files)?;
        } else if ty.is_file() && !test_file(name) {
            let full = entry.path();
            let Some(lang) = language(&full) else {
                continue;
            };
            let relative = full
                .strip_prefix(root)?
                .to_str()
                .context("invalid relative path")?;
            files.insert(relative.to_owned(), lang);
        }
    }
    Ok(())
}

fn census(root: &Path, corpus: &str, out: &Path) -> Result<()> {
    let mut files = BTreeMap::new();
    source_files(root, root, &mut files)?;
    files.retain(|_, lang| match corpus {
        "P1" | "P2" => *lang == Language::Python,
        "X" | "T" => js(*lang),
        "G1" | "G2" => *lang == Language::Go,
        _ => true,
    });
    ensure!(
        !files.is_empty(),
        "no supported files found under {}",
        root.display()
    );
    let mut output = BufWriter::new(File::create(out)?);
    let mut recovery = 0;
    let mut unreadable = 0;
    for (file, lang) in &files {
        let source = match fs::read_to_string(root.join(file)) {
            Ok(source) => source,
            Err(error) if error.kind() == std::io::ErrorKind::InvalidData => {
                eprintln!("{file}: non-UTF-8 source: {error}");
                unreadable += 1;
                continue;
            }
            Err(error) => return Err(error).with_context(|| format!("reading {file}")),
        };
        let parsed =
            ParsedFile::parse(file, &source, *lang).with_context(|| format!("parsing {file}"))?;
        if parsed.parse_error_count != 0 {
            recovery += 1;
            eprintln!("{file}: {} parse recovery nodes", parsed.parse_error_count);
        }
        let mut inventory = Inventory::new(&parsed);
        let module = parsed.tree.root_node();
        let (owner, scope) = inventory.owner(module, None, None);
        inventory.walk(
            module,
            owner,
            ContextAt {
                scope,
                route: Route::default(),
            },
        );
        for owner in &inventory.owners {
            let mut rows = Rows {
                writer: &mut output,
                corpus,
                lang: lang_name(*lang),
                file,
                owner: &owner.label,
            };
            let (events, n_sub, go_delete) =
                index_events(&parsed, &inventory.scopes, owner, &mut rows)?;
            if n_sub > 0 {
                rows.emit("owner", json!({
                    "line": line(owner.node), "n_sub": n_sub, "has_nested_callable": owner.nested,
                    "n_go_delete": go_delete,
                }))?;
            }
            emit_pairs(&parsed, owner, &inventory.scopes, &events, &mut rows)?;
            emit_whole(&events, &mut rows)?;
            emit_literals(&parsed, owner, &inventory.scopes, &events, &mut rows)?;
        }
    }
    output.flush()?;
    eprintln!(
        "{corpus}: {} selected files, {recovery} parsed with recovery, {unreadable} non-UTF-8 skipped",
        files.len()
    );
    Ok(())
}

fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    ensure!(
        args.len() == 3,
        "usage: subscript_census <repo-root> <corpus-name> <out.jsonl>"
    );
    prism::api::with_build_pool(|| census(Path::new(&args[0]), &args[1], Path::new(&args[2])))
}
