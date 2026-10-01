#[test]
fn d15_executable_barrel_escape_keeps_exact() {
    for ext in ["jsx", "tsx"] {
        let lib = "import { setG } from './impl';\nfunction f(){}\nsetG(f);\nexport { g as f } from './impl';";
        let cg = graph(
            ext,
            &[
                ("lib", lib),
                (
                    "impl",
                    "export let g = null; export function setG(x){ g = x; }",
                ),
                ("app", APP),
            ],
        );
        // E5 also keeps the row; independently guard the non-escape premise.
        assert!(!cg.js_ts_exports[&format!("lib.{ext}")].namespace_private_barrel);
        assert_eq!(rows(&cg, "run", "f"), [exact(ext, "lib", "f", 2)]);
    }
}

#[test]
fn d16_e5a_bare_maycall_keeps_base() {
    for ext in ["jsx", "tsx"] {
        let cg = graph(
            ext,
            &[
                (
                    "lib",
                    "function wrap(x){return x;}\nexport const f=wrap(function f(){});",
                ),
                ("app", &APP.replace("'./lib'", "'pkg/lib'")),
            ],
        );
        let terminal = &cg.js_ts_namespace_exports[&format!("lib.{ext}")]["f"];
        assert_eq!(
            (&terminal.file, &terminal.local_name, terminal.span),
            (&format!("lib.{ext}"), &"f".to_string(), None)
        );
        assert_eq!(rows(&cg, "run", "f"), [exact(ext, "lib", "f", 2)]);
    }
}

#[test]
fn d16_e5b_bare_written_keeps_base() {
    for ext in ["jsx", "tsx"] {
        let cg = graph(
            ext,
            &[
                ("lib", "export function f(){}; f=o;"),
                ("app", &APP.replace("'./lib'", "'pkg/lib'")),
            ],
        );
        assert_eq!(rows(&cg, "run", "f"), [exact(ext, "lib", "f", 1)]);
    }
}

#[test]
fn d16_e5c_star_written_keeps_base() {
    for ext in ["jsx", "tsx"] {
        let cg = graph(
            ext,
            &[
                (
                    "lib",
                    "function holder(){function f(){}}\nexport * from './impl';",
                ),
                ("impl", "export function f(){}; f=o;"),
                ("app", APP),
            ],
        );
        assert!(cg.js_ts_exports[&format!("lib.{ext}")].namespace_private_barrel);
        assert_eq!(rows(&cg, "run", "f"), [exact(ext, "lib", "f", 1)]);
    }
}

#[test]
fn d16_e5e_named_maycall_keeps_base() {
    for ext in ["jsx", "tsx"] {
        for barrel in [
            "export {f} from './impl';",
            "import {f as alias} from './impl'; export {alias as f};",
        ] {
            let cg = graph(
                ext,
                &[
                    ("lib", &format!("function f(){{}}\n{barrel}")),
                    (
                        "impl",
                        "function wrap(x){return x;}\nexport const f=wrap(function f(){});",
                    ),
                    ("app", APP),
                ],
            );
            let terminal = &cg.js_ts_namespace_exports[&format!("lib.{ext}")]["f"];
            assert_eq!(terminal.file, format!("impl.{ext}"));
            assert_eq!(rows(&cg, "run", "f"), [exact(ext, "lib", "f", 1)]);
        }
    }
}

#[test]
fn d17_written_kinds_keep_base() {
    for ext in ["jsx", "tsx"] {
        for declaration in [
            "export class f {}",
            "export let f=0;",
            "export const f=()=>1;",
        ] {
            for (file, barrel) in [
                ("lib", None),
                (
                    "impl",
                    Some("function h(){function f(){}}\nimport {f as alias} from './impl'; export {alias as f};"),
                ),
            ] {
                let lib = format!("function h(){{function f(){{}}}}\n{declaration}\nf=o;");
                let mut sources = vec![(file, lib.as_str()), ("app", APP)];
                if let Some(barrel) = barrel {
                    sources.push(("lib", barrel));
                }
                let cg = graph(ext, &sources);
                if barrel.is_none() && declaration.contains("=>") {
                    assert_eq!(
                        rows(&cg, "run", "f"),
                        [format!(
                            "{}, {}",
                            exact(ext, "lib", "f", 1),
                            exact(ext, "lib", "f", 2)
                        )],
                        "{declaration}"
                    );
                } else {
                    assert_eq!(
                        rows(&cg, "run", "f"),
                        [exact(ext, "lib", "f", 1)],
                        "{declaration}"
                    );
                }
            }
        }
    }
}

#[test]
fn d18_e5_origin_serde_and_incremental_epochs() {
    use prism::cpg::CodePropertyGraph;
    use std::collections::BTreeSet;
    for ext in ["jsx", "tsx"] {
        let app = APP.replace("'./lib'", "'pkg/lib'");
        let mut previous: Option<CodePropertyGraph> = None;
        for (lib, grade) in [
            (
                "function wrap(x){return x;}\nexport const f=wrap(function f(){});",
                "Exact",
            ),
            (
                "function make(){\n function f(){}\n return f;\n}\nexport const f=make();",
                "NameOnly",
            ),
            (
                "function wrap(x){return x;}\nexport const f=wrap(function f(){});",
                "Exact",
            ),
        ] {
            let files = parsed(ext, &[("lib", lib), ("app", &app)]);
            let full = CodePropertyGraph::build(&files);
            let current = if let Some(old) = previous {
                CodePropertyGraph::build_incremental(
                    old.call_graph,
                    old.dfg,
                    &BTreeSet::from([format!("lib.{ext}")]),
                    &files,
                    None,
                )
            } else {
                CodePropertyGraph::build(&files)
            };
            assert_eq!(
                current.call_graph.js_ts_namespace_exports,
                full.call_graph.js_ts_namespace_exports
            );
            assert_eq!(
                rows(&current.call_graph, "run", "f"),
                [format!("lib.{ext}:f@2-2 {grade}/import_qualified")]
            );
            let facts = &current.call_graph.js_ts_exports[&format!("lib.{ext}")];
            assert_eq!(
                facts.namespace_may_call_locals.contains("f"),
                grade == "Exact"
            );
            let bytes = bincode::serialize(&current.call_graph).unwrap();
            let back: CallGraph = bincode::deserialize(&bytes).unwrap();
            assert_eq!(
                rows(&back, "run", "f"),
                rows(&current.call_graph, "run", "f")
            );
            let mut old_facts = serde_json::to_value(facts).unwrap();
            old_facts
                .as_object_mut()
                .unwrap()
                .remove("namespace_may_call_locals");
            let defaulted: prism::js_exports::JsExportFacts =
                serde_json::from_value(old_facts).unwrap();
            assert!(defaulted.namespace_may_call_locals.is_empty());
            previous = Some(current);
        }
    }
}
