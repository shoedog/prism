//! Slice V: where the returned-value route does not produce a proven hit, the site keeps the
//! base's result (same rungs, same outcome, same drop reason), never a new drop and never a new
//! wrong edge. Each expectation below is the record set slice L's head (the base of this slice)
//! gives for the same source, captured by running these fixtures against that tree; the head
//! must equal it. Each case runs twice: the binding and the call in one function (the base's
//! lexical guard drops it) and the binding in an outer function with the call in a nested one
//! (the base's name search reaches the same-named decoy).
use super::js_alias_follow_test::graph_n;
use super::js_returned_follow_test::rec;

const DECOY: &str = "export function save() {\n  return 2;\n}\n\
    export function run() {\n  return 3;\n}\nexport function other() {\n  return 4;\n}\n";

const SPANLESS: &str =
    "let w;\nw = compute();\nexport { w };\nfunction other() {\n  function w() {}\n}\n";
const HOOK_HEAD: &str = "function a() {\n  return 1;\n}\nfunction b() {\n  return 2;\n}\n";
const IMPORT: &str = "import { useThing } from './hooks';";
const DESTRUCTURE: &str = "const { save } = useThing();";

struct Spec {
    label: &'static str,
    /// The hook-side files (the caller and the decoy are added).
    files: Vec<(&'static str, String)>,
    import: &'static str,
    decl: &'static str,
    /// The enclosing function's parameter list (the hook is a parameter in one case).
    params: &'static str,
}

fn hooks(body: &str) -> Vec<(&'static str, String)> {
    vec![(
        "hooks.ts",
        format!("{HOOK_HEAD}export function useThing(c, p) {{\n{body}\n}}\n"),
    )]
}

fn spec(label: &'static str, files: Vec<(&'static str, String)>, decl: &'static str) -> Spec {
    Spec {
        label,
        files,
        import: IMPORT,
        decl,
        params: "",
    }
}

fn specs() -> Vec<Spec> {
    vec![
        spec(
            "divergent returns",
            hooks("  if (c) return { save: a };\n  return { save: b };"),
            DESTRUCTURE,
        ),
        spec(
            "spread after the key",
            hooks("  return { save: a, ...p };"),
            DESTRUCTURE,
        ),
        spec(
            "parameter value",
            hooks("  return { save: p };"),
            DESTRUCTURE,
        ),
        spec(
            "computed key",
            hooks("  return { [c]: 1, save: a };"),
            DESTRUCTURE,
        ),
        spec(
            "getter in the literal",
            hooks("  return { get x() { return 1; }, save: a };"),
            DESTRUCTURE,
        ),
        spec(
            "duplicate key",
            hooks("  return { save: a, save: b };"),
            DESTRUCTURE,
        ),
        spec(
            "key not returned",
            hooks("  return { other: a };"),
            DESTRUCTURE,
        ),
        spec(
            "hook returns a call result",
            hooks("  return { save: a() };"),
            DESTRUCTURE,
        ),
        spec(
            "two returned-value steps",
            vec![(
                "hooks.ts",
                format!(
                    "{HOOK_HEAD}function inner() {{\n  return {{ save: a }};\n}}\n\
                     export function useThing() {{\n  const {{ save }} = inner();\n  \
                     return {{ save }};\n}}\n"
                ),
            )],
            DESTRUCTURE,
        ),
        spec(
            "async hook",
            vec![(
                "hooks.ts",
                format!(
                    "{HOOK_HEAD}export async function useThing() {{\n  return {{ save: a }};\n}}\n"
                ),
            )],
            DESTRUCTURE,
        ),
        Spec {
            import: "import { useThing } from 'some-hooks';",
            ..spec("package hook", vec![], DESTRUCTURE)
        },
        Spec {
            import: "import { useThing } from './missing';",
            ..spec("unresolved relative hook module", vec![], DESTRUCTURE)
        },
        Spec {
            import: "",
            params: "useThing",
            ..spec("hook is a parameter", vec![], DESTRUCTURE)
        },
        spec(
            "value is an import of a span-less export",
            vec![
                (
                    "hooks.ts",
                    "import { w } from './producer';\nexport function useThing() {\n  \
                     return { save: w };\n}\n"
                        .to_string(),
                ),
                ("producer.ts", SPANLESS.to_string()),
            ],
            DESTRUCTURE,
        ),
        spec(
            "value is a namespace member alias of a span-less export",
            vec![
                (
                    "hooks.ts",
                    "import * as U from './producer';\nconst viaAlias = U.w;\n\
                     export function useThing() {\n  return { save: viaAlias };\n}\n"
                        .to_string(),
                ),
                ("producer.ts", SPANLESS.to_string()),
            ],
            DESTRUCTURE,
        ),
        spec(
            "let binding",
            hooks("  return { save: a };"),
            "let { save } = useThing();",
        ),
        spec(
            "reassigned binding",
            hooks("  return { save: a };"),
            "let { save } = useThing();\nsave = null;",
        ),
        spec(
            "awaited call",
            hooks("  return { save: a };"),
            "const { save } = await useThing();",
        ),
        Spec {
            import: "import * as H from './hooks';",
            ..spec(
                "member call on the hook",
                hooks("  return { save: a };"),
                "const { save } = H.useThing();",
            )
        },
        spec(
            "rest in the destructure",
            hooks("  return { save: a };"),
            "const { save, ...more } = useThing();",
        ),
        spec(
            "nested array pattern",
            hooks("  return [a, b];"),
            "const [[x], save] = useThing();",
        ),
        spec(
            "array spread before the slot",
            hooks("  return [...p, a];"),
            "const [x, save] = useThing();",
        ),
        Spec {
            import: "import { useThing } from './index';",
            ..spec(
                "two star exports of the hook name",
                vec![
                    ("hooks.ts", hooks("  return { save: a };").remove(0).1),
                    ("hooks2.ts", hooks("  return { save: b };").remove(0).1),
                    (
                        "index.ts",
                        "export * from './hooks';\nexport * from './hooks2';\n".to_string(),
                    ),
                ],
                DESTRUCTURE,
            )
        },
    ]
}

/// The record set the base gives for `label`, with the binding and the call in one function
/// or in nested ones, captured from slice L's head.
fn base_records(label: &str, nested: bool) -> &'static [&'static str] {
    match (label, nested) {
        ("divergent returns", false) => &["L4 save Source: drop UnknownName"],
        ("divergent returns", true) => &["L5 save Source: decoy.ts:save@1-3 free_single exact"],
        ("spread after the key", false) => &["L4 save Source: drop UnknownName"],
        ("spread after the key", true) => &["L5 save Source: decoy.ts:save@1-3 free_single exact"],
        ("parameter value", false) => &["L4 save Source: drop UnknownName"],
        ("parameter value", true) => &["L5 save Source: decoy.ts:save@1-3 free_single exact"],
        ("computed key", false) => &["L4 save Source: drop UnknownName"],
        ("computed key", true) => &["L5 save Source: decoy.ts:save@1-3 free_single exact"],
        ("getter in the literal", false) => &["L4 save Source: drop UnknownName"],
        ("getter in the literal", true) => &["L5 save Source: decoy.ts:save@1-3 free_single exact"],
        ("duplicate key", false) => &["L4 save Source: drop UnknownName"],
        ("duplicate key", true) => &["L5 save Source: decoy.ts:save@1-3 free_single exact"],
        ("key not returned", false) => &["L4 save Source: drop UnknownName"],
        ("key not returned", true) => &["L5 save Source: decoy.ts:save@1-3 free_single exact"],
        ("hook returns a call result", false) => &["L4 save Source: drop UnknownName"],
        ("hook returns a call result", true) => {
            &["L5 save Source: decoy.ts:save@1-3 free_single exact"]
        }
        ("two returned-value steps", false) => &["L4 save Source: drop UnknownName"],
        ("two returned-value steps", true) => {
            &["L5 save Source: decoy.ts:save@1-3 free_single exact"]
        }
        ("async hook", false) => &["L4 save Source: drop UnknownName"],
        ("async hook", true) => &["L5 save Source: decoy.ts:save@1-3 free_single exact"],
        ("package hook", false) => &["L4 save Source: drop UnknownName"],
        ("package hook", true) => &["L5 save Source: decoy.ts:save@1-3 free_single exact"],
        ("unresolved relative hook module", false) => &["L4 save Source: drop UnknownName"],
        ("unresolved relative hook module", true) => {
            &["L5 save Source: decoy.ts:save@1-3 free_single exact"]
        }
        ("hook is a parameter", false) => &["L4 save Source: drop UnknownName"],
        ("hook is a parameter", true) => &["L5 save Source: decoy.ts:save@1-3 free_single exact"],
        ("value is an import of a span-less export", false) => {
            &["L4 save Source: drop UnknownName"]
        }
        ("value is an import of a span-less export", true) => {
            &["L5 save Source: decoy.ts:save@1-3 free_single exact"]
        }
        ("value is a namespace member alias of a span-less export", false) => {
            &["L4 save Source: drop UnknownName"]
        }
        ("value is a namespace member alias of a span-less export", true) => {
            &["L5 save Source: decoy.ts:save@1-3 free_single exact"]
        }
        ("let binding", false) => &["L4 save Source: drop UnknownName"],
        ("let binding", true) => &["L5 save Source: decoy.ts:save@1-3 free_single exact"],
        ("reassigned binding", false) => &["L5 save Source: drop UnknownName"],
        ("reassigned binding", true) => &["L6 save Source: decoy.ts:save@1-3 free_single exact"],
        ("awaited call", false) => &["L4 save Source: drop UnknownName"],
        ("awaited call", true) => &["L5 save Source: decoy.ts:save@1-3 free_single exact"],
        ("member call on the hook", false) => &["L4 save Source: drop UnknownName"],
        ("member call on the hook", true) => {
            &["L5 save Source: decoy.ts:save@1-3 free_single exact"]
        }
        ("rest in the destructure", false) => &["L4 save Source: drop UnknownName"],
        ("rest in the destructure", true) => {
            &["L5 save Source: decoy.ts:save@1-3 free_single exact"]
        }
        ("nested array pattern", false) => &["L4 save Source: drop UnknownName"],
        ("nested array pattern", true) => &["L5 save Source: decoy.ts:save@1-3 free_single exact"],
        ("array spread before the slot", false) => &["L4 save Source: drop UnknownName"],
        ("array spread before the slot", true) => {
            &["L5 save Source: decoy.ts:save@1-3 free_single exact"]
        }
        ("two star exports of the hook name", false) => &["L4 save Source: drop UnknownName"],
        ("two star exports of the hook name", true) => {
            &["L5 save Source: decoy.ts:save@1-3 free_single exact"]
        }
        _ => panic!("no base record for {label}"),
    }
}

fn caller_source(s: &Spec, nested: bool) -> String {
    let kw = if s.decl.contains("await") {
        "async "
    } else {
        ""
    };
    let decl = s.decl.replace('\n', "\n  ");
    if nested {
        format!(
            "{}\nexport {kw}function outer({}) {{\n  {decl}\n  function main() {{\n    return save();\n  }}\n  \
             return main;\n}}\n",
            s.import, s.params
        )
    } else {
        format!(
            "{}\nexport {kw}function main({}) {{\n  {decl}\n  return save();\n}}\n",
            s.import, s.params
        )
    }
}

#[test]
fn every_unproven_returned_value_site_equals_the_base() {
    let mut wrong = Vec::new();
    for s in specs() {
        for nested in [false, true] {
            let caller = caller_source(&s, nested);
            let mut files: Vec<(&str, &str)> =
                s.files.iter().map(|(p, t)| (*p, t.as_str())).collect();
            files.push(("caller.ts", &caller));
            files.push(("decoy.ts", DECOY));
            let mut cg = graph_n(&files);
            let got = rec(&cg, "caller.ts", "save");
            // With every returned-value fact removed, a site the facts did not prove must not
            // change: an unproven route is no route.
            cg.js_ts_return_facts.clear();
            assert_eq!(
                rec(&cg, "caller.ts", "save"),
                got,
                "{}: removing the facts changed an unproven site",
                s.label
            );
            let scope = if nested { "nested" } else { "one function" };
            if std::env::var("ALIAS_PRINT").is_ok() {
                eprintln!("CASE {}|{scope}|{got:?}", s.label);
            } else if got != base_records(s.label, nested) {
                let main = base_records(s.label, nested);
                wrong.push(format!("{} ({scope}): got {got:?}, want {main:?}", s.label));
            }
        }
    }
    assert!(
        wrong.is_empty(),
        "differs from the base:\n{}",
        wrong.join("\n")
    );
}

/// The indirect (textual function-pointer) records the audit seam gives for `save` calls.
fn textual_records(files: &[(&str, &str)], stand_down: bool) -> Vec<String> {
    use prism::ast::ParsedFile;
    use prism::languages::Language;
    let parsed: std::collections::BTreeMap<String, ParsedFile> = files
        .iter()
        .map(|(p, s)| {
            let lang = Language::from_path(p).unwrap();
            (p.to_string(), ParsedFile::parse(p, s, lang).unwrap())
        })
        .collect();
    let cg = prism::call_graph::CallGraph::build(&parsed);
    let mut out: Vec<String> = cg
        .audit_indirect_records(&parsed, stand_down)
        .iter()
        .map(|(caller, site)| format!("{} -> {}", caller.name, site.callee_name))
        .collect();
    out.sort();
    out
}

#[test]
fn the_textual_path_stands_down_for_a_returned_value_site_only_on_a_proven_hit() {
    // A line `save = target;` inside a template literal of the calling function is text the
    // function-pointer path reads as an assignment of `save` (main emits an indirect record to
    // `target` for it). The binding sits in the outer function, so the declaration line is not in
    // the caller's text. The stand-down must remove that record for a proven returned-value hit
    // and keep it everywhere else.
    let decoy = "export function target() {\n  return 9;\n}\n";
    let caller = "import { useThing } from './hooks';\nexport function outer() {\n  \
                  const { save } = useThing();\n  function inner() {\n    const note = `\nsave = target;\n`;\n    \
                  return save();\n  }\n  return inner;\n}\n";
    let proven =
        "function a() {\n  return 1;\n}\nexport function useThing() {\n  return { save: a };\n}\n";
    let divergent = "function a() {\n  return 1;\n}\nfunction b() {\n  return 2;\n}\n\
                     export function useThing(c) {\n  if (c) return { save: a };\n  return { save: b };\n}\n";
    for (label, hooks, held) in [("proven", proven, false), ("divergent", divergent, true)] {
        let files = [
            ("hooks.ts", hooks),
            ("caller.ts", caller),
            ("decoy.ts", decoy),
        ];
        let with = textual_records(&files, true);
        let without = textual_records(&files, false);
        assert_eq!(
            without,
            ["inner -> target"],
            "{label}: main's textual path reads the template line"
        );
        assert_eq!(
            with,
            if held { without } else { Vec::<String>::new() },
            "{label}: the stand-down follows the predicate"
        );
    }
}
