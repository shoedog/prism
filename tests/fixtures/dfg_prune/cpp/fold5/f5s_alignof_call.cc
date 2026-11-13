int* keep; int saved; int gv;
int _alignof(int& r) { keep = &r; saved = r; return 0; }
void f() {
  int x = 1; //D  // @f5s_alignof_call:D x
  int y = _alignof(x); (void)y;
  x = 2;
  *keep = saved;
  int z = x; //U  // @f5s_alignof_call:U:keep:enum3_sonnet_area4
  (void)z;
}
