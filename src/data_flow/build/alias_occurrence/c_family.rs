//! C and C++ occurrence rows `C-01..C-10`, `X-READ`, `X-DECL` (alias-twin lane, slice 2b).
//!
//! Binding writes (no target twin): `C-01`/`C-02` a name written by `=`, a compound operator or `++`/`--`
//! (C always; C++ only with positive evidence: a pointer declarator, or a declaration prefix made only of
//! builtin type KEYWORDS, visible at the occurrence; a `&` in the prefix is a reference and a storage write;
//! typedef/`using` names, `size_t`, `auto`, class and dependent types, undeclared names and visible
//! declarations that disagree are `Unsupported`); `C-03` a declaration or parameter name (initialising a
//! reference does not write the referent); `C-04` declarator syntax (`*p`, `a[N]`, `&r`, `(*p)`) with
//! declarator-shaped text. Storage writes keep the twin: `C-05` `*u = ..`, `C-06` `u->f`/`(*u).f`, `C-07`
//! `u.f` through a C++ reference, `C-08` `u[i]` through a pointer or an array parameter, `C-10` a name inside
//! a `++`/parenthesis/cast wrapper whose value is dereferenced as a target (`*p++ = v`). Everything else is
//! `Unsupported` and keeps the twin. `X-READ` (the extraction records a read as an lvalue span, such as
//! `int r = bar(u)`) stays `Unsupported`. A C++ declaration inside a `for` initialiser is `Unsupported` (the
//! oracle's prefix starts at the `for`; kept, more conservative than a keyword-only prefix). A `.h` header
//! (parsed as C) takes the C++ rules.
use super::shared::{field_in_parent, is_plain_ident, span_node, walk_owner};
use super::{Classified, Disposition};
use crate::ast::{ParsedFile, PathSpan};
use crate::languages::Language;
use tree_sitter::Node;

/// C++ evidence rules apply to a C++ file and to a `.h` header: prism parses `.h` as C, but a header
/// in a C++ tree holds C++ (references, classes, members), so it gets the stricter rules (more
/// conservative than the table, which classifies the corpus language).
pub(super) fn cpp_rules(parsed: &ParsedFile) -> bool {
    parsed.language == Language::Cpp || parsed.path.ends_with(".h")
}

const fn done(disposition: Disposition, rule: &'static str) -> Classified {
    Classified { disposition, rule }
}

/// C++ builtin type KEYWORDS: the only spelling that is positive evidence of an arithmetic value.
const BUILTIN: [&str; 21] = [
    "const",
    "volatile",
    "static",
    "register",
    "unsigned",
    "signed",
    "short",
    "long",
    "int",
    "char",
    "bool",
    "float",
    "double",
    "wchar_t",
    "char16_t",
    "char32_t",
    "constexpr",
    "extern",
    "thread_local",
    "mutable",
    "inline",
];

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Kind {
    Ref,
    Pointer,
    Builtin,
    Other,
}

/// A declaration of the alias name visible to the occurrence.
pub(super) struct Site {
    pub prefix: String,
    pub text: String,
    pub param: bool,
}

fn ends_with_pointer(prefix: &str) -> bool {
    let mut rest = prefix.trim_end();
    loop {
        let before = rest;
        for word in ["const", "volatile", "restrict", "__restrict"] {
            if let Some(r) = rest.strip_suffix(word) {
                rest = r.trim_end();
            }
        }
        if rest == before {
            break;
        }
    }
    rest.ends_with('*')
}

fn site_kind(site: &Site) -> Kind {
    if site.prefix.contains('&') {
        Kind::Ref
    } else if ends_with_pointer(&site.prefix) {
        Kind::Pointer
    } else {
        let mut words = site.prefix.split_whitespace().peekable();
        if words.peek().is_some() && words.all(|w| BUILTIN.contains(&w)) {
            Kind::Builtin
        } else {
            Kind::Other
        }
    }
}

/// The value kind of the alias: every visible declaration must agree.
fn value_kind(sites: &[Site]) -> Kind {
    let mut kinds = sites.iter().map(site_kind);
    match kinds.next() {
        Some(first) if kinds.all(|k| k == first) => first,
        _ => Kind::Other,
    }
}

const SCOPES: [&str; 12] = [
    "compound_statement",
    "for_statement",
    "for_range_loop",
    "function_definition",
    "if_statement",
    "while_statement",
    "do_statement",
    "switch_statement",
    "translation_unit",
    "lambda_expression",
    "case_statement",
    "namespace_definition",
];

/// Declarations of `name` inside `owner` whose scope holds `byte` and that precede it.
pub(super) fn visible_sites(
    parsed: &ParsedFile,
    owner: &Node<'_>,
    name: &str,
    byte: usize,
) -> Vec<Site> {
    let mut out = Vec::new();
    walk_owner(owner, |id| {
        if id.kind() != "identifier" || parsed.node_text(&id) != name || id.start_byte() > byte {
            return;
        }
        let mut cur = id;
        let root = loop {
            let Some(p) = cur.parent() else { return };
            let field = field_in_parent(cur).unwrap_or("");
            match p.kind() {
                "pointer_declarator" | "array_declarator" | "init_declarator" => {
                    if field != "declarator" {
                        return;
                    }
                }
                "reference_declarator" | "parenthesized_declarator" => {}
                "declaration" | "parameter_declaration" | "for_range_loop" => {
                    if field != "declarator" {
                        return;
                    }
                    break p;
                }
                _ => return,
            }
            cur = p;
        };
        let mut scope = root.parent();
        while let Some(s) = scope {
            if SCOPES.contains(&s.kind()) {
                break;
            }
            scope = s.parent();
        }
        let scope_end = scope.map_or(usize::MAX, |s| s.end_byte());
        if byte >= scope_end {
            return;
        }
        // A declaration that is a `for` initialiser: the prefix starts at the `for` (kept unsupported).
        let prefix_from = match root.parent() {
            Some(f) if f.kind() == "for_statement" => f.start_byte(),
            _ => root.start_byte(),
        };
        out.push(Site {
            prefix: parsed.source[prefix_from..id.start_byte()].to_string(),
            text: parsed.node_text(&root).to_string(),
            param: matches!(
                root.kind(),
                "parameter_declaration" | "optional_parameter_declaration"
            ),
        });
    });
    out
}

fn has_array(sites: &[Site], name: &str) -> bool {
    sites.iter().any(|s| {
        let mut from = 0;
        while let Some(i) = s.text[from..].find(name) {
            let at = from + i;
            let after = s.text[at + name.len()..].trim_start();
            let before_ok = at == 0
                || !s.text.as_bytes()[at - 1].is_ascii_alphanumeric()
                    && s.text.as_bytes()[at - 1] != b'_';
            if before_ok && after.starts_with('[') {
                return true;
            }
            from = at + name.len();
        }
        false
    })
}

pub(super) fn declarator_shaped(kind: &str, text: &str) -> bool {
    let word = |c: char| c.is_alphanumeric() || c == '_' || c.is_whitespace();
    match kind {
        "pointer_declarator" => text.chars().all(|c| c == '*' || word(c)),
        "reference_declarator" => text.chars().all(|c| c == '&' || word(c)),
        "parenthesized_declarator" => {
            text.starts_with('(')
                && text.ends_with(')')
                && text[1..text.len() - 1]
                    .chars()
                    .all(|c| c == '*' || c == '&' || word(c))
        }
        "array_declarator" => {
            let Some(open) = text.find('[') else {
                return false;
            };
            text[..open].chars().all(|c| c == '*' || word(c))
                && text.ends_with(']')
                && !text[open..].contains('=')
        }
        _ => false,
    }
}

/// `(*name).` at the start of a dotted path.
fn paren_deref_dot(text: &str) -> bool {
    let t = text.trim_start();
    let Some(rest) = t.strip_prefix('(') else {
        return false;
    };
    let rest = rest.trim_start();
    let stars = rest.chars().take_while(|c| *c == '*').count();
    if stars == 0 {
        return false;
    }
    let rest = rest[stars..].trim_start();
    let ident = rest
        .chars()
        .take_while(|c| c.is_alphanumeric() || *c == '_')
        .count();
    ident > 0
        && rest[ident..]
            .trim_start()
            .strip_prefix(')')
            .is_some_and(|r| r.trim_start().starts_with('.'))
}

/// Does the update wrapper chain around `node` end in a dereference used as a target
/// (`*p++ = v`, `*++p = v`, `(*p--) = v`, `*p++ += v`)?
fn dest_deref(node: Node<'_>) -> bool {
    let mut cur = node;
    let mut parent = cur.parent();
    while let Some(p) = parent {
        if matches!(
            p.kind(),
            "update_expression" | "parenthesized_expression" | "cast_expression"
        ) {
            cur = p;
            parent = cur.parent();
        } else {
            break;
        }
    }
    let Some(p) = parent else { return false };
    if p.kind() != "pointer_expression" || field_in_parent(cur) != Some("argument") {
        return false;
    }
    let mut cur = p;
    let mut parent = cur.parent();
    while let Some(q) = parent {
        if q.kind() == "parenthesized_expression" {
            cur = q;
            parent = cur.parent();
        } else {
            break;
        }
    }
    parent.is_some_and(|q| {
        (q.kind() == "assignment_expression" && field_in_parent(cur) == Some("left"))
            || q.kind() == "update_expression"
    })
}

fn is_read(node: Node<'_>, parent: Node<'_>, field: &str) -> bool {
    let k0 = parent.kind();
    let init_value = k0 == "init_declarator" && field == "value";
    let grand = parent.parent();
    if matches!(
        k0,
        "argument_list"
            | "binary_expression"
            | "cast_expression"
            | "conditional_expression"
            | "initializer_list"
            | "unary_expression"
            | "sizeof_expression"
            | "subscript_expression"
    ) || init_value
        || (matches!(k0, "pointer_expression" | "field_expression") && field == "argument")
    {
        return true;
    }
    if k0 == "parenthesized_expression"
        && grand.is_some_and(|g| {
            matches!(
                g.kind(),
                "sizeof_expression"
                    | "binary_expression"
                    | "pointer_expression"
                    | "cast_expression"
            )
        })
    {
        return true;
    }
    matches!(node.kind(), "call_expression" | "binary_expression")
        || (matches!(
            node.kind(),
            "field_expression" | "subscript_expression" | "pointer_expression"
        ) && grand.is_some_and(|g| {
            g.kind() == "init_declarator" && field_in_parent(parent) == Some("value")
        }))
}

/// Classify the lvalue occurrence `span` of a C or C++ file (`owner` supplies the declarations).
pub(super) fn classify(parsed: &ParsedFile, owner: &Node<'_>, span: &PathSpan) -> Classified {
    let unrecognised = done(Disposition::Unsupported, "X-UNRECOGNISED");
    let Some(node) = span_node(parsed, span) else {
        return unrecognised;
    };
    let Some(parent) = node.parent() else {
        return unrecognised;
    };
    let cpp = cpp_rules(parsed);
    // An occurrence whose own statement holds a parse error (an unexpanded macro is the usual
    // cause) is not classified: the node kinds around the error cannot be trusted.
    let mut statement = node;
    while let Some(p) = statement.parent() {
        if matches!(p.kind(), "compound_statement" | "translation_unit") {
            break;
        }
        statement = p;
    }
    if statement.has_error() {
        return done(Disposition::Unsupported, "X-ERROR");
    }
    let field = field_in_parent(node).unwrap_or("");
    if is_read(node, parent, field) {
        return done(Disposition::Unsupported, "X-READ");
    }
    let text = parsed.node_text(&node).trim();
    let kind = node.kind();
    let name = span.path.base.as_str();
    let sites = || visible_sites(parsed, owner, name, span.start_byte);
    let assign_left = parent.kind() == "assignment_expression" && field == "left";
    let is_name = kind == "identifier" && is_plain_ident(text);

    if is_name && parent.kind() == "update_expression" && field == "argument" && dest_deref(node) {
        // C-10: the destination is storage; the pointer increment is incidental.
        let pointer = cpp && value_kind(&sites()) != Kind::Pointer;
        return done(
            if pointer {
                Disposition::Unsupported
            } else {
                Disposition::StorageWrite
            },
            "C-10",
        );
    }
    if is_name && (assign_left || (parent.kind() == "update_expression" && field == "argument")) {
        let compound = parent.kind() == "update_expression"
            || parent
                .child_by_field_name("operator")
                .is_some_and(|op| parsed.node_text(&op).trim() != "=");
        let rule = if compound { "C-02" } else { "C-01" };
        if !cpp {
            return done(Disposition::BindingWrite, rule);
        }
        return match value_kind(&sites()) {
            Kind::Ref => done(Disposition::StorageWrite, rule),
            Kind::Pointer | Kind::Builtin => done(Disposition::BindingWrite, rule),
            Kind::Other => done(Disposition::Unsupported, rule),
        };
    }
    let decl_slot = matches!(
        (parent.kind(), field),
        ("init_declarator", "declarator")
            | ("declaration", "declarator")
            | ("parameter_declaration", "declarator")
            | ("for_range_loop", "declarator")
    ) || matches!(
        parent.kind(),
        "reference_declarator" | "parenthesized_declarator"
    );
    if is_name && decl_slot {
        return done(Disposition::BindingWrite, "C-03");
    }
    if matches!(
        kind,
        "pointer_declarator"
            | "array_declarator"
            | "reference_declarator"
            | "parenthesized_declarator"
    ) {
        let in_decl = matches!(
            (parent.kind(), field),
            ("init_declarator", "declarator")
                | ("declaration", "declarator")
                | ("parameter_declaration", "declarator")
                | ("pointer_declarator", "declarator")
                | ("array_declarator", "declarator")
        ) || matches!(
            parent.kind(),
            "parenthesized_declarator" | "reference_declarator"
        );
        let in_function = parent
            .parent()
            .is_some_and(|g| g.kind() == "function_declarator");
        if !declarator_shaped(kind, text) || in_function {
            return done(Disposition::Unsupported, "X-DECL");
        }
        return if in_decl {
            done(Disposition::BindingWrite, "C-04")
        } else {
            done(Disposition::Unsupported, "X-DECL")
        };
    }
    if matches!(
        kind,
        "function_declarator" | "type_identifier" | "parameter_declaration"
    ) || (is_name && parent.kind() == "function_declarator")
    {
        return done(Disposition::Unsupported, "X-DECL");
    }
    if !assign_left {
        return unrecognised;
    }
    let pointer_evidence = || {
        let s = sites();
        if cpp {
            value_kind(&s) == Kind::Pointer
        } else {
            s.iter().any(|x| ends_with_pointer(&x.prefix))
        }
    };
    match kind {
        "pointer_expression" if text.starts_with('*') => {
            let storage = !cpp || pointer_evidence();
            done(
                if storage {
                    Disposition::StorageWrite
                } else {
                    Disposition::Unsupported
                },
                "C-05",
            )
        }
        "field_expression" if text.contains("->") => {
            let storage = !cpp || pointer_evidence();
            done(
                if storage {
                    Disposition::StorageWrite
                } else {
                    Disposition::Unsupported
                },
                "C-06",
            )
        }
        "field_expression" => {
            if paren_deref_dot(text) {
                return done(Disposition::StorageWrite, "C-06");
            }
            let storage = cpp && value_kind(&sites()) == Kind::Ref;
            done(
                if storage {
                    Disposition::StorageWrite
                } else {
                    Disposition::Unsupported
                },
                "C-07",
            )
        }
        "subscript_expression" => {
            let s = sites();
            let pointer = if cpp {
                value_kind(&s) == Kind::Pointer
            } else {
                s.iter().any(|x| ends_with_pointer(&x.prefix))
            };
            let storage = pointer || (has_array(&s, name) && s.iter().any(|x| x.param));
            done(
                if storage {
                    Disposition::StorageWrite
                } else {
                    Disposition::Unsupported
                },
                "C-08",
            )
        }
        _ => done(Disposition::Unsupported, "C-09"),
    }
}
