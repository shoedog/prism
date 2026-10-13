// PR-B Gap-1 census on prism's own predicates (not a syntactic mirror).
// Copy to examples/pd_census_b.rs; build --release --example pd_census_b --offline --locked.
// Usage: pd_census_b <root> [<root>...]  -> one JSON object on stdout (aggregate only).
use prism::languages::Language;
use serde_json::json;
use std::collections::BTreeMap;
fn parent_class(node: tree_sitter::Node<'_>) -> String {
    let mut child = node;
    let mut parent = node.parent();
    while let Some(p) = parent {
        if p.kind() == "parenthesized_expression" { child = p; parent = p.parent(); continue; }
        break;
    }
    let Some(p) = parent else { return "root".into() };
    match p.kind() {
        "arguments" => match p.parent().map(|g| g.kind()) {
            Some("call_expression") => "call_argument".into(),
            Some("new_expression") => "new_argument".into(),
            other => format!("arguments_of:{other:?}"),
        },
        "call_expression" if p.child_by_field_name("function").is_some_and(|f| f.id() == child.id()) => "iife".into(),
        "jsx_expression" => "jsx_expression".into(),
        "return_statement" => "return".into(),
        "assignment_expression" => "assignment_other_lhs".into(),
        "arrow_function" => "arrow_body_curried".into(),
        "variable_declarator" => "declarator_unnamed".into(),
        "pair" => "pair_unnamed".into(),
        "export_statement" => "export_default".into(),
        "ternary_expression" => "conditional".into(),
        "binary_expression" => "binary_or_logical".into(),
        "array" => "array_element".into(),
        "assignment_pattern" | "required_parameter" | "optional_parameter" => "default_parameter".into(),
        k => format!("other:{k}"),
    }
}
fn main() -> anyhow::Result<()> {
    let mut agg: BTreeMap<String, u64> = BTreeMap::new();
    let mut inc = |k: String, n: u64| *agg.entry(k).or_insert(0) += n;
    for root in std::env::args().skip(1) {
        let repo = prism::repo_loader::load_repo(std::path::Path::new(&root))?;
        inc("roots".into(), 1);
        for (_file, parsed) in &repo.files {
            if !matches!(parsed.language, Language::JavaScript | Language::TypeScript | Language::Tsx) { continue; }
            inc("files".into(), 1);
            let funcs = parsed.all_functions();
            let types = parsed.language.function_node_types();
            for f in &funcs {
                inc("callables".into(), 1);
                if parsed.language.function_name(f).is_some() { inc("named".into(), 1); continue; }
                inc("anonymous".into(), 1);
                inc(format!("anon_kind:{}", f.kind()), 1);
                let pc = parent_class(*f);
                inc(format!("anon_parent:{pc}"), 1);
                // nesting: nearest pass-owning (named) ancestor and nearest callable ancestor
                let (mut named_anc, mut anon_anc) = (false, false);
                let mut a = f.parent();
                while let Some(n) = a {
                    if types.contains(&n.kind()) {
                        if parsed.language.function_name(&n).is_some() { named_anc = true; } else { anon_anc = true; }
                    }
                    a = n.parent();
                }
                let nest = if named_anc { "under_named" } else if anon_anc { "under_anon_only" } else { "top_level" };
                inc(format!("anon_nest:{nest}"), 1);
                let lines = parsed.node_line_range(f);
                inc(format!("anon_lines_{nest}"), (lines.1 - lines.0 + 1) as u64);
                let occ = parsed.function_parameter_occurrences(f);
                inc("anon_formal_occurrences".into(), occ.len() as u64);
                let mut bare = 0u64;
                for (name, _s, _e) in &occ {
                    if parsed.has_bare_references(f, name) {
                        bare += 1;
                        let path = prism::access_path::AccessPath::simple(name.clone());
                        let refs = parsed.find_path_references_scoped(f, &path, lines.0);
                        inc(format!("anon_bare_formal_ref_lines_{nest}"), refs.len() as u64);
                    } else { inc("anon_formal_member_only_or_unused".into(), 1); }
                }
                inc(format!("anon_bare_formals_{nest}"), bare);
                inc(format!("anon_bare_formals_parent:{pc}"), bare);
                if bare > 0 { inc("anon_with_bare_formal".into(), 1); }
            }
        }
    }
    println!("{}", json!(agg));
    Ok(())
}
