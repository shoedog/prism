int* keep; int saved; int gv;
void hold(int& r) { keep = &r; saved = r; }
void f() {
  int x = 1; //D  // @f5s_generic_arg:D x
  hold(_Generic(0, int: x));
  x = 2;
  *keep = saved;
  int z = x; //U  // @f5s_generic_arg:U:keep:enum3_sonnet_area3
  (void)z;
}
