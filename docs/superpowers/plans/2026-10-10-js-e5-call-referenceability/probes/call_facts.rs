// Static evidence only, no execution of corpus modules.
use prism::call_graph::CallGraph;
use prism::languages::Language;
use serde_json::json;
fn form(f: &tree_sitter::Node<'_>) -> &'static str {
    let p = f.parent();
    let parent = p.map(|p| p.kind()).unwrap_or("none");
    match f.kind() {
        "function_declaration" | "generator_function_declaration" => "function_declaration",
        "method_definition" if parent == "object" => "object_shorthand_method",
        "method_definition" => "class_member",
        _ => match parent {
            "variable_declarator" => "variable_direct_function",
            "pair" => "object_pair_value",
            "public_field_definition" | "field_definition" => "class_member",
            "assignment_expression" => match p.and_then(|p| p.child_by_field_name("left")).map(|l| l.kind()) {
                Some("member_expression" | "subscript_expression") => "member_assignment",
                _ => "other:identifier_assignment",
            },
            "arguments" if p.and_then(|p|p.parent()).and_then(|p|p.parent()).is_some_and(|p|p.kind()=="variable_declarator") => "variable_wrapper_call",
            _ if f.kind() == "function_expression" && f.child_by_field_name("name").is_some() => "other:named_function_expression",
            _ => "other:syntax",
        },
    }
}
fn main() -> anyhow::Result<()> {
    let root = std::env::args().nth(1).expect("root");
    let repo = prism::repo_loader::load_repo(std::path::Path::new(&root))?;
    let cg = CallGraph::build_with_scope_graph_inputs(&repo.files, repo.scope_graph_inputs.as_ref());
    for sites in cg.calls.values() {
        for site in sites {
            let out = cg.resolve_call_site_full(site);
            for r in out.resolved {
                if r.kind != prism::resolution::ResolutionKind::FreeSingle
                    || r.confidence != prism::resolution::ResolutionConfidence::Exact
                    || !matches!(Language::from_path(&site.caller.file), Some(Language::JavaScript | Language::TypeScript | Language::Tsx)) {
                    continue;
                }
                let parsed = &repo.files[&r.target.file];
                let forms: Vec<_> = parsed.all_functions().iter().filter_map(|f| {
                    let n = parsed.language.function_name(f)?;
                    let (start, end) = parsed.node_line_range(f);
                    if parsed.node_text(&n) != r.target.name || start != r.target.start_line || end != r.target.end_line { return None; }
                    let js = matches!(parsed.language, Language::JavaScript | Language::TypeScript | Language::Tsx);
                    let root = parsed.tree.root_node(); let mut cursor = root.walk();
                    let esm = js && root.named_children(&mut cursor).any(|n|matches!(n.kind(),"import_statement" | "export_statement"));
                    Some(json!({"kind":f.kind(),"name_explicit":f.child_by_field_name("name").is_some(),
                        "form": if js { form(f).to_string() } else { format!("other:foreign:{:?}",parsed.language) },"esm":esm,
                        "parent":f.parent().map(|p|p.kind()),"start_byte":f.start_byte(),"end_byte":f.end_byte()}))
                }).collect();
                println!("{}",json!({"caller":site.caller,"span":[site.start_byte,site.end_byte],"name":site.callee_name,
                    "origin":site.origin,"jsx":site.jsx_element,"qualifier":site.qualifier,"local_binding":site.local_binding,
                    "target":r.target,"forms":forms}));
            }
        }
    }
    Ok(())
}
