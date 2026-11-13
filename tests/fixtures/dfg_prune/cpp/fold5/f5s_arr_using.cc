int* keep; int saved; int gv;
struct S {};
struct A {};
A ga; S gs;
template <class T> int operator+(int& l, const T&) { keep = &l; saved = l; return 0; }
template <class T> int operator+(const T&, int& r) { keep = &r; saved = r; return 0; }
template <class T> int operator+(T&, int& r) { keep = &r; saved = r; return 0; }
namespace N { S a[2]; }
void f() {
  int x = 1; //D  // @f5s_arr_using:D x
  int a[2] = {0, 0};
  { using N::a; int y = x + a[0]; (void)y; }
  x = 2;
  *keep = saved;
  int z = x; //U  // @f5s_arr_using:U:keep:enum3_sonnet_area3
  (void)z;
}
