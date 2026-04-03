#[path = "../../common/mod.rs"]
mod common;
use common::*;
use prism::react_hooks::{detect_hooks, HookType};

// ====== React Hook Detection Tests ======

#[test]
fn test_detect_usestate() {
    let source = r#"
function Counter() {
    const [count, setCount] = useState(0);
    return <div>{count}</div>;
}
"#;
    let path = "Counter.tsx";
    let parsed = ParsedFile::parse(path, source, Language::Tsx).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let hooks = detect_hooks(&files);
    let file_hooks = hooks.get(path).expect("should detect hooks in file");

    assert_eq!(file_hooks.len(), 1);
    assert_eq!(file_hooks[0].hook_type, HookType::UseState);
    assert_eq!(file_hooks[0].function, "Counter");
    assert!(file_hooks[0].callback.is_none());
    assert!(file_hooks[0].deps.is_none());
}

#[test]
fn test_detect_useeffect_with_deps() {
    let source = r#"
function UserProfile({ userId }) {
    const [data, setData] = useState(null);

    useEffect(() => {
        fetchData(userId).then(setData);
    }, [userId]);

    return <div>{data}</div>;
}
"#;
    let path = "UserProfile.tsx";
    let parsed = ParsedFile::parse(path, source, Language::Tsx).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let hooks = detect_hooks(&files);
    let file_hooks = hooks.get(path).expect("should detect hooks in file");

    // useState + useEffect
    assert_eq!(file_hooks.len(), 2);

    let effect = file_hooks
        .iter()
        .find(|h| h.hook_type == HookType::UseEffect)
        .expect("should find useEffect");

    assert_eq!(effect.function, "UserProfile");

    // Callback info
    let callback = effect
        .callback
        .as_ref()
        .expect("useEffect should have callback");
    assert!(callback.start_line > 0);
    assert!(callback.end_line >= callback.start_line);
    assert!(
        !callback.all_identifiers.is_empty(),
        "callback should contain identifiers"
    );

    // Deps info
    let deps = effect.deps.as_ref().expect("useEffect should have deps");
    assert!(!deps.is_missing, "deps should not be missing");
    assert!(!deps.is_empty, "deps should not be empty");
    assert!(
        deps.identifiers.iter().any(|(name, _)| name == "userId"),
        "deps should contain userId, got: {:?}",
        deps.identifiers
    );
}

#[test]
fn test_detect_useeffect_empty_deps() {
    let source = r#"
function App() {
    useEffect(() => {
        initialize();
    }, []);

    return <div />;
}
"#;
    let path = "App.tsx";
    let parsed = ParsedFile::parse(path, source, Language::Tsx).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let hooks = detect_hooks(&files);
    let file_hooks = hooks.get(path).expect("should detect hooks in file");

    let effect = &file_hooks[0];
    assert_eq!(effect.hook_type, HookType::UseEffect);

    let deps = effect.deps.as_ref().expect("should have deps");
    assert!(deps.is_empty, "empty deps array should be detected");
    assert!(!deps.is_missing);
}

#[test]
fn test_detect_useeffect_missing_deps() {
    let source = r#"
function Timer() {
    useEffect(() => {
        const interval = setInterval(tick, 1000);
        return () => clearInterval(interval);
    });

    return <div />;
}
"#;
    let path = "Timer.tsx";
    let parsed = ParsedFile::parse(path, source, Language::Tsx).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let hooks = detect_hooks(&files);
    let file_hooks = hooks.get(path).expect("should detect hooks in file");

    let effect = &file_hooks[0];
    let deps = effect.deps.as_ref().expect("should have deps info");
    assert!(deps.is_missing, "missing deps should be detected");
}

#[test]
fn test_detect_usememo_and_usecallback() {
    let source = r#"
function SearchResults({ query, items }) {
    const filtered = useMemo(() => {
        return items.filter(i => i.name.includes(query));
    }, [items, query]);

    const handleSelect = useCallback((item) => {
        selectItem(item);
    }, []);

    return <List items={filtered} onSelect={handleSelect} />;
}
"#;
    let path = "Search.tsx";
    let parsed = ParsedFile::parse(path, source, Language::Tsx).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let hooks = detect_hooks(&files);
    let file_hooks = hooks.get(path).expect("should detect hooks");

    let memo = file_hooks
        .iter()
        .find(|h| h.hook_type == HookType::UseMemo)
        .expect("should find useMemo");
    assert!(memo.callback.is_some());
    let memo_deps = memo.deps.as_ref().unwrap();
    assert!(!memo_deps.is_empty);

    let cb = file_hooks
        .iter()
        .find(|h| h.hook_type == HookType::UseCallback)
        .expect("should find useCallback");
    assert!(cb.callback.is_some());
    let cb_deps = cb.deps.as_ref().unwrap();
    assert!(
        cb_deps.is_empty,
        "useCallback with [] should have empty deps"
    );
}

#[test]
fn test_detect_useref_usecontext() {
    let source = r#"
function Form() {
    const inputRef = useRef(null);
    const theme = useContext(ThemeContext);
    return <input ref={inputRef} style={theme} />;
}
"#;
    let path = "Form.tsx";
    let parsed = ParsedFile::parse(path, source, Language::Tsx).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let hooks = detect_hooks(&files);
    let file_hooks = hooks.get(path).expect("should detect hooks");

    assert!(file_hooks.iter().any(|h| h.hook_type == HookType::UseRef));
    assert!(file_hooks
        .iter()
        .any(|h| h.hook_type == HookType::UseContext));
}

#[test]
fn test_detect_custom_hooks() {
    let source = r#"
function Dashboard() {
    const data = useFetchData("/api/dashboard");
    const [theme, toggleTheme] = useToggle(false);
    return <div>{data}</div>;
}
"#;
    let path = "Dashboard.tsx";
    let parsed = ParsedFile::parse(path, source, Language::Tsx).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let hooks = detect_hooks(&files);
    let file_hooks = hooks.get(path).expect("should detect hooks");

    assert_eq!(file_hooks.len(), 2);
    assert!(
        file_hooks.iter().any(|h| matches!(&h.hook_type,
            HookType::Custom(name) if name == "useFetchData")),
        "should detect useFetchData custom hook"
    );
    assert!(
        file_hooks
            .iter()
            .any(|h| matches!(&h.hook_type, HookType::Custom(name) if name == "useToggle")),
        "should detect useToggle custom hook"
    );
}

#[test]
fn test_hooks_in_arrow_component() {
    // With arrow function naming fix, arrow components should also have hooks detected
    let source = r#"
const Counter = () => {
    const [count, setCount] = useState(0);
    useEffect(() => {
        document.title = `Count: ${count}`;
    }, [count]);
    return <button onClick={() => setCount(count + 1)}>{count}</button>;
};
"#;
    let path = "Counter.tsx";
    let parsed = ParsedFile::parse(path, source, Language::Tsx).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let hooks = detect_hooks(&files);
    let file_hooks = hooks
        .get(path)
        .expect("should detect hooks in arrow component");

    assert_eq!(file_hooks.len(), 2);
    assert!(file_hooks.iter().all(|h| h.function == "Counter"));
}

#[test]
fn test_hooks_not_detected_in_non_js_files() {
    let source = r#"
def use_state():
    return None

def component():
    state = use_state()
    return state
"#;
    let path = "component.py";
    let parsed = ParsedFile::parse(path, source, Language::Python).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let hooks = detect_hooks(&files);
    assert!(hooks.is_empty(), "should not detect hooks in Python files");
}

#[test]
fn test_multiple_hooks_in_component() {
    let source = r#"
function ComplexComponent({ id, config }) {
    const [data, setData] = useState(null);
    const [loading, setLoading] = useState(true);
    const [error, setError] = useState(null);
    const prevId = useRef(id);
    const theme = useContext(ThemeContext);
    const componentId = useId();

    useEffect(() => {
        setLoading(true);
        fetchData(id, config).then(setData).catch(setError);
    }, [id, config]);

    useLayoutEffect(() => {
        prevId.current = id;
    });

    const processedData = useMemo(() => {
        return data ? transform(data) : null;
    }, [data]);

    return <div>{processedData}</div>;
}
"#;
    let path = "Complex.tsx";
    let parsed = ParsedFile::parse(path, source, Language::Tsx).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let hooks = detect_hooks(&files);
    let file_hooks = hooks.get(path).expect("should detect hooks");

    // Count hook types
    let state_count = file_hooks
        .iter()
        .filter(|h| h.hook_type == HookType::UseState)
        .count();
    let effect_count = file_hooks
        .iter()
        .filter(|h| h.hook_type == HookType::UseEffect)
        .count();

    assert_eq!(state_count, 3, "should detect 3 useState calls");
    assert_eq!(effect_count, 1, "should detect 1 useEffect call");
    assert!(file_hooks.iter().any(|h| h.hook_type == HookType::UseRef));
    assert!(file_hooks
        .iter()
        .any(|h| h.hook_type == HookType::UseContext));
    assert!(file_hooks.iter().any(|h| h.hook_type == HookType::UseId));
    assert!(file_hooks
        .iter()
        .any(|h| h.hook_type == HookType::UseLayoutEffect));
    assert!(file_hooks.iter().any(|h| h.hook_type == HookType::UseMemo));
}
