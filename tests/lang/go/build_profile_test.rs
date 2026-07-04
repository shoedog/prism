use prism::ast::ParsedFile;
use prism::go_build_profile::{
    extract_go_file_profile, go_same_package_visible, go_same_package_visible_detailed, BuildExpr,
    GoBuildProfile,
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
fn build_line_after_license_blank_is_honored() {
    let p = parsed(
        "x.go",
        "/* license */\n// copyright\n\n//go:build linux\n\npackage demo\n",
    );
    assert_eq!(p.build_expr, Some(BuildExpr::Tag("linux".into())));
}

#[test]
fn go_build_after_blank_takes_precedence_over_legacy() {
    let p = parsed(
        "x.go",
        "// +build linux\n\n//go:build windows\n\npackage demo\n",
    );
    assert_eq!(p.build_expr, Some(BuildExpr::Tag("windows".into())));
}

#[test]
fn build_line_after_package_clause_is_ignored() {
    let p = parsed("x.go", "package demo\n\n//go:build linux\n");
    assert!(p.build_expr.is_none());
}

#[test]
fn directive_detection_matches_go_syntax() {
    assert!(parsed("x.go", "// go:build windows\n\npackage demo\n")
        .build_expr
        .is_none());
    assert!(parsed("x.go", "///go:build windows\n\npackage demo\n")
        .build_expr
        .is_none());
    assert!(parsed("x.go", "//go:buildfoo\n\npackage demo\n")
        .build_expr
        .is_none());
    assert_eq!(
        parsed("x.go", "//go:build\tlinux\n\npackage demo\n").build_expr,
        Some(BuildExpr::Tag("linux".into()))
    );
    assert_eq!(
        parsed("x.go", "//   +build linux\n\npackage demo\n").build_expr,
        Some(BuildExpr::Tag("linux".into()))
    );
    assert_eq!(
        parsed("x.go", "//+build linux\n\npackage demo\n").build_expr,
        Some(BuildExpr::Tag("linux".into()))
    );
    assert!(parsed("x.go", "// +buildfoo\n\npackage demo\n")
        .build_expr
        .is_none());
}

#[test]
fn go_build_after_block_close_on_same_physical_line_is_not_directive() {
    let candidate = parsed(
        "x.go",
        "/**///go:build linux
package demo
",
    );
    assert!(candidate.build_expr.is_none());
    assert!(go_same_package_visible(
        &parsed(
            "use_windows.go",
            "package demo
"
        ),
        &candidate
    ));
}

#[test]
fn legacy_plus_build_requires_following_blank_line_but_go_build_does_not() {
    assert_eq!(
        parsed(
            "x.go",
            "//go:build linux
package demo
"
        )
        .build_expr,
        Some(BuildExpr::Tag("linux".into()))
    );
    assert!(parsed(
        "x.go",
        "// +build linux
package demo
"
    )
    .build_expr
    .is_none());
}

#[test]
fn sat_bound_fail_open_visibility_is_uncertain() {
    let caller = parsed(
        "use.go",
        "//go:build t0

package demo
",
    );
    let candidate = parsed(
        "x.go",
        "//go:build !t0 && t1 && t2 && t3 && t4 && t5 && t6 && t7 && t8

package demo
",
    );
    let vis = go_same_package_visible_detailed(&caller, &candidate);
    assert!(vis.visible);
    assert!(!vis.certain);
    assert_eq!(vis.diagnostics.unparsed, 1);
}

#[test]
fn syslist_suffixes_are_recognized() {
    assert_eq!(
        parsed("x_zos.go", "package demo\n").goos.as_deref(),
        Some("zos")
    );
    assert_eq!(
        parsed("x_amd64p32.go", "package demo\n").goarch.as_deref(),
        Some("amd64p32")
    );
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

#[test]
fn negation_only_constraints_have_fresh_values() {
    assert!(go_same_package_visible(
        &prof("p", "x.go", None),
        &prof(
            "p",
            "y.go",
            Some(BuildExpr::And(vec![
                BuildExpr::Not(Box::new(BuildExpr::Tag("windows".into()))),
                BuildExpr::Not(Box::new(BuildExpr::Tag("plan9".into()))),
                BuildExpr::Not(Box::new(BuildExpr::Tag("solaris".into()))),
            ]))
        )
    ));
    assert!(go_same_package_visible(
        &prof("p", "x.go", None),
        &prof(
            "p",
            "y.go",
            Some(BuildExpr::Not(Box::new(BuildExpr::Tag("amd64".into()))))
        )
    ));
}

#[test]
fn unix_near_exhaustion_still_has_remaining_os() {
    let excluded = [
        "linux",
        "darwin",
        "android",
        "freebsd",
        "netbsd",
        "openbsd",
        "dragonfly",
        "solaris",
        "illumos",
        "ios",
        "hurd",
    ];
    let mut terms = vec![BuildExpr::Tag("unix".into())];
    terms.extend(
        excluded
            .iter()
            .map(|tag| BuildExpr::Not(Box::new(BuildExpr::Tag((*tag).into())))),
    );
    assert!(go_same_package_visible(
        &prof("p", "x.go", None),
        &prof("p", "y.go", Some(BuildExpr::And(terms)))
    ));
}
