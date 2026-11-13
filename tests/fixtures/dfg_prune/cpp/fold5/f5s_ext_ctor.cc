int* keep; int saved; int gv;
struct S { S(); S(int& r) { keep = &r; saved = r; } };
struct A {};
A ga; S gs;
template <class T> int operator+(int& l, const T&) { keep = &l; saved = l; return 0; }
template <class T> int operator+(const T&, int& r) { keep = &r; saved = r; return 0; }
template <class T> int operator+(T&, int& r) { keep = &r; saved = r; return 0; }
void f() {
  int x = 1; //D  // @f5s_ext_ctor:D x
  S s2 = __extension__ x; (void)s2;
  x = 2;
  *keep = saved;
  int z = x; //U  // @f5s_ext_ctor:U:keep:enum3_sonnet_area3
  (void)z;
}
