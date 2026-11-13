int* keep; int saved; int gv; int gi2;
struct A {};
struct R {};
A ga; A ga2; int h(int) { return 0; }
R operator<(const A&, int& r) { keep = &r; saved = r; return R(); }
R operator<(int& l, const A&) { keep = &l; saved = l; return R(); }
int& operator>(const R&, int&) { return gi2; }
int operator>(const R&, int&&) { return 0; }
int operator>(const R&, int*) { return 0; }
int g() { return 0; }
void f() {
  int x = 1; //D  // @f5s_tpl_init:D x
  int y6 = 0; (void)y6;
  { ga < x > y6 = 0; }
  x = 2;
  *keep = saved;
  int z = x; //U  // @f5s_tpl_init:U:keep:enum3_sonnet_area2
  (void)z;
}
