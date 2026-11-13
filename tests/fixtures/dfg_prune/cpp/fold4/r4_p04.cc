int* kept;
struct S { S(int& r) { kept = &r; } };
S operator+(S, S);
int src();
void sink(int);
void f(S s) {
  int x = src();      // 7  D  // @r4_p04:D x
  s + x;              // 8  x -> S through S(int&): the constructor keeps &x
  int saved = *kept;
  x = 0;              // 10 W
  *kept = saved;
  int z = x + 1;      // 12 U  // @r4_p04:U:keep:fold4
  sink(z);
}
