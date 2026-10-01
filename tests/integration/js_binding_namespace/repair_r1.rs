const CROSS_BAR: &str =
    "function f(){return 1;} export {f as rootFn}; export {g as f} from './impl';";
const CROSS_IMPL: &str = "import {rootFn} from './lib'; const g=rootFn; export {g};";
fn exact(ext: &str, file: &str, name: &str, line: usize) -> String {
    format!("{file}.{ext}:{name}@{line}-{line} Exact/import_qualified")
}
// Controller re-scope 2026-10-01: review inputs are preservation rows unless
// a unique Callable identity can filter candidates base actually selected.
fn base_rows(cg: &CallGraph, member: &str) -> Vec<String> {
    let mut base = cg.clone();
    base.js_ts_namespace_exports.clear();
    for sites in base.calls.values_mut() {
        *sites = std::mem::take(sites)
            .into_iter()
            .map(|mut site| {
                if matches!(site.local_binding, JsLocalBinding::NamespaceImport { .. }) {
                    site.local_binding = JsLocalBinding::Unchecked;
                }
                site
            })
            .collect();
    }
    rows(&base, "run", member)
}
fn keeps_base(ext: &str, sources: &[(&str, &str)], member: &str) {
    let cg = graph(ext, sources);
    assert_eq!(
        rows(&cg, "run", member),
        base_rows(&cg, member),
        "{ext}: {sources:?}"
    );
}
#[test]
fn r1_opus_w1_default_hoc_and_callable_forms() {
    for ext in ["jsx", "tsx"] {
        for value in [
            "memo(Button)",
            "withRouter(Button)",
            "connect(null)(Button)",
            "(function Button(){})",
            "make()",
        ] {
            let producer = format!("function Button(){{ return <div/>; }} export default {value};");
            let app =
                "import * as UI from './components'; export function run(){ return <UI.Button/>; }";
            keeps_base(
                ext,
                &[
                    (
                        "components/index",
                        "export { default as Button } from './Button';",
                    ),
                    ("components/Button", &producer),
                    ("app", app),
                ],
                "Button",
            );
        }
        for value in [
            "(function f(){return 2;})",
            "g = function f(){return 2;}",
            "(function f(){return 2;}) as any",
        ] {
            let lib = format!(
                "function mk(){{ return function f(){{return 1;}}; }}\nexport const f = {value};"
            );
            keeps_base(ext, &[("lib", &lib), ("app", APP)], "f");
        }
        for barrel in [
            "export {default as f} from './impl';",
            "import f from './impl'; export {f};",
        ] {
            keeps_base(
                ext,
                &[
                    ("lib/index", barrel),
                    ("lib/impl", "function f(){} export default make();"),
                    ("app", APP),
                ],
                "f",
            );
        }
        // TS is reachable; JSX is the requested recovery twin.
        keeps_base(
            ext,
            &[
                ("lib/index", "import f = require('./impl'); export {f};"),
                ("lib/impl", "function f(){} export = f;"),
                ("app", APP),
            ],
            "f",
        );
    }
}
#[test]
fn r1_opus_w2_revisited_file_depth_cut() {
    for ext in ["jsx", "tsx"] {
        keeps_base(
            ext,
            &[
                (
                    "lib/index",
                    "export {g as f} from './a'; export {f as h} from './c';",
                ),
                ("lib/a", "export {h as g} from './index';"),
                ("lib/c", "export function f(){}"),
                ("app", APP),
            ],
            "f",
        );
    }
}
#[test]
fn r1_rule4_never_add_target_or_enter_skipped_r3() {
    for ext in ["jsx", "tsx"] {
        for app in [
            "import * as ns from './lib'; export function run(a=ns.f()){var ns;return a;}",
            r"import * as ns from './l\u0069b'; export function run(){return ns.f();}",
        ] {
            for producer in [
                "function make(){return function f(){};} export const f=make();",
                "export function f(){}",
            ] {
                keeps_base(
                    ext,
                    &[
                        ("lib/index", "export {f} from './impl';"),
                        ("lib/impl", producer),
                        ("lib/other", "export function f(){}"),
                        ("app", app),
                    ],
                    "f",
                );
            }
        }
        // A real positive identity outside the stem candidate set cannot add.
        keeps_base(
            ext,
            &[
                (
                    "lib",
                    "function h(){function f(){}} export {f} from './impl';",
                ),
                ("impl", "export function f(){}"),
                ("app", APP),
            ],
            "f",
        );
        // A renamed positive identity cannot mint base's missing member.
        keeps_base(
            ext,
            &[
                ("lib", "function actual(){} export {actual as f};"),
                ("app", APP),
            ],
            "f",
        );
    }
}
#[test]
fn r1_rule7_e7_only_regrades_all_candidates() {
    for ext in ["jsx", "tsx"] {
        for helpers in [
            "function mk(){return function f(){};} export const g=mk();",
            "export function f(){}",
        ] {
            let cg = graph(
                ext,
                &[
                    (
                        "lib/index",
                        "function h(){function f(){}} export {g as f} from './helpers';",
                    ),
                    ("lib/helpers", helpers),
                    ("lib/other", "export function f(){}"),
                    ("app", &APP.replace("'./lib'", "'lib'")),
                ],
            );
            let expected = base_rows(&cg, "f")
                .into_iter()
                .map(|r| r.replace("Exact/", "NameOnly/"))
                .collect::<Vec<_>>();
            assert_eq!(rows(&cg, "run", "f"), expected);
        }
    }
}
#[test]
fn r1_rule2_uncertain_and_competing_stars_keep_base() {
    for ext in ["jsx", "tsx"] {
        for other in [
            "export const f=0;",
            "export * as f from './a';",
            "export * from './c';",
            "export * from './index';",
            "export * from './missing';",
            "export const unrelated=make();",
            "export function f(){}",
        ] {
            let cg = graph(
                ext,
                &[
                    ("lib/index", "export * from './a'; export * from './b';"),
                    ("lib/a", LIB),
                    ("lib/b", other),
                    ("lib/c", "export * from './d';"),
                    ("lib/d", "export function f(){}"),
                    ("app", APP),
                ],
            );
            assert!(
                !cg.js_ts_namespace_exports
                    .get(&format!("lib/index.{ext}"))
                    .is_some_and(|e| e.contains_key("f")),
                "{other}"
            );
            assert_eq!(rows(&cg, "run", "f"), base_rows(&cg, "f"), "{other}");
        }
        // The exact review inputs have no base stem candidates; uncertainty
        // must not mint a new target there either.
        for other in [
            "export const f=0;",
            "export * as f from './a';",
            "export * from './c';",
        ] {
            keeps_base(
                ext,
                &[
                    ("lib", "export * from './a'; export * from './other';"),
                    ("a", "export function f(){}"),
                    ("other", other),
                    ("c", "export * from './d';"),
                    ("d", "export function f(){}"),
                    ("app", APP),
                ],
                "f",
            );
        }
        // Direct named precedence is positive even with an unresolved star.
        let cg = graph(
            ext,
            &[
                ("lib", &format!("{LIB}export * from './missing';")),
                ("app", APP),
            ],
        );
        assert_eq!(rows(&cg, "run", "f"), [exact(ext, "lib", "f", 5)]);
    }
}
#[test]
fn r1_rule6_callable_core_only_and_cycles_bounded() {
    for ext in ["jsx", "tsx"] {
        for declaration in [
            "export namespace f {export function f(){return 7;}} f=f.f;",
            "export enum f {A}; f=o;",
            "export declare let f:any; f=o;",
            "export class f{}",
            "export const f=f(()=>1);",
            "const a=b(()=>1), b=a(()=>2); export {a as f};",
            "const f=memo(()=>1); export {f};",
        ] {
            keeps_base(
                ext,
                &[
                    (
                        "lib",
                        &format!("function h(){{function f(){{}}}}\n{declaration}"),
                    ),
                    ("app", APP),
                ],
                "f",
            );
        }
        keeps_base(ext,&[("lib", "function f(){return 17;} export {f as rootFn}; export {default as f} from './impl';"),("impl","import {rootFn} from './lib'; class g{} g=rootFn; export default g;"),("app",APP)],"f");
        // Sol W5/X6: explicit order makes the TS namespace cycle executable.
        keeps_base(ext,&[("lib", "function f(){return 23;} export namespace N {export const value=f;} export {g as f} from './impl';"),("impl","import {N} from './lib'; export const g=N.value;"),("app","import './impl'; import * as ns from './lib'; export function run(){return ns.f();}")],"f");
        // Written callable has a syntactic span but is not Callable in the core.
        let cg = graph(ext, &[("lib", &format!("{LIB}f=o;")), ("app", APP)]);
        assert_eq!(rows(&cg, "run", "f"), base_rows(&cg, "f"));
        assert!(!cg
            .js_ts_namespace_exports
            .get(&format!("lib.{ext}"))
            .is_some_and(|e| e.contains_key("f")));
    }
}

#[test]
fn r1_positive_named_star_and_import_forward_candidates() {
    for ext in ["jsx", "tsx"] {
        for barrel in [
            "export {f} from './impl';",
            "export * from './impl';",
            "import {f as alias} from './impl'; export {alias as f};",
        ] {
            let cg = graph(
                ext,
                &[
                    ("lib/index", barrel),
                    ("lib/impl", LIB),
                    ("lib/other", "export function f(){}"),
                    ("app", APP),
                ],
            );
            assert_eq!(rows(&cg, "run", "f"), [exact(ext, "lib/impl", "f", 5)]);
        }
    }
}
#[test]
fn r1_depth_budget_preserves_decoys() {
    for ext in ["jsx", "tsx"] {
        for export in ["export * from", "export {f} from"] {
            let one = format!("{export} './a';");
            let two = format!("{export} './b';");
            let three = format!("{export} './c';");
            let cg = graph(
                ext,
                &[
                    ("lib/index", &one),
                    ("lib/a", &two),
                    ("lib/b", &three),
                    ("lib/c", LIB),
                    ("app", APP),
                ],
            );
            assert_eq!(rows(&cg, "run", "f"), base_rows(&cg, "f"));
        }
    }
}
#[test]
fn r1_wrapped_nonjsx_keeps_base_jsx_filters() {
    for ext in ["jsx", "tsx"] {
        let cg = graph(
            ext,
            &[
                (
                    "lib",
                    "import {memo} from 'react';
function h(){function f(){}}
export const f=memo(()=>null);",
                ),
                (
                    "app",
                    "import * as ns from './lib'; export function run(){ns.f();return <ns.f/>;}",
                ),
            ],
        );
        let base = base_rows(&cg, "f");
        assert_eq!(
            rows(&cg, "run", "f"),
            [base[0].clone(), exact(ext, "lib", "f", 3)]
        );
    }
}
include!("repair_r2.rs");
