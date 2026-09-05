use prism::{
    ast::ParsedFile,
    call_graph::CallGraph,
    languages::Language,
    resolution::{ResolutionConfidence, ResolutionKind},
};
use std::collections::{BTreeMap, BTreeSet};

fn files(srcs: &[(&str, &str)], lang: Language) -> BTreeMap<String, ParsedFile> {
    srcs.iter()
        .map(|(p, s)| (p.to_string(), ParsedFile::parse(p, s, lang).unwrap()))
        .collect()
}

fn check(cg: &CallGraph, caller: &str, target: Option<&str>, kind: ResolutionKind) {
    let site = cg
        .calls
        .iter()
        .filter(|(id, _)| id.file == caller && id.name == "run")
        .flat_map(|(_, s)| s)
        .find(|s| s.callee_name == "m")
        .unwrap();
    let edges = cg.resolve_call_site(site);
    let exact: Vec<_> = edges
        .iter()
        .filter(|e| e.confidence == ResolutionConfidence::Exact)
        .collect();
    assert_eq!(
        exact.len(),
        usize::from(target.is_some()),
        "{site:?}: {edges:?}"
    );
    if let Some(target) = target {
        assert_eq!(exact[0].target.file, target);
        assert_eq!(exact[0].kind, kind);
    }
}

#[test]
fn indirect_default_class_identity() {
    for (lang, ext) in [
        (Language::JavaScript, "js"),
        (Language::TypeScript, "ts"),
        (Language::Tsx, "tsx"),
    ] {
        let caller = format!("app.{ext}");
        let owner = format!("client.{ext}");
        for declaration in [
            "class Client { m() {} } export default Client;",
            "export class Client { m() {} } export default Client;",
            "class Client { m() {} } function change(Client) { Client = other; } export default Client;",
        ] {
            for (import, body, kind) in [
                ("import Alias from './client';", "function run() { const x = new Alias(); x.m(); }", ResolutionKind::ConstructorLocal),
                ("import {default as Alias} from './client';", "function run(x: Alias) { x.m(); }", ResolutionKind::TypedParam),
                ("import type Alias from './client';", "function run(x: Alias) { x.m(); }", ResolutionKind::TypedParam),
            ] {
                if lang == Language::JavaScript && kind == ResolutionKind::TypedParam { continue; }
                let parsed = files(&[(&caller, &format!("{import} {body}")), (&owner, declaration)], lang);
                for cg in [CallGraph::build(&parsed), CallGraph::build_direct_subset(&parsed, &parsed.keys().cloned().collect())] {
                    check(&cg, &caller, Some(&owner), kind);
                }
            }
        }
    }
}

#[test]
fn indirect_default_class_duplicate_and_write_barriers() {
    let mut failures = Vec::new();
    for declaration in [
        "export default Client; class Client { m() {} }",
        "class Client { m() {} } class Client { m() {} } export default Client;",
        "class Client { m() {} } function Client() {} export default Client;",
        "class Client { m() {} } import Client from './other'; export default Client;",
        "class Client { m() {} } import * as Client from './other'; export default Client;",
        "class Client { m() {} } import type Client from './other'; export default Client;",
        "class Client { m() {} } const {Client} = other; export default Client;",
        "class Client { m() {} } let Client = other; export default Client;",
        "class Client { m() {} } Client = Other; export default Client;",
        "class Client { m() {} } export default Client; Client = Other;",
        "class Client { m() {} } function change() { Client = Other; } export default Client;",
        "class Client { m() {} } ({Client} = other); export default Client;",
        "class Client { m() {} } for (Client of xs) {} export default Client;",
        "class Client { m() {} } Client++; export default Client;",
        "class Client { m() {} } Client += other; export default Client;",
        "class Client { m() {} } export default Client; export default Client;",
        "class Client { m() {} } export {Client as default}; export default Client;",
        "function factory() { class Client { m() {} } } export default Client;",
        "import Client from './other'; export default Client;",
        "const Client = class { m() {} }; export default Client;",
        "class Client { m() {} } const Alias = Client; export default Alias;",
        "class Client { m() {} } export default (Client);",
        "@decorate class Client { m() {} } export default Client;",
    ] {
        let cg = CallGraph::build(&files(&[("app.ts", "import Alias from './client'; function run() { const x = new Alias(); x.m(); }"), ("client.ts", declaration), ("other.ts", "export default class Other { m() {} }")], Language::TypeScript));
        if std::panic::catch_unwind(|| check(&cg, "app.ts", None, ResolutionKind::ConstructorLocal))
            .is_err()
        {
            failures.push(declaration);
        }
    }
    assert!(failures.is_empty(), "{failures:?}");
}

#[test]
fn indirect_default_class_facts_do_not_fall_back_to_callable_exports() {
    use prism::js_exports::JsExportTarget;
    for (src, class, poison) in [
        (
            "class Client { m() {} } export default Client;",
            true,
            false,
        ),
        (
            "class Client { m() {} } function Client() {} export default Client;",
            true,
            true,
        ),
        (
            "class Client { m() {} } Client = other; export default Client;",
            true,
            true,
        ),
        ("function Client() {} export default Client;", false, false),
    ] {
        let parsed = ParsedFile::parse("client.ts", src, Language::TypeScript).unwrap();
        let facts = parsed.extract_js_ts_export_facts();
        let expected = if class {
            JsExportTarget::Class("Client".into())
        } else {
            JsExportTarget::Local("Client".into())
        };
        assert_eq!(facts.named.get("default"), Some(&expected), "{src}");
        assert_eq!(facts.conflicted.contains("default"), poison, "{src}");
    }
}

#[test]
fn direct_default_class_receiver_identity() {
    for (lang, ext) in [
        (Language::JavaScript, "js"),
        (Language::TypeScript, "ts"),
        (Language::Tsx, "tsx"),
    ] {
        let caller = format!("app.{ext}");
        let owner = format!("client.{ext}");
        for import in [
            "import Alias from './client';",
            "import {default as Alias} from './client';",
        ] {
            let parsed = files(
                &[
                    (
                        &caller,
                        &format!("{import} function run() {{ const x = new Alias(); x.m(); }}"),
                    ),
                    (&owner, "export default class Client { m() {} }"),
                    (&format!("decoy.{ext}"), "class Client { m() {} }"),
                ],
                lang,
            );
            for cg in [
                CallGraph::build(&parsed),
                CallGraph::build_direct_subset(&parsed, &parsed.keys().cloned().collect()),
            ] {
                check(&cg, &caller, Some(&owner), ResolutionKind::ConstructorLocal);
            }
        }
        if lang == Language::JavaScript {
            continue;
        }
        for import in [
            "import Alias from './client';",
            "import {default as Alias} from './client';",
            "import type Alias from './client';",
            "import type {default as Alias} from './client';",
            "import {type default as Alias} from './client';",
        ] {
            let parsed = files(
                &[
                    (
                        &caller,
                        &format!("{import} function run(x: Alias) {{ x.m(); }}"),
                    ),
                    (&owner, "export default class Client { m() {} }"),
                ],
                lang,
            );
            let cg = CallGraph::build(&parsed);
            check(&cg, &caller, Some(&owner), ResolutionKind::TypedParam);
            assert_eq!(
                cg.import_bindings
                    .get(&caller)
                    .is_some_and(|bs| bs.iter().any(|b| b.local == "Alias")),
                !import.contains("type")
            );
        }
    }
}

#[test]
fn default_class_receiver_boundaries() {
    for (import, owner, body) in [
        (
            "import Alias from './client';",
            "export default class { m() {} }",
            "const x = new Alias(); x.m();",
        ),
        (
            "import Alias from './client';",
            "class Client { m() {} } export {Client as default};",
            "const x = new Alias(); x.m();",
        ),
        (
            "import Alias from './client';",
            "export {Client as default} from './other';",
            "const x = new Alias(); x.m();",
        ),
        (
            "import Alias from './client';",
            "export default class Client { m() {} } export default class Other { m() {} }",
            "const x = new Alias(); x.m();",
        ),
        (
            "import Alias from './client'; Alias = other;",
            "export default class Client { m() {} }",
            "const x = new Alias(); x.m();",
        ),
        (
            "import Alias from './client';",
            "export default class Client { m() {} }",
            "const Alias = other; const x = new Alias(); x.m();",
        ),
        (
            "import type Alias from './client';",
            "export default class Client { m() {} }",
            "const x = new Alias(); x.m();",
        ),
        (
            "import Alias from './client';",
            "export default class Client { static m() {} }",
            "const x = new Alias(); x.m();",
        ),
        (
            "const Alias = require('./client');",
            "export default class Client { m() {} }",
            "const x = new Alias(); x.m();",
        ),
    ] {
        let cg = CallGraph::build(&files(
            &[
                ("app.ts", &format!("{import} function run() {{ {body} }}")),
                ("client.ts", owner),
                ("other.ts", "export class Client { m() {} }"),
            ],
            Language::TypeScript,
        ));
        check(&cg, "app.ts", None, ResolutionKind::ConstructorLocal);
    }
    for import in [
        "import type * as Alias from './client';",
        "import type Alias from './client'; type Alias = Other;",
        "import type Alias from './client'; import type Alias from './other';",
        "import type Alias from './client'; import Alias from './other';",
    ] {
        let cg = CallGraph::build(&files(
            &[
                (
                    "app.ts",
                    &format!("{import} function run(x: Alias) {{ x.m(); }}"),
                ),
                ("client.ts", "export default class Client { m() {} }"),
                ("other.ts", "export default class Other { m() {} }"),
            ],
            Language::TypeScript,
        ));
        check(&cg, "app.ts", None, ResolutionKind::TypedParam);
    }
}

#[test]
fn type_only_named_class_parameters_have_defining_identity() {
    for (lang, ext) in [(Language::TypeScript, "ts"), (Language::Tsx, "tsx")] {
        for import in [
            "import type { Client as Alias } from './client';",
            "import { type Client as Alias } from './client';",
        ] {
            for body in [
                "function run(x: Alias) { x.m(); }",
                "const Alias = other; function run(x: Alias) { x.m(); }",
                "function outer(x: Alias) { function run() { x.m(); } }",
                "export const run = (x: Alias) => { x.m(); }",
                "function Alias() {} function run(x: Alias) { x.m(); }",
            ] {
                let caller = format!("app.{ext}");
                let owner = format!("client.{ext}");
                let parsed = files(
                    &[
                        (&caller, &format!("{import}\n{body}")),
                        (&owner, "export class Client { m() {} }"),
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
                    assert!(
                        !cg.import_bindings
                            .get(&caller)
                            .is_some_and(|bs| bs.iter().any(|b| b.local == "Alias")),
                        "type-only import leaked into runtime bindings"
                    );
                    check(&cg, &caller, Some(&owner), ResolutionKind::TypedParam);
                }
            }
        }
    }
}

#[test]
fn type_only_imports_do_not_supply_runtime_or_shadowed_authority() {
    let import = "import type {Client as Alias} from './client';";
    for body in [
        "function run() { const x = new Alias(); x.m(); }",
        "function run<Alias>(x: Alias) { x.m(); }",
        "function outer() { type Alias = Other; function run(x: Alias) { x.m(); } }",
        "function outer() { namespace Alias {} function run(x: Alias) { x.m(); } }",
        "type Alias = Other; function run(x: Alias) { x.m(); }",
        "import type {Other as Alias} from './other'; function run(x: Alias) { x.m(); }",
        "import {Other as Alias} from './other'; function run(x: Alias) { x.m(); }",
        "function run(x: Alias) { x = other; x.m(); }",
    ] {
        let cg = CallGraph::build(&files(
            &[
                ("app.ts", &format!("{import}\n{body}")),
                ("client.ts", "export class Client { m() {} }"),
                ("other.ts", "export class Other { m() {} }"),
            ],
            Language::TypeScript,
        ));
        check(&cg, "app.ts", None, ResolutionKind::TypedParam);
    }
}

#[test]
fn python_explicit_relative_submodule_receivers() {
    for (caller, import) in [
        ("pkg/app.py", "from . import models as m"),
        ("pkg/sub/app.py", "from .. import models as m"),
        ("pkg/app.py", "from .nested import models as m"),
    ] {
        let owner = if import.contains("nested") {
            "pkg/nested/models.py"
        } else {
            "pkg/models.py"
        };
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
            let parsed = files(
                &[
                    (caller, &format!("{import}\n{body}")),
                    ("pkg/__init__.py", ""),
                    ("pkg/sub/__init__.py", "pass\n"),
                    ("pkg/nested/__init__.py", "\"nested\"\n"),
                    (owner, "class Client:\n    def m(self): pass\n"),
                ],
                Language::Python,
            );
            check(&CallGraph::build(&parsed), caller, Some(owner), kind);
        }
    }
}

#[test]
fn python_relative_anchor_negative_matrix() {
    for (caller, import, init) in [
        ("pkg/app.py", "from . import models", "setup()"),
        ("app.py", "from .pkg import models", ""),
        ("pkg/app.py", "from .. import models", ""),
        ("pkg/app.py", "from ...pkg import models", ""),
        ("pkg/app.py", "from . import models", "models = other"),
    ] {
        let parsed = files(
            &[
                (
                    caller,
                    &format!("{import}\ndef run(x: models.Client):\n    x.m()\n"),
                ),
                ("pkg/__init__.py", init),
                ("pkg/models.py", "class Client:\n    def m(self): pass\n"),
                ("models.py", "class Client:\n    def m(self): pass\n"),
            ],
            Language::Python,
        );
        check(
            &CallGraph::build(&parsed),
            caller,
            None,
            ResolutionKind::TypedParam,
        );
    }
}

#[test]
fn type_only_namespace_and_module_type_boundaries() {
    let mut failures = Vec::new();
    for import in [
        "import type Alias from './client';",
        "import type * as Alias from './client';",
        "import { type default as Alias } from './client';",
        "import type {Client as Alias} from './client'; import type * as Alias from './other';",
        "import type {Client as Alias} from './client'; interface Alias {}",
        "import type {Client as Alias} from './client'; class Alias { m() {} }",
        "import type {Client as Alias} from './client'; declare class Alias { m(): void; }",
        "import type {Client as Alias} from './client'; namespace Alias {}",
        "import type {Client as Alias} from './client'; namespace Alias.Inner {}",
        "import type {Client as Alias} from './client'; export namespace Alias {}",
    ] {
        let parsed = files(
            &[
                (
                    "app.ts",
                    &format!("{import}\nfunction run(x: Alias) {{ x.m(); }}"),
                ),
                ("client.ts", "export class Client { m() {} }"),
                ("other.ts", "export class Client { m() {} }"),
            ],
            Language::TypeScript,
        );
        if std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            check(
                &CallGraph::build(&parsed),
                "app.ts",
                None,
                ResolutionKind::TypedParam,
            )
        }))
        .is_err()
        {
            eprintln!("{}", parsed["app.ts"].tree.root_node().to_sexp());
            failures.push(import);
        }
    }
    assert!(failures.is_empty(), "{failures:?}");
    let parsed = files(&[("app.ts", "import {type Client as Alias, other} from './client'; function run(x: Alias) { x.m(); other(); }"), ("client.ts", "export class Client { m() {} } export function other() {}")], Language::TypeScript);
    let cg = CallGraph::build(&parsed);
    check(&cg, "app.ts", Some("client.ts"), ResolutionKind::TypedParam);
    let site = cg
        .calls
        .values()
        .flatten()
        .find(|site| site.callee_name == "other")
        .unwrap();
    assert!(cg
        .resolve_call_site(site)
        .iter()
        .any(|e| e.confidence == ResolutionConfidence::Exact && e.target.file == "client.ts"));
}

#[test]
fn relative_receiver_incremental_anchor_and_candidate_transitions() {
    use prism::cpg::CodePropertyGraph;
    for (caller, import, changed, good, bad) in [
        (
            "pkg/app.py",
            "from . import models",
            "pkg/__init__.py",
            Some(""),
            None,
        ),
        (
            "pkg/sub/app.py",
            "from .. import models",
            "pkg/__init__.py",
            Some(""),
            Some("setup()"),
        ),
        (
            "pkg/app.py",
            "from . import models",
            "pkg/models/__init__.py",
            None,
            Some("class Client:\n    def m(self): pass\n"),
        ),
    ] {
        let make = |body: Option<&str>| {
            let mut parsed = files(
                &[
                    (
                        caller,
                        &format!("{import}\ndef run(x: models.Client):\n    x.m()\n"),
                    ),
                    ("pkg/__init__.py", ""),
                    ("pkg/sub/__init__.py", ""),
                    ("pkg/models.py", "class Client:\n    def m(self): pass\n"),
                ],
                Language::Python,
            );
            if let Some(body) = body {
                parsed.insert(
                    changed.to_string(),
                    ParsedFile::parse(changed, body, Language::Python).unwrap(),
                );
            } else {
                parsed.remove(changed);
            }
            parsed
        };
        for (before, after, expected) in [(bad, good, Some("pkg/models.py")), (good, bad, None)] {
            let previous = CodePropertyGraph::build(&make(before));
            let after = make(after);
            let incremental = CodePropertyGraph::build_incremental(
                previous.call_graph,
                previous.dfg,
                &BTreeSet::from([changed.to_string()]),
                &after,
                None,
            );
            let full = CodePropertyGraph::build(&after);
            for cg in [
                &full.call_graph,
                &incremental.call_graph,
                &CallGraph::build_direct_subset(&after, &after.keys().cloned().collect()),
            ] {
                check(cg, caller, expected, ResolutionKind::TypedParam);
            }
            assert_eq!(full.call_graph.calls, incremental.call_graph.calls);
        }
    }
}
