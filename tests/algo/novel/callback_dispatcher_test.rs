#[path = "../../common/mod.rs"]
mod common;
use common::*;
use prism::slice::SliceResult;

fn run_callback_dispatcher(
    files_input: Vec<(&str, &str, Language)>,
    diff: DiffInput,
) -> SliceResult {
    let mut files = BTreeMap::new();
    for (path, source, lang) in files_input {
        let parsed = ParsedFile::parse(path, source, lang).unwrap();
        files.insert(path.to_string(), parsed);
    }
    let config = SliceConfig::default().with_algorithm(SlicingAlgorithm::CallbackDispatcherSlice);
    algorithms::run_slicing_compat(&files, &diff, &config, None).unwrap()
}

#[test]
fn test_callback_dispatcher_designated_init_to_null_dispatch_c() {
    // File A: defines `my_func` and registers it via designated initialiser.
    let file_a = r#"
void my_func(struct vty *vty, struct lsa *lsa) {
    vty_out(vty, "lsa=%p\n", lsa);
}

static struct functab my_tab = {
    .show_opaque_info = my_func,
};
"#;
    // File B: dispatches via field with NULL first arg.
    let file_b = r#"
void some_dispatcher(struct functab *tab, struct lsa *lsa) {
    tab->show_opaque_info(NULL, lsa);
}
"#;
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: "ospfd/ospf_ext.c".to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([3]), // touches body of my_func
        }],
    };
    let result = run_callback_dispatcher(
        vec![
            ("ospfd/ospf_ext.c", file_a, Language::C),
            ("lib/log.c", file_b, Language::C),
        ],
        diff,
    );
    let null_findings: Vec<_> = result
        .findings
        .iter()
        .filter(|f| f.category.as_deref() == Some("callback_null_arg_dispatch"))
        .collect();
    assert_eq!(null_findings.len(), 1, "expected one NULL-arg finding");
    assert_eq!(null_findings[0].severity, "concern");
    assert!(
        null_findings[0]
            .related_files
            .iter()
            .any(|f| f == "lib/log.c"),
        "related_files should contain dispatch site, got: {:?}",
        null_findings[0].related_files
    );
}

#[test]
fn test_callback_dispatcher_assignment_field_clean_dispatch_c() {
    // Registration via `obj->cb = my_func` and clean (no NULL) dispatch.
    let file_a = r#"
void my_func(struct vty *vty, int x) {
    vty_out(vty, "%d\n", x);
}

void register_handler(struct callbacks *cb) {
    cb->show_opaque_info = my_func;
}
"#;
    let file_b = r#"
void dispatcher(struct callbacks *cb, struct vty *vty) {
    cb->show_opaque_info(vty, 42);
}
"#;
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: "src/handler.c".to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([3]),
        }],
    };
    let result = run_callback_dispatcher(
        vec![
            ("src/handler.c", file_a, Language::C),
            ("src/dispatch.c", file_b, Language::C),
        ],
        diff,
    );
    let chain_findings: Vec<_> = result
        .findings
        .iter()
        .filter(|f| f.category.as_deref() == Some("callback_dispatcher_chain"))
        .collect();
    assert_eq!(chain_findings.len(), 1);
    assert_eq!(chain_findings[0].severity, "info");
}

#[test]
fn test_callback_dispatcher_registrar_call_arg_c() {
    // Function registered via `*register*(my_func)` style call-arg.
    let file_a = r#"
void my_func(struct vty *vty) {
    vty_out(vty, "hello\n");
}

void setup(void) {
    ospf_register_opaque_functab(LSA_TYPE_RI, my_func);
}
"#;
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: "ospfd/handler.c".to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([3]),
        }],
    };
    let result = run_callback_dispatcher(vec![("ospfd/handler.c", file_a, Language::C)], diff);
    let registrar_findings: Vec<_> = result
        .findings
        .iter()
        .filter(|f| f.category.as_deref() == Some("callback_registrar_call"))
        .collect();
    assert_eq!(registrar_findings.len(), 1);
    assert_eq!(registrar_findings[0].severity, "warning");
    assert!(
        registrar_findings[0]
            .description
            .contains("ospf_register_opaque_functab"),
        "description should name the registrar, got: {}",
        registrar_findings[0].description
    );
}

#[test]
fn test_callback_dispatcher_no_invocations_no_finding_c() {
    // Registration exists, but no file invokes the field.
    let file_a = r#"
void my_func(struct vty *vty) {
    vty_out(vty, "x\n");
}

static struct functab my_tab = {
    .show_opaque_info = my_func,
};
"#;
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: "src/handler.c".to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([3]),
        }],
    };
    let result = run_callback_dispatcher(vec![("src/handler.c", file_a, Language::C)], diff);
    let chain_findings: Vec<_> = result
        .findings
        .iter()
        .filter(|f| {
            f.category.as_deref() == Some("callback_dispatcher_chain")
                || f.category.as_deref() == Some("callback_null_arg_dispatch")
        })
        .collect();
    assert!(
        chain_findings.is_empty(),
        "expected no chain finding when no invocations exist"
    );
}

#[test]
fn test_callback_dispatcher_designated_init_cpp() {
    let file_a = r#"
void my_handler(Widget *w, int code) {
    w->process(code);
}

static struct ops my_ops = {
    .on_event = my_handler,
};
"#;
    let file_b = r#"
void event_loop(struct ops *o, Widget *w) {
    o->on_event(w, 42);
}
"#;
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: "src/handler.cpp".to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([3]),
        }],
    };
    let result = run_callback_dispatcher(
        vec![
            ("src/handler.cpp", file_a, Language::Cpp),
            ("src/loop.cpp", file_b, Language::Cpp),
        ],
        diff,
    );
    let chain_findings: Vec<_> = result
        .findings
        .iter()
        .filter(|f| f.category.as_deref() == Some("callback_dispatcher_chain"))
        .collect();
    assert_eq!(chain_findings.len(), 1);
}

#[test]
fn test_callback_dispatcher_g_signal_connect_cpp() {
    // GLib `g_signal_connect(obj, "name", callback, user_data)` registrar pattern.
    let file_a = r#"
void on_clicked(GtkButton *btn, gpointer user_data) {
    do_work(btn);
}

void wire_up(GtkButton *btn) {
    g_signal_connect(btn, "clicked", on_clicked, NULL);
}
"#;
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: "src/ui.cpp".to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([3]),
        }],
    };
    let result = run_callback_dispatcher(vec![("src/ui.cpp", file_a, Language::Cpp)], diff);
    let registrar_findings: Vec<_> = result
        .findings
        .iter()
        .filter(|f| f.category.as_deref() == Some("callback_registrar_call"))
        .collect();
    assert_eq!(registrar_findings.len(), 1);
    assert_eq!(registrar_findings[0].severity, "warning");
    assert!(
        registrar_findings[0]
            .description
            .contains("g_signal_connect"),
        "description should name g_signal_connect, got: {}",
        registrar_findings[0].description
    );
}

#[test]
fn test_callback_dispatcher_unrelated_function_no_finding_cpp() {
    // Diff touches `my_func`, but only `other_func` is registered.
    let file_a = r#"
void my_func(Widget *w) {
    w->draw();
}

void other_func(Widget *w) {
    w->draw();
}

static struct ops my_ops = {
    .on_event = other_func,
};
"#;
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: "src/handler.cpp".to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([3]),
        }],
    };
    let result = run_callback_dispatcher(vec![("src/handler.cpp", file_a, Language::Cpp)], diff);
    let any_findings: Vec<_> = result
        .findings
        .iter()
        .filter(|f| {
            f.category
                .as_deref()
                .map(|c| c.starts_with("callback_"))
                .unwrap_or(false)
        })
        .collect();
    assert!(
        any_findings.is_empty(),
        "expected no callback findings when diff function is not the registered one"
    );
}
