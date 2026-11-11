// Fold 4: a comment between `#[macro_use]` and `extern crate`.
#[macro_use]
/* imported macros */
extern crate ext;

fn macro_use_comment() -> i32 {
    let mut x = 1; // @macro_use_comment:D
    assert!(x);
    x = get!() + 1; // @macro_use_comment:U:keep:macro_use_after_comment
    x
}
