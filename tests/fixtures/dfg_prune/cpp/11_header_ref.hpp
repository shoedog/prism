// C++ header control (#630 slice 6c, fold 1): the C++ grammar sees the reference declarator,
// so the reference initializer escapes `x` and the pair is kept by that escape.
int src();
void sink(int);
typedef int T;
void save(T& r);
void restore(T& r);
static inline void header_typedef_ref_paren_cpp_keep() {
  int x = src();            // @hr:D x
  T& r = (x);
  save(r);
  x = 0;
  restore(r);
  int z = x + 1;            // @hr:U:keep:reference_initializer_escapes_x_in_a_cpp_header
  sink(z);
}
