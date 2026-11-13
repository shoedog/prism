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
  int x = 1; //D  // @f5s_tpl_ref:D x
  int y5 = 0; (void)y5;
  { ga < x > & y5; }
  x = 2;
  *keep = saved;
  int z = x; //U  // @f5s_tpl_ref:U:keep:enum3_sonnet_area2
  (void)z;
}
