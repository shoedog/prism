//! Read-only JS/TS/TSX research observations, not resolution authority.
//! Run on an owner-approved immutable snapshot; stdout contains local paths and
//! identifiers and must stay private for private corpora. Bound subprocess time
//! and output externally. No installs, compiler invocation or cache writes.
use anyhow::{ensure, Result};
use prism::{
    ast::ParsedFile,
    cpg::{CodePropertyGraph, CpgEdge, CpgNode, VarAccess},
    languages::Language,
};
use serde_json::{json, Value};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};
use tree_sitter::Node;

fn form(node: Node<'_>, single: bool) -> &'static str {
    if recovery(node) {
        return "recovery";
    }
    if single {
        return "unparenthesized_identifier";
    }
    if node.child_by_field_name("decorator").is_some() {
        return "decorated";
    }
    let mut cursor = node.walk();
    if node.children(&mut cursor).any(|n| {
        matches!(
            n.kind(),
            "accessibility_modifier" | "override_modifier" | "readonly"
        )
    }) {
        return "parameter_property";
    }
    let pattern = node
        .child_by_field_name("pattern")
        .or_else(|| node.child_by_field_name("name"))
        .or_else(|| node.child_by_field_name("left"))
        .unwrap_or(node);
    if pattern.kind() == "this" {
        return "erased_this";
    }
    let identifier = pattern.kind() == "identifier";
    if node.kind() == "optional_parameter" {
        return if identifier {
            "optional_identifier"
        } else {
            "optional_pattern"
        };
    }
    if node.child_by_field_name("value").is_some() || node.kind() == "assignment_pattern" {
        return if identifier {
            "default_identifier"
        } else {
            "default_pattern"
        };
    }
    match pattern.kind() {
        "rest_pattern" => "rest",
        "object_pattern" | "array_pattern" => "destructured",
        "identifier" if node.kind() == "required_parameter" => "required_identifier",
        "identifier" if node.kind() == "identifier" => "javascript_identifier",
        _ => "other",
    }
}

fn recovery(node: Node<'_>) -> bool {
    let mut cursor = node.walk();
    node.is_error() || node.is_missing() || node.children(&mut cursor).any(recovery)
}

type DefKey = (String, String, usize, String, usize, usize);
fn endpoint(node: &CpgNode) -> Option<Value> {
    match node {
        CpgNode::Variable {
            file,
            function,
            function_start_line,
            line,
            path,
            start_byte,
            end_byte,
            ..
        } => Some(
            json!({"file":file,"function":function,"function_start_line":function_start_line,
                "line":line,"path":path.to_string(),"start_byte":start_byte,"end_byte":end_byte}),
        ),
        _ => None,
    }
}

fn observe(
    files: &BTreeMap<String, ParsedFile>,
    hashes: &BTreeMap<String, String>,
    skipped: Value,
) -> Value {
    let cpg = prism::build_pool::install(|| CodePropertyGraph::build(files));
    let dfg_defs: BTreeSet<DefKey> = cpg
        .dfg
        .defs
        .values()
        .flatten()
        .map(|d| {
            (
                d.file.clone(),
                d.function.clone(),
                d.function_start_line,
                d.path.to_string(),
                d.start_byte,
                d.end_byte,
            )
        })
        .collect();
    let cpg_defs: BTreeSet<DefKey> = cpg
        .graph
        .node_weights()
        .filter_map(|node| match node {
            CpgNode::Variable {
                file,
                function,
                function_start_line,
                path,
                start_byte,
                end_byte,
                access: VarAccess::Def,
                ..
            } => Some((
                file.clone(),
                function.clone(),
                *function_start_line,
                path.to_string(),
                *start_byte,
                *end_byte,
            )),
            _ => None,
        })
        .collect();
    let mut flows = Vec::new();
    for edge in cpg.graph.edge_indices() {
        let CpgEdge::DataFlow(confidence) = &cpg.graph[edge] else {
            continue;
        };
        let (from, to) = cpg.graph.edge_endpoints(edge).unwrap();
        if matches!(
            cpg.graph[from],
            CpgNode::Variable {
                access: VarAccess::Use,
                ..
            }
        ) && matches!(
            cpg.graph[to],
            CpgNode::Variable {
                access: VarAccess::Def,
                ..
            }
        ) {
            flows.push(json!({"from":endpoint(&cpg.graph[from]).unwrap(),"to":endpoint(&cpg.graph[to]).unwrap(),"confidence":confidence}));
        }
    }
    flows.sort_by_cached_key(Value::to_string);
    let mut functions = Vec::new();
    for (file, parsed) in files {
        for function in parsed.all_functions() {
            let owner = parsed
                .language
                .function_name(&function)
                .map(|n| parsed.node_text(&n).to_string());
            let start_line = function.start_position().row + 1;
            let params = function
                .child_by_field_name("parameters")
                .or_else(|| function.child_by_field_name("parameter"));
            let parameters: Vec<_> = params.map(|params| {
                let mut cursor = params.walk();
                let single = params.kind() == "identifier";
                let nodes = if single {vec![params]} else {params.named_children(&mut cursor).filter(|n| n.kind() != "comment").collect()};
                nodes.into_iter().map(|node|json!({"start_byte":node.start_byte(),"end_byte":node.end_byte(),"kind":node.kind(),"form":form(node,single)})).collect()
            }).unwrap_or_default();
            let occurrences: Vec<_> = parsed
                .function_parameter_occurrences(&function)
                .into_iter()
                .map(|(name, start, end)| {
                    let key = (
                        file.clone(),
                        owner.clone().unwrap_or_default(),
                        start_line,
                        name.clone(),
                        start,
                        end,
                    );
                    json!({"name":name,"start_byte":start,"end_byte":end,
                    "bare_reference":parsed.has_bare_references(&function,&name),
                    "dfg_def":owner.is_some() && dfg_defs.contains(&key),
                    "cpg_def":owner.is_some() && cpg_defs.contains(&key)})
                })
                .collect();
            let slots = parsed.function_parameter_slot_occurrences(&function).map(|slots|slots.into_iter().map(|(name,start,end)|json!({"name":name,"start_byte":start,"end_byte":end})).collect::<Vec<_>>());
            functions.push(json!({"file":file,"start_byte":function.start_byte(),"end_byte":function.end_byte(),
                "start_line":start_line,"owner_name":owner,"kind":function.kind(),
                "parameter_recovery":params.is_some_and(recovery),"parameters":parameters,"occurrences":occurrences,"slots":slots}));
        }
    }
    let manifest: Vec<_> = files.iter().map(|(file,parsed)|json!({"file":file,"sha256":hashes[file],"language":parsed.language,"parse_errors":parsed.parse_error_count})).collect();
    json!({"schema":"prism.parameter-site-census/1","authorizes_runtime_edge":false,
        "files":manifest,"skipped":skipped,"functions":functions,"flows":flows})
}

fn main() -> Result<()> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    ensure!(args.len() == 1, "expected one absolute snapshot root");
    let root = Path::new(&args[0]);
    ensure!(root.is_absolute(), "expected absolute snapshot root");
    let root = root.canonicalize()?;
    ensure!(
        root.parent().is_some() && root.is_dir(),
        "invalid snapshot root"
    );
    let loaded = prism::repo_loader::load_repo(&root)?;
    let files = loaded
        .files
        .into_iter()
        .filter(|(_, p)| {
            matches!(
                p.language,
                Language::JavaScript | Language::TypeScript | Language::Tsx
            )
        })
        .collect::<BTreeMap<_, _>>();
    ensure!(!files.is_empty(), "empty supported source population");
    println!(
        "{}",
        observe(&files, &loaded.file_hashes, json!(loaded.skipped))
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn run(source: &str) -> Value {
        let p = ParsedFile::parse("case.ts", source, Language::TypeScript).unwrap();
        assert_eq!(p.parse_error_count, 0);
        observe(
            &BTreeMap::from([("case.ts".into(), p)]),
            &BTreeMap::from([("case.ts".into(), "0".repeat(64))]),
            json!([]),
        )
    }
    #[test]
    fn classifies_syntax_without_compressing_default_and_pattern_forms() {
        let r=run("function f(a:any, b?:any, c:any=0, ...rest:any[]) {}\nfunction g({x}:any,[y]:any) {}\nconst h = x => x;");
        let forms: Vec<_> = r["functions"]
            .as_array()
            .unwrap()
            .iter()
            .flat_map(|f| f["parameters"].as_array().unwrap())
            .map(|p| p["form"].as_str().unwrap())
            .collect();
        assert_eq!(
            forms,
            vec![
                "required_identifier",
                "optional_identifier",
                "default_identifier",
                "rest",
                "destructured",
                "destructured",
                "unparenthesized_identifier"
            ]
        );
    }
    #[test]
    fn separates_properties_decorators_and_erased_this() {
        let r=run("class C { constructor(public a:any, readonly b:any, @inject c:any){} }\nfunction f(this:void,a:any){}");
        let forms: Vec<_> = r["functions"]
            .as_array()
            .unwrap()
            .iter()
            .flat_map(|f| f["parameters"].as_array().unwrap())
            .map(|p| p["form"].as_str().unwrap())
            .collect();
        assert_eq!(
            forms,
            vec![
                "parameter_property",
                "parameter_property",
                "decorated",
                "erased_this",
                "required_identifier"
            ]
        );
    }
    #[test]
    fn exact_occurrences_are_not_syntax_counts_or_field_only_defs() {
        let source = "function f(\n/* Ω */ a:any, b:any) { sink(a); sink(b.field); }";
        let r = run(source);
        let f = &r["functions"][0];
        assert_eq!(f["occurrences"].as_array().unwrap().len(), 2);
        assert_eq!(f["occurrences"][0]["dfg_def"], true);
        assert_eq!(f["occurrences"][1]["dfg_def"], false);
        for o in f["occurrences"].as_array().unwrap() {
            assert_eq!(
                &source[o["start_byte"].as_u64().unwrap() as usize
                    ..o["end_byte"].as_u64().unwrap() as usize],
                o["name"].as_str().unwrap()
            );
        }
    }
    #[test]
    fn flow_endpoints_retain_parameter_tokens_without_body_fallback() {
        let r=run("function take(a:any) { sink(a); }\nfunction fallback(a:any=0) {\n a=clean();\n sink(a);\n}\nfunction run(v:any) {\n take(v);\n fallback(v);\n}");
        let functions = r["functions"].as_array().unwrap();
        for name in ["take", "fallback"] {
            let f = functions.iter().find(|f| f["owner_name"] == name).unwrap();
            let flows: Vec<_> = r["flows"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|e| e["to"]["function"] == name)
                .collect();
            // Both plain and bounded literal-default parameters now bind; a
            // body assignment still cannot substitute for the signature token.
            assert_eq!(flows.len(), 1);
            assert_eq!(flows[0]["to"]["start_byte"], f["slots"][0]["start_byte"]);
            assert_eq!(flows[0]["to"]["end_byte"], f["slots"][0]["end_byte"]);
        }
    }
    #[test]
    fn duplicate_lists_preserve_syntax_but_gain_no_occurrences() {
        let r = run("function f(a:any,a:any){sink(a);}");
        assert_eq!(r["functions"][0]["parameters"].as_array().unwrap().len(), 2);
        assert_eq!(r["functions"][0]["occurrences"], json!([]));
        assert_eq!(r["functions"][0]["slots"], Value::Null);
        assert_eq!(r["authorizes_runtime_edge"], false);
    }

    #[test]
    fn classifiers_keep_javascript_patterns_and_recovery_distinct() {
        for (source, language, expected) in [
            (
                "function f(a,b=0,...rest){}",
                Language::JavaScript,
                vec!["javascript_identifier", "default_identifier", "rest"],
            ),
            (
                "function f({x}:any={},[y]?:any){}",
                Language::TypeScript,
                vec!["default_pattern", "optional_pattern"],
            ),
            (
                "function f(a: ){}",
                Language::TypeScript,
                vec!["required_identifier", "recovery"],
            ),
        ] {
            let parsed = ParsedFile::parse("case", source, language).unwrap();
            let function = parsed.all_functions()[0];
            let params = function.child_by_field_name("parameters").unwrap();
            let mut cursor = params.walk();
            let actual: Vec<_> = params
                .named_children(&mut cursor)
                .map(|p| form(p, false))
                .collect();
            assert_eq!(actual, expected, "{source}: {}", params.to_sexp());
            if parsed.parse_error_count > 0 {
                assert!(recovery(params));
                assert!(parsed.function_parameter_occurrences(&function).is_empty());
            }
        }
    }
}
