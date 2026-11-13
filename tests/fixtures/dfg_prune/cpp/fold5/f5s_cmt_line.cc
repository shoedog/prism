int* keep; int saved; int gv;
struct A {};
A ga;
void hold(int& r) { keep = &r; saved = r; }
template <class T> int operator+(int& l, const T&) { keep = &l; saved = l; return 0; }
void f() {
  int x = 1; //D  // @f5s_cmt_line:D x
  hold((// c
 x));
  x = 2;
  *keep = saved;
  int z = x; //U  // @f5s_cmt_line:U:keep:enum3_sonnet_area3
  (void)z;
}
