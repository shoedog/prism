use prism::ast::ParsedFile;
use prism::call_graph::CallGraph;
use prism::languages::Language;
use prism::resolution::{ResolutionConfidence, ResolutionKind};
use std::collections::{BTreeMap, BTreeSet};

fn files(srcs: &[(&str, &str)], language: Language) -> BTreeMap<String, ParsedFile> {
    srcs.iter()
        .map(|(path, src)| {
            (
                path.to_string(),
                ParsedFile::parse(path, src, language).unwrap(),
            )
        })
        .collect()
}

#[test]
fn exported_class_identity_is_not_a_function_export() {
    for (lang, path) in [
        (Language::JavaScript, "svc.js"),
        (Language::TypeScript, "svc.ts"),
        (Language::Tsx, "svc.tsx"),
    ] {
        let parsed = files(
            &[(
                path,
                "export class Client { m() {} }\nfunction run() { const x = new Client(); x.m(); }",
            )],
            lang,
        );
        let raw = parsed[path].extract_js_ts_export_facts();
        assert!(raw.named.contains_key("Client"), "{lang:?}: {raw:?}");
        let cg = CallGraph::build(&parsed);
        assert!(cg
            .clean_class_spans
            .contains_key(&(path.to_string(), "Client".into())));
        assert!(!cg
            .js_ts_resolved_exports
            .get(path)
            .is_some_and(|e| e.contains_key("Client")));
        check(&cg, path, Some(path), ResolutionKind::ConstructorLocal);
    }
}

#[test]
fn imported_named_class_constructor_and_parameter() {
    for (lang, ext) in [
        (Language::JavaScript, "js"),
        (Language::TypeScript, "ts"),
        (Language::Tsx, "tsx"),
    ] {
        let caller = format!("app.{ext}");
        let owner = format!("client.{ext}");
        for (body, kind) in [
            (
                "function run() { const x = new Alias(); x.m(); }",
                ResolutionKind::ConstructorLocal,
            ),
            (
                "function run(x: Alias) { x.m(); }",
                ResolutionKind::TypedParam,
            ),
        ] {
            if lang == Language::JavaScript && kind == ResolutionKind::TypedParam {
                continue;
            }
            let src = format!("import {{ Client as Alias }} from './client';\n{body}");
            let parsed = files(
                &[
                    (&caller, &src),
                    (&owner, "export class Client { m() {} }"),
                    ("decoy.ts", "class Alias { m() {} }"),
                ],
                lang,
            );
            for cg in [
                CallGraph::build(&parsed),
                CallGraph::build_direct_subset(
                    &parsed,
                    &BTreeSet::from([caller.clone(), owner.clone()]),
                ),
            ] {
                check(&cg, &caller, Some(&owner), kind);
            }
        }
    }
}

#[test]
fn class_export_conflict_cannot_become_function_import() {
    let parsed = files(
        &[
            (
                "app.ts",
                "import { Client } from './barrel'; function run() { Client(); }",
            ),
            ("class.ts", "export class Client {}"),
            ("fn.ts", "export function Client() {}"),
            (
                "barrel.ts",
                "export * from './class'; export * from './fn';",
            ),
        ],
        Language::TypeScript,
    );
    let cg = CallGraph::build(&parsed);
    let site = cg
        .calls
        .iter()
        .filter(|(id, _)| id.name == "run")
        .flat_map(|(_, sites)| sites)
        .find(|s| s.callee_name == "Client")
        .unwrap();
    let edges = cg.resolve_call_site(site);
    assert!(
        !edges
            .iter()
            .any(|e| e.confidence == ResolutionConfidence::Exact),
        "exports={:?}, edges={edges:?}",
        cg.js_ts_resolved_exports
    );
}

#[test]
fn failed_member_import_cannot_retry_global_function() {
    for module in ["export class Client {}", "export function Other() {}"] {
        let cg = CallGraph::build(&files(
            &[
                (
                    "app.ts",
                    "import { Client } from './client'; function run() { Client(); }",
                ),
                ("client.ts", module),
                ("decoy.ts", "export function Client() {}"),
            ],
            Language::TypeScript,
        ));
        let site = cg
            .calls
            .iter()
            .filter(|(id, _)| id.name == "run")
            .flat_map(|(_, sites)| sites)
            .find(|s| s.callee_name == "Client")
            .unwrap();
        assert!(cg.resolve_call_site(site).is_empty());
    }
}

#[test]
fn class_instance_slot_override_cannot_become_direct_method() {
    let mut failures = Vec::new();
    for (label, class) in [
        ("field", "class Client {\n m() {}\n m = other;\n}"),
        (
            "constructor write",
            "class Client {\n m() {}\n constructor() { this.m = other; }\n}",
        ),
        ("accessor", "class Client {\n get m() { return other; }\n}"),
        ("computed", "class Client {\n m() {}\n [key] = other;\n}"),
        (
            "computed write",
            "class Client {\n m() {}\n constructor() { this[key] = other; }\n}",
        ),
    ] {
        let source = format!("{class}\nfunction run() {{ const x = new Client(); x.m(); }}");
        let cg = CallGraph::build(&files(&[("app.ts", &source)], Language::TypeScript));
        if std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            check(&cg, "app.ts", None, ResolutionKind::ConstructorLocal)
        }))
        .is_err()
        {
            failures.push(label);
        }
    }
    assert!(failures.is_empty(), "{failures:?}");
}

#[test]
fn imported_named_class_negative_matrix() {
    let import = "import { Client as Alias } from './client';\n";
    let exported = "export class Client { m() {} }";
    let mut failures = Vec::new();
    for (label, imports, body, owner, extra) in [
        ("enclosing shadow", import, "function outer(Alias) { function run() { const x = new Alias(); x.m(); } }", exported, ""),
        ("generic shadow", import, "function run<Alias>(x: Alias) { x.m(); }", exported, ""),
        ("local type shadow", import, "function outer() { type Alias = Other; function run(x: Alias) { x.m(); } }", exported, ""),
        ("write", import, "function run() { let x = new Alias(); x = other(); x.m(); }", exported, ""),
        ("conditional", import, "function run() { if (ok) { var x = new Alias(); } x.m(); }", exported, ""),
        ("backedge", import, "function run() { let x = new Alias(); while(ok) { x.m(); x = other(); } }", exported, ""),
        ("module shadow", import, "const Alias = Other; function run() { const x = new Alias(); x.m(); }", exported, ""),
        ("duplicate import", "import { Client as Alias } from './client'; import { Client as Alias } from './other';", "function run(x: Alias) { x.m(); }", exported, ""),
        ("local default list", "import Alias from './client';", "function run(x: Alias) { x.m(); }", "class Client { m() {} } export {Client as default};", ""),
        ("type only constructor", "import type { Client as Alias } from './client';", "function run() { const x = new Alias(); x.m(); }", exported, ""),
        ("require", "const { Client: Alias } = require('./client');", "function run(x: Alias) { x.m(); }", exported, ""),
        ("not exported", import, "function run(x: Alias) { x.m(); }", "class Client { m() {} }", ""),
        ("reexport", import, "function run(x: Alias) { x.m(); }", "export { Client } from './other';", ""),
        ("static", import, "function run(x: Alias) { x.m(); }", "export class Client { static m() {} }", ""),
        ("duplicate method", import, "function run(x: Alias) { x.m(); }", "export class Client {\n m() {}\n m() {}\n}", ""),
        ("duplicate class", import, "function run(x: Alias) { x.m(); }", "export class Client { m() {} }\nexport class Client { m() {} }", ""),
        ("rebound class", import, "function run(x: Alias) { x.m(); }", "export class Client { m() {} }\nClient = Other;", ""),
        ("module ambiguity", import, "function run(x: Alias) { x.m(); }", exported, "export class Client { m() {} }"),
    ] {
        let source = format!("{imports}\n{body}");
        let mut parsed = files(&[("app.ts", &source), ("client.ts", owner), ("other.ts", exported)], Language::TypeScript);
        if !extra.is_empty() { parsed.extend(files(&[("client.js", extra)], Language::JavaScript)); }
        let cg = CallGraph::build(&parsed);
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| check(&cg, "app.ts", None, ResolutionKind::TypedParam)));
        if result.is_err() { failures.push(label); }
    }
    assert!(failures.is_empty(), "{failures:?}");
}

#[test]
fn same_file_class_write_revokes_receiver_authority() {
    for body in [
        "Client = Other;",
        "function replace() { Client = Other; }",
        "if (ok) { Client = Other; }",
    ] {
        let source = format!("class Client {{ m() {{}} }}\nclass Other {{ m() {{}} }}\n{body}\nfunction run(x: Client) {{ x.m(); }}");
        check(
            &CallGraph::build(&files(&[("app.ts", &source)], Language::TypeScript)),
            "app.ts",
            None,
            ResolutionKind::TypedParam,
        );
    }
}

#[test]
fn python_inert_regular_package_submodule_receiver() {
    for init in [
        "",
        "# package\n",
        "\"\"\"package documentation\"\"\"\npass\n",
    ] {
        for (body, kind) in [
            (
                "def run(x: m.Client):\n    x.m()\n",
                ResolutionKind::TypedParam,
            ),
            (
                "def run():\n    x = m.Client()\n    x.m()\n",
                ResolutionKind::ConstructorLocal,
            ),
        ] {
            let source = format!("from pkg import models as m\n{body}");
            let parsed = files(
                &[
                    ("app.py", &source),
                    ("pkg/__init__.py", init),
                    ("pkg/models.py", "class Client:\n    def m(self): pass\n"),
                ],
                Language::Python,
            );
            check(
                &CallGraph::build(&parsed),
                "app.py",
                Some("pkg/models.py"),
                kind,
            );
        }
    }
}

#[test]
fn python_regular_package_initializer_negative_matrix() {
    for init in [
        "models = other\n",
        "from . import other as models\n",
        "from other import *\n",
        "def __getattr__(name): return other\n",
        "__all__ = ['models']\n",
        "setup()\n",
        "if flag:\n    models = other\n",
        "f'{setup()}'\n",
        "def broken(\n",
    ] {
        let parsed = files(
            &[
                (
                    "app.py",
                    "from pkg import models\ndef run(x: models.Client):\n    x.m()\n",
                ),
                ("pkg/__init__.py", init),
                ("pkg/models.py", "class Client:\n    def m(self): pass\n"),
            ],
            Language::Python,
        );
        check(
            &CallGraph::build(&parsed),
            "app.py",
            None,
            ResolutionKind::TypedParam,
        );
    }
}

#[test]
fn preserved_imported_owner_and_slot_controls() {
    for (owner, importer) in [
        ("export class Client { other = 1; m() {}\n constructor() { this.other = 2; } }", "import {Client as Alias} from './client'; function run(x: Alias) { x.m(); }"),
        ("export class Client { static m = other;\n m() {} }", "import {Client as Alias} from './client'; function run(x: Alias) { x.m(); }"),
        ("export class Client { m() {} }\nfunction unrelated(Client) { Client = other; }", "import {Client as Alias} from './client'; function run(x: Alias) { x.m(); }"),
        ("export class Client { m() {} }", "import {Client as Alias} from './client'; function unrelated(Alias) { Alias = other; } function run(x: Alias) { x.m(); }"),
    ] {
        check(&CallGraph::build(&files(&[("app.ts", importer), ("client.ts", owner)], Language::TypeScript)), "app.ts", Some("client.ts"), ResolutionKind::TypedParam);
    }
}

#[test]
fn python_parent_chain_requires_inert_regular_initializers() {
    for (parent, competing, expected) in [
        ("", false, true),
        ("hook()", false, false),
        ("", true, false),
    ] {
        let mut parsed = files(
            &[
                (
                    "app.py",
                    "from pkg.nested import models\ndef run(x: models.Client):\n    x.m()\n",
                ),
                ("pkg/__init__.py", parent),
                ("pkg/nested/__init__.py", "pass\n"),
                (
                    "pkg/nested/models.py",
                    "class Client:\n    def m(self): pass\n",
                ),
            ],
            Language::Python,
        );
        if competing {
            parsed.extend(files(&[("pkg.py", "")], Language::Python));
        }
        check(
            &CallGraph::build(&parsed),
            "app.py",
            expected.then_some("pkg/nested/models.py"),
            ResolutionKind::TypedParam,
        );
    }
}

fn check(cg: &CallGraph, caller: &str, target: Option<&str>, kind: ResolutionKind) {
    let site = cg
        .calls
        .iter()
        .filter(|(id, _)| id.file == caller && id.name == "run")
        .flat_map(|(_, sites)| sites)
        .find(|s| s.callee_name == "m")
        .expect("run -> m");
    let edges = cg.resolve_call_site(site);
    let exact: Vec<_> = edges
        .iter()
        .filter(|e| e.confidence == ResolutionConfidence::Exact)
        .collect();
    if let Some(target) = target {
        assert_eq!(exact.len(), 1, "{site:?}: {edges:?}");
        assert_eq!(exact[0].target.file, target);
        assert_eq!(exact[0].kind, kind);
    } else {
        assert!(exact.is_empty(), "{site:?}: {edges:?}");
    }
}
