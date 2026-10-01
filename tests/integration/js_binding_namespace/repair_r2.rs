// N1 and N5 exercise written declarators and class terminals separately from
// N6's written declaration. None may borrow g's same-named function identity.
const R2_N1: &str = "const g = function f(){ return 1; };\nfunction inner(){ function f(){ return 2; } return f; }\nlet f = 0;\nf = inner();\nexport { f, g };\n";
const R2_N5: &str = "const g = function f(){ return 1; };\nfunction mk(){ function f(){ return 2; } return f; }\nexport class f {}\nexport { g };\n";
const R2_N6: &str = "const g = function f(){ return 1; };\nfunction mk(){ function f(){ return 2; } return f; }\nexport function f(){ return 3; }\nf = mk();\nexport { g };\n";

fn r2_collision_rows(ext: &str, lib: &str, lines: &[usize]) {
    let cg = graph(ext, &[("lib/index", lib), ("app", APP)]);
    let expected = vec![lines
        .iter()
        .map(|line| exact(ext, "lib/index", "f", *line))
        .collect::<Vec<_>>()
        .join(", ")];
    // Pin the exact target identities, spans, grades and kinds from base,
    // independently of the namespace projection's keep-base implementation.
    assert_eq!(base_rows(&cg, "f"), expected);
    assert_eq!(rows(&cg, "run", "f"), expected);
}

#[test]
fn r2_n1_written_declarator_jsx() {
    r2_collision_rows("jsx", R2_N1, &[1, 2]);
}
#[test]
fn r2_n1_written_declarator_tsx() {
    r2_collision_rows("tsx", R2_N1, &[1, 2]);
}
#[test]
fn r2_n5_class_terminal_jsx() {
    r2_collision_rows("jsx", R2_N5, &[1, 2]);
}
#[test]
fn r2_n5_class_terminal_tsx() {
    r2_collision_rows("tsx", R2_N5, &[1, 2]);
}
#[test]
fn r2_n6_written_declaration_jsx() {
    r2_collision_rows("jsx", R2_N6, &[1, 2, 3]);
}
#[test]
fn r2_n6_written_declaration_tsx() {
    r2_collision_rows("tsx", R2_N6, &[1, 2, 3]);
}
