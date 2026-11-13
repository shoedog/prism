int *held;
int saved;
struct A {};
A operator,(A, int& v) { saved = v; held = &v; return {}; }
int f() {
  int x = 7; // DEF  // @x3_op_comma_ref:D x
  A a{};
  (void)(a, x);
  x = 0;
  *held = saved;
  return x; // USE  // @x3_op_comma_ref:U:keep:fold4
}
