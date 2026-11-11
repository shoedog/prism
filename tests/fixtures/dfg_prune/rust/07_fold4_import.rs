// Fold 4: a raw-identifier import alias binds the std spelling.
use ext::install_get as r#assert;

fn raw_alias_import() -> i32 {
    let mut x = 1; // @raw_alias_import:D
    assert!(x);
    x = get!() + 1; // @raw_alias_import:U:keep:raw_alias_binds_assert
    x
}

fn raw_local(mut r#type: i32) -> i32 { // @raw_local:D
    ext::gen!(r#type);
    r#type = get!() + 1; // @raw_local:U:keep:raw_identifier_refused
    r#type
}
