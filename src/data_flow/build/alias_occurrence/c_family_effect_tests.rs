//! C and C++ producer effect of the alias-twin policy: a binding write mints no target def or
//! edge, storage writes and unsupported forms keep theirs, and every refusal keeps the twins of
//! its target with a positive control that removes them. These tests use only the producer and
//! the policy, so they also run (and fail) on a tree without the C-family classifier.
use super::test_support::{refs, suppressed_lines, twins};
use crate::languages::Language;

const C_HEAD: &str = "struct S { int f; };\nvoid use(void *); int bar(void *);\nvoid t(struct S *o, struct S *q, int n, char *buf0) {\n";
const CPP_HEAD: &str = "struct S { int f; };\nvoid use(void *); void bump(int &);\nvoid t(S &o, S *op, int m, char *buf0) {\n";

fn csrc(body: &str) -> String {
    format!("{C_HEAD}{body}\n}}\n")
}
fn ppsrc(body: &str) -> String {
    format!("{CPP_HEAD}{body}\n}}\n")
}
/// Suppressed occurrence lines, relative to the first body line (1-based); the head is three lines.
fn lc(body: &str) -> Vec<usize> {
    suppressed_lines("a.c", &csrc(body), Language::C)
        .into_iter()
        .map(|l| l - 3)
        .collect()
}
fn lp(body: &str) -> Vec<usize> {
    suppressed_lines("a.cc", &ppsrc(body), Language::Cpp)
        .into_iter()
        .map(|l| l - 3)
        .collect()
}

#[test]
fn c_binding_writes_mint_no_target_twin() {
    // C-04 declaration, C-01 assignment, C-02 compound (a pointer step).
    let body = "struct S *u = o;\nu = o;\nu += 1;\nuse(o);";
    assert_eq!(lc(body), vec![1, 2, 3]);
    let r = refs("a.c", &csrc(body), Language::C);
    assert!(twins(&r, "u").is_empty(), "{:?}", twins(&r, "u"));
    assert!(r.defs.values().flatten().any(|d| d.path.base == "u"));
    // A C-03 scalar declaration and reassignment.
    assert_eq!(lc("int k = n;\nk = 2;"), vec![1, 2]);
}

#[test]
fn c_storage_writes_keep_their_twins() {
    let body = "struct S *u = o;\nu->f = 1;\n(*u).f = 2;\n*u = *q;\nuse(o);";
    assert_eq!(lc(body), vec![1]);
    let r = refs("a.c", &csrc(body), Language::C);
    let t = twins(&r, "u");
    assert!(t.iter().any(|(p, l)| p == "o.f" && *l == 5), "{t:?}");
    assert!(t.iter().any(|(p, l)| p == "o" && *l == 7), "{t:?}");
    // `*p++ = v` (C-10) stays: the destination is storage.
    let step = "char *bp = buf0;\n*bp++ = 'a';\n*++bp = 'b';\n*bp++ += 1;";
    assert_eq!(lc(step), vec![1]);
    // A subscript through a pointer (C-08).
    assert_eq!(lc("int *w = buf0;\nw[1] = 2;"), vec![1]);
}

#[test]
fn c_unsupported_forms_keep_their_twin() {
    // X-READ: a call argument and an initialiser value are reads recorded as lvalue spans.
    let read = "struct S *u = o;\nint r = bar(u);\nuse(o);";
    assert_eq!(lc(read), vec![1]);
    let r = refs("a.c", &csrc(read), Language::C);
    assert!(
        twins(&r, "u").iter().any(|(p, l)| p == "o" && *l == 5),
        "{:?}",
        twins(&r, "u")
    );
    // C-07: a value copy's field write is unsupported without type facts.
    let value = "struct S v = sv;\nv.f = 3;";
    assert_eq!(lc(value), vec![1]);
}

#[test]
fn field_label_enum_macro_and_type_tokens_refuse_the_removal() {
    let base = "struct S *u = o;\nu = o;";
    assert_eq!(lc(base), vec![1, 2]);
    for (what, extra) in [
        ("field after ->", "n = q->o;"),
        ("field after .", "n = x.o;"),
        ("designated initialiser", "struct S z = { .o = 1 };"),
        ("goto label", "goto o;\no: ;"),
        ("macro parameter", "#define M(o) ((o) + 1)"),
        ("macro name", "#define o 7"),
        ("local enum constant", "enum { o = 3 };"),
        ("type name", "n = sizeof(struct o);\no *z = 0;"),
    ] {
        assert_eq!(
            lc(&format!("{base}\n{extra}")),
            Vec::<usize>::new(),
            "{what}"
        );
    }
    // The target spelling alone matters: another name leaves the removal.
    assert_eq!(lc(&format!("{base}\nn = q->p;")), vec![1, 2]);
}

#[test]
fn file_level_enum_and_macro_names_refuse_the_removal() {
    for decl in ["enum { o = 3 };\n", "#define o 7\n"] {
        let src = format!("{decl}{}", csrc("struct S *u = o;\nu = o;"));
        let got = suppressed_lines("a.c", &src, Language::C);
        assert!(got.is_empty(), "{decl:?}: {got:?}");
    }
    let src = format!("enum {{ p = 3 }};\n{}", csrc("struct S *u = o;\nu = o;"));
    assert_eq!(suppressed_lines("a.c", &src, Language::C).len(), 2);
}

#[test]
fn taking_the_target_address_in_any_form_refuses_the_removal() {
    let base = "struct S *u = o;\nu = o;";
    for (what, extra) in [
        ("&o", "int *p1 = (int *)&o;"),
        ("&o.f", "int *p2 = &o.f;"),
        ("&o[i]", "int *p3 = (int *)&o[1];"),
        ("&(o)", "int *p4 = (int *)&(o);"),
        ("&o->f", "int *p5 = &o->f;"),
    ] {
        assert_eq!(
            lc(&format!("{base}\n{extra}")),
            Vec::<usize>::new(),
            "{what}"
        );
    }
    // `&u` is the alias's own address, not the target's.
    assert_eq!(lc(&format!("{base}\nstruct S **pp = &u;")), vec![1, 2]);
}

#[test]
fn an_array_target_refuses_the_removal() {
    let local = "int a[4];\nint *u = a;\nu = &n;\nuse(a);";
    assert_eq!(
        suppressed_lines("a.c", &csrc(local), Language::C),
        Vec::<usize>::new()
    );
    let param = "void t(int a[], int *x) {\n int *u = a;\n u = a;\n}\n";
    assert!(suppressed_lines("a.c", param, Language::C).is_empty());
    // Control: the same shape with a pointer target.
    let control = "void t(int *a, int *x) {\n int *u = a;\n u = a;\n}\n";
    assert_eq!(suppressed_lines("a.c", control, Language::C), vec![2, 3]);
}

#[test]
fn a_decaying_undeclared_target_passed_to_a_call_refuses_the_removal() {
    let body = "int *u = g;\nu = &n;\nmemcpy(g, buf0, 4);";
    assert_eq!(lc(body), Vec::<usize>::new());
    // Control: a declared (local, non-array) target passed by value cannot be written.
    let declared = "int *g = &n;\nint *u = g;\nu = &n;\nuse(g);";
    assert_eq!(lc(declared), vec![2, 3]);
}

#[test]
fn macro_arguments_refuse_the_removal() {
    let base = "struct S *u = o;\nu = o;";
    for (what, call) in [
        ("all-caps macro", "SET_FIELD(o, 1);"),
        ("all-caps macro, nested", "n = MAX(o->f + 1, 2);"),
    ] {
        assert_eq!(
            lc(&format!("{base}\n{call}")),
            Vec::<usize>::new(),
            "{what}"
        );
    }
    // A lowercase macro defined in the file counts too.
    let src = format!(
        "#define set_it(x) ((x)->f = 1)\n{}",
        csrc("struct S *u = o;\nu = o;\nset_it(o);")
    );
    assert!(suppressed_lines("a.c", &src, Language::C).is_empty());
    // Controls: an ordinary function call with the target by value, and a macro without it.
    assert_eq!(lc(&format!("{base}\nbar(o);")), vec![1, 2]);
    assert_eq!(lc(&format!("{base}\nSET_FIELD(q, 1);")), vec![1, 2]);
}

#[test]
fn an_update_of_the_target_refuses_the_removal() {
    // The DFG records no def for `o++`: the twin's kill hid the resulting wrong `exact`.
    let base = "struct S *u = o;\nu = o;";
    for (what, stmt) in [("o++", "o++;"), ("--o", "--o;"), ("o->f++", "o->f++;")] {
        assert_eq!(
            lc(&format!("{base}\n{stmt}")),
            Vec::<usize>::new(),
            "{what}"
        );
    }
    assert_eq!(lc(&format!("{base}\nn++;")), vec![1, 2]);
}

#[test]
fn a_parse_error_keeps_the_twins_it_touches() {
    // An unexpanded macro leaves an ERROR node. An identifier of the target inside it refuses the
    // removal for the whole owner; an occurrence whose own statement holds the error is unsupported.
    let base = "struct S *u = o;\nu = o;";
    assert_eq!(lc(base), vec![1, 2]);
    assert!(lc(&format!("{base}\nfoo(o, ;")).is_empty());
    // An error elsewhere that holds no identifier of the target leaves the other statements alone.
    assert_eq!(lc(&format!("{base}\nn = @ q;")), vec![1, 2]);
    // The occurrence's own statement holds the error: kept.
    assert_eq!(lc("struct S *u = o;\nu = o @ ;"), vec![1]);
}

#[test]
fn cpp_reference_writes_keep_their_twin_and_pointer_and_builtin_binding_writes_go() {
    // `T& r = o` initialises (C-03); `r = v` writes the referent (C-01 reference): kept.
    let r = "S &r = o;\nr = o;\nr.f = 1;";
    assert_eq!(lp(r), vec![1]);
    // Pointer and builtin-keyword types are binding writes.
    assert_eq!(lp("S *p = op;\np = op;\np->f = 2;"), vec![1, 2]);
    assert_eq!(lp("int k = m;\nk = 7;\nk += 2;"), vec![1, 2, 3]);
    assert_eq!(lp("unsigned long ul = m;\nul = 3;"), vec![1, 2]);
}

#[test]
fn cpp_names_without_positive_evidence_keep_their_twin() {
    assert_eq!(
        lp("using Ref = int&;\nRef a = m;\na = 1;\na += 1;"),
        vec![2],
        "a typedef'd reference: the declaration only"
    );
    assert_eq!(lp("using size_t = int&;\nsize_t a = m;\na = 7;"), vec![2]);
    // Direct pins.
    assert_eq!(
        lp("size_t sa = m;\nsa = 7;"),
        vec![1],
        "unresolved size_t: declaration only"
    );
    assert_eq!(lp("auto z = op;\nz = op;"), vec![1]);
    assert_eq!(lp("S v = o;\nv = *op;"), vec![1]);
    assert_eq!(lp("g = m;"), Vec::<usize>::new());
    // The declaration is a binding write; the assignment in the `for` header has no
    // keyword-only prefix evidence (the prefix starts at the `for`) and keeps its twin.
    assert_eq!(lp("for (int i = m; i < 3; i = m) {}"), vec![1]);
    // Control: the keyword-only prefix without the `for`.
    assert_eq!(lp("int i = m;\ni = i + 1;"), vec![1, 2]);
}

#[test]
fn cpp_call_reference_method_stream_and_lambda_hazards_refuse_the_removal() {
    let base = "int k = m;\nk = 7;";
    assert_eq!(lp(base), vec![1, 2]);
    // Precise pins on the target `m`.
    assert_eq!(lp(&format!("{base}\nbump(m);")), Vec::<usize>::new());
    assert_eq!(lp(&format!("{base}\nstd::cin >> m;")), Vec::<usize>::new());
    assert_eq!(
        lp(&format!("{base}\nauto l = [&]() {{ m = 3; }};")),
        Vec::<usize>::new()
    );
    assert_eq!(
        lp(&format!("{base}\nauto l = [&m]() {{ m = 3; }};")),
        Vec::<usize>::new()
    );
    // A reference bound to a plain name is paired by the alias map (its writes keep their twin);
    // one bound to part of the target is a hidden write.
    assert_eq!(lp(&format!("{base}\nint &w = m;")), vec![1, 2, 3]);
    let hidden = "S *p = op;\np = op;\nint &w = op->f;";
    assert!(lp(hidden).is_empty());
    assert_eq!(lp("S *p = op;\np = op;\nint &w = m;"), vec![1, 2, 3]);
    // Controls: an argument of another name, a by-value capture.
    assert_eq!(lp(&format!("{base}\nbump(op);")), vec![1, 2]);
    assert_eq!(
        lp(&format!("{base}\nauto l = [=]() {{ return m; }};")).len(),
        2
    );
}

#[test]
fn cpp_qualified_and_template_names_refuse_the_removal() {
    let base = "int k = m;\nk = 7;";
    assert_eq!(
        lp(&format!("{base}\nstd::vector<int> x = ns::m;")),
        Vec::<usize>::new()
    );
    assert_eq!(
        lp(&format!("{base}\nstd::vector<m> x;")),
        Vec::<usize>::new()
    );
    assert_eq!(
        lp(&format!("{base}\nstd::vector<int> x = ns::p;")),
        vec![1, 2]
    );
}

#[test]
fn a_dot_h_header_takes_the_cpp_evidence_rules() {
    // prism parses `.h` as C, but a header in a C++ tree holds C++: an assignment to a name
    // with no keyword or pointer evidence keeps its twin there, and is a binding write in `.c`.
    let body = "T u = o;\nu = o;";
    let src = format!("void t(T o) {{\n{body}\n}}\n");
    assert_eq!(suppressed_lines("a.c", &src, Language::C), vec![2, 3]);
    assert_eq!(suppressed_lines("a.h", &src, Language::C), vec![2]);
    // Positive control: keyword evidence still removes in a header.
    let kw = "void t(int o) {\nint u = o;\nu = o;\n}\n";
    assert_eq!(suppressed_lines("a.h", kw, Language::C), vec![2, 3]);
}

#[test]
fn a_bare_target_argument_of_an_unproven_callee_refuses_the_removal() {
    // `Py_CLEAR(o)` and `atomicGet(v, o)` are header macros that assign their argument; the file
    // declares neither, and neither is all-caps, so only the callee's absence from the file's
    // function declarations marks them.
    let base = "struct S *u = o;\nu = o;";
    assert_eq!(lc(&format!("{base}\nPy_CLEAR(o);")), Vec::<usize>::new());
    assert_eq!(
        lc(&format!("{base}\natomicGet(n, o);")),
        Vec::<usize>::new()
    );
    // Controls: a function the file prototypes (`bar`), a library name (`memcpy`), a nested use,
    // and a function pointer call do not refuse.
    assert_eq!(lc(&format!("{base}\nbar(o);")), vec![1, 2]);
    assert_eq!(lc(&format!("{base}\nmemmove(buf0, o, 4);")), vec![1, 2]);
    assert_eq!(lc(&format!("{base}\nPy_CLEAR(o->f);")), vec![1, 2]);
    assert_eq!(lc(&format!("{base}\nq->cb(o);")), vec![1, 2]);
}

#[test]
fn cpp_pointer_arguments_refuse_like_any_other_argument() {
    // A bare pointer passed to a C++ call is no longer exempt: a `T*&` parameter can rebind it and
    // the exemption could not tell scopes apart (an expired inner `int *o;`).
    let base = "S *p = op;\np = op;";
    assert_eq!(lp(base), vec![1, 2]);
    assert_eq!(
        lp(&format!("{base}\nuse(op);")),
        Vec::<usize>::new(),
        "bare pointer argument"
    );
    assert_eq!(
        lp(&format!("{base}\nbump(op->f);")),
        Vec::<usize>::new(),
        "member chain"
    );
    // C keeps passing a declared pointer by value: no refusal.
    assert_eq!(lc("struct S *u = o;\nu = o;\nuse(o);"), vec![1, 2]);
    // `ptr`: an expired inner pointer declaration must not let a scalar reference argument through.
    let ptr = "void bump(int&);\nint f(int o) {\n    { int *o; }\n    int u = o;\n    u = 0;\n    bump(o);\n    return o;\n}\n";
    assert!(
        suppressed_lines("a.cc", ptr, Language::Cpp).is_empty(),
        "ptr"
    );
    // Control `ptr_ctl`: without the inner declaration the refusal fires the same way.
    let ptr_ctl = "void bump(int&);\nint f(int o) {\n    int u = o;\n    u = 0;\n    bump(o);\n    return o;\n}\n";
    assert!(
        suppressed_lines("a.cc", ptr_ctl, Language::Cpp).is_empty(),
        "ptr_ctl"
    );
}

#[test]
fn prototype_and_function_type_parameters_are_not_visible_evidence() {
    // `proto`: the prototype's `int u` is in scope only inside its own declarator; it must not make
    // the reference `u` an `int` binding write.
    let proto = "int o;\nint& u = o;\nint f() {\n    o = 1;\n    void decoy(int u);\n    u = o;\n    u = 7;\n    return o;\n}\n";
    let got = suppressed_lines("a.cc", proto, Language::Cpp);
    assert!(!got.contains(&6) && !got.contains(&7), "{got:?}");
    // `proto_ctl`: the parameter renamed behaves the same.
    let ctl = proto.replace("decoy(int u)", "decoy(int w)");
    let got = suppressed_lines("a.cc", &ctl, Language::Cpp);
    assert!(!got.contains(&6) && !got.contains(&7), "{got:?}");
    // A function-pointer type's parameter is no evidence either.
    let fp = "int o;\nint& u = o;\nint f() {\n    o = 1;\n    void (*cb)(int u);\n    u = o;\n    u = 7;\n    return o;\n}\n";
    let got = suppressed_lines("a.cc", fp, Language::Cpp);
    assert!(!got.contains(&6) && !got.contains(&7), "{got:?}");
    // Positive control: the owner's own parameter is evidence.
    let own = "int f(int u, int o) {\n    u = o;\n    u = 7;\n    return o;\n}\n";
    let got = suppressed_lines("a.cc", own, Language::Cpp);
    assert!(got.contains(&2) && got.contains(&3), "{got:?}");
}

#[test]
fn a_hazard_on_a_reference_alias_is_a_hazard_on_its_target() {
    // `++r` through `int& r = o;` writes `o`, and `clear((o))` is a parenthesised macro argument.
    let reference = "o = 1;\nint u = o;\nint& r = o;\n++r;\nu = o;";
    assert!(lp(reference).is_empty(), "{:?}", lp(reference));
    // Control: a copy, not a reference, leaves the removal.
    let copy = "o = 1;\nint u = o;\nint r = o;\n++r;\nu = o;";
    assert!(!lp(copy).is_empty());
    let paren = "int u = o;\no = 1;\nclear((o));\nu = o;";
    assert!(lc(paren).is_empty(), "{:?}", lc(paren));
    let paren_ctl = "int u = o;\no = 1;\nclear((q));\nu = o;";
    assert!(!lc(paren_ctl).is_empty());
}

fn cpp_lines(src: &str) -> Vec<usize> {
    suppressed_lines("a.cc", src, Language::Cpp)
}

/// `int f(..) { <decl>; int u = o; u = 5; <write>; return o; }` with the hazard lines inserted.
fn hz(decl: &str, write: &str) -> String {
    format!("int f(int o, int q, int c) {{\n    {decl}\n    int u = o;\n    u = 5;\n    {write}\n    return o;\n}}\n")
}

#[test]
fn a_returned_function_types_parameters_are_not_the_definitions() {
    // `ret`: `(int u)` belongs to the returned pointer-to-function type.
    let ret = "int o;\nint& u = o;\n\nint (*f())(int u) {\n    o = 1;\n    u = o;\n    u = 7;\n    int seen = o;\n    return nullptr;\n}\n";
    let got = cpp_lines(ret);
    assert!(!got.contains(&6) && !got.contains(&7), "{got:?}");
    // `ret_ctl`: the parameter renamed behaves the same (the global reference stays unsupported).
    let got = cpp_lines(&ret.replace("(int u)", "(int w)"));
    assert!(!got.contains(&6) && !got.contains(&7), "{got:?}");
    // `p1_fnret`: the definition's own parameter list sits in the inner declarator, the returned type's does not.
    let fn_ret = "int o;\nint& u = o;\nint g(int);\n\nint (*h(int a))(int u) {\n    o = 1;\n    u = o;\n    u = 7;\n    if (o) return g;\n    return g;\n}\n";
    let got = cpp_lines(fn_ret);
    assert!(!got.contains(&7) && !got.contains(&8), "{got:?}");
    // Positive control: a definition's own parameter, even beside a returned function type, is evidence.
    let own =
        "int g(int);\nint o;\nint (*h(int u))(int v) {\n    u = o;\n    u = 7;\n    return g;\n}\n";
    let got = cpp_lines(own);
    assert!(got.contains(&4) && got.contains(&5), "{got:?}");
}

#[test]
fn reference_initialiser_forms_and_other_hazards_keep_the_twins() {
    // Controls first: a plain copy leaves the formation twins removable.
    let ctl = hz("int r = o;", "r = 9;");
    assert!(!cpp_lines(&ctl).is_empty());
    for (what, decl, write) in [
        ("brace-initialised reference", "int& r{o};", "r = 9;"),
        (
            "conditional reference initialiser",
            "int& r = c ? o : q;",
            "r = 9;",
        ),
        (
            "reference declarator in a pointer chain",
            "int *&r = o;",
            "r = 9;",
        ),
        (
            "reference init-capture",
            "auto l = [&x = o]() { x = 9; };",
            "l();",
        ),
        (
            "address of a conditional",
            "int *p = &(c ? o : q);",
            "*p = 9;",
        ),
    ] {
        let got = cpp_lines(&hz(decl, write));
        assert!(got.is_empty(), "{what}: {got:?}");
    }
    // Controls of the same shapes without the reference.
    for (decl, write) in [
        ("auto l = [x = o]() { return x; };", "l();"),
        ("int *p = (c ? &q : &q);", "*p = 9;"),
        ("int r = c ? o : q;", "r = 9;"),
    ] {
        assert!(!cpp_lines(&hz(decl, write)).is_empty(), "{decl}");
    }
}

#[test]
fn a_reference_declared_outside_the_owner_carries_its_hazard_to_the_target() {
    let global = "int o;\nint& r = o;\nint f() {\n    o = 1;\n    int u = o;\n    u = 5;\n    ++r;\n    return o;\n}\n";
    assert!(cpp_lines(global).is_empty(), "{:?}", cpp_lines(global));
    let member = "struct S {\n    int o;\n    int& r = o;\n    int f() {\n        o = 1;\n        int u = o;\n        u = 5;\n        ++r;\n        return o;\n    }\n};\n";
    assert!(cpp_lines(member).is_empty(), "{:?}", cpp_lines(member));
    // Control: the update does not go through the reference.
    let ctl = "int o;\nint& r = o;\nint f() {\n    o = 1;\n    int u = o;\n    u = 5;\n    r;\n    return o;\n}\n";
    assert!(!cpp_lines(ctl).is_empty());
}

#[test]
fn a_reference_chain_carries_the_hazard_to_the_target() {
    // `s` refers to `r`, which refers to `o`: the fixpoint reaches `o` from `++s`.
    let chain = "int f(int o) {\n    int u;\n    u = o;\n    u = 5;\n    int& r = o;\n    int& s = r;\n    ++s;\n    return o;\n}\n";
    assert!(cpp_lines(chain).is_empty(), "{:?}", cpp_lines(chain));
    let ctl = chain.replace("++s;", "s;");
    assert!(!cpp_lines(&ctl).is_empty());
}
