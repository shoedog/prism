// Fold 5: directive spellings the allow-list must refuse (keyword exemption keeps the escape).
// Control `c`: `T*` has no keyword proof and keeps as base does.
struct S { int v; S& operator=(int n) { v += n; return *this; } };
struct T;
int a(S x) {
#if/**/0
}
int b(int x) {   // @g2x:D x
#endif
    { const T* x; }
    x = 5;               // @g2x:U:keep:g2_directive_spelling_exemption_refuses
    return x;
}
