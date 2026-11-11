// Fold 4 (#630 slice 5, Rust): format captures in nested statements, macro_rules! in token
// soup outside owners, comments in soup, and the format grammar.
// Markers: `// @id:D` is the def line, `// @id:U:keep|drop:reason` the use line of a row.
macro_rules! identity { ($($t:tt)*) => { $($t)* }; }
macro_rules! gen_cmt { ($v:ident) => { macro_rules! get { () => { $v }; } }; }
macro_rules! outer_cmt { ($v:ident) => { gen_cmt /* c */ !($v); }; }
macro_rules! outer_bang { ($v:ident) => { gen_cmt ! /* c */ ($v); }; }
macro_rules! outer_meta { ($m:ident, $v:ident) => { $m /* c */ ! ($v); }; }
mod unrelated {
    macro_rules! install_soup { ($v:ident) => { $v }; }
    macro_rules! install_cfg { ($v:ident) => { $v }; }
}
identity! {
    macro_rules! install_soup {
        ($v:ident) => { macro_rules! get { () => { $v }; } };
    }
}
identity! {
    macro_rules! assert {
        ($v:ident) => { macro_rules! get { () => { $v }; } };
    }
}
cfg_if::cfg_if! {
    if #[cfg(unix)] {
        macro_rules! install_cfg {
            ($v:ident) => { macro_rules! get { () => { $v }; } };
        }
    }
}

fn label_if_let(prefix: Option<&str>) -> String {
    let mut name = String::from("n"); // @label_if_let:D
    name = if let Some(p) = prefix { // @label_if_let:U:keep:capture_in_nested_block
        format!("{p}::{name}")
    } else {
        String::from("anon")
    };
    name
}

fn label_closure(items: &[u8]) -> String {
    let mut name = String::from("n"); // @label_closure:D
    name = items.iter().map(|i| { // @label_closure:U:keep:capture_in_closure_block
        format!("{name}{i}")
    }).collect();
    name
}

fn label_match(k: u8) -> String {
    let mut x = String::from("n"); // @label_match:D
    x = match k { // @label_match:U:keep:capture_in_match_arm
        1 => {
            format!("{x}1")
        }
        _ => String::new(),
    };
    x
}

fn label_dominated(prefix: Option<&str>) -> String {
    let mut x = String::from("n"); // @label_dominated:D
    x = String::from("m");
    x = if let Some(p) = prefix { // @label_dominated:U:drop:capture_dominated_by_later_write
        format!("{p}::{x}")
    } else {
        String::new()
    };
    x
}

fn soup_def_clean_elsewhere() -> i32 {
    let mut x = 1; // @soup_def_clean_elsewhere:D
    install_soup!(x);
    x = get!() + 1; // @soup_def_clean_elsewhere:U:keep:module_soup_macro_rules
    x
}

fn soup_def_std_spelling() -> i32 {
    let mut x = 1; // @soup_def_std_spelling:D
    assert!(x);
    x = get!() + 1; // @soup_def_std_spelling:U:keep:module_soup_defines_std_spelling
    x
}

fn soup_def_cfg_if() -> i32 {
    let mut x = 1; // @soup_def_cfg_if:D
    install_cfg!(x);
    x = get!() + 1; // @soup_def_cfg_if:U:keep:cfg_if_soup_macro_rules
    x
}

fn comment_in_owner_soup() -> i32 {
    let mut x = 1; // @comment_in_owner_soup:D
    identity! { gen_cmt /* c */ !(x); }
    x = get!() + 1; // @comment_in_owner_soup:U:keep:comment_before_bang
    x
}

fn comment_in_definition_body() -> i32 {
    let mut x = 1; // @comment_in_definition_body:D
    outer_cmt!(x);
    x = get!() + 1; // @comment_in_definition_body:U:keep:transitive_comment_before_bang
    x
}

fn comment_after_bang() -> i32 {
    let mut x = 1; // @comment_after_bang:D
    outer_bang!(x);
    x = get!() + 1; // @comment_after_bang:U:keep:comment_between_bang_and_tree
    x
}

fn comment_after_metavariable() -> i32 {
    let mut x = 1; // @comment_after_metavariable:D
    outer_meta!(gen_cmt, x);
    x = get!() + 1; // @comment_after_metavariable:U:keep:comment_between_metavariable_and_bang
    x
}

fn comment_between_bang_and_tree_in_soup() -> i32 {
    let mut x = 1; // @comment_between_bang_and_tree_in_soup:D
    identity! { gen_cmt ! /* c */ (x); }
    x = get!() + 1; // @comment_between_bang_and_tree_in_soup:U:keep:soup_comment_after_bang
    x
}

fn format_zero_width() -> usize {
    let mut x: usize = 3; // @format_zero_width:D
    x = format!("{:0x$}", 1).len(); // @format_zero_width:U:keep:zero_flag_then_named_width
    x
}

fn format_all_flags() -> usize {
    let mut x: usize = 3; // @format_all_flags:D
    x = format!("{:>+#0x$.y$}", 1, y = 2).len(); // @format_all_flags:U:keep:full_spec_named_width
    x
}

fn format_hex_escape() -> String {
    let mut x = String::from("a"); // @format_hex_escape:D
    x = format!("\x7bx}b"); // @format_hex_escape:U:keep:cooked_escape_decodes_to_capture
    x
}

fn format_unicode_escape() -> String {
    let mut x = String::from("a"); // @format_unicode_escape:D
    x = format!("\u{7b}x}b"); // @format_unicode_escape:U:keep:unicode_escape_decodes_to_capture
    x
}

fn format_raw_double_brace() -> String {
    let mut x = String::from("a"); // @format_raw_double_brace:D
    x = format!(r"{{x}}"); // @format_raw_double_brace:U:drop:raw_string_stays_raw
    x
}

fn format_plain_width() -> String {
    let mut x = String::from("a"); // @format_plain_width:D
    x = format!("{:5}", 1); // @format_plain_width:U:drop:numeric_width_names_nothing
    x
}
