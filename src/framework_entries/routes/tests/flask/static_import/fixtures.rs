//! Shared fixture builders. Each row derives from a control; the builders take the part the
//! row changes.
use super::{files, Files};

/// A fresh Flask app with `static_folder=None`.
pub(super) const FLASK: &str =
    "from flask import Flask, Blueprint\napp = Flask(__name__, static_folder=None)\n";

/// `patch(view)` with no effect.
pub(super) const PATCH_NOOP: &str = "def patch(view): pass\n";
/// `patch(view)` renames and re-methods the view.
pub(super) const PATCH_MUT: &str =
    "def patch(view):\n    view.__name__ = 'renamed'\n    view.methods = ['POST']\n";
/// A mutating `_mut(view)` beside a direct no-effect `patch`.
pub(super) const MUT: &str =
    "def _mut(view):\n    view.__name__ = 'renamed'\n    view.methods = ['POST']\n";

pub(super) fn app(body: &str) -> String {
    format!("{FLASK}{body}")
}

pub(super) fn plus(mut base: Files, extra: &[(&str, &str)]) -> Files {
    base.extend(files(extra));
    base
}

/// fk-c02: `views.py` defines `f`; `app.py` imports it directly.
pub(super) fn c02() -> Files {
    files(&[
        (
            "app.py",
            &app("from views import f\napp.add_url_rule('/x', view_func=f)\n"),
        ),
        ("views.py", "def f(): pass\n"),
    ])
}

/// fk-c03: `from setup import patch` and `patch(f)` before the registration. `before` is
/// written first in `app.py`, `setup` is `setup.py`.
pub(super) fn c03(before: &str, setup: &str) -> Files {
    files(&[
        (
            "app.py",
            &app(&format!(
                "{before}from views import f\nfrom setup import patch\npatch(f)\napp.add_url_rule('/x', view_func=f)\n"
            )),
        ),
        ("views.py", "def f(): pass\n"),
        ("setup.py", setup),
    ])
}

/// fk-c06: `import setup` and `setup.patch(f)`.
pub(super) fn c06(setup: &str) -> Files {
    files(&[
        (
            "app.py",
            &app("import setup\nfrom views import f\nsetup.patch(f)\napp.add_url_rule('/x', view_func=f)\n"),
        ),
        ("views.py", "def f(): pass\n"),
        ("setup.py", setup),
    ])
}

/// fk-c11: `import setup as rules` and `rules.patch(f)`.
pub(super) fn c11(setup: &str) -> Files {
    files(&[
        (
            "app.py",
            &app("import setup as rules\nfrom views import f\nrules.patch(f)\napp.add_url_rule('/x', view_func=f)\n"),
        ),
        ("views.py", "def f(): pass\n"),
        ("setup.py", setup),
    ])
}

/// fk-c09: two plain views and a registration at endpoint `shared`, plus `after`.
pub(super) fn c09(after: &str) -> Files {
    files(&[(
        "app.py",
        &app(&format!(
            "def a(): pass\ndef b(): pass\napp.add_url_rule('/x', 'shared', a)\n{after}"
        )),
    )])
}

/// fk-c01: `import pkg.routes` and a decorator route, with `lines` after the app.
pub(super) fn c01(lines: &str) -> Files {
    files(&[
        (
            "app.py",
            &app(&format!(
                "import pkg.routes\n{lines}@app.get('/x')\ndef f(): pass\n"
            )),
        ),
        ("pkg/__init__.py", ""),
        ("pkg/routes.py", "x = 1\n"),
    ])
}

/// A Blueprint module serving `route`.
pub(super) fn bp_module(name: &str, route: &str) -> String {
    format!(
        "from flask import Blueprint\nbp = Blueprint('{name}', __name__)\n@bp.get('{route}')\ndef {name}_view(): return '{name}'\n"
    )
}

/// `<pkg>/routes.py` (`/routes`) and `<pkg>/replacement.py` (`/replacement`) with this initializer.
pub(super) fn pair(pkg: &str, init: &str) -> Files {
    files(&[
        (&format!("{pkg}/__init__.py"), init),
        (&format!("{pkg}/routes.py"), &bp_module("routes", "/routes")),
        (
            &format!("{pkg}/replacement.py"),
            &bp_module("replacement", "/replacement"),
        ),
    ])
}

/// `app.py` that registers `child` after `imports`.
pub(super) fn mount_app(imports: &str, child: &str) -> String {
    app(&format!("{imports}app.register_blueprint({child})\n"))
}

/// The fk-49 initializer: two competing lexical writes to `routes`.
pub(super) const COMPETING_INIT: &str =
    "from . import routes\nif True:\n    from . import replacement as routes\n";

/// A regular package with an empty initializer and `routes.py` serving `route`.
pub(super) fn pkg_routes(pkg: &str, init: &str, route: &str) -> Files {
    files(&[
        (&format!("{pkg}/__init__.py"), init),
        (&format!("{pkg}/routes.py"), &bp_module("routes", route)),
    ])
}

/// fk-68 shape: initializer redirecting `__path__`, with `routes` and `alt/routes`.
pub(super) fn redirected(pkg: &str, init: &str) -> Files {
    files(&[
        (&format!("{pkg}/__init__.py"), init),
        (&format!("{pkg}/routes.py"), &bp_module("routes", "/routes")),
        (
            &format!("{pkg}/alt/routes.py"),
            &bp_module("replacement", "/replacement"),
        ),
    ])
}

pub(super) const PATH_WRITE: &str =
    "import os\n__path__ = [os.path.join(os.path.dirname(__file__), 'alt')]\n";
