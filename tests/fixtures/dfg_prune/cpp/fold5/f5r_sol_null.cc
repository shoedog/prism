int *held;
int saved;
struct S {};
S operator+(S, int& v) { saved = v; held = &v; return {}; }
S NULL;
int f() {
  int x = 7;  // @f5r_sol_null:D x
  (void)(NULL + x);
  x = 0;
  *held = saved;
  return x;  // @f5r_sol_null:U:keep:r5_sol_null_object
}
