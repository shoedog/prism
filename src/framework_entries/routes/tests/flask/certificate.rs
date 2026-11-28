//! RED rows for the ONE certificate of PR #928 part 2 (lane 12; contract in
//! `p2-contract-and-certificate.md`, DESIGN 5.3): a route proof is a static-binding proof, the
//! name it relies on has exactly one binding, by Python's declaration, scope, import and
//! global/nonlocal syntax.
//!
//! Verdict of a row: PROVED when every `FlaskEvent::Rule` of the entry has a `Router` receiver
//! and only `Resolved` callbacks, WITHHELD when every rule has an unproven receiver or an
//! unresolved callback. A row must produce at least one rule event, and a fixture that does not
//! parse cleanly is a failed probe (tree-sitter reports an error), except in the `syntax_error`
//! rows, so an accidental syntax error can never pass as a withheld row.
//!
//! Test names are `<form>::<role>`: roles are `router_after` / `router_before` (the proved name
//! is a router in its own module, the second binding sits after the registration or before the
//! allocation), `view_source` (a captured view is rebound in the SOURCE module),
//! `view_capture` (the capturing name is rebound), `router_import_source` and
//! `router_import_capture` (the same two for a router imported with `from app import app`).
//! Control rows (`controls::*`) keep the proof.
//!
//! STATUS at head cffd488d (308 rows: 224 GREEN, 84 RED), by running this module:
//!
//! RED (the current code does not give the expected verdict):
//! - `forms::<form>::router_after` (21 of 25 forms) and `router_before` (17): the proved router
//!   is rebound and the route is still PROVED today (the walk is position-sensitive: a later
//!   rebinding is ignored, an earlier one is overwritten). GREEN: the four `global_*` forms and
//!   `global_in_method`, and `router_before` for import / import-as / from-import forms.
//! - `controls::<scope>::view_capture` and `router_import_capture` (9 scopes each) and
//!   `controls::comprehension_target::view_source` / `router_import_source`: the flat capture
//!   census over-refuses a same spelling in another scope (parameter, local, class body,
//!   lambda, loop and except target, import inside a function, nested function local) and a
//!   module-level comprehension target counts as a module binding.
//! - `module_system::router_*` (11 names) and `view_source_doc` / `view_source_path`: a
//!   module-system name is not counted as one more binding.
//! - `star::router_in_repo_star_supplies_the_name_after` / `_before` and
//!   `view_capture_in_repo_star_supplies_the_name`: a star that can bind the key is not counted
//!   for a router, or in the capturing module.
//! - `syntax_error::*` (3 of the 3 withheld rows): a module with a parse error is proved today.
//! - `nonlocal::router_rebound_through_nonlocal` (proved today) and
//!   `nonlocal::view_own_local_in_nested_def_control` (over-refused today).
//! - `used::s04_*` and `used::s05_function_local_*`: harmless same-spelling uses refused today.
//! - AMBIGUOUS, RED today (see the report): `star::view_source_external_star_control`,
//!   `module_system::package_child_not_imported_by_the_package_control`,
//!   `module_system::package_from_child_import_of_the_same_name_control`.
//!
//! GREEN today (these pin the replacement): every `forms::*::view_source`, `view_capture`,
//! `router_import_source`, `router_import_capture` row; `binding_kind::*` (conditional and
//! other-kind single bindings, zero bindings); every other control; `star` rows of the source
//! and import roles; the `used::*` rows other than the above (including S01+S07 WITHHELD and
//! S08, whose router proof stands while the callback is `InnerDecorator`); every
//! `known_limits::*` row (they pin today's verdicts).
use super::*;
use crate::framework_entries::routes::python::flask::FlaskRule;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum V {
    Proved,
    Withheld,
}

fn verdict(rule: &FlaskRule<'_>) -> V {
    let proved = matches!(rule.route.receiver, Receiver::Router { .. })
        && rule
            .route
            .callbacks
            .iter()
            .all(|callback| matches!(callback.status, CallbackStatus::Resolved { .. }));
    if proved {
        V::Proved
    } else {
        V::Withheld
    }
}

fn verdicts_of(files: &BTreeMap<String, ParsedFile>, entry: &str, clean: bool) -> Vec<V> {
    if clean {
        for (path, file) in files {
            assert!(
                !file.tree.root_node().has_error(),
                "fixture {path} does not parse cleanly (a failed probe, not a row)"
            );
        }
    }
    let bindings = index_python(files);
    flask_facts(&bindings, entry, &SetupContext::default())
        .iter()
        .filter_map(|event| match event {
            FlaskEvent::Rule(rule) => Some(verdict(rule)),
            _ => None,
        })
        .collect()
}

/// PROVED: at least one rule event and every rule proved. WITHHELD: no rule is proved (a
/// rule with an unproven receiver or an unresolved callback, or no event at all: an imported
/// router that is not proved leaves no hint, so the site is omitted rather than retained).
#[track_caller]
fn expect(files: &[(&str, &str)], entry: &str, want: V) {
    let got = verdicts_of(&parsed(files), entry, true);
    match want {
        V::Proved => assert!(
            !got.is_empty() && got.iter().all(|&found| found == V::Proved),
            "want PROVED for {entry}, got {got:?}; files {files:?}"
        ),
        V::Withheld => assert!(
            !got.contains(&V::Proved),
            "want WITHHELD for {entry}, got {got:?}; files {files:?}"
        ),
    }
}

const ALLOC: &str = "from flask import Flask\napp = Flask(__name__)\n";
const ROUTE: &str = "@app.route('/x')\ndef f(): pass\n";
const VIEW_REG: &str = "from views import f\napp.add_url_rule('/x', view_func=f)\n";
const ROUTER_USE: &str = "from app import app\n@app.route('/x')\ndef f(): pass\n";
const VIEWS: &str = "def f(): pass\n";

fn at(template: &str, name: &str) -> String {
    template.replace("{k}", name)
}

/// The six roles of one binding form; `snippet` is a template over `{k}`. Every role first
/// checks that the same fixture WITHOUT the snippet is proved, so a withheld row can never be
/// the result of a broken fixture.
mod roles {
    use super::*;

    #[track_caller]
    fn run(
        files: impl Fn(&str) -> Vec<(&'static str, String)>,
        entry: &str,
        snippet: &str,
        want: V,
    ) {
        let built = files("");
        let refs: Vec<(&str, &str)> = built.iter().map(|(p, s)| (*p, s.as_str())).collect();
        let base = verdicts_of(&parsed(&refs), entry, true);
        assert!(
            !base.is_empty() && base.iter().all(|&v| v == V::Proved),
            "BASELINE (fixture without the snippet) is not proved: a failed probe, got {base:?}"
        );
        let built = files(snippet);
        let refs: Vec<(&str, &str)> = built.iter().map(|(p, s)| (*p, s.as_str())).collect();
        expect(&refs, entry, want);
    }

    #[track_caller]
    pub fn router_after(snippet: &str, want: V) {
        let files = |s: &str| vec![("app.py", format!("{ALLOC}{ROUTE}{}", at(s, "app")))];
        run(files, "app.py", snippet, want);
    }
    #[track_caller]
    pub fn router_before(snippet: &str, want: V) {
        let files = |s: &str| {
            vec![(
                "app.py",
                format!(
                    "from flask import Flask\n{}app = Flask(__name__)\n{ROUTE}",
                    at(s, "app")
                ),
            )]
        };
        run(files, "app.py", snippet, want);
    }
    #[track_caller]
    pub fn view_source(snippet: &str, want: V) {
        let files = |s: &str| {
            vec![
                ("app.py", format!("{ALLOC}{VIEW_REG}")),
                ("views.py", format!("{VIEWS}{}", at(s, "f"))),
            ]
        };
        run(files, "app.py", snippet, want);
    }
    #[track_caller]
    pub fn view_capture(snippet: &str, want: V) {
        let files = |s: &str| {
            vec![
                ("app.py", format!("{ALLOC}{VIEW_REG}{}", at(s, "f"))),
                ("views.py", VIEWS.to_owned()),
            ]
        };
        run(files, "app.py", snippet, want);
    }
    #[track_caller]
    pub fn router_import_source(snippet: &str, want: V) {
        let files = |s: &str| {
            vec![
                ("app.py", format!("{ALLOC}{}", at(s, "app"))),
                ("routes.py", ROUTER_USE.to_owned()),
            ]
        };
        run(files, "routes.py", snippet, want);
    }
    #[track_caller]
    pub fn router_import_capture(snippet: &str, want: V) {
        let files = |s: &str| {
            vec![
                ("app.py", ALLOC.to_owned()),
                ("routes.py", format!("{ROUTER_USE}{}", at(s, "app"))),
            ]
        };
        run(files, "routes.py", snippet, want);
    }
}

/// One binding form as a SECOND binding of the proved name, in all six roles: withheld.
macro_rules! form {
    ($name:ident, $snippet:expr) => {
        mod $name {
            use super::*;
            const S: &str = $snippet;
            #[test]
            fn router_after() {
                roles::router_after(S, V::Withheld);
            }
            #[test]
            fn router_before() {
                roles::router_before(S, V::Withheld);
            }
            #[test]
            fn view_source() {
                roles::view_source(S, V::Withheld);
            }
            #[test]
            fn view_capture() {
                roles::view_capture(S, V::Withheld);
            }
            #[test]
            fn router_import_source() {
                roles::router_import_source(S, V::Withheld);
            }
            #[test]
            fn router_import_capture() {
                roles::router_import_capture(S, V::Withheld);
            }
        }
    };
}

mod forms {
    use super::*;

    form!(import_plain, "import {k}\n");
    form!(import_as, "import os as {k}\n");
    form!(from_import, "from helpers import {k}\n");
    form!(from_import_as, "from os import path as {k}\n");
    form!(def, "def {k}(): pass\n");
    form!(async_def, "async def {k}(): pass\n");
    form!(class, "class {k}: pass\n");
    form!(assign, "{k} = 1\n");
    form!(augmented, "{k} += 1\n");
    form!(annotated_value, "{k}: int = 1\n");
    form!(unpack_tuple, "{k}, _other = 1, 2\n");
    form!(unpack_list, "[{k}, _other] = [1, 2]\n");
    form!(unpack_starred, "*{k}, _other = [1, 2]\n");
    form!(for_target, "for {k} in range(2):\n    pass\n");
    form!(with_as, "with open('x') as {k}:\n    pass\n");
    form!(
        except_as,
        "try:\n    pass\nexcept Exception as {k}:\n    pass\n"
    );
    form!(
        except_star_as,
        "try:\n    pass\nexcept* Exception as {k}:\n    pass\n"
    );
    form!(walrus, "if ({k} := 1):\n    pass\n");
    form!(match_capture, "match 1:\n    case {k}:\n        pass\n");
    form!(del, "del {k}\n");
    form!(type_alias, "type {k} = int\n");
    // A `global K` declaration redirects the write in the def to the module binding.
    form!(
        global_write,
        "def _rebind():\n    global {k}\n    {k} = 1\n_rebind()\n"
    );
    form!(
        global_import_write,
        "def _rebind():\n    global {k}\n    from os import path as {k}\n_rebind()\n"
    );
    form!(
        global_del,
        "def _rebind():\n    global {k}\n    del {k}\n_rebind()\n"
    );
    form!(
        global_augmented,
        "def _rebind():\n    global {k}\n    {k} += 1\n_rebind()\n"
    );
    // A global write inside a method (class scope between the def and the module).
    form!(
        global_in_method,
        "class _C:\n    def _m(self):\n        global {k}\n        {k} = 1\n"
    );
}

/// Clause (c): a star import counts only when the repository says it can bind the key.
mod star {
    use super::*;

    const STAR_LIB: &str = "from lib import *\n";
    const STAR_EXTERNAL: &str = "from nonexistent_pkg import *\n";

    #[test]
    fn router_in_repo_star_supplies_the_name_after() {
        let app = format!("{ALLOC}{ROUTE}{STAR_LIB}");
        expect(
            &[("app.py", &app), ("lib.py", "app = 1\n")],
            "app.py",
            V::Withheld,
        );
    }
    #[test]
    fn router_in_repo_star_supplies_the_name_before() {
        let app = format!("from flask import Flask\n{STAR_LIB}app = Flask(__name__)\n{ROUTE}");
        expect(
            &[("app.py", &app), ("lib.py", "app = 1\n")],
            "app.py",
            V::Withheld,
        );
    }
    #[test]
    fn view_source_in_repo_star_supplies_the_name() {
        let views = format!("{VIEWS}{STAR_LIB}");
        let app = format!("{ALLOC}{VIEW_REG}");
        expect(
            &[
                ("app.py", &app),
                ("views.py", &views),
                ("lib.py", "f = 1\n"),
            ],
            "app.py",
            V::Withheld,
        );
    }
    #[test]
    fn view_capture_in_repo_star_supplies_the_name() {
        let app = format!("{ALLOC}{VIEW_REG}{STAR_LIB}");
        expect(
            &[("app.py", &app), ("views.py", VIEWS), ("lib.py", "f = 1\n")],
            "app.py",
            V::Withheld,
        );
    }
    #[test]
    fn router_import_source_in_repo_star_supplies_the_name() {
        let app = format!("{ALLOC}{STAR_LIB}");
        expect(
            &[
                ("app.py", &app),
                ("lib.py", "app = 1\n"),
                ("routes.py", ROUTER_USE),
            ],
            "routes.py",
            V::Withheld,
        );
    }
    // Controls: an external or unresolved star, and an in-repo star that cannot bind the key.
    #[test]
    fn router_external_star_control() {
        for star in [STAR_EXTERNAL, "from flask_sqlalchemy import *\n"] {
            let app = format!("{ALLOC}{ROUTE}{star}");
            expect(&[("app.py", &app)], "app.py", V::Proved);
            let before = format!("from flask import Flask\n{star}app = Flask(__name__)\n{ROUTE}");
            expect(&[("app.py", &before)], "app.py", V::Proved);
        }
    }
    #[test]
    fn router_in_repo_star_that_cannot_bind_control() {
        let app = format!("{ALLOC}{ROUTE}{STAR_LIB}");
        expect(
            &[("app.py", &app), ("lib.py", "other = 1\n")],
            "app.py",
            V::Proved,
        );
    }
    // AMBIGUOUS (reported): clause (c) says an external star is not counted, but the contract
    // also keeps the source-side `star_import` predicate, which today refuses on any star that
    // does not resolve in-repo (`star_could_supply` answers true when the source is unknown).
    #[test]
    fn view_source_external_star_control() {
        let views = format!("{VIEWS}{STAR_EXTERNAL}");
        let app = format!("{ALLOC}{VIEW_REG}");
        expect(
            &[("app.py", &app), ("views.py", &views)],
            "app.py",
            V::Proved,
        );
    }
    #[test]
    fn view_source_in_repo_star_that_cannot_bind_control() {
        let views = format!("{VIEWS}{STAR_LIB}");
        let app = format!("{ALLOC}{VIEW_REG}");
        expect(
            &[
                ("app.py", &app),
                ("views.py", &views),
                ("lib.py", "g = 1\n"),
            ],
            "app.py",
            V::Proved,
        );
    }
    #[test]
    fn view_capture_external_star_control() {
        let app = format!("{ALLOC}{VIEW_REG}{STAR_EXTERNAL}");
        expect(
            &[("app.py", &app), ("views.py", VIEWS)],
            "app.py",
            V::Proved,
        );
    }
}

/// Clause (b): the single binding must be a direct statement of an admitted kind.
mod binding_kind {
    use super::*;

    fn nest(kind: &str, body: &str) -> String {
        let indented: String = body.lines().map(|line| format!("    {line}\n")).collect();
        match kind {
            "if" => format!("import os\nif os.environ:\n{indented}"),
            "try" => format!("try:\n{indented}except Exception:\n    pass\n"),
            "with" => format!("import contextlib\nwith contextlib.nullcontext():\n{indented}"),
            "for" => format!("for _i in range(1):\n{indented}"),
            "while" => format!("while False:\n{indented}"),
            "match" => format!(
                "match 1:\n    case 1:\n{}",
                body.lines()
                    .map(|line| format!("        {line}\n"))
                    .collect::<String>()
            ),
            _ => unreachable!(),
        }
    }

    macro_rules! nested {
        ($name:ident, $kind:expr) => {
            mod $name {
                use super::*;
                #[test]
                fn router_single_binding_is_not_direct() {
                    let app = format!(
                        "from flask import Flask\n{}{ROUTE}",
                        nest($kind, "app = Flask(__name__)")
                    );
                    expect(&[("app.py", &app)], "app.py", V::Withheld);
                }
                #[test]
                fn view_single_binding_is_not_direct() {
                    let views = nest($kind, "def f(): pass");
                    let app = format!("{ALLOC}{VIEW_REG}");
                    expect(
                        &[("app.py", &app), ("views.py", &views)],
                        "app.py",
                        V::Withheld,
                    );
                }
                #[test]
                fn router_import_single_binding_is_not_direct() {
                    let app = format!(
                        "from flask import Flask\n{}",
                        nest($kind, "app = Flask(__name__)")
                    );
                    expect(
                        &[("app.py", &app), ("routes.py", ROUTER_USE)],
                        "routes.py",
                        V::Withheld,
                    );
                }
            }
        };
    }
    nested!(under_if, "if");
    nested!(under_try, "try");
    nested!(under_with, "with");
    nested!(under_for, "for");
    nested!(under_while, "while");
    nested!(under_match, "match");

    #[test]
    fn direct_single_binding_controls() {
        expect(
            &[("app.py", &format!("{ALLOC}{ROUTE}"))],
            "app.py",
            V::Proved,
        );
        expect(
            &[
                ("app.py", &format!("{ALLOC}{VIEW_REG}")),
                ("views.py", VIEWS),
            ],
            "app.py",
            V::Proved,
        );
        expect(
            &[("app.py", ALLOC), ("routes.py", ROUTER_USE)],
            "routes.py",
            V::Proved,
        );
    }

    // The single binding is of a kind the certificate does not admit (tuple, for, with,
    // except, walrus, match, type alias, augmented).
    fn view_only(binding: &str) {
        let app = format!("{ALLOC}{VIEW_REG}");
        expect(
            &[("app.py", &app), ("views.py", binding)],
            "app.py",
            V::Withheld,
        );
    }
    #[test]
    fn only_binding_tuple_view() {
        view_only("def _h(): pass\nf, _g = _h, _h\n");
    }
    #[test]
    fn only_binding_for_view() {
        view_only("def _h(): pass\nfor f in [_h]:\n    pass\n");
    }
    #[test]
    fn only_binding_with_view() {
        view_only("import contextlib\nwith contextlib.nullcontext() as f:\n    pass\n");
    }
    #[test]
    fn only_binding_except_view() {
        view_only("try:\n    pass\nexcept Exception as f:\n    pass\n");
    }
    #[test]
    fn only_binding_walrus_view() {
        view_only("def _h(): pass\nif (f := _h):\n    pass\n");
    }
    #[test]
    fn only_binding_match_view() {
        view_only("def _h(): pass\nmatch _h:\n    case f:\n        pass\n");
    }
    #[test]
    fn only_binding_type_alias_view() {
        view_only("type f = int\n");
    }
    #[test]
    fn only_binding_augmented_view() {
        view_only("f += 1\n");
    }
    fn router_only(binding: &str) {
        let app = format!("from flask import Flask\n{binding}{ROUTE}");
        expect(&[("app.py", &app)], "app.py", V::Withheld);
    }
    #[test]
    fn only_binding_tuple_router() {
        router_only("app, _other = Flask(__name__), 1\n");
    }
    #[test]
    fn only_binding_for_router() {
        router_only("for app in [Flask(__name__)]:\n    pass\n");
    }
    #[test]
    fn only_binding_with_router() {
        router_only("with Flask(__name__) as app:\n    pass\n");
    }
    #[test]
    fn only_binding_walrus_router() {
        router_only("if (app := Flask(__name__)):\n    pass\n");
    }

    // Clause (a): zero bindings fails too.
    #[test]
    fn zero_bindings_view_source() {
        let app = format!("{ALLOC}{VIEW_REG}");
        expect(
            &[("app.py", &app), ("views.py", "def other(): pass\n")],
            "app.py",
            V::Withheld,
        );
    }
    #[test]
    fn zero_bindings_router() {
        let app = "@app.route('/x')\ndef f(): pass\n";
        expect(&[("app.py", app)], "app.py", V::Withheld);
    }
}

/// The same spelling bound in ANOTHER scope without global/nonlocal is a different binding.
mod controls {
    use super::*;

    const PARAMETER: &str = "def _g({k}):\n    return {k}\n";
    const LOCAL: &str = "def _g():\n    {k} = 1\n    return {k}\n";
    const CLASS_BODY: &str = "class _C:\n    {k} = 1\n";
    const COMPREHENSION: &str = "_xs = [{k} for {k} in range(3)]\n";
    const LAMBDA: &str = "_g = lambda {k}: {k}\n";
    const LOOP_IN_FUNCTION: &str = "def _g():\n    for {k} in range(3):\n        pass\n";
    const NESTED_LOCAL: &str = "def _g():\n    def _h():\n        {k} = 1\n    return _h\n";
    const IMPORT_IN_FUNCTION: &str = "def _g():\n    from os import path as {k}\n    return {k}\n";
    const EXCEPT_IN_FUNCTION: &str =
        "def _g():\n    try:\n        pass\n    except Exception as {k}:\n        pass\n";

    macro_rules! scope_control {
        ($name:ident, $snippet:expr) => {
            mod $name {
                use super::*;
                const S: &str = $snippet;
                #[test]
                fn router_after() {
                    roles::router_after(S, V::Proved);
                }
                #[test]
                fn router_before() {
                    roles::router_before(S, V::Proved);
                }
                #[test]
                fn view_source() {
                    roles::view_source(S, V::Proved);
                }
                #[test]
                fn view_capture() {
                    roles::view_capture(S, V::Proved);
                }
                #[test]
                fn router_import_source() {
                    roles::router_import_source(S, V::Proved);
                }
                #[test]
                fn router_import_capture() {
                    roles::router_import_capture(S, V::Proved);
                }
            }
        };
    }
    scope_control!(function_parameter, PARAMETER);
    scope_control!(function_local, LOCAL);
    scope_control!(class_body_name, CLASS_BODY);
    scope_control!(comprehension_target, COMPREHENSION);
    scope_control!(lambda_parameter, LAMBDA);
    scope_control!(loop_target_in_function, LOOP_IN_FUNCTION);
    scope_control!(nested_function_local, NESTED_LOCAL);
    scope_control!(import_in_function, IMPORT_IN_FUNCTION);
    scope_control!(except_in_function, EXCEPT_IN_FUNCTION);

    #[test]
    fn baseline_router() {
        expect(
            &[("app.py", &format!("{ALLOC}{ROUTE}"))],
            "app.py",
            V::Proved,
        );
    }
    #[test]
    fn baseline_view_import() {
        expect(
            &[
                ("app.py", &format!("{ALLOC}{VIEW_REG}")),
                ("views.py", VIEWS),
            ],
            "app.py",
            V::Proved,
        );
    }
    #[test]
    fn baseline_router_import() {
        expect(
            &[("app.py", ALLOC), ("routes.py", ROUTER_USE)],
            "routes.py",
            V::Proved,
        );
    }
}

/// Clause (d): module-system names count as one more binding of the key.
mod module_system {
    use super::*;

    fn router(name: &str, want: V) {
        let app = format!("from flask import Flask\n{name} = Flask(__name__)\n@{name}.route('/x')\ndef f(): pass\n");
        expect(&[("app.py", &app)], "app.py", want);
    }
    macro_rules! name_row {
        ($test:ident, $name:expr) => {
            #[test]
            fn $test() {
                router($name, V::Withheld);
            }
        };
    }
    name_row!(router_doc, "__doc__");
    name_row!(router_annotations, "__annotations__");
    name_row!(router_package, "__package__");
    name_row!(router_name, "__name__");
    name_row!(router_file, "__file__");
    name_row!(router_spec, "__spec__");
    name_row!(router_loader, "__loader__");
    name_row!(router_builtins, "__builtins__");
    name_row!(router_cached, "__cached__");
    name_row!(router_path, "__path__");
    name_row!(router_warningregistry, "__warningregistry__");

    #[test]
    fn router_other_dunder_control() {
        router("__app__", V::Proved);
    }

    fn view(name: &str, want: V) {
        let views = format!("def {name}(): pass\n");
        let app =
            format!("{ALLOC}from views import {name}\napp.add_url_rule('/x', view_func={name})\n");
        expect(&[("app.py", &app), ("views.py", &views)], "app.py", want);
    }
    #[test]
    fn view_source_doc() {
        view("__doc__", V::Withheld);
    }
    #[test]
    fn view_source_path() {
        view("__path__", V::Withheld);
    }
    #[test]
    fn view_source_other_dunder_control() {
        view("__view__", V::Proved);
    }

    // A package `__init__` that imports a child module named like the key.
    const CAPTURE: &str = "from pkg import child\napp.add_url_rule('/x', view_func=child)\n";

    #[test]
    fn package_child_import_is_one_more_binding() {
        let app = format!("{ALLOC}{CAPTURE}");
        expect(
            &[
                ("app.py", &app),
                (
                    "pkg/__init__.py",
                    "from .child import handler\ndef child(): pass\n",
                ),
                ("pkg/child.py", "def handler(): pass\n"),
            ],
            "app.py",
            V::Withheld,
        );
    }
    // AMBIGUOUS (reported): the contract counts the child only when the package imports it;
    // today `submodule_shadow` refuses whenever the child module exists in the repository.
    #[test]
    fn package_child_not_imported_by_the_package_control() {
        // Contract wording: the child counts only when the package imports it.
        let app = format!("{ALLOC}{CAPTURE}");
        expect(
            &[
                ("app.py", &app),
                ("pkg/__init__.py", "def child(): pass\n"),
                ("pkg/child.py", "def handler(): pass\n"),
            ],
            "app.py",
            V::Proved,
        );
    }
    // formspree shape (S11): `from .K import K` binds the function last, harmless.
    // Today the proof resolves `create_app` to the child MODULE (Selection::Module), so the
    // callback is unresolved; the contract's exception wants the function proved.
    #[test]
    fn package_from_child_import_of_the_same_name_control() {
        let app = format!(
            "{ALLOC}from pkg import create_app\napp.add_url_rule('/x', view_func=create_app)\n"
        );
        expect(
            &[
                ("app.py", &app),
                ("pkg/__init__.py", "from .create_app import create_app\n"),
                ("pkg/create_app.py", "def create_app(): pass\n"),
            ],
            "app.py",
            V::Proved,
        );
    }
}

/// Clause (e): a module with a parse error fails every key of that module.
mod syntax_error {
    use super::*;

    const BROKEN: &str = "def broken(:\n";

    fn want(files: &[(&str, &str)], entry: &str, want: V) {
        let got = verdicts_of(&parsed(files), entry, false);
        match want {
            V::Proved => assert!(!got.is_empty() && !got.contains(&V::Withheld), "{got:?}"),
            V::Withheld => assert!(!got.contains(&V::Proved), "{got:?}"),
        }
    }
    #[test]
    fn router_in_a_broken_module_every_key() {
        let app = format!(
            "{ALLOC}other = Flask(__name__)\n{ROUTE}@other.route('/y')\ndef g(): pass\n{BROKEN}"
        );
        want(&[("app.py", &app)], "app.py", V::Withheld);
    }
    #[test]
    fn view_source_module_is_broken() {
        let views = format!("{VIEWS}{BROKEN}");
        let app = format!("{ALLOC}{VIEW_REG}");
        want(
            &[("app.py", &app), ("views.py", &views)],
            "app.py",
            V::Withheld,
        );
    }
    #[test]
    fn router_source_module_is_broken() {
        let app = format!("{ALLOC}{BROKEN}");
        want(
            &[("app.py", &app), ("routes.py", ROUTER_USE)],
            "routes.py",
            V::Withheld,
        );
    }
    #[test]
    fn clean_files_beside_a_broken_one_control() {
        let app = format!("{ALLOC}{VIEW_REG}");
        want(
            &[
                ("app.py", &app),
                ("views.py", VIEWS),
                ("unrelated.py", BROKEN),
            ],
            "app.py",
            V::Proved,
        );
    }
}

/// `nonlocal K` in a nested def redirects the write to the setup function's local K.
mod nonlocal {
    use super::*;

    fn setup_verdicts(source: &str) -> Vec<V> {
        let files = parsed(&[
            ("app.py", source),
            ("views.py", "def f(): pass\ndef g(): pass\n"),
        ]);
        let file = &files["app.py"];
        assert!(!file.tree.root_node().has_error(), "failed probe");
        let root = file.tree.root_node();
        let factory = (0..root.named_child_count())
            .filter_map(|i| root.named_child(i))
            .find(|node| {
                node.kind() == "function_definition"
                    && node
                        .child_by_field_name("name")
                        .is_some_and(|name| file.node_text(&name) == "setup")
            })
            .expect("a setup function");
        let start_byte = source.rfind("setup()").unwrap();
        let context = SetupContext {
            factory: Some(PythonMember {
                file,
                node: factory,
            }),
            frames: vec![Invocation::Call {
                file: "app.py".into(),
                start_byte,
                end_byte: start_byte + "setup()".len(),
            }],
        };
        let bindings = index_python(&files);
        flask_facts(&bindings, "app.py", &context)
            .iter()
            .filter_map(|event| match event {
                FlaskEvent::Rule(rule) => Some(verdict(rule)),
                _ => None,
            })
            .collect()
    }
    #[track_caller]
    fn expect_setup(source: &str, want: V) {
        let got = setup_verdicts(source);
        match want {
            V::Proved => assert!(
                !got.is_empty() && !got.contains(&V::Withheld),
                "{got:?}: {source}"
            ),
            V::Withheld => assert!(!got.contains(&V::Proved), "{got:?}: {source}"),
        }
    }

    const ROUTER_NONLOCAL: &str = "from flask import Flask\ndef setup():\n    app = Flask(__name__)\n    def swap():\n        nonlocal app\n        app = Flask(__name__)\n    swap()\n    @app.route('/x')\n    def f(): pass\nsetup()\n";
    const ROUTER_OWN_LOCAL: &str = "from flask import Flask\ndef setup():\n    app = Flask(__name__)\n    def swap():\n        app = Flask(__name__)\n        return app\n    swap()\n    @app.route('/x')\n    def f(): pass\nsetup()\n";
    const VIEW_NONLOCAL: &str = "from flask import Flask\ndef setup():\n    app = Flask(__name__)\n    from views import f as handler\n    def swap():\n        nonlocal handler\n        from views import g as handler\n    swap()\n    app.add_url_rule('/x', view_func=handler)\nsetup()\n";
    const VIEW_OWN_LOCAL: &str = "from flask import Flask\ndef setup():\n    app = Flask(__name__)\n    from views import f as handler\n    def swap():\n        from views import g as handler\n        return handler\n    swap()\n    app.add_url_rule('/x', view_func=handler)\nsetup()\n";

    #[test]
    fn view_local_import_baseline() {
        let source = "from flask import Flask\ndef setup():\n    app = Flask(__name__)\n    from views import f as handler\n    app.add_url_rule('/x', view_func=handler)\nsetup()\n";
        expect_setup(source, V::Proved);
    }
    #[test]
    fn router_rebound_through_nonlocal() {
        expect_setup(ROUTER_NONLOCAL, V::Withheld);
    }
    #[test]
    fn router_own_local_in_nested_def_control() {
        expect_setup(ROUTER_OWN_LOCAL, V::Proved);
    }
    #[test]
    fn view_rebound_through_nonlocal() {
        expect_setup(VIEW_NONLOCAL, V::Withheld);
    }
    #[test]
    fn view_own_local_in_nested_def_control() {
        expect_setup(VIEW_OWN_LOCAL, V::Proved);
    }
}

mod known_limits;
mod used;
