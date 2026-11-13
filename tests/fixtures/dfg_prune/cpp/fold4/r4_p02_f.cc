enum E { A, B };
int* kept;
int operator+(int& a, E) { kept = &a; return a; }
int src();
void sink(int);
void f(E e) {
  int x = src();      // 7  D  // @r4_p02_f:D x
  sink(x + e);        // 8  binds &x through operator+(int&, E)
  int saved = *kept;
  x = 0;              // 10 W
  *kept = saved;
  int z = x + 1;      // 12 U  // @r4_p02_f:U:keep:fold4
  sink(z);
}
enum class F { P };
int*& operator|=(int*& a, F) { static int** kp; kp = &a; return a; }
int* psrc();
void sinkp(int*);
void g() {
  int* p = psrc();    // 20 D
  p |= F::P;          // 21 compound, scoped-enum right operand
  p = nullptr;        // 22 W
  sinkp(p + 1);       // 23 U
}
