int* kept;
struct S { S(int& r) { kept = &r; } explicit operator bool() const { return true; } };
extern S nul_s;
#define NULL nul_s
int src();
void sink(int);
void f(bool c) {
  int x = src();            // 8  D  // @r4_p07:D x
  if (c ? x : NULL) {}      // 9  x -> S through S(int&)
  int saved = *kept;
  x = 0;                    // 11 W
  *kept = saved;
  int z = x + 1;            // 13 U  // @r4_p07:U:keep:fold4
  sink(z);
}
