//! Harmless used patterns of real applications (census 2026-11-27, `census-used/run5a`), each
//! reduced to a synthetic file. The hit named in the comment is where the shape was seen.
use super::*;

// S01 + S07, trape core/stats.py:29-31 (`app = Flask(..)` on both branches of an if). Two
// bindings of the router, so WITHHELD; trape's own routes are not proved today either.
#[test]
fn s01_s07_app_built_on_both_branches_of_an_if() {
    let app = "import sys\nfrom flask import Flask\nif getattr(sys, 'frozen', False):\n    app = Flask(__name__, template_folder='a')\nelse:\n    app = Flask(__name__, template_folder='b')\n@app.route('/x')\ndef f(): pass\n";
    expect(&[("app.py", app)], "app.py", V::Withheld);
}

// S04, kamiFaka service/api/admin.py:620 and user.py:43 (`for index, row in ..` in a function
// beside a module view `index`), simple-login app/auth/views/fido.py:167.
#[test]
fn s04_loop_target_in_a_function_spelled_like_a_view() {
    let app = format!(
        "{ALLOC}from views import f as index\napp.add_url_rule('/', view_func=index)\ndef report(rows):\n    for index, row in enumerate(rows):\n        pass\n"
    );
    expect(
        &[("app.py", &app), ("views.py", VIEWS)],
        "app.py",
        V::Proved,
    );
}

// S14g, redash redash/authentication/__init__.py:258 and simple-login fido.py:145
// (`for blueprint in [..]` in a function, the module also holds a router of that name).
#[test]
fn s14g_for_target_in_a_function_spelled_like_a_router() {
    let app = "from flask import Flask, Blueprint\napp = Flask(__name__)\nbp = Blueprint('b', __name__)\n@bp.route('/x')\ndef f(): pass\ndef mount(bps):\n    for bp in bps:\n        app.register_blueprint(bp)\n";
    expect(&[("app.py", app)], "app.py", V::Proved);
}

// S14e, kamiFaka app.py:17 and dev.py:16, sqlite-web sqlite_web.py:838 (comprehension target
// spelled like a module name).
#[test]
fn s14e_comprehension_target_spelled_like_a_router() {
    let app = "from flask import Flask, Blueprint\nbp = Blueprint('b', __name__)\n@bp.route('/x')\ndef f(): pass\n_names = [bp.name for bp in []]\n";
    expect(&[("app.py", app)], "app.py", V::Proved);
}

// S05, CTFd CTFd/git.py:24 and :143, microblog app/main/routes.py:74 and :92 (a function
// local spelled like a module-scope name; 41 hits in 9 applications).
#[test]
fn s05_function_local_spelled_like_an_imported_view() {
    let app = format!("{ALLOC}{VIEW_REG}def provision(user):\n    f = user.token\n    return f\n");
    expect(
        &[("app.py", &app), ("views.py", VIEWS)],
        "app.py",
        V::Proved,
    );
}
#[test]
fn s05_function_parameter_spelled_like_a_router() {
    let app = format!("{ALLOC}{ROUTE}def configure(app):\n    app.config['X'] = 1\n");
    expect(&[("app.py", &app)], "app.py", V::Proved);
}

// S06, sqlite-web sqlite_web/sqlite_web.py:73 (`from peewee import *`, external) and trape
// core/user.py:17 (`from core.user_objects import *`, an in-repo module that cannot bind
// the relied names).
#[test]
fn s06_external_star_import() {
    let app = format!("from peewee import *\n{ALLOC}{ROUTE}");
    expect(&[("app.py", &app)], "app.py", V::Proved);
}
#[test]
fn s06_in_repo_star_import_that_cannot_bind_the_relied_name() {
    let app = format!("from user_objects import *\n{ALLOC}{ROUTE}");
    expect(
        &[
            ("app.py", &app),
            ("user_objects.py", "class Victim: pass\nvictims = []\n"),
        ],
        "app.py",
        V::Proved,
    );
}

// S08, CTFd CTFd/auth.py:37 and :119 (`@ratelimit_from_config(..)` under `@auth.route`):
// a non-router wrapper decorator on a view, 436 sites in 11 applications.
#[test]
fn s08_non_router_wrapper_decorator_on_a_view() {
    let app = "from flask import Blueprint\nfrom decorators import ratelimit_from_config\nauth = Blueprint('auth', __name__)\n@auth.route('/confirm', methods=['POST', 'GET'])\n@ratelimit_from_config(method='POST', config_key='RATELIMIT_CONFIRM')\ndef confirm(data=None): pass\n";
    let decorators = "import functools\ndef ratelimit_from_config(method='POST', config_key='K'):\n    def decorator(f):\n        @functools.wraps(f)\n        def wrapper(*args, **kwargs):\n            return f(*args, **kwargs)\n        return wrapper\n    return decorator\n";
    let files = parsed(&[("auth.py", app), ("decorators.py", decorators)]);
    let bindings = index_python(&files);
    let facts = flask_facts(&bindings, "auth.py", &SetupContext::default());
    let [FlaskEvent::Rule(rule)] = facts.as_slice() else {
        panic!("expected one rule");
    };
    // The ROUTER proof (the census counts a row without `candidate` as proved) stands. The
    // callback identity is a separate question (`InnerDecorator`, decorator semantics),
    // untouched by the certificate.
    assert!(matches!(rule.route.receiver, Receiver::Router { .. }));
    assert_eq!(rule.route.candidate, None);
    assert!(matches!(
        rule.route.callbacks[0].status,
        CallbackStatus::Unresolved {
            reason: Reason::InnerDecorator,
            ..
        }
    ));
}

// S09, CTFd CTFd/plugins/challenges/__init__.py:345 (`blueprint = Blueprint(..)` in a class
// body, the module has its own `blueprint`), trape core/user.py:42 (a def in a class body).
#[test]
fn s09_class_body_name_spelled_like_a_router() {
    let app = "from flask import Blueprint\nblueprint = Blueprint('main', __name__)\n@blueprint.route('/x')\ndef f(): pass\nclass Plugin:\n    blueprint = Blueprint('standard', __name__, static_folder='assets')\n    def register(self): pass\n";
    expect(&[("app.py", app)], "app.py", V::Proved);
}

// S11, formspree formspree/__init__.py:2 (`from .create_app import create_app`); the
// `package_from_child_import_of_the_same_name_control` row of `module_system` pins the
// resolution, this one pins the router-side shape of the same package.
#[test]
fn s11_package_imports_its_own_child_of_the_same_name() {
    let init = "from . import settings\nfrom .create_app import create_app\napp = create_app()\n";
    let child = "from flask import Flask\napp = Flask(__name__)\n@app.route('/x')\ndef f(): pass\ndef create_app(): return app\n";
    expect(
        &[
            ("pkg/__init__.py", init),
            ("pkg/create_app.py", child),
            ("pkg/settings.py", "DEBUG = True\n"),
        ],
        "pkg/create_app.py",
        V::Proved,
    );
}

// S22, kamiFaka app.py:17 and dev.py:16 (`[app.register_blueprint(bp) for bp in bps]`), H04
// kamiFaka app.py:16 (`bps = [base, admin, common]`), S23 redash redash/authentication/
// __init__.py:267 (a for loop mount), H19 RedInk backend/routes/__init__.py:36-37 (a mount
// whose argument is a call). The blueprint's own routes stay proved.
fn mounts(mount: &str) {
    let app = format!(
        "from flask import Flask, Blueprint\napp = Flask(__name__)\nbase = Blueprint('base', __name__)\n@base.route('/x')\ndef f(): pass\n@app.route('/y')\ndef g(): pass\n{mount}"
    );
    expect(&[("app.py", &app)], "app.py", V::Proved);
}
#[test]
fn s22_comprehension_mount() {
    mounts("bps = [base]\n[app.register_blueprint(bp) for bp in bps]\n");
}
#[test]
fn s23_loop_mount() {
    mounts(
        "for blueprint in [base]:\n    if blueprint:\n        app.register_blueprint(blueprint)\n",
    );
}
#[test]
fn h04_router_in_a_container_display() {
    mounts("saved = [app, base]\n");
}
#[test]
fn h19_mount_of_a_call_result() {
    mounts("def make(): return base\napp.register_blueprint(make())\n");
}

// H10, simple-login server.py:11, maintenance_app.py:7 (annotations naming `Flask`).
#[test]
fn h10_annotation_naming_the_constructor() {
    let app = format!(
        "{ALLOC}{ROUTE}def create_light_app() -> Flask:\n    return app\ndef observe(x: Flask, y: 'Flask'): pass\n"
    );
    expect(&[("app.py", &app)], "app.py", V::Proved);
}

// H03a, FractalCore server.py:74, simple-login oauth_tester.py:18 (`app.secret_key = ..`).
#[test]
fn h03a_secret_key_and_config_stores() {
    let app = format!(
        "import os\n{ALLOC}app.secret_key = os.getenv('SECRET_KEY')\napp.config['SESSION_COOKIE_PATH'] = '/'\n{ROUTE}"
    );
    expect(&[("app.py", &app)], "app.py", V::Proved);
}

// R01, simple-login app/dashboard/views/alias_log.py:62 (`render_template(.., **locals())`).
#[test]
fn r01_locals_into_a_template() {
    let app = format!(
        "{ALLOC}from flask import render_template\n@app.route('/x')\ndef f():\n    name = 1\n    return render_template('x.html', **locals())\n"
    );
    expect(&[("app.py", &app)], "app.py", V::Proved);
}

// R02, DockFlare / sqlite-web (`re.compile`, one confirmed eval): not a binding form.
#[test]
fn r02_compile_and_eval_calls() {
    let app = format!(
        "import re\n{ALLOC}PATTERN = re.compile('x')\ndef calc(expr):\n    return eval(expr)\n{ROUTE}"
    );
    expect(&[("app.py", &app)], "app.py", V::Proved);
}
