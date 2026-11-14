// Fold 5: directive spellings the allow-list must refuse (keyword exemption keeps the escape).
// Control `c`: `T*` has no keyword proof and keeps as base does.
struct S { int v; S& operator=(int n) { v += n; return *this; } };
struct T;
int b(S x) {
#line 5 "a//b" /*
*/ } int c(int x) {   // @q2c:D x
    { T* x; }
    x = 5;               // @q2c:U:keep:q2_directive_spelling_control_no_keyword_proof
    return x;
}
