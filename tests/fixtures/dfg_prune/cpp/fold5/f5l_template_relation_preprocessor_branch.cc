int *held;
int saved;
struct A {
    int operator<(int& r) const { held = &r; saved = r; return 0; }
};
int f() {
    A a;
    int x = 7;  // @f5l_template_relation_preprocessor_branch:D x
    int y = 1;
#if 1
    a < x > y;
#endif
    x = 0;
    *held = saved;
    return x + 1;  // @f5l_template_relation_preprocessor_branch:U:keep:enum3_luna_area2
}
