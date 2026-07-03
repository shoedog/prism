//! P9: Flask/FastAPI/Express route-registration nav surfacing (S3),
//! exercised end-to-end through `nav_callers`/`nav_callees`. Mirrors
//! `property_access_test.rs`'s style for the P7 Python property-access
//! feature.

use prism::navigation::types::{Reason, SymbolRef};
use prism::navigation::{queries, NavigationIndex, NavigationSession};
use prism::repo_loader::load_repo;
use std::sync::Arc;

fn session(files: &[(&str, &str)]) -> NavigationSession {
    let dir = tempfile::tempdir().unwrap();
    for (name, src) in files {
        let path = dir.path().join(name);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(path, src).unwrap();
    }
    let repo = Arc::new(load_repo(dir.path()).unwrap());
    let index = Arc::new(NavigationIndex::build(&repo));
    NavigationSession { repo, index }
}

#[test]
fn module_level_flask_route_surfaces_as_nameonly_caller_with_registration_line() {
    let s = session(&[(
        "app.py",
        "from flask import Flask\napp = Flask(__name__)\n\n@app.route(\"/x\")\ndef handler():\n    return \"ok\"\n",
    )]);
    let ev = queries::callers(&s, Some("handler"), None, None, 1).unwrap();
    let hit = ev
        .items
        .iter()
        .find(|i| matches!(&i.symbol, Some(SymbolRef::Function { name, .. }) if name == "<module>"))
        .expect("<module> surfaces as a caller of handler via the route registration");
    assert!(
        (hit.score - 0.6).abs() < 1e-6,
        "framework_entry is NameOnly -> score 0.6, got {}",
        hit.score
    );
    assert!(hit
        .why
        .iter()
        .any(|r| matches!(r, Reason::Resolution { kind } if kind == "framework_entry")));
    assert!(
        hit.why
            .iter()
            .any(|r| matches!(r, Reason::CalledBy { call_site_line, .. } if *call_site_line == 4)),
        "registration site must be the decorator line (line 4), got {:?}",
        hit.why
    );
}

#[test]
fn express_registration_surfaces_at_the_registration_call_line() {
    let s = session(&[(
        "app.js",
        "const express = require(\"express\");\nconst app = express();\n\nfunction handler(req, res) {}\n\napp.get(\"/x\", handler);\n",
    )]);
    let ev = queries::callers(&s, Some("handler"), None, None, 1).unwrap();
    let hit = ev
        .items
        .iter()
        .find(|i| matches!(&i.symbol, Some(SymbolRef::Function { name, .. }) if name == "<module>"))
        .expect("<module> surfaces as a caller of handler via the route registration");
    assert!(hit
        .why
        .iter()
        .any(|r| matches!(r, Reason::Resolution { kind } if kind == "framework_entry")));
    assert!(hit
        .why
        .iter()
        .any(|r| matches!(r, Reason::CalledBy { call_site_line, .. } if *call_site_line == 6)));
}

#[test]
fn registration_inside_setup_function_surfaces_symmetrically_in_callees() {
    let s = session(&[(
        "app.js",
        "const express = require(\"express\");\nconst app = express();\n\nfunction handler(req, res) {}\n\nfunction setup() {\n    app.get(\"/x\", handler);\n}\n",
    )]);
    let callers_ev = queries::callers(&s, Some("handler"), None, None, 1).unwrap();
    assert!(
        callers_ev.items.iter().any(
            |i| matches!(&i.symbol, Some(SymbolRef::Function { name, .. }) if name == "setup")
        ),
        "callers(handler) should include setup() via the framework_entry edge"
    );

    let callees_ev = queries::callees(&s, Some("setup"), None, None, 1).unwrap();
    assert!(
        callees_ev.items.iter().any(
            |i| matches!(&i.symbol, Some(SymbolRef::Function { name, .. }) if name == "handler")
        ),
        "callees(setup) should include handler via the framework_entry edge (real enclosing function)"
    );
}
