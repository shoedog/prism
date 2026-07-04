use prism::ast::ParsedFile;
use prism::go_build_profile::{
    extract_go_file_profile, go_same_package_visible, BuildExpr, GoBuildProfile,
};
use prism::languages::Language;

fn parsed(path: &str, src: &str) -> GoBuildProfile {
    let pf = ParsedFile::parse(path, src, Language::Go).unwrap();
    extract_go_file_profile(path, &pf).0
}

fn prof(pkg: &str, file: &str, expr: Option<BuildExpr>) -> GoBuildProfile {
    let pf = ParsedFile::parse(file, &format!("package {pkg}\n"), Language::Go).unwrap();
    let (mut profile, _) = extract_go_file_profile(file, &pf);
    profile.build_expr = expr;
    profile
}

#[test]
fn filename_suffix_rules_strip_test_and_require_prefix() {
    let p = parsed("x_linux_amd64_test.go", "package demo\n");
    assert_eq!(p.goos.as_deref(), Some("linux"));
    assert_eq!(p.goarch.as_deref(), Some("amd64"));
    let bare = parsed("linux.go", "package demo\n");
    assert_eq!(bare.goos, None);
    assert_eq!(bare.goarch, None);
    let unix = parsed("x_unix.go", "package demo\n");
    assert_eq!(unix.goos, None);
}

#[test]
fn extracts_package_clause_and_go_build_precedence() {
    let p = parsed(
        "x_linux.go",
        "//go:build linux && cgo\n// +build windows\n\npackage demo\n",
    );
    assert_eq!(p.package_clause, "demo");
    assert_eq!(p.goos.as_deref(), Some("linux"));
    assert!(matches!(p.build_expr, Some(BuildExpr::And(_))));
}

#[test]
fn multiple_go_build_lines_are_unparsed() {
    let pf = ParsedFile::parse(
        "x.go",
        "//go:build linux\n//go:build windows\n\npackage demo\n",
        Language::Go,
    )
    .unwrap();
    let (p, n) = extract_go_file_profile("x.go", &pf);
    assert!(p.build_expr.is_none());
    assert_eq!(n, 1);
}

#[test]
fn build_line_after_header_blank_is_ignored() {
    let p = parsed("x.go", "// license\n\n//go:build linux\npackage demo\n");
    assert!(p.build_expr.is_none());
}

#[test]
fn sat_alias_and_custom_tag_cases() {
    assert!(go_same_package_visible(
        &prof("p", "x_linux.go", None),
        &prof("p", "x_android.go", None)
    ));
    assert!(go_same_package_visible(
        &prof("p", "x_android.go", None),
        &prof("p", "x.go", Some(BuildExpr::Tag("linux".into())))
    ));
    assert!(go_same_package_visible(
        &prof("p", "x_illumos.go", None),
        &prof("p", "x_solaris.go", None)
    ));
    assert!(go_same_package_visible(
        &prof("p", "x_ios.go", None),
        &prof("p", "x_darwin.go", None)
    ));
    assert!(go_same_package_visible(
        &prof("p", "x_android.go", None),
        &prof("p", "x.go", Some(BuildExpr::Tag("unix".into())))
    ));
    assert!(!go_same_package_visible(
        &prof("p", "x_linux.go", None),
        &prof("p", "x_windows.go", None)
    ));
    assert!(!go_same_package_visible(
        &prof("p", "x.go", Some(BuildExpr::Tag("X".into()))),
        &prof(
            "p",
            "y.go",
            Some(BuildExpr::Not(Box::new(BuildExpr::Tag("X".into()))))
        )
    ));
}
