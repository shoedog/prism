//! 6d measurement: dump every function declarator (name, parameter passing kinds) of a corpus.
use prism::ast::ParsedFile;
use prism::languages::Language;
use std::path::Path;
use tree_sitter::Node;

fn walk_dir(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    for e in rd.flatten() {
        let p = e.path();
        let name = e.file_name().to_string_lossy().to_string();
        if name == ".git" || name == "third_party" || name == "secbench" {
            continue;
        }
        if p.is_dir() {
            walk_dir(&p, out);
        } else {
            out.push(p);
        }
    }
}

fn dkind(mut d: Option<Node>, src: &str) -> &'static str {
    let mut k = "val";
    while let Some(n) = d {
        match n.kind() {
            "reference_declarator" => {
                let t = &src[n.start_byte()..n.end_byte()];
                return if t.starts_with("&&") { "rref" } else { "ref" };
            }
            "pointer_declarator" => k = "ptr",
            "array_declarator" => k = "ptr",
            "function_declarator" => return "fnptr",
            _ => {}
        }
        d = n
            .child_by_field_name("declarator")
            .or_else(|| n.named_child(0));
        if matches!(d, Some(x) if x.kind()=="identifier"||x.kind()=="field_identifier"||x.kind()=="abstract_pointer_declarator" && false)
        {
            break;
        }
    }
    k
}

fn visit(n: Node, src: &str, file: &str, out: &mut Vec<String>) {
    if n.kind() == "function_declarator" {
        let mut inside_param = false;
        let mut tmpl = false;
        let mut class = false;
        let mut p = n.parent();
        while let Some(x) = p {
            match x.kind() {
                "parameter_declaration" | "optional_parameter_declaration" => inside_param = true,
                "template_declaration" => tmpl = true,
                "class_specifier" | "struct_specifier" | "field_declaration_list" => class = true,
                _ => {}
            }
            p = x.parent();
        }
        if !inside_param {
            if let (Some(d), Some(params)) = (
                n.child_by_field_name("declarator"),
                n.child_by_field_name("parameters"),
            ) {
                let q = &src[d.start_byte()..d.end_byte()];
                let name = q.rsplit("::").next().unwrap_or(q).trim().to_string();
                let mut kinds = Vec::new();
                let mut c = params.walk();
                for pr in params.named_children(&mut c) {
                    match pr.kind() {
                        "parameter_declaration" | "optional_parameter_declaration" => {
                            let ty = pr
                                .child_by_field_name("type")
                                .map(|t| src[t.start_byte()..t.end_byte()].to_string())
                                .unwrap_or_default();
                            let k = dkind(pr.child_by_field_name("declarator"), src);
                            kinds.push(format!("{k}:{}", ty.replace(['\t', '\n', ' ', '|'], "_")));
                        }
                        "variadic_parameter_declaration" => kinds.push("variadic:".into()),
                        "variadic_parameter" => kinds.push("variadic:".into()),
                        _ => {}
                    }
                }
                let is_def = n
                    .parent()
                    .is_some_and(|p| p.kind() == "function_definition");
                out.push(format!(
                    "{file}\t{}\t{name}\t{}\t{}\t{}\t{}\t{}\t{}",
                    n.start_position().row + 1,
                    q.replace(['\t', '\n'], " "),
                    kinds.len(),
                    tmpl as u8,
                    (class || q.contains("::")) as u8,
                    is_def as u8,
                    kinds.join("|"),
                ));
            }
        }
    }
    let mut c = n.walk();
    for k in n.named_children(&mut c) {
        visit(k, src, file, out);
    }
}

fn main() {
    let root = std::env::args().nth(1).expect("dir");
    let mut files = Vec::new();
    walk_dir(Path::new(&root), &mut files);
    let mut lines = Vec::new();
    for f in files {
        let ext = f
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();
        let lang = match ext.as_str() {
            "c" => Language::C,
            "h" | "cc" | "cpp" | "cxx" | "hpp" | "hh" | "hxx" | "inc" => Language::Cpp,
            _ => continue,
        };
        let Ok(src) = std::fs::read_to_string(&f) else {
            continue;
        };
        let rel = f.strip_prefix(&root).unwrap().to_string_lossy().to_string();
        let Ok(pf) = ParsedFile::parse(&rel, &src, lang) else {
            continue;
        };
        visit(pf.tree.root_node(), &pf.source, &rel, &mut lines);
    }
    lines.sort();
    println!("{}", lines.join("\n"));
}
