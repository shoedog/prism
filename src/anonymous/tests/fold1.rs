use super::*;

#[test]
fn shorthand_default() {
    for ext in ["js", "ts", "tsx"] {
        let m = one("function f({cb = () => {}}) {}", ext);
        assert_eq!(
            m.bound_as,
            Some(BoundAs::DefaultParam {
                function: m.enclosing.clone().unwrap(),
                param: "cb".into(),
            })
        );
        assert_eq!(m.enclosing.as_ref().unwrap().name, "f");
        assert_eq!(m.label, "f › default cb (1:18)");
        let local = one("function f() { const {cb = () => {}} = o; }", ext);
        assert!(local.bound_as.is_none());
        assert!(local.label.starts_with("f › anonymous callable ("));
        let react = one("const Comp = ({ onClick = () => {} }) => null;", ext);
        assert_eq!(
            react.bound_as,
            Some(BoundAs::DefaultParam {
                function: react.enclosing.clone().unwrap(),
                param: "onClick".into(),
            })
        );
        assert_eq!(react.enclosing.as_ref().unwrap().name, "Comp");
        assert_eq!(react.label, "Comp › default onClick (1:27)");
    }
}

#[test]
fn computed_key_barrier() {
    for ext in ["js", "ts", "tsx"] {
        let r = rows(
            "function outer() { class C { [key(() => {})] = sink(() => {}); } }",
            ext,
        );
        assert_eq!(r.len(), 2);
        let key = &r[0].1;
        let init = &r[1].1;
        assert_eq!(key.enclosing.as_ref().unwrap().name, "outer");
        assert_eq!(key.enclosing_scope, None);
        assert_eq!(key.label, "outer › key callback (1:35)");
        assert_eq!(
            key.anchor.root,
            AnchorRoot::Named {
                function: key.enclosing.clone().unwrap()
            }
        );
        assert_eq!(key.anchor.ordinals, [0]);
        assert_eq!(init.enclosing, None);
        let scope = "class C field [key(() => {})]";
        assert_eq!(init.enclosing_scope.as_deref(), Some(scope));
        assert_eq!(init.label, "C.[key(() => {})] › sink callback (1:53)");
        assert_eq!(
            init.anchor.root,
            AnchorRoot::Scope {
                file: format!("a.{ext}"),
                scope: scope.into()
            }
        );
        assert_eq!(init.anchor.ordinals, [0]);
    }
}

#[test]
fn abstract_field() {
    for ext in ["ts", "tsx"] {
        let m = one("abstract class C { handle = debounce(() => hit()) }", ext);
        assert_eq!(
            m.bound_as,
            Some(BoundAs::ClassField {
                class: "C".into(),
                field: "handle".into()
            })
        );
        assert_eq!(m.label, "C.handle › debounce callback (1:38)");
        assert_eq!(m.enclosing_scope.as_deref(), Some("class C field handle"));
    }
}

#[test]
fn abstract_method_label() {
    for ext in ["ts", "tsx"] {
        let m = one("abstract class C { m() { sink(() => {}); } }", ext);
        assert_eq!(m.label, "C.m › sink callback (1:31)");
        assert_eq!(m.enclosing.as_ref().unwrap().name, "m");
        assert_eq!(
            m.anchor.root,
            AnchorRoot::Named {
                function: m.enclosing.clone().unwrap()
            }
        );
        let object = one("const o = { m() { sink(() => {}); } };", ext);
        assert!(object.label.starts_with("m › sink callback ("));
    }
}

#[test]
fn callee_display_whitespace() {
    for ext in ["js", "ts", "tsx"] {
        let raw = "(function factory() {\n  return invoke;\n})()";
        let r = rows(&format!("{raw}(() => {{}})"), ext);
        let m = &r.last().unwrap().1;
        assert_eq!(m.passed_to.as_ref().unwrap().callee_text, raw);
        assert_eq!(m.anchor.callee_text.as_deref(), Some(raw));
        assert_eq!(
            m.label,
            "module › (function factory() { return invoke; })() callback (3:6)"
        );
    }
}

#[test]
fn callee_display_bound() {
    for ext in ["js", "ts", "tsx"] {
        let raw = format!("(firstHandler || {})", "x".repeat(80));
        let m = one(&format!("{raw}(() => {{}})"), ext);
        assert_eq!(m.passed_to.as_ref().unwrap().callee_text, raw);
        let short = format!("{}…", raw.chars().take(47).collect::<String>());
        assert_eq!(m.label, format!("module › {short} callback (1:100)"));
        let m = one("(a || b)(() => {})", ext);
        assert_eq!(m.label, "module › (a || b) callback (1:10)");
        let raw = format!("(a || {})", "é".repeat(60));
        let m = one(&format!("{raw}(() => {{}})"), ext);
        let short = format!("{}…", raw.chars().take(47).collect::<String>());
        assert!(m.label.starts_with(&format!("module › {short} callback (")));
        assert_eq!(m.passed_to.unwrap().callee_text, raw);
    }
}

#[test]
fn builder_file_key() {
    for ext in ["js", "ts", "tsx"] {
        let path = format!("parsed.{ext}");
        let file = format!("builder/{ext}");
        let src =
            "sink(() => {}); function outer() { sink(() => {}); } class C { x = sink(() => {}); }";
        let parsed = ParsedFile::parse(&path, src, Language::from_path(&path).unwrap()).unwrap();
        let result = extract(&parsed, &file);
        assert_eq!(result.len(), 3);
        assert!(result.keys().all(|id| id.file == file));
        for m in result.values() {
            match &m.anchor.root {
                AnchorRoot::Named { function } => assert_eq!(function.file, file),
                AnchorRoot::Scope {
                    file: root_file, ..
                } => assert_eq!(root_file, &file),
            }
            if let Some(parent) = &m.enclosing {
                assert_eq!(parent.file, file);
            }
        }
        assert!(extract(&parsed, &path).keys().all(|id| id.file == path));
    }
}
