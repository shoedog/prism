//! C++ additions to rules U and D (#630 slice 6c), consumed by `c_prune.rs`.
//! Design of record: `docs/superpowers/plans/2026-11-11-630-s6-cpp-ud/DESIGN.md` §5.1.
//!
//! C++ owners (`function_definition`, and a `template_declaration` holding one) run the C rules
//! plus (a `.h` file is parsed as C, #529, and stays off: the C grammar parses C++ shapes such
//! as `T& r = (x);` without an error, so the escapes below cannot see them):
//! - the scalar gate (E2): the declared type is a keyword `primitive_type` or
//!   `sized_type_specifier`, any pointer declarator, or `auto` initialised by a literal;
//!   references, enums (they can carry every overloaded operator), class and typedef types,
//!   `decltype`, `auto` from anything else and `char8_t`/`char16_t`/`char32_t` (`char8_t` is a
//!   keyword only from C++20; all three are refused) fail it;
//! - the C++ escape set (E3), owner-wide by name: every lvalue root (`escape_roots`) of an
//!   address-of operand and of the range of a range-for, a bare argument of any call, a
//!   constructor or brace initializer, anything inside the initializer of a reference or
//!   structured binding, every name inside a lambda with a non-empty capture list
//!   (captures are read when the closure is created), a block-scope using-declaration's names,
//!   and the bare initializer of a declaration whose type fails the gate;
//! - the name introducers the resolver cannot see (`cpp_prune_names.rs`, fold 5): enumerators of
//!   any enum, the declarators of a declaration with a non-scalar base type (pointer declarators
//!   too: `ga * x;` can be an expression read as a declaration), every `type_identifier`, template
//!   syntax everywhere (every name under a `template_argument_list` and the name of every
//!   `template_type`, `template_function` and `template_method`, so `a < x > y;`, `a < x > (0)` and
//!   `ga<x> *y;` all escape `x`), and the names inside `offsetof` and non-keyword `alignof` forms;
//! - the left side of `=` must itself be a bindable scalar local (closed-world, settled after
//!   the walk to a fixpoint: `settle`, a worklist), else the right side's roots escape; the
//!   operand certificate (below) covers every operator;
//! - owners that are off: an owner spelling an in-repo macro named like an entry of `KEYWORDS`
//!   (`#define int IR`), a block-scope `using namespace` or `using enum` (the enumerators it
//!   brings in are not spelled), a coroutine (`co_await`, `co_yield`,
//!   `co_return` outside nested lambdas: promise construction and `await_transform` /
//!   `yield_value` can bind a parameter or an operand by reference);
//! - bindings from `catch` parameters, condition and init-statement declarations and range-for
//!   variables (never dominating writes).
//!
//! Operand certificate (fold 4, `cpp_prune_operand.rs`; 2026-11-12: X3 closed in code, supersedes
//! D4): a user-defined operator needs a class or enumeration operand, and a converting
//! constructor of a by-value class parameter can bind a scalar (`S(int&)` with
//! `S operator+(S, S)`), so an operator is built-in only when every operand is. For every binary
//! operator (shifts, comparison, logical and comma included), compound assignment, subscript and
//! `?:`, the lvalue roots of an operand escape unless EVERY OTHER operand is a built-in scalar
//! expression (a literal, a bindable scalar local, a local array of scalars or pointers, a cast
//! to a keyword scalar or pointer, `sizeof`/`alignof`, `nullptr` when no in-repo macro has that
//! name, and arithmetic or logical operators over these). Only the lowercase keyword spellings
//! `true`, `false`, `nullptr` and `alignof` count (the grammar reads `TRUE`, `FALSE` and `NULL` as
//! the same node kinds, and a program can declare them as objects or enumerators): `TRUE`,
//! `FALSE`, `NULL` and `_alignof` are never built-in, macro or not, nor literals for `auto`. A
//! local array counts only while its name is unescaped, like a scalar local (a `using` or hidden
//! declaration can make the same spelling another entity; the check is part of the worklist).
//! Calls, members, `this`, globals and unknown names are not scalar. Plain `=` keeps the fix E
//! rule above. Root finders (`lvalue_roots`, `escape_roots`, and the C rules' `chain_roots`) share
//! `prune_roots.rs`: comments, parentheses, `__extension__` and `_Generic` are peeled by one
//! helper, and an unrecognized wrapper escapes every identifier in it.
//! Dialect: every file the C++ grammar parses (`cpp cc cxx hpp hxx hh`) is compiled as C++11 or
//! later, which makes `nullptr` and `auto x = <literal>` safe; only `char8_t` depends on the
//! dialect (C++20), and `char8_t`/`char16_t`/`char32_t` are all refused. Pre-C++11 code that
//! declares an object named `nullptr` is outside the model (documented, not closed).
use super::c_prune::COwner;
use super::cpp_prune_operand::{Deferred, Need};
use super::prune_roots::{peel, wrapper_operands};
use crate::ast::ParsedFile;
use tree_sitter::Node;

/// The identifiers an expression can hand over as an lvalue: itself, through parentheses, both
/// branches of `?:`, a comma sequence, the target of an assignment, an increment, or a cast to
/// a reference type. A call such as `f(c ? x : y)` binds a reference to `x`.
pub(super) fn lvalue_roots<'a>(parsed: &ParsedFile, node: Node<'a>, out: &mut Vec<Node<'a>>) {
    let mut kids = Vec::new();
    if wrapper_operands(node, &mut kids) {
        for k in kids {
            lvalue_roots(parsed, k, out);
        }
        return;
    }
    match node.kind() {
        "identifier" => out.push(node),
        "conditional_expression" => {
            // `c ?: y` (GNU) yields the condition itself when it is true.
            let fields: &[&str] = if node.child_by_field_name("consequence").is_some() {
                &["consequence", "alternative"]
            } else {
                &["condition", "alternative"]
            };
            for f in fields {
                if let Some(c) = node.child_by_field_name(f) {
                    lvalue_roots(parsed, c, out);
                }
            }
        }
        "comma_expression" => {
            for f in ["left", "right"] {
                if let Some(c) = node.child_by_field_name(f) {
                    lvalue_roots(parsed, c, out);
                }
            }
        }
        "assignment_expression" => {
            if let Some(c) = node.child_by_field_name("left") {
                lvalue_roots(parsed, c, out);
            }
        }
        "update_expression" => {
            if let Some(c) = node.child_by_field_name("argument") {
                lvalue_roots(parsed, c, out);
            }
        }
        // Every cast but one to a keyword built-in scalar (or a pointer to one) can bind its
        // operand: `(IR)x` a typedef'd reference, `(S)x` a converting constructor, `(Unknown)x`.
        "cast_expression" => {
            if !node
                .child_by_field_name("type")
                .is_some_and(|t| scalar_cast_type(parsed, t))
            {
                if let Some(c) = node.child_by_field_name("value") {
                    lvalue_roots(parsed, c, out);
                }
            }
        }
        // A designated initializer's value.
        "initializer_pair" => {
            if let Some(c) = node.child_by_field_name("value") {
                lvalue_roots(parsed, c, out);
            }
        }
        // A shape not known to yield an rvalue may hand over an lvalue (a wrapper the finder
        // does not know): every identifier in it is a root.
        k if !NON_LVALUE.contains(&k) => identifiers_in(node, out),
        _ => {}
    }
}

/// Expression and leaf kinds whose value is never an lvalue the operand names. Their operands
/// are judged by the arms that own them (`argument_list`, the operand certificate, `addr_of`),
/// so `lvalue_roots` yields nothing for them. Any other kind is an unrecognized wrapper and
/// yields every identifier in it (fail closed).
const NON_LVALUE: &[&str] = &[
    "alignof_expression",
    "argument_list",
    "binary_expression",
    "call_expression",
    "char_literal",
    "comment",
    "compound_literal_expression",
    "concatenated_string",
    "delete_expression",
    "false",
    "field_expression",
    "field_identifier",
    "fold_expression",
    "initializer_list",
    "lambda_expression",
    "namespace_identifier",
    "new_expression",
    "null",
    "nullptr",
    "number_literal",
    "offsetof_expression",
    "parameter_pack_expansion",
    "pointer_expression",
    "primitive_type",
    "qualified_identifier",
    "raw_string_literal",
    "sizeof_expression",
    "string_literal",
    "subscript_expression",
    "template_function",
    "template_method",
    "this",
    "true",
    "type_identifier",
    "unary_expression",
    "user_defined_literal",
];

/// Every identifier an address-taking site (`&E`) or a range-for range can bind a pointer or
/// a reference to: the lvalue roots of `lvalue_roots` (any cast included), plus `.`/`->` and
/// subscript chains down to their root; a shape not listed escapes every identifier in it.
pub(super) fn escape_roots<'a>(node: Node<'a>, out: &mut Vec<Node<'a>>) {
    let mut kids = Vec::new();
    if wrapper_operands(node, &mut kids) {
        for k in kids {
            escape_roots(k, out);
        }
        return;
    }
    match node.kind() {
        "identifier" => out.push(node),
        "conditional_expression" => {
            let fields: &[&str] = if node.child_by_field_name("consequence").is_some() {
                &["consequence", "alternative"]
            } else {
                &["condition", "alternative"]
            };
            for f in fields {
                if let Some(c) = node.child_by_field_name(f) {
                    escape_roots(c, out);
                }
            }
        }
        "comma_expression" => {
            for f in ["left", "right"] {
                if let Some(c) = node.child_by_field_name(f) {
                    escape_roots(c, out);
                }
            }
        }
        "assignment_expression" => {
            if let Some(c) = node.child_by_field_name("left") {
                escape_roots(c, out);
            }
        }
        "update_expression" | "field_expression" | "subscript_expression" => {
            if let Some(c) = node.child_by_field_name("argument") {
                escape_roots(c, out);
            }
        }
        "cast_expression" => {
            if let Some(c) = node.child_by_field_name("value") {
                escape_roots(c, out);
            }
        }
        _ => identifiers_in(node, out),
    }
}

fn is_reference_like(declarator: Node<'_>) -> bool {
    let mut node = declarator;
    loop {
        match node.kind() {
            "reference_declarator" | "structured_binding_declarator" => return true,
            "pointer_declarator"
            | "parenthesized_declarator"
            | "array_declarator"
            | "function_declarator"
            | "attributed_declarator" => match node
                .child_by_field_name("declarator")
                .or_else(|| node.named_child(0))
            {
                Some(n) => node = n,
                None => return false,
            },
            _ => return false,
        }
    }
}

/// A literal `auto` can be initialised by to a scalar: a number, character or string literal
/// (user-defined suffixes are another node kind), or the lowercase keyword `true`/`false`. The
/// grammar also makes `TRUE` and `FALSE` `true`/`false` nodes, so the text is checked.
fn literal_node(parsed: &ParsedFile, node: Node<'_>) -> bool {
    match node.kind() {
        "number_literal" | "char_literal" | "string_literal" | "raw_string_literal" => true,
        "true" | "false" => keyword_spelling(parsed, node),
        _ => false,
    }
}

/// The node's text is exactly its lowercase keyword spelling (`true`, `false`, `nullptr`): the
/// grammar reads the identifiers `TRUE`, `FALSE` and `NULL` as the same node kinds.
pub(super) fn keyword_spelling(parsed: &ParsedFile, node: Node<'_>) -> bool {
    matches!(
        (node.kind(), parsed.node_text(&node)),
        ("true", "true") | ("false", "false") | ("nullptr" | "null", "nullptr")
    )
}

/// The C++ built-in type keywords. The grammar also calls `size_t`, `ptrdiff_t`, `int8_t`,
/// `nullptr_t` and the like `primitive_type`, but a program (or a template parameter, or a block
/// typedef) can give any of those spellings a class, so only a keyword proves a scalar.
const SCALAR_KEYWORDS: &[&str] = &[
    "bool", "char", "wchar_t", "short", "int", "long", "float", "double", "void", "signed",
    "unsigned",
];

/// The C++ keywords the proofs read (type, storage, qualifier, cast, operator and expression
/// keywords, the coroutine words) plus the statement keywords and access specifiers, which no
/// proof reads and which only cost recall. It is NOT every C++ keyword: `noexcept`, `requires`,
/// `concept`, `consteval`, `constinit`, `static_assert`, `typeid`, `alignas`, `export`, `asm` and
/// the alternative tokens are absent. `char8_t`, `char16_t` and `char32_t` are listed but are
/// never scalar proofs. An in-repo macro of one of these names rewrites the spelling the proofs
/// read, so an owner spelling it is off. The census is repo-wide, so the gate is by spelling
/// across the repository: a C-only header's `#define bool int` turns off every C++ owner that
/// spells `bool`.
const KEYWORDS: &[&str] = &[
    "bool",
    "char",
    "char8_t",
    "char16_t",
    "char32_t",
    "wchar_t",
    "short",
    "int",
    "long",
    "float",
    "double",
    "void",
    "signed",
    "unsigned",
    "auto",
    "const",
    "volatile",
    "static",
    "extern",
    "register",
    "thread_local",
    "mutable",
    "struct",
    "class",
    "union",
    "enum",
    "typename",
    "decltype",
    "using",
    "namespace",
    "operator",
    "new",
    "delete",
    "this",
    "static_cast",
    "dynamic_cast",
    "const_cast",
    "reinterpret_cast",
    "sizeof",
    "alignof",
    "template",
    "typedef",
    "true",
    "false",
    "nullptr",
    "constexpr",
    "explicit",
    "inline",
    "virtual",
    "friend",
    "public",
    "private",
    "protected",
    "if",
    "else",
    "for",
    "while",
    "do",
    "switch",
    "case",
    "default",
    "return",
    "break",
    "continue",
    "goto",
    "try",
    "catch",
    "throw",
    "co_await",
    "co_yield",
    "co_return",
];

pub(super) fn is_keyword(name: &str) -> bool {
    KEYWORDS.contains(&name)
}

/// A `primitive_type` spelled as a keyword, or a `sized_type_specifier` built only from keywords
/// (and `const`).
pub(super) fn keyword_scalar(parsed: &ParsedFile, ty: Node<'_>) -> bool {
    match ty.kind() {
        "primitive_type" => SCALAR_KEYWORDS.contains(&parsed.node_text(&ty)),
        "sized_type_specifier" => parsed
            .node_text(&ty)
            .split_whitespace()
            .all(|t| t == "const" || SCALAR_KEYWORDS.contains(&t)),
        _ => false,
    }
}

/// A cast target that cannot bind its operand: a keyword built-in scalar, or a pointer to one
/// (a `type_descriptor` with no reference, function or array declarator).
pub(super) fn scalar_cast_type(parsed: &ParsedFile, td: Node<'_>) -> bool {
    if !td
        .child_by_field_name("type")
        .is_some_and(|t| keyword_scalar(parsed, t))
    {
        return false;
    }
    let mut d = td.child_by_field_name("declarator");
    while let Some(n) = d {
        if n.kind() != "abstract_pointer_declarator" {
            return false;
        }
        d = n.child_by_field_name("declarator");
    }
    true
}

/// E2, C++ half: the type of the declaration passes the scalar gate. `decl` is the
/// `declaration` or `parameter_declaration`, `top` the declarator chain under it (or under its
/// `init_declarator`).
pub(super) fn scalar_gate(
    parsed: &ParsedFile,
    decl: Node<'_>,
    top: Node<'_>,
    pointer: bool,
) -> bool {
    if pointer {
        return true;
    }
    let Some(ty) = decl.child_by_field_name("type") else {
        return false;
    };
    match ty.kind() {
        "primitive_type" | "sized_type_specifier" => keyword_scalar(parsed, ty),
        "placeholder_type_specifier" => {
            if parsed.node_text(&ty).trim() != "auto" {
                return false;
            }
            let init = match top.parent() {
                Some(p) if p.kind() == "init_declarator" => p.child_by_field_name("value"),
                Some(p) if p.kind() == "declaration" => p.child_by_field_name("value"),
                _ => None,
            };
            init.is_some_and(|v| literal_node(parsed, v))
        }
        _ => false,
    }
}

fn identifiers_in<'a>(node: Node<'a>, out: &mut Vec<Node<'a>>) {
    if node.kind() == "identifier" {
        out.push(node);
    }
    let mut cursor = node.walk();
    for c in node.named_children(&mut cursor) {
        identifiers_in(c, out);
    }
}

/// Every node of one of `kinds` under `node` (itself included).
pub(super) fn names_in<'a>(node: Node<'a>, kinds: &[&str], out: &mut Vec<Node<'a>>) {
    if kinds.contains(&node.kind()) {
        out.push(node);
    }
    let mut cursor = node.walk();
    for c in node.named_children(&mut cursor) {
        names_in(c, kinds, out);
    }
}

/// An identifier that is (the last component of) a qualified name, `N::x`, `::x`, `S::x`,
/// `T::x`: it names an entity of another scope, never an owner local.
pub(super) fn is_qualified_name(occ: Node<'_>) -> bool {
    occ.parent()
        .is_some_and(|p| matches!(p.kind(), "qualified_identifier" | "dependent_name"))
}

/// A macro whose transitive body spells a coroutine keyword: the owner using it is a coroutine
/// the parser cannot see.
pub(super) fn coroutine_macro(free: &std::collections::BTreeSet<String>) -> bool {
    ["co_await", "co_yield", "co_return"]
        .iter()
        .any(|k| free.contains(*k))
}

/// Direct children of a block that introduce no name the resolver misses (a declaration is read
/// by `scope_decls`, `using` by the `using_decl` escape, a typedef or alias names a type, and
/// the rest are statements). Anything else (an anonymous union, an enum or class specifier, a
/// stray type name) escapes every name in it: the resolver is closed-world.
const MODELLED_BLOCK_CHILD: &[&str] = &[
    "alias_declaration",
    "attributed_statement",
    "break_statement",
    "case_statement",
    "comment",
    "compound_statement",
    "concept_definition",
    "continue_statement",
    "co_return_statement",
    "co_yield_statement",
    "declaration",
    "do_statement",
    "expression_statement",
    "for_range_loop",
    "for_statement",
    "function_definition",
    "goto_statement",
    "if_statement",
    "labeled_statement",
    "namespace_alias_definition",
    "preproc_call",
    "preproc_def",
    "preproc_function_def",
    "preproc_if",
    "preproc_ifdef",
    "preproc_include",
    "return_statement",
    "seh_leave_statement",
    "seh_try_statement",
    "static_assert_declaration",
    "switch_statement",
    "throw_statement",
    "try_statement",
    "type_definition",
    "using_declaration",
    "while_statement",
];

/// An anonymous `union`/`struct`/`class` with a body and no declarator: its members are names
/// of the enclosing scope (and an anonymous union's members share storage).
fn anonymous_aggregate(node: Node<'_>) -> bool {
    matches!(
        node.kind(),
        "union_specifier" | "struct_specifier" | "class_specifier"
    ) && node.child_by_field_name("body").is_some()
        && node.child_by_field_name("name").is_none()
        && !node.parent().is_some_and(|p| {
            matches!(
                p.kind(),
                "declaration" | "type_definition" | "field_declaration" | "parameter_declaration"
            ) && p.child_by_field_name("declarator").is_some()
        })
}

/// The initializer expression of one declarator of a declaration.
fn initializer_of<'a>(decl: Node<'a>, declarator: Node<'a>) -> Option<Node<'a>> {
    if declarator.kind() == "init_declarator" {
        declarator.child_by_field_name("value")
    } else {
        decl.child_by_field_name("value")
    }
}

/// Record the C++ escapes one node causes (E3). Called for every node of the owner.
pub(super) fn note_node(parsed: &ParsedFile, node: Node<'_>, out: &mut COwner) {
    super::cpp_prune_names::note(parsed, node, out);
    let mut roots = Vec::new();
    let esc = |out: &mut COwner, kind: &'static str, nodes: &[Node<'_>]| {
        for n in nodes {
            out.escape(kind, parsed.node_text(n));
        }
    };
    // Fail closed on block-scope forms the resolver does not model (fix F).
    if node
        .parent()
        .is_some_and(|p| matches!(p.kind(), "compound_statement" | "case_statement"))
        && !MODELLED_BLOCK_CHILD.contains(&node.kind())
    {
        collect_names(node, &mut roots);
        esc(out, "unmodelled_block_form", &roots);
        roots.clear();
    }
    if anonymous_aggregate(node) {
        names_in(node, &["field_identifier"], &mut roots);
        esc(out, "anonymous_aggregate", &roots);
        roots.clear();
    }
    match node.kind() {
        // A bare argument binds a reference parameter or a constructor parameter.
        "argument_list" | "initializer_list" => {
            let mut cursor = node.walk();
            for c in node.named_children(&mut cursor) {
                lvalue_roots(parsed, c, &mut roots);
            }
            let kind = match node.parent().map(|p| p.kind()) {
                Some("field_initializer") => "mem_init",
                Some("new_expression") => "new_arg",
                _ if node.kind() == "initializer_list" => "brace_init",
                _ => "call_arg",
            };
            esc(out, kind, &roots);
        }
        // Every operator with operands: the operand certificate (`cpp_prune_operand`).
        "binary_expression" | "comma_expression" | "subscript_expression" => {
            super::cpp_prune_operand::certify(parsed, node, out);
        }
        "declaration" => {
            let mut cursor = node.walk();
            let declarators: Vec<Node<'_>> = node
                .children_by_field_name("declarator", &mut cursor)
                .collect();
            let mut ids = Vec::new();
            for d in &declarators {
                let top = if d.kind() == "init_declarator" {
                    match d.child_by_field_name("declarator") {
                        Some(t) => t,
                        None => continue,
                    }
                } else {
                    *d
                };
                let init = initializer_of(node, *d);
                if is_reference_like(top) {
                    // Anything in the initializer of a reference or structured binding, and
                    // the direct-initialisation arguments the grammar reads as parameter types
                    // (`int& r(x);`, `int*& rp(p);`, `auto& r(x);`).
                    if let Some(i) = init {
                        names_in(i, &["identifier", "type_identifier"], &mut ids);
                    }
                    names_in(top, &["identifier", "type_identifier"], &mut ids);
                    esc(out, "ref_init", &ids);
                    ids.clear();
                    continue;
                }
                // `Foo f(a);` parses as a function declarator whose parameter type is the
                // name `a`: a direct-initialisation argument.
                if top.kind() == "function_declarator" {
                    let mut ids = Vec::new();
                    if let Some(params) = top.child_by_field_name("parameters") {
                        identifiers_in_types(params, &mut ids);
                    }
                    for n in ids {
                        out.escape("direct_init", parsed.node_text(&n));
                    }
                    continue;
                }
                let pointer = {
                    let mut n = top;
                    let mut p = false;
                    loop {
                        match n.kind() {
                            "pointer_declarator" => {
                                p = true;
                                match n.child_by_field_name("declarator") {
                                    Some(x) => n = x,
                                    None => break,
                                }
                            }
                            "parenthesized_declarator" => match n.named_child(0) {
                                Some(x) => n = x,
                                None => break,
                            },
                            _ => break,
                        }
                    }
                    p
                };
                // The bare initializer of a declaration whose type fails the gate may be
                // taken by a converting constructor by reference.
                if !scalar_gate(parsed, node, top, pointer) {
                    if let Some(i) = init {
                        lvalue_roots(parsed, i, &mut roots);
                        esc(out, "ctor_conv", &roots);
                        roots.clear();
                    }
                    out.scalar_fail += 1;
                }
            }
        }
        // `=` and compound assignment: a left side other than a scalar local may be a class
        // whose `operator=(int&)` or converting constructor keeps the right operand, and an
        // lvalue the producer does not see written (`(c ? x : y) += 1`).
        "assignment_expression" => {
            if let (Some(left), Some(right)) = (
                node.child_by_field_name("left"),
                node.child_by_field_name("right"),
            ) {
                let bare = peel(left);
                if bare.kind() == "identifier" {
                    // Whether the left side is a bindable scalar local is known only once
                    // every escape of the owner is (`settle`).
                    escape_roots(right, &mut roots);
                    out.deferred.push(Deferred {
                        needs: vec![Need::Scalar(bare.start_byte(), bare.end_byte())],
                        kind: "assign_rhs",
                        roots: roots
                            .iter()
                            .map(|n| parsed.node_text(n).to_owned())
                            .collect(),
                    });
                } else {
                    lvalue_roots(parsed, left, &mut roots);
                    esc(out, "assign_lhs", &roots);
                    roots.clear();
                    escape_roots(right, &mut roots);
                    esc(out, "assign_rhs", &roots);
                }
            }
            // A compound assignment is an operator too: each side against the other.
            super::cpp_prune_operand::certify(parsed, node, out);
        }
        // `c ? a : b`: a branch that is not a built-in scalar expression can convert the
        // other to a class through `S(int&)`: the operand certificate over the two branches.
        "conditional_expression" => super::cpp_prune_operand::certify(parsed, node, out),
        // A cast to anything but a keyword built-in scalar (or a pointer to one), wherever it
        // sits: `(Ret)x;`, `f((IR)x)`, `y = (S)x + 1`.
        "cast_expression" => {
            lvalue_roots(parsed, node, &mut roots);
            esc(out, "cast", &roots);
        }
        // Every operand of a fold expression is an lvalue when the pack is empty
        // (`(ts, ..., x)` is `x`).
        "fold_expression" => {
            identifiers_in(node, &mut roots);
            esc(out, "fold", &roots);
        }
        // The range is bound through an implicit reference, and ADL `begin(E&)` / `end(E&)` can
        // keep it, whatever the loop variable's own declarator.
        "for_range_loop" => {
            if let Some(r) = node.child_by_field_name("right") {
                escape_roots(r, &mut roots);
                esc(out, "range_for", &roots);
            }
        }
        // `&E`: every lvalue root of E (the C rule's `chain_roots` sees identifiers, `.`,
        // subscripts and parentheses only).
        "pointer_expression" => {
            let amp = node
                .child_by_field_name("operator")
                .is_some_and(|op| parsed.node_text(&op) == "&");
            if amp {
                if let Some(arg) = node.child_by_field_name("argument") {
                    escape_roots(arg, &mut roots);
                    esc(out, "addr_of", &roots);
                }
            }
        }
        // Captures are initialised when the closure is created, on the creating statement's
        // line, and a by-reference capture or a stored closure keeps the variable: every name
        // in a lambda with any capture escapes. `[]` reads no local.
        "lambda_expression" => {
            let non_empty = node.child_by_field_name("captures").is_some_and(|caps| {
                let mut cursor = caps.walk();
                let any = caps
                    .named_children(&mut cursor)
                    .any(|c| c.kind() != "comment");
                any
            });
            if non_empty {
                identifiers_in(node, &mut roots);
                esc(out, "lambda_capture", &roots);
            }
        }
        // A block-scope using-declaration names a variable of another scope (`using N::x;`
        // hides an enclosing local `x` and writes N::x).
        "using_declaration" if !is_using_directive(node) => {
            collect_names(node, &mut roots);
            esc(out, "using_decl", &roots);
        }
        _ => {}
    }
}

/// `using namespace N;` and `using enum E;`: either brings names into scope without spelling
/// them (the enumerators of `E`), so the owner is off.
fn is_using_directive(node: Node<'_>) -> bool {
    let mut cursor = node.walk();
    let found = node
        .children(&mut cursor)
        .any(|c| matches!(c.kind(), "namespace" | "enum"));
    found
}

pub(super) fn collect_names<'a>(node: Node<'a>, out: &mut Vec<Node<'a>>) {
    if matches!(
        node.kind(),
        "identifier" | "namespace_identifier" | "type_identifier" | "field_identifier"
    ) {
        out.push(node);
    }
    let mut cursor = node.walk();
    for c in node.named_children(&mut cursor) {
        collect_names(c, out);
    }
}

/// Why a C++ owner is off, when it is (`None` otherwise): a block-scope using-directive makes
/// any name rebind; a coroutine (outside nested lambdas, which are their own callables) has
/// implicit promise and awaiter calls that can bind parameters and operands by reference.
pub(super) fn off_reason(node: Node<'_>, owner: Node<'_>) -> Option<super::c_prune::Off> {
    use super::c_prune::Off;
    match node.kind() {
        "using_declaration" if is_using_directive(node) => Some(Off::UsingDirective),
        "co_await_expression" | "co_yield_statement" | "co_return_statement" => {
            let mut p = node.parent();
            while let Some(n) = p {
                if n.id() == owner.id() {
                    break;
                }
                if n.kind() == "lambda_expression" {
                    return None;
                }
                p = n.parent();
            }
            Some(Off::Coroutine)
        }
        _ => None,
    }
}

/// Names parsed as types inside a function-declarator parameter list (`Foo f(a);`).
fn identifiers_in_types<'a>(params: Node<'a>, out: &mut Vec<Node<'a>>) {
    let mut cursor = params.walk();
    for p in params.named_children(&mut cursor) {
        if p.kind() == "parameter_declaration" && p.child_by_field_name("declarator").is_none() {
            if let Some(t) = p.child_by_field_name("type") {
                if t.kind() == "type_identifier" {
                    out.push(t);
                }
            }
        }
    }
}

/// Declarations a statement header makes visible (`if (int v = f(); v)`, `while (T* p = ..)`,
/// `switch (int s = ..; s)`): the `condition_clause` of a `condition` field.
pub(super) fn condition_decls<'a>(
    stmt: Node<'a>,
    found: &mut Vec<Node<'a>>,
    decl_idents: &dyn Fn(Node<'a>, &mut Vec<Node<'a>>),
) {
    let Some(cond) = stmt.child_by_field_name("condition") else {
        return;
    };
    if cond.kind() != "condition_clause" {
        return;
    }
    if let Some(init) = cond.child_by_field_name("initializer") {
        let mut cursor = init.walk();
        for d in init.named_children(&mut cursor) {
            if d.kind() == "declaration" {
                decl_idents(d, found);
            }
        }
    }
    if let Some(v) = cond.child_by_field_name("value") {
        if v.kind() == "declaration" {
            decl_idents(v, found);
        }
    }
}
