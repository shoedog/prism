//! PROVEN controls (and fk-74) for views, callees and imported receivers: the proof stands and
//! no refusal is counted.
//!
//! Skipped clauses per row (checkpoint 2): the dispatch binding and full-exposure clauses of
//! every row here (rows whose proof is "same as fk-c02/fk-c03" include them by reference) and
//! every native observation. The tests pin receiver, path, callback, endpoint and methods.
//! fk-c83 and fk-c84 assert only the empty refusal multiset because the indirect nested
//! function and class body stay inactive.
use super::*;

/// The `@app.get('/x')` proof of fk-c01 and the KNOWN_LIMIT rows that compare to it.
pub(super) fn decorated() -> RuleSpec {
    rule("/x")
        .def("app.py", "f")
        .methods(DEFAULT_METHODS)
        .endpoint("f")
        .declared(&["GET"])
}

pub(super) const NOOP_MUT: &str = "def patch(view): pass\ndef _mut(view):\n    view.__name__ = 'renamed'\n    view.methods = ['POST']\n";

/// A proof of `/x` to `views.f`, the shape shared by fk-c02/c03/c06/c11 and rows that recover it.
pub(super) fn views_f() -> Expect {
    rules(&[proven_view("/x", "views.py", "f")])
}

#[test]
fn fk_c01() {
    check(&c01(""), "app.py", rules(&[decorated()]), &[]);
}

#[test]
fn fk_c02() {
    check(&c02(), "app.py", views_f(), &[]);
}

#[test]
fn fk_c03() {
    check(&c03("", PATCH_NOOP), "app.py", views_f(), &[]);
}

#[test]
fn fk_c06() {
    check(&c06(PATCH_NOOP), "app.py", views_f(), &[]);
}

#[test]
fn fk_c09() {
    let spec = rule("/x")
        .def("app.py", "a")
        .methods(DEFAULT_METHODS)
        .endpoint("shared")
        .declared_none();
    check(&c09(""), "app.py", rules(&[spec]), &[]);
}

#[test]
fn fk_c10() {
    let views = "from flask import Blueprint\nbp = Blueprint('views', __name__)\n@bp.get('/a')\ndef f(): pass\n";
    let files = files(&[
        (
            "app.py",
            &app("from views import f\napp.add_url_rule('/x', view_func=f)\n"),
        ),
        ("views.py", views),
    ]);
    check(&files, "app.py", views_f(), &[]);
}

#[test]
fn fk_c11() {
    check(&c11(PATCH_NOOP), "app.py", views_f(), &[]);
}

#[test]
fn fk_c12() {
    let setup = format!("{NOOP_MUT}class Holder:\n    patch = _mut\n");
    check(&c03("", &setup), "app.py", views_f(), &[]);
}

#[test]
fn fk_c13() {
    let setup = format!("{NOOP_MUT}match _mut:\n    case other:\n        pass\n");
    check(&c03("", &setup), "app.py", views_f(), &[]);
}

#[test]
fn fk_c15() {
    let views = format!(
        "{FLASK}def f(): return \"second\"\nimport helper\nhelper.patch(f)\napp.add_url_rule('/x', view_func=f)\n"
    );
    let files = files(&[("views.py", &views), ("helper.py", PATCH_NOOP)]);
    check(
        &files,
        "views.py",
        rules(&[proven_view("/x", "views.py", "f")]),
        &[],
    );
}

#[test]
fn fk_c19() {
    let setup = "try:\n    def alternate(view):\n        view.__name__ = 'renamed'\n        view.methods = ['POST']\nexcept Exception:\n    pass\ndef patch(view): pass\n";
    check(&c03("", setup), "app.py", views_f(), &[]);
}

#[test]
fn fk_c20() {
    let setup = format!(
        "{NOOP_MUT}def replace():\n    global alternate\n    alternate = _mut\nreplace()\n"
    );
    check(&c03("", &setup), "app.py", views_f(), &[]);
}

#[test]
fn fk_c21() {
    let files = plus(
        c03(
            "",
            "def patch(view): pass\nfrom other import patch as alternate\n",
        ),
        &[("other.py", PATCH_MUT)],
    );
    check(&files, "app.py", views_f(), &[]);
}

#[test]
fn fk_c22() {
    let setup = format!(
        "{MUT}def patch(view): pass\ndef _p(view): pass\nalternate = _mut\nalternate = _p\n"
    );
    check(&c03("", &setup), "app.py", views_f(), &[]);
}

#[test]
fn fk_c23() {
    let files = files(&[
        (
            "app.py",
            &app("from setup import f\napp.add_url_rule(\"/x\", view_func=f)\n"),
        ),
        ("setup.py", "def f(): return \"first\"\n"),
    ]);
    check(
        &files,
        "app.py",
        rules(&[proven_view("/x", "setup.py", "f")]),
        &[],
    );
}

/// fk-38/fk-c38 and fk-39/fk-c39: the captured `__doc__` def registered at endpoint `x`.
fn doc_view(views: &str) -> Files {
    files(&[
        (
            "app.py",
            &app("from views import __doc__ as f\napp.add_url_rule(\"/x\", endpoint=\"x\", view_func=f, methods=[\"GET\"])\n"),
        ),
        ("views.py", views),
    ])
}

// `__doc__` is a module-system name: it counts as one more binding of the key (clause d), so a
// def of it is not one static binding and the view is withheld (it was proved before).
fn doc_withheld() -> Expect {
    rules(&[rule("/x")
        .unresolved(Reason::ImportedUnproven, "f")
        .endpoint("x")
        .methods_unknown()
        .declared(&["GET"])])
}

const DOC_REFUSAL: [Diag; 1] = [(
    MultiWrite,
    "app.py",
    "from views import __doc__ as f",
    "__doc__",
)];

#[test]
fn fk_c38() {
    let files = doc_view("class Init:\n    pass\ndef __doc__(): return \"late\"\n");
    check(&files, "app.py", doc_withheld(), &DOC_REFUSAL);
}

#[test]
fn fk_c39() {
    let files = doc_view("def init(): pass\ninit()\ndef __doc__(): return \"late\"\n");
    check(&files, "app.py", doc_withheld(), &DOC_REFUSAL);
}

pub(super) fn methods_write_files(pkg_init: &str) -> Files {
    files(&[
        (
            "app.py",
            &app("from views import f\napp.add_url_rule(\"/x\", view_func=f)\n"),
        ),
        (
            "views.py",
            "def f(): pass\nimport pkg.util\nf.methods = [\"POST\"]\n",
        ),
        ("pkg/__init__.py", pkg_init),
        ("pkg/util.py", ""),
    ])
}

pub(super) fn post_options() -> Expect {
    rules(&[proven_view("/x", "views.py", "f").methods(&["POST", "OPTIONS"])])
}

#[test]
fn fk_c40() {
    check(&methods_write_files(""), "app.py", post_options(), &[]);
}

#[test]
fn fk_c41() {
    let files = files(&[
        (
            "app.py",
            &app("import preload\nfrom pkg import view\napp.add_url_rule(\"/x\", endpoint=\"x\", view_func=view, methods=[\"GET\"])\n"),
        ),
        ("preload.py", "import pkg.preloaded\n"),
        ("pkg/__init__.py", "def view(): return \"package def\"\n"),
        ("pkg/preloaded.py", ""),
    ]);
    let spec = rule("/x")
        .def("pkg/__init__.py", "view")
        .endpoint("x")
        .methods(DEFAULT_METHODS)
        .declared(&["GET"]);
    check(&files, "app.py", rules(&[spec]), &[]);
}

#[test]
fn fk_c42() {
    let files = files(&[
        (
            "app.py",
            &app("from pkg import handler\napp.add_url_rule(\"/x\", view_func=handler)\n"),
        ),
        (
            "pkg/__init__.py",
            "def handler(): return \"package def\"\nfrom .support import helper\n",
        ),
        (
            "pkg/support.py",
            "def helper(): pass\nmethods = [\"POST\"]\n",
        ),
    ]);
    check(
        &files,
        "app.py",
        rules(&[proven_view("/x", "pkg/__init__.py", "handler")]),
        &[],
    );
}

pub(super) fn setup_with(extra: &str) -> String {
    format!("{NOOP_MUT}{extra}")
}

#[test]
fn fk_c43() {
    let setup = setup_with("globals()[\"unused\"] = _mut\n");
    check(&c03("", &setup), "app.py", views_f(), &[]);
}

pub(super) fn mutator_files(mutator: &str) -> Files {
    plus(
        c03("import mutator\n", PATCH_NOOP),
        &[("mutator.py", mutator)],
    )
}

#[test]
fn fk_c44() {
    // At 56a9a0d4 the disjoint `setup.unused = _mut` store invalidated the whole module for
    // the no-effect exemption; the amendment recovers `setup.patch`.
    let mutator = format!("import setup\n{MUT}setup.unused = _mut\n");
    check(&mutator_files(&mutator), "app.py", views_f(), &[]);
}

#[test]
fn fk_c45() {
    let mutator = format!("import setup as m\n{MUT}vars(m)[\"unused\"] = _mut\n");
    check(&mutator_files(&mutator), "app.py", views_f(), &[]);
}

pub(super) fn walrus_files(target: &str) -> Files {
    files(&[
        (
            "app.py",
            &app("from views import f\napp.add_url_rule(\"/x\", view_func=f)\n"),
        ),
        (
            "views.py",
            &format!("def f(): return \"first\"\ndef other(): return \"other\"\n[({target} := other) for _ in (0,)]\n"),
        ),
    ])
}

#[test]
fn fk_c46() {
    check(&walrus_files("unused"), "app.py", views_f(), &[]);
}

#[test]
fn fk_c47() {
    let files = files(&[
        (
            "app.py",
            "from setup import api\n@api.get(\"/x\")\ndef f(): pass\n",
        ),
        (
            "setup.py",
            "from flask import Flask\nother = Flask(__name__)\napi = Flask(__name__)\n",
        ),
    ]);
    let spec = rule("/x")
        .def("app.py", "f")
        .declared(&["GET"])
        .methods(DEFAULT_METHODS)
        .endpoint("f");
    check(&files, "app.py", rules(&[spec]), &[]);
}

pub(super) fn descriptor_files(name: &str) -> Files {
    files(&[
        (
            "app.py",
            &app(&format!("from views import {name} as f\napp.add_url_rule(\"/x\", endpoint=\"x\", view_func=f, methods=[\"GET\"])\n")),
        ),
        ("views.py", &format!("def {name}(): return \"view\"\n")),
    ])
}

#[test]
fn fk_c52() {
    let spec = rule("/x")
        .def("views.py", "ordinary_view")
        .endpoint("x")
        .methods(DEFAULT_METHODS)
        .declared(&["GET"]);
    check(
        &descriptor_files("ordinary_view"),
        "app.py",
        rules(&[spec]),
        &[],
    );
}

pub(super) fn reflective_mutator(key: &str) -> String {
    format!("def change(view):\n    view.__name__ = 'renamed'\n    view.methods = ['POST']\ndef rewrite():\n    import setup as source\n    target = source if __debug__ else None\n    vars(target)[\"{key}\"] = change\nrewrite()\n")
}

#[test]
fn fk_c53() {
    check(
        &mutator_files(&reflective_mutator("unused")),
        "app.py",
        views_f(),
        &[],
    );
}

pub(super) fn two_defs(second: &str) -> Files {
    files(&[
        (
            "app.py",
            &app("from views import f\napp.add_url_rule('/x', view_func=f)\napp.add_url_rule('/y', view_func=f)\n"),
        ),
        (
            "views.py",
            &format!("def f(): return \"first\"\ndef {second}(): return \"second\"\n"),
        ),
    ])
}

#[test]
fn fk_c60() {
    check(
        &two_defs("\u{ff47}"),
        "app.py",
        rules(&[
            proven_view("/x", "views.py", "f"),
            proven_view("/y", "views.py", "f"),
        ]),
        &[],
    );
}

#[test]
fn fk_c61() {
    let name = "\u{ff4f}\u{ff52}\u{ff44}\u{ff49}\u{ff4e}\u{ff41}\u{ff52}\u{ff59}";
    let files = files(&[
        (
            "app.py",
            &app(&format!(
                "import views\napp.add_url_rule('/x', endpoint='x', view_func=views.{name}, methods=['GET'])\n"
            )),
        ),
        ("views.py", &format!("def {name}(): return \"first\"\n")),
    ]);
    // The normalised spelling of the callable name is not asserted.
    let spec = rule("/x")
        .def_in("views.py")
        .endpoint("x")
        .methods(DEFAULT_METHODS)
        .declared(&["GET"]);
    check(&files, "app.py", rules(&[spec]), &[]);
}

pub(super) fn assigned(tail: &str) -> Files {
    files(&[
        (
            "app.py",
            &app("from views import f\napp.add_url_rule('/x', view_func=f)\n"),
        ),
        (
            "views.py",
            &format!("def f(): return \"first\"\ndef other(): return \"second\"\n{tail}"),
        ),
    ])
}

#[test]
fn fk_c62() {
    check(&assigned("f: object\n"), "app.py", views_f(), &[]);
}

#[test]
fn fk_c63() {
    check(&assigned("type other = int\n"), "app.py", views_f(), &[]);
}

#[test]
fn fk_c65() {
    let setup = "def patch(view): pass\ndef unused(view):\n    view.__name__ = 'renamed_' + view.__name__\n    view.methods = ['POST']\n";
    let files = files(&[
        (
            "app.py",
            &app("from setup import patch\nfrom views import f, g\npatch(f)\npatch(g)\napp.add_url_rule('/x', view_func=f)\napp.add_url_rule('/y', view_func=g)\n"),
        ),
        ("views.py", "def f(): pass\ndef g(): pass\n"),
        ("setup.py", setup),
    ]);
    check(
        &files,
        "app.py",
        rules(&[
            proven_view("/x", "views.py", "f"),
            proven_view("/y", "views.py", "g"),
        ]),
        &[],
    );
}

pub(super) const MODULE_REPLACEMENT: &str = "def f(): return \"first\"\nfrom types import ModuleType\nreplacement = ModuleType(__name__)\ndef runtime_view(): return replacement\nreplacement.f = runtime_view\n";

#[test]
fn fk_c66() {
    let files = files(&[
        (
            "app.py",
            &app("from views import f\napp.add_url_rule('/x', view_func=f)\n"),
        ),
        ("views.py", MODULE_REPLACEMENT),
    ]);
    check(&files, "app.py", views_f(), &[]);
}

pub(super) const PRIVATE_VIEWS: &str = "def _Install__f(): return \"first\"\ndef __f(): return \"raw\"\nclass Install:\n    def __f(): return \"second\"\n";

#[test]
fn fk_c69() {
    let files = files(&[
        (
            "app.py",
            &app("from views import _Install__f as f\napp.add_url_rule('/x', view_func=f)\n"),
        ),
        ("views.py", PRIVATE_VIEWS),
    ]);
    check(
        &files,
        "app.py",
        rules(&[proven_view("/x", "views.py", "_Install__f")]),
        &[],
    );
}

#[test]
fn fk_c70() {
    let views = format!("{PRIVATE_VIEWS}def f(): return \"ordinary\"\n");
    let files = files(&[
        (
            "app.py",
            &app("from views import f\napp.add_url_rule('/x', view_func=f)\n"),
        ),
        ("views.py", &views),
    ]);
    check(&files, "app.py", views_f(), &[]);
}

pub(super) fn fullwidth_import(with_ascii: bool, with_fullwidth: bool) -> Files {
    let fw = "\u{ff56}\u{ff49}\u{ff45}\u{ff57}\u{ff53}";
    let mut list = vec![(
        "app.py".to_owned(),
        app(&format!(
            "from {fw} import f\napp.add_url_rule('/x', view_func=f)\n"
        )),
    )];
    if with_ascii {
        list.push(("views.py".into(), "def f(): return \"first\"\n".into()));
    }
    if with_fullwidth {
        list.push((format!("{fw}.py"), "def f(): return \"fullwidth\"\n".into()));
    }
    list
}

#[test]
fn fk_74() {
    // NFKC makes the import key `views`; the ASCII filename supplies the binding.
    check(&fullwidth_import(true, true), "app.py", views_f(), &[]);
}

#[test]
fn fk_c74() {
    check(&fullwidth_import(true, false), "app.py", views_f(), &[]);
}

#[test]
fn fk_c76() {
    check(&c02(), "app.py", views_f(), &[]);
}

#[test]
fn fk_c80() {
    let files = files(&[
        (
            "app.py",
            &app("from impl import handler as f\napp.add_url_rule('/x', view_func=f)\n"),
        ),
        ("impl.py", "def handler(): return \"first\"\n"),
    ]);
    check(
        &files,
        "app.py",
        rules(&[proven_view("/x", "impl.py", "handler")]),
        &[],
    );
}

pub(super) fn explicit_x(file: &'static str, name: &'static str) -> Expect {
    rules(&[rule("/x")
        .def(file, name)
        .endpoint("x")
        .methods(DEFAULT_METHODS)
        .declared(&["GET"])])
}

const VIEWS_F: &str = "def f(): return \"first\"\n";

#[test]
fn fk_c81() {
    let files = files(&[
        (
            "app.py",
            &app("from views import f\napp.add_url_rule('/x', endpoint='x', view_func=f, methods=['GET'])\n"),
        ),
        ("views.py", VIEWS_F),
    ]);
    check(&files, "app.py", explicit_x("views.py", "f"), &[]);
}

const FK82_VIEWS: &str =
    "def __f(): return \"raw\"\ndef _C__f(): return \"mangled\"\ndef f(): return \"first\"\n";

#[test]
fn fk_c82() {
    let files = files(&[
        (
            "app.py",
            &app("class C:\n    def install(self):\n        from views import f as handler\n        app.add_url_rule('/x', endpoint='x', view_func=handler, methods=['GET'])\nC().install()\n"),
        ),
        ("views.py", FK82_VIEWS),
    ]);
    check(&files, "app.py", explicit_x("views.py", "f"), &[]);
}

#[test]
fn fk_c83() {
    let files = files(&[
        (
            "app.py",
            &app("class C:\n    def install(self):\n        def inner():\n            from views import f\n            app.add_url_rule('/x', endpoint='x', view_func=f, methods=['GET'])\n        inner()\nC().install()\n"),
        ),
        ("views.py", FK82_VIEWS),
    ]);
    check(&files, "app.py", nothing(), &[]);
}

#[test]
fn fk_c84() {
    let files = files(&[
        (
            "app.py",
            &app("import views\nclass C:\n    app.add_url_rule('/x', endpoint='x', view_func=views.f, methods=['GET'])\n"),
        ),
        ("views.py", FK82_VIEWS),
    ]);
    check(&files, "app.py", nothing(), &[]);
}

#[test]
fn fk_c85() {
    let files = files(&[
        (
            "app.py",
            &app("import views\napp.add_url_rule('/x', endpoint='x', view_func=views.f, methods=['GET'])\n"),
        ),
        ("views.py", FK82_VIEWS),
    ]);
    check(&files, "app.py", explicit_x("views.py", "f"), &[]);
}
