//! 6d-2 prototype (measurement only): repo-wide type-spelling census with the tree-sitter-cpp grammar.
//! `census6d <dir>`        facts on stdout (FILE / S / H / D / F / META lines), timing on stderr.
//! `census6d probe <file>` prints kind|parent.field|role for every identifier-ish node of a snippet.
use prism::ast::ParsedFile;
use prism::languages::Language;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use tree_sitter::Node;

const EXTS: &[&str] = &[
    "c", "h", "cc", "cpp", "cxx", "hpp", "hh", "hxx", "h++", "c++", "inl", "inc", "ipp", "tcc",
    "tpp", "txx", "def", "cu", "cuh", "x",
];

fn walk_dir(dir: &Path, out: &mut Vec<std::path::PathBuf>, skipped: &mut Vec<String>) {
    let Ok(rd) = std::fs::read_dir(dir) else {
        skipped.push(format!("unreadable-dir {}", dir.display()));
        return;
    };
    for e in rd.flatten() {
        let p = e.path();
        let name = e.file_name().to_string_lossy().to_string();
        if name == ".git" || name == "secbench" || name == "frontend-portal" {
            continue;
        }
        let Ok(ft) = e.file_type() else { continue };
        if ft.is_symlink() {
            skipped.push(format!("symlink {}", p.display()));
            continue;
        }
        if ft.is_dir() {
            walk_dir(&p, out, skipped);
        } else {
            out.push(p);
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
enum Role {
    TypeUse,
    TypeDecl,
    Other,
    Unknown,
}

fn field_of(n: Node) -> Option<&'static str> {
    let p = n.parent()?;
    let mut c = p.walk();
    if !c.goto_first_child() {
        return None;
    }
    loop {
        if c.node().id() == n.id() {
            return c.field_name();
        }
        if !c.goto_next_sibling() {
            return None;
        }
    }
}

fn is_c2_spelling(t: &str) -> bool {
    let t = t.strip_prefix("::").unwrap_or(t);
    let t = t.strip_prefix("std::").unwrap_or(t);
    if matches!(
        t,
        "size_t" | "ssize_t" | "ptrdiff_t" | "intptr_t" | "uintptr_t" | "intmax_t" | "uintmax_t"
    ) {
        return true;
    }
    let t = t.strip_prefix('u').unwrap_or(t);
    let Some(rest) = t.strip_prefix("int") else {
        return false;
    };
    let Some(rest) = rest.strip_suffix("_t") else {
        return false;
    };
    let rest = rest
        .strip_prefix("_least")
        .or_else(|| rest.strip_prefix("_fast"))
        .unwrap_or(rest);
    !rest.is_empty() && rest.chars().all(|c| c.is_ascii_digit())
}

const OCC_KINDS: &[&str] = &[
    "identifier",
    "type_identifier",
    "primitive_type",
    "namespace_identifier",
    "field_identifier",
    "statement_identifier",
];

fn stmt_like(k: &str) -> bool {
    k.ends_with("statement") || k.ends_with("declaration") || k.ends_with("definition")
}

fn kw_int(text: &str) -> bool {
    let toks: Vec<&str> = text.split_whitespace().collect();
    !toks.is_empty()
        && toks
            .iter()
            .all(|t| matches!(*t, "char" | "short" | "int" | "long" | "signed" | "unsigned"))
}

fn direct_int_type(parsed_src: &str, ty: Node, owner: Node) -> bool {
    if !matches!(ty.kind(), "primitive_type" | "sized_type_specifier") {
        return false;
    }
    let mut c = owner.walk();
    if owner.children(&mut c).any(|k| k.kind() == "type_qualifier") {
        return false;
    }
    kw_int(&parsed_src[ty.start_byte()..ty.end_byte()])
}

fn relaxed() -> bool {
    std::env::var("CENSUS_RELAXED").is_ok()
}

fn classify(n: Node, src: &str) -> (Role, String) {
    // 1. error / missing ownership
    let mut a = n.parent();
    let mut near: Option<Node> = None;
    while let Some(x) = a {
        if x.is_error() || x.is_missing() {
            // relaxed (sensitivity only): an ERROR above the nearest declaration/statement is ignored
            if !(relaxed() && near.is_some()) {
                return (Role::Unknown, "error-ancestor".into());
            }
        }
        if near.is_none() && stmt_like(x.kind()) {
            near = Some(x);
        }
        a = x.parent();
    }
    if let Some(d) = near {
        if d.has_error() {
            return (Role::Unknown, "error-in-enclosing-decl".into());
        }
    }
    let kind = n.kind();
    match kind {
        "namespace_identifier" => {
            if relaxed()
                && n.parent().is_some_and(|p| {
                    p.kind() == "qualified_identifier" && field_of(n) == Some("scope")
                })
            {
                return (Role::TypeUse, "scope-position(relaxed)".into());
            }
            return (Role::Other, "namespace-or-scope-name".into());
        }
        "field_identifier" => return (Role::Other, "field".into()),
        "statement_identifier" => return (Role::Other, "label".into()),
        "identifier" => {
            let mut prev = n;
            let mut a = n.parent();
            while let Some(x) = a {
                let k = x.kind();
                if k.starts_with("preproc_") {
                    let container = matches!(
                        k,
                        "preproc_if" | "preproc_ifdef" | "preproc_else" | "preproc_elif" | "preproc_elifdef"
                    );
                    if !container
                        || matches!(field_of(prev), Some("condition") | Some("name"))
                    {
                        return (Role::Unknown, "preproc-cond".into());
                    }
                }
                prev = x;
                a = x.parent();
            }
            if let Some(q) = n.parent().filter(|q| q.kind() == "qualified_identifier") {
                if q.parent().is_some_and(|g| g.kind() == "using_declaration") {
                    return (Role::TypeDecl, "using-decl".into());
                }
            }
            return (Role::Other, "identifier".into());
        }
        _ => {}
    }
    // type_identifier / primitive_type
    let mut cur = n;
    let mut qualified = false;
    if let Some(p) = n.parent() {
        if p.kind() == "qualified_identifier" {
            if field_of(n) == Some("name") {
                match p.child_by_field_name("scope") {
                    None => {}
                    Some(s) if &src[s.start_byte()..s.end_byte()] == "std" => {}
                    Some(_) => return (Role::Other, "qualified-foreign-scope".into()),
                }
                cur = p;
                qualified = true;
            } else {
                return (Role::Other, "qualified-scope-position".into());
            }
        }
    }
    let Some(p) = cur.parent() else {
        return (Role::Unknown, "no-parent".into());
    };
    let pk = p.kind();
    let f = field_of(cur).unwrap_or("");
    let q = if qualified { "q:" } else { "" };
    match (pk, f) {
        ("declaration", "type")
        | ("parameter_declaration", "type")
        | ("optional_parameter_declaration", "type")
        | ("field_declaration", "type")
        | ("function_definition", "type")
        | ("type_descriptor", "type")
        | ("for_range_loop", "type")
        | ("new_expression", "type")
        | ("type_definition", "type")
        | ("condition_declaration", "type")
        | ("enum_specifier", "base")
        | ("compound_literal_expression", "type")
        | ("variadic_parameter_declaration", "type")
        | ("optional_type_parameter_declaration", "default_type")
        | ("template_type", "name")
        | ("friend_declaration", "")
        | ("template_argument_list", "")
        | ("base_class_clause", "")
        | ("trailing_return_type", "")
        | ("template_template_parameter_declaration", "") => {
            return (Role::TypeUse, format!("{q}{pk}.{f}"));
        }
        ("type_descriptor", "") | ("sizeof_expression", "type") | ("alignof_expression", "type") => {
            return (Role::TypeUse, format!("{q}{pk}.{f}"));
        }
        _ => {}
    }
    match pk {
        "class_specifier" | "struct_specifier" | "union_specifier" | "enum_specifier"
            if f == "name" =>
        {
            if p.child_by_field_name("body").is_some() {
                return (Role::TypeDecl, format!("{q}{pk}-definition"));
            }
            if let Some(g) = p.parent() {
                if matches!(g.kind(), "declaration" | "field_declaration")
                    && g.child_by_field_name("declarator").is_none()
                {
                    return (Role::TypeDecl, format!("{q}{pk}-forward-decl"));
                }
                if matches!(
                    g.kind(),
                    "translation_unit" | "declaration_list" | "field_declaration_list" | "compound_statement"
                ) {
                    return (Role::TypeDecl, format!("{q}{pk}-forward-decl"));
                }
                if g.kind() == "friend_declaration" {
                    return (Role::TypeDecl, format!("{q}friend-decl"));
                }
            }
            return (Role::TypeUse, format!("{q}elaborated-{pk}"));
        }
        "type_parameter_declaration"
        | "optional_type_parameter_declaration"
        | "variadic_type_parameter_declaration" => {
            if f == "name" || f.is_empty() {
                return (Role::TypeDecl, format!("{q}template-type-param"));
            }
        }
        "alias_declaration" if f == "name" => {
            let direct = p
                .child_by_field_name("type")
                .and_then(|t| {
                    if t.kind() == "type_descriptor" && t.child_by_field_name("declarator").is_none()
                    {
                        t.child_by_field_name("type").map(|x| direct_int_type(src, x, t))
                    } else {
                        None
                    }
                })
                .unwrap_or(false);
            return (
                Role::TypeDecl,
                format!("{q}{}", if direct { "alias-direct-int" } else { "alias-other" }),
            );
        }
        "using_declaration" => return (Role::TypeDecl, format!("{q}using-decl")),
        "type_definition" if f == "declarator" => {
            let direct = p
                .child_by_field_name("type")
                .map(|t| direct_int_type(src, t, p))
                .unwrap_or(false);
            return (
                Role::TypeDecl,
                format!("{q}{}", if direct { "typedef-direct-int" } else { "typedef-other" }),
            );
        }
        _ => {}
    }
    // typedef declarator wrapped in pointer/array/function/paren declarators
    if f == "declarator"
        && matches!(
            pk,
            "pointer_declarator"
                | "array_declarator"
                | "function_declarator"
                | "parenthesized_declarator"
                | "reference_declarator"
        )
    {
        let mut a = p.parent();
        while let Some(x) = a {
            match x.kind() {
                "type_definition" => {
                    return (Role::TypeDecl, format!("{q}typedef-complex-declarator"))
                }
                "pointer_declarator" | "array_declarator" | "function_declarator"
                | "parenthesized_declarator" | "reference_declarator" => a = x.parent(),
                _ => break,
            }
        }
    }
    // function-declarator names (constructors etc.)
    if pk == "function_declarator" && f == "declarator" {
        return (Role::Other, format!("{q}function-name"));
    }
    (Role::Unknown, format!("{q}unclassified:{pk}.{f}"))
}

struct Occ {
    counts: BTreeMap<(Role, String), usize>,
    first: BTreeMap<(Role, String), Vec<String>>,
}

#[derive(Default)]
struct Decl {
    file: String,
    line: usize,
    var: String,
    ty: String,
    tykind: String,
    shapes: String,
    fstart: usize,
    fend: usize,
}

fn text<'a>(n: Node, src: &'a str) -> &'a str {
    &src[n.start_byte()..n.end_byte()]
}

fn declarator_name<'a>(mut d: Node<'a>, src: &str) -> Option<String> {
    loop {
        match d.kind() {
            "identifier" => return Some(text(d, src).to_string()),
            "init_declarator" | "pointer_declarator" | "parenthesized_declarator"
            | "reference_declarator" | "array_declarator" => {
                d = d
                    .child_by_field_name("declarator")
                    .or_else(|| d.named_child(0))?;
            }
            _ => return None,
        }
    }
}

fn pointer_top(d: Node) -> Option<Node> {
    let top = if d.kind() == "init_declarator" {
        d.child_by_field_name("declarator")?
    } else {
        d
    };
    let mut n = top;
    loop {
        match n.kind() {
            "pointer_declarator" => return Some(top),
            "parenthesized_declarator" => n = n.named_child(0)?,
            _ => return None,
        }
    }
}

fn collect_decls(n: Node, src: &str, file: &str, fstack: &mut Vec<(usize, usize)>, out: &mut Vec<Decl>) {
    let is_fn = n.kind() == "function_definition";
    if is_fn {
        fstack.push((n.start_position().row + 1, n.end_position().row + 1));
    }
    if n.kind() == "declaration" && !fstack.is_empty() {
        if let Some(ty) = n.child_by_field_name("type") {
            let tk = ty.kind();
            let scalar = matches!(tk, "primitive_type" | "sized_type_specifier");
            if !scalar {
                let mut c = n.walk();
                let decls: Vec<Node> = n.children_by_field_name("declarator", &mut c).collect();
                for d in decls {
                    let Some(top) = pointer_top(d) else { continue };
                    let mut shapes = Vec::new();
                    let mut c2 = n.walk();
                    for k in n.children(&mut c2) {
                        match k.kind() {
                            "type_qualifier" => shapes.push("qual"),
                            "storage_class_specifier" => shapes.push("storage"),
                            "attribute_specifier" => shapes.push("attr"),
                            _ => {}
                        }
                    }
                    if matches!(
                        tk,
                        "struct_specifier" | "class_specifier" | "union_specifier" | "enum_specifier"
                    ) {
                        shapes.push("spec-type");
                    }
                    let mut x = Some(top);
                    let mut depth = 0;
                    while let Some(y) = x {
                        if y.kind() == "pointer_declarator" {
                            depth += 1;
                            let mut c3 = y.walk();
                            if y.children(&mut c3).any(|k| k.kind() == "type_qualifier") {
                                shapes.push("ptr-const");
                            }
                        }
                        x = y.child_by_field_name("declarator");
                    }
                    if depth > 1 {
                        shapes.push("multi-ptr");
                    }
                    shapes.sort();
                    shapes.dedup();
                    let (fs, fe) = *fstack.last().unwrap();
                    out.push(Decl {
                        file: file.to_string(),
                        line: d.start_position().row + 1,
                        var: declarator_name(d, src).unwrap_or_default(),
                        ty: text(ty, src).replace(['\t', '\n'], " "),
                        tykind: tk.to_string(),
                        shapes: if shapes.is_empty() { "plain".into() } else { shapes.join("+") },
                        fstart: fs,
                        fend: fe,
                    });
                }
            }
        }
    }
    let mut c = n.walk();
    for k in n.children(&mut c) {
        collect_decls(k, src, file, fstack, out);
    }
    if is_fn {
        fstack.pop();
    }
}

fn collect_occ(n: Node, src: &str, f: &mut dyn FnMut(Node)) {
    if OCC_KINDS.contains(&n.kind()) {
        f(n);
    }
    let mut c = n.walk();
    for k in n.children(&mut c) {
        collect_occ(k, src, f);
    }
}

fn fn_ranges(n: Node, file: &str, out: &mut Vec<String>) {
    if n.kind() == "function_definition" {
        out.push(format!(
            "F\t{file}\t{}\t{}",
            n.start_position().row + 1,
            n.end_position().row + 1
        ));
    }
    let mut c = n.walk();
    for k in n.children(&mut c) {
        fn_ranges(k, file, out);
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).map(String::as_str) == Some("probe") {
        let src = std::fs::read_to_string(&args[2]).unwrap();
        let pf = ParsedFile::parse("probe.cc", &src, Language::Cpp).unwrap();
        println!("{}", pf.tree.root_node().to_sexp());
        collect_occ(pf.tree.root_node(), &pf.source, &mut |n| {
            let (r, why) = classify(n, &pf.source);
            let p = n.parent().map(|p| p.kind()).unwrap_or("");
            println!(
                "{}:{} {} `{}` parent={}.{} -> {:?} {}",
                n.start_position().row + 1,
                n.start_position().column,
                n.kind(),
                text(n, &pf.source),
                p,
                field_of(n).unwrap_or(""),
                r,
                why
            );
        });
        return;
    }
    let root = args[1].clone();
    let t0 = std::time::Instant::now();
    let mut files = Vec::new();
    let mut skipped = Vec::new();
    walk_dir(Path::new(&root), &mut files, &mut skipped);
    files.sort();
    let mut parsed: Vec<(String, ParsedFile)> = Vec::new();
    let mut bytes = 0usize;
    let mut failed = Vec::new();
    for f in files {
        let ext = f.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
        if !EXTS.contains(&ext.as_str()) {
            continue;
        }
        let rel = f.strip_prefix(&root).unwrap().to_string_lossy().to_string();
        let Ok(bytes_raw) = std::fs::read(&f) else {
            failed.push(format!("unreadable {rel}"));
            continue;
        };
        let src = String::from_utf8_lossy(&bytes_raw).to_string();
        bytes += src.len();
        println!("FILE\t{rel}\t{}", src.len());
        match ParsedFile::parse(&rel, &src, Language::Cpp) {
            Ok(pf) => parsed.push((rel, pf)),
            Err(e) => failed.push(format!("parse-failed {rel}: {e}")),
        }
    }
    let t_parse = t0.elapsed();
    // pass 1: decls (C4 candidates), function ranges, interest set
    let mut decls = Vec::new();
    let mut interest: BTreeSet<String> = BTreeSet::new();
    let mut frs = Vec::new();
    let mut err_files = 0;
    for (rel, pf) in &parsed {
        if pf.tree.root_node().has_error() {
            err_files += 1;
        }
        collect_decls(pf.tree.root_node(), &pf.source, rel, &mut Vec::new(), &mut decls);
        fn_ranges(pf.tree.root_node(), rel, &mut frs);
        collect_occ(pf.tree.root_node(), &pf.source, &mut |n| {
            let t = text(n, &pf.source);
            if is_c2_spelling(t) {
                interest.insert(t.to_string());
            }
        });
    }
    for d in &decls {
        if d.tykind == "type_identifier" {
            interest.insert(d.ty.clone());
        }
    }
    // pass 2: roles of every occurrence of an interest spelling
    let mut table: BTreeMap<String, Occ> = BTreeMap::new();
    let mut hist: BTreeMap<(String, Role), usize> = BTreeMap::new();
    for (rel, pf) in &parsed {
        collect_occ(pf.tree.root_node(), &pf.source, &mut |n| {
            let t = text(n, &pf.source);
            if !interest.contains(t) {
                return;
            }
            let (r, why) = classify(n, &pf.source);
            let p = n.parent().map(|p| p.kind()).unwrap_or("");
            let ctx = format!("{}|{}.{}", n.kind(), p, field_of(n).unwrap_or(""));
            *hist.entry((ctx, r)).or_default() += 1;
            let o = table.entry(t.to_string()).or_insert_with(|| Occ {
                counts: BTreeMap::new(),
                first: BTreeMap::new(),
            });
            *o.counts.entry((r, why.clone())).or_default() += 1;
            let v = o.first.entry((r, why)).or_default();
            if v.len() < 4 {
                v.push(format!("{rel}:{}", n.start_position().row + 1));
            }
        });
    }
    // textual occurrences inside non-define preprocessor text (pragma/error/call args) are Unknown
    let mut preproc_text: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (rel, pf) in &parsed {
        fn walk_pp(n: Node, src: &str, rel: &str, interest: &BTreeSet<String>, out: &mut BTreeMap<String, Vec<String>>) {
            if matches!(n.kind(), "preproc_call" | "preproc_directive") {
                let t = text(n, src);
                let mut tok = String::new();
                for ch in t.chars().chain(std::iter::once(' ')) {
                    if ch.is_alphanumeric() || ch == '_' {
                        tok.push(ch);
                    } else {
                        if interest.contains(&tok) {
                            out.entry(tok.clone())
                                .or_default()
                                .push(format!("{rel}:{}", n.start_position().row + 1));
                        }
                        tok.clear();
                    }
                }
            }
            let mut c = n.walk();
            for k in n.children(&mut c) {
                walk_pp(k, src, rel, interest, out);
            }
        }
        walk_pp(pf.tree.root_node(), &pf.source, rel, &interest, &mut preproc_text);
    }
    let t_all = t0.elapsed();
    for (s, o) in &table {
        for ((r, why), c) in &o.counts {
            let locs = o.first[&(*r, why.clone())].join(";");
            println!("S\t{s}\t{r:?}\t{why}\t{c}\t{locs}");
        }
    }
    for (s, v) in &preproc_text {
        println!(
            "S\t{s}\tUnknown\tpreproc-text\t{}\t{}",
            v.len(),
            v.iter().take(4).cloned().collect::<Vec<_>>().join(";")
        );
    }
    for ((ctx, r), c) in &hist {
        println!("H\t{ctx}\t{r:?}\t{c}");
    }
    for d in &decls {
        println!(
            "D\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            d.file, d.line, d.var, d.ty, d.tykind, d.shapes, d.fstart, d.fend
        );
    }
    for l in frs {
        println!("{l}");
    }
    println!(
        "META\tfiles_parsed={}\tfiles_with_error={}\tbytes={bytes}\tinterest={}\tfailed={}\tskipped={}\tparse_ms={}\ttotal_ms={}",
        parsed.len(),
        err_files,
        interest.len(),
        failed.len(),
        skipped.len(),
        t_parse.as_millis(),
        t_all.as_millis()
    );
    for f in failed.iter().chain(skipped.iter()) {
        println!("LOSS\t{f}");
    }
}
