// Fold 5: directive spellings the allow-list must refuse (keyword exemption keeps the escape).
// Control `c`: `T*` has no keyword proof and keeps as base does.
struct S { int v; S& operator=(int n) { v += n; return *this; } };
struct T;
void c() {
    int y = 0;   // @q5x:D y
    { const T* y; }
/**/#ifdef Z
    y = 2;
/**/#endif
    y = 3;               // @q5x:U:keep:q5_directive_spelling_exemption_refuses
}
