// Fold 5: directive spellings the allow-list must refuse (keyword exemption keeps the escape).
// Control `c`: `T*` has no keyword proof and keeps as base does.
struct S { int v; S& operator=(int n) { v += n; return *this; } };
struct T;
int a(S x) {
/*gap*/ #if 0
}
int b(int x) {   // @g1c:D x
/*gap*/ #endif
    { T* x; }
    x = 5;               // @g1c:U:keep:g1_directive_spelling_control_no_keyword_proof
    return x;
}
