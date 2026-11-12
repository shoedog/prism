int *held;
int saved;
struct A {
  A& operator=(int& v) { saved = v; held = &v; return *this; }
};
int f() {
  int x = 7; // DEF  // @op_assignment_ref:D x
  A a{};
  a = x;
  x = 0;
  *held = saved;
  return x; // USE  // @op_assignment_ref:U:keep:fold2_enum
}