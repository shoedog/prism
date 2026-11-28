//! Capture and escape admission, round-3 members of PR #928 (lane 12).
//! Each input is refused by ONE rule (a competing binding of a captured name
//! counts wherever it sits or runs), beside a control that stays proved.
use super::*;

const APP: &str = "from flask import Flask\napp = Flask(__name__)\n";
const VIEWS: &str = "def f(): pass\ndef g(): pass\n";

fn default_for(setup: &str, view: &str, patch: bool) -> Option<MethodSet> {
    let source = format!(
        "{APP}import views\nfrom helpers import patch\n{setup}\napp.add_url_rule('/y', view_func={view})\n"
    );
    let files = parsed(&[
        ("app.py", &source),
        ("views.py", VIEWS),
        (
            "helpers.py",
            if patch {
                "def patch(v): v.methods = ['POST']\n"
            } else {
                "def patch(v): pass\n"
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
    rule.route.default_methods.clone()
}

// Member 1: an unrelated first import must not end the search for redirects.
const FIRST_UNRELATED: &str = "from math import sin as h\n\
def swap():\n    global h\n    from views import g as h\nswap()\npatch(h)\n";
const SOLE_REDIRECT: &str =
    "def swap():\n    global h\n    from views import g as h\nswap()\npatch(h)\n";

#[test]
fn escape_unrelated_first_import_then_global_redirect() {
    assert_eq!(default_for(FIRST_UNRELATED, "views.g", true), None);
}
#[test]
fn escape_sole_redirected_import() {
    assert_eq!(default_for(SOLE_REDIRECT, "views.g", true), None);
}
#[test]
fn escape_unrelated_first_import_no_effect_control() {
    assert_eq!(
        default_for(FIRST_UNRELATED, "views.g", false),
        Some(MethodSet::known(["GET"]))
    );
}

// Member 4: a competing write reaches the read by a back edge or a second call.
const SECOND_CALL: &str = "from views import f as h\n\
def run():\n    global h\n    patch(h)\n    from views import g as h\nrun()\nrun()\n";
const LOOP: &str = "from views import f as h\n\
for _ in range(2):\n    patch(h)\n    from views import g as h\n";

#[test]
fn escape_write_after_read_in_a_second_call() {
    assert_eq!(default_for(SECOND_CALL, "views.g", true), None);
}
#[test]
fn escape_write_after_read_in_a_loop() {
    assert_eq!(default_for(LOOP, "views.g", true), None);
}
#[test]
fn escape_write_after_read_no_effect_control() {
    assert_eq!(
        default_for(SECOND_CALL, "views.g", false),
        Some(MethodSet::known(["GET"]))
    );
    assert_eq!(
        default_for(LOOP, "views.g", false),
        Some(MethodSet::known(["GET"]))
    );
}

fn receiver_of(source: &str, extra: &[(&str, &str)]) -> Receiver {
    let mut sources = vec![("app.py", source)];
    sources.extend_from_slice(extra);
    let files = parsed(&sources);
    let bindings = index_python(&files);
    let facts = flask_facts(&bindings, "app.py", &SetupContext::default());
    // A withheld imported router leaves no event at all (no proved rule is the verdict).
    match facts.as_slice() {
        [] => Receiver::Unproven {
            reason: Reason::ReceiverUnproven,
        },
        [FlaskEvent::Rule(rule)] => rule.route.receiver.clone(),
        other => panic!("unexpected events: {}", other.len()),
    }
}

// Member 2: a refused direct factory capture stays refused.
#[test]
fn refused_factory_capture_does_not_fall_back_to_the_local_factory() {
    let source = format!(
        "{APP}from legacy import make\n\
def make():\n    other = Flask(__name__)\n    return other\n\
make().add_url_rule('/x', endpoint='x')\n"
    );
    let receiver = receiver_of(&source, &[("legacy.py", "def make(config): pass\n")]);
    assert!(
        matches!(receiver, Receiver::Unproven { .. }),
        "{receiver:?}"
    );
}
#[test]
fn single_local_factory_control() {
    let source = format!(
        "{APP}def make():\n    other = Flask(__name__)\n    return other\n\
make().add_url_rule('/x', endpoint='x')\n"
    );
    assert!(matches!(receiver_of(&source, &[]), Receiver::Router { .. }));
}

// Member 3: a direct write to a view class's methods after its definition.
fn class_view_default(extra: &str) -> Option<MethodSet> {
    let source = format!(
        "{APP}from flask.views import View\n\
class Handler(View):\n    def dispatch_request(self): pass\n\
class Other(View):\n    def dispatch_request(self): pass\n\
{extra}app.add_url_rule('/x', view_func=Handler.as_view('x'))\n"
    );
    let files = parsed(&[("app.py", &source)]);
    let bindings = index_python(&files);
    let facts = flask_facts(&bindings, "app.py", &SetupContext::default());
    let [FlaskEvent::Rule(rule)] = facts.as_slice() else {
        panic!("expected a rule");
    };
    rule.route.default_methods.clone()
}
// KNOWN_LIMIT: a store on a view class is a change through another holder of the class,
// outside the static-binding contract (DESIGN 5.3/10): the default keeps the class's methods.
#[test]
fn class_view_direct_methods_write_withholds_the_default() {
    assert_eq!(
        class_view_default("Handler.methods = ['POST']\n"),
        Some(MethodSet::known(["GET"]))
    );
}
#[test]
fn class_view_unwritten_control() {
    assert_eq!(class_view_default(""), Some(MethodSet::known(["GET"])));
}
#[test]
fn class_view_write_to_another_class_control() {
    assert_eq!(
        class_view_default("Other.methods = ['POST']\n"),
        Some(MethodSet::known(["GET"]))
    );
}
