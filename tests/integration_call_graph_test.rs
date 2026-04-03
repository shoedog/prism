mod common;
use common::*;

#[test]
fn test_call_graph_field_expression_call() {
    let source = r#"
#include <stdlib.h>

typedef struct {
    void (*callback)(void *data);
    void *data;
} timer_t;

void timeout_handler(void *data) {
    // handle timeout
}

void fire_timer(timer_t *timer) {
    timer->callback(timer->data);
}

void setup_timer(timer_t *timer) {
    timer->callback = timeout_handler;
    timer->data = NULL;
    fire_timer(timer);
}
"#;

    let mut files = BTreeMap::new();
    files.insert(
        "src/timer.c".to_string(),
        ParsedFile::parse("src/timer.c", source, Language::C).unwrap(),
    );

    let call_graph = CallGraph::build(&files);

    // fire_timer calls timer->callback — should resolve to callee_name "callback"
    let fire_timer_id = call_graph
        .functions
        .get("fire_timer")
        .expect("fire_timer should be in call graph");

    let fire_timer_calls = call_graph
        .calls
        .get(&fire_timer_id[0])
        .expect("fire_timer should have call sites");

    let callee_names: Vec<&str> = fire_timer_calls
        .iter()
        .map(|s| s.callee_name.as_str())
        .collect();
    assert!(
        callee_names.contains(&"callback"),
        "timer->callback(...) should resolve callee_name to 'callback', got: {:?}",
        callee_names
    );
}

#[test]
fn test_call_graph_level2_dispatch_table() {
    let source = r#"
#include <stdlib.h>

void handle_get(int req) { }
void handle_set(int req) { }
void handle_delete(int req) { }

typedef void (*handler_fn)(int);

void dispatch(int cmd, int req) {
    handler_fn handlers[] = {handle_get, handle_set, handle_delete};
    handlers[cmd](req);
}
"#;

    let mut files = BTreeMap::new();
    files.insert(
        "src/dispatch.c".to_string(),
        ParsedFile::parse("src/dispatch.c", source, Language::C).unwrap(),
    );

    let call_graph = CallGraph::build(&files);

    let dispatch_id = &call_graph.functions.get("dispatch").unwrap()[0];
    let dispatch_calls = call_graph.calls.get(dispatch_id).unwrap();
    let callee_names: BTreeSet<&str> = dispatch_calls
        .iter()
        .map(|s| s.callee_name.as_str())
        .collect();

    assert!(
        callee_names.contains("handle_get"),
        "Level 2: dispatch table should resolve to handle_get, got: {:?}",
        callee_names
    );
    assert!(
        callee_names.contains("handle_set"),
        "Level 2: dispatch table should resolve to handle_set, got: {:?}",
        callee_names
    );
    assert!(
        callee_names.contains("handle_delete"),
        "Level 2: dispatch table should resolve to handle_delete, got: {:?}",
        callee_names
    );
}

#[test]
fn test_call_graph_level2_global_dispatch_table() {
    let source = r#"
#include <stdlib.h>

int my_open(int fd) { return 0; }
int my_read(int fd) { return 0; }
int my_write(int fd) { return 0; }

typedef int (*file_op)(int);

file_op file_ops[] = {my_open, my_read, my_write};

void do_operation(int op, int fd) {
    file_ops[op](fd);
}
"#;

    let mut files = BTreeMap::new();
    files.insert(
        "src/fileops.c".to_string(),
        ParsedFile::parse("src/fileops.c", source, Language::C).unwrap(),
    );

    let call_graph = CallGraph::build(&files);

    let do_op_id = &call_graph.functions.get("do_operation").unwrap()[0];
    let do_op_calls = call_graph.calls.get(do_op_id).unwrap();
    let callee_names: BTreeSet<&str> = do_op_calls.iter().map(|s| s.callee_name.as_str()).collect();

    assert!(
        callee_names.contains("my_open"),
        "Level 2: global dispatch table should resolve to my_open, got: {:?}",
        callee_names
    );
    assert!(
        callee_names.contains("my_read"),
        "Level 2: global dispatch table should resolve to my_read, got: {:?}",
        callee_names
    );
    assert!(
        callee_names.contains("my_write"),
        "Level 2: global dispatch table should resolve to my_write, got: {:?}",
        callee_names
    );
}

#[test]
fn test_call_graph_static_disambiguation() {
    let file_a = r#"
static int init(void) {
    return 0;
}

int setup_a(void) {
    return init();
}
"#;

    let file_b = r#"
static int init(void) {
    return 1;
}

int setup_b(void) {
    return init();
}
"#;

    let mut files = BTreeMap::new();
    files.insert(
        "src/a.c".to_string(),
        ParsedFile::parse("src/a.c", file_a, Language::C).unwrap(),
    );
    files.insert(
        "src/b.c".to_string(),
        ParsedFile::parse("src/b.c", file_b, Language::C).unwrap(),
    );

    let call_graph = CallGraph::build(&files);

    // Both init functions should be registered as static
    assert!(
        call_graph
            .static_functions
            .contains(&("src/a.c".to_string(), "init".to_string())),
        "init in a.c should be detected as static"
    );
    assert!(
        call_graph
            .static_functions
            .contains(&("src/b.c".to_string(), "init".to_string())),
        "init in b.c should be detected as static"
    );

    // setup_a's call to init() should resolve to a.c's init, not b.c's
    let callees_a = call_graph.resolve_callees("init", "src/a.c");
    assert_eq!(
        callees_a.len(),
        1,
        "init() called from a.c should resolve to exactly 1 function, got: {:?}",
        callees_a
    );
    assert_eq!(
        callees_a[0].file, "src/a.c",
        "init() called from a.c should resolve to a.c's init"
    );

    // setup_b's call to init() should resolve to b.c's init
    let callees_b = call_graph.resolve_callees("init", "src/b.c");
    assert_eq!(
        callees_b.len(),
        1,
        "init() called from b.c should resolve to exactly 1 function, got: {:?}",
        callees_b
    );
    assert_eq!(
        callees_b[0].file, "src/b.c",
        "init() called from b.c should resolve to b.c's init"
    );
}

#[test]
fn test_call_graph_static_vs_non_static() {
    let file_a = r#"
static int helper(int x) {
    return x * 2;
}

int process_a(int x) {
    return helper(x);
}
"#;

    let file_b = r#"
int helper(int x) {
    return x + 1;
}
"#;

    let file_c = r#"
int process_c(int x) {
    return helper(x);
}
"#;

    let mut files = BTreeMap::new();
    files.insert(
        "src/a.c".to_string(),
        ParsedFile::parse("src/a.c", file_a, Language::C).unwrap(),
    );
    files.insert(
        "src/b.c".to_string(),
        ParsedFile::parse("src/b.c", file_b, Language::C).unwrap(),
    );
    files.insert(
        "src/c.c".to_string(),
        ParsedFile::parse("src/c.c", file_c, Language::C).unwrap(),
    );

    let call_graph = CallGraph::build(&files);

    // a.c has static helper — process_a's call should resolve to a.c's helper only
    let callees_a = call_graph.resolve_callees("helper", "src/a.c");
    assert_eq!(
        callees_a.len(),
        1,
        "helper() from a.c should resolve to a.c's static helper only"
    );
    assert_eq!(callees_a[0].file, "src/a.c");

    // c.c has no local helper — should resolve to b.c's non-static helper
    // (but NOT a.c's static helper)
    let callees_c = call_graph.resolve_callees("helper", "src/c.c");
    assert_eq!(
        callees_c.len(),
        1,
        "helper() from c.c should resolve to b.c's non-static helper only, got: {:?}",
        callees_c
    );
    assert_eq!(
        callees_c[0].file, "src/b.c",
        "helper() from c.c should resolve to b.c (not a.c's static)"
    );
}

#[test]
fn test_call_graph_level3_cross_file_callback() {
    let executor_source = r#"
typedef void (*event_cb)(int);

void on_event(event_cb callback, int event_id) {
    callback(event_id);
}
"#;

    let caller_source = r#"
void handle_connect(int id) {
    // process connect event
}

void init_events(void) {
    on_event(handle_connect, 1);
}
"#;

    let mut files = BTreeMap::new();
    files.insert(
        "src/executor.c".to_string(),
        ParsedFile::parse("src/executor.c", executor_source, Language::C).unwrap(),
    );
    files.insert(
        "src/caller.c".to_string(),
        ParsedFile::parse("src/caller.c", caller_source, Language::C).unwrap(),
    );

    let call_graph = CallGraph::build(&files);

    let on_event_id = &call_graph.functions.get("on_event").unwrap()[0];
    let on_event_calls = call_graph.calls.get(on_event_id).unwrap();
    let callee_names: BTreeSet<&str> = on_event_calls
        .iter()
        .map(|s| s.callee_name.as_str())
        .collect();

    assert!(
        callee_names.contains("handle_connect"),
        "Level 3: cross-file on_event(handle_connect, 1) should resolve callback, got: {:?}",
        callee_names
    );
}

#[test]
fn test_call_graph_level3_with_local_variable() {
    let source = r#"
void real_handler(int x) {
    // actual implementation
}

typedef void (*handler_fn)(int);

void invoke(handler_fn cb, int val) {
    cb(val);
}

void setup(void) {
    handler_fn h = real_handler;
    invoke(h, 10);
}
"#;

    let mut files = BTreeMap::new();
    files.insert(
        "src/composed.c".to_string(),
        ParsedFile::parse("src/composed.c", source, Language::C).unwrap(),
    );

    let call_graph = CallGraph::build(&files);

    let invoke_id = &call_graph.functions.get("invoke").unwrap()[0];
    let invoke_calls = call_graph.calls.get(invoke_id).unwrap();
    let callee_names: BTreeSet<&str> = invoke_calls
        .iter()
        .map(|s| s.callee_name.as_str())
        .collect();

    assert!(
        callee_names.contains("real_handler"),
        "Level 3+1: invoke(h, 10) where h = real_handler should resolve cb to real_handler, got: {:?}",
        callee_names
    );
}
