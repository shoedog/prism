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
