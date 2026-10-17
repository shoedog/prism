// Match production stack budgets for case setup, CPG builds and nested Rayon work.
use super::common::*;
use prism::data_flow::test_support::assert_corpus;
use std::path::PathBuf;
use std::time::Instant;
trait Case {
    fn pair(self) -> (BTreeMap<String, ParsedFile>, DiffInput);
}
impl Case for (BTreeMap<String, ParsedFile>, DiffInput) {
    fn pair(self) -> Self {
        self
    }
}
impl Case
    for (
        BTreeMap<String, ParsedFile>,
        BTreeMap<String, String>,
        DiffInput,
    )
{
    fn pair(self) -> (BTreeMap<String, ParsedFile>, DiffInput) {
        (self.0, self.2)
    }
}

pub fn fixture_paths(dir: &Path, out: &mut Vec<PathBuf>) {
    let mut entries: Vec<_> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    entries.sort();
    for path in entries {
        let meta = std::fs::symlink_metadata(&path).unwrap();
        assert!(
            !meta.file_type().is_symlink(),
            "fixture symlink: {}",
            path.display()
        );
        if meta.is_dir() {
            fixture_paths(&path, out);
        } else if Language::from_path(path.to_str().unwrap()).is_some() {
            out.push(path);
        }
    }
}
pub fn parsed(path: &str) -> ParsedFile {
    ParsedFile::parse(
        path,
        &std::fs::read_to_string(path).unwrap(),
        Language::from_path(path).unwrap(),
    )
    .unwrap()
}
pub fn check(name: &str, files: &BTreeMap<String, ParsedFile>) {
    let start = Instant::now();
    assert_corpus(files);
    super::outputs::assert_assembly(name, files);
    eprintln!("DFG_CASE {name} {:?}", start.elapsed());
}
macro_rules! fixtures {
    ($($name:ident => $path:literal,)*) => {
        $(#[test] fn $name() {
            prism::build_pool::install(|| {
                check(&format!("fixture-{}",$path.replace('/',"__")), &BTreeMap::from([($path.into(),parsed($path))]));
            });
        })*
        pub(super) const REGISTERED: &[&str] = &[$($path,)*];
    };
}
fixtures! {
    fixture_bash_firmware_update_sh => "tests/fixtures/bash/firmware_update.sh",
    fixture_c_cpg_stack_regression_table_enc_12bit_h => "tests/fixtures/c/cpg_stack_regression/table_enc_12bit.h",
    fixture_c_cpg_stack_regression_tables_c => "tests/fixtures/c/cpg_stack_regression/tables.c",
    fixture_c_cve_array_index_c => "tests/fixtures/c/cve_array_index.c",
    fixture_c_cve_double_free_shared_label_c => "tests/fixtures/c/cve_double_free_shared_label.c",
    fixture_c_cve_fd_leak_c => "tests/fixtures/c/cve_fd_leak.c",
    fixture_c_cve_ioctl_overflow_c => "tests/fixtures/c/cve_ioctl_overflow.c",
    fixture_c_cve_kernel_leak_c => "tests/fixtures/c/cve_kernel_leak.c",
    fixture_c_cve_netlink_overflow_c => "tests/fixtures/c/cve_netlink_overflow.c",
    fixture_c_cve_null_deref_c => "tests/fixtures/c/cve_null_deref.c",
    fixture_c_cve_off_by_one_c => "tests/fixtures/c/cve_off_by_one.c",
    fixture_c_cve_ploam_goto_leak_c => "tests/fixtures/c/cve_ploam_goto_leak.c",
    fixture_c_cve_realloc_overflow_c => "tests/fixtures/c/cve_realloc_overflow.c",
    fixture_c_cve_refcount_race_c => "tests/fixtures/c/cve_refcount_race.c",
    fixture_c_cve_signedness_c => "tests/fixtures/c/cve_signedness.c",
    fixture_c_cve_strncpy_no_null_c => "tests/fixtures/c/cve_strncpy_no_null.c",
    fixture_c_cve_syslog_format_c => "tests/fixtures/c/cve_syslog_format.c",
    fixture_c_cve_uci_injection_c => "tests/fixtures/c/cve_uci_injection.c",
    fixture_c_cve_unchecked_return_c => "tests/fixtures/c/cve_unchecked_return.c",
    fixture_c_cve_variadic_wrapper_c => "tests/fixtures/c/cve_variadic_wrapper.c",
    fixture_c_deep_switch_c => "tests/fixtures/c/deep_switch.c",
    fixture_c_double_free_c => "tests/fixtures/c/double_free.c",
    fixture_c_large_function_c => "tests/fixtures/c/large_function.c",
    fixture_c_macro_heavy_c => "tests/fixtures/c/macro_heavy.c",
    fixture_c_onu_state_machine_c => "tests/fixtures/c/onu_state_machine.c",
    fixture_c_ring_overflow_c => "tests/fixtures/c/ring_overflow.c",
    fixture_c_snmp_overflow_c => "tests/fixtures/c/snmp_overflow.c",
    fixture_c_timer_uaf_c => "tests/fixtures/c/timer_uaf.c",
    fixture_hapi_4552_source_lib_transmit_js => "tests/fixtures/hapi-4552-source/lib/transmit.js",
    fixture_hapi_4552_source_test_transmit_js => "tests/fixtures/hapi-4552-source/test/transmit.js",
    fixture_item2_selected_evidence_a_py => "tests/fixtures/item2-selected-evidence/a.py",
    fixture_item2_selected_evidence_b_py => "tests/fixtures/item2-selected-evidence/b.py",
    fixture_nav_callgraph_main_py => "tests/fixtures/nav_callgraph/main.py",
    fixture_nav_callgraph_util_py => "tests/fixtures/nav_callgraph/util.py",
    fixture_mcp_launcher_argv_stub_sh => "tests/fixtures/mcp/launcher/argv-stub.sh",
    fixture_nav_compat_a_py => "tests/fixtures/nav_compat/a.py",
    fixture_nav_scoped_rust_src_algo_rs => "tests/fixtures/nav_scoped_rust/src/algo.rs",
    fixture_nav_scoped_rust_src_lib_rs => "tests/fixtures/nav_scoped_rust/src/lib.rs",
    fixture_p8_param_slots_callbacks_js => "tests/fixtures/p8-param-slots/callbacks.js",
    fixture_python_calc_py => "tests/fixtures/python/calc.py",
    fixture_review_no_diagrams_views_py => "tests/fixtures/review_no_diagrams/views.py",
    fixture_sanitizer_suite_go_sanitized_01_path_clean_hasprefix_go => "tests/fixtures/sanitizer-suite-go/sanitized/01_path_clean_hasprefix.go",
    fixture_sanitizer_suite_go_sanitized_02_filepath_rel_check_go => "tests/fixtures/sanitizer-suite-go/sanitized/02_filepath_rel_check.go",
    fixture_sanitizer_suite_go_sanitized_03_clean_then_join_with_check_go => "tests/fixtures/sanitizer-suite-go/sanitized/03_clean_then_join_with_check.go",
    fixture_sanitizer_suite_go_sanitized_04_gin_param_clean_go => "tests/fixtures/sanitizer-suite-go/sanitized/04_gin_param_clean.go",
    fixture_sanitizer_suite_go_sanitized_05_gorilla_mux_clean_go => "tests/fixtures/sanitizer-suite-go/sanitized/05_gorilla_mux_clean.go",
    fixture_sanitizer_suite_go_sanitized_06_path_clean_for_write_go => "tests/fixtures/sanitizer-suite-go/sanitized/06_path_clean_for_write.go",
    fixture_sanitizer_suite_go_sanitized_07_path_clean_for_remove_go => "tests/fixtures/sanitizer-suite-go/sanitized/07_path_clean_for_remove.go",
    fixture_sanitizer_suite_go_sanitized_08_nethttp_servefile_with_check_go => "tests/fixtures/sanitizer-suite-go/sanitized/08_nethttp_servefile_with_check.go",
    fixture_sanitizer_suite_go_sanitized_09_gin_cfile_with_check_go => "tests/fixtures/sanitizer-suite-go/sanitized/09_gin_cfile_with_check.go",
    fixture_sanitizer_suite_go_sanitized_10_complex_path_validation_go => "tests/fixtures/sanitizer-suite-go/sanitized/10_complex_path_validation.go",
    fixture_sanitizer_suite_go_unsanitized_01_direct_taint_to_readfile_go => "tests/fixtures/sanitizer-suite-go/unsanitized/01_direct_taint_to_readfile.go",
    fixture_sanitizer_suite_go_unsanitized_02_clean_no_hasprefix_go => "tests/fixtures/sanitizer-suite-go/unsanitized/02_clean_no_hasprefix.go",
    fixture_sanitizer_suite_go_unsanitized_03_filepath_base_not_a_cleanser_go => "tests/fixtures/sanitizer-suite-go/unsanitized/03_filepath_base_not_a_cleanser.go",
    fixture_sanitizer_suite_go_unsanitized_04_nethttp_servefile_no_check_go => "tests/fixtures/sanitizer-suite-go/unsanitized/04_nethttp_servefile_no_check.go",
    fixture_sanitizer_suite_go_unsanitized_05_gin_cfile_no_check_go => "tests/fixtures/sanitizer-suite-go/unsanitized/05_gin_cfile_no_check.go",
    fixture_sanitizer_suite_go_unsanitized_06_gorilla_mux_no_check_go => "tests/fixtures/sanitizer-suite-go/unsanitized/06_gorilla_mux_no_check.go",
    fixture_sanitizer_suite_go_unsanitized_07_writefile_unsanitized_go => "tests/fixtures/sanitizer-suite-go/unsanitized/07_writefile_unsanitized.go",
    fixture_sanitizer_suite_go_unsanitized_08_remove_unsanitized_go => "tests/fixtures/sanitizer-suite-go/unsanitized/08_remove_unsanitized.go",
    fixture_sanitizer_suite_go_unsanitized_09_cwe78_shell_wrapped_go => "tests/fixtures/sanitizer-suite-go/unsanitized/09_cwe78_shell_wrapped.go",
    fixture_sanitizer_suite_go_unsanitized_10_cwe78_tainted_binary_go => "tests/fixtures/sanitizer-suite-go/unsanitized/10_cwe78_tainted_binary.go",
    fixture_sanitizer_suite_js_ts_sanitized_01_nestjs_tsx_text_interpolation_tsx => "tests/fixtures/sanitizer-suite-js-ts/sanitized/01_nestjs_tsx_text_interpolation.tsx",
    fixture_sanitizer_suite_js_ts_sanitized_02_express_sequelize_bind_js => "tests/fixtures/sanitizer-suite-js-ts/sanitized/02_express_sequelize_bind.js",
    fixture_sanitizer_suite_js_ts_sanitized_03_koa_fetch_allowlist_js => "tests/fixtures/sanitizer-suite-js-ts/sanitized/03_koa_fetch_allowlist.js",
    fixture_sanitizer_suite_js_ts_sanitized_04_express_json_parse_js => "tests/fixtures/sanitizer-suite-js-ts/sanitized/04_express_json_parse.js",
    fixture_sanitizer_suite_js_ts_sanitized_05_fastify_execfile_literal_ts => "tests/fixtures/sanitizer-suite-js-ts/sanitized/05_fastify_execfile_literal.ts",
    fixture_sanitizer_suite_js_ts_sanitized_06_express_sendfile_resolved_js => "tests/fixtures/sanitizer-suite-js-ts/sanitized/06_express_sendfile_resolved.js",
    fixture_sanitizer_suite_js_ts_unsanitized_01_nestjs_tsx_dangerous_html_tsx => "tests/fixtures/sanitizer-suite-js-ts/unsanitized/01_nestjs_tsx_dangerous_html.tsx",
    fixture_sanitizer_suite_js_ts_unsanitized_02_express_sequelize_query_js => "tests/fixtures/sanitizer-suite-js-ts/unsanitized/02_express_sequelize_query.js",
    fixture_sanitizer_suite_js_ts_unsanitized_03_koa_fetch_ssrf_js => "tests/fixtures/sanitizer-suite-js-ts/unsanitized/03_koa_fetch_ssrf.js",
    fixture_sanitizer_suite_js_ts_unsanitized_04_express_node_serialize_js => "tests/fixtures/sanitizer-suite-js-ts/unsanitized/04_express_node_serialize.js",
    fixture_sanitizer_suite_js_ts_unsanitized_05_fastify_child_process_exec_ts => "tests/fixtures/sanitizer-suite-js-ts/unsanitized/05_fastify_child_process_exec.ts",
    fixture_sanitizer_suite_js_ts_unsanitized_06_express_sendfile_path_js => "tests/fixtures/sanitizer-suite-js-ts/unsanitized/06_express_sendfile_path.js",
    fixture_sanitizer_suite_python_sanitized_01_xss_html_escape_mark_safe_py => "tests/fixtures/sanitizer-suite-python/sanitized/01_xss_html_escape_mark_safe.py",
    fixture_sanitizer_suite_python_sanitized_02_xss_autoescape_render_py => "tests/fixtures/sanitizer-suite-python/sanitized/02_xss_autoescape_render.py",
    fixture_sanitizer_suite_python_sanitized_03_xss_format_html_literal_py => "tests/fixtures/sanitizer-suite-python/sanitized/03_xss_format_html_literal.py",
    fixture_sanitizer_suite_python_sanitized_04_sql_dbapi_params_py => "tests/fixtures/sanitizer-suite-python/sanitized/04_sql_dbapi_params.py",
    fixture_sanitizer_suite_python_sanitized_05_sqlalchemy_bindparams_py => "tests/fixtures/sanitizer-suite-python/sanitized/05_sqlalchemy_bindparams.py",
    fixture_sanitizer_suite_python_sanitized_06_ssrf_url_allowlist_py => "tests/fixtures/sanitizer-suite-python/sanitized/06_ssrf_url_allowlist.py",
    fixture_sanitizer_suite_python_sanitized_07_yaml_safe_load_py => "tests/fixtures/sanitizer-suite-python/sanitized/07_yaml_safe_load.py",
    fixture_sanitizer_suite_python_sanitized_08_yaml_load_safeloader_py => "tests/fixtures/sanitizer-suite-python/sanitized/08_yaml_load_safeloader.py",
    fixture_sanitizer_suite_python_sanitized_09_fastapi_pydantic_sql_params_py => "tests/fixtures/sanitizer-suite-python/sanitized/09_fastapi_pydantic_sql_params.py",
    fixture_sanitizer_suite_python_sanitized_10_fastapi_request_yaml_safeloader_py => "tests/fixtures/sanitizer-suite-python/sanitized/10_fastapi_request_yaml_safeloader.py",
    fixture_sanitizer_suite_python_sanitized_11_ssrf_aiohttp_url_allowlist_py => "tests/fixtures/sanitizer-suite-python/sanitized/11_ssrf_aiohttp_url_allowlist.py",
    fixture_sanitizer_suite_python_unsanitized_01_xss_mark_safe_py => "tests/fixtures/sanitizer-suite-python/unsanitized/01_xss_mark_safe.py",
    fixture_sanitizer_suite_python_unsanitized_02_xss_safe_filter_py => "tests/fixtures/sanitizer-suite-python/unsanitized/02_xss_safe_filter.py",
    fixture_sanitizer_suite_python_unsanitized_03_xss_format_html_tainted_format_py => "tests/fixtures/sanitizer-suite-python/unsanitized/03_xss_format_html_tainted_format.py",
    fixture_sanitizer_suite_python_unsanitized_04_sql_fstring_py => "tests/fixtures/sanitizer-suite-python/unsanitized/04_sql_fstring.py",
    fixture_sanitizer_suite_python_unsanitized_05_sqlalchemy_raw_text_py => "tests/fixtures/sanitizer-suite-python/unsanitized/05_sqlalchemy_raw_text.py",
    fixture_sanitizer_suite_python_unsanitized_06_ssrf_requests_get_py => "tests/fixtures/sanitizer-suite-python/unsanitized/06_ssrf_requests_get.py",
    fixture_sanitizer_suite_python_unsanitized_07_ssrf_inverted_allowlist_py => "tests/fixtures/sanitizer-suite-python/unsanitized/07_ssrf_inverted_allowlist.py",
    fixture_sanitizer_suite_python_unsanitized_08_yaml_load_py => "tests/fixtures/sanitizer-suite-python/unsanitized/08_yaml_load.py",
    fixture_sanitizer_suite_python_unsanitized_09_pickle_loads_request_body_py => "tests/fixtures/sanitizer-suite-python/unsanitized/09_pickle_loads_request_body.py",
    fixture_sanitizer_suite_python_unsanitized_10_fastapi_pydantic_sql_fstring_py => "tests/fixtures/sanitizer-suite-python/unsanitized/10_fastapi_pydantic_sql_fstring.py",
    fixture_sanitizer_suite_python_unsanitized_11_ssrf_aiohttp_client_session_py => "tests/fixtures/sanitizer-suite-python/unsanitized/11_ssrf_aiohttp_client_session.py",
    fixture_targets_client_py => "tests/fixtures/targets/client.py",
    fixture_targets_svc_py => "tests/fixtures/targets/svc.py",
    fixture_targets_hint_requests_py => "tests/fixtures/targets_hint/requests.py",
    fixture_targets_hint_svc_py => "tests/fixtures/targets_hint/svc.py",
    fixture_terraform_main_tf => "tests/fixtures/terraform/main.tf",
    fixture_js_alias_matrix_generator_export_cjs => "tests/fixtures/js_alias_matrix/generator/export.cjs",
    fixture_js_alias_matrix_generator_lib_cjs => "tests/fixtures/js_alias_matrix/generator/lib.cjs",
    fixture_js_alias_matrix_generator_run_cjs => "tests/fixtures/js_alias_matrix/generator/run.cjs",
    fixture_js_alias_matrix_generator_variants_anon_cjs => "tests/fixtures/js_alias_matrix/generator/variants-anon.cjs",
    fixture_js_alias_matrix_generator_variants_common_cjs =>"tests/fixtures/js_alias_matrix/generator/variants-common.cjs",
    fixture_js_alias_matrix_generator_variants_imports_cjs => "tests/fixtures/js_alias_matrix/generator/variants-imports.cjs",
    fixture_js_alias_matrix_generator_variants_local_shapes_cjs => "tests/fixtures/js_alias_matrix/generator/variants-local-shapes.cjs",
    fixture_js_alias_matrix_generator_variants_local_cjs => "tests/fixtures/js_alias_matrix/generator/variants-local.cjs",
    fixture_js_alias_matrix_generator_variants_params_cjs => "tests/fixtures/js_alias_matrix/generator/variants-params.cjs",
    fixture_js_alias_matrix_generator_variants_returns_cjs => "tests/fixtures/js_alias_matrix/generator/variants-returns.cjs",
    fixture_js_alias_matrix_generator_variants_cjs => "tests/fixtures/js_alias_matrix/generator/variants.cjs",
}
/// The whole fixture tree as one multi-file case. The census (every fixture registered) is in
/// `pins::`, with the stored hashes, so a fixture added on main fails the controller gate, not
/// the CI parity job of an unrelated PR (review round 2, Opus S2).
#[test]
fn fixture_tree_multifile_storage() {
    prism::build_pool::install(|| {
        let mut paths = Vec::new();
        fixture_paths(Path::new("tests/fixtures"), &mut paths);
        let mut counts = BTreeMap::new();
        let mut files = BTreeMap::new();
        for p in paths {
            let path = p.to_str().unwrap();
            *counts
                .entry(format!("{:?}", Language::from_path(path).unwrap()))
                .or_insert(0usize) += 1;
            files.insert(path.into(), parsed(path));
        }
        eprintln!("DFG_FIXTURES {:?}", counts);
        prism::data_flow::test_support::assert_storage_assembly(&files);
        super::outputs::assert_assembly("fixture-tree", &files);
    });
}
macro_rules! generator {
    ($($test:ident,$name:literal,$make:ident;)*) => {
        $(#[test]
        fn $test() {
            prism::build_pool::install(|| {
                let (files, diff) = $make().pair();
                check(concat!("generator-", $name), &files);
                let _ = diff;
            });
        })*
        pub(super) const GENERATOR_NAMES: &[&str] = &[$(concat!("generator-", $name),)*];
    };
}
generator! {
    generator_python, "python", make_python_test;
    generator_javascript, "javascript", make_javascript_test;
    generator_go, "go", make_go_test;
    generator_java, "java", make_java_test;
    generator_typescript, "typescript", make_typescript_test;
    generator_c, "c", make_c_test;
    generator_c_multifile, "c-multifile", make_c_multifile_test;
    generator_cpp, "cpp", make_cpp_test;
    generator_tsx, "tsx", make_tsx_test;
    generator_terraform, "terraform", make_terraform_test;
    generator_bash, "bash", make_bash_test;
    generator_snmp, "snmp", make_snmp_overflow_test;
    generator_double_free, "double-free", make_double_free_test;
    generator_ring, "ring", make_ring_overflow_test;
    generator_timer, "timer", make_timer_uaf_test;
    generator_large, "large", make_large_function_test;
    generator_deep_switch, "deep-switch", make_deep_switch_test;
}

pub fn explicit(path: &str, src: &str, lang: Language) -> BTreeMap<String, ParsedFile> {
    BTreeMap::from([(path.into(), ParsedFile::parse(path, src, lang).unwrap())])
}
#[test]
fn generator_lua() {
    prism::build_pool::install(|| {
        check(
            "generator-lua",
            &explicit(
                "compact.lua",
                "local function f(x)\n local y = x\n return y\nend\n",
                Language::Lua,
            ),
        );
    });
}
#[test]
fn generator_rust() {
    prism::build_pool::install(|| {
        check(
            "generator-rust",
            &explicit(
                "compact.rs",
                "fn f(x: i32) -> i32 { let y = x; y }",
                Language::Rust,
            ),
        );
    });
}
#[test]
fn helper_builders() {
    prism::build_pool::install(|| {
        let py = "def f(x):\n    return x\n";
        let rs = "fn f(x: i32) -> i32 { x }";
        let p = explicit("test.py", py, Language::Python);
        let r = explicit("test.rs", rs, Language::Rust);
        for (name, files, graph) in [
            ("helper-python", p, build_python_cpg(py)),
            ("helper-rust", r, build_rust_cpg(rs)),
        ] {
            check(&format!("generator-{name}"), &files);
            assert_eq!(
                super::outputs::cpg_bytes(&graph),
                super::outputs::cpg_bytes(&CodePropertyGraph::build(&files))
            );
        }
        assert_eq!(
            super::outputs::cpg_bytes(&build_cpg("test.py", py, Language::Python)),
            super::outputs::cpg_bytes(&build_python_cpg(py))
        );
    });
}
#[test]
fn twelve_languages() {
    prism::build_pool::install(|| {
        let cases = [
            make_python_test(),
            make_javascript_test(),
            make_go_test(),
            make_java_test(),
            make_typescript_test(),
            make_c_test(),
            make_c_multifile_test(),
            make_cpp_test(),
            make_tsx_test(),
            make_terraform_test(),
            make_bash_test(),
        ];
        let mut seen = BTreeSet::new();
        for (files, _, _) in cases {
            for p in files.keys() {
                seen.insert(format!("{:?}", Language::from_path(p).unwrap()));
            }
        }
        seen.insert(format!("{:?}", Language::Lua));
        seen.insert(format!("{:?}", Language::Rust));
        assert_eq!(
            seen,
            BTreeSet::from_iter(
                [
                    "Python",
                    "JavaScript",
                    "TypeScript",
                    "Tsx",
                    "Go",
                    "Java",
                    "C",
                    "Cpp",
                    "Rust",
                    "Lua",
                    "Terraform",
                    "Bash"
                ]
                .map(str::to_string)
            )
        );
    });
}
