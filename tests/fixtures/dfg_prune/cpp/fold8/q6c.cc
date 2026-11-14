// Fold 5: directive spellings the allow-list must refuse (keyword exemption keeps the escape).
// Control `c`: `T*` has no keyword proof and keeps as base does.
struct S { int v; S& operator=(int n) { v += n; return *this; } };
struct T;
/* a
*/ #ifndef A
int b(S x) {
/* b
*/ #else
int b(int x) {   // @q6c:D x
/* c
*/ #endif
    { T* x; }
    x = 5;               // @q6c:U:keep:q6_directive_spelling_control_no_keyword_proof
    return x;
}
