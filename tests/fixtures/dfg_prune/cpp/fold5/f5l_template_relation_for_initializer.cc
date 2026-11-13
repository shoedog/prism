int *held;
int saved;
struct A {
    int operator<(int& r) const { held = &r; saved = r; return 0; }
};
int f() {
    A a;
    int x = 7;  // @f5l_template_relation_for_initializer:D x
    int y = 1;
    for (a < x > y; false; ) {}
    x = 0;
    *held = saved;
    return x + 1;  // @f5l_template_relation_for_initializer:U:keep:enum3_luna_area2
}
