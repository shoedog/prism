//! Slice L round 1, item 1: a flat object pattern is followed only when EVERY entry is a
//! shorthand identifier or a pair with a plain property-identifier key and an identifier value.
//! Any rest, nested pattern, computed, string or numeric key, or default anywhere is not
//! followed, whatever the other entries say (an early SyntaxError otherwise leaves no valid
//! binding to follow).
use super::tests::{check, row, ANY, NS};

#[test]
fn a_pattern_with_any_non_flat_entry_is_not_followed() {
    let shapes = [
        ("rest sibling", "const { f, ...rest } = N;\nf();\n", "f"),
        (
            "rest sibling of a rename",
            "const { g: a, ...rest } = N;\na();\n",
            "a",
        ),
        ("nested sibling", "const { f, x: { y } } = N;\nf();\n", "f"),
        (
            "rest rebinding the name",
            "const { f, ...f } = N;\nf();\n",
            "f",
        ),
        (
            "nested rebinding the name",
            "const { f, x: { f } } = N;\nf();\n",
            "f",
        ),
        (
            "computed sibling",
            "const { f: a, ['x']: a } = N;\na();\n",
            "a",
        ),
        ("string sibling", "const { f: a, 'x': a } = N;\na();\n", "a"),
        ("numeric sibling", "const { f, 1: g } = N;\nf();\n", "f"),
        ("default sibling", "const { f, g = 1 } = N;\nf();\n", "f"),
        ("array sibling", "const { f, x: [y] } = N;\nf();\n", "f"),
    ];
    let rows = shapes
        .iter()
        .map(|(id, body, name)| row(id, &format!("{NS}{body}"), name, None))
        .collect();
    check(&ANY, rows);
}

#[test]
fn comments_inside_a_flat_pattern_are_ignored() {
    use super::tests::member;
    let rows = vec![row(
        "comment between entries",
        &format!("{NS}const {{ g, /* c */ f }} = N;\nf();\n"),
        "f",
        member("./m", "f"),
    )];
    check(&ANY, rows);
}

#[test]
fn a_pattern_binding_any_name_twice_is_not_followed() {
    // Whichever name is looked up, a repeated bound name makes the declaration invalid.
    let shapes = [
        ("shorthand twice", "const { f, f } = N;\nf();\n", "f"),
        ("shorthand then pair", "const { f, x: f } = N;\nf();\n", "f"),
        ("pair twice", "const { x: a, y: a } = N;\na();\n", "a"),
        (
            "another name repeats",
            "const { f, x: a, y: a } = N;\nf();\n",
            "f",
        ),
        (
            "reviewer input",
            "const { f, x: a, f: a } = N;\nf();\n",
            "f",
        ),
    ];
    let rows = shapes
        .iter()
        .map(|(id, body, name)| row(id, &format!("{NS}{body}"), name, None))
        .collect();
    check(&ANY, rows);
}

#[test]
fn the_same_property_key_bound_to_different_names_is_valid_and_followed() {
    use super::tests::member;
    let rows = vec![
        row(
            "first binding",
            &format!("{NS}const {{ f: a, f: b }} = N;\na();\n"),
            "a",
            member("./m", "f"),
        ),
        row(
            "second binding",
            &format!("{NS}const {{ f: a, f: b }} = N;\nb();\n"),
            "b",
            member("./m", "f"),
        ),
    ];
    check(&ANY, rows);
}
