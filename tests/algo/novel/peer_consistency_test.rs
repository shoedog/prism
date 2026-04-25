#[path = "../../common/mod.rs"]
mod common;
use common::*;
use prism::slice::SliceResult;

fn run_peer_consistency(
    source: &str,
    path: &str,
    lang: Language,
    diff_lines: BTreeSet<usize>,
) -> SliceResult {
    let parsed = ParsedFile::parse(path, source, lang).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines,
        }],
    };

    let config = SliceConfig::default().with_algorithm(SlicingAlgorithm::PeerConsistencySlice);
    algorithms::run_slicing_compat(&files, &diff, &config, None).unwrap()
}

#[test]
fn test_peer_consistency_uniform_unguarded_cluster_c() {
    // 3 siblings sharing first-param `vty`, all dereference (vty_out), none guard.
    let source = r#"
void show_vty_a(struct vty *vty, int x) {
    vty_out(vty, "a=%d\n", x);
}

void show_vty_b(struct vty *vty, int y) {
    vty_out(vty, "b=%d\n", y);
}

void show_vty_c(struct vty *vty, int z) {
    vty_out(vty, "c=%d\n", z);
}
"#;
    // Diff touches show_vty_a body
    let result = run_peer_consistency(source, "ospfd/ospf_ext.c", Language::C, BTreeSet::from([3]));
    let findings: Vec<_> = result
        .findings
        .iter()
        .filter(|f| f.category.as_deref() == Some("peer_guard_divergence"))
        .collect();
    assert_eq!(findings.len(), 1, "expected exactly one cluster finding");
    assert_eq!(findings[0].severity, "concern");
    assert!(
        findings[0].description.contains("sibling functions"),
        "description should describe cluster, got: {}",
        findings[0].description
    );
}

#[test]
fn test_peer_consistency_divergent_cluster_c() {
    // 4 siblings sharing first-param `vty`. 3 guard with `if (vty)`, 1 unguarded.
    let source = r#"
void show_vty_a(struct vty *vty, int x) {
    if (vty) {
        vty_out(vty, "a=%d\n", x);
    }
}

void show_vty_b(struct vty *vty, int y) {
    if (vty) {
        vty_out(vty, "b=%d\n", y);
    }
}

void show_vty_c(struct vty *vty, int z) {
    if (vty) {
        vty_out(vty, "c=%d\n", z);
    }
}

void show_vty_d(struct vty *vty, int w) {
    vty_out(vty, "d=%d\n", w);
}
"#;
    // Diff touches show_vty_d body (the divergent one)
    let result = run_peer_consistency(
        source,
        "ospfd/ospf_ext.c",
        Language::C,
        BTreeSet::from([21]),
    );
    let findings: Vec<_> = result
        .findings
        .iter()
        .filter(|f| f.category.as_deref() == Some("peer_guard_divergence"))
        .collect();
    assert_eq!(findings.len(), 1, "expected one divergent-cluster finding");
    assert_eq!(findings[0].severity, "warning");
    assert!(
        findings[0].description.contains("show_vty_d"),
        "description should name the divergent function, got: {}",
        findings[0].description
    );
}

#[test]
fn test_peer_consistency_all_guarded_no_finding_c() {
    let source = r#"
void show_vty_a(struct vty *vty, int x) {
    if (vty) {
        vty_out(vty, "a=%d\n", x);
    }
}

void show_vty_b(struct vty *vty, int y) {
    if (vty) {
        vty_out(vty, "b=%d\n", y);
    }
}

void show_vty_c(struct vty *vty, int z) {
    if (vty) {
        vty_out(vty, "c=%d\n", z);
    }
}
"#;
    let result = run_peer_consistency(source, "ospfd/ospf_ext.c", Language::C, BTreeSet::from([3]));
    let findings: Vec<_> = result
        .findings
        .iter()
        .filter(|f| f.category.as_deref() == Some("peer_guard_divergence"))
        .collect();
    assert!(
        findings.is_empty(),
        "expected no finding when all siblings are guarded, got: {:?}",
        findings.iter().map(|f| &f.description).collect::<Vec<_>>()
    );
}

#[test]
fn test_peer_consistency_cluster_too_small_no_finding_c() {
    // Only 2 siblings — below the cluster_size >= 3 threshold.
    let source = r#"
void show_vty_a(struct vty *vty, int x) {
    vty_out(vty, "a=%d\n", x);
}

void show_vty_b(struct vty *vty, int y) {
    vty_out(vty, "b=%d\n", y);
}
"#;
    let result = run_peer_consistency(source, "ospfd/ospf_ext.c", Language::C, BTreeSet::from([3]));
    let findings: Vec<_> = result
        .findings
        .iter()
        .filter(|f| f.category.as_deref() == Some("peer_guard_divergence"))
        .collect();
    assert!(
        findings.is_empty(),
        "expected no finding when cluster size < 3"
    );
}

#[test]
fn test_peer_consistency_uniform_unguarded_cluster_cpp() {
    let source = r#"
void show_widget_a(Widget *w, int x) {
    w->draw(x);
}

void show_widget_b(Widget *w, int y) {
    w->draw(y);
}

void show_widget_c(Widget *w, int z) {
    w->draw(z);
}
"#;
    let result = run_peer_consistency(
        source,
        "src/widget_show.cpp",
        Language::Cpp,
        BTreeSet::from([3]),
    );
    let findings: Vec<_> = result
        .findings
        .iter()
        .filter(|f| f.category.as_deref() == Some("peer_guard_divergence"))
        .collect();
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].severity, "concern");
}

#[test]
fn test_peer_consistency_divergent_cluster_cpp() {
    let source = r#"
void show_widget_a(Widget *w, int x) {
    if (w) {
        w->draw(x);
    }
}

void show_widget_b(Widget *w, int y) {
    if (w) {
        w->draw(y);
    }
}

void show_widget_c(Widget *w, int z) {
    if (w) {
        w->draw(z);
    }
}

void show_widget_d(Widget *w, int q) {
    w->draw(q);
}
"#;
    let result = run_peer_consistency(
        source,
        "src/widget_show.cpp",
        Language::Cpp,
        BTreeSet::from([21]),
    );
    let findings: Vec<_> = result
        .findings
        .iter()
        .filter(|f| f.category.as_deref() == Some("peer_guard_divergence"))
        .collect();
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].severity, "warning");
    assert!(findings[0].description.contains("show_widget_d"));
}

#[test]
fn test_peer_consistency_only_fires_on_diff_touched_param_cpp() {
    // 3 siblings exist but the diff doesn't touch any of them — no finding,
    // because `touched_params` only includes params from diff-touched functions.
    let source = r#"
void unrelated() {
    int x = 0;
}

void show_widget_a(Widget *w, int x) {
    w->draw(x);
}

void show_widget_b(Widget *w, int y) {
    w->draw(y);
}

void show_widget_c(Widget *w, int z) {
    w->draw(z);
}
"#;
    // Diff touches `unrelated` (line 3), not any sibling
    let result = run_peer_consistency(
        source,
        "src/widget_show.cpp",
        Language::Cpp,
        BTreeSet::from([3]),
    );
    let findings: Vec<_> = result
        .findings
        .iter()
        .filter(|f| f.category.as_deref() == Some("peer_guard_divergence"))
        .collect();
    assert!(
        findings.is_empty(),
        "expected no finding when diff doesn't touch a cluster member"
    );
}
