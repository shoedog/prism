int* keep; int saved; int gv;
void hold(int& r) { keep = &r; saved = r; }
void f() {
  int x = 1; //D  // @f5s_ext_paren:D x
  hold((__extension__ x));
  x = 2;
  *keep = saved;
  int z = x; //U  // @f5s_ext_paren:U:keep:enum3_sonnet_area3
  (void)z;
}
