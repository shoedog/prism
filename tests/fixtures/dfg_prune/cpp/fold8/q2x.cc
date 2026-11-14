// Fold 5: directive spellings the allow-list must refuse (keyword exemption keeps the escape).
// Control `c`: `T*` has no keyword proof and keeps as base does.
struct S { int v; S& operator=(int n) { v += n; return *this; } };
struct T;
int b(S x) {
#line 5 "a//b" /*
*/ } int c(int x) {   // @q2x:D x
    { const T* x; }
    x = 5;               // @q2x:U:keep:q2_directive_spelling_exemption_refuses
    return x;
}
