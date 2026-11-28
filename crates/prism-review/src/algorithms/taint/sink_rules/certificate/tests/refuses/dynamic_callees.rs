use super::*;

pub(super) const ROWS: &[Refusal] = &[
    (
        CalleeKindUnlisted,
        (
            Go,
            "package a\nfunc g() { func(_ int) F { return eval }(x) }",
            ("call_expression", "func(_ int) F { return eval }(x)"),
            ("identifier", "eval"),
            ("identifier", "x"),
            Argument(0),
        ),
    ),
    (
        CalleeKindUnlisted,
        (
            Lua,
            "(function(_) return eval end)(x)",
            ("function_call", "(function(_) return eval end)(x)"),
            ("identifier", "eval"),
            ("identifier", "x"),
            Argument(0),
        ),
    ),
    (
        CalleeKindUnlisted,
        (
            Rust,
            "fn g() { (|_| eval)(x); }",
            ("call_expression", "(|_| eval)(x)"),
            ("identifier", "eval"),
            ("identifier", "x"),
            Argument(0),
        ),
    ),
    (
        CalleeKindUnlisted,
        (
            Cpp,
            "void g() { [](int){ return eval; }(x); }",
            ("call_expression", "[](int){ return eval; }(x)"),
            ("identifier", "eval"),
            ("identifier", "x"),
            Argument(0),
        ),
    ),
    (
        CalleeKindUnlisted,
        (
            JavaScript,
            "(_ => eval)(x)",
            ("call_expression", "(_ => eval)(x)"),
            ("identifier", "eval"),
            ("identifier", "x"),
            Argument(0),
        ),
    ),
    (
        CalleeKindUnlisted,
        (
            Python,
            "(eval if True else other)(x)",
            ("call", "(eval if True else other)(x)"),
            ("identifier", "other"),
            ("identifier", "x"),
            Argument(0),
        ),
    ),
    (
        CalleeKindUnlisted,
        (
            Rust,
            "fn g() { (if true {eval} else {other})(x); }",
            ("call_expression", "(if true {eval} else {other})(x)"),
            ("identifier", "other"),
            ("identifier", "x"),
            Argument(0),
        ),
    ),
    (
        CalleeKindUnlisted,
        (
            JavaScript,
            "([eval])(x)",
            ("call_expression", "([eval])(x)"),
            ("identifier", "eval"),
            ("identifier", "x"),
            Argument(0),
        ),
    ),
    (
        CalleeKindUnlisted,
        (
            Cpp,
            "void g() { Builder{eval}(x); }",
            ("call_expression", "Builder{eval}(x)"),
            ("identifier", "eval"),
            ("identifier", "x"),
            Argument(0),
        ),
    ),
    (
        CalleeKindUnlisted,
        (
            Python,
            "async def g():\n    (await eval)(x)",
            ("call", "(await eval)(x)"),
            ("identifier", "eval"),
            ("identifier", "x"),
            Argument(0),
        ),
    ),
    (
        CalleeKindUnlisted,
        (
            Rust,
            "async fn g() { (eval.await)(x); }",
            ("call_expression", "(eval.await)(x)"),
            ("identifier", "eval"),
            ("identifier", "x"),
            Argument(0),
        ),
    ),
    (
        CalleeKindUnlisted,
        (
            Go,
            "package a\nfunc g() { (<-eval)(x) }",
            ("call_expression", "(<-eval)(x)"),
            ("identifier", "eval"),
            ("identifier", "x"),
            Argument(0),
        ),
    ),
    (
        CalleeKindUnlisted,
        (
            Python,
            "def g():\n    (yield eval)(x)",
            ("call", "(yield eval)(x)"),
            ("identifier", "eval"),
            ("identifier", "x"),
            Argument(0),
        ),
    ),
    (
        CalleeKindUnlisted,
        (
            Cpp,
            "void g() { p->eval<T>(x); }",
            ("call_expression", "p->eval<T>(x)"),
            ("type_identifier", "T"),
            ("identifier", "x"),
            Argument(0),
        ),
    ),
    (
        CalleeKindUnlisted,
        (
            Cpp,
            "void g() { p->template eval<T>(x); }",
            ("call_expression", "p->template eval<T>(x)"),
            ("type_identifier", "T"),
            ("identifier", "x"),
            Argument(0),
        ),
    ),
    (
        CalleeKindUnlisted,
        (
            JavaScript,
            "obj?.get().eval(x)",
            ("call_expression", "obj?.get().eval(x)"),
            ("property_identifier", "eval"),
            ("identifier", "x"),
            Argument(0),
        ),
    ),
    (
        CalleeTargetUnlisted,
        (
            Python,
            "obj[\"eval\"](x)",
            ("call", "obj[\"eval\"](x)"),
            ("subscript", "obj[\"eval\"]"),
            ("identifier", "x"),
            Argument(0),
        ),
    ),
    (
        CalleeTargetUnlisted,
        (
            TypeScript,
            "obj[\"eval\"](x)",
            ("call_expression", "obj[\"eval\"](x)"),
            ("subscript_expression", "obj[\"eval\"]"),
            ("identifier", "x"),
            Argument(0),
        ),
    ),
    (
        SubscriptKeyNotPlainLiteral,
        (
            Python,
            "a[eval](x)",
            ("call", "a[eval](x)"),
            ("identifier", "eval"),
            ("identifier", "x"),
            Argument(0),
        ),
    ),
    (
        SubscriptKeyNotPlainLiteral,
        (
            JavaScript,
            "a[eval](x)",
            ("call_expression", "a[eval](x)"),
            ("identifier", "eval"),
            ("identifier", "x"),
            Argument(0),
        ),
    ),
];
