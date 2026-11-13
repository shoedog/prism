int *held;
int saved;
struct A {
    int operator<(int& r) const { held = &r; saved = r; return 0; }
};
int f() {
    A a;
    int x = 7;  // @f5l_template_relation_case_declaration:D x
    int y = 1;
    switch (0) {
    case 0: a < x > y; break;
    }
    x = 0;
    *held = saved;
    return x + 1;  // @f5l_template_relation_case_declaration:U:keep:enum3_luna_area2
}
