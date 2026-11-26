//! KNOWN_LIMIT rows: the form is out of model, so prism emits what the source says (the
//! declaration, written callback, proved receiver or mount) with no counted refusal. Native
//! runtime observations in the rows earn zero runtime credit and are not asserted.
//!
//! Skipped clauses per row (checkpoint 2): the emitted exposure and emitted binding of every
//! row (fk-01..fk-77 all name them), and the frozen-ledger disposition and runtime-evidence
//! clauses. Asserted: the source declaration, written callback, proved receiver or mount child,
//! declared and effective methods and the inferred endpoint where the row states them.
use super::proven::{
    decorated, doc_expect, doc_view, methods_write_files, mutator_files, post_options,
    reflective_mutator, setup_with, views_f, MODULE_REPLACEMENT,
};
use super::proven_mounts::{relative_package, star_hooks, star_paths, MACHINERY};
use super::*;

fn mounted(imports: &str, child: &str, base: Files) -> Files {
    let mut files = base;
    files.push(("app.py".into(), mount_app(imports, child)));
    files
}

// KNOWN_LIMIT: tuple-index rebind of a module attribute, then `x.get = skip` (post-creation method replacement).
#[test]
fn fk_01() {
    check(
        &c01("pkg.routes = (app,)[0]\npkg.routes.get = skip\n"),
        "app.py",
        rules(&[decorated()]),
        &[],
    );
}

// KNOWN_LIMIT: list-index rebind of a module attribute, then `x.get = skip`.
#[test]
fn fk_02() {
    check(
        &c01("pkg.routes = [app][0]\npkg.routes.get = skip\n"),
        "app.py",
        rules(&[decorated()]),
        &[],
    );
}

// KNOWN_LIMIT: dict-index rebind of a module attribute, then `x.get = skip`.
#[test]
fn fk_03() {
    check(
        &c01("pkg.routes = {'r': app}['r']\npkg.routes.get = skip\n"),
        "app.py",
        rules(&[decorated()]),
        &[],
    );
}

// KNOWN_LIMIT: conditional alias rebind of a module attribute, then `x.get = skip`.
#[test]
fn fk_04() {
    check(
        &c01("owner = app if True else app\npkg.routes = owner\npkg.routes.get = skip\n"),
        "app.py",
        rules(&[decorated()]),
        &[],
    );
}

// KNOWN_LIMIT: reflective `setattr` through a rebound module attribute.
#[test]
fn fk_05() {
    check(
        &c01("pkg.routes = (app,)[0]\nsetattr(pkg.routes, 'get', skip)\n"),
        "app.py",
        rules(&[decorated()]),
        &[],
    );
}

// KNOWN_LIMIT: `del` of a router attribute through a rebound module attribute.
#[test]
fn fk_06() {
    check(
        &c01("pkg.routes = (app,)[0]\ndel pkg.routes.url_map\n"),
        "app.py",
        rules(&[decorated()]),
        &[],
    );
}

// KNOWN_LIMIT: view mutation through a subscript argument (untraced argument path).
#[test]
fn fk_07() {
    let files = files(&[
        (
            "app.py",
            &app("import setup\nfrom views import f\nsetup.patch((f,)[0])\napp.add_url_rule('/x', view_func=f)\n"),
        ),
        ("views.py", "def f(): pass\n"),
        ("setup.py", PATCH_MUT),
    ]);
    check(&files, "app.py", views_f(), &[]);
}

// KNOWN_LIMIT: view mutation through a class attribute reached by a class decorator.
#[test]
fn fk_09() {
    let files = files(&[
        (
            "app.py",
            &app("from views import f\nimport setup\n@setup.touch\nclass Holder: view = f\napp.add_url_rule('/x', view_func=f)\n"),
        ),
        ("views.py", "def f(): pass\n"),
        (
            "setup.py",
            "def touch(cls):\n    cls.view.methods = ['POST']\n    return cls\n",
        ),
    ]);
    check(&files, "app.py", views_f(), &[]);
}

// KNOWN_LIMIT: tuple-mediated `view_functions` map write.
#[test]
fn fk_10() {
    let files = files(&[
        (
            "app.py",
            &app("from views import f\napp.add_url_rule('/x', view_func=f)\ndef wrap(): return f()\nmaps = (app.view_functions,)\nmaps[0]['f'] = wrap\n"),
        ),
        ("views.py", "def f(): pass\n"),
    ]);
    check(&files, "app.py", views_f(), &[]);
}

// KNOWN_LIMIT: view mutation through a container bound to a name (`vs = [f]`).
#[test]
fn fk_25() {
    let files = files(&[
        (
            "app.py",
            &app("from views import f\nfrom setup import patch_all\nvs = [f]\npatch_all(vs)\napp.add_url_rule('/x', view_func=f)\n"),
        ),
        ("views.py", "def f(): pass\n"),
        (
            "setup.py",
            "def patch_all(views):\n    for view in views:\n        view.__name__ = 'renamed'\n        view.methods = ['POST']\n",
        ),
    ]);
    check(&files, "app.py", views_f(), &[]);
}

/// The kept registration of fk-c09 (`/x` at endpoint `shared`, written callback `a`).
fn shared_a() -> Expect {
    rules(&[rule("/x")
        .def("app.py", "a")
        .methods(DEFAULT_METHODS)
        .endpoint("shared")
        .declared_none()])
}

// KNOWN_LIMIT: `view_functions.update(..)` registry mutation.
#[test]
fn fk_26() {
    check(
        &c09("app.view_functions.update({'shared': b})\n"),
        "app.py",
        shared_a(),
        &[],
    );
}

// KNOWN_LIMIT: write through a `view_functions` alias.
#[test]
fn fk_27() {
    check(
        &c09("m = app.view_functions\nm['shared'] = b\n"),
        "app.py",
        shared_a(),
        &[],
    );
}

// KNOWN_LIMIT: `view_functions.pop(..)` registry mutation.
#[test]
fn fk_28() {
    check(
        &c09("app.view_functions.pop('shared')\n"),
        "app.py",
        shared_a(),
        &[],
    );
}

// KNOWN_LIMIT: `del` of a `view_functions` entry.
#[test]
fn fk_29() {
    check(
        &c09("del app.view_functions['shared']\n"),
        "app.py",
        shared_a(),
        &[],
    );
}

// KNOWN_LIMIT: initialization order: a class-body import runs `app` before the def exists.
#[test]
fn fk_38() {
    let files = doc_view("class Init:\n    import app\ndef __doc__(): return \"late\"\n");
    check(&files, "app.py", doc_expect(), &[]);
}

// KNOWN_LIMIT: initialization order: a function-local import runs `app` before the def exists.
#[test]
fn fk_39() {
    let files = doc_view("def init():\n    import app\ninit()\ndef __doc__(): return \"late\"\n");
    check(&files, "app.py", doc_expect(), &[]);
}

// KNOWN_LIMIT: initialization order: an implicit package initializer runs `app` before the methods write.
#[test]
fn fk_40() {
    check(
        &methods_write_files("import app\n"),
        "app.py",
        post_options(),
        &[],
    );
}

// KNOWN_LIMIT: `globals()[..]` namespace store.
#[test]
fn fk_43() {
    let setup = setup_with("globals()[\"patch\"] = _mut\n");
    check(&c03("", &setup), "app.py", views_f(), &[]);
}

// KNOWN_LIMIT: external module attribute store `setup.patch = _mut`.
#[test]
fn fk_44() {
    let mutator = format!("import setup\n{MUT}setup.patch = _mut\n");
    check(&mutator_files(&mutator), "app.py", views_f(), &[]);
}

// KNOWN_LIMIT: keyed `vars(m)[key]` store.
#[test]
fn fk_45() {
    let mutator = format!("import setup as m\n{MUT}key = \"patch\"\nvars(m)[key] = _mut\n");
    check(&mutator_files(&mutator), "app.py", views_f(), &[]);
}

// KNOWN_LIMIT: indirect mutator (`operator.setitem`) through a container-held module dictionary.
#[test]
fn fk_48() {
    let setup = setup_with("import operator\npatch_dict = operator.setitem\npatch_dict((globals(),)[0], \"patch\", _mut)\n");
    check(&c03("", &setup), "app.py", views_f(), &[]);
}

// KNOWN_LIMIT: reflective `vars(target)[..]` store through a function-local import and conditional alias.
#[test]
fn fk_53() {
    check(
        &mutator_files(&reflective_mutator("patch")),
        "app.py",
        views_f(),
        &[],
    );
}

// KNOWN_LIMIT: `sys.modules` replacement.
#[test]
fn fk_66() {
    let files = files(&[
        (
            "app.py",
            &app("from views import f\napp.add_url_rule('/x', view_func=f)\n"),
        ),
        (
            "views.py",
            &format!("import sys\n{MODULE_REPLACEMENT}sys.modules[__name__] = replacement\n"),
        ),
    ]);
    check(&files, "app.py", views_f(), &[]);
}

// KNOWN_LIMIT: `__path__` write in the package initializer (import machinery).
#[test]
fn fk_68() {
    let files = mounted(
        "from pkg.routes import bp\n",
        "bp",
        redirected("pkg", PATH_WRITE),
    );
    check(&files, "app.py", mount("pkg/routes.py"), &[]);
}

// KNOWN_LIMIT: `__package__` write in the capturing module (import machinery).
#[test]
fn fk_71() {
    check(
        &relative_package("__package__ = 'other'\n"),
        "pkg/app.py",
        mount("pkg/routes.py"),
        &[],
    );
}

// KNOWN_LIMIT: wildcard-supplied `__getattr__` hook (outside the explicit lexical hook check).
#[test]
fn fk_72() {
    check(
        &star_hooks("__getattr__"),
        "app.py",
        mount("pkg_s/routes.py"),
        &[],
    );
}

// KNOWN_LIMIT: wildcard-supplied `__path__` (import machinery).
#[test]
fn fk_73() {
    check(
        &star_paths("__path__"),
        "app.py",
        mount("pkg_star/routes.py"),
        &[],
    );
}

// KNOWN_LIMIT: import-machinery writes in the terminal module.
#[test]
fn fk_76() {
    let files = files(&[
        (
            "app.py",
            &app("from views import f\napp.add_url_rule('/x', view_func=f)\n"),
        ),
        (
            "views.py",
            &format!("def f(): return \"first\"\n__path__ = []\n{MACHINERY}"),
        ),
    ]);
    check(&files, "app.py", views_f(), &[]);
}

// KNOWN_LIMIT: import-machinery writes in the hop package initializer.
#[test]
fn fk_77() {
    let init = format!("{PATH_WRITE}{MACHINERY}");
    let files = mounted(
        "import pkg_hop.routes as selected\n",
        "selected.bp",
        redirected("pkg_hop", &init),
    );
    check(&files, "app.py", mount("pkg_hop/routes.py"), &[]);
}
