// Fold 5: directive spellings the allow-list must refuse (keyword exemption keeps the escape).
// Control `c`: `T*` has no keyword proof and keeps as base does.
struct S { int v; S& operator=(int n) { v += n; return *this; } };
struct T;
/**/#ifndef A
int b(S x) {
/**/#else
int b(int x) {   // @q1x:D x
/**/#endif
    { const T* x; }
    x = 5;               // @q1x:U:keep:q1_directive_spelling_exemption_refuses
    return x;
}
