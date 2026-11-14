// Fold 5: directive spellings the allow-list must refuse (keyword exemption keeps the escape).
// Control `c`: `T*` has no keyword proof and keeps as base does.
struct S { int v; S& operator=(int n) { v += n; return *this; } };
struct T;
int a(S x) {
/*gap*/ #if 0
}
int b(int x) {   // @g1x:D x
/*gap*/ #endif
    { const T* x; }
    x = 5;               // @g1x:U:keep:g1_directive_spelling_exemption_refuses
    return x;
}
