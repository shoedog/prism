//! W1: whole computed names, never individual string leaves. Static fixtures only.
use super::*;

fn exact(source: &str, ext: &str) -> bool {
    let fs = files(source, ext);
    assert!(
        !fs.values().next().unwrap().tree.root_node().has_error(),
        "{source}"
    );
    let cg = CallGraph::build(&fs);
    let sites: Vec<_> = cg
        .calls
        .values()
        .flatten()
        .filter(|s| is_anonymous_owner_name(&s.caller.name) && s.callee_name == "f")
        .collect();
    assert_eq!(sites.len(), 1, "{source}");
    cg.resolve_call_site_full(sites[0])
        .resolved
        .iter()
        .any(|r| r.confidence == ResolutionConfidence::Exact)
}
fn script(suffix: &str) -> String {
    format!(
        "function f() {{return 1;}}\nfunction evil() {{return 2;}}\n{suffix}\n[0].map(() => f());"
    )
}
#[test]
fn computed_eval_concatenation_loses_exact() {
    for ext in ["js", "ts", "tsx"] {
        assert!(!exact(
            &script("globalThis[\"e\" + \"val\"]('f=evil');"),
            ext
        ));
    }
}
#[test]
fn computed_eval_unknown_name_loses_exact() {
    for ext in ["js", "ts", "tsx"] {
        assert!(!exact(
            &script("const k = 'ev' + 'al'; globalThis[k]('f=evil');"),
            ext
        ));
    }
}
#[test]
fn computed_eval_constants_and_unknown_controls() {
    for ext in ["js", "ts", "tsx"] {
        for index in [
            "'eval'",
            "`eval`",
            "('e' + ('v' + 'al'))",
            "`e${'v'}al`",
            "`e` + `val`",
            "true ? 'eval' : 'log'",
            "`e${unknown}al`",
            "'e\\x76al'",
        ] {
            assert!(
                !exact(&script(&format!("globalThis[{index}]('f=evil');")), ext),
                "{ext}: {index}"
            );
        }
        for suffix in [
            "",
            "globalThis['foo']('f=evil');",
            "globalThis['eval' + 'uate']();",
            "globalThis[`log${'ger'}`]();",
            "globalThis[('lo' + 'g')]();",
            "eval('f=evil');",
        ] {
            assert_eq!(
                exact(&script(suffix), ext),
                !suffix.starts_with("eval("),
                "{ext}: {suffix}"
            );
        }
        if ext != "js" {
            for index in [
                "('e' as string) + ('val' satisfies string)",
                "(('eval')!)",
                "('eval' as const)",
            ] {
                assert!(
                    !exact(&script(&format!("globalThis[{index}]('f=evil');")), ext),
                    "{ext}: {index}"
                );
            }
            assert!(exact(&script("globalThis[('log' as string)!]();"), ext));
        }
    }
    assert!(!exact(
        &script("globalThis[<string>('e' + 'val')]('f=evil');"),
        "ts"
    ));
}
#[test]
fn computed_eval_in_target_and_reexport_dependencies() {
    for ext in ["js", "ts", "tsx"] {
        for changed in ["lib", "barrel"] {
            for suffix in [
                "globalThis['e' + 'val']('f=evil');",
                "const k='ev'+'al';globalThis[k]('f=evil');",
            ] {
                let mut fs = BTreeMap::new();
                for (name, source) in [
                    ("a", "import {f} from './barrel'; sink(() => f());"),
                    ("barrel", "export {f} from './lib';"),
                    ("lib", "export function f(){return 1;}"),
                ] {
                    let path = format!("{name}.{ext}");
                    let src = format!("{source}\n{}", if name == changed { suffix } else { "" });
                    let p = ParsedFile::parse(&path, &src, Language::from_path(&path).unwrap())
                        .unwrap();
                    assert!(!p.tree.root_node().has_error());
                    fs.insert(path, p);
                }
                let cg = CallGraph::build(&fs);
                let site = cg
                    .calls
                    .values()
                    .flatten()
                    .find(|s| is_anonymous_owner_name(&s.caller.name) && s.callee_name == "f")
                    .unwrap();
                assert!(
                    !cg.resolve_call_site_full(site)
                        .resolved
                        .iter()
                        .any(|r| r.confidence == ResolutionConfidence::Exact),
                    "{ext}/{changed}/{suffix}"
                );
            }
        }
    }
}

#[test]
fn computed_unknown_global_receivers_and_aliases_keep_ordinary_recall() {
    for ext in ["js", "ts", "tsx"] {
        for receiver in ["globalThis", "window", "self", "global", "this"] {
            for prefix in ["", "const a=RECEIVER; const b=(a);"] {
                let prefix = prefix.replace("RECEIVER", receiver);
                let selected = if prefix.is_empty() { receiver } else { "b" };
                let suffix = format!("{prefix} const k='e'+'val'; {selected}[k]('f=evil');");
                assert!(!exact(&script(&suffix), ext), "{ext}: {suffix}");
            }
        }
        for suffix in [
            "const a=obj; a[k]();",
            "const a=b; const b=a; a[k]();",
            "const a=globalThis; function ordinary(a){a[k]();}",
            "function ordinary(){this[k]();}",
            "class C { method(){this[k]();} }",
            "const a=globalThis; {const a={};a[k]();}",
        ] {
            assert!(exact(&script(suffix), ext), "{ext}: {suffix}");
        }
        for suffix in [
            "obj['e' + 'val']();",
            "const a=globalThis; (() => {a[k]();})();",
            "(() => {this[k]();})();",
        ] {
            assert!(!exact(&script(suffix), ext), "{ext}: {suffix}");
        }
        if ext != "js" {
            assert!(!exact(
                &script("const a=(globalThis as object); a[k]();"),
                ext
            ));
        }
    }
}

#[test]
fn computed_literal_keys_and_shadowed_host_names_are_safe() {
    for ext in ["js", "ts", "tsx"] {
        for suffix in [
            "globalThis[0]; globalThis[0xff]; globalThis[false]; globalThis[null];",
            "function ordinary(global){global[k];}",
            "const window={}; window[k];",
            "const self=[]; self[k];",
            "const globalThis={}; const g=globalThis; g[k];",
        ] {
            assert!(exact(&script(suffix), ext), "{ext}: {suffix}");
        }
        assert!(!exact(&script("const window=globalThis; window[k];"), ext));
    }
}

#[test]
fn computed_class_keys_use_program_this_but_initializers_do_not() {
    for ext in ["js", "ts", "tsx"] {
        for suffix in [
            "class C { [this[k]('f=evil')](){} }",
            "class C { [this[k]('f=evil')]=1; }",
            "const obj={ [this[k]('f=evil')](){} };",
            "class C extends this[k]('f=evil') {}",
        ] {
            assert!(!exact(&script(suffix), ext), "{ext}: {suffix}");
        }
        assert!(exact(&script("class C { field=this[k]; }"), ext));
        if ext != "js" {
            for suffix in [
                "@dec(this[k]('f=evil')) class C {}",
                "class C { @dec(this[k]('f=evil')) field=1; }",
                "class C { method(@dec(this[k]('f=evil')) p){} }",
            ] {
                assert!(!exact(&script(suffix), ext), "{ext}: {suffix}");
            }
            assert!(exact(&script("@dec(function(){this[k];}) class C {}"), ext));
        }
    }
}

#[test]
fn computed_host_program_var_and_ambient_declarations_are_not_shadows() {
    for ext in ["js", "ts", "tsx"] {
        for host in ["globalThis", "window", "self", "global"] {
            let suffix = format!("var {host}; const k='ev'+'al';{host}[k]('f=evil');");
            assert!(!exact(&script(&suffix), ext), "{ext}: {suffix}");
            for suffix in [
                format!("function ordinary(){{var {host};{host}[k];}}"),
                format!("let {host};{host}[k];"),
                format!("function {host}(){{}} {host}[k];"),
                format!("class {host}{{}} {host}[k];"),
            ] {
                assert!(exact(&script(&suffix), ext), "{ext}: {suffix}");
            }
            if ext != "js" {
                for declaration in [
                    format!("namespace {host} {{ export const marker=1; }}"),
                    format!("enum {host} {{ Marker }}"),
                ] {
                    let suffix = format!("{declaration} const k='ev'+'al';{host}[k]('f=evil');");
                    assert!(!exact(&script(&suffix), ext), "{ext}: {suffix}");
                }
                for prefix in ["declare var", "declare const"] {
                    let suffix =
                        format!("{prefix} {host}:any; const k='ev'+'al';{host}[k]('f=evil');");
                    assert!(!exact(&script(&suffix), ext), "{ext}: {suffix}");
                }
            }
        }
    }
}
