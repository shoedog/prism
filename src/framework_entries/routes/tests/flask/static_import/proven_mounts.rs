//! PROVEN mounts through imported Blueprint modules (controls and fk-30, fk-56, fk-64): the
//! mount child is proved as the allocation in the expected file and no refusal is counted.
//!
//! Skipped clauses per row (checkpoint 2): "full path" and "dispatch binding" on app, in fk-30,
//! fk-56, fk-64 and every control here (fk-c05, fk-c07, fk-c08, fk-c14, fk-c49..c51, fk-c54,
//! fk-c55, fk-c57..c59, fk-c67, fk-c68, fk-c71..c73, fk-c77, fk-c79), and native GET observations.
use super::*;

fn mounted(imports: &str, child: &str, base: Files) -> Files {
    let mut files = base;
    files.push(("app.py".into(), mount_app(imports, child)));
    files
}

#[test]
fn fk_c05() {
    let files = files(&[(
        "app.py",
        &app("bp = Blueprint('b', __name__)\n@bp.get('/x')\ndef f(): pass\napp.register_blueprint(bp)\n"),
    )]);
    check(
        &files,
        "app.py",
        rules(&[rule("/x").def("app.py", "f")]).with_mount(Some("app.py")),
        &[],
    );
}

fn bridge(setup_tail: &str) -> Files {
    files(&[
        (
            "setup.py",
            &format!("{}{setup_tail}", bp_module("child", "/first")),
        ),
        ("bridge.py", "import app\n"),
        ("app.py", &mount_app("from setup import bp\n", "bp")),
    ])
}

#[test]
fn fk_30() {
    check(&bridge("import bridge\n"), "app.py", mount("setup.py"), &[]);
}

#[test]
fn fk_c07() {
    check(&bridge(""), "app.py", mount("setup.py"), &[]);
}

#[test]
fn fk_c08() {
    let files = mounted(
        "import pkg.routes\n",
        "pkg.routes.bp",
        pkg_routes("pkg", "", "/x"),
    );
    check(&files, "app.py", mount("pkg/routes.py"), &[]);
}

#[test]
fn fk_c14() {
    let routes = "from flask import Blueprint\nbp = Blueprint('first', __name__)\n@bp.get('/first')\ndef first(): pass\n";
    let files = mounted(
        "import pkg_dupe.routes\n",
        "pkg_dupe.routes.bp",
        files(&[("pkg_dupe/__init__.py", ""), ("pkg_dupe/routes.py", routes)]),
    );
    check(&files, "app.py", mount("pkg_dupe/routes.py"), &[]);
}

#[test]
fn fk_c49() {
    let files = mounted(
        "import pkg_plain\n",
        "pkg_plain.routes.bp",
        pair("pkg_plain", "from . import routes\n"),
    );
    check(&files, "app.py", mount("pkg_plain/routes.py"), &[]);
}

#[test]
fn fk_c50() {
    let files = mounted(
        "import pkg_dotted.routes as selected\n",
        "selected.bp",
        pair("pkg_dotted", ""),
    );
    check(&files, "app.py", mount("pkg_dotted/routes.py"), &[]);
}

#[test]
fn fk_c51() {
    let files = mounted(
        "import pkg_cached.routes as selected\n",
        "selected.bp",
        pair("pkg_cached", "from . import routes\n"),
    );
    check(&files, "app.py", mount("pkg_cached/routes.py"), &[]);
}

#[test]
fn fk_64() {
    // Importing the module path selects its leaf through import machinery, with the
    // competing initializer writes of fk-51 present.
    let files = mounted(
        "from pkg_from.routes import bp\n",
        "bp",
        pair("pkg_from", COMPETING_INIT),
    );
    check(&files, "app.py", mount("pkg_from/routes.py"), &[]);
}

#[test]
fn fk_c54() {
    let files = mounted(
        "import pkg.routes\n",
        "pkg.routes.bp",
        pkg_routes("pkg", "", "/routes"),
    );
    check(&files, "app.py", mount("pkg/routes.py"), &[]);
}

#[test]
fn fk_c55() {
    let files = plus(
        mounted(
            "import pkg.routes\n",
            "pkg.routes.bp",
            pkg_routes("pkg", "", "/routes"),
        ),
        &[("tests/test_unrelated.py", "import pkg.routes\n")],
    );
    check(&files, "app.py", mount("pkg/routes.py"), &[]);
}

#[test]
fn fk_56() {
    // The initializer's explicit alias supplies the module-valued hop.
    let mut files = mounted(
        "import pkg\n",
        "pkg.routes.bp",
        files(&[("pkg/__init__.py", "from . import real as routes\n")]),
    );
    files.push(("pkg/real.py".into(), bp_module("routes", "/routes")));
    check(&files, "app.py", mount("pkg/real.py"), &[]);
}

#[test]
fn fk_c57() {
    let files = mounted(
        "import pkg_assign.routes as selected\n",
        "selected.bp",
        pair(
            "pkg_assign",
            "from . import replacement\nalternate = replacement\n",
        ),
    );
    check(&files, "app.py", mount("pkg_assign/routes.py"), &[]);
}

#[test]
fn fk_c58() {
    let init =
        "from .routes import bp as _first\nfrom . import replacement\nalternate = replacement\n";
    let files = mounted(
        "import pkg_assign_cached.routes as selected\n",
        "selected.bp",
        pair("pkg_assign_cached", init),
    );
    check(&files, "app.py", mount("pkg_assign_cached/routes.py"), &[]);
}

#[test]
fn fk_c59() {
    let mut files = mounted(
        "from pkg_g import routes\n",
        "routes.bp",
        pair("pkg_g", "from ._hook import __getattr__ as other\n"),
    );
    files.push(("pkg_g/_hook.py".into(), super::withheld_mounts::HOOK.into()));
    check(&files, "app.py", mount("pkg_g/routes.py"), &[]);
}

#[test]
fn fk_c67() {
    let mut files = mounted(
        "from pkg.base import bp\n",
        "bp",
        files(&[("pkg/__init__.py", "from .base import bp\n")]),
    );
    files.push(("pkg/base.py".into(), bp_module("routes", "/routes")));
    check(&files, "app.py", mount("pkg/base.py"), &[]);
}

#[test]
fn fk_c79() {
    let files = mounted(
        "from pkg_alias import routes\n",
        "routes.bp",
        pkg_routes("pkg_alias", "type other = int\n", "/routes"),
    );
    check(&files, "app.py", mount("pkg_alias/routes.py"), &[]);
}

#[test]
fn fk_c68() {
    let files = mounted(
        "from pkg.routes import bp\n",
        "bp",
        redirected("pkg", "import os\n"),
    );
    check(&files, "app.py", mount("pkg/routes.py"), &[]);
}

#[test]
fn fk_c71() {
    check(
        &relative_package(""),
        "pkg/app.py",
        mount("pkg/routes.py"),
        &[],
    );
}

/// fk-71 shape: `pkg.app` mounts `.routes.bp`, `other.routes` is the native redirect target.
pub(super) fn relative_package(package_write: &str) -> Files {
    files(&[
        ("pkg/__init__.py", ""),
        ("other/__init__.py", ""),
        ("pkg/routes.py", &bp_module("routes", "/routes")),
        ("other/routes.py", &bp_module("replacement", "/replacement")),
        (
            "pkg/app.py",
            &app(&format!(
                "{package_write}from .routes import bp\napp.register_blueprint(bp)\n"
            )),
        ),
    ])
}

pub(super) fn star_hooks(all: &str) -> Files {
    let hooks = format!(
        "other = None\n__all__ = ['{all}']\ndef __getattr__(name):\n    import pkg_s.replacement\n    if name == 'routes':\n        return pkg_s.replacement\n    raise AttributeError(name)\n"
    );
    let mut files = mounted(
        "from pkg_s import routes\n",
        "routes.bp",
        pair("pkg_s", "from ._hooks import *\n"),
    );
    files.push(("pkg_s/_hooks.py".into(), hooks));
    files
}

#[test]
fn fk_c72() {
    check(
        &star_hooks("other"),
        "app.py",
        mount("pkg_s/routes.py"),
        &[],
    );
}

pub(super) fn star_paths(all: &str) -> Files {
    let paths = format!("other = None\n{PATH_WRITE}__all__ = ['{all}']\n");
    let mut files = mounted(
        "from pkg_star.routes import bp\n",
        "bp",
        redirected("pkg_star", "from ._paths import *\n"),
    );
    files.push(("pkg_star/_paths.py".into(), paths));
    files
}

#[test]
fn fk_c73() {
    check(
        &star_paths("other"),
        "app.py",
        mount("pkg_star/routes.py"),
        &[],
    );
}

pub(super) const MACHINERY: &str =
    "__name__ = 'renamed'\n__spec__ = None\n__package__ = None\n__loader__ = None\n";

#[test]
fn fk_c77() {
    let files = mounted(
        "import pkg_hop.routes as selected\n",
        "selected.bp",
        redirected("pkg_hop", ""),
    );
    check(&files, "app.py", mount("pkg_hop/routes.py"), &[]);
}
