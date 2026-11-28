//! WITHHELD rows about Blueprint mounts through imported modules (`mount_child_unproven`).
//!
//! Skipped clauses per row (checkpoint 2): "no full exposure or binding through the refused
//! mount" on fk-14, fk-24, fk-32, fk-49, fk-50, fk-51, fk-54, fk-55, fk-57, fk-58, fk-59, fk-67,
//! fk-78 and fk-79; the independent child declarations of those rows live in the child module,
//! not in the entry file's facts, and are not asserted (fk-14's own Blueprint declaration is).
use super::*;

/// `import <import>`-style app registering `pkg.routes.bp`-like expressions.
fn app_files(imports: &str, child: &str, base: Files) -> Files {
    let mut files = base;
    files.push(("app.py".into(), mount_app(imports, child)));
    files
}

#[test]
fn fk_14() {
    let files = files(&[(
        "app.py",
        &app("bp = Blueprint('b', __name__)\n@bp.get('/x')\ndef f(): pass\napp.register_blueprint((bp,)[0])\n"),
    )]);
    // The Blueprint declaration stays; the mount child is unproven.
    check(
        &files,
        "app.py",
        rules(&[rule("/x").def("app.py", "f")]).with_mount(None),
        &[],
    );
}

#[test]
fn fk_24() {
    let routes = "from flask import Blueprint\nbp = Blueprint('first', __name__)\n@bp.get('/first')\ndef first(): pass\nbp = Blueprint('second', __name__)\n@bp.get('/second')\ndef second(): pass\n";
    let files = app_files(
        "import pkg_dupe.routes\n",
        "pkg_dupe.routes.bp",
        files(&[("pkg_dupe/__init__.py", ""), ("pkg_dupe/routes.py", routes)]),
    );
    check(
        &files,
        "app.py",
        mount_unproven(),
        &[(
            MultiWrite,
            "app.py",
            "app.register_blueprint(pkg_dupe.routes.bp)",
            "pkg_dupe.routes.bp",
        )],
    );
}

#[test]
fn fk_32() {
    let files = app_files(
        "import pkg.routes\n",
        "pkg.routes.bp",
        pkg_routes(
            "pkg",
            "def __getattr__(name):\n    raise AttributeError(name)\n",
            "/x",
        ),
    );
    check(
        &files,
        "app.py",
        mount_unproven(),
        &[(
            GetattrModule,
            "app.py",
            "app.register_blueprint(pkg.routes.bp)",
            "pkg.routes.bp",
        )],
    );
}

#[test]
fn fk_49() {
    let files = app_files(
        "import pkg_plain\n",
        "pkg_plain.routes.bp",
        pair("pkg_plain", COMPETING_INIT),
    );
    check(
        &files,
        "app.py",
        mount_unproven(),
        &[(
            MultiWrite,
            "app.py",
            "app.register_blueprint(pkg_plain.routes.bp)",
            "pkg_plain.routes.bp",
        )],
    );
}

#[test]
fn fk_50() {
    let files = app_files(
        "import pkg_dotted.routes as selected\n",
        "selected.bp",
        pair("pkg_dotted", "from . import replacement as routes\n"),
    );
    check(
        &files,
        "app.py",
        mount_unproven(),
        &[(
            SubmoduleShadow,
            "app.py",
            "app.register_blueprint(selected.bp)",
            "selected.bp",
        )],
    );
}

#[test]
fn fk_51() {
    let files = app_files(
        "import pkg_cached.routes as selected\n",
        "selected.bp",
        pair("pkg_cached", COMPETING_INIT),
    );
    check(
        &files,
        "app.py",
        mount_unproven(),
        &[(
            MultiWrite,
            "app.py",
            "app.register_blueprint(selected.bp)",
            "selected.bp",
        )],
    );
}

/// fk-54 shape: an empty initializer and `import pkg` only; the `routes` hop has no binding.
fn fk_54_files() -> Files {
    app_files(
        "import pkg\n",
        "pkg.routes.bp",
        pkg_routes("pkg", "", "/routes"),
    )
}

#[test]
fn fk_54() {
    // Unestablished hop: no counted reason, only the role-specific refusal.
    check(&fk_54_files(), "app.py", mount_unproven(), &[]);
}

#[test]
fn fk_55() {
    for unrelated in ["tests/test_unrelated.py", "unrelated.py"] {
        let files = plus(fk_54_files(), &[(unrelated, "import pkg.routes\n")]);
        check(&files, "app.py", mount_unproven(), &[]);
    }
}

#[test]
fn fk_57() {
    let files = app_files(
        "import pkg_assign.routes as selected\n",
        "selected.bp",
        pair(
            "pkg_assign",
            "from . import replacement\nroutes = replacement\n",
        ),
    );
    check(
        &files,
        "app.py",
        mount_unproven(),
        &[(
            SubmoduleShadow,
            "app.py",
            "app.register_blueprint(selected.bp)",
            "selected.bp",
        )],
    );
}

#[test]
fn fk_58() {
    let init =
        "from .routes import bp as _first\nfrom . import replacement\nroutes = replacement\n";
    let files = app_files(
        "import pkg_assign_cached.routes as selected\n",
        "selected.bp",
        pair("pkg_assign_cached", init),
    );
    check(
        &files,
        "app.py",
        mount_unproven(),
        &[(
            SubmoduleShadow,
            "app.py",
            "app.register_blueprint(selected.bp)",
            "selected.bp",
        )],
    );
}

pub(super) const HOOK: &str = "from . import replacement\ndef __getattr__(name):\n    if name == 'routes':\n        return replacement\n    raise AttributeError(name)\n";

#[test]
fn fk_59() {
    let mut files = app_files(
        "from pkg_g import routes\n",
        "routes.bp",
        pair("pkg_g", "from ._hook import __getattr__\n"),
    );
    files.push(("pkg_g/_hook.py".into(), HOOK.into()));
    check(
        &files,
        "app.py",
        mount_unproven(),
        &[(
            GetattrModule,
            "app.py",
            "app.register_blueprint(routes.bp)",
            "routes.bp",
        )],
    );
}

#[test]
fn fk_67() {
    let mut files = app_files(
        "from pkg import bp\n",
        "bp",
        files(&[("pkg/__init__.py", "from .base import bp\n")]),
    );
    files.push(("pkg/base.py".into(), bp_module("routes", "/routes")));
    // A terminal re-export: no counted reason.
    check(&files, "app.py", mount_unproven(), &[]);
}

#[test]
fn fk_78() {
    let mut files = app_files(
        "import pkg\n",
        "pkg.routes.bp",
        files(&[("pkg/__init__.py", "from . import real as routes\n")]),
    );
    files.push(("pkg/real.py".into(), bp_module("routes", "/routes")));
    files.push((
        "pkg/routes.py".into(),
        bp_module("replacement", "/replacement"),
    ));
    check(
        &files,
        "app.py",
        mount_unproven(),
        &[(
            SubmoduleShadow,
            "app.py",
            "app.register_blueprint(pkg.routes.bp)",
            "pkg.routes.bp",
        )],
    );
}

#[test]
fn fk_79() {
    let files = app_files(
        "from pkg_alias import routes\n",
        "routes.bp",
        pkg_routes("pkg_alias", "type __getattr__ = int\n", "/routes"),
    );
    check(
        &files,
        "app.py",
        mount_unproven(),
        &[(
            GetattrModule,
            "app.py",
            "app.register_blueprint(routes.bp)",
            "routes.bp",
        )],
    );
}
