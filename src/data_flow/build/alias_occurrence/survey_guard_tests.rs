//! Guard tests for the #632 type-facts survey (main a0f2c427): they pin the alias-twin behaviour
//! of the survey's probe shapes that no other test in this directory observes at the same level,
//! so a later change (the #632 C/C++ slice, #818, a new language row) cannot move one silently.
//! Each test says whether the pinned result is the correct answer or a known residual
//! ("pins residual": the twin is spurious for a value copy or a label is stronger than the truth
//! needs). Nothing here is a claim that a residual is desirable.
//!
//! C, C++ and Java rows assert twin presence and the policy's suppressed lines only, never edge
//! labels: those labels depend on the initialiser-as-def extraction defect (#818, X-READ), which
//! is about to change. One exception: the C++ member row pins `M@2->M@3`, whose only `M` def on
//! line 2 is the twin (a static member operand is no X-READ def), so #818 does not move it. Rows whose twin presence itself would depend on an initialiser or operand
//! being recorded as a def are not pinned at all.
use super::test_support::{refs, suppressed_lines, twins};
use crate::ast::ParsedFile;
use crate::cpg::CodePropertyGraph;
use crate::data_flow::test_support::label_snapshot;
use crate::languages::Language;
use std::collections::BTreeMap;

/// Twin defs of `alias` as `(target path, line)`, sorted and deduplicated.
fn tw(file: &str, src: &str, lang: Language, alias: &str) -> Vec<(String, usize)> {
    let mut t = twins(&refs(file, src, lang), alias);
    t.sort();
    t.dedup();
    t
}
fn pair(path: &str, line: usize) -> (String, usize) {
    (path.to_string(), line)
}
fn supp(file: &str, src: &str, lang: Language) -> Vec<usize> {
    suppressed_lines(file, src, lang)
}

/// Rendered labels of the DataFlow rows whose endpoints are `edge` (`path@line->path@line`).
fn label(file: &str, lang: Language, src: &str, edge: &str) -> Vec<String> {
    let parsed = ParsedFile::parse(file, src, lang).unwrap();
    assert_eq!(parsed.parse_error_count, 0, "{file}: {src}");
    let cpg = CodePropertyGraph::build(&BTreeMap::from([(file.to_string(), parsed)]));
    let (from, to) = edge.split_once("->").unwrap();
    let split = |s: &str| {
        let (p, l) = s.rsplit_once('@').unwrap();
        (p.to_string(), l.parse::<usize>().unwrap())
    };
    let (from, to) = (split(from), split(to));
    let mut out: Vec<String> = label_snapshot(&cpg.dfg)
        .into_iter()
        .filter(|((f, t), _)| {
            (f.path.to_string(), f.line) == from && (t.path.to_string(), t.line) == to
        })
        .map(|(_, l)| format!("{l:?}"))
        .collect();
    out.sort();
    out
}

const ALIAS_UNSTABLE: &str = "NameOnly(AliasUnstable)";
const EXACT: &str = "Exact";

// ---- Go ----

const GO_STRUCT: &str = "package p\n\ntype S struct{ f int }\n\n";

#[test]
fn go_struct_value_field_write_keeps_the_twin_and_its_alias_unstable_edge() {
    // #632 (survey probe go/a.go `gstruct`): pins residual, `u := o` copies a struct value, so the
    // twin `o.f` at `u.f = 2` is spurious; the edge it feeds is graded nameonly/alias_unstable and
    // the real `o.f@6 -> o.f@9` edge stays exact (a Go twin never kills).
    let src = format!(
        "{GO_STRUCT}func g(o S) int {{\n\to.f = 1\n\tu := o\n\tu.f = 2\n\treturn o.f\n}}\n"
    );
    assert_eq!(tw("a.go", &src, Language::Go, "u"), vec![pair("o.f", 8)]);
    assert_eq!(supp("a.go", &src, Language::Go), vec![7]);
    assert_eq!(
        label("a.go", Language::Go, &src, "o.f@8->o.f@9"),
        [ALIAS_UNSTABLE]
    );
    assert_eq!(label("a.go", Language::Go, &src, "o.f@6->o.f@9"), [EXACT]);
}

#[test]
fn go_struct_pointer_field_write_keeps_the_twin_and_its_alias_unstable_edge() {
    // #632 (probe `gptr`): the twin is correct, a write through a pointer copy writes `o.f`; same
    // twin and labels as the value case, which is why the occurrence table cannot tell them apart.
    // Pins residual: `o.f@6->o.f@9` exact is wrong here (`u.f = 2` overwrites `o.f`; the twin
    // never kills, #783-style), so a slice that fixes it should update this assert.
    let src = format!(
        "{GO_STRUCT}func g(o *S) int {{\n\to.f = 1\n\tu := o\n\tu.f = 2\n\treturn o.f\n}}\n"
    );
    assert_eq!(tw("a.go", &src, Language::Go, "u"), vec![pair("o.f", 8)]);
    assert_eq!(
        label("a.go", Language::Go, &src, "o.f@8->o.f@9"),
        [ALIAS_UNSTABLE]
    );
    assert_eq!(label("a.go", Language::Go, &src, "o.f@6->o.f@9"), [EXACT]);
}

#[test]
fn go_slice_element_write_keeps_the_twin() {
    // #632 (probe `gslice`): correct, `u := o` shares the backing array so `u[0] = 2` writes `o[0]`.
    let src =
        "package p\n\nfunc g(o []int) int {\n\to[0] = 1\n\tu := o\n\tu[0] = 2\n\treturn o[0]\n}\n";
    assert_eq!(tw("a.go", src, Language::Go, "u"), vec![pair("o[]", 6)]);
    assert_eq!(supp("a.go", src, Language::Go), vec![5]);
}

#[test]
fn go_integer_alias_increment_and_compound_assignment_mint_no_twin() {
    // #632 (probe `gint`): correct. The declaration and `x += 3` are binding writes; `x++` is
    // no lvalue span at all, so it has neither a twin nor a suppression entry.
    let src = "package p\n\nfunc g(o int) int {\n\tx := o\n\tx++\n\tx += 3\n\treturn o + x\n}\n";
    assert!(tw("a.go", src, Language::Go, "x").is_empty());
    assert_eq!(supp("a.go", src, Language::Go), vec![4, 6]);
}

// ---- Java (twin presence only) ----

#[test]
fn java_integer_alias_compound_and_increment_mint_no_twin() {
    // #632 (probe `jint`): correct; `u++` is no lvalue span (nothing to suppress or keep).
    let src = "class B {\n  int f;\n  int jint(int o) {\n    int u = o;\n    u += 3;\n    u++;\n    return o + u;\n  }\n}\n";
    assert!(tw("B.java", src, Language::Java, "u").is_empty());
    assert_eq!(supp("B.java", src, Language::Java), vec![4, 5]);
}

#[test]
fn java_string_alias_concatenation_assignment_mints_no_twin() {
    // #632 (probe `jstr`): correct, `u += "x"` rebinds `u` and never writes `o`.
    let src = "class B {\n  String jstr(String o) {\n    String u = o;\n    u += \"x\";\n    return o + u;\n  }\n}\n";
    assert!(tw("B.java", src, Language::Java, "u").is_empty());
    assert_eq!(supp("B.java", src, Language::Java), vec![3, 4]);
}

// ---- C (twin presence only) ----

#[test]
fn c_scalar_alias_compound_and_increment_mint_no_twin() {
    // #632 (probe `cint`): correct; the declaration and `u += 3` are binding writes, `u++` has no span.
    let src = "int cint(int o) {\n  int u = o;\n  u += 3;\n  u++;\n  return o + u;\n}\n";
    assert!(tw("c.c", src, Language::C, "u").is_empty());
    assert_eq!(supp("c.c", src, Language::C), vec![2, 3]);
}

#[test]
fn c_struct_value_copy_field_write_keeps_the_twin() {
    // #632 (probe `cstruct`, row C-07): pins residual, `struct S u = o` copies the struct so the
    // twin `o.f` at `u.f = 2` is spurious; the classifier has no field table to prove it.
    let src = "struct S { int f; };\nint cstruct(struct S o) {\n  struct S u = o;\n  u.f = 2;\n  return o.f;\n}\n";
    assert_eq!(tw("c.c", src, Language::C, "u"), vec![pair("o.f", 4)]);
    assert_eq!(supp("c.c", src, Language::C), vec![3]);
}

#[test]
fn c_scalar_argument_of_an_unproven_callee_keeps_every_twin() {
    // #632 (probe `ccall`): pins residual, `ext` has no declaration in the file, so a by-value
    // `int o` argument is treated as possibly written (a header macro); for a by-value scalar the
    // twins are spurious. Every occurrence of the alias keeps its twin.
    let src = "int ccall(int o) {\n  int u = o;\n  u = 7;\n  ext(o);\n  return u;\n}\n";
    assert_eq!(
        tw("c.c", src, Language::C, "u"),
        vec![pair("o", 2), pair("o", 3)]
    );
    assert!(supp("c.c", src, Language::C).is_empty());
    // Control: the same body with a prototype of `ext` in the file removes both twins.
    let declared = format!("void ext(int);\n{src}");
    assert!(tw("c.c", &declared, Language::C, "u").is_empty());
    assert_eq!(supp("c.c", &declared, Language::C), vec![3, 4]);
}

#[test]
fn c_assignment_to_an_outer_name_is_a_binding_write() {
    // #632 (survey 3.2, outer-name aliases): correct, `g = m` writes the global `g`, never `m`.
    // C treats any plain assignment target as a binding write (C++ needs declaration evidence).
    let src = "int g;\nint cg(int m) {\n  g = m;\n  return m;\n}\n";
    assert!(tw("c.c", src, Language::C, "g").is_empty());
    assert_eq!(supp("c.c", src, Language::C), vec![3]);
}

// ---- C++ (twin presence only) ----

#[test]
fn cpp_size_t_alias_compound_assignment_keeps_the_twin() {
    // #632 (probe `cppsz`): pins residual, `size_t` is a typedef spelling with no keyword-type
    // evidence, so `u += 3` keeps its twin although `size_t` is a plain scalar.
    let src = "#include <cstddef>\nsize_t cppsz(size_t o) {\n  size_t u = o;\n  u += 3;\n  return o + u;\n}\n";
    assert_eq!(tw("d.cc", src, Language::Cpp, "u"), vec![pair("o", 4)]);
    assert_eq!(supp("d.cc", src, Language::Cpp), vec![3]);
}

#[test]
fn cpp_auto_alias_assignment_keeps_the_twin() {
    // #632 (probe `cppauto`): pins residual, `auto u = o` from a builtin `int` is a value copy but
    // `auto` carries no evidence, so `u = 5` keeps its twin.
    let src = "int cppauto(int o) {\n  auto u = o;\n  u = 5;\n  return o + u;\n}\n";
    assert_eq!(tw("d.cc", src, Language::Cpp, "u"), vec![pair("o", 3)]);
    assert_eq!(supp("d.cc", src, Language::Cpp), vec![2]);
}

#[test]
fn cpp_int_reference_alias_write_keeps_the_twin() {
    // #632 (probe `cppref`): correct, `int& u = o; u = 5` writes `o`.
    let src = "int cppref(int o) {\n  int& u = o;\n  u = 5;\n  return o;\n}\n";
    assert_eq!(tw("d.cc", src, Language::Cpp, "u"), vec![pair("o", 3)]);
    assert_eq!(supp("d.cc", src, Language::Cpp), vec![2]);
}

#[test]
fn cpp_member_alias_with_a_constant_scalar_target_keeps_the_twin() {
    // #632 (survey 3.2, members; leveldb `seed_ -= M`): pins residual, the alias is a member with
    // no visible declaration (unsupported), so the twin of the `static const` scalar `M` stays.
    let src = "struct K { unsigned seed_; static const unsigned M = 7; void a() {\n  seed_ -= M;\n  use(M);\n} };\n";
    assert_eq!(tw("d.cc", src, Language::Cpp, "seed_"), vec![pair("M", 2)]);
    assert!(supp("d.cc", src, Language::Cpp).is_empty());
    assert_eq!(
        label("d.cc", Language::Cpp, src, "M@2->M@3"),
        [ALIAS_UNSTABLE]
    );
}

// ---- Rust ----

#[test]
fn rust_let_some_pattern_names_the_alias_some_and_keeps_the_scrutinee_twin() {
    // #632 (survey 3.2, tokio `time_alt.rs`): pins residual, an extraction defect, not a type
    // fact: `let Some(x) = y else { .. }` is read as an alias spelled `Some` of `y`, and its
    // occurrence keeps a twin of `y` at the `let`.
    let src = "fn r(y: Option<i32>) -> i32 {\n    let Some(x) = y else {\n        return 0;\n    };\n    x + y.unwrap()\n}\n";
    assert_eq!(tw("a.rs", src, Language::Rust, "Some"), vec![pair("y", 2)]);
    assert!(supp("a.rs", src, Language::Rust).is_empty());
}

#[test]
fn rust_borrow_hazard_keeps_a_twin_that_demotes_a_correct_edge_to_killed() {
    // #632 (survey 3.2, twin collapsing a correct edge): pins residual. `let mut u = o` copies a
    // usize, but `&mut o` refuses the removal for every occurrence of the alias, and the kept twin
    // of `o` at line 3 kills `o@2`'s edge to the `&mut o` read at line 5, which is exact in truth.
    let src = "fn f(mut o: usize) -> usize {\n    o = 1;\n    let mut u = o;\n    u = 5;\n    bump(&mut o);\n    o + u\n}\n";
    assert_eq!(
        tw("a.rs", src, Language::Rust, "u"),
        vec![pair("o", 3), pair("o", 4)]
    );
    assert!(supp("a.rs", src, Language::Rust).is_empty());
    assert_eq!(
        label("a.rs", Language::Rust, src, "o@2->o@5"),
        ["NameOnly(Killed { kill_line: 3 })"]
    );
    // Control: the unaffected declaration edge stays exact.
    assert_eq!(label("a.rs", Language::Rust, src, "o@2->o@3"), [EXACT]);
}

// ---- Python ----

#[test]
fn python_integer_alias_augmented_assignment_keeps_a_twin_that_leaves_the_target_edge_exact() {
    // #632 (probe `pint`, row Y-02): pins residual, `u += 3` keeps its twin of `o` although an int
    // is immutable; the twin only feeds the nameonly edge from line 3, and a Python twin never
    // kills, so the real `o@1 -> o@5` edge stays exact.
    let src = "def pint(o):\n    u = o\n    u += 3\n    u = 4\n    return o + u\n";
    assert_eq!(tw("f.py", src, Language::Python, "u"), vec![pair("o", 3)]);
    assert_eq!(supp("f.py", src, Language::Python), vec![2, 4]);
    assert_eq!(
        label("f.py", Language::Python, src, "o@3->o@5"),
        [ALIAS_UNSTABLE]
    );
    assert_eq!(label("f.py", Language::Python, src, "o@1->o@5"), [EXACT]);
}

#[test]
fn python_list_alias_augmented_and_subscript_writes_keep_their_twins() {
    // #632 (probe `plist`, rows Y-02/Y-03): correct for a list, `u += [1]` and `u[0] = 3` mutate
    // the shared object; `u.append(2)` is a call and has no span.
    let src =
        "def plist(o):\n    u = o\n    u += [1]\n    u.append(2)\n    u[0] = 3\n    return o\n";
    assert_eq!(
        tw("f.py", src, Language::Python, "u"),
        vec![pair("o", 3), pair("o[]", 5)]
    );
    assert_eq!(supp("f.py", src, Language::Python), vec![2]);
}

#[test]
fn python_attribute_write_through_a_copy_keeps_the_twin_edge_alias_unstable() {
    // #632 (probe `pattr`, row Y-03): correct for an object, `u.f = o.f + 1` writes `o.f`; the
    // read on the same line keeps its exact edge from the earlier real write.
    let src = "def g(o):\n    o.f = 1\n    u = o\n    u.f = o.f + 1\n    return o.f\n";
    assert_eq!(tw("a.py", src, Language::Python, "u"), vec![pair("o.f", 4)]);
    assert_eq!(
        label("a.py", Language::Python, src, "o.f@2->o.f@4"),
        [EXACT]
    );
    assert_eq!(
        label("a.py", Language::Python, src, "o.f@4->o.f@5"),
        [ALIAS_UNSTABLE]
    );
}

// ---- JS / TS ----

#[test]
fn js_member_update_through_a_copy_keeps_the_declaration_twin_by_rule_e() {
    // #632 (probe `jobj`, rule E): rule E keeps the binding twin of `o` at `let u = o` because the
    // owner writes through `u`. Pins residual for the label: the declaration twin kills the
    // whole-object edge `o@1 -> o@5`, which is graded nameonly/killed at kill line 2.
    let src = "function jobj(o) {\n  let u = o;\n  u.x = 1;\n  u.y++;\n  return o.x;\n}\n";
    assert_eq!(
        tw("g.js", src, Language::JavaScript, "u"),
        vec![pair("o", 2), pair("o.x", 3)]
    );
    assert!(supp("g.js", src, Language::JavaScript).is_empty());
    assert_eq!(
        label("g.js", Language::JavaScript, src, "o@1->o@5"),
        ["NameOnly(Killed { kill_line: 2 })"]
    );
    // The real write through the alias reaches the read (correct).
    assert_eq!(
        label("g.js", Language::JavaScript, src, "o.x@3->o.x@5"),
        [ALIAS_UNSTABLE]
    );
}

#[test]
fn ts_number_alias_scalar_updates_mint_no_twin_and_keep_the_target_edge_exact() {
    // #632 (probe `tint`): correct. An annotated `number` copy is only ever rebound.
    let src = "function tint(o: number): number {\n  let u: number = o;\n  u += 3;\n  u = 4;\n  return o + u;\n}\n";
    assert!(tw("h.ts", src, Language::TypeScript, "u").is_empty());
    assert_eq!(supp("h.ts", src, Language::TypeScript), vec![2, 3, 4]);
    assert_eq!(
        label("h.ts", Language::TypeScript, src, "o@1->o@5"),
        [EXACT]
    );
}

#[test]
fn ts_declaration_twin_demotes_a_correct_edge_when_the_copy_is_written_on_a_later_line() {
    // #632 (survey 3.2, twin collapsing a correct edge): pins residual. `u.f = o.f + 1` reads
    // `o.f` before it writes, so `o.f@2 -> o.f@4` is exact in truth, but the rule-E declaration
    // twin of `o` at line 3 kills it (kill line 3).
    let src = "function g(o: {f: number}) {\n  o.f = 1;\n  const u = o;\n  u.f = o.f + 1;\n  return o.f;\n}\n";
    assert_eq!(
        label("a.ts", Language::TypeScript, src, "o.f@2->o.f@4"),
        ["NameOnly(Killed { kill_line: 3 })"]
    );
}

#[test]
fn ts_twin_keeps_the_real_kill_of_a_write_through_the_alias_off_exact() {
    // #632 (survey 3.2, a twin hiding a real kill in TS): correct answer, `u.x = 1` overwrites
    // `o.x`, so `o.x@2 -> o.x@5` must not be exact. The label is right in kind; its kill line is
    // the declaration twin's (3), not the write (4). Control: a read through the alias is exact
    // and mints no twin.
    let src =
        "function g(o: {x: number}) {\n  o.x = 0;\n  const u = o;\n  u.x = 1;\n  return o.x;\n}\n";
    assert_eq!(
        label("a.ts", Language::TypeScript, src, "o.x@2->o.x@5"),
        ["NameOnly(Killed { kill_line: 3 })"]
    );
    let read =
        "function g(o: {x: number}) {\n  o.x = 0;\n  const u = o;\n  log(u.x);\n  return o.x;\n}\n";
    assert!(tw("a.ts", read, Language::TypeScript, "u").is_empty());
    assert_eq!(
        label("a.ts", Language::TypeScript, read, "o.x@2->o.x@5"),
        [EXACT]
    );
}
