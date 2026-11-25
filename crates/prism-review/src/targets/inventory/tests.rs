use super::{
    collect, evidence, Family, Grade, InventoryOptions, InventoryOutput, UnprovenCause,
    UnsupportedReason, UnsupportedSite,
};
use crate::targets::{inventory_document, TargetsMeta};
use prism::ast::ParsedFile;
use prism::finding_confidence::{FindingConfidence, FindingTier, MinConfidence, ParseQuality};
use prism::languages::Language;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use tempfile::tempdir;

fn file(input: &str, repo: &Path, families: &[Family]) -> InventoryOutput {
    let name = "app/main.py";
    let parsed = ParsedFile::parse(name, input, Language::Python).unwrap();
    let mut out = InventoryOutput::default();
    super::python::collect_file(
        name,
        &parsed,
        repo,
        &families.iter().copied().collect::<BTreeSet<_>>(),
        &mut out,
    );
    out
}

#[test]
fn mixed_grades_are_filtered_before_unsupported_grouping() {
    let mut out = InventoryOutput::default();
    out.parse_quality
        .insert("client.py".into(), ParseQuality::Clean);
    for grade in [Grade::AnnotationTyped, Grade::Exact] {
        out.unsupported.push(UnsupportedSite {
            file: "client.py".into(),
            line: 1,
            symbol: None,
            family: Family::Http,
            resolved_callee: "requests.get".into(),
            reason: UnsupportedReason::ModuleLevel,
            grade,
        });
    }
    out.unproven.insert(
        "client.py".into(),
        BTreeMap::from([(UnprovenCause::ShadowedRoot, 1)]),
    );
    let mut meta = TargetsMeta::default();
    let both = inventory_document(&meta, &out);
    assert_eq!(
        both.targets[0].expected.detail.as_deref(),
        Some("module_level; 2 calls")
    );
    assert_eq!(both.targets[0].confidence, FindingConfidence::Exact);
    assert_eq!(both.targets[0].tier, FindingTier::Asserted);
    meta.min_confidence = MinConfidence::Exact;
    let exact = inventory_document(&meta, &out);
    assert_eq!(
        exact.targets[0].expected.detail.as_deref(),
        Some("module_level")
    );
    assert_eq!(exact.targets[0].id, both.targets[0].id);
    meta.min_tier = FindingTier::Asserted;
    out.parse_quality
        .insert("client.py".into(), ParseQuality::Degraded);
    let asserted = inventory_document(&meta, &out);
    assert!(asserted.targets.is_empty());
    assert!(asserted
        .warnings
        .iter()
        .any(|w| w.contains("shadowed_root: 1")));
}

#[test]
fn import_locality_and_builtin_startup_precedence() {
    let repo = tempdir().unwrap();
    let src = repo.path().join("src");
    std::fs::create_dir_all(src.join("requests")).unwrap();
    std::fs::write(src.join("requests/__init__.py"), "").unwrap();
    for module in ["builtins", "os", "io"] {
        std::fs::write(src.join(format!("{module}.py")), "").unwrap();
    }
    let http = file(
        "import requests\ndef f(p):\n    return requests.get(p)\n",
        repo.path(),
        &[Family::Http],
    );
    assert_eq!(
        http.unproven["app/main.py"][&UnprovenCause::RepositoryLocalModule],
        1
    );
    let internals = file(
        "import builtins as b\nfrom builtins import open as opened\nimport io\nimport os.path as p\nfrom os import path as os_path\ndef f(pth):\n    open(pth)\n    b.open(pth)\n    opened(pth)\n    io.open(pth)\n    p.exists(pth)\n    os_path.exists(pth)\n",
        repo.path(), &[Family::Filesystem],
    );
    let observed: Vec<_> = internals
        .sites
        .iter()
        .map(|s| s.resolved_callee.as_str())
        .collect();
    assert_eq!(
        observed,
        [
            "builtins.open",
            "builtins.open",
            "builtins.open",
            "io.open",
            "os.path.exists",
            "os.path.exists"
        ]
    );
    let relative = file(
        "from . import os\ndef f(p):\n    return os.open(p, 0)\n",
        repo.path(),
        &[Family::Filesystem],
    );
    assert_eq!(
        relative.unproven["app/main.py"][&UnprovenCause::RepositoryLocalModule],
        1
    );
    let shadowed = file(
        "import io\ndef f(io,p):\n    return io.open(p)\n",
        repo.path(),
        &[Family::Filesystem],
    );
    assert_eq!(
        shadowed.unproven["app/main.py"][&UnprovenCause::ShadowedRoot],
        1
    );
    let parsed = ParsedFile::parse("x.py", "def f(p): return open(p)\n", Language::Python).unwrap();
    let disabled = collect(
        &BTreeMap::from([("x.py".to_owned(), parsed)]),
        repo.path(),
        &InventoryOptions::default(),
    );
    assert!(disabled.sites.is_empty() && disabled.unproven.is_empty());
    assert_eq!(disabled.parse_quality["x.py"], ParseQuality::Clean);
}

#[test]
fn type_only_guards_and_member_writes_use_the_guard_site() {
    let repo = tempdir().unwrap();
    for guard in [
        "TYPE_CHECKING = False\nif TYPE_CHECKING:",
        "import typing_extensions as te\nif te.TYPE_CHECKING:",
    ] {
        let source = format!("{guard}\n    import requests as r\ndef f(u):\n    return r.get(u)\n");
        let out = file(&source, repo.path(), &[Family::Http]);
        assert_eq!(
            out.unproven["app/main.py"][&UnprovenCause::BindingUnavailable],
            1
        );
    }
    for source in [
        "import typing_extensions as te\nte.TYPE_CHECKING = True\nif te.TYPE_CHECKING:\n    import requests as r\ndef f(u):\n    return r.get(u)\n",
        "import typing as t\ndef f(t, u):\n    if t.TYPE_CHECKING:\n        import requests as r\n    return r.get(u)\n",
    ] {
        let out = file(source, repo.path(), &[Family::Http]);
        assert_eq!(out.sites.len(), 1);
        assert!(out.unproven.is_empty());
    }
}

#[test]
fn reflective_builtin_writes_and_target_operands() {
    let repo = tempdir().unwrap();
    for (source, cause) in [
        ("import builtins as b\ndef f(p, replacement):\n    setattr(b, 'o' 'pen', replacement)\n    return open(p)\n", UnprovenCause::MemberRebound),
        ("def f(p, replacement):\n    __builtins__['open'] = replacement\n    return open(p)\n", UnprovenCause::MemberRebound),
        ("def f(p, replacement):\n    globals()['__builtins__'] = replacement\n    return open(p)\n", UnprovenCause::ShadowedRoot),
    ] {
        let out = file(source, repo.path(), &[Family::Filesystem]);
        assert!(out.sites.is_empty());
        assert_eq!(out.unproven["app/main.py"][&cause], 1);
    }
    for (key, expected_sites) in [
        (r"\N{LATIN SMALL LETTER G}et", 0),
        (r"\N{LATIN SMALL LETTER P}ut", 1),
    ] {
        let source = format!("import requests\ndef f(u, replacement):\n    setattr(requests, '{key}', replacement)\n    return requests.get(u)\n");
        let out = file(&source, repo.path(), &[Family::Http]);
        assert_eq!(out.sites.len(), expected_sites, "{key}");
    }
    let out = file(
        "import requests\ndef f(cache,u):\n    cache[requests.get(u)]: requests.post(u) = 1\n",
        repo.path(),
        &[Family::Http],
    );
    assert_eq!(out.sites[0].resolved_callee, "requests.get");
    assert!(out.unsupported.is_empty() && out.unproven.is_empty());
}

#[test]
fn projected_sidecar_retains_selected_source_occurrences() {
    let repo = tempdir().unwrap();
    let out = file(
        "import requests\ndef f(u):\n    requests.get(u), requests.get(u)\n",
        repo.path(),
        &[Family::Http],
    );
    let doc = inventory_document(&TargetsMeta::default(), &out);
    let traces = evidence::sidecar(&doc.targets, &out.occurrences);
    let site = doc
        .targets
        .iter()
        .find(|row| row.kind == "external_call")
        .unwrap();
    let site_evidence: Vec<_> = traces.iter().filter(|trace| trace.id == site.id).collect();
    let [trace] = site_evidence.as_slice() else {
        panic!("one selected occurrence per site")
    };
    assert_eq!(trace.file, "app/main.py");
    assert_eq!(trace.proof.route, evidence::Route::Root);
    assert_eq!(trace.proof.call.text, "requests.get(u)");
    assert_eq!(trace.proof.call.range, [3, 4, 3, 19]);
    assert_eq!(trace.proof.callee.range, [3, 4, 3, 16]);
    let other = traces.iter().find(|row| row.id != trace.id).unwrap();
    assert_eq!(trace.proof.call.text, other.proof.call.text);
    assert_ne!(trace.proof.call.range, other.proof.call.range);
    assert_ne!(trace.occurrence, other.occurrence);
    assert_eq!(
        serde_json::to_value(&traces).unwrap()[0]["revision"],
        prism::api::build_info().git_sha
    );
}

#[test]
fn writes_distinguish_fresh_local_slots_and_persistent_members() {
    let repo = tempdir().unwrap();
    let local = file(
        "def f(urls):\n    for u in urls:\n        import requests as r\n        r.get(u)\n        del r\n",
        repo.path(), &[Family::Http],
    );
    assert_eq!(local.sites.len(), 1);
    let persistent = file(
        "import requests\ndef f(u, replacement):\n    from requests import get\n    get(u)\n    requests.get = replacement\n",
        repo.path(), &[Family::Http],
    );
    assert_eq!(
        persistent.unproven["app/main.py"][&UnprovenCause::MemberRebound],
        1
    );

    let class_owner = file(
        "from requests.sessions import Session as Impl\nfrom requests import Session\nImpl.get = replacement\ndef f(u):\n    return Session.get(u)\n",
        repo.path(), &[Family::Http],
    );
    assert_eq!(
        class_owner.unproven["app/main.py"][&UnprovenCause::MemberRebound],
        1
    );
    let shadowed_helper = file(
        "import requests\nglobals()['setattr'] = other\ndef f(u,replacement):\n    setattr(requests, 'get', replacement)\n    return requests.get(u)\n",
        repo.path(), &[Family::Http],
    );
    assert_eq!(shadowed_helper.sites.len(), 1);
}
