//! KNOWN_LIMIT rows: route-bearing patterns of real applications that are OUTSIDE the contract
//! (a proof is a static-binding proof; another holder of the object, or a run-time registration,
//! can change what serves a route). Each row pins what the product does TODAY (verdict per rule
//! event, in order); the holder barriers (a router handed to a call, a store on a router)
//! STAY in the replacement, so these are expected to keep passing. Census counts: R06 1 site,
//! H03b 17 (13 applications-only hits), H03a/H12 wsgi and URL rewriting 2 sites, S23 1 site.
use super::*;

#[track_caller]
fn today(files: &[(&str, &str)], entry: &str, want: &[V]) {
    assert_eq!(verdicts_of(&parsed(files), entry, true), want);
}

// R06, CTFd CTFd/plugins/__init__.py:214 (`importlib.import_module(..)` then
// `module.load(app)` registers plugin routes at run time; 1 site in the census). Today the
// router handed to `init_plugins(app)` is withheld by the holder barrier.
#[test]
fn r06_run_time_plugin_load_is_outside_the_claim() {
    let app = format!(
        "import importlib\n{ALLOC}{ROUTE}def init_plugins(app):\n    module = importlib.import_module('.x', package='plugins')\n    module.load(app)\ninit_plugins(app)\n"
    );
    today(&[("app.py", &app)], "app.py", &[V::Withheld]);
}

// H03b, CTFd CTFd/api/__init__.py:35 (`Api(blueprint)` adds swagger endpoints), trape
// core/sockets.py:33 (`SocketIO(app)` wraps `app.wsgi_app`), ihatemoney api/v1/resources.py.
// Today: withheld by the same barrier (the router is passed to a call).
#[test]
fn h03b_extension_handed_the_app_is_outside_the_claim() {
    let app = format!(
        "from flask_socketio import SocketIO\n{ALLOC}socketio = SocketIO(app, async_mode=None)\n{ROUTE}"
    );
    today(&[("app.py", &app)], "app.py", &[V::Withheld]);
}
#[test]
fn h03b_extension_handed_the_blueprint_is_outside_the_claim() {
    let api = "from flask import Blueprint\nfrom flask_restx import Api\napi = Blueprint('api', __name__, url_prefix='/api/v1')\nrest = Api(api, version='v1', doc=False)\n@api.route('/x')\ndef f(): pass\n";
    today(&[("api.py", api)], "api.py", &[V::Withheld]);
}

// H03a, sqlite-web sqlite_web/sqlite_web.py:1928 (`app.wsgi_app = PrefixMiddleware(..)`
// changes every URL). Today PROVED: an attribute store on the router is not a barrier.
#[test]
fn h03a_wsgi_app_wrapping_is_outside_the_claim() {
    let app = format!("{ALLOC}app.wsgi_app = PrefixMiddleware(app.wsgi_app, prefix='/p')\n{ROUTE}");
    today(&[("app.py", &app)], "app.py", &[V::Proved]);
}
// H12, redash `add_org_resource` (a custom API class rewriting the URLs). Today withheld,
// because the blueprint is handed to the `ApiExt(bp)` call.
#[test]
fn h12_url_rewriting_api_class_is_outside_the_claim() {
    let api = "from flask import Blueprint\nfrom flask_restful import Api\nclass ApiExt(Api):\n    def add_org_resource(self, resource, *urls, **kwargs):\n        urls = [f'/<org_slug>{url}' for url in urls]\n        return self.add_resource(resource, *urls, **kwargs)\nbp = Blueprint('api', __name__)\napi = ApiExt(bp)\n@bp.route('/x')\ndef f(): pass\n";
    today(&[("api.py", api)], "api.py", &[V::Withheld]);
}

// S23, redash redash/authentication/__init__.py:258-267: blueprints mounted in a loop behind
// a settings-driven factory (only those whose factory returns non-None are mounted). Today:
// the blueprint's route is proved, the app's route is withheld (`make_google(app)`).
#[test]
fn s23_settings_driven_loop_mount_behind_a_factory_is_outside_the_claim() {
    let app = "from flask import Flask, Blueprint\napp = Flask(__name__)\nbase = Blueprint('base', __name__)\n@base.route('/x')\ndef f(): pass\n@app.route('/y')\ndef g(): pass\ndef init_app(settings):\n    for blueprint in [make_google(app) if settings.GOOGLE else None, base]:\n        if blueprint:\n            app.register_blueprint(blueprint)\n";
    today(&[("app.py", app)], "app.py", &[V::Proved, V::Withheld]);
}
