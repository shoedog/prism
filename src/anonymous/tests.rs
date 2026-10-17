use super::*;
use crate::{ast::ParsedFile, languages::Language};

fn rows(src: &str, ext: &str) -> Vec<(FunctionId, AnonMeta)> {
    let file = format!("a.{ext}");
    let parsed = ParsedFile::parse(&file, src, Language::from_path(&file).unwrap()).unwrap();
    let mut rows: Vec<_> = extract(&parsed).into_iter().collect();
    rows.sort_by_key(|(id, m)| (id.start_line, m.col));
    rows
}

fn one(src: &str, ext: &str) -> AnonMeta {
    let mut r = rows(src, ext);
    assert_eq!(r.len(), 1, "{src}");
    r.remove(0).1
}

fn role_case(src: &str, role: BoundAs, prefix: &str) {
    for ext in ["js", "ts", "tsx"] {
        let m = one(src, ext);
        assert_eq!(m.bound_as.as_ref(), Some(&role));
        assert!(m.label.starts_with(&format!("{prefix} (")), "{}", m.label);
    }
}

#[test]
fn object_key() {
    role_case(
        "({ run: (() => hit()) })",
        BoundAs::ObjectKey { key: "run".into() },
        "module › object run",
    );
    for ext in ["js", "ts", "tsx"] {
        let m = one("({ 'run': wrap((() => {})) })", ext);
        assert_eq!(
            m.bound_as,
            Some(BoundAs::ObjectKey {
                key: "'run'".into()
            })
        );
        assert!(m.label.starts_with("module › wrap callback ("));
    }
}

#[test]
fn class_field() {
    role_case(
        "class C { handle = debounce(() => hit()) }",
        BoundAs::ClassField {
            class: "C".into(),
            field: "handle".into(),
        },
        "C.handle › debounce callback",
    );
    for ext in ["js", "ts", "tsx"] {
        let m = one("(class { ['x'] = (() => {}) })", ext);
        assert_eq!(
            m.enclosing_scope.as_deref(),
            Some("class <anonymous> field ['x']")
        );
        assert!(m.label.starts_with("<anonymous>.['x'] › field ['x'] ("));
        assert!(one("class C { x = [() => {}] }", ext).bound_as.is_none());
    }
}

#[test]
fn default_export() {
    role_case(
        "export default () => hit()",
        BoundAs::DefaultExport,
        "module › default export",
    );
}

#[test]
fn variable_parenthesized() {
    role_case(
        "const run = (() => hit())",
        BoundAs::VariableParenthesized { name: "run".into() },
        "module › variable run",
    );
}

#[test]
fn returned() {
    role_case(
        "function f() { return () => hit() }",
        BoundAs::Returned,
        "f › returned callable",
    );
}

#[test]
fn iife() {
    for callable in ["() => hit()", "function() {}", "function* () {}"] {
        role_case(
            &format!("function f() {{ ({callable})() }}"),
            BoundAs::Iife,
            "f › IIFE",
        );
    }
}

#[test]
fn default_param() {
    for ext in ["js", "ts", "tsx"] {
        let m = one("function f(cb = () => hit()) {}", ext);
        assert!(
            matches!(m.bound_as, Some(BoundAs::DefaultParam { ref function, ref param }) if function.name == "f" && param == "cb")
        );
        assert!(m.label.starts_with("f › default cb ("));
    }
    for ext in ["ts", "tsx"] {
        assert!(matches!(
            one("function f(cb?: Fn = () => {}) {}", ext).bound_as,
            Some(BoundAs::DefaultParam { .. })
        ));
    }
}

#[test]
fn jsx_prop() {
    for ext in ["js", "tsx"] {
        let m = one(
            "function Toolbar() { return <Button onClick={() => hit()} /> }",
            ext,
        );
        assert_eq!(
            m.bound_as,
            Some(BoundAs::JsxProp {
                element: "Button".into(),
                prop: "onClick".into()
            })
        );
        assert!(m.label.starts_with("Toolbar › onClick handler ("));
        assert!(rows("const el = <Button onClick={named} count={1} />", ext).is_empty());
    }
}

#[test]
fn named_and_non_callable_controls() {
    for ext in ["js", "ts", "tsx"] {
        assert!(rows("function named() {} const direct = () => hit(); const memo = useMemo(() => hit(), []); const cb = useCallback(() => hit(), []); ({run: () => hit()}); class C { handle = () => hit() }", ext).is_empty());
        assert!(rows("export default function named() {}; export { named };", ext).is_empty());
        assert!(rows("function f() { return 1 }", ext).is_empty());
        assert!(one("[() => {}]", ext)
            .label
            .starts_with("module › anonymous callable ("));
        let r = rows("sink(() => sink(() => sink(() => {})))", ext);
        assert_eq!(r[2].1.anchor.ordinals, [0, 0, 0]);
        assert_eq!(r[2].1.enclosing.as_ref(), Some(&r[1].0));
        assert!(r.iter().all(|(_, m)| !m.r#async));
        assert!(one("class C { method() { sink(() => {}) } }", ext)
            .label
            .starts_with("C.method › sink callback ("));
    }
    assert!(rows("def f():\n return lambda x: x", "py").is_empty());
}

macro_rules! label_case {
    ($name:ident, $src:expr, $expected:expr, $exts:expr) => {
        #[test]
        fn $name() {
            for ext in $exts {
                assert_eq!(one($src, ext).label, $expected);
            }
        }
    };
}
label_case!(
    editor_label,
    "function Editor() {\n\n\nuseEffect(() => hit()); }",
    "Editor › useEffect callback (4:11)",
    ["js", "ts", "tsx"]
);
label_case!(
    toolbar_label,
    "function Toolbar() {\n\n\n\n\n\n\n\n\n\n\nreturn <B onClick={() => hit()} /> }",
    "Toolbar › onClick handler (12:20)",
    ["js", "tsx"]
);
label_case!(
    map_label,
    "function A() {\n\n        list.map(() => hit()); }",
    "A › .map callback (3:18)",
    ["js", "ts", "tsx"]
);
label_case!(
    module_label,
    "  forEach( () => hit());",
    "module › forEach callback (1:12)",
    ["js", "ts", "tsx"]
);
label_case!(
    field_label,
    "class C {\n\n  handle = debounce(() => hit()); }",
    "C.handle › debounce callback (3:21)",
    ["js", "ts", "tsx"]
);

#[test]
fn nested_label_and_parent_chain() {
    for ext in ["js", "ts", "tsx"] {
        let r = rows("function A() {\nlist.map(() => {                             xs.map(() => hit()); }); }", ext);
        assert_eq!(r[1].1.label, "A › .map callback › .map callback (2:53)");
        assert_eq!(r[1].1.enclosing.as_ref(), Some(&r[0].0));
        assert_eq!(r[1].1.anchor.ordinals, [0, 0]);
    }
}

#[test]
fn wrappers_and_value_changing_barriers() {
    for ext in ["js", "ts", "tsx"] {
        let m = one("({ ['run']: (((() => hit()))) })", ext);
        assert_eq!(
            m.bound_as,
            Some(BoundAs::ObjectKey {
                key: "['run']".into()
            })
        );
        assert!(one("const run = [() => hit()]", ext).bound_as.is_none());
        let r = rows("({ run: (() => { sink(() => hit()) }) })", ext);
        assert!(r[1].1.bound_as.is_none());
        assert_eq!(r[1].1.passed_to.as_ref().unwrap().callee_text, "sink");
        assert!(one("sink([() => hit()])", ext).passed_to.is_none());
    }
    for ext in ["ts", "tsx"] {
        let m = one("sink((((() => hit()) as Fn) satisfies Fn)!)", ext);
        assert_eq!(m.passed_to.as_ref().unwrap().arg_index, 0);
        let m = one("function f({cb} = () => hit()) {}", ext);
        assert!(
            matches!(m.bound_as, Some(BoundAs::DefaultParam { ref param, .. }) if param == "{cb}")
        );
    }
    assert_eq!(
        one("sink(<Fn>(() => hit()))", "ts")
            .passed_to
            .unwrap()
            .callee_text,
        "sink"
    );
}

#[test]
fn argument_index_and_call_spans() {
    for ext in ["js", "ts", "tsx"] {
        let src = "function f() { obj.map(/* first */ ...xs, /* second */ (() => hit())); new Promise(function() {}); }";
        let r = rows(src, ext);
        let p = r[0].1.passed_to.as_ref().unwrap();
        assert_eq!(
            (p.callee_text.as_str(), p.arg_index, p.call_line),
            ("obj.map", 1, 1)
        );
        assert_eq!(
            &src[p.call_span.0..p.call_span.1],
            "obj.map(/* first */ ...xs, /* second */ (() => hit()))"
        );
        assert!(p.callee.is_none());
        let p = r[1].1.passed_to.as_ref().unwrap();
        assert_eq!((p.callee_text.as_str(), p.arg_index), ("Promise", 0));
        assert_eq!(
            &src[p.call_span.0..p.call_span.1],
            "new Promise(function() {})"
        );
        assert!(p.callee.is_none());
        assert!(one("get('a.b')(() => {})", ext)
            .label
            .starts_with("module › get('a.b') callback ("));
    }
}

#[test]
fn roots_barriers_ordinals_and_early_errors() {
    for ext in ["js", "ts", "tsx"] {
        let r = rows("sink(() => {}, () => {}); function outer() { class C { handle = debounce(() => { sink(() => {}); }); } sink(() => { function inner() { sink(() => {}); } sink(() => {}); }); }", ext);
        assert_eq!(r.len(), 7);
        assert_ne!(r[0].0, r[1].0);
        assert_eq!(r[0].1.anchor.ordinals, [0]);
        assert_eq!(r[1].1.anchor.ordinals, [1]);
        assert_eq!(r[0].1.enclosing_scope.as_deref(), Some("module"));
        assert_eq!(
            r[2].1.enclosing_scope.as_deref(),
            Some("class C field handle")
        );
        assert!(r[2].1.enclosing.is_none());
        assert_eq!(r[2].1.anchor.ordinals, [0]);
        assert_eq!(r[3].1.enclosing.as_ref(), Some(&r[2].0));
        assert_eq!(r[4].1.anchor.ordinals, [0]);
        assert_eq!(r[5].1.anchor.ordinals, [0]);
        assert!(
            matches!(&r[5].1.anchor.root, AnchorRoot::Named { function } if function.name == "inner")
        );
        assert_eq!(r[6].1.anchor.ordinals, [0, 0]);
        let r = rows(
            "/* é */ sink(async (x,x) => {}, async function() {}, async function* () {});",
            ext,
        );
        assert_eq!(r.len(), 3);
        assert_eq!(r[0].1.kind, AnonKind::Arrow);
        assert_eq!(r[0].1.col, 15);
        assert_eq!(r[1].1.kind, AnonKind::Function);
        assert_eq!(r[2].1.kind, AnonKind::Generator);
        assert!(r
            .iter()
            .all(|(_, m)| m.anonymous && m.r#async && m.anchor.version == 1));
        assert!(r
            .iter()
            .all(|(_, m)| m.anchor.callee_text.as_deref() == Some("sink")));
        assert_eq!(
            serde_json::from_str::<AnonMeta>(&serde_json::to_string(&r[0].1).unwrap()).unwrap(),
            r[0].1
        );
    }
}

#[test]
fn label_precedence_and_named_root_restart() {
    for ext in ["js", "ts", "tsx"] {
        let m = one("function f(cb = wrap((() => {}))) {}", ext);
        assert!(m.label.starts_with("f › wrap callback ("));
        assert!(one("export default (() => {})", ext)
            .label
            .starts_with("module › default export ("));
        let r = rows(
            "sink(() => { function named() { return () => () => {}; } });",
            ext,
        );
        assert!(r[1].1.label.starts_with("named › returned callable ("));
        assert!(r[2]
            .1
            .label
            .starts_with("named › returned callable › returned callable ("));
        assert_eq!(r[2].1.label.matches('(').count(), 1);
        assert!(one("const f = (() => {})", ext).passed_to.is_none());
    }
    for ext in ["js", "tsx"] {
        let m = one("function T() { return <B onClick={(() => {})()} /> }", ext);
        assert_eq!(m.bound_as, Some(BoundAs::Iife));
        let r = rows(
            "function T() { return <B onClick={wrap(() => {})} /> }",
            ext,
        );
        // A handoff inside a JSX value is not itself the prop value.
        assert!(r[0].1.bound_as.is_none());
        assert!(r[0].1.label.starts_with("T › wrap callback ("));
    }
}

#[test]
fn destructuring_defaults_are_parameters_only_in_parameter_scope() {
    for ext in ["js", "ts", "tsx"] {
        let local = one("function f() { const [cb = () => {}] = values; }", ext);
        assert!(local.bound_as.is_none(), "{ext}: {local:?}");
        let parameter = one("function f([cb = () => {}]) {}", ext);
        assert!(
            matches!(parameter.bound_as, Some(BoundAs::DefaultParam { ref param, .. }) if param == "cb")
        );
    }
}
