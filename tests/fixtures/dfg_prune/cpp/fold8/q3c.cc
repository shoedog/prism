// Fold 5: directive spellings the allow-list must refuse (keyword exemption keeps the escape).
// Control `c`: `T*` has no keyword proof and keeps as base does.
struct S { int v; S& operator=(int n) { v += n; return *this; } };
struct T;
int b(S x) {
#ident "a//b" /*
*/ } int c(int x) {   // @q3c:D x
    { T* x; }
    x = 5;               // @q3c:U:keep:q3_directive_spelling_control_no_keyword_proof
    return x;
}
