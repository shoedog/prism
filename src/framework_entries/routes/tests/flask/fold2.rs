use super::*;
use crate::framework_entries::routes::receivers::refusals::ImportRefusalReason;

const APP: &str = "from flask import Flask\napp = Flask(__name__)\n";
const FIRST: &str = "from flask import Flask, Blueprint\n\
def app_factory():\n    app = Flask(__name__)\n    return app\n\
def child_factory():\n    bp = Blueprint('first', __name__)\n    return bp\n\
from flask.views import View\n\
class Handler(View):\n    def dispatch_request(self): pass\n";
const SECOND: &str = "from flask import Flask, Blueprint\n\
def app_factory():\n    other = Flask(__name__)\n    return other\n\
def child_factory():\n    other = Blueprint('second', __name__)\n    return other\n\
from flask.views import MethodView\n\
class Handler(MethodView):\n    def post(self): pass\n";

fn capture_case(role: &str, member: &str, qualified: bool, competing: bool) {
    let imports = if qualified {
        "import first as selected\n".to_owned()
    } else {
        format!("from first import {member} as selected\n")
    };
    let replacement = if qualified {
        "import second as selected".to_owned()
    } else {
        format!("from second import {member} as selected")
    };
    let swap = if competing {
        format!("def swap():\n    global selected\n    {replacement}\nswap()\n")
    } else {
        String::new()
    };
    let selected = if qualified {
        format!("selected.{member}")
    } else {
        "selected".to_owned()
    };
    let suffix = match role {
        "receiver" => {
            format!("{selected}().add_url_rule('/x', endpoint='written', methods=['PUT'])")
        }
        "mount" => format!("app.register_blueprint({selected}(), url_prefix='/written')"),
        "class" => {
            format!("app.add_url_rule('/x', endpoint='written', view_func={selected}.as_view('x'))")
        }
        _ => unreachable!(),
    };
    let source = format!("{APP}{imports}{swap}{suffix}\n");
    let files = parsed(&[
        ("app.py", &source),
        ("first.py", FIRST),
        ("second.py", SECOND),
    ]);
    let bindings = index_python(&files);
    let facts = flask_facts(&bindings, "app.py", &SetupContext::default());
    match (role, facts.as_slice()) {
        ("mount", [FlaskEvent::Mount(mount)]) => {
            assert!(matches!(mount.parent, Receiver::Router { .. }));
            assert_eq!(
                mount.prefix,
                MountPrefix::Literal {
                    value: "/written".into()
                }
            );
            if competing {
                assert!(matches!(
                    mount.child,
                    Receiver::Unproven {
                        reason: Reason::MountChildUnproven
                    }
                ));
            } else {
                assert!(
                    matches!(&mount.child, Receiver::Router { identity, .. } if identity.file == "first.py")
                );
            }
        }
        (_, [FlaskEvent::Rule(rule)]) => {
            let route = &rule.route;
            assert_eq!(route.endpoint.as_deref(), Some("written"));
            let declaration = route.declaration.as_ref().unwrap();
            assert_eq!(declaration.path.literal.as_deref(), Some("/x"));
            assert_eq!(declaration.path.raw, "'/x'");
            if role == "receiver" {
                assert_eq!(declaration.methods, Some(MethodSet::known(["PUT"])));
                if competing {
                    assert!(matches!(
                        route.receiver,
                        Receiver::Unproven {
                            reason: Reason::ImportedUnproven
                        }
                    ));
                    assert_eq!(route.candidate, Some(true));
                } else {
                    assert!(
                        matches!(&route.receiver, Receiver::Router { identity, .. } if identity.file == "first.py")
                    );
                }
            } else {
                assert!(matches!(route.receiver, Receiver::Router { .. }));
                assert_eq!(declaration.methods, None);
                assert!(matches!(
                    route.callbacks[0].status,
                    CallbackStatus::Unresolved {
                        reason: Reason::ClassView,
                        ..
                    }
                ));
                assert_eq!(
                    route.default_methods,
                    (!competing).then(|| MethodSet::known(["GET"]))
                );
            }
        }
        _ => panic!("expected one retained {role} fact"),
    }
    let refusals = bindings.import_refusals();
    if competing {
        let span = if role == "class" && !qualified {
            member.to_owned()
        } else if role == "class" {
            selected
        } else {
            format!("{selected}()")
        };
        let start = if role == "class" && !qualified {
            APP.len() + imports.find(&span).unwrap()
        } else {
            source.len() - suffix.len() - 1 + suffix.find(&span).unwrap()
        };
        let mut spans = vec![(start, start + span.len())];
        if role != "class" && qualified {
            // Converter-owner indexing also inspects the written module root.
            let root = APP.len() + imports.find("first").unwrap();
            spans.insert(0, (root, root + "first".len()));
        }
        assert!(
            refusals.iter().all(|refusal| {
                refusal.file == "app.py" && refusal.reason == ImportRefusalReason::GlobalWrite
            }),
            "{refusals:?}"
        );
        assert_eq!(
            refusals
                .iter()
                .map(|refusal| (refusal.start_byte, refusal.end_byte))
                .collect::<Vec<_>>(),
            spans
        );
    } else {
        assert!(refusals.is_empty());
    }
}

#[test]
fn factory_receiver_bare() {
    capture_case("receiver", "app_factory", false, true);
}
#[test]
fn factory_receiver_qualified() {
    capture_case("receiver", "app_factory", true, true);
}
#[test]
fn factory_receiver_bare_control() {
    capture_case("receiver", "app_factory", false, false);
}
#[test]
fn factory_receiver_qualified_control() {
    capture_case("receiver", "app_factory", true, false);
}
#[test]
fn factory_mount_bare() {
    capture_case("mount", "child_factory", false, true);
}
#[test]
fn factory_mount_qualified() {
    capture_case("mount", "child_factory", true, true);
}
#[test]
fn factory_mount_bare_control() {
    capture_case("mount", "child_factory", false, false);
}
#[test]
fn factory_mount_qualified_control() {
    capture_case("mount", "child_factory", true, false);
}
#[test]
fn class_view_bare() {
    capture_case("class", "Handler", false, true);
}
#[test]
fn class_view_qualified() {
    capture_case("class", "Handler", true, true);
}
#[test]
fn class_view_bare_control() {
    capture_case("class", "Handler", false, false);
}
#[test]
fn class_view_qualified_control() {
    capture_case("class", "Handler", true, false);
}

const GLOBAL: &str = "from views import f as selected\n\
def swap():\n    global selected\n    from views import g as selected\n\
swap()\npatch(selected)\n";
const NONLOCAL: &str = "def setup():\n    from views import f as selected\n    def swap():\n        nonlocal selected\n        from views import g as selected\n    swap()\n    patch(selected)\nsetup()\n";
const GUARDED: &str = "from views import f as selected\n\
if first:\n    from views import g as selected\n\
if second:\n    from views import h as selected\n\
patch(selected)\n";
const QUALIFIED: &str = "import first as selected\n\
def swap():\n    global selected\n    import second as selected\n\
swap()\npatch(selected.f)\n";
const QUALIFIED_GUARDED: &str = "import first as selected\n\
if first_branch:\n    import second as selected\n\
if second_branch:\n    import third as selected\n\
patch(selected.f)\n";
const UNRELATED: &str = "def unrelated():\n    from views import g as selected\n\
from views import f as selected\npatch(selected)\n";

fn escape_case(setup: &str, view: &str, no_effect: bool, withhold: bool) {
    let source = format!("{APP}import views, first, second, third\nfrom helpers import patch\n{setup}\napp.add_url_rule('/x', view_func={view})\n");
    let files = parsed(&[
        ("app.py", &source),
        ("views.py", "def f(): pass\ndef g(): pass\ndef h(): pass\n"),
        ("first.py", "def f(): pass\n"),
        ("second.py", "def f(): pass\n"),
        ("third.py", "def f(): pass\n"),
        (
            "helpers.py",
            if no_effect {
                "def patch(v): pass\n"
            } else {
                "def patch(v): v.methods = ['POST']\n"
            },
        ),
    ]);
    let bindings = index_python(&files);
    let facts = flask_facts(&bindings, "app.py", &SetupContext::default());
    let [FlaskEvent::Rule(rule)] = facts.as_slice() else {
        panic!("expected a rule");
    };
    assert!(matches!(rule.route.receiver, Receiver::Router { .. }));
    assert!(matches!(
        rule.route.callbacks[0].status,
        CallbackStatus::Resolved { .. }
    ));
    assert_eq!(
        rule.route.default_methods,
        (!withhold).then(|| MethodSet::known(["GET"])),
        "{view}"
    );
}

#[test]
fn escape_global() {
    escape_case(GLOBAL, "views.g", false, true);
}
#[test]
fn escape_nonlocal() {
    escape_case(NONLOCAL, "views.g", false, true);
}
#[test]
fn escape_guarded_earlier() {
    escape_case(GUARDED, "views.g", false, true);
}
#[test]
fn escape_guarded_later() {
    escape_case(GUARDED, "views.h", false, true);
}
#[test]
fn escape_qualified_global() {
    escape_case(QUALIFIED, "second.f", false, true);
}
#[test]
fn escape_qualified_guarded() {
    escape_case(QUALIFIED_GUARDED, "second.f", false, true);
}
#[test]
fn escape_qualified_guarded_alias() {
    let setup = "import first as selected\nif enabled:\n    selected = second\npatch(selected.f)\n";
    escape_case(setup, "second.f", false, true);
}
#[test]
fn escape_qualified_duplicate_binding_in_one_import() {
    let setup = "import first as selected, second as selected\npatch(selected.f)\n";
    escape_case(setup, "second.f", false, true);
}
#[test]
fn escape_retains_known_candidates_beside_unresolved_import() {
    let setup = "from views import f as selected\nif first:\n    from views import g as selected\nif second:\n    from external import view as selected\npatch(selected)\n";
    escape_case(setup, "views.g", false, true);
}
#[test]
fn escape_unrelated_local_control() {
    escape_case(UNRELATED, "views.g", false, false);
}
#[test]
fn escape_no_effect_control() {
    escape_case(GLOBAL, "views.g", true, false);
}

fn rules(source: &str, check: impl FnOnce(&[FlaskEvent<'_>])) {
    let source = format!("{APP}{source}");
    let files = parsed(&[("app.py", &source)]);
    let bindings = index_python(&files);
    check(&flask_facts(&bindings, "app.py", &SetupContext::default()));
}

#[test]
fn keyword_methods_use_nfkc_and_keep_source_spans() {
    for call in [
        "app.add_url_rule('/é', endpoint='ｗritten', view_func=f, ｍethods=['POST'])",
        "app.add_url_rule(ｒule='/é', ｅndpoint='ｗritten', view_func=f, ｍethods=['POST'])",
    ] {
        let body = format!("def f(): pass\n{call}\n");
        rules(&body, |facts| {
            let [FlaskEvent::Rule(rule)] = facts else {
                panic!("expected a rule");
            };
            let route = &rule.route;
            let declaration = route.declaration.as_ref().unwrap();
            assert_eq!(declaration.path.raw, "'/é'");
            assert_eq!(route.endpoint.as_deref(), Some("ｗritten"));
            assert_eq!(declaration.methods, Some(MethodSet::known(["POST"])));
            assert_eq!(route.default_methods, None);
            let start = APP.len() + body.find(call).unwrap();
            assert_eq!(
                (route.site.start_byte, route.site.end_byte),
                (start, start + call.len())
            );
        });
    }
}

#[test]
fn keyword_callback_and_metadata_write_use_nfkc() {
    rules(
        "def ｆ(): pass\nｆ.ｍethods = ['POST']\napp.add_url_rule('/x', ｖiew_func=ｆ)\n",
        |facts| {
            let [FlaskEvent::Rule(rule)] = facts else {
                panic!("expected a rule");
            };
            assert!(
                matches!(&rule.route.callbacks[0].status, CallbackStatus::Resolved { argument_text, .. } if argument_text == "ｆ")
            );
            assert_eq!(rule.route.default_methods, None);
            assert_eq!(
                rule.effective_methods,
                MethodSet::known(["OPTIONS", "POST"])
            );
        },
    );
}

#[test]
fn metadata_attribute_write_uses_nfkc() {
    rules(
        "def f(): pass\nf.ｍethods = ['POST']\napp.add_url_rule('/x', view_func=f)\n",
        |facts| {
            let [FlaskEvent::Rule(rule)] = facts else {
                panic!("expected a rule");
            };
            assert_eq!(rule.route.default_methods, None);
            assert_eq!(
                rule.effective_methods,
                MethodSet::known(["OPTIONS", "POST"])
            );
        },
    );
}

#[test]
fn native_registration_members_use_nfkc() {
    rules(
        "def f(): pass\napp.ａｄｄ_url_rule('/x', view_func=f)\n@app.ｐost('/y')\ndef y(): pass\n",
        |facts| {
            let [FlaskEvent::Rule(first), FlaskEvent::Rule(second)] = facts else {
                panic!("expected both native rules");
            };
            assert_eq!(first.route.default_methods, Some(MethodSet::known(["GET"])));
            assert_eq!(
                second.route.declaration.as_ref().unwrap().methods,
                Some(MethodSet::known(["POST"]))
            );
        },
    );
}

#[test]
fn normalized_duplicate_keywords_are_refused() {
    rules("app.add_url_rule('/x', methods=['POST'], ｍethods=['GET'])\napp.add_url_rule('/y', ｒule='/other')\napp.add_url_rule('/z', ｓelf=app)\n", |facts| assert!(facts.is_empty()));
}

#[test]
fn class_view_member_uses_nfkc() {
    rules("from flask.views import View\nclass Handler(View): pass\napp.add_url_rule('/x', view_func=Handler.ａｓ_view(ｎame='x'))\n", |facts| {
        let [FlaskEvent::Rule(rule)] = facts else { panic!("expected a class view rule"); };
        assert!(matches!(rule.route.callbacks[0].status, CallbackStatus::Unresolved { reason: Reason::ClassView, .. }));
        assert_eq!(rule.route.default_methods, Some(MethodSet::known(["GET"])));
        assert_eq!(rule.effective_endpoint, EffectiveEndpoint::Known("x".into()));
    });
}

#[test]
fn class_view_verbs_use_nfkc() {
    rules("from flask.views import MethodView\nclass Handler(MethodView):\n    def ｐost(self): pass\napp.add_url_rule('/x', view_func=Handler.as_view('x'))\n", |facts| {
        let [FlaskEvent::Rule(rule)] = facts else { panic!("expected a class view rule"); };
        assert_eq!(rule.route.default_methods, None);
        assert_eq!(rule.effective_methods, MethodSet::known(["OPTIONS", "POST"]));
    });
}

#[test]
fn normalized_as_view_override_blocks_native_metadata() {
    rules("from flask.views import MethodView\nclass Handler(MethodView):\n    def ａｓ_view(self): pass\n    def post(self): pass\napp.add_url_rule('/x', view_func=Handler.as_view('x'))\n", |facts| {
        let [FlaskEvent::Rule(rule)] = facts else { panic!("expected a class view rule"); };
        assert_eq!(rule.effective_endpoint, EffectiveEndpoint::Unknown);
        assert!(matches!(rule.effective_methods, MethodSet::Unknown { .. }));
    });
}

#[test]
fn string_attribute_keys_keep_their_spelling_control() {
    rules(
        "def f(): pass\nsetattr(f, 'ｍethods', ['POST'])\napp.add_url_rule('/x', view_func=f)\n",
        |facts| {
            let [FlaskEvent::Rule(rule)] = facts else {
                panic!("expected a rule");
            };
            assert_eq!(rule.route.default_methods, Some(MethodSet::known(["GET"])));
        },
    );
}

#[test]
fn setup_rebound_function_is_not_activated() {
    rules("def install():\n    app.add_url_rule('/x')\ndef noop(): pass\ndef swap():\n    global install\n    install = noop\nswap()\ninstall()\n", |facts| assert!(facts.is_empty()));
}

#[test]
fn setup_rebound_class_is_not_activated() {
    rules("class Setup:\n    def install(self):\n        app.add_url_rule('/x')\nclass Noop:\n    def install(self): pass\ndef swap():\n    global Setup\n    Setup = Noop\nswap()\nSetup().install()\n", |facts| assert!(facts.is_empty()));
}

#[test]
fn setup_class_local_name_ignores_same_spelling_global_write_control() {
    rules("class Setup:\n    def install(self):\n        app.add_url_rule('/x')\ndef noop(): pass\ndef swap():\n    global install\n    install = noop\nswap()\nSetup().install()\n", |facts| {
        assert!(matches!(facts, [FlaskEvent::Rule(_)]));
    });
}

#[test]
fn opaque_mount_marks_competing_child() {
    let source = format!("{APP}from first import bp as child\ndef swap():\n    global child\n    from second import bp as child\nswap()\nopaque.register_blueprint(child)\n");
    let first = "from flask import Blueprint\nbp = Blueprint('first', __name__)\n";
    let second = "from flask import Blueprint\nbp = Blueprint('second', __name__)\n@bp.get('/x')\ndef f(): pass\n";
    let files = parsed(&[
        ("app.py", &source),
        ("first.py", first),
        ("second.py", second),
    ]);
    let bindings = index_python(&files);
    let facts = flask_facts(&bindings, "second.py", &SetupContext::default());
    let [FlaskEvent::Rule(rule)] = facts.as_slice() else {
        panic!("expected a retained child rule");
    };
    assert!(matches!(
        rule.route.receiver,
        Receiver::Unproven {
            reason: Reason::ReceiverUnproven
        }
    ));
    assert_eq!(rule.route.default_methods, None);
}

#[test]
fn opaque_context_mount_keeps_local_blueprint_rules() {
    let app = "from flask import Flask
class CustomApp(Flask): pass
def create_app():
    app = CustomApp(__name__)
    with app.app_context():
        from pkg.outer import outer
        app.register_blueprint(outer)
    return app
";
    let outer = "from flask import Blueprint
from pkg.inner import inner
import pkg.unmounted
outer = Blueprint('outer', __name__)
outer.register_blueprint(inner)
@outer.get('/outer')
def view(): pass
";
    let inner = "from flask import Blueprint
inner = Blueprint('inner', __name__)
@inner.get('/inner')
def view(): pass
";
    let unmounted = "from flask import Blueprint
other = Blueprint('other', __name__)
@other.get('/other')
def view(): pass
";
    let files = parsed(&[
        ("pkg/__init__.py", app),
        ("pkg/outer.py", outer),
        ("pkg/inner.py", inner),
        ("pkg/unmounted.py", unmounted),
    ]);
    let bindings = index_python(&files);
    let context = SetupContext::default();
    assert!(flask_facts(&bindings, "pkg/__init__.py", &context).is_empty());
    let local_proofs = ["pkg/outer.py", "pkg/inner.py", "pkg/unmounted.py"].map(|path| {
        let events = flask_facts(&bindings, path, &context);
        let [rule] = events
            .iter()
            .filter_map(|event| match event {
                FlaskEvent::Rule(rule) => Some(rule),
                _ => None,
            })
            .collect::<Vec<_>>()[..]
        else {
            panic!("expected one local rule in {path}");
        };
        matches!(&rule.route.receiver, Receiver::Router { identity, .. } if identity.file == path)
    });
    assert_eq!(local_proofs, [true, true, true], "outer, inner, unmounted");
}
