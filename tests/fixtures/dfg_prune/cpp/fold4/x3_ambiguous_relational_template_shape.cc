int *held;
int saved;
struct A {};
struct B {};
B operator<(A, int& v) { saved = v; held = &v; return {}; }
int operator>(B, int) { return 0; }
int f() {
  int x = 7; // DEF  // @x3_ambiguous_relational_template_shape:D x
  A a{};
  (void)(a < x > (0));
  x = 0;
  *held = saved;
  return x; // USE  // @x3_ambiguous_relational_template_shape:U:keep:fold4
}
