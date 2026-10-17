use super::*;

mod computed_keys;

fn checked(src: &str, ext: &str) -> AnonMeta {
    let file = format!("a.{ext}");
    let parsed = ParsedFile::parse(&file, src, Language::from_path(&file).unwrap()).unwrap();
    assert!(
        !parsed.tree.root_node().has_error(),
        "{}",
        parsed.tree.root_node().to_sexp()
    );
    one(src, ext)
}

#[test]
fn parameter_initializer_assignment_is_not_binding() {
    for ext in ["js", "ts", "tsx"] {
        let m = checked("function f(x = ({cb = () => {}} = obj)) {}", ext);
        assert_eq!(m.bound_as, None, "{ext}");
        assert_eq!(m.label, "f › anonymous callable (1:23)");
    }
}

#[test]
fn parameter_initializer_class_is_not_binding() {
    for ext in ["js", "ts", "tsx"] {
        let m = checked(
            "function f(x = class { static { const {cb = () => {}} = obj; } }) {}",
            ext,
        );
        assert_eq!(m.bound_as, None, "{ext}");
        assert_eq!(m.label, "f › anonymous callable (1:45)");
    }
}

#[test]
fn binding_pattern_edges_keep_defaults() {
    for ext in ["js", "ts", "tsx"] {
        for (src, col) in [
            ("function f({outer: {cb = () => {}}}) {}", 26),
            ("function f({outer: {x: cb = () => {}}}) {}", 29),
            ("function f([{outer: {cb = (() => {})}}]) {}", 28),
            ("function f({cb = function* () {}}) {}", 18),
            ("function f({cb = async () => {}} = {}) {}", 18),
            ("function f([cb = () => {}]) {}", 18),
            ("function f(...[cb = () => {}]) {}", 21),
        ] {
            let m = checked(src, ext);
            assert_eq!(
                m.bound_as,
                Some(BoundAs::DefaultParam {
                    function: m.enclosing.clone().unwrap(),
                    param: "cb".into(),
                }),
                "{ext}: {src}"
            );
            assert_eq!(m.label, format!("f › default cb (1:{col})"));
        }
    }
}

#[test]
fn label_param_is_display_text() {
    for ext in ["js", "ts", "tsx"] {
        let param = "{\n  cb\n}";
        let m = checked(&format!("function f({param} = () => {{}}) {{}}"), ext);
        assert_eq!(
            m.bound_as,
            Some(BoundAs::DefaultParam {
                function: m.enclosing.clone().unwrap(),
                param: param.into()
            })
        );
        assert_eq!(m.label, "f › default { cb } (3:5)");
    }
}

#[test]
fn label_object_key_is_display_text() {
    for ext in ["js", "ts", "tsx"] {
        let key = "[\n  key\n]";
        let m = checked(&format!("({{ {key}: (() => {{}}) }})"), ext);
        assert_eq!(m.bound_as, Some(BoundAs::ObjectKey { key: key.into() }));
        assert_eq!(m.label, "module › object [ key ] (3:5)");
    }
}

#[test]
fn label_field_and_root_are_display_text() {
    for ext in ["js", "ts", "tsx"] {
        let field = format!("[{}]", "x".repeat(80));
        let m = checked(&format!("class C {{ {field} = (() => {{}}); }}"), ext);
        assert_eq!(
            m.bound_as,
            Some(BoundAs::ClassField {
                class: "C".into(),
                field: field.clone()
            })
        );
        let short = format!("[{}…", "x".repeat(46));
        assert_eq!(m.label, format!("C.{short} › field {short} (1:97)"));
        assert_eq!(m.enclosing_scope, Some(format!("class C field {field}")));
    }
}

#[test]
fn initializer_and_computed_binding_edges_are_barriers() {
    for ext in ["js", "ts", "tsx"] {
        for src in [
            "function f({x = ({cb = () => {}} = obj)}) {}",
            "function f([x = ({cb = () => {}} = obj)]) {}",
            "function f({[({cb = () => {}} = obj)]: x}) {}",
        ] {
            let m = checked(src, ext);
            assert_eq!(m.bound_as, None, "{ext}: {src}");
            assert_eq!(
                m.label,
                format!(
                    "f › anonymous callable (1:{})",
                    src.find("() =>").unwrap() + 1
                )
            );
        }
    }
}

#[test]
fn ts_parameter_wrappers_keep_binding_edges() {
    for ext in ["ts", "tsx"] {
        for src in [
            "function f({cb = () => {}}: T) {}",
            "function f({cb = () => {}}?: T) {}",
        ] {
            let m = checked(src, ext);
            assert_eq!(
                m.bound_as,
                Some(BoundAs::DefaultParam {
                    function: m.enclosing.clone().unwrap(),
                    param: "cb".into(),
                })
            );
            assert_eq!(m.label, "f › default cb (1:18)");
        }
    }
}

#[test]
fn label_named_roots_are_display_text() {
    for ext in ["js", "ts", "tsx"] {
        let class = "C".repeat(60);
        let src = format!("class {class} {{ [\n key\n]() {{ return () => {{}}; }} }}");
        let m = checked(&src, ext);
        assert_eq!(m.bound_as, Some(BoundAs::Returned));
        assert_eq!(m.enclosing.as_ref().unwrap().name, "[\n key\n]");
        assert_eq!(
            m.label,
            format!("{}….[ key ] › returned callable (3:14)", "C".repeat(47))
        );
        assert_eq!(
            m.anchor.root,
            AnchorRoot::Named {
                function: m.enclosing.clone().unwrap()
            }
        );
        let name = "f".repeat(60);
        let m = checked(&format!("function {name}() {{ return () => {{}}; }}"), ext);
        assert_eq!(m.bound_as, Some(BoundAs::Returned));
        assert_eq!(m.enclosing.as_ref().unwrap().name, name);
        assert_eq!(
            m.label,
            format!("{}… › returned callable (1:82)", "f".repeat(47))
        );
    }
}

#[test]
fn label_variable_is_display_text() {
    for ext in ["js", "ts", "tsx"] {
        let name = "{\n cb\n}";
        let m = checked(&format!("const {name} = (() => {{}});"), ext);
        assert_eq!(
            m.bound_as,
            Some(BoundAs::VariableParenthesized { name: name.into() })
        );
        assert_eq!(m.label, "module › variable { cb } (3:6)");
    }
}

#[test]
fn label_jsx_prop_is_display_text() {
    for ext in ["js", "tsx"] {
        let prop = "p".repeat(60);
        let element = "B".repeat(60);
        let m = checked(&format!("<{element} {prop}={{() => {{}}}} />"), ext);
        assert_eq!(m.bound_as, Some(BoundAs::JsxProp { element, prop }));
        assert_eq!(
            m.label,
            format!("module › {}… handler (1:125)", "p".repeat(47))
        );
    }
}
