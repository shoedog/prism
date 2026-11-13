int* keep; int saved; int gv;
int offsetof(int& r, int) { keep = &r; saved = r; return 0; }
void f() {
  int x = 1; //D  // @f5s_offsetof_call:D x
  int y = offsetof(x, gv); (void)y;
  x = 2;
  *keep = saved;
  int z = x; //U  // @f5s_offsetof_call:U:keep:enum3_sonnet_area4
  (void)z;
}
