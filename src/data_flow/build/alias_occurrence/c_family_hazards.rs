//! C and C++ refusals of the alias-twin policy (slice 2b): spellings whose binding-write twins stay.
//!
//! A target root named here keeps every binding-write twin of an alias of it, for one of two reasons.
//!
//! Extraction roles (refusal R1): the occurrence extraction records a token spelled like the target that
//! is not the variable: a struct field after `.`/`->` or in a designated initialiser, a goto label, an
//! enum constant or macro name declared in the file (and a macro parameter), a type name (`sizeof(T)`,
//! casts, template arguments), a namespace or a `ns::f`/`Class::member` segment. A twin of the target
//! currently, by accident, kills edges that run through such a token.
//!
//! Writes the DFG does not model (the twin's kill used to hide the resulting wrong `exact`): the
//! address taken in any form (`&o`, `&o.f`, `&o[i]`, `&(o)`), an update (`o++`, `--o`, `o->n++`: the DFG records no def for one), an array (it decays to a pointer),
//! a target undeclared in the owner passed to a call (it may be an array), an argument of a call whose
//! callee looks like a macro (all-caps, or `#define`d in the file) at any depth, any identifier inside an
//! `ERROR` node, and, for C++, an argument or a reference-bound initialiser rooted at the target (a
//! non-const reference parameter or `T& r = o.f` writes it), a method called on it, the right operand of
//! `>>` (stream extraction), and every identifier of a lambda body when the lambda captures by
//! reference. Residual, un-masked and counted in the gate: a macro that is neither all-caps nor
//! defined in this file, array fields decaying inside a call argument (`memcpy(o->buf, ..)`), and
//! a callee mutating the target through a pointer or global (out of model, as for `Exact`).
use super::shared::walk_owner;
use crate::ast::ParsedFile;
use std::collections::BTreeSet;
use tree_sitter::Node;

/// `ALL_CAPS_WITH_DIGITS`: a macro-looking callee.
fn macro_like(name: &str) -> bool {
    name.len() > 1
        && name.chars().next().is_some_and(|c| c.is_ascii_uppercase())
        && name
            .chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
}

/// The root identifier of a place expression (`o`, `o.f`, `o->f`, `o[i]`, `(o)`, `*o`, `(T)o`).
fn root_ident<'t>(parsed: &ParsedFile, mut node: Node<'t>) -> Option<String> {
    loop {
        match node.kind() {
            "identifier" => return Some(parsed.node_text(&node).to_string()),
            "field_expression" | "pointer_expression" => {
                node = node.child_by_field_name("argument")?;
            }
            "subscript_expression" => node = node.child_by_field_name("argument")?,
            "cast_expression" => node = node.child_by_field_name("value")?,
            "parenthesized_expression" => node = node.named_child(0)?,
            _ => return None,
        }
    }
}

/// `int *&r = o;`, `int& r{o}`, `T& r = c ? o : q;`: the reference's name and every identifier its
/// initialiser can yield, when a `reference_declarator` sits anywhere in the declarator chain.
fn reference_init(
    parsed: &ParsedFile,
    declarator: Node<'_>,
    value: Node<'_>,
) -> Option<(String, Vec<String>)> {
    let mut cur = Some(declarator);
    let mut by_ref = false;
    let mut name = None;
    while let Some(d) = cur {
        match d.kind() {
            "identifier" | "field_identifier" => {
                name = Some(parsed.node_text(&d).to_string());
                break;
            }
            "reference_declarator" => by_ref = true,
            _ => {}
        }
        cur = d
            .child_by_field_name("declarator")
            .or_else(|| d.named_child(0));
    }
    if !by_ref {
        return None;
    }
    let mut ids = BTreeSet::new();
    all_idents(parsed, value, &mut ids);
    Some((name?, ids.into_iter().collect()))
}

fn all_idents(parsed: &ParsedFile, node: Node<'_>, out: &mut BTreeSet<String>) {
    walk_owner(&node, |n| {
        if n.kind() == "identifier" {
            out.insert(parsed.node_text(&n).to_string());
        }
    });
}

/// What the file declares at top level (function bodies are not entered).
pub(super) struct FileNames {
    /// Enumerators and `#define`d macros: spellings that are not variables.
    pub tokens: BTreeSet<String>,
    /// `#define`d macros.
    pub macros: BTreeSet<String>,
    /// Functions defined or prototyped in the file.
    pub functions: BTreeSet<String>,
    /// File-scope and class-member references and the identifiers their initialisers name
    /// (`int& r = o;`): a hazard on one inside an owner is a hazard on its target.
    pub ref_pairs: Vec<(String, String)>,
}

/// Well-known C library names: a call to one cannot assign a by-value argument.
const LIBC: [&str; 163] = [
    "memcpy",
    "memmove",
    "memset",
    "memcmp",
    "memchr",
    "strlen",
    "strcmp",
    "strncmp",
    "strcasecmp",
    "strcpy",
    "strncpy",
    "strcat",
    "strncat",
    "strchr",
    "strrchr",
    "strstr",
    "strdup",
    "strndup",
    "strtok",
    "strspn",
    "strcspn",
    "strpbrk",
    "strtol",
    "strtoul",
    "strtoll",
    "strtoull",
    "strtod",
    "strtof",
    "atoi",
    "atol",
    "atoll",
    "atof",
    "printf",
    "fprintf",
    "sprintf",
    "snprintf",
    "vprintf",
    "vfprintf",
    "vsnprintf",
    "sscanf",
    "scanf",
    "fscanf",
    "puts",
    "fputs",
    "fputc",
    "putc",
    "putchar",
    "fgets",
    "fgetc",
    "getc",
    "getchar",
    "fwrite",
    "fread",
    "fopen",
    "fdopen",
    "freopen",
    "fclose",
    "fflush",
    "fseek",
    "ftell",
    "fileno",
    "feof",
    "ferror",
    "malloc",
    "calloc",
    "realloc",
    "free",
    "abort",
    "exit",
    "_exit",
    "atexit",
    "read",
    "write",
    "close",
    "open",
    "lseek",
    "stat",
    "fstat",
    "lstat",
    "unlink",
    "rename",
    "access",
    "chdir",
    "mkdir",
    "rmdir",
    "opendir",
    "readdir",
    "closedir",
    "dup",
    "dup2",
    "pipe",
    "fcntl",
    "ioctl",
    "mmap",
    "munmap",
    "system",
    "popen",
    "pclose",
    "fork",
    "wait",
    "waitpid",
    "kill",
    "signal",
    "getenv",
    "setenv",
    "putenv",
    "unsetenv",
    "getpid",
    "getuid",
    "abs",
    "labs",
    "llabs",
    "fabs",
    "sqrt",
    "pow",
    "floor",
    "ceil",
    "round",
    "fmod",
    "exp",
    "log",
    "sin",
    "cos",
    "tan",
    "rand",
    "srand",
    "time",
    "clock",
    "difftime",
    "gettimeofday",
    "clock_gettime",
    "sleep",
    "usleep",
    "nanosleep",
    "select",
    "poll",
    "socket",
    "bind",
    "listen",
    "accept",
    "connect",
    "send",
    "recv",
    "sendto",
    "recvfrom",
    "shutdown",
    "setsockopt",
    "getsockopt",
    "qsort",
    "bsearch",
    "perror",
    "strerror",
    "isdigit",
    "isspace",
    "isalpha",
    "isalnum",
    "isupper",
    "islower",
    "ispunct",
    "isprint",
    "toupper",
    "tolower",
    "assert",
];

pub(super) fn file_names(parsed: &ParsedFile) -> FileNames {
    let mut names = FileNames {
        tokens: BTreeSet::new(),
        macros: BTreeSet::new(),
        functions: BTreeSet::new(),
        ref_pairs: Vec::new(),
    };
    let mut stack = vec![parsed.tree.root_node()];
    while let Some(node) = stack.pop() {
        match node.kind() {
            "enumerator" | "preproc_def" | "preproc_function_def" => {
                if let Some(name) = node.child_by_field_name("name") {
                    let text = parsed.node_text(&name).to_string();
                    if node.kind() != "enumerator" {
                        names.macros.insert(text.clone());
                    }
                    names.tokens.insert(text);
                }
            }
            "function_declarator" => {
                let mut cur = node;
                while let Some(inner) = cur.child_by_field_name("declarator") {
                    cur = inner;
                }
                if matches!(cur.kind(), "identifier" | "field_identifier") {
                    names.functions.insert(parsed.node_text(&cur).to_string());
                }
            }
            "init_declarator" | "field_declaration" => {
                let value = node
                    .child_by_field_name("value")
                    .or_else(|| node.child_by_field_name("default_value"));
                if let (Some(d), Some(v)) = (node.child_by_field_name("declarator"), value) {
                    if let Some((name, ids)) = reference_init(parsed, d, v) {
                        names
                            .ref_pairs
                            .extend(ids.into_iter().map(|t| (name.clone(), t)));
                    }
                }
            }
            "function_definition" => {
                // The declarator (name), not the body.
                if let Some(d) = node.child_by_field_name("declarator") {
                    stack.push(d);
                }
                continue;
            }
            _ => {}
        }
        let mut cursor = node.walk();
        stack.extend(node.children(&mut cursor));
    }
    names
}

/// Every spelling of `owner` that keeps the twins of a target of that spelling.
pub(super) fn tokens(parsed: &ParsedFile, owner: &Node<'_>, file: &FileNames) -> BTreeSet<String> {
    let cpp = super::c_family::cpp_rules(parsed);
    let mut out: BTreeSet<String> = file.tokens.clone();
    // Names declared in the owner, to tell a decaying undeclared (outer) target from a local.
    let mut declared = BTreeSet::new();
    // C++ `T& r = o;` pairs (reference, target), to carry hazards on `r` over to `o`.
    let mut ref_aliases: Vec<(String, String)> = Vec::new();
    let mut calls: Vec<Node<'_>> = Vec::new();
    walk_owner(owner, |node| match node.kind() {
        "field_identifier"
        | "statement_identifier"
        | "type_identifier"
        | "namespace_identifier" => {
            out.insert(parsed.node_text(&node).to_string());
        }
        "enumerator" | "preproc_def" | "preproc_function_def" => {
            if let Some(name) = node.child_by_field_name("name") {
                out.insert(parsed.node_text(&name).to_string());
            }
        }
        "preproc_params" | "ERROR" => all_idents(parsed, node, &mut out),
        "qualified_identifier" => {
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if child.kind() == "identifier" {
                    out.insert(parsed.node_text(&child).to_string());
                }
            }
        }
        "template_argument_list" => all_idents(parsed, node, &mut out),
        "array_declarator" => {
            // An array: its name (the innermost declarator identifier).
            let mut cur = node;
            while let Some(inner) = cur.child_by_field_name("declarator") {
                cur = inner;
            }
            if cur.kind() == "identifier" {
                out.insert(parsed.node_text(&cur).to_string());
            }
        }
        "init_declarator" | "declaration" | "parameter_declaration" => {
            let mut cursor = node.walk();
            for first in node.children_by_field_name("declarator", &mut cursor) {
                let mut cur = first;
                while let Some(inner) = cur.child_by_field_name("declarator") {
                    cur = inner;
                }
                if cur.kind() == "identifier" {
                    declared.insert(parsed.node_text(&cur).to_string());
                }
            }
            if cpp && node.kind() == "init_declarator" {
                if let (Some(d), Some(value)) = (
                    node.child_by_field_name("declarator"),
                    node.child_by_field_name("value"),
                ) {
                    if let Some((name, idents)) = reference_init(parsed, d, value) {
                        // The alias map pairs only the plain `T& r = o;`: writes through that reference
                        // are storage writes whose twins stay. Every other initialiser form (`{o}`,
                        // `(o)`, `c ? o : q`, `o.f`) is a hidden write of whatever it can yield.
                        let plain =
                            value.kind() == "identifier" && d.kind() == "reference_declarator";
                        if !plain {
                            out.extend(idents.iter().cloned());
                        }
                        // A hazard on the reference is a hazard on its targets (fixpoint below).
                        ref_aliases.extend(idents.into_iter().map(|t| (name.clone(), t)));
                    }
                }
            }
        }
        "pointer_expression" => {
            let addr = node
                .child_by_field_name("operator")
                .is_some_and(|op| parsed.node_text(&op).trim() == "&");
            if let (true, Some(arg)) = (addr, node.child_by_field_name("argument")) {
                match root_ident(parsed, arg) {
                    Some(root) => {
                        out.insert(root);
                    }
                    // `&(c ? o : q)`: no single root; every identifier of the operand may be written
                    None => all_idents(parsed, arg, &mut out),
                }
            }
        }
        "call_expression" => calls.push(node),
        "update_expression" => {
            // `o++`, `--o`, `o->n++`: the DFG records no def for an update statement.
            if let Some(root) = node
                .child_by_field_name("argument")
                .and_then(|a| root_ident(parsed, a))
            {
                out.insert(root);
            }
        }
        "binary_expression" if cpp => {
            let extract = node
                .child_by_field_name("operator")
                .is_some_and(|op| parsed.node_text(&op).trim() == ">>");
            if let (true, Some(right)) = (extract, node.child_by_field_name("right")) {
                if let Some(root) = root_ident(parsed, right) {
                    out.insert(root);
                }
            }
        }
        "for_range_loop" if cpp => {
            let by_ref = node
                .child_by_field_name("declarator")
                .is_some_and(|d| d.kind() == "reference_declarator");
            if let (true, Some(range)) = (by_ref, node.child_by_field_name("right")) {
                if let Some(root) = root_ident(parsed, range) {
                    out.insert(root);
                }
            }
        }
        "lambda_expression" if cpp => {
            let captures = node.child_by_field_name("captures");
            let by_ref = captures.is_some_and(|c| parsed.node_text(&c).contains('&'));
            if let (true, Some(body)) = (by_ref, node.child_by_field_name("body")) {
                all_idents(parsed, body, &mut out);
            }
            // `[&x = o]`, `[&o]`, `[x = &o]`: the captured expression names what the body may write
            if let (true, Some(c)) = (by_ref, captures) {
                all_idents(parsed, c, &mut out);
            }
        }
        _ => {}
    });
    for call in calls {
        let callee = call.child_by_field_name("function");
        let callee_name = callee.map(|c| parsed.node_text(&c).trim().to_string());
        let macroish = callee_name
            .as_deref()
            .is_some_and(|n| macro_like(n) || file.macros.contains(n));
        // A callee that is no function of this file and no library name may be a macro of a header
        // (`Py_CLEAR(x)`, `atomicGet(v, dst)`): a bare name among its arguments may be assigned.
        let unproven = !macroish
            && callee.is_some_and(|c| c.kind() == "identifier")
            && callee_name
                .as_deref()
                .is_some_and(|n| !file.functions.contains(n) && !LIBC.contains(&n));
        let Some(args) = call.child_by_field_name("arguments") else {
            continue;
        };
        if macroish {
            all_idents(parsed, args, &mut out);
            continue;
        }
        // C++: a method called on the target may mutate it.
        if cpp {
            if let Some(c) = callee.filter(|c| c.kind() == "field_expression") {
                if let Some(root) = c
                    .child_by_field_name("argument")
                    .and_then(|a| root_ident(parsed, a))
                {
                    out.insert(root);
                }
            }
        }
        let mut cursor = args.walk();
        for arg in args.named_children(&mut cursor) {
            // `clear((o))`: parentheses around the bare name change nothing.
            let mut bare = arg;
            while bare.kind() == "parenthesized_expression" {
                match bare.named_child(0) {
                    Some(inner) => bare = inner,
                    None => break,
                }
            }
            if unproven && bare.kind() == "identifier" {
                out.insert(parsed.node_text(&bare).to_string());
            }
            let Some(root) = root_ident(parsed, arg) else {
                continue;
            };
            // C++: any argument can bind a non-const reference, a bare pointer included (`T*&` parameter).
            // C: an undeclared bare name may be an array (declared arrays are refused at their declarator).
            if cpp || (arg.kind() == "identifier" && !declared.contains(&root)) {
                out.insert(root);
            }
        }
    }
    // A hazard on a proven reference alias (`++r`, `f(r)`, `r.m()`, `&r`), local or declared outside the
    // owner, writes its target (fixpoint).
    ref_aliases.extend(file.ref_pairs.iter().cloned());
    loop {
        let before = out.len();
        for (r, o) in &ref_aliases {
            if out.contains(r) {
                out.insert(o.clone());
            }
        }
        if out.len() == before {
            break;
        }
    }
    out
}
