//! WITHHELD rows about views and callees (counted import refusals and escape refusals).
//!
//! Skipped clauses per row (checkpoint 2 owns exposure and dispatch binding):
//! - dispatch binding withheld: fk-23, fk-41, fk-42, fk-46, fk-52, fk-60, fk-61, fk-62, fk-63,
//!   fk-65 (dependent binding), fk-69, fk-70, fk-75, fk-80, fk-85
//! - exposures empty and no binding: fk-17
//! - fk-81, fk-83 and fk-84 register in a class body or indirect nested function, so their
//!   scopes stay inactive. They assert the refusal and an empty fact list. The retained app,
//!   literal path, endpoint, written methods, unresolved callback and binding clauses are skipped
//! - nothing skipped: fk-08, fk-13, fk-15, fk-16, fk-19, fk-20, fk-21, fk-22, fk-31, fk-33,
//!   fk-34, fk-35, fk-36, fk-37, fk-47
//!
//! Where a row keeps an explicit endpoint and `methods=['GET']` (fk-41, fk-52, fk-61, fk-85) the
//! test asserts the written endpoint and declared methods; the effective method set of an
//! unresolved view is not asserted (the row text does not state it).
use super::*;

#[test]
fn fk_08() {
    let files = files(&[
        (
            "app.py",
            &app("from functools import partial\nimport setup\nfrom views import f\npartial(setup.patch, f)()\napp.add_url_rule('/x', view_func=f)\n"),
        ),
        ("views.py", "def f(): pass\n"),
        ("setup.py", PATCH_MUT),
    ]);
    check(
        &files,
        "app.py",
        rules(&[withheld_callee("/x", "views.py", "f")]),
        &[],
    );
}

#[test]
fn fk_13() {
    let files = files(&[("app.py", &app("@((app,)[0]).get('/x')\ndef f(): pass\n"))]);
    // Flask has no receiver hint for the parenthesised subscript and omits the site.
    check(&files, "app.py", nothing(), &[]);
}

#[test]
fn fk_15() {
    let files = files(&[
        (
            "app.py",
            &app("from views import f\napp.add_url_rule('/x', view_func=(f,)[0])\n"),
        ),
        ("views.py", "def f(): pass\n"),
    ]);
    let spec = rule("/x")
        .unresolved(Reason::CallbackUnproven, "(f,)[0]")
        .methods_unknown()
        .endpoint_unknown();
    check(&files, "app.py", rules(&[spec]), &[]);
}

#[test]
fn fk_16() {
    check(
        &c06(PATCH_MUT),
        "app.py",
        rules(&[withheld_callee("/x", "views.py", "f")]),
        &[],
    );
}

#[test]
fn fk_17() {
    // The direct store of the traced app escapes it: the source stays as a candidate with an
    // unproven receiver and an unresolved callback.
    let spec = rule("/x")
        .unproven(Reason::ReceiverUnproven)
        .unresolved(Reason::ReceiverUnproven, "f");
    check(&c01("pkg.routes = app\n"), "app.py", rules(&[spec]), &[]);
}

const CALL_F: Diag = (MultiWrite, "app.py", "patch(f)", "patch(f)");

#[test]
fn fk_19() {
    let setup = "try:\n    def patch(view):\n        view.__name__ = 'renamed'\n        view.methods = ['POST']\nexcept Exception:\n    pass\nimport app\ndef patch(view): pass\n";
    check(
        &c03("", setup),
        "app.py",
        rules(&[withheld_callee("/x", "views.py", "f")]),
        &[CALL_F],
    );
}

#[test]
fn fk_20() {
    let setup = format!(
        "{PATCH_NOOP}{MUT}def replace():\n    global patch\n    patch = _mut\nreplace()\nimport app\n"
    );
    check(
        &c03("", &setup),
        "app.py",
        rules(&[withheld_callee("/x", "views.py", "f")]),
        &[(GlobalWrite, "app.py", "patch(f)", "patch(f)")],
    );
}

#[test]
fn fk_21() {
    let files = plus(
        c03(
            "",
            "def patch(view): pass\nfrom other import patch\nimport app\n",
        ),
        &[("other.py", PATCH_MUT)],
    );
    check(
        &files,
        "app.py",
        rules(&[withheld_callee("/x", "views.py", "f")]),
        &[CALL_F],
    );
}

#[test]
fn fk_22() {
    let setup = format!("{MUT}def _p(view): pass\npatch = _mut\nimport app\npatch = _p\n");
    check(
        &c03("", &setup),
        "app.py",
        rules(&[withheld_callee("/x", "views.py", "f")]),
        &[CALL_F],
    );
}

#[test]
fn fk_23() {
    let files = files(&[
        (
            "app.py",
            &app("from setup import f\napp.add_url_rule(\"/x\", view_func=f)\n"),
        ),
        (
            "setup.py",
            "def f(): return \"first\"\nimport app\ndef f(): return \"second\"\n",
        ),
    ]);
    check(
        &files,
        "app.py",
        rules(&[withheld_view("/x", "f")]),
        &[(MultiWrite, "app.py", "from setup import f", "f")],
    );
}

#[test]
fn fk_31() {
    let setup = format!(
        "{PATCH_NOOP}{MUT}def __getattr__(name):\n    if name == 'patch':\n        return _mut\n    raise AttributeError(name)\n"
    );
    check(
        &c03("", &setup),
        "app.py",
        rules(&[withheld_callee("/x", "views.py", "f")]),
        &[(GetattrModule, "app.py", "patch(f)", "patch(f)")],
    );
}

#[test]
fn fk_33() {
    let files = plus(
        c11(&format!("{PATCH_NOOP}from support import *\n")),
        &[("support.py", PATCH_MUT)],
    );
    check(
        &files,
        "app.py",
        rules(&[withheld_callee("/x", "views.py", "f")]),
        &[(StarImport, "app.py", "rules.patch(f)", "rules.patch(f)")],
    );
}

#[test]
fn fk_34() {
    let views = format!(
        "{FLASK}def f(): return \"first\"\ndef f(): return \"second\"\nimport helper\napp.add_url_rule('/x', view_func=f)\n"
    );
    let files = files(&[
        ("views.py", &views),
        (
            "helper.py",
            "from views import f\nf.__name__ = 'renamed'\nf.methods = ['POST']\n",
        ),
    ]);
    check(
        &files,
        "views.py",
        rules(&[withheld_callee("/x", "views.py", "f")]),
        &[(MultiWrite, "helper.py", "from views import f", "f")],
    );
    // The written local callback stays at the second definition.
    run(&files, "views.py", |events, _| {
        let [FlaskEvent::Rule(rule)] = events else {
            panic!("one rule")
        };
        let [Callback {
            status: CallbackStatus::Resolved { identity, .. },
            ..
        }] = rule.route.callbacks.as_slice()
        else {
            panic!("resolved callback")
        };
        assert_eq!(identity.start_byte, views.rfind("def f()").unwrap());
    });
}

#[test]
fn fk_35() {
    let setup = format!("{PATCH_NOOP}{MUT}class Holder:\n    global patch\n    patch = _mut\n");
    check(
        &c03("", &setup),
        "app.py",
        rules(&[withheld_callee("/x", "views.py", "f")]),
        &[(GlobalWrite, "app.py", "patch(f)", "patch(f)")],
    );
}

#[test]
fn fk_36() {
    let setup = format!("{PATCH_NOOP}{MUT}match _mut:\n    case patch:\n        pass\n");
    check(
        &c03("", &setup),
        "app.py",
        rules(&[withheld_callee("/x", "views.py", "f")]),
        &[CALL_F],
    );
}

#[test]
fn fk_37() {
    check(
        &c06("if True:\n    def patch(view): pass\n"),
        "app.py",
        rules(&[withheld_callee("/x", "views.py", "f")]),
        &[(NestedSingle, "app.py", "setup.patch(f)", "setup.patch(f)")],
    );
}

fn shadowed_view(init: &str, native: &str) -> Files {
    files(&[("app.py", &app(native)), ("pkg/__init__.py", init)])
}

#[test]
fn fk_41() {
    let mut files = shadowed_view(
        "def view(): return \"package def\"\n",
        "import preload\nfrom pkg import view\napp.add_url_rule(\"/x\", endpoint=\"x\", view_func=view, methods=[\"GET\"])\n",
    );
    files.extend(super::files(&[
        ("preload.py", "import pkg.view\n"),
        ("pkg/view.py", ""),
    ]));
    let spec = rule("/x")
        .unresolved(Reason::ImportedUnproven, "view")
        .endpoint("x")
        .declared(&["GET"]);
    check(
        &files,
        "app.py",
        rules(&[spec]),
        &[(SubmoduleShadow, "app.py", "from pkg import view", "view")],
    );
}

#[test]
fn fk_42() {
    let mut files = shadowed_view(
        "def handler(): return \"package def\"\nfrom .handler import helper\n",
        "from pkg import handler\napp.add_url_rule(\"/x\", view_func=handler)\n",
    );
    files.extend(super::files(&[(
        "pkg/handler.py",
        "def helper(): pass\nmethods = [\"POST\"]\n",
    )]));
    check(
        &files,
        "app.py",
        rules(&[withheld_view("/x", "handler")]),
        &[(
            SubmoduleShadow,
            "app.py",
            "from pkg import handler",
            "handler",
        )],
    );
}

#[test]
fn fk_46() {
    let files = files(&[
        (
            "app.py",
            &app("from views import f\napp.add_url_rule(\"/x\", view_func=f)\n"),
        ),
        (
            "views.py",
            "def f(): return \"first\"\ndef other(): return \"other\"\n[(f := other) for _ in (0,)]\n",
        ),
    ]);
    check(
        &files,
        "app.py",
        rules(&[withheld_view("/x", "f")]),
        &[(MultiWrite, "app.py", "from views import f", "f")],
    );
}

#[test]
fn fk_47() {
    let files = files(&[
        (
            "app.py",
            "from setup import api\n@api.get(\"/x\")\ndef f(): pass\n",
        ),
        (
            "setup.py",
            "from flask import Flask\napi = Flask(__name__)\napi = Flask(__name__)\n",
        ),
    ]);
    // No local receiver hint: the listing drops the site and still reports the diagnostic.
    check(
        &files,
        "app.py",
        nothing(),
        &[(MultiWrite, "app.py", "from setup import api", "api")],
    );
}

#[test]
fn fk_52() {
    let files = files(&[
        (
            "app.py",
            &app("from views import __dict__ as f\napp.add_url_rule(\"/x\", endpoint=\"x\", view_func=f, methods=[\"GET\"])\n"),
        ),
        ("views.py", "def __dict__(): return \"view\"\n"),
    ]);
    let spec = rule("/x")
        .unresolved(Reason::ImportedUnproven, "f")
        .endpoint("x")
        .declared(&["GET"]);
    check(
        &files,
        "app.py",
        rules(&[spec]),
        &[(
            ModuleDescriptorName,
            "app.py",
            "from views import __dict__ as f",
            "__dict__",
        )],
    );
}

#[test]
fn fk_60() {
    let files = files(&[
        (
            "app.py",
            &app("from views import f\napp.add_url_rule('/x', view_func=f)\napp.add_url_rule('/y', view_func=f)\n"),
        ),
        (
            "views.py",
            "def f(): return \"first\"\ndef \u{ff46}(): return \"second\"\n",
        ),
    ]);
    // One diagnostic for the one refused capture used by both registrations.
    check(
        &files,
        "app.py",
        rules(&[withheld_view("/x", "f"), withheld_view("/y", "f")]),
        &[(MultiWrite, "app.py", "from views import f", "f")],
    );
}

#[test]
fn fk_61() {
    let name = "__\u{ff44}\u{ff49}\u{ff43}\u{ff54}__";
    let call =
        format!("app.add_url_rule('/x', endpoint='x', view_func=views.{name}, methods=['GET'])");
    let files = files(&[
        ("app.py", &app(&format!("import views\n{call}\n"))),
        ("views.py", &format!("def {name}(): return \"first\"\n")),
    ]);
    let text = leak(format!("views.{name}"));
    let spec = rule("/x")
        .unresolved(Reason::ImportedUnproven, text)
        .endpoint("x")
        .declared(&["GET"]);
    check(
        &files,
        "app.py",
        rules(&[spec]),
        &[(
            ModuleDescriptorName,
            "app.py",
            leak(format!("view_func={text}")),
            text,
        )],
    );
}

#[test]
fn fk_62() {
    let files = files(&[
        (
            "app.py",
            &app("from views import f\napp.add_url_rule('/x', view_func=f)\n"),
        ),
        (
            "views.py",
            "def f(): return \"first\"\ndef other(): return \"second\"\nf: object = other\n",
        ),
    ]);
    check(
        &files,
        "app.py",
        rules(&[withheld_view("/x", "f")]),
        &[(MultiWrite, "app.py", "from views import f", "f")],
    );
}

#[test]
fn fk_63() {
    let files = files(&[
        (
            "app.py",
            &app("from views import f\napp.add_url_rule('/x', view_func=f)\n"),
        ),
        ("views.py", "def f(): return \"first\"\ntype f = int\n"),
    ]);
    check(
        &files,
        "app.py",
        rules(&[withheld_view("/x", "f")]),
        &[(MultiWrite, "app.py", "from views import f", "f")],
    );
}

#[test]
fn fk_65() {
    let setup = "def patch(view): pass\ndef patch(view):\n    view.__name__ = 'renamed_' + view.__name__\n    view.methods = ['POST']\n";
    let files = files(&[
        (
            "app.py",
            &app("from setup import patch\nfrom views import f, g\npatch(f)\npatch(g)\napp.add_url_rule('/x', view_func=f)\napp.add_url_rule('/y', view_func=g)\n"),
        ),
        ("views.py", "def f(): pass\ndef g(): pass\n"),
        ("setup.py", setup),
    ]);
    // One diagnostic at each distinct whole-call span of the refused callee.
    check(
        &files,
        "app.py",
        rules(&[
            withheld_callee("/x", "views.py", "f"),
            withheld_callee("/y", "views.py", "g"),
        ]),
        &[
            (MultiWrite, "app.py", "patch(f)", "patch(f)"),
            (MultiWrite, "app.py", "patch(g)", "patch(g)"),
        ],
    );
}

const FK69_VIEWS: &str = "def _Install__f(): return \"first\"\ndef __f(): return \"raw\"\nclass Install:\n    global __f\n    def __f(): return \"second\"\n";

#[test]
fn fk_69() {
    let files = files(&[
        (
            "app.py",
            &app("from views import _Install__f as f\napp.add_url_rule('/x', view_func=f)\n"),
        ),
        ("views.py", FK69_VIEWS),
    ]);
    check(
        &files,
        "app.py",
        rules(&[withheld_view("/x", "f")]),
        &[(
            GlobalWrite,
            "app.py",
            "from views import _Install__f as f",
            "_Install__f",
        )],
    );
}

#[test]
fn fk_70() {
    let views = format!("{FK69_VIEWS}def f(): return \"ordinary\"\n");
    let files = files(&[
        (
            "app.py",
            &app("from views import __f as f\napp.add_url_rule('/x', view_func=f)\n"),
        ),
        ("views.py", &views),
    ]);
    check(
        &files,
        "app.py",
        rules(&[withheld_view("/x", "f")]),
        &[(
            ClassPrivateName,
            "app.py",
            "from views import __f as f",
            "__f",
        )],
    );
}

#[test]
fn fk_75() {
    let files = files(&[
        (
            "app.py",
            &app("from \u{ff56}\u{ff49}\u{ff45}\u{ff57}\u{ff53} import f\napp.add_url_rule('/x', view_func=f)\n"),
        ),
        (
            "\u{ff56}\u{ff49}\u{ff45}\u{ff57}\u{ff53}.py",
            "def f(): return \"fullwidth\"\n",
        ),
    ]);
    // The import key has no eligible file: unproven without a counted reason.
    check(&files, "app.py", rules(&[withheld_view("/x", "f")]), &[]);
}

#[test]
fn fk_80() {
    let files = files(&[
        (
            "app.py",
            &app("from views import f\napp.add_url_rule('/x', view_func=f)\n"),
        ),
        ("views.py", "from impl import handler\nf = handler\n"),
        ("impl.py", "def handler(): return \"first\"\n"),
    ]);
    // A terminal assignment alias re-exporting an import: no counted reason.
    check(&files, "app.py", rules(&[withheld_view("/x", "f")]), &[]);
}

#[test]
fn fk_81() {
    let files = files(&[
        (
            "app.py",
            &app("class C:\n    from views import f\n    if True:\n        app.add_url_rule('/x', endpoint='x', view_func=f, methods=['GET'])\n"),
        ),
        ("views.py", "def f(): return \"first\"\n"),
    ]);
    check(
        &files,
        "app.py",
        nothing(),
        &[(ClassBodyImport, "app.py", "from views import f", "f")],
    );
}

const FK82_VIEWS: &str =
    "def __f(): return \"raw\"\ndef _C__f(): return \"mangled\"\ndef f(): return \"first\"\n";

#[test]
fn fk_82() {
    let files = files(&[
        (
            "app.py",
            &app("class C:\n    def install(self):\n        from views import __f as handler\n        app.add_url_rule('/x', endpoint='x', view_func=handler, methods=['GET'])\nC().install()\n"),
        ),
        ("views.py", FK82_VIEWS),
    ]);
    check(
        &files,
        "app.py",
        rules(&[withheld_view("/x", "handler")
            .endpoint("x")
            .declared(&["GET"])]),
        &[(
            ClassPrivateName,
            "app.py",
            "from views import __f as handler",
            "__f",
        )],
    );
}

#[test]
fn fk_83() {
    let files = files(&[
        (
            "app.py",
            &app("class C:\n    def install(self):\n        def inner():\n            from views import __f\n            app.add_url_rule('/x', endpoint='x', view_func=__f, methods=['GET'])\n        inner()\nC().install()\n"),
        ),
        ("views.py", FK82_VIEWS),
    ]);
    check(
        &files,
        "app.py",
        nothing(),
        &[(ClassPrivateName, "app.py", "from views import __f", "__f")],
    );
}

#[test]
fn fk_84() {
    let call = "app.add_url_rule('/x', endpoint='x', view_func=views.__f, methods=['GET'])";
    let files = files(&[
        (
            "app.py",
            &app(&format!("import views\nclass C:\n    {call}\n")),
        ),
        ("views.py", FK82_VIEWS),
    ]);
    check(
        &files,
        "app.py",
        nothing(),
        &[(ClassPrivateName, "app.py", call, "views.__f")],
    );
}

#[test]
fn fk_85() {
    let call = "app.add_url_rule('/x', endpoint='x', view_func=views.__f, methods=['GET'])";
    let files = files(&[
        ("app.py", &app(&format!("import views\n{call}\n"))),
        ("views.py", FK82_VIEWS),
    ]);
    // Accepted over-refusal: the module-scope read is withheld on spelling.
    let spec = rule("/x")
        .unresolved(Reason::ImportedUnproven, "views.__f")
        .endpoint("x")
        .declared(&["GET"]);
    check(
        &files,
        "app.py",
        rules(&[spec]),
        &[(ClassPrivateName, "app.py", call, "views.__f")],
    );
}
