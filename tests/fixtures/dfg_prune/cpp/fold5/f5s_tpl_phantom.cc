int* keep; int saved;
struct A {};
struct R {};
A ga;
R operator<(const A&, const A&) { return R(); }
int operator>(const R&, int& r) { keep = &r; saved = r; return 0; }
void f() {
  int x = 1; //D  // @f5s_tpl_phantom:D x
  { ga < ga > x; }
  x = 2;
  *keep = saved;
  int z = x; //U  // @f5s_tpl_phantom:U:keep:enum3_sonnet_area1
  (void)z;
}
