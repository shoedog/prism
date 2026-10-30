use super::*;

fn shapes(key: &str, body: &str) -> Vec<(String, bool)> {
    let mut result = Vec::new();
    for (prefix, parameters) in [("", ""), ("get ", ""), ("set ", "value")] {
        let method = format!("{prefix}[{key}]({parameters}) {{ {body} }}");
        result.push((format!("class C {{ {method} }}"), true));
        result.push((format!("const o = {{ {method} }};"), false));
    }
    result
}

#[test]
fn computed_method_keys_use_surrounding_scope() {
    for ext in ["js", "ts", "tsx"] {
        for (shape, _) in shapes("k(() => {})", "") {
            let src = format!("function outer() {{ {shape} }}");
            let m = checked(&src, ext);
            assert_eq!(m.bound_as, None, "{ext}: {src}");
            assert_eq!(m.enclosing.as_ref().unwrap().name, "outer", "{ext}: {src}");
            assert_eq!(m.enclosing_scope, None);
            let col = src.find("() =>").unwrap() + 1;
            assert_eq!(m.label, format!("outer › k callback (1:{col})"));
            assert_eq!(
                m.anchor.root,
                AnchorRoot::Named {
                    function: m.enclosing.clone().unwrap()
                }
            );
            assert_eq!(m.anchor.ordinals, [0]);
        }
    }
}

#[test]
fn computed_keys_keep_surrounding_context_and_nearest_callable() {
    for ext in ["js", "ts", "tsx"] {
        for (shape, _) in shapes("k(() => sink(() => {}))", "") {
            for (src, root, scope) in [
                (shape.clone(), "module", "module"),
                (
                    format!("class Holder {{ field = wrap(() => {{ {shape} }}); }}"),
                    "Holder.field › wrap callback",
                    "class Holder field field",
                ),
            ] {
                let parsed = ParsedFile::parse(
                    &format!("a.{ext}"),
                    &src,
                    Language::from_path(&format!("a.{ext}")).unwrap(),
                )
                .unwrap();
                assert!(!parsed.tree.root_node().has_error(), "{ext}: {src}");
                let r = rows(&src, ext);
                let offset = usize::from(root != "module");
                let key = &r[offset].1;
                let inner = &r[offset + 1].1;
                assert_eq!(key.bound_as, None);
                assert_eq!(inner.bound_as, None);
                assert_eq!(inner.enclosing.as_ref(), Some(&r[offset].0));
                assert_eq!(
                    key.enclosing_scope.as_deref(),
                    (offset == 0).then_some(scope)
                );
                if offset == 1 {
                    assert_eq!(key.enclosing.as_ref(), Some(&r[0].0));
                }
                let col = src.find("() => sink").unwrap() + 1;
                assert_eq!(key.label, format!("{root} › k callback (1:{col})"));
                let col = src.find("() => {}").unwrap() + 1;
                assert_eq!(
                    inner.label,
                    format!("{root} › k callback › sink callback (1:{col})")
                );
                assert_eq!(key.anchor.ordinals, vec![0; offset + 1]);
                assert_eq!(inner.anchor.ordinals, vec![0; offset + 2]);
            }
        }
    }
}

#[test]
fn computed_method_bodies_and_parameter_defaults_keep_owner() {
    for ext in ["js", "ts", "tsx"] {
        for (shape, class) in shapes("key", "sink(() => {});") {
            let src = format!("function outer() {{ {shape} }}");
            let m = checked(&src, ext);
            assert_eq!(m.bound_as, None);
            assert_eq!(m.enclosing.as_ref().unwrap().name, "[key]");
            let root = if class { "C.[key]" } else { "[key]" };
            let col = src.find("() =>").unwrap() + 1;
            assert_eq!(m.label, format!("{root} › sink callback (1:{col})"));
        }
        for (shape, class) in shapes("key", "").into_iter().take(2) {
            let src = shape.replace("[key]()", "[key](cb = () => {})");
            let m = checked(&src, ext);
            assert_eq!(
                m.bound_as,
                Some(BoundAs::DefaultParam {
                    function: m.enclosing.clone().unwrap(),
                    param: "cb".into()
                })
            );
            assert_eq!(m.enclosing.as_ref().unwrap().name, "[key]");
            let root = if class { "C.[key]" } else { "[key]" };
            let col = src.find("() =>").unwrap() + 1;
            assert_eq!(m.label, format!("{root} › default cb (1:{col})"));
        }
    }
}
