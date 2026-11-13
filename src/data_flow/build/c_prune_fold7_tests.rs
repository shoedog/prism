//! #630 slice 6c fold 7: grammar-bump pins. tree-sitter-c currently rejects the spellings
//! below, so the owner has a parse error and is off (G1Error). If a grammar update starts
//! parsing one, `&__real__ x` becomes an lvalue of `x` the root finders have not been shown:
//! this test turns red, and the finder and its rows get a look before the owner is trusted.
use crate::ast::ParsedFile;
use crate::languages::Language;

#[test]
fn fold7_grammar_pins_hold() {
    for (name, src) in [
        (
            "real_unparenthesized",
            "int f(double c){double x=c;double*p=&__real__ x;return x;}",
        ),
        (
            "complex_type_alone",
            "int f(double c){_Complex double z=c;return z;}",
        ),
        (
            "complex_type",
            "int f(double c){_Complex double z=c;double*p=&__real__ z;return z;}",
        ),
        (
            "computed_goto_label_address",
            "int f(int c){void*l=&&done;goto *l;done:return c;}",
        ),
    ] {
        let parsed = ParsedFile::parse("a.c", src, Language::C).unwrap();
        assert!(
            parsed.tree.root_node().has_error(),
            "{name}: the C grammar now parses this spelling; re-check prune_roots::chain_roots"
        );
    }
}
