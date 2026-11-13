int* keep; int saved; int gv;
struct A {};
A ga;
void hold(int& r) { keep = &r; saved = r; }
template <class T> int operator+(int& l, const T&) { keep = &l; saved = l; return 0; }
void f() {
  int x = 1; //D  // @f5s_cmt_operand:D x
  int y = (/*c*/ x) + ga; (void)y;
  x = 2;
  *keep = saved;
  int z = x; //U  // @f5s_cmt_operand:U:keep:enum3_sonnet_area3
  (void)z;
}
