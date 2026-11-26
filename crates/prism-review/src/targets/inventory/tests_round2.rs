use super::{file, Family, Grade, UnprovenCause, UnsupportedReason};
use tempfile::tempdir;

#[path = "tests_coverage.rs"]
mod coverage;

fn assert_http_cases(cases: &[(&str, &str, Option<UnprovenCause>)], callee: &str) {
    let repo = tempdir().unwrap();
    let mut failures = Vec::new();
    for &(label, source, cause) in cases {
        let out = file(source, repo.path(), &[Family::Http]);
        let exact = matches!(out.sites.as_slice(), [site]
            if site.grade == Grade::Exact && site.resolved_callee == callee);
        let expected_causes: Vec<_> = cause.into_iter().map(|cause| (cause, 1)).collect();
        let causes: Vec<_> = out
            .unproven
            .values()
            .flatten()
            .map(|(&k, &v)| (k, v))
            .collect();
        if exact != cause.is_none()
            || out.sites.len() != usize::from(cause.is_none())
            || !out.unsupported.is_empty()
            || causes != expected_causes
        {
            failures.push(format!(
                "{label}: expected {cause:?}, got {} sites, {} unsupported, {causes:?}",
                out.sites.len(),
                out.unsupported.len()
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn class_if_test_bindings_reach_both_successors() {
    use UnprovenCause::ShadowedRoot;
    assert_http_cases(&[
        ("if before exit", "import requests\ndef fake(u): pass\nclass C:\n    if (requests := fake) is None:\n        pass\n    x = requests.get('u')\n", Some(ShadowedRoot)),
        ("if true arm", "import requests\nclass C:\n    if (requests := fake):\n        x = requests.get('u')\n", Some(ShadowedRoot)),
        ("if false arm", "import requests\nclass C:\n    if (requests := fake):\n        pass\n    else:\n        x = requests.get('u')\n", Some(ShadowedRoot)),
        ("elif true arm", "import requests\nimport os\nclass C:\n    if flag:\n        pass\n    elif (requests := os):\n        x = requests.get('u')\n", Some(ShadowedRoot)),
        ("elif false arm", "import requests\nclass C:\n    if flag:\n        pass\n    elif (requests := fake):\n        pass\n    else:\n        x = requests.get('u')\n", Some(ShadowedRoot)),
        ("elif false path after exit", "import requests\nclass C:\n    if flag:\n        pass\n    elif (requests := fake):\n        pass\n    x = requests.get('u')\n", Some(ShadowedRoot)),
        ("nested inner if", "import requests\nclass C:\n    if flag:\n        if (requests := fake):\n            pass\n        x = requests.get('u')\n", Some(ShadowedRoot)),
        ("earlier binding inside test", "import requests\nclass C:\n    if (requests := fake) and requests.get('u'):\n        pass\n", Some(ShadowedRoot)),
        ("earlier binding inside elif test", "import requests\nclass C:\n    if flag:\n        pass\n    elif (requests := fake) and requests.get('u'):\n        pass\n", Some(ShadowedRoot)),
        ("method skips class slot", "import requests\nclass C:\n    if flag:\n        pass\n    elif (requests := fake):\n        pass\n    def f(self, u):\n        return requests.get(u)\n", None),
        ("unrelated walrus", "import requests\nclass C:\n    if (other := fake):\n        pass\ndef f(u): return requests.get(u)\n", None),
    ], "requests.get");

    let repo = tempdir().unwrap();
    for (label, source) in [
        ("earlier arm", "import requests\nclass C:\n    if flag:\n        x = requests.get('u')\n    elif (requests := fake):\n        pass\n"),
        ("before the walrus stores", "import requests\nclass C:\n    if (requests := requests.get('u')):\n        pass\n"),
    ] {
        let out = file(source, repo.path(), &[Family::Http]);
        assert_eq!(out.unsupported.len(), 1, "{label}: {:?}", out.unproven);
        assert_eq!(out.unsupported[0].reason, UnsupportedReason::ModuleLevel, "{label}");
        assert!(out.unproven.is_empty(), "{label}");
    }
}

#[test]
fn type_only_guards_follow_redirects_through_nested_scopes() {
    let mut sources = Vec::new();
    for (label, guard, nested, cause) in [
        (
            "if class",
            "if TYPE_CHECKING:",
            "class C:",
            Some(UnprovenCause::BindingUnavailable),
        ),
        (
            "if function",
            "if TYPE_CHECKING:",
            "def setup():",
            Some(UnprovenCause::BindingUnavailable),
        ),
        (
            "elif class",
            "if flag:\n    pass\nelif TYPE_CHECKING:",
            "class C:",
            Some(UnprovenCause::BindingUnavailable),
        ),
        (
            "elif function",
            "if flag:\n    pass\nelif TYPE_CHECKING:",
            "def setup():",
            Some(UnprovenCause::BindingUnavailable),
        ),
        (
            "guard at its defining scope",
            "if TYPE_CHECKING:",
            "def setup(TYPE_CHECKING):",
            Some(UnprovenCause::BindingUnavailable),
        ),
        ("ordinary guard", "if flag:", "class C:", None),
        (
            "ordinary elif guard",
            "if TYPE_CHECKING:\n    pass\nelif flag:",
            "def setup():",
            None,
        ),
    ] {
        sources.push((label, format!("from typing import TYPE_CHECKING\n{guard}\n    {nested}\n        global r\n        import requests as r\ndef f(u): return r.get(u)\n"), cause));
    }
    sources.push(("nested guard uses enclosing function", "def setup(TYPE_CHECKING=False):\n    from typing import TYPE_CHECKING as tc\n    if tc:\n        class C:\n            global r\n            import requests as r\ndef f(u): return r.get(u)\n".into(), Some(UnprovenCause::BindingUnavailable)));
    sources.push(("class guard ignores inner parameter", "class Outer:\n    from typing import TYPE_CHECKING as tc\n    if tc:\n        def setup(tc):\n            global r\n            import requests as r\ndef f(u): return r.get(u)\n".into(), Some(UnprovenCause::BindingUnavailable)));
    assert_http_cases(
        &sources
            .iter()
            .map(|(label, source, cause)| (*label, source.as_str(), *cause))
            .collect::<Vec<_>>(),
        "requests.get",
    );
}

#[test]
fn type_only_class_guards_require_a_proven_reaching_import_on_each_path() {
    use UnprovenCause::BindingUnavailable;
    assert_http_cases(&[
        ("arm-local alias", "class C:\n    if flag:\n        from typing import TYPE_CHECKING as tc\n        if tc:\n            class D:\n                global requests\n                import requests\ndef f(u): return requests.get(u)\n", Some(BindingUnavailable)),
        ("both class paths bind typing aliases", "class C:\n    if flag:\n        from typing import TYPE_CHECKING as tc\n    else:\n        from typing_extensions import TYPE_CHECKING as tc\n    if tc:\n        class D:\n            global requests\n            import requests\ndef f(u): return requests.get(u)\n", Some(BindingUnavailable)),
        ("missing alias on one path", "class C:\n    if flag:\n        from typing import TYPE_CHECKING as tc\n    if tc:\n        class D:\n            global requests\n            import requests\ndef f(u): return requests.get(u)\n", None),
        ("ordinary binding on another path", "class C:\n    if flag:\n        from typing import TYPE_CHECKING as tc\n    else:\n        tc = True\n    if tc:\n        class D:\n            global requests\n            import requests\ndef f(u): return requests.get(u)\n", None),
        ("unproven module fallback", "if flag:\n    from typing import TYPE_CHECKING as tc\nclass C:\n    if tc:\n        class D:\n            global requests\n            import requests\ndef f(u): return requests.get(u)\n", None),
        ("deferred module fallback", "def setup():\n    global tc\n    from typing import TYPE_CHECKING as tc\nclass C:\n    if tc:\n        class D:\n            global requests\n            import requests\ndef f(u): return requests.get(u)\n", None),
        ("possibly skipped with import", "tc = True\nclass C:\n    with manager:\n        raise ValueError\n        from typing import TYPE_CHECKING as tc\n    if tc:\n        class D:\n            global requests\n            import requests\ndef f(u): return requests.get(u)\n", None),
        ("typing member written before import", "import typing\ntyping.TYPE_CHECKING = True\nclass C:\n    if flag:\n        from typing import TYPE_CHECKING as tc\n        if tc:\n            class D:\n                global requests\n                import requests\ndef f(u): return requests.get(u)\n", None),
    ], "requests.get");
}

#[test]
fn class_elif_test_bindings_reach_member_writes() {
    use UnprovenCause::MemberRebound;
    assert_http_cases(&[
        ("reviewed member write", "import requests\nclass Dummy: pass\nclass C:\n    r = Dummy()\n    if flag:\n        pass\n    elif (r := requests):\n        r.get = fake\ndef f(u): return requests.get(u)\n", Some(MemberRebound)),
        ("known unrelated old slot", "import requests\nimport httpx\nclass C:\n    r = httpx\n    if flag:\n        pass\n    elif (r := requests):\n        r.get = fake\ndef f(u): return requests.get(u)\n", Some(MemberRebound)),
        ("effect-free test", "import requests\nimport httpx\nclass C:\n    r = httpx\n    if flag:\n        pass\n    elif other:\n        r.get = fake\ndef f(u): return requests.get(u)\n", None),
    ], "requests.get");
}

#[test]
fn known_limits_assignment_target_execution_order() {
    assert_http_cases(&[
        // Runtime write order: evaluating a later target can see an earlier store.
        ("chained target", "import requests\nclass Dummy: pass\nr = Dummy()\nr = r.get = requests\ndef f(u): return requests.get(u)\n", None),
        ("tuple targets", "import requests\nclass Dummy: pass\nr = Dummy()\nr, r.get = requests, fake\ndef f(u): return requests.get(u)\n", None),
        ("chained target with scalar", "import requests\nr = 5\nr = r.get = requests\ndef f(u): return requests.get(u)\n", None),
        ("chained target in function", "import requests\ndef g():\n    r = 5\n    r = r.get = requests\ndef f(u): return requests.get(u)\n", None),
        ("with destructuring", "import requests\nr = 5\nwith manager() as (r, r.get):\n    pass\ndef f(u): return requests.get(u)\n", None),
        ("star target", "import requests\nr = 5\nr, *r.get = requests, fake\ndef f(u): return requests.get(u)\n", None),
        ("independent targets control", "import requests\nclass Dummy: pass\nr = Dummy()\nr.get, r = fake, requests\ndef f(u): return requests.get(u)\n", None),
    ], "requests.get");
    assert_http_cases(&[
        // Runtime write order: the original module has already been captured.
        ("inverse urllib request", "import urllib.request as saved\nimport urllib.request\nclass Replacement: pass\nurllib.request = urllib.request.urlopen = Replacement()\ndef f(u): return saved.urlopen(u)\n", Some(UnprovenCause::MemberRebound)),
        ("separate targets control", "import urllib.request as saved\nimport urllib.request\nclass Replacement: pass\nurllib.request = Replacement()\nurllib.request.urlopen = fake\ndef f(u): return saved.urlopen(u)\n", None),
    ], "urllib.request.urlopen");
}

#[test]
fn known_limits_captured_object_identity_and_execution() {
    use UnprovenCause::MemberRebound;
    assert_http_cases(&[
        // Captured identity: class and comprehension writes use separate lexical scopes.
        ("class body", "import urllib.request as saved\nimport urllib.request\nclass Replacement: pass\nurllib.request = Replacement()\nclass Writes:\n    urllib.request.urlopen = fake\ndef f(u): return saved.urlopen(u)\n", Some(MemberRebound)),
        ("eager comprehension", "import urllib.request as saved\nimport urllib.request\nclass Replacement: pass\nurllib.request = Replacement()\n[setattr(urllib.request, 'urlopen', fake) for _ in [0]]\ndef f(u): return saved.urlopen(u)\n", Some(MemberRebound)),
        // Captured identity: constructing a nonempty local class is opaque.
        ("local replacement with a nonempty class body", "import urllib.request as saved\nimport urllib.request\nclass Replacement:\n    x = 1\nurllib.request = Replacement()\nurllib.request.urlopen = fake\ndef f(u): return saved.urlopen(u)\n", Some(MemberRebound)),
        // Execution order: an earlier conditional effect survives a definite overwrite.
        ("conditional prefix replacement", "import urllib.request as saved\nimport urllib.request\nclass Replacement: pass\nif flag:\n    urllib.request = mystery\nurllib.request = Replacement()\nurllib.request.urlopen = fake\ndef f(u): return saved.urlopen(u)\n", Some(MemberRebound)),
        ("straight-line capture control", "import urllib.request as saved\nimport urllib.request\nclass Replacement: pass\nurllib.request = Replacement()\nurllib.request.urlopen = fake\ndef f(u): return saved.urlopen(u)\n", None),
    ], "urllib.request.urlopen");
    assert_http_cases(&[
        // Captured identity: local class objects are opaque to the write index.
        ("unrelated local class", "import requests\nclass Other:\n    x = 1\nOther.get = fake\ndef f(u): return requests.get(u)\n", Some(MemberRebound)),
        ("unrelated imported object control", "import requests\nimport httpx as other\nother.get = fake\ndef f(u): return requests.get(u)\n", None),
    ], "requests.get");
}

#[test]
fn known_limits_computed_reflection() {
    assert_http_cases(&[
        // Reflection: a computed member or namespace key is outside the literal forms.
        ("computed setattr", "import requests\nk = 'get'\nsetattr(requests, k, fake)\ndef f(u): return requests.get(u)\n", None),
        ("computed vars key", "import requests\nk = 'get'\nvars(requests)[k] = fake\ndef f(u): return requests.get(u)\n", None),
        ("computed globals key", "import requests\nk = 'requests'\nglobals()[k] = fake\ndef f(u): return requests.get(u)\n", None),
        ("literal unrelated member control", "import requests\nsetattr(requests, 'post', fake)\ndef f(u): return requests.get(u)\n", None),
        ("literal selected member control", "import requests\nvars(requests)['get'] = fake\ndef f(u): return requests.get(u)\n", Some(UnprovenCause::MemberRebound)),
        ("literal globals control", "import requests\nglobals()['requests'] = fake\ndef f(u): return requests.get(u)\n", Some(UnprovenCause::ShadowedRoot)),
    ], "requests.get");
}
