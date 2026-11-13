enum E { tag };
int *kept;
int saved;
int source();
int& operator+=(int& r, E) { kept = &r; saved = r; return r; }
int f() {
  int x = source();  // @r4_sol_enum_compound:D x
  x += tag;
  x = 0;
  *kept = saved;
  return x + 1;  // @r4_sol_enum_compound:U:keep:fold4_sol
}
