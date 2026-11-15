//! #829 DESIGN-v2 / PLAN-v2 task 1 and 1a: the rows of the amended design. What a row asserts is
//! the oracle's frozen verdict (`c_prune_oracle_rows`, `scripts/dfg_prune/XARM.md`). Sections:
//! the inline rows of cases 35 to 40 (bodysplit, bodyfwd, deferred, shield_nested, outer_shield
//! and its identity-wrapper twin, and the shield_safe guide); `hull2` (an opener and a closer
//! under `#else` branches); the hidden-opener rows (`aspan` and its C++ twin, `aspan_sib`, a
//! control with the macro spelled after the owner, and the pinned hole `aspan3`; each declares
//! its translation unit); the 17 review-repair rows of PLAN-v2 task 1a (an outside
//! `external829.h` is the oracle prelude only, never part of the indexed repository) with their
//! parser-off controls, and the parser pins. Macro inputs are Rust strings: the repository
//! loader would read a `#define` under `tests/fixtures` into the census of prism's own tree.
use super::c_prune_brackets_r3_support::{guard_pair, kept_guard, pruned_guard, Batch};
use super::c_prune_brackets_tests::check_row;
use super::c_prune_oracle_rows::{declare_in_model, declare_tu};

macro_rules! inline_rows {
    ($file:expr => $($name:ident $id:expr),+ $(,)?) => {
        $(#[test] fn $name() { check_row($file, $id); })+
    };
}

// ---- inline cases 35 to 40 (the census of each case is its own) ----
inline_rows!("35_v2_bodysplit.c" => row_v2_bodysplit_c "bs1");
inline_rows!("35_v2_bodysplit.cc" => row_v2_bodysplit_cc "bs1");
inline_rows!("36_v2_bodyfwd.c" => row_v2_bodyfwd_c "bf1");
inline_rows!("36_v2_bodyfwd.cc" => row_v2_bodyfwd_cc "bf1");
inline_rows!("37_v2_deferred.c" => row_v2_deferred_c "df1");
inline_rows!("37_v2_deferred.cc" => row_v2_deferred_cc "df1");
inline_rows!("38_v2_shield_nested.c" => row_v2_shield_nested_c "sn1");
inline_rows!("38_v2_shield_nested.cc" => row_v2_shield_nested_cc "sn1");
inline_rows!("39_v2_outer_shield.c" => row_v2_outer_shield "os1", row_v2_inner_shield "os2");
// Guide: the oracle says the commas stay inside unknown calls' parentheses (CONFIRMED).
inline_rows!("40_v2_shield_safe.c" => row_v2_shield_safe "ss1");

// ---- hull2: an opener and a closer under `#else` branches ----
const HULL2_C: &str = r####"#define W3(p, q, r) p; r
#if defined(NEVER_DEFINED_829)
#else
W3(
#endif
int dummy829;
int f(int c) {
    int x = 41;
    int b;
    int a = 0, d;
    x = 2;
    d = 0, a = 1;
    return x + a;
}
#if defined(NEVER_DEFINED_829)
#else
)
#endif
int main(void) { return f(0); }
"####;
const HULL2_CC: &str = r####"#define W3(p, q, r) p; r
#if defined(NEVER_DEFINED_829)
#else
W3(
#endif
int dummy829;
int f(int c) {
    int x = 41;
    int b = 0;
    int a = 0, d;
    x = 2;
    int e = 0, b;
    return x + 1;
}
#if defined(NEVER_DEFINED_829)
#else
)
#endif
int main(void) { return f(0); }
"####;

/// The opener `W3(` and its closer sit under `#else` branches of two conditionals that are never
/// taken together with an `#if` branch: the interval hull keeps the owner exposed. C and C++.
#[test]
fn row_v2_hull2_c() {
    kept_guard("hull2 c", "a.c", HULL2_C, Some("f"));
}

#[test]
fn row_v2_hull2_cc() {
    kept_guard("hull2 cc", "a.cc", HULL2_CC, Some("f"));
}

// ---- hidden opener (OD-C): the two-file form of A-span ----

const ASPAN_DEFS_H: &str = r####"#define W3(p, q, r) p; r
#define BEGIN_IMPL W3(
"####;
const ASPAN_IMPL_C: &str = r####"BEGIN_IMPL
int dummy829;
int f(int c) {
    int x = 41;
    int b;
    int a = 0, d;
    x = 2;
    d = 0, a = 1;
    return x + a;
}
"####;
const ASPAN_UNITY_C: &str = r####"#include "defs.h"
#include "impl.c"
)
int main(void) { return f(0); }
"####;
const ASPAN_CC_DEFS_H: &str = r####"#define W3(p, q, r) p; r
#define BEGIN_IMPL W3(
"####;
const ASPAN_CC_IMPL_CC: &str = r####"BEGIN_IMPL
int dummy829;
int f(int c) {
    int x = 41;
    int b = 0;
    int a = 0, d;
    x = 2;
    int e = 0, b;
    return x + 1;
}
"####;
const ASPAN_CC_UNITY_CC: &str = r####"#include "defs.h"
#include "impl.cc"
)
int main(void) { return f(0); }
"####;
const SIB_DEFS_H: &str = r####"#define W4(o, p, q, r) o p; r
#define OPEN_IMPL W4(
"####;
const SIB_IMPL_C: &str = r####"int g(int c) {
    return c OPEN_IMPL ; }
, int f(int c) {
    int x = 41;
    int b;
    int a = 0, d;
    x = 2;
    d = 0, a = 1;
    return x + a;
}
"####;
const SIB_UNITY_C: &str = r####"#include "defs.h"
#include "impl.c"
)
int main(void) { return f(0) + g(0); }
"####;
const ASPAN3_DEFS_H: &str = r####"#define W3(p, q, r) p; r
"####;
const ASPAN3_IMPL_C: &str = r####"#include "open.h"
int dummy829;
int f(int c) {
    int x = 41;
    int b;
    int a = 0, d;
    x = 2;
    d = 0, a = 1;
    return x + a;
}
"####;
const ASPAN3_OPEN_H: &str = r####"int before829;
W3(
"####;
const ASPAN3_UNITY_C: &str = r####"#include "defs.h"
#include "impl.c"
)
int main(void) { return f(0); }
"####;

const CTRL_IMPL_C: &str = r####"#include "defs.h"
int dummy829;
int dummy830;
int f(int c) {
    int x = 41;
    int b;
    int a = 0, d;
    x = 2;
    d = 0, a = 1;
    return x + a;
}
BEGIN_IMPL
int tail1;
, int tail2;
, int tail3;
)
"####;

/// `BEGIN_IMPL` is `W3(` in `defs.h`; `impl.c` spells it at file scope before `f`, and the `)`
/// that closes the list is in `unity.c`, which includes both. Runtime receipt: `unity` returns
/// 42 exactly when the first def of `x` is live (the compiled `f` drops `x = 2`).
#[test]
fn row_v2_aspan_c() {
    let (from, to) = guard_pair(ASPAN_IMPL_C);
    declare_tu("unity.c");
    let mut b = Batch::new();
    b.kept(
        "aspan c",
        &[
            ("defs.h", ASPAN_DEFS_H),
            ("impl.c", ASPAN_IMPL_C),
            ("unity.c", ASPAN_UNITY_C),
        ],
        "impl.c",
        &[],
        Some("f"),
        from,
        to,
    );
    b.finish();
}

#[test]
fn row_v2_aspan_cc() {
    let (from, to) = guard_pair(ASPAN_CC_IMPL_CC);
    declare_tu("unity.cc");
    let mut b = Batch::new();
    b.kept(
        "aspan cc",
        &[
            ("defs.h", ASPAN_CC_DEFS_H),
            ("impl.cc", ASPAN_CC_IMPL_CC),
            ("unity.cc", ASPAN_CC_UNITY_CC),
        ],
        "impl.cc",
        &[],
        Some("f"),
        from,
        to,
    );
    b.finish();
}

/// The opener is spelled inside a sibling owner (`g`) before `f`.
#[test]
fn row_v2_aspan_sib() {
    let (from, to) = guard_pair(SIB_IMPL_C);
    declare_tu("unity.c");
    let mut b = Batch::new();
    b.kept(
        "aspan_sib",
        &[
            ("defs.h", SIB_DEFS_H),
            ("impl.c", SIB_IMPL_C),
            ("unity.c", SIB_UNITY_C),
        ],
        "impl.c",
        &[],
        Some("f"),
        from,
        to,
    );
    b.finish();
}

/// Control: the macro is spelled only AFTER `f`, in a program that closes its own list, so no
/// argument list it opens can hold `f` (CONFIRMED by the oracle: a guide, the pair may be
/// dropped). The literal `)` after `f` makes the INTERVAL half of G10 refuse it, so this row
/// does not isolate the hidden-opener half (the unit tests of `c_file_parens` do).
#[test]
fn row_v2_aspan_control_spelled_after() {
    let (from, to) = guard_pair(CTRL_IMPL_C);
    let mut b = Batch::new();
    b.pruned(
        "aspan control",
        &[("defs.h", ASPAN_DEFS_H), ("impl.c", CTRL_IMPL_C)],
        "impl.c",
        &[],
        Some("f"),
        from,
        to,
    );
    b.finish();
}

/// Pinned hole (A-span as narrowed): `open.h` ends with a literal `W3(` that `impl.c` includes
/// and `unity.c` closes; no test on `impl.c` sees the opener. Today's drop is asserted.
#[test]
fn row_v2_aspan3_pinned() {
    let (from, to) = guard_pair(ASPAN3_IMPL_C);
    declare_tu("unity.c");
    let mut b = Batch::new();
    b.kept(
        "aspan3",
        &[
            ("defs.h", ASPAN3_DEFS_H),
            ("impl.c", ASPAN3_IMPL_C),
            ("open.h", ASPAN3_OPEN_H),
            ("unity.c", ASPAN3_UNITY_C),
        ],
        "impl.c",
        &[],
        Some("f"),
        from,
        to,
    );
    b.finish();
}

// ---- review-repair rows (PLAN-v2 task 1a): sol's erasure family and the in-model siblings ----
// `external829.h` is the ORACLE prelude of the row (`-I{PRELUDE}`), never an indexed file.

const ERASE_SRC: &str = r####"#include <external829.h>
#define ID(v) v
#define sizeof(p,q,r) q if(r) p

int probe(int c) {
    int x = 41;
    int b = 0;
    {
        ID(sizeof E({ int a, b; }, c));
        x = 2;
        ID(sizeof E({ int a, b; }, 1));
        return x;
    }
}
"####;
const ERASE_HDR: &str = r####"#define E
"####;

const ERASE_ALIAS1_SRC: &str = r####"#include <external829.h>
#define ID(v) v
#define sizeof(p,q,r) q if(r) p

int probe(int c) {
    int x = 41;
    int b = 0;
    {
        ID(sizeof E({ int a, b; }, c));
        x = 2;
        ID(sizeof E({ int a, b; }, 1));
        return x;
    }
}
"####;
const ERASE_ALIAS1_HDR: &str = r####"#define E E0
#define E0
"####;

const ERASE_ALIAS2_SRC: &str = r####"#include <external829.h>
#define ID(v) v
#define sizeof(p,q,r) q if(r) p

int probe(int c) {
    int x = 41;
    int b = 0;
    {
        ID(sizeof E({ int a, b; }, c));
        x = 2;
        ID(sizeof E({ int a, b; }, 1));
        return x;
    }
}
"####;
const ERASE_ALIAS2_HDR: &str = r####"#define E E0
#define E0 E1
#define E1
"####;

const KNOWN_EMPTY_SRC: &str = r####"#include <external829.h>
#define ID(v) v
#define sizeof(p,q,r) q if(r) p
#define E

int probe(int c) {
    int x = 41;
    int b = 0;
    {
        ID(sizeof E({ int a, b; }, c));
        x = 2;
        ID(sizeof E({ int a, b; }, 1));
        return x;
    }
}
"####;
const KNOWN_EMPTY_HDR: &str = r####""####;

const FN_EMPTY_SRC: &str = r####"#include <external829.h>
#define ID(v) v
#define sizeof(p,q,r) q if(r) p

int probe(int c) {
    int x = 41;
    int b = 0;
    {
        ID(sizeof E()({ int a, b; }, c));
        x = 2;
        ID(sizeof E()({ int a, b; }, 1));
        return x;
    }
}
"####;
const FN_EMPTY_HDR: &str = r####"#define E()
"####;

const EXTERNAL_IDENTITY_SRC: &str = r####"#include <external829.h>
#define sizeof(p,q,r) q if(r) p

int probe(int c) {
    int x = 41;
    int b = 0;
    {
        EXT(sizeof E({ int a, b; }, c));
        x = 2;
        EXT(sizeof E({ int a, b; }, 1));
        return x;
    }
}
"####;
const EXTERNAL_IDENTITY_HDR: &str = r####"#define E
#define EXT(v) v
"####;

const EXTERNAL_IDENTITY_FN_SRC: &str = r####"#include <external829.h>
#define sizeof(p,q,r) q if(r) p

int probe(int c) {
    int x = 41;
    int b = 0;
    {
        EXT(sizeof E()({ int a, b; }, c));
        x = 2;
        EXT(sizeof E()({ int a, b; }, 1));
        return x;
    }
}
"####;
const EXTERNAL_IDENTITY_FN_HDR: &str = r####"#define E()
#define EXT(v) v
"####;

const EXTERNAL_IDENTITY_INSERT_SRC: &str = r####"#include <external829.h>
#define sizeof(p,q,r) q if(r) p

int probe(int c) {
    int x = 41;
    int b = 0;
    {
        EXT(sizeof E({ int a, b; }, c));
        x = 2;
        EXT(sizeof E({ int a, b; }, 1));
        return x;
    }
}
"####;
const EXTERNAL_IDENTITY_INSERT_HDR: &str = r####"#define E MORE
#define MORE
#define EXT(v) v
"####;

const BODY_IDENTITY_SRC: &str = r####"#include <external829.h>
#define sizeof(p,q,r) q if(r) p
#define HI EXT(sizeof E({ int a, b; }, c))
#define JI EXT(sizeof E({ int a, b; }, 1))

int probe(int c) {
    int x = 41;
    int b = 0;
    {
        HI;
        x = 2;
        JI;
        return x;
    }
}
"####;
const BODY_IDENTITY_HDR: &str = r####"#define E
#define EXT(v) v
"####;

const EXTERNAL_ALIAS_C_SRC: &str = r####"#include <external829.h>
#define F(p,q,r) q if(r) p

int probe(int c) {
    int x = 41;
    int b = 0;
    {
        E({ int a, b; }, c);
        x = 2;
        E({ int a, b; }, 1);
        return x;
    }
}
"####;
const EXTERNAL_ALIAS_C_HDR: &str = r####"#define E F
"####;

const EXTERNAL_ALIAS_CC_SRC: &str = r####"#include <external829.h>
#define F(p,q,r) q if(r) p

int probe(int c) {
    int x = 41;
    int b = 0;
    {
        E({ int a, b; }, c);
        x = 2;
        E({ int a, b; }, 1);
        return x;
    }
}
"####;
const EXTERNAL_ALIAS_CC_HDR: &str = r####"#define E F
"####;

const BODY_ALIAS_C_SRC: &str = r####"#include <external829.h>
#define F(p,q,r) q if(r) p
#define HI E({ int a, b; }, c)
#define JI E({ int a, b; }, 1)

int probe(int c) {
    int x = 41;
    int b = 0;
    {
        HI;
        x = 2;
        JI;
        return x;
    }
}
"####;
const BODY_ALIAS_C_HDR: &str = r####"#define E F
"####;

const BODY_ALIAS_CC_SRC: &str = r####"#include <external829.h>
#define F(p,q,r) q if(r) p
#define HI E({ int a, b; }, c)
#define JI E({ int a, b; }, 1)

int probe(int c) {
    int x = 41;
    int b = 0;
    {
        HI;
        x = 2;
        JI;
        return x;
    }
}
"####;
const BODY_ALIAS_CC_HDR: &str = r####"#define E F
"####;

const BODY_ERASE_SRC: &str = r####"#include <external829.h>
#define ID(v) v
#define sizeof(p,q,r) q if(r) p
#define HI sizeof E({ int a, b; }, c)
#define JI sizeof E({ int a, b; }, 1)

int probe(int c) {
    int x = 41;
    int b = 0;
    {
        ID(HI);
        x = 2;
        ID(JI);
        return x;
    }
}
"####;
const BODY_ERASE_HDR: &str = r####"#define E
"####;

const VAOPT_CHAIN_SRC: &str = r####"#include <external829.h>
#define F(p,q,r) q if(r) p
#define H(...) __VA_OPT__(F)({ int a, b; }, __VA_ARGS__)

int probe(int c) {
    int x = 41;
    int b = 0;
    {
        H(c);
        x = 2;
        H(1);
        return x;
    }
}
"####;
const VAOPT_CHAIN_HDR: &str = r####""####;

const KNOWN_ALIAS_SRC: &str = r####"#include <external829.h>
#define F(p,q,r) q if(r) p
#define ALIAS F

int probe(int c) {
    int x = 41;
    int b = 0;
    {
        ALIAS({ int a, b; }, c);
        x = 2;
        ALIAS({ int a, b; }, 1);
        return x;
    }
}
"####;
const KNOWN_ALIAS_HDR: &str = r####""####;

const KNOWN_CHAIN_SRC: &str = r####"#include <external829.h>
#define F(p,q,r) q if(r) p
#define ID(v) v

int probe(int c) {
    int x = 41;
    int b = 0;
    {
        ID(F)({ int a, b; }, c);
        x = 2;
        ID(F)({ int a, b; }, 1);
        return x;
    }
}
"####;
const KNOWN_CHAIN_HDR: &str = r####""####;

const INSERT_PARENS_SRC: &str = r####"#include <external829.h>
#define F() K
#define K(p,q,r) q if(r) p
#define ID(v) v

int probe(int c) {
    int x = 41;
    int b = 0;
    {
        ID(F E({ int a, b; }, c));
        x = 2;
        ID(F E({ int a, b; }, 1));
        return x;
    }
}
"####;
const INSERT_PARENS_HDR: &str = r####"#define E ()
"####;

const EXTERNAL_INSERT_PARENS_SRC: &str = r####"#include <external829.h>
#define F() K
#define K(p,q,r) q if(r) p

int probe(int c) {
    int x = 41;
    int b = 0;
    {
        EXT(F E({ int a, b; }, c));
        x = 2;
        EXT(F E({ int a, b; }, 1));
        return x;
    }
}
"####;
const EXTERNAL_INSERT_PARENS_HDR: &str = r####"#define E ()
#define EXT(v) v
"####;

const NAMED_VARIADIC_SRC: &str = r####"#include <external829.h>
#define F(p,q,r) q if(r) p
#define H(args...) F({args}, c)
#define J(args...) F({args}, 1)

int probe(int c) {
    int x = 41;
    int b = 0;
    {
        H(int a, b;);
        x = 2;
        J(int a, b;);
        return x;
    }
}
"####;
const NAMED_VARIADIC_HDR: &str = r####""####;

fn astra_row(what: &str, file: &str, src: &str, hdr: &str, kept: bool) {
    declare_in_model(&[("external829.h", hdr)]);
    if kept {
        kept_guard(what, file, src, Some("probe"));
    } else {
        pruned_guard(what, file, src, Some("probe"));
    }
}

#[test]
fn row_v2_astra_erase() {
    astra_row("erase", "source.c", ERASE_SRC, ERASE_HDR, true);
}

#[test]
fn row_v2_astra_erase_alias1() {
    astra_row(
        "erase_alias1",
        "source.c",
        ERASE_ALIAS1_SRC,
        ERASE_ALIAS1_HDR,
        true,
    );
}

#[test]
fn row_v2_astra_erase_alias2() {
    astra_row(
        "erase_alias2",
        "source.c",
        ERASE_ALIAS2_SRC,
        ERASE_ALIAS2_HDR,
        true,
    );
}

#[test]
fn row_v2_astra_known_empty() {
    astra_row(
        "known_empty",
        "source.c",
        KNOWN_EMPTY_SRC,
        KNOWN_EMPTY_HDR,
        true,
    );
}

#[test]
fn row_v2_astra_fn_empty() {
    astra_row("fn_empty", "source.c", FN_EMPTY_SRC, FN_EMPTY_HDR, true);
}

#[test]
fn row_v2_astra_external_identity() {
    astra_row(
        "external_identity",
        "source.c",
        EXTERNAL_IDENTITY_SRC,
        EXTERNAL_IDENTITY_HDR,
        true,
    );
}

#[test]
fn row_v2_astra_external_identity_fn() {
    astra_row(
        "external_identity_fn",
        "source.c",
        EXTERNAL_IDENTITY_FN_SRC,
        EXTERNAL_IDENTITY_FN_HDR,
        true,
    );
}

#[test]
fn row_v2_astra_external_identity_insert() {
    astra_row(
        "external_identity_insert",
        "source.c",
        EXTERNAL_IDENTITY_INSERT_SRC,
        EXTERNAL_IDENTITY_INSERT_HDR,
        true,
    );
}

#[test]
fn row_v2_astra_body_identity() {
    astra_row(
        "body_identity",
        "source.c",
        BODY_IDENTITY_SRC,
        BODY_IDENTITY_HDR,
        true,
    );
}

#[test]
fn row_v2_astra_external_alias_c() {
    astra_row(
        "external_alias_c",
        "source.c",
        EXTERNAL_ALIAS_C_SRC,
        EXTERNAL_ALIAS_C_HDR,
        true,
    );
}

#[test]
fn row_v2_astra_external_alias_cc() {
    astra_row(
        "external_alias_cc",
        "source.cc",
        EXTERNAL_ALIAS_CC_SRC,
        EXTERNAL_ALIAS_CC_HDR,
        true,
    );
}

#[test]
fn row_v2_astra_body_alias_c() {
    astra_row(
        "body_alias_c",
        "source.c",
        BODY_ALIAS_C_SRC,
        BODY_ALIAS_C_HDR,
        true,
    );
}

#[test]
fn row_v2_astra_body_alias_cc() {
    astra_row(
        "body_alias_cc",
        "source.cc",
        BODY_ALIAS_CC_SRC,
        BODY_ALIAS_CC_HDR,
        true,
    );
}

#[test]
fn row_v2_astra_body_erase() {
    astra_row(
        "body_erase",
        "source.c",
        BODY_ERASE_SRC,
        BODY_ERASE_HDR,
        true,
    );
}

#[test]
fn row_v2_astra_vaopt_chain() {
    astra_row(
        "vaopt_chain",
        "source.c",
        VAOPT_CHAIN_SRC,
        VAOPT_CHAIN_HDR,
        true,
    );
}

#[test]
fn row_v2_astra_known_alias() {
    astra_row(
        "known_alias",
        "source.c",
        KNOWN_ALIAS_SRC,
        KNOWN_ALIAS_HDR,
        true,
    );
}

#[test]
fn row_v2_astra_known_chain() {
    astra_row(
        "known_chain",
        "source.c",
        KNOWN_CHAIN_SRC,
        KNOWN_CHAIN_HDR,
        true,
    );
}

// Parser-off controls: the source owner is already G1Error, so the pair is kept on base too
// (not new contradicted pairs, not new-gate coverage).

#[test]
fn row_v2_control_insert_parens() {
    astra_row(
        "insert_parens",
        "source.c",
        INSERT_PARENS_SRC,
        INSERT_PARENS_HDR,
        true,
    );
}

#[test]
fn row_v2_control_external_insert_parens() {
    astra_row(
        "external_insert_parens",
        "source.c",
        EXTERNAL_INSERT_PARENS_SRC,
        EXTERNAL_INSERT_PARENS_HDR,
        true,
    );
}

#[test]
fn row_v2_control_named_variadic() {
    astra_row(
        "named_variadic",
        "source.c",
        NAMED_VARIADIC_SRC,
        NAMED_VARIADIC_HDR,
        true,
    );
}

const COND_BLOCK: &str = r####"#define ID(v) v
int probe(void) {
    int x = 41;
    ID({
#if 1
        int a = 0;
#else
        int a = 1;
#endif
        (void)a;
    });
    x = 2;
    return x;
}
"####;
const COND_DIRECT: &str = r####"#define ID(v) v
int probe(void) {
    int x = 41;
    int y = ID(
#if 1
        2
#else
        3
#endif
    );
    x = 2;
    return x + y;
}
"####;
const TIGHT_CALL: &str = r####"#define F(p, q, r) ((p) + (q) + (r))
int probe(int c) {
    int x = 41;
    int b = 0;
    {
        (void)F(c, 1, 2);
        x = 2;
        (void)F(c, 3, 4);
        return x;
    }
}
"####;

/// Parser pin: a compound-block argument holding complete `#if` block items is admitted, and the
/// three pairs the rules drop are CONFIRMED by the oracle (guide: no universal "conditional in
/// arguments is G1Error" claim).
#[test]
fn row_v2_parser_conditional_block() {
    pruned_guard("conditional_block", "a.c", COND_BLOCK, Some("probe"));
}

/// Parser pin: a direct expression argument interrupted by `#if` is a G1Error owner, so the
/// pair is kept on base (clang accepts the program; a parser-off control).
#[test]
fn row_v2_parser_conditional_direct() {
    kept_guard(
        "conditional_direct_valid",
        "a.c",
        COND_DIRECT,
        Some("probe"),
    );
}

/// Admitted negative: ordinary tight calls. The overwrite dominates, the pair is droppable.
#[test]
fn row_v2_tight_call_admitted() {
    pruned_guard("tight_call", "a.c", TIGHT_CALL, Some("probe"));
}
