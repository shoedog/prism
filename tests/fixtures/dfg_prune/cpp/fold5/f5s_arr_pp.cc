int* keep; int saved; int gv;
struct S {};
struct A {};
A ga; S gs;
template <class T> int operator+(int& l, const T&) { keep = &l; saved = l; return 0; }
template <class T> int operator+(const T&, int& r) { keep = &r; saved = r; return 0; }
template <class T> int operator+(T&, int& r) { keep = &r; saved = r; return 0; }
void f() {
  int x = 1; //D  // @f5s_arr_pp:D x
  int a[2] = {0, 0};
  {
#if 1
 S a[2];
#endif
 int y = x + a[0]; (void)y; }
  x = 2;
  *keep = saved;
  int z = x; //U  // @f5s_arr_pp:U:keep:enum3_sonnet_area3
  (void)z;
}
