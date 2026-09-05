//! `dependency_hint` acceptance tests for `kind: "external_call"` targets
//! (roadmap `03-tooling-plan-roadmap.md` §3 Phase 1; controller ruling
//! 2026-09-04 after the astra review).
//!
//! The contract these pin down: `callee` is the source-verbatim call chain at
//! the site and is emitted whenever one call can be attributed to the site;
//! `kind` is emitted ONLY with verified dependency identity, because the
//! harness restricts fault selection by `kind` BEFORE any callee glob runs
//! (`~/code/tools/specs/2026-09-04-runtime-harness-v0-spec.md` §5.2 steps
//! 2–4), so a wrong `kind` is worse than none.
//!
//! Split out of `targets_mapping_test.rs` (which was 686 lines) so every test
//! file stays under the 600-line cap.

use prism::api::{load_review_inputs, ReviewInputs, ReviewOptions};
use prism::slice::SliceFinding;
use prism::targets::{project, TargetsMeta};
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

/// A temp repo holding `files` (repo-relative path → source), with `path`
/// diffed at `diff_line`.
fn inputs_with_repo(
    files: &[(&str, &str)],
    path: &str,
    diff_line: usize,
) -> (TempDir, ReviewInputs) {
    let temp = TempDir::new().unwrap();
    for (name, source) in files {
        let full = temp.path().join(name);
        fs::create_dir_all(full.parent().unwrap()).unwrap();
        fs::write(&full, source).unwrap();
    }
    let diff = serde_json::json!({
        "files": [{"file_path": path, "modify_type": "Modified", "diff_lines": [diff_line]}]
    });
    let loaded = load_review_inputs(
        &ReviewOptions::new(temp.path()),
        &serde_json::to_string(&diff).unwrap(),
    )
    .unwrap();
    (temp, loaded)
}

/// One-file convenience over `inputs_with_repo`.
fn inputs_with(path: &str, source: &str, diff_line: usize) -> (TempDir, ReviewInputs) {
    inputs_with_repo(&[(path, source)], path, diff_line)
}

/// An `echo`/`missing_error_handling` finding at `(file, line)` whose
/// description carries no parseable callee — so the hint under test cannot
/// have come from `mapping::echo_callee`.
fn echo_finding(file: &str, line: usize) -> SliceFinding {
    SliceFinding {
        algorithm: "echo".to_string(),
        file: file.to_string(),
        line,
        severity: "warning".to_string(),
        description: "echo finding with no parseable callee in its prose".to_string(),
        function_name: None,
        related_lines: Vec::new(),
        related_files: Vec::new(),
        category: Some("missing_error_handling".to_string()),
        parse_quality: None,
        diagrams: Vec::new(),
    }
}

/// An `echo` finding whose description is the real `echo_slice.rs` prose, so
/// `mapping::echo_callee` recovers `callee` as the finding's own evidence.
fn echo_finding_naming(file: &str, line: usize, callee: &str) -> SliceFinding {
    let mut finding = echo_finding(file, line);
    finding.description = format!("'caller' calls '{callee}' without handling: ValueError");
    finding
}

fn meta(root: PathBuf) -> TargetsMeta {
    let mut meta = TargetsMeta::default();
    meta.algorithms_run = vec!["Test".to_string()];
    meta.repo_root = root;
    meta.min_severity_rank = prism::output::severity_rank("info");
    meta
}

fn hint_for(
    files: &[(&str, &str)],
    path: &str,
    line: usize,
    finding: SliceFinding,
) -> (Option<String>, Option<String>, Vec<String>) {
    let (temp, inputs) = inputs_with_repo(files, path, line);
    let doc = project(&[finding], &inputs, &meta(temp.path().to_path_buf()));
    let hint = doc.targets[0].dependency_hint.clone().unwrap();
    (hint.kind, hint.callee, doc.warnings.clone())
}

/// The positive case the whole feature exists for: an imported, unshadowed,
/// single-purpose library at the site yields both the verbatim chain and the
/// harness `kind`.
#[test]
fn imported_unshadowed_library_yields_callee_and_kind() {
    let (kind, callee, _) = hint_for(
        &[(
            "svc.py",
            "import requests\n\n\ndef send():\n    requests.post(\"http://x\")\n",
        )],
        "svc.py",
        5,
        echo_finding("svc.py", 5),
    );
    assert_eq!(callee.as_deref(), Some("requests.post"));
    assert_eq!(kind.as_deref(), Some("http"));
}

/// WRONG 1 (astra review, `dependency_hint.rs:131`). A repo that ships its own
/// `requests.py` binds `import requests` to that module. The spelling is
/// identical; the dependency identity is not. `callee` still ships.
#[test]
fn repo_local_module_shadowing_a_library_yields_callee_without_kind() {
    let (kind, callee, _) = hint_for(
        &[
            ("requests.py", "def post(url):\n    return None\n"),
            (
                "svc.py",
                "import requests\n\n\ndef send():\n    requests.post(\"http://x\")\n",
            ),
        ],
        "svc.py",
        5,
        echo_finding("svc.py", 5),
    );
    assert_eq!(callee.as_deref(), Some("requests.post"));
    assert_eq!(
        kind, None,
        "the repo's own requests.py is what `import requests` resolves to"
    );
}

/// WRONG 2 (`dependency_hint.rs:164`). `self.client` is owned by the class it
/// is assigned in; class A's constructor must not type class B's receiver.
#[test]
fn receiver_kind_comes_from_the_receivers_own_class() {
    let source = concat!(
        "import requests\n",
        "import sqlalchemy\n",
        "\n",
        "\n",
        "class A:\n",
        "    def __init__(self):\n",
        "        self.client = requests.Session()\n",
        "\n",
        "\n",
        "class B:\n",
        "    def __init__(self):\n",
        "        self.client = sqlalchemy.Session()\n",
        "\n",
        "    def load(self, model, key):\n",
        "        self.client.get(model, key)\n",
    );
    let (kind, callee, _) = hint_for(
        &[("svc.py", source)],
        "svc.py",
        15,
        echo_finding("svc.py", 15),
    );
    assert_eq!(callee.as_deref(), Some("self.client.get"));
    assert_eq!(
        kind.as_deref(),
        Some("db"),
        "an http kind here would exclude every db fault under §5.2 step 2"
    );
}

/// WRONG 3 (`dependency_hint.rs:90`). Two calls share the site line; the
/// finding's own resolved callee says which one it is about.
#[test]
fn same_line_sibling_call_follows_the_findings_own_callee() {
    let (kind, callee, _) = hint_for(
        &[(
            "svc.py",
            "import requests\n\n\ndef run(db, url):\n    db.commit(); requests.get(url)\n",
        )],
        "svc.py",
        5,
        echo_finding_naming("svc.py", 5, "commit"),
    );
    assert_eq!(
        callee.as_deref(),
        Some("db.commit"),
        "the finding is about db.commit(), not the http call beside it"
    );
    assert_eq!(kind, None);
}

/// WRONG 3, ambiguous half. When the evidence cannot single one call out, the
/// existing description-derived hint stands and the run records the ambiguity
/// rather than attributing an arbitrary call.
#[test]
fn unresolvable_same_line_ambiguity_keeps_the_existing_hint_and_warns() {
    let (kind, callee, warnings) = hint_for(
        &[("svc.py", "def run(a, b):\n    a.send(); b.send()\n")],
        "svc.py",
        2,
        echo_finding_naming("svc.py", 2, "send"),
    );
    assert_eq!(
        callee.as_deref(),
        Some("send"),
        "the description-derived hint is kept, not overwritten by a guess"
    );
    assert_eq!(kind, None);
    assert!(
        warnings
            .iter()
            .any(|warning| warning.contains("ambiguous-callee: 2 calls on line 2")),
        "ambiguity must be recorded: {warnings:?}"
    );
}

/// WRONG 4 (`dependency_hint.rs:204`). `redis` spans cache and queue, so no
/// entry of either kind can be trusted from the library name alone.
#[test]
fn multipurpose_library_yields_callee_without_kind() {
    let source = concat!(
        "import redis\n",
        "\n",
        "\n",
        "def run(payload):\n",
        "    r = redis.Redis()\n",
        "    r.publish(\"jobs\", payload)\n",
    );
    let (kind, callee, _) = hint_for(
        &[("svc.py", source)],
        "svc.py",
        6,
        echo_finding("svc.py", 6),
    );
    assert_eq!(callee.as_deref(), Some("r.publish"));
    assert_eq!(kind, None, "publish is a queue operation, not a cache one");
}

/// WRONG 5 (`dependency_hint.rs:105`). Java's `method_invocation` splits
/// receiver and name into separate fields; the callee is still the source span
/// from receiver through method name.
#[test]
fn java_callee_is_the_source_span_from_receiver_through_name() {
    let source = "class S {\n  void run() {\n    this.client.get(\"x\");\n  }\n}\n";
    let (kind, callee, _) = hint_for(
        &[("S.java", source)],
        "S.java",
        3,
        echo_finding("S.java", 3),
    );
    assert_eq!(callee.as_deref(), Some("this.client.get"));
    assert_eq!(kind, None, "there is no Java root-library table");
}

/// The receiver-shape, Go and negative-root shapes named in the acceptance
/// criteria, all through `project()` (moved here from
/// `targets_mapping_test.rs`, retightened for the verified-identity rule).
#[test]
fn external_call_dependency_hint_covers_receiver_go_and_negative_shapes() {
    let (temp, py_inputs) = inputs_with(
        "svc.py",
        "import requests\n\n\nclass C:\n    def __init__(self):\n        self.client = requests.Session()\n\n    def send(self):\n        self.client.get(\"x\")\n",
        9,
    );
    let doc = project(
        &[echo_finding("svc.py", 9)],
        &py_inputs,
        &meta(temp.path().to_path_buf()),
    );
    let hint = doc.targets[0].dependency_hint.clone().unwrap();
    assert_eq!(hint.callee.as_deref(), Some("self.client.get"));
    assert_eq!(hint.kind.as_deref(), Some("http"));

    let (temp, py_inputs) = inputs_with(
        "svc.py",
        "class C:\n    def send(self):\n        self.client = make_client()\n        self.client.get(\"x\")\n",
        4,
    );
    let doc = project(
        &[echo_finding("svc.py", 4)],
        &py_inputs,
        &meta(temp.path().to_path_buf()),
    );
    let hint = doc.targets[0].dependency_hint.clone().unwrap();
    assert_eq!(hint.callee.as_deref(), Some("self.client.get"));
    assert_eq!(
        hint.kind, None,
        "make_client() does not resolve through the root-library table"
    );

    let (temp, go_inputs) = inputs_with(
        "svc.go",
        "package main\n\nimport \"net/http\"\n\nfunc send() {\n\thttp.Get(\"x\")\n}\n",
        6,
    );
    let doc = project(
        &[echo_finding("svc.go", 6)],
        &go_inputs,
        &meta(temp.path().to_path_buf()),
    );
    let hint = doc.targets[0].dependency_hint.clone().unwrap();
    assert_eq!(hint.callee.as_deref(), Some("http.Get"));
    assert_eq!(hint.kind.as_deref(), Some("http"));

    let (temp, py_inputs) = inputs_with(
        "svc.py",
        "def send():\n    unknownlib.frobnicate(\"x\")\n",
        2,
    );
    let doc = project(
        &[echo_finding("svc.py", 2)],
        &py_inputs,
        &meta(temp.path().to_path_buf()),
    );
    let hint = doc.targets[0].dependency_hint.clone().unwrap();
    assert_eq!(hint.callee.as_deref(), Some("unknownlib.frobnicate"));
    assert_eq!(hint.kind, None, "unmapped root must never invent a kind");
}

/// A site with no call node at all keeps whatever hint the mapping produced —
/// the AST path may only improve a hint, never delete one.
#[test]
fn a_site_without_a_call_node_keeps_the_description_derived_hint() {
    let (kind, callee, _) = hint_for(
        &[("svc.py", "def run():\n    x = 1\n    return x\n")],
        "svc.py",
        2,
        echo_finding_naming("svc.py", 2, "commit"),
    );
    assert_eq!(callee.as_deref(), Some("commit"));
    assert_eq!(kind, None);
}
