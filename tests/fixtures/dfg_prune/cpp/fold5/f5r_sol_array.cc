int *held;
int saved;
struct S {};
S operator+(S, int& v) { saved = v; held = &v; return {}; }
namespace N { S a; }
int f() {
  int a[1];
  int x = 7;  // @f5r_sol_array:D x
  {
    using N::a;
    (void)(a + x);
  }
  x = 0;
  *held = saved;
  return x;  // @f5r_sol_array:U:keep:r5_sol_array_escape
}
