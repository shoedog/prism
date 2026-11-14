// Fold 5: directive spellings the allow-list must refuse (keyword exemption keeps the escape).
// Control `c`: `T*` has no keyword proof and keeps as base does.
struct S { int v; S& operator=(int n) { v += n; return *this; } };
struct T;
void c() {
    int y = 0;   // @q7c:D y
    { T* y; }
/**/#ifdef Z
/**/#endif /* note
*/ y = 2;
    y = 3;               // @q7c:U:keep:q7_directive_spelling_control_no_keyword_proof
}
