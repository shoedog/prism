int* keep; int saved; int gv;
struct A {};
A ga;
void hold(int& r) { keep = &r; saved = r; }
template <class T> int operator+(int& l, const T&) { keep = &l; saved = l; return 0; }
struct S { S(int& r) { keep = &r; saved = r; } };
void f() {
  int x = 1; //D  // @f5s_cmt_ctor:D x
  S s2 = (/*c*/ x); (void)s2;
  x = 2;
  *keep = saved;
  int z = x; //U  // @f5s_cmt_ctor:U:keep:enum3_sonnet_area3
  (void)z;
}
