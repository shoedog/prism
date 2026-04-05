#[path = "../common/mod.rs"]
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

// ---------------------------------------------------------------------------
// Static function disambiguation tests
// ---------------------------------------------------------------------------

/// Two files both define `static int init()`. callers_of_in_file should only
/// return callers from the correct file.
#[test]
fn test_static_init_disambiguation_callers_of_in_file() {
    let source_a = r#"
static int init(void) {
    return 0;
}

void main_a(void) {
    init();
}
"#;

    let source_b = r#"
static int init(void) {
    return 1;
}

void main_b(void) {
    init();
}
"#;

    let mut files = BTreeMap::new();
    files.insert(
        "a.c".to_string(),
        ParsedFile::parse("a.c", source_a, Language::C).unwrap(),
    );
    files.insert(
        "b.c".to_string(),
        ParsedFile::parse("b.c", source_b, Language::C).unwrap(),
    );

    let call_graph = CallGraph::build(&files);

    // Callers of init in a.c should only be main_a
    let callers_a = call_graph.callers_of_in_file("init", 1, Some("a.c"));
    let caller_names_a: Vec<&str> = callers_a.iter().map(|(fid, _)| fid.name.as_str()).collect();
    assert!(
        caller_names_a.contains(&"main_a"),
        "callers_of_in_file(init, a.c) should include main_a, got: {:?}",
        caller_names_a
    );
    assert!(
        !caller_names_a.contains(&"main_b"),
        "callers_of_in_file(init, a.c) should NOT include main_b, got: {:?}",
        caller_names_a
    );

    // Callers of init in b.c should only be main_b
    let callers_b = call_graph.callers_of_in_file("init", 1, Some("b.c"));
    let caller_names_b: Vec<&str> = callers_b.iter().map(|(fid, _)| fid.name.as_str()).collect();
    assert!(
        caller_names_b.contains(&"main_b"),
        "callers_of_in_file(init, b.c) should include main_b, got: {:?}",
        caller_names_b
    );
    assert!(
        !caller_names_b.contains(&"main_a"),
        "callers_of_in_file(init, b.c) should NOT include main_a, got: {:?}",
        caller_names_b
    );
}

/// Static + non-static same name: a.c has static init(), c.c has non-static
/// init(), d.c calls init(). d.c's call should resolve to c.c's init only.
#[test]
fn test_static_plus_nonstatic_same_name() {
    let source_a = r#"
static int init(void) {
    return 0;
}

void main_a(void) {
    init();
}
"#;

    let source_c = r#"
int init(void) {
    return 42;
}
"#;

    let source_d = r#"
void main_d(void) {
    init();
}
"#;

    let mut files = BTreeMap::new();
    files.insert(
        "a.c".to_string(),
        ParsedFile::parse("a.c", source_a, Language::C).unwrap(),
    );
    files.insert(
        "c.c".to_string(),
        ParsedFile::parse("c.c", source_c, Language::C).unwrap(),
    );
    files.insert(
        "d.c".to_string(),
        ParsedFile::parse("d.c", source_d, Language::C).unwrap(),
    );

    let call_graph = CallGraph::build(&files);

    // d.c calls init() — should resolve to c.c's non-static init, not a.c's static init
    let resolved = call_graph.resolve_callees("init", "d.c");
    let resolved_files: Vec<&str> = resolved.iter().map(|fid| fid.file.as_str()).collect();
    assert!(
        resolved_files.contains(&"c.c"),
        "d.c's call to init() should resolve to c.c, got: {:?}",
        resolved_files
    );
    assert!(
        !resolved_files.contains(&"a.c"),
        "d.c's call to init() should NOT resolve to a.c (static), got: {:?}",
        resolved_files
    );

    // Callers of c.c:init should include d.c's main_d but NOT a.c's main_a
    let callers_c = call_graph.callers_of_in_file("init", 1, Some("c.c"));
    let caller_names: Vec<&str> = callers_c.iter().map(|(fid, _)| fid.name.as_str()).collect();
    assert!(
        caller_names.contains(&"main_d"),
        "callers_of_in_file(init, c.c) should include main_d, got: {:?}",
        caller_names
    );
    assert!(
        !caller_names.contains(&"main_a"),
        "callers_of_in_file(init, c.c) should NOT include main_a, got: {:?}",
        caller_names
    );
}

/// resolve_callers should filter out calls that target a different static definition.
#[test]
fn test_resolve_callers_filters_static() {
    let source_a = r#"
static int init(void) {
    return 0;
}

void main_a(void) {
    init();
}
"#;

    let source_b = r#"
static int init(void) {
    return 1;
}

void main_b(void) {
    init();
}
"#;

    let mut files = BTreeMap::new();
    files.insert(
        "a.c".to_string(),
        ParsedFile::parse("a.c", source_a, Language::C).unwrap(),
    );
    files.insert(
        "b.c".to_string(),
        ParsedFile::parse("b.c", source_b, Language::C).unwrap(),
    );

    let call_graph = CallGraph::build(&files);

    // resolve_callers for init in a.c should only return call sites from a.c
    let callers_a = call_graph.resolve_callers("init", "a.c");
    let caller_files: Vec<&str> = callers_a
        .iter()
        .map(|site| site.caller.file.as_str())
        .collect();
    assert!(
        caller_files.contains(&"a.c"),
        "resolve_callers(init, a.c) should include caller from a.c, got: {:?}",
        caller_files
    );
    assert!(
        !caller_files.contains(&"b.c"),
        "resolve_callers(init, a.c) should NOT include caller from b.c, got: {:?}",
        caller_files
    );
}

/// Non-static functions should still be found cross-file via callers_of.
#[test]
fn test_nonstatic_cross_file_callers() {
    let source_lib = r#"
void process(int x) {
    // do work
}
"#;

    let source_main = r#"
void run(void) {
    process(42);
}
"#;

    let mut files = BTreeMap::new();
    files.insert(
        "lib.c".to_string(),
        ParsedFile::parse("lib.c", source_lib, Language::C).unwrap(),
    );
    files.insert(
        "main.c".to_string(),
        ParsedFile::parse("main.c", source_main, Language::C).unwrap(),
    );

    let call_graph = CallGraph::build(&files);

    // callers_of (no file filter) should find cross-file callers
    let callers = call_graph.callers_of("process", 1);
    let caller_names: Vec<&str> = callers.iter().map(|(fid, _)| fid.name.as_str()).collect();
    assert!(
        caller_names.contains(&"run"),
        "callers_of(process) should include cross-file caller 'run', got: {:?}",
        caller_names
    );

    // callers_of_in_file targeting lib.c should also find cross-file caller
    let callers_in_file = call_graph.callers_of_in_file("process", 1, Some("lib.c"));
    let caller_names_in_file: Vec<&str> = callers_in_file
        .iter()
        .map(|(fid, _)| fid.name.as_str())
        .collect();
    assert!(
        caller_names_in_file.contains(&"run"),
        "callers_of_in_file(process, lib.c) should include cross-file caller 'run', got: {:?}",
        caller_names_in_file
    );
}

// ===== Level 4: Struct field callback resolution =====

#[test]
fn test_struct_callback_same_file() {
    let source = r#"
typedef struct {
    void (*callback)(void *data);
    void *data;
} timer_t;

void timeout_handler(void *data) { }

void fire_timer(timer_t *timer) {
    timer->callback(timer->data);
}

void setup_timer(timer_t *timer) {
    timer->callback = timeout_handler;
    fire_timer(timer);
}
"#;

    let mut files = BTreeMap::new();
    files.insert(
        "src/timer.c".to_string(),
        ParsedFile::parse("src/timer.c", source, Language::C).unwrap(),
    );
    let call_graph = CallGraph::build(&files);

    let fire_timer_id = &call_graph.functions.get("fire_timer").unwrap()[0];
    let fire_timer_calls = call_graph.calls.get(fire_timer_id).unwrap();
    let callee_names: BTreeSet<&str> = fire_timer_calls
        .iter()
        .map(|s| s.callee_name.as_str())
        .collect();

    assert!(
        callee_names.contains("timeout_handler"),
        "Level 4: timer->callback should resolve to timeout_handler, got: {:?}",
        callee_names
    );
}

#[test]
fn test_struct_callback_cross_file() {
    let setup_src = r#"
void timeout_handler(void *data) { }

void setup_timer(void *timer) {
    timer->callback = timeout_handler;
}
"#;
    let engine_src = r#"
void fire_timer(void *timer) {
    timer->callback(timer->data);
}
"#;

    let mut files = BTreeMap::new();
    files.insert(
        "src/setup.c".to_string(),
        ParsedFile::parse("src/setup.c", setup_src, Language::C).unwrap(),
    );
    files.insert(
        "src/engine.c".to_string(),
        ParsedFile::parse("src/engine.c", engine_src, Language::C).unwrap(),
    );
    let call_graph = CallGraph::build(&files);

    let fire_timer_id = &call_graph.functions.get("fire_timer").unwrap()[0];
    let fire_timer_calls = call_graph.calls.get(fire_timer_id).unwrap();
    let callee_names: BTreeSet<&str> = fire_timer_calls
        .iter()
        .map(|s| s.callee_name.as_str())
        .collect();

    assert!(
        callee_names.contains("timeout_handler"),
        "Level 4 cross-file: timer->callback should resolve to timeout_handler, got: {:?}",
        callee_names
    );
}

#[test]
fn test_struct_dot_callback() {
    let source = r#"
void handler(void) { }

void caller(void) {
    struct s;
    s.on_event(42);
}

void setup(void) {
    struct s;
    s.on_event = handler;
}
"#;

    let mut files = BTreeMap::new();
    files.insert(
        "src/dot.c".to_string(),
        ParsedFile::parse("src/dot.c", source, Language::C).unwrap(),
    );
    let call_graph = CallGraph::build(&files);

    let caller_id = &call_graph.functions.get("caller").unwrap()[0];
    let caller_calls = call_graph.calls.get(caller_id).unwrap();
    let callee_names: BTreeSet<&str> = caller_calls
        .iter()
        .map(|s| s.callee_name.as_str())
        .collect();

    assert!(
        callee_names.contains("handler"),
        "Level 4: s.on_event should resolve to handler, got: {:?}",
        callee_names
    );
}

#[test]
fn test_designated_initializer() {
    let source = r#"
void my_handler(void *data) { }

void fire(void *t) {
    t->callback(t->data);
}

void init(void) {
    struct timer_t t = { .callback = my_handler };
    fire(&t);
}
"#;

    let mut files = BTreeMap::new();
    files.insert(
        "src/init.c".to_string(),
        ParsedFile::parse("src/init.c", source, Language::C).unwrap(),
    );
    let call_graph = CallGraph::build(&files);

    let fire_id = &call_graph.functions.get("fire").unwrap()[0];
    let fire_calls = call_graph.calls.get(fire_id).unwrap();
    let callee_names: BTreeSet<&str> = fire_calls.iter().map(|s| s.callee_name.as_str()).collect();

    assert!(
        callee_names.contains("my_handler"),
        "Level 4: designated initializer .callback = my_handler should resolve, got: {:?}",
        callee_names
    );
}

#[test]
fn test_multiple_assignments() {
    let source = r#"
void handler_a(void) { }
void handler_b(void) { }

void dispatch(void *ctx) {
    ctx->on_event(ctx);
}

void setup_a(void *ctx) { ctx->on_event = handler_a; }
void setup_b(void *ctx) { ctx->on_event = handler_b; }
"#;

    let mut files = BTreeMap::new();
    files.insert(
        "src/multi.c".to_string(),
        ParsedFile::parse("src/multi.c", source, Language::C).unwrap(),
    );
    let call_graph = CallGraph::build(&files);

    let dispatch_id = &call_graph.functions.get("dispatch").unwrap()[0];
    let dispatch_calls = call_graph.calls.get(dispatch_id).unwrap();
    let callee_names: BTreeSet<&str> = dispatch_calls
        .iter()
        .map(|s| s.callee_name.as_str())
        .collect();

    assert!(
        callee_names.contains("handler_a") && callee_names.contains("handler_b"),
        "Level 4: multiple assignments should resolve both handlers, got: {:?}",
        callee_names
    );
}

#[test]
fn test_no_false_resolution() {
    // "callback" is also a function name, but the call has no qualifier — Level 4 shouldn't fire
    let source = r#"
void callback(void) { }

void caller(void) {
    callback();
}
"#;

    let mut files = BTreeMap::new();
    files.insert(
        "src/direct.c".to_string(),
        ParsedFile::parse("src/direct.c", source, Language::C).unwrap(),
    );
    let call_graph = CallGraph::build(&files);

    let caller_id = &call_graph.functions.get("caller").unwrap()[0];
    let caller_calls = call_graph.calls.get(caller_id).unwrap();
    let callee_names: Vec<&str> = caller_calls
        .iter()
        .map(|s| s.callee_name.as_str())
        .collect();

    // Should only have "callback" as a direct call, not duplicated
    assert_eq!(
        callee_names.iter().filter(|&&n| n == "callback").count(),
        1,
        "Direct call to callback() should not trigger Level 4, got: {:?}",
        callee_names
    );
}
