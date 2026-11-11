// Fold 3 (#630 slice 5, Rust): macro hygiene by per-name census refusal, and format captures.
// Markers: `// @id:D` is the def line, `// @id:U:keep|drop:reason` the use line of a row.
macro_rules! install_get {
    ($v:ident) => {
        macro_rules! get { () => { $v }; }
    };
}
macro_rules! identity { ($($t:tt)*) => { $($t)* }; }
macro_rules! install {
    ($k:ident, $v:ident) => { $k! get2 { () => { $v }; } };
}
macro_rules! outer { ($v:ident) => { install_get!($v) }; }
macro_rules! via { ($m:ident, $v:ident) => { $m!($v) }; }
macro_rules! m_clean { ($v:ident) => { $v }; }
macro_rules! dup { ($v:ident) => { $v }; }
macro_rules! dup { ($v:ident) => { macro_rules! get3 { () => { $v }; } }; }
macro_rules! install_expr {
    ($e:expr) => {
        macro_rules! get4 { () => { $e }; }
    };
}

fn gen_u() -> i32 {
    let mut x = 1; // @gen_u:D
    install_get!(x);
    x = get!() + 1; // @gen_u:U:keep:generated_reader_u
    x
}

fn gen_d() {
    let x = 1; // @gen_d:D
    install_get!(x);
    let x = 2;
    std::hint::black_box(get!() + x); // @gen_d:U:keep:generated_reader_d
}

fn soup_macro_rules() -> i32 {
    let mut x = 1; // @soup_macro_rules:D
    identity! { macro_rules! bump { () => { x + 1 }; } }
    x = bump!(); // @soup_macro_rules:U:keep:soup_macro_rules_owner_off
    x
}

fn metavariable_macro_rules() -> i32 {
    let mut x = 1; // @metavariable_macro_rules:D
    install!(macro_rules, x);
    x = get2!(); // @metavariable_macro_rules:U:keep:install_macro_rules_owner_off
    x
}

fn generator_transitive() -> i32 {
    let mut x = 1; // @generator_transitive:D
    outer!(x);
    x = get!() + 1; // @generator_transitive:U:keep:generator_transitive
    x
}

fn metavariable_name() -> i32 {
    let mut x = 1; // @metavariable_name:D
    via!(install_get, x);
    x = get!() + 1; // @metavariable_name:U:keep:metavariable_in_macro_position
    x
}

fn generator_nested_in_soup() -> i32 {
    let mut x = 1; // @generator_nested_in_soup:D
    identity!(install_get!(x));
    x = get!() + 1; // @generator_nested_in_soup:U:keep:generator_nested_in_soup
    x
}

fn external_refused() -> i32 {
    let mut x = 1; // @external_refused:D
    ext::install!(x);
    x = get!() + 1; // @external_refused:U:keep:external_refused
    x
}

fn unknown_refused() -> i32 {
    let mut x = 1; // @unknown_refused:D
    install_unknown!(x);
    x = get!() + 1; // @unknown_refused:U:keep:unknown_refused
    x
}

fn clean_in_repo() {
    let x = 1; // @clean_in_repo:D
    m_clean!(x);
    let x = 2;
    sink(x); // @clean_in_repo:U:drop:clean_in_repo_allowed
}

fn std_matches() {
    let x = 1; // @std_matches:D
    let _ = matches!(x, 1);
    let x = 2;
    sink(x); // @std_matches:U:drop:std_control
}

fn std_qualified() {
    let x = 1; // @std_qualified:D
    let x = 2;
    std::println!("{}", x); // @std_qualified:U:drop:std_qualified_allowed
}

fn conflicting_defs() {
    let x = 1; // @conflicting_defs:D
    dup!(x);
    let x = 2;
    sink(get3!() + x); // @conflicting_defs:U:keep:conflicting_defs_refused
}

fn name_builder() -> i32 {
    let mut x = 1; // @name_builder:D
    paste::paste! { let _k = 1; }
    x = 5; // @name_builder:U:keep:name_builder_owner_off
    x
}

fn expr_fragment() -> i32 {
    let mut x = 1; // @expr_fragment:D
    install_expr!(x + 1);
    x = get4!(); // @expr_fragment:U:keep:expr_fragment_generator
    x
}

fn format_capture() {
    let mut x = String::new(); // @format_capture:D
    x = format!("{x}b"); // @format_capture:U:keep:format_capture_read
    sink_s(x);
}

fn format_capture_spec() {
    let mut x = String::new(); // @format_capture_spec:D
    x = format!("{x:?}"); // @format_capture_spec:U:keep:format_capture_debug_spec
    sink_s(x);
}

fn format_capture_write(f: &mut Out) {
    let mut x = 1; // @format_capture_write:D
    x = write!(f, "{x}").map(|_| 1).unwrap(); // @format_capture_write:U:keep:format_capture_write_line
    sink(x);
}

fn format_capture_multiline() {
    let mut x = String::new(); // @format_capture_multiline:D
    x = format!( // @format_capture_multiline:U:keep:statement_lines_decide
        "{x}b"
    );
    sink_s(x);
}

fn format_escaped_control() {
    let mut x = String::new(); // @format_escaped_control:D
    x = format!("{{x}}"); // @format_escaped_control:U:drop:escaped_brace_is_no_capture
    sink_s(x);
}

fn format_capture_dominated() {
    let x = 1; // @format_capture_dominated:D
    let x = 2;
    pair(x, format!("{x}")); // @format_capture_dominated:U:drop:capture_reads_dominating_def
}
