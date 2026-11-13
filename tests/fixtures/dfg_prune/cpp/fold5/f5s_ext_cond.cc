int* keep; int saved; int gv;
void hold(int& r) { keep = &r; saved = r; }
void f() {
  int x = 1; //D  // @f5s_ext_cond:D x
  hold(gv ? __extension__ x : gv);
  x = 2;
  *keep = saved;
  int z = x; //U  // @f5s_ext_cond:U:keep:enum3_sonnet_area3
  (void)z;
}
