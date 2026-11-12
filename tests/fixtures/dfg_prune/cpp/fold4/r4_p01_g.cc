enum E { A, B };
int* kept;
int operator+(int& a, E) { kept = &a; return a; }
int src();
void sink(int);
void f(E e) {
  int x = src();      // 7  D
  int t = x + e;      // 8  binds &x through operator+(int&, E)
  int saved = *kept;
  x = 0;              // 10 W
  *kept = saved;
  int z = x + 1;      // 12 U
  sink(z + t);
}
int& operator+=(int& a, E) { kept = &a; return a; }
void g() {
  int x = src();      // 17 D  // @r4_p01_g:D x
  x += A;             // 18 compound, enum right operand (enumerator)
  int saved = *kept;
  x = 0;              // 20 W
  *kept = saved;
  int z = x + 1;      // 22 U  // @r4_p01_g:U:keep:fold4
  sink(z);
}
